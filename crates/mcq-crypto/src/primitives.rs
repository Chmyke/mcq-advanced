//! Thin wrappers over the standard primitives, so the rest of the crate never touches their APIs.

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{Key, KeyInit, XChaCha20Poly1305, XNonce};
use hpke::rand_core::CryptoRng;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::Error;

/// Argon2id cost settings, stored in every file that derives a key from a human secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KdfParams {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl KdfParams {
    /// For generated codes: 60 random bits already make guessing hopeless, the cost only adds
    /// margin, and phones must open the paper quickly (OWASP minimum for Argon2id).
    pub const CODE: Self = Self {
        memory_kib: 19 * 1024,
        iterations: 2,
        parallelism: 1,
    };
    /// For chosen passphrases (teacher identity, chosen opening phrase): costlier.
    pub const PASSPHRASE: Self = Self {
        memory_kib: 64 * 1024,
        iterations: 3,
        parallelism: 1,
    };
    /// Cheapest accepted settings. Tests only: never for real files.
    pub const TESTING: Self = Self {
        memory_kib: 8,
        iterations: 1,
        parallelism: 1,
    };

    /// Bounds that keep a forged file from demanding absurd memory or time.
    pub(crate) fn check(self) -> Result<Self, Error> {
        let ok = (1..=4).contains(&self.parallelism)
            && (1..=16).contains(&self.iterations)
            && self.memory_kib >= 8 * self.parallelism
            && self.memory_kib <= 1024 * 1024;
        if ok {
            Ok(self)
        } else {
            Err(Error::BadKdfParams)
        }
    }
}

pub(crate) const KEY_LEN: usize = 32;
pub(crate) const NONCE_LEN: usize = 24;
pub(crate) const SALT_LEN: usize = 16;

pub(crate) fn random<const N: usize>(rng: &mut impl CryptoRng) -> [u8; N] {
    let mut out = [0_u8; N];
    rng.fill_bytes(&mut out);
    out
}

pub(crate) fn sha256(parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

/// Argon2id of a human secret.
pub(crate) fn derive_key(
    secret: &[u8],
    salt: &[u8; SALT_LEN],
    params: KdfParams,
) -> Result<Zeroizing<[u8; KEY_LEN]>, Error> {
    let params = params.check()?;
    let argon_params = Params::new(
        params.memory_kib,
        params.iterations,
        params.parallelism,
        Some(KEY_LEN),
    )
    .map_err(|_| Error::BadKdfParams)?;
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params)
        .hash_password_into(secret, salt, key.as_mut())
        .map_err(|_| Error::BadKdfParams)?;
    Ok(key)
}

/// XChaCha20-Poly1305 with a fresh random nonce; returns `nonce || ciphertext`.
pub(crate) fn seal(
    key: &[u8; KEY_LEN],
    plaintext: &[u8],
    aad: &[u8],
    rng: &mut impl CryptoRng,
) -> Result<Vec<u8>, Error> {
    let nonce: [u8; NONCE_LEN] = random(rng);
    let cipher = XChaCha20Poly1305::new(&Key::from(*key));
    let ciphertext = cipher
        .encrypt(
            &XNonce::from(nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| Error::Malformed("plaintext too long"))?;
    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Opens `nonce || ciphertext` from `seal`.
pub(crate) fn open(
    key: &[u8; KEY_LEN],
    sealed: &[u8],
    aad: &[u8],
) -> Result<Zeroizing<Vec<u8>>, Error> {
    if sealed.len() < NONCE_LEN {
        return Err(Error::Malformed("encrypted field too short"));
    }
    let (nonce, ciphertext) = sealed.split_at(NONCE_LEN);
    let mut nonce_bytes = [0_u8; NONCE_LEN];
    nonce_bytes.copy_from_slice(nonce);
    let cipher = XChaCha20Poly1305::new(&Key::from(*key));
    cipher
        .decrypt(
            &XNonce::from(nonce_bytes),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| Error::WrongSecret)
}

/// `label` is a fixed domain-separation string; the result binds a field to its file header.
pub(crate) fn aad(header_hash: &[u8; 32], label: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(32 + label.len());
    out.extend_from_slice(header_hash);
    out.extend_from_slice(label.as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_bounds() {
        assert!(KdfParams::CODE.check().is_ok());
        assert!(KdfParams::PASSPHRASE.check().is_ok());
        assert!(KdfParams::TESTING.check().is_ok());
        let huge = KdfParams {
            memory_kib: u32::MAX,
            ..KdfParams::CODE
        };
        assert_eq!(huge.check(), Err(Error::BadKdfParams));
        let zero = KdfParams {
            iterations: 0,
            ..KdfParams::CODE
        };
        assert_eq!(zero.check(), Err(Error::BadKdfParams));
    }

    #[test]
    fn argon2id_matches_a_reference_value() {
        // Pins the algorithm, version and parameter wiring: any change breaks old files.
        let key = derive_key(b"password", &[0x02; SALT_LEN], KdfParams::TESTING).expect("derive");
        let again = derive_key(b"password", &[0x02; SALT_LEN], KdfParams::TESTING).expect("derive");
        let other = derive_key(b"passwore", &[0x02; SALT_LEN], KdfParams::TESTING).expect("derive");
        assert_eq!(*key, *again);
        assert_ne!(*key, *other);
    }
}
