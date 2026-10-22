//! The student package (`.mcqpack`): the paper without its answers, encrypted for an opening code,
//! the answer key encrypted apart, everything signed by the teacher.
//!
//! Layout, integers little-endian; `bytes` fields carry a `u32` length prefix:
//! ```text
//! header (130 bytes):
//!   "MCQP" | version u8 | flags u8 | paper id [16] | sign public [32] | seal public [32]
//!   | memory_kib u32 | iterations u32 | parallelism u32 | content salt [16] | end salt [16]
//! bytes content wrap   open: content key; else seal(Argon2id(opening), content key)
//! bytes content        seal(content key, archive of the student copy and media)
//! bytes key for teacher   HPKE enc [32] || HPKE ciphertext of the answer key's key
//! bytes key for end code  empty, or seal(Argon2id(end code), answer key's key)
//! bytes key in clear      practice packages only: the answer key's key
//! bytes answer key     seal(answer key's key, archive of the full paper)
//! signature [64]       Ed25519 over every byte before it
//! ```
//! `seal` is XChaCha20-Poly1305 with a random nonce in front; its associated data is
//! SHA-256(header) followed by a label naming the field.

use hpke::aead::ChaCha20Poly1305;
use hpke::kdf::HkdfSha256;
use hpke::rand_core::CryptoRng;
use hpke::{Deserializable, OpModeR, OpModeS, Serializable};
use mcq_format::binary::{ByteReader, ByteWriter, Entry, decode_archive, encode_archive};
use mcq_model::Paper;
use zeroize::Zeroizing;

use crate::identity::{Identity, Kem, PublicIdentity};
use crate::primitives::{self, KEY_LEN, KdfParams, SALT_LEN, aad};
use crate::{Error, checked_passphrase, normalize_code};

const MAGIC: &[u8; 4] = b"MCQP";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 130;
const SIGNATURE_LEN: usize = 64;

const FLAG_OPEN: u8 = 1;
const FLAG_PASSPHRASE: u8 = 2;
const FLAG_END_CODE: u8 = 4;

const HPKE_INFO: &[u8] = b"mcq-advanced/answer-key/v1";

/// Name of the paper text inside package archives.
pub const PAPER_FILE: &str = "paper.qcm.md";

/// How students open the paper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opening {
    /// A generated code (see `generate_code`), given out when the exam starts.
    Code(String),
    /// A phrase chosen by the teacher, at least `MIN_PASSPHRASE_CHARS` characters.
    Passphrase(String),
    /// No secret: a practice paper, whose answers students can see right away.
    Open,
}

/// The opening kind recorded in a package, without the secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpeningKind {
    Code,
    Passphrase,
    Open,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishOptions {
    pub opening: Opening,
    /// A generated code the teacher reveals once every copy is in, so students can read the
    /// correction. Not allowed on practice packages, whose answers are already readable.
    pub end_code: Option<String>,
    pub kdf: KdfParams,
}

/// What anyone can read from a package once its signature is checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageHeader {
    pub paper_id: [u8; 16],
    pub teacher: PublicIdentity,
    pub opening: OpeningKind,
    pub has_end_code: bool,
    pub kdf: KdfParams,
    content_salt: [u8; SALT_LEN],
    end_salt: [u8; SALT_LEN],
}

/// What a student's player gets after opening a package.
#[derive(Debug, Clone, PartialEq)]
pub struct StudentPackage {
    pub header: PackageHeader,
    /// The student copy: no correct flag, no explanation, no rule.
    pub paper: Paper,
    pub media: Vec<Entry>,
}

/// Ways to open the answer key.
pub enum KeyUnlock<'a> {
    /// The teacher, any time.
    Teacher(&'a Identity),
    /// Anyone with the end code, once the teacher reveals it.
    EndCode(&'a str),
    /// Practice packages only.
    Practice,
}

/// Builds a signed package. `paper` is the full paper; `media` the files its statements use.
pub fn publish(
    identity: &Identity,
    paper: &Paper,
    media: &[Entry],
    options: &PublishOptions,
    rng: &mut impl CryptoRng,
) -> Result<Vec<u8>, Error> {
    let kdf = options.kdf.check()?;
    let (flags, opening_secret) = match &options.opening {
        Opening::Code(code) => (0, Some(normalize_code(code).ok_or(Error::BadCode)?)),
        Opening::Passphrase(phrase) => (
            FLAG_PASSPHRASE,
            Some(checked_passphrase(phrase)?.to_string()),
        ),
        Opening::Open => (FLAG_OPEN, None),
    };
    let end_secret = match (&options.end_code, &options.opening) {
        (None, _) => None,
        (Some(_), Opening::Open) => {
            return Err(Error::NotAllowed("an end code on a practice package"));
        }
        (Some(code), _) => Some(normalize_code(code).ok_or(Error::BadCode)?),
    };
    let flags = flags
        | if end_secret.is_some() {
            FLAG_END_CODE
        } else {
            0
        };
    let public = identity.public();

    let paper_id: [u8; 16] = primitives::random(rng);
    let content_salt: [u8; SALT_LEN] = primitives::random(rng);
    let end_salt: [u8; SALT_LEN] = primitives::random(rng);
    let mut w = ByteWriter::new();
    w.fixed(MAGIC)
        .u8(VERSION)
        .u8(flags)
        .fixed(&paper_id)
        .fixed(&public.sign)
        .fixed(&public.seal)
        .u32(kdf.memory_kib)
        .u32(kdf.iterations)
        .u32(kdf.parallelism)
        .fixed(&content_salt)
        .fixed(&end_salt);
    debug_assert_eq!(w.len(), HEADER_LEN);
    let header_hash = primitives::sha256(&[w.as_slice()]);

    // Author notes stay with the author: the end code opens the answer key to students.
    let mut paper = paper.clone();
    paper.notes.clear();
    for theme in &mut paper.themes {
        theme.notes.clear();
    }
    for question in &mut paper.questions {
        question.line = 0;
    }
    let mut content_files = vec![Entry {
        path: PAPER_FILE.to_string(),
        data: mcq_format::write(&mcq_format::student_view(&paper)).into_bytes(),
    }];
    content_files.extend(media.iter().cloned());
    let content = Zeroizing::new(encode_archive(&content_files));
    let answers = Zeroizing::new(encode_archive(&[Entry {
        path: PAPER_FILE.to_string(),
        data: mcq_format::write(&paper).into_bytes(),
    }]));

    let content_key = Zeroizing::new(primitives::random::<KEY_LEN>(rng));
    let answer_key = Zeroizing::new(primitives::random::<KEY_LEN>(rng));

    match &opening_secret {
        None => w.bytes(content_key.as_ref()),
        Some(secret) => {
            let kek = primitives::derive_key(secret.as_bytes(), &content_salt, kdf)?;
            let wrap = primitives::seal(
                &kek,
                content_key.as_ref(),
                &aad(&header_hash, "content key"),
                rng,
            )?;
            w.bytes(&wrap)
        }
    };
    let sealed_content =
        primitives::seal(&content_key, &content, &aad(&header_hash, "content"), rng)?;
    w.bytes(&sealed_content);

    let (encapped, wrapped) = hpke::single_shot_seal_with_rng::<ChaCha20Poly1305, HkdfSha256, Kem>(
        &OpModeS::Base,
        &public.seal_key()?,
        HPKE_INFO,
        answer_key.as_ref(),
        &header_hash,
        rng,
    )
    .map_err(|_| Error::Malformed("cannot seal for the teacher key"))?;
    let mut teacher_wrap = encapped.to_bytes().to_vec();
    teacher_wrap.extend_from_slice(&wrapped);
    w.bytes(&teacher_wrap);

    match &end_secret {
        None => w.bytes(&[]),
        Some(secret) => {
            let kek = primitives::derive_key(secret.as_bytes(), &end_salt, kdf)?;
            let wrap = primitives::seal(
                &kek,
                answer_key.as_ref(),
                &aad(&header_hash, "end key"),
                rng,
            )?;
            w.bytes(&wrap)
        }
    };
    if opening_secret.is_none() {
        w.bytes(answer_key.as_ref());
    } else {
        w.bytes(&[]);
    }
    let sealed_answers =
        primitives::seal(&answer_key, &answers, &aad(&header_hash, "answer key"), rng)?;
    w.bytes(&sealed_answers);

    let signature = identity.sign(w.as_slice());
    w.fixed(&signature);
    Ok(w.into_vec())
}

/// Checks the signature and reads the header. Compare `header.teacher.fingerprint()` with the
/// one the teacher announced: the signature only proves the package matches the key inside it.
pub fn inspect(package: &[u8]) -> Result<PackageHeader, Error> {
    Ok(Parsed::read(package)?.header)
}

/// Opens the paper with the code or phrase given at the start (ignored for practice packages).
pub fn open(package: &[u8], secret: &str) -> Result<StudentPackage, Error> {
    let parsed = Parsed::read(package)?;
    let header = parsed.header.clone();
    let content_key: Zeroizing<[u8; KEY_LEN]> = match header.opening {
        OpeningKind::Open => key_from(parsed.content_wrap)?,
        kind => {
            let secret = match kind {
                OpeningKind::Code => normalize_code(secret).ok_or(Error::WrongSecret)?,
                _ => secret.trim().to_string(),
            };
            let kek = primitives::derive_key(secret.as_bytes(), &header.content_salt, header.kdf)?;
            let raw =
                primitives::open(&kek, parsed.content_wrap, &aad(&parsed.hash, "content key"))?;
            key_from(&raw)?
        }
    };
    let content = primitives::open(&content_key, parsed.content, &aad(&parsed.hash, "content"))?;
    let mut files = decode_archive(&content)?;
    let index = files
        .iter()
        .position(|f| f.path == PAPER_FILE)
        .ok_or(Error::Malformed("no paper in the package"))?;
    let paper = read_paper(&files.remove(index).data)?;
    Ok(StudentPackage {
        header,
        paper,
        media: files,
    })
}

/// The full paper, with its answers.
pub fn open_answer_key(package: &[u8], unlock: KeyUnlock<'_>) -> Result<Paper, Error> {
    let parsed = Parsed::read(package)?;
    let answer_key: Zeroizing<[u8; KEY_LEN]> = match unlock {
        KeyUnlock::Teacher(identity) => {
            if identity.public() != parsed.header.teacher {
                return Err(Error::WrongSecret);
            }
            if parsed.teacher_wrap.len() < 32 {
                return Err(Error::Malformed("teacher key field"));
            }
            let (enc, ciphertext) = parsed.teacher_wrap.split_at(32);
            let encapped = <Kem as hpke::Kem>::EncappedKey::from_bytes(enc)
                .map_err(|_| Error::Malformed("teacher key field"))?;
            let raw = hpke::single_shot_open::<ChaCha20Poly1305, HkdfSha256, Kem>(
                &OpModeR::Base,
                &identity.seal_secret()?,
                &encapped,
                HPKE_INFO,
                ciphertext,
                &parsed.hash,
            )
            .map_err(|_| Error::WrongSecret)?;
            key_from(&Zeroizing::new(raw))?
        }
        KeyUnlock::EndCode(code) => {
            if !parsed.header.has_end_code {
                return Err(Error::NotAllowed("this package has no end code"));
            }
            let code = normalize_code(code).ok_or(Error::WrongSecret)?;
            let kek = primitives::derive_key(
                code.as_bytes(),
                &parsed.header.end_salt,
                parsed.header.kdf,
            )?;
            let raw = primitives::open(&kek, parsed.end_wrap, &aad(&parsed.hash, "end key"))?;
            key_from(&raw)?
        }
        KeyUnlock::Practice => {
            if parsed.header.opening != OpeningKind::Open {
                return Err(Error::NotAllowed(
                    "the answers of an exam package are not in clear",
                ));
            }
            key_from(parsed.clear_key)?
        }
    };
    let answers = primitives::open(
        &answer_key,
        parsed.answers,
        &aad(&parsed.hash, "answer key"),
    )?;
    let files = decode_archive(&answers)?;
    let paper = files
        .iter()
        .find(|f| f.path == PAPER_FILE)
        .ok_or(Error::Malformed("no paper in the answer key"))?;
    read_paper(&paper.data)
}

fn key_from(bytes: &[u8]) -> Result<Zeroizing<[u8; KEY_LEN]>, Error> {
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    if bytes.len() != KEY_LEN {
        return Err(Error::Malformed("key length"));
    }
    key.copy_from_slice(bytes);
    Ok(key)
}

fn read_paper(data: &[u8]) -> Result<Paper, Error> {
    let text = std::str::from_utf8(data).map_err(|_| Error::Malformed("paper is not UTF-8"))?;
    let (mut paper, issues) = mcq_format::read(text);
    if issues
        .iter()
        .any(|i| i.severity == mcq_model::Severity::Error)
    {
        return Err(Error::Malformed("paper text"));
    }
    // Lines of the packaged text mean nothing to the author's source file.
    for question in &mut paper.questions {
        question.line = 0;
    }
    Ok(paper)
}

/// A package whose signature has been checked, split into its fields.
struct Parsed<'a> {
    header: PackageHeader,
    hash: [u8; 32],
    content_wrap: &'a [u8],
    content: &'a [u8],
    teacher_wrap: &'a [u8],
    end_wrap: &'a [u8],
    clear_key: &'a [u8],
    answers: &'a [u8],
}

impl<'a> Parsed<'a> {
    fn read(package: &'a [u8]) -> Result<Self, Error> {
        if package.len() < 4 || &package[..4] != MAGIC {
            return Err(Error::WrongKind);
        }
        if package.len() < HEADER_LEN + SIGNATURE_LEN {
            return Err(Error::Malformed("truncated"));
        }
        let (signed, signature) = package.split_at(package.len() - SIGNATURE_LEN);
        let mut r = ByteReader::new(signed);
        r.take(4)?;
        let version = r.u8()?;
        if version != VERSION {
            return Err(Error::UnsupportedVersion(version));
        }
        let flags = r.u8()?;
        let paper_id = r.fixed()?;
        let teacher = PublicIdentity {
            sign: r.fixed()?,
            seal: r.fixed()?,
        };
        let mut signature_bytes = [0_u8; SIGNATURE_LEN];
        signature_bytes.copy_from_slice(signature);
        teacher.verify(signed, &signature_bytes)?;

        let kdf = KdfParams {
            memory_kib: r.u32()?,
            iterations: r.u32()?,
            parallelism: r.u32()?,
        }
        .check()?;
        let content_salt = r.fixed()?;
        let end_salt = r.fixed()?;
        let hash = primitives::sha256(&[&signed[..HEADER_LEN]]);

        if flags & !(FLAG_OPEN | FLAG_PASSPHRASE | FLAG_END_CODE) != 0 {
            return Err(Error::Malformed("unknown flags"));
        }
        let opening = match (flags & FLAG_OPEN != 0, flags & FLAG_PASSPHRASE != 0) {
            (false, false) => OpeningKind::Code,
            (false, true) => OpeningKind::Passphrase,
            (true, false) => OpeningKind::Open,
            (true, true) => return Err(Error::Malformed("contradictory flags")),
        };
        let has_end_code = flags & FLAG_END_CODE != 0;
        if has_end_code && opening == OpeningKind::Open {
            return Err(Error::Malformed("contradictory flags"));
        }

        let parsed = Self {
            header: PackageHeader {
                paper_id,
                teacher,
                opening,
                has_end_code,
                kdf,
                content_salt,
                end_salt,
            },
            hash,
            content_wrap: r.bytes()?,
            content: r.bytes()?,
            teacher_wrap: r.bytes()?,
            end_wrap: r.bytes()?,
            clear_key: r.bytes()?,
            answers: r.bytes()?,
        };
        r.finish()?;
        Ok(parsed)
    }
}
