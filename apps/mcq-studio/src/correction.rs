//! The Correct space's state, without any screen: the package and its answer key, the copies
//! opened with the teacher's identity, files waiting for that identity, and files refused.

use mcq_analysis::sheet::{RevisionSheet, revision_sheet};
use mcq_analysis::timeline::{Timeline, timeline};
use mcq_analysis::{Adjustment, Copy, Report, analyse_with};
use mcq_crypto::{Error, Identity, KeyUnlock};
use mcq_model::Paper;

/// A published package with its answer key, opened by its teacher.
#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub file_name: String,
    pub paper_id: [u8; 16],
    pub key: Paper,
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
                Ok((_, receipt)) if self.copies.iter().any(|c| c.receipt == receipt) => {
                    self.refused.push((name.to_string(), Refusal::AlreadyIn));
                }
                Ok((submission, receipt)) => self.copies.push(Copy {
                    submission,
                    receipt,
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

    /// The revision sheet of the student whose copy has this receipt.
    pub fn sheet(&self, report: &Report, receipt: &str) -> Option<RevisionSheet> {
        let package = self.package.as_ref()?;
        let student = report.students.iter().find(|s| s.receipt == receipt)?;
        let copy = self.copies.iter().find(|c| c.receipt == receipt)?;
        Some(revision_sheet(&package.key, report, student, copy))
    }

    /// The timeline of the copy with this receipt, judged with the key in force.
    pub fn timeline(&self, report: &Report, receipt: &str) -> Option<(String, Timeline)> {
        let package = self.package.as_ref()?;
        let copy = self.copies.iter().find(|c| c.receipt == receipt)?;
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
