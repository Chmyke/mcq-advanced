//! ROLE:    turns a checked paper into what a class needs: the signed package, its codes, and the
//!          sheet for the exam room. Shared by the CLI and the Studio.
//! OWNER:   stateless; the caller provides the identity, the media files and the randomness.
//! INVARIANTS: a paper with errors is never published; code is coloured once, here; the room
//!             sheet never shows the end code, which is revealed only after hand-in.

use mcq_crypto::rand_core::CryptoRng;
use mcq_crypto::{Identity, KdfParams, Opening, PublishOptions};
use mcq_format::binary::{CODE_TOKENS_FILE, Entry, encode_code_tokens};
use mcq_model::{Issue, Paper, Severity};

/// How students will take the paper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Opened with a generated code; `end_code` adds a code that reveals the correction later.
    Exam { end_code: bool },
    /// No code: answers readable by students, for training.
    Practice,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PublishError {
    /// The paper has errors: fix them first.
    HasErrors(Vec<Issue>),
    /// A media file the paper uses cannot be read.
    Media {
        path: String,
        reason: String,
    },
    Crypto(mcq_crypto::Error),
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublishError::HasErrors(issues) => write!(
                f,
                "the paper has {} error(s): fix them before publishing",
                issues.len()
            ),
            PublishError::Media { path, reason } => write!(f, "cannot read {path}: {reason}"),
            PublishError::Crypto(e) => e.fmt(f),
        }
    }
}

/// A published paper and what the teacher must keep or hand out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
    pub package: Vec<u8>,
    pub paper_id: [u8; 16],
    /// Written on the board when the exam starts; `None` for practice.
    pub opening_code: Option<String>,
    /// Revealed once every copy is in, so students can read the correction.
    pub end_code: Option<String>,
    /// What students compare before trusting the paper.
    pub fingerprint: String,
}

/// The errors that block publishing, as the linter finds them.
pub fn blocking_issues(paper: &Paper) -> Vec<Issue> {
    mcq_lint::lint(paper)
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .collect()
}

/// The files a package carries besides the paper: each media file once, and the coloured code.
/// `read_media` gives the bytes of a path relative to the paper.
pub fn package_files(
    paper: &Paper,
    mut read_media: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<Vec<Entry>, PublishError> {
    let mut files: Vec<Entry> = Vec::new();
    for media in paper.questions.iter().flat_map(|q| q.media()) {
        if files.iter().any(|f| f.path == media.path) {
            continue;
        }
        let data = read_media(&media.path).map_err(|reason| PublishError::Media {
            path: media.path.clone(),
            reason,
        })?;
        files.push(Entry {
            path: media.path.clone(),
            data,
        });
    }
    // Code is coloured once, here, so players only map token kinds to theme colours.
    let tokens = mcq_highlight::Highlighter::new().paper(paper);
    if !tokens.is_empty() {
        files.push(Entry {
            path: CODE_TOKENS_FILE.to_string(),
            data: encode_code_tokens(&tokens),
        });
    }
    Ok(files)
}

/// Checks the paper, generates the codes and builds the signed package.
pub fn publish(
    identity: &Identity,
    paper: &Paper,
    read_media: impl FnMut(&str) -> Result<Vec<u8>, String>,
    mode: Mode,
    kdf: KdfParams,
    rng: &mut impl CryptoRng,
) -> Result<Published, PublishError> {
    let errors = blocking_issues(paper);
    if !errors.is_empty() {
        return Err(PublishError::HasErrors(errors));
    }
    let files = package_files(paper, read_media)?;
    let (opening, end_code) = match mode {
        Mode::Practice => (Opening::Open, None),
        Mode::Exam { end_code } => (
            Opening::Code(mcq_crypto::generate_code(rng)),
            end_code.then(|| mcq_crypto::generate_code(rng)),
        ),
    };
    let options = PublishOptions {
        opening: opening.clone(),
        end_code: end_code.clone(),
        kdf,
    };
    let package = mcq_crypto::publish(identity, paper, &files, &options, rng)
        .map_err(PublishError::Crypto)?;
    let header = mcq_crypto::inspect(&package).map_err(PublishError::Crypto)?;
    Ok(Published {
        package,
        paper_id: header.paper_id,
        opening_code: match opening {
            Opening::Code(code) => Some(code),
            _ => None,
        },
        end_code,
        fingerprint: identity.public().fingerprint(),
    })
}

/// What the room sheet shows: everything students need to start, nothing they must not see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomSheet {
    pub title: String,
    pub question_count: usize,
    /// 0 for no time limit.
    pub duration_minutes: u32,
    pub opening_code: Option<String>,
    pub fingerprint: String,
    /// The correction opens later with a code the teacher reveals.
    pub correction_later: bool,
}

impl RoomSheet {
    pub fn new(paper: &Paper, published: &Published) -> Self {
        Self {
            title: paper.title.clone(),
            question_count: paper.questions.len(),
            duration_minutes: paper.duration_minutes,
            opening_code: published.opening_code.clone(),
            fingerprint: published.fingerprint.clone(),
            correction_later: published.end_code.is_some(),
        }
    }
}
