//! ROLE:    the security of MCQ Advanced: teacher identity, signed and encrypted packages, an answer
//!          key encrypted apart, and copies sealed for the teacher.
//! OWNER:   pure functions over bytes; randomness comes from the caller's RNG, never from a global.
//! INVARIANTS: only standard primitives (Argon2id, XChaCha20-Poly1305, Ed25519, HPKE RFC 9180);
//!             every decoder refuses malformed input without panicking; a package is verified
//!             against its signature before anything in it is decrypted.
//!
//! The threat model and the byte layouts are documented in `SECURITY.md`.

mod codes;
mod identity;
mod package;
mod primitives;
mod result;

pub use codes::{CODE_SYMBOLS, generate_code, normalize_code};
pub use hpke::rand_core;
pub use identity::{Identity, PublicIdentity};
pub use package::{
    KeyUnlock, Opening, OpeningKind, PAPER_FILE, PackageHeader, PublishOptions, StudentPackage,
    inspect, open, open_answer_key, publish,
};
pub use primitives::KdfParams;
pub use result::{SealedResult, open_result, receipt_code, seal_result};

/// Chosen passphrases shorter than this are refused.
pub const MIN_PASSPHRASE_CHARS: usize = 12;

/// Why an operation failed. Messages never include secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Not a file of the expected kind.
    WrongKind,
    UnsupportedVersion(u8),
    /// The bytes break the format.
    Malformed(&'static str),
    /// The signature does not match: the file was altered or forged.
    BadSignature,
    /// Decryption failed: wrong code, passphrase or identity, or altered data.
    WrongSecret,
    /// A chosen passphrase is too short.
    WeakPassphrase,
    /// A code does not have the expected form.
    BadCode,
    /// The package does not allow this way of opening its answer key.
    NotAllowed(&'static str),
    /// Key derivation settings out of the accepted bounds.
    BadKdfParams,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::WrongKind => f.write_str("not a file of the expected kind"),
            Error::UnsupportedVersion(v) => write!(f, "unsupported format version {v}"),
            Error::Malformed(what) => write!(f, "malformed file: {what}"),
            Error::BadSignature => f.write_str("the signature does not match: altered or forged"),
            Error::WrongSecret => {
                f.write_str("cannot decrypt: wrong code, passphrase or identity, or altered data")
            }
            Error::WeakPassphrase => write!(
                f,
                "the passphrase needs at least {MIN_PASSPHRASE_CHARS} characters"
            ),
            Error::BadCode => f.write_str("the code does not have the expected form"),
            Error::NotAllowed(what) => write!(f, "not allowed: {what}"),
            Error::BadKdfParams => f.write_str("key derivation settings out of bounds"),
        }
    }
}

impl std::error::Error for Error {}

impl From<mcq_format::binary::DecodeError> for Error {
    fn from(e: mcq_format::binary::DecodeError) -> Self {
        use mcq_format::binary::DecodeError;
        match e {
            DecodeError::Truncated => Error::Malformed("truncated"),
            DecodeError::TrailingBytes => Error::Malformed("trailing bytes"),
            DecodeError::NotUtf8 => Error::Malformed("text is not UTF-8"),
            DecodeError::UnsupportedVersion(v) => Error::UnsupportedVersion(v),
            DecodeError::Invalid(what) => Error::Malformed(what),
        }
    }
}

/// Trims a chosen passphrase and refuses it when too short.
fn checked_passphrase(passphrase: &str) -> Result<&str, Error> {
    let trimmed = passphrase.trim();
    if trimmed.chars().count() < MIN_PASSPHRASE_CHARS {
        Err(Error::WeakPassphrase)
    } else {
        Ok(trimmed)
    }
}
