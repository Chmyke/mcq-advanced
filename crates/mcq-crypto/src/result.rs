//! A student's copy, sealed for the teacher (`.mcqres`).
//!
//! Layout: `"MCQR" | version u8 | paper id [16] | seal public [32] | HPKE enc [32] | bytes ciphertext`
//! The ciphertext is HPKE (RFC 9180, base mode, X25519, HKDF-SHA256, ChaCha20-Poly1305) of the
//! encoded submission, with every byte before it as associated data. Only the teacher's seal key
//! opens it; the student keeps a receipt code derived from the whole file.

use hpke::aead::ChaCha20Poly1305;
use hpke::kdf::HkdfSha256;
use hpke::rand_core::CryptoRng;
use hpke::{Deserializable, OpModeR, OpModeS, Serializable};
use mcq_format::binary::{ByteReader, ByteWriter, decode_submission, encode_submission};
use mcq_model::Submission;
use zeroize::Zeroizing;

use crate::codes::{encode, group};
use crate::identity::{Identity, Kem};
use crate::package::PackageHeader;
use crate::{Error, primitives};

const MAGIC: &[u8; 4] = b"MCQR";
const VERSION: u8 = 1;
const HPKE_INFO: &[u8] = b"mcq-advanced/result/v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedResult {
    pub bytes: Vec<u8>,
    /// Shown to the student and found again by the teacher, such as `7KQ2M-XW9DA`.
    pub receipt: String,
}

/// Seals a copy for the teacher of the package it answers.
pub fn seal_result(
    package: &PackageHeader,
    submission: &Submission,
    rng: &mut impl CryptoRng,
) -> Result<SealedResult, Error> {
    if submission.paper_id != package.paper_id {
        return Err(Error::NotAllowed("the copy answers another package"));
    }
    let mut w = ByteWriter::new();
    w.fixed(MAGIC)
        .u8(VERSION)
        .fixed(&package.paper_id)
        .fixed(&package.teacher.seal);
    let plaintext = Zeroizing::new(encode_submission(submission));
    let (encapped, ciphertext) =
        hpke::single_shot_seal_with_rng::<ChaCha20Poly1305, HkdfSha256, Kem>(
            &OpModeS::Base,
            &package.teacher.seal_key()?,
            HPKE_INFO,
            &plaintext,
            w.as_slice(),
            rng,
        )
        .map_err(|_| Error::Malformed("cannot seal for the teacher key"))?;
    // The encapsulated key is authenticated by HPKE itself, so it sits after the associated data.
    w.fixed(&encapped.to_bytes()).bytes(&ciphertext);
    let bytes = w.into_vec();
    let receipt = receipt_code(&bytes);
    Ok(SealedResult { bytes, receipt })
}

/// Opens a copy with the teacher identity; returns it with its receipt code.
pub fn open_result(bytes: &[u8], identity: &Identity) -> Result<(Submission, String), Error> {
    let mut r = ByteReader::new(bytes);
    if r.take(4).ok() != Some(MAGIC.as_slice()) {
        return Err(Error::WrongKind);
    }
    let version = r.u8()?;
    if version != VERSION {
        return Err(Error::UnsupportedVersion(version));
    }
    let paper_id: [u8; 16] = r.fixed()?;
    let seal: [u8; 32] = r.fixed()?;
    let aad_len = r.position();
    let enc: [u8; 32] = r.fixed()?;
    let ciphertext = r.bytes()?;
    r.finish()?;
    if seal != identity.public().seal {
        return Err(Error::WrongSecret);
    }
    let encapped = <Kem as hpke::Kem>::EncappedKey::from_bytes(&enc)
        .map_err(|_| Error::Malformed("encapsulated key"))?;
    let plaintext = hpke::single_shot_open::<ChaCha20Poly1305, HkdfSha256, Kem>(
        &OpModeR::Base,
        &identity.seal_secret()?,
        &encapped,
        HPKE_INFO,
        ciphertext,
        &bytes[..aad_len],
    )
    .map_err(|_| Error::WrongSecret)?;
    let submission = decode_submission(&Zeroizing::new(plaintext))?;
    if submission.paper_id != paper_id {
        return Err(Error::Malformed("paper id differs inside and outside"));
    }
    Ok((submission, receipt_code(bytes)))
}

/// 50 bits of SHA-256 of the sealed file, in two groups of five symbols.
pub fn receipt_code(sealed: &[u8]) -> String {
    let hash = primitives::sha256(&[b"mcq-advanced/receipt/v1", sealed]);
    group(&encode(&hash, 10), 5)
}
