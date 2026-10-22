//! The teacher identity: an Ed25519 key that signs packages and an X25519 (HPKE) key that copies
//! and answer keys are sealed for, locked in a file by the teacher's passphrase.
//!
//! File layout (`.mcqid`), integers little-endian:
//! `"MCQI" | version u8 | sign public [32] | seal public [32] | memory_kib u32 | iterations u32 |
//!  parallelism u32 | salt [16] | len u32 | nonce [24] || XChaCha20-Poly1305(sign seed || seal secret)`
//! The associated data is every byte before the length.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use hpke::kem::X25519HkdfSha256;
use hpke::rand_core::CryptoRng;
use hpke::{Deserializable, Kem as _, Serializable};
use mcq_format::binary::{ByteReader, ByteWriter};
use zeroize::Zeroizing;

use crate::codes::{encode, group};
use crate::primitives::{self, KdfParams, SALT_LEN};
use crate::{Error, checked_passphrase};

const MAGIC: &[u8; 4] = b"MCQI";
const VERSION: u8 = 1;

pub(crate) type Kem = X25519HkdfSha256;
pub(crate) type SealSecret = <Kem as hpke::Kem>::PrivateKey;
pub(crate) type SealPublic = <Kem as hpke::Kem>::PublicKey;

/// The public half: what students' players see and check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicIdentity {
    /// Ed25519 key that signs packages.
    pub sign: [u8; 32],
    /// X25519 key that copies and answer keys are sealed for.
    pub seal: [u8; 32],
}

impl PublicIdentity {
    /// A short code to compare out of band, such as `3F9K-QW2M-7HXA-CC4D`. Printed on the room
    /// sheet and shown by the player, so a forged package with another key shows another code.
    pub fn fingerprint(&self) -> String {
        let hash = primitives::sha256(&[b"mcq-advanced/teacher/v1", &self.sign, &self.seal]);
        group(&encode(&hash, 16), 4)
    }

    pub(crate) fn verify(&self, message: &[u8], signature: &[u8; 64]) -> Result<(), Error> {
        let key = VerifyingKey::from_bytes(&self.sign).map_err(|_| Error::BadSignature)?;
        key.verify_strict(message, &Signature::from_bytes(signature))
            .map_err(|_| Error::BadSignature)
    }

    pub(crate) fn seal_key(&self) -> Result<SealPublic, Error> {
        SealPublic::from_bytes(&self.seal).map_err(|_| Error::Malformed("seal public key"))
    }
}

/// The full identity. Never leaves the teacher's device unlocked.
pub struct Identity {
    signing: SigningKey,
    seal_secret: Zeroizing<[u8; 32]>,
    public: PublicIdentity,
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Identity")
            .field("public", &self.public)
            .finish_non_exhaustive()
    }
}

impl Identity {
    pub fn generate(rng: &mut impl CryptoRng) -> Self {
        let sign_seed = Zeroizing::new(primitives::random::<32>(rng));
        let ikm = Zeroizing::new(primitives::random::<32>(rng));
        let (secret, _) = Kem::derive_keypair(ikm.as_ref());
        let mut seal_secret = Zeroizing::new([0_u8; 32]);
        seal_secret.copy_from_slice(&secret.to_bytes());
        Self::from_secrets(&sign_seed, seal_secret).expect("a derived X25519 key is valid")
    }

    fn from_secrets(sign_seed: &[u8; 32], seal_secret: Zeroizing<[u8; 32]>) -> Result<Self, Error> {
        let signing = SigningKey::from_bytes(sign_seed);
        let secret = SealSecret::from_bytes(seal_secret.as_ref())
            .map_err(|_| Error::Malformed("seal secret key"))?;
        let mut seal = [0_u8; 32];
        seal.copy_from_slice(&Kem::sk_to_pk(&secret).to_bytes());
        let public = PublicIdentity {
            sign: signing.verifying_key().to_bytes(),
            seal,
        };
        Ok(Self {
            signing,
            seal_secret,
            public,
        })
    }

    pub fn public(&self) -> PublicIdentity {
        self.public
    }

    pub(crate) fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing.sign(message).to_bytes()
    }

    pub(crate) fn seal_secret(&self) -> Result<SealSecret, Error> {
        SealSecret::from_bytes(self.seal_secret.as_ref())
            .map_err(|_| Error::Malformed("seal secret key"))
    }

    /// The identity file, locked by a passphrase of at least `MIN_PASSPHRASE_CHARS` characters.
    pub fn lock(
        &self,
        passphrase: &str,
        params: KdfParams,
        rng: &mut impl CryptoRng,
    ) -> Result<Vec<u8>, Error> {
        let passphrase = checked_passphrase(passphrase)?;
        let salt: [u8; SALT_LEN] = primitives::random(rng);
        let mut w = ByteWriter::new();
        w.fixed(MAGIC)
            .u8(VERSION)
            .fixed(&self.public.sign)
            .fixed(&self.public.seal)
            .u32(params.memory_kib)
            .u32(params.iterations)
            .u32(params.parallelism)
            .fixed(&salt);
        let key = primitives::derive_key(passphrase.as_bytes(), &salt, params)?;
        let mut secrets = Zeroizing::new([0_u8; 64]);
        secrets[..32].copy_from_slice(&self.signing.to_bytes());
        secrets[32..].copy_from_slice(self.seal_secret.as_ref());
        let sealed = primitives::seal(&key, secrets.as_ref(), w.as_slice(), rng)?;
        w.bytes(&sealed);
        Ok(w.into_vec())
    }

    /// Opens an identity file with its passphrase.
    pub fn unlock(file: &[u8], passphrase: &str) -> Result<Self, Error> {
        let header = read_header(file)?;
        let key =
            primitives::derive_key(passphrase.trim().as_bytes(), &header.salt, header.params)?;
        let secrets = primitives::open(&key, header.sealed, &file[..header.aad_len])?;
        if secrets.len() != 64 {
            return Err(Error::Malformed("identity secrets"));
        }
        let mut sign_seed = Zeroizing::new([0_u8; 32]);
        sign_seed.copy_from_slice(&secrets[..32]);
        let mut seal_secret = Zeroizing::new([0_u8; 32]);
        seal_secret.copy_from_slice(&secrets[32..]);
        let identity = Self::from_secrets(&sign_seed, seal_secret)?;
        if identity.public != header.public {
            return Err(Error::Malformed("public keys do not match the secrets"));
        }
        Ok(identity)
    }

    /// The public half of an identity file, without the passphrase.
    pub fn read_public(file: &[u8]) -> Result<PublicIdentity, Error> {
        Ok(read_header(file)?.public)
    }
}

struct Header<'a> {
    public: PublicIdentity,
    params: KdfParams,
    salt: [u8; SALT_LEN],
    aad_len: usize,
    sealed: &'a [u8],
}

fn read_header(file: &[u8]) -> Result<Header<'_>, Error> {
    let mut r = ByteReader::new(file);
    if r.take(4).ok() != Some(MAGIC.as_slice()) {
        return Err(Error::WrongKind);
    }
    let version = r.u8()?;
    if version != VERSION {
        return Err(Error::UnsupportedVersion(version));
    }
    let public = PublicIdentity {
        sign: r.fixed()?,
        seal: r.fixed()?,
    };
    let params = KdfParams {
        memory_kib: r.u32()?,
        iterations: r.u32()?,
        parallelism: r.u32()?,
    }
    .check()?;
    let salt = r.fixed()?;
    let aad_len = r.position();
    let sealed = r.bytes()?;
    r.finish()?;
    Ok(Header {
        public,
        params,
        salt,
        aad_len,
        sealed,
    })
}
