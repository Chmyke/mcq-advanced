//! The Correct space's state, without any screen: the package and its answer key, the copies
//! opened with the teacher's identity, files waiting for that identity, and files refused.

use mcq_analysis::sheet::{RevisionSheet, revision_sheet};
use mcq_analysis::timeline::{Timeline, timeline};
use mcq_analysis::{Adjustment, Copy, Report, analyse_with};
use mcq_crypto::{Error, Identity, KeyUnlock};
use mcq_model::Paper;
use mcq_player::b64;

/// A published package with its answer key, opened by its teacher.
#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub file_name: String,
    pub paper_id: [u8; 16],
    pub key: Paper,
}

/// The last package published from this browser, kept so that Correct opens on it without asking
/// for the file again. The package is the published file itself: its answer key still opens with
/// the teacher's identity only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptPaper {
    pub file_name: String,
    pub title: String,
    pub package: Vec<u8>,
}

impl KeptPaper {
    /// One line per field, each in base 64: a title may hold any character.
    pub fn encode(&self) -> String {
        [
            self.file_name.as_bytes(),
            self.title.as_bytes(),
            self.package.as_slice(),
        ]
        .map(b64::encode)
        .join("\n")
    }

    /// What `encode` gave; `None` for anything else, so a damaged record is simply ignored.
    pub fn decode(text: &str) -> Option<Self> {
        let mut fields = text.split('\n').map(b64::decode);
        let file_name = String::from_utf8(fields.next()??).ok()?;
        let title = String::from_utf8(fields.next()??).ok()?;
        let package = fields.next()??;
        (fields.next().is_none() && is_package(&package)).then_some(Self {
            file_name,
            title,
            package,
        })
    }
}

/// Why a file was not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Neither a package nor a copy.
    Unknown,
    /// A package or a copy for another teacher's identity.
    NotYours,
    /// The same copy, already in.
    AlreadyIn,
    /// A damaged or forged file.
    Damaged,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Correction {
    pub package: Option<Package>,
    pub copies: Vec<Copy>,
    /// Files received before the identity was unlocked, opened as soon as it is.
    pending: Vec<(String, Vec<u8>)>,
    pub refused: Vec<(String, Refusal)>,
    /// The teacher's decisions on doubtful keys, applied at once to every mark.
    pub adjustments: Vec<Adjustment>,
}

impl Correction {
    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    /// Takes a dropped or chosen file: a package (`.mcqpack`) or a copy (`.mcqres`).
    pub fn add_file(&mut self, name: &str, bytes: Vec<u8>, identity: Option<&Identity>) {
        match identity {
            Some(identity) => self.open(name, bytes, identity),
            None if is_package(&bytes) || is_copy(&bytes) => {
                self.pending.push((name.to_string(), bytes));
            }
            None => self.refused.push((name.to_string(), Refusal::Unknown)),
        }
    }

    /// A package waits for the identity.
    pub fn package_pending(&self) -> bool {
        self.pending.iter().any(|(_, bytes)| is_package(bytes))
    }

    /// Takes the last published paper by default: only when no package is in or on its way, and
    /// not again once it was refused (another teacher's identity), so nothing piles up.
    pub fn take_kept(&mut self, kept: &KeptPaper, identity: Option<&Identity>) {
        let refused = self.refused.iter().any(|(name, _)| *name == kept.file_name);
        if self.package.is_none() && !self.package_pending() && !refused {
            self.add_file(&kept.file_name, kept.package.clone(), identity);
        }
    }

    /// True when the paper in is the kept one (same published paper).
    pub fn holds(&self, kept: &KeptPaper) -> bool {
        let paper_id = mcq_crypto::inspect(&kept.package).map(|h| h.paper_id);
        self.package
            .as_ref()
            .is_some_and(|p| paper_id.is_ok_and(|id| id == p.paper_id))
    }

    /// Opens the files that were waiting for the identity.
    pub fn open_pending(&mut self, identity: &Identity) {
        for (name, bytes) in std::mem::take(&mut self.pending) {
            self.open(&name, bytes, identity);
        }
    }

    fn open(&mut self, name: &str, bytes: Vec<u8>, identity: &Identity) {
        let refusal = |e: Error| match e {
            Error::WrongSecret => Refusal::NotYours,
            Error::WrongKind => Refusal::Unknown,
            _ => Refusal::Damaged,
        };
        if is_package(&bytes) {
            let opened = mcq_crypto::inspect(&bytes).and_then(|header| {
                mcq_crypto::open_answer_key(&bytes, KeyUnlock::Teacher(identity))
                    .map(|key| (header, key))
            });
            match opened {
                Ok((header, key)) => {
                    self.package = Some(Package {
                        file_name: name.to_string(),
                        paper_id: header.paper_id,
                        key,
                    });
                }
                Err(e) => self.refused.push((name.to_string(), refusal(e))),
            }
        } else if is_copy(&bytes) {
            match mcq_crypto::open_result(&bytes, identity) {
                Ok((_, copy_id)) if self.copies.iter().any(|c| c.copy_id == copy_id) => {
                    self.refused.push((name.to_string(), Refusal::AlreadyIn));
                }
                Ok((submission, copy_id)) => self.copies.push(Copy {
                    submission,
                    copy_id,
                }),
                Err(e) => self.refused.push((name.to_string(), refusal(e))),
            }
        } else {
            self.refused.push((name.to_string(), Refusal::Unknown));
        }
    }

    /// The analysis, once a package and at least one copy are in.
    pub fn report(&self) -> Option<Report> {
        let package = self.package.as_ref()?;
        (!self.copies.is_empty()).then(|| {
            analyse_with(
                &package.key,
                package.paper_id,
                &self.copies,
                &self.adjustments,
            )
        })
    }

    /// Records a decision about a question, replacing any earlier one on it.
    pub fn decide(&mut self, adjustment: Adjustment) {
        self.undo(&adjustment.question);
        self.adjustments.push(adjustment);
    }

    /// Drops the decision about a question: it is graded with its key again.
    pub fn undo(&mut self, question: &str) {
        self.adjustments.retain(|a| a.question != question);
    }

    /// The revision sheet of the student whose copy has this id.
    pub fn sheet(&self, report: &Report, copy_id: &str) -> Option<RevisionSheet> {
        let package = self.package.as_ref()?;
        let student = report.students.iter().find(|s| s.copy_id == copy_id)?;
        let copy = self.copies.iter().find(|c| c.copy_id == copy_id)?;
        Some(revision_sheet(&package.key, report, student, copy))
    }

    /// The timeline of the copy with this copy_id, judged with the key in force.
    pub fn timeline(&self, report: &Report, copy_id: &str) -> Option<(String, Timeline)> {
        let package = self.package.as_ref()?;
        let copy = self.copies.iter().find(|c| c.copy_id == copy_id)?;
        let (key, _) = mcq_analysis::decision::apply(&package.key, &report.adjustments);
        Some((
            copy.submission.student_name.clone(),
            timeline(&key, &copy.submission),
        ))
    }
}

fn is_package(bytes: &[u8]) -> bool {
    bytes.starts_with(b"MCQP")
}

fn is_copy(bytes: &[u8]) -> bool {
    bytes.starts_with(b"MCQR")
}

/// True for the files the Correct space takes.
pub fn is_correction_file(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.ends_with(".mcqpack") || name.ends_with(".mcqres")
}
