//! ROLE:    the class's results beside a question in the Write space, once copies are in the
//!          Correct space: how many got it, how many left it blank, which answers were ticked.
//! OWNER:   pure functions over the Correct space's report and the published key; no screen.
//! INVARIANTS: answers are matched by their text in the published key, not by position, so
//!             reordering or rewording answers in the editor never puts a figure on the wrong
//!             answer (a reworded answer simply shows none).

use mcq_analysis::Report;
use mcq_model::{Paper, Question};

use crate::correction::Correction;

/// The analysis of the copies and the key they were answered against.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassResults {
    pub report: Report,
    pub key: Paper,
}

impl ClassResults {
    /// The results of the Correct space, when a package and copies are in.
    pub fn from_correction(correction: &Correction) -> Option<Self> {
        Some(Self {
            report: correction.report()?,
            key: correction.package.as_ref()?.key.clone(),
        })
    }
}

/// One answer of the question as written now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnswerShare {
    pub picked: u32,
    /// Share of the answers given (blanks aside), in whole percents.
    pub percent: u32,
    /// Correct in the key the copies were marked against.
    pub correct: bool,
}

/// The class's figures for one question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionResults {
    pub asked: u32,
    pub blank: u32,
    /// Share of the class who got it, in whole percents.
    pub percent: u32,
    pub mean_seconds: u32,
    /// Few copies: a tendency to confirm, not a measure.
    pub small_cohort: bool,
    /// Per answer of the question, in the editor's order; `None` for an answer the key lacks.
    pub answers: Vec<Option<AnswerShare>>,
}

fn percent(share: f64) -> u32 {
    (share * 100.0).round().clamp(0.0, 100.0) as u32
}

fn same_text(a: &str, b: &str) -> bool {
    a.split_whitespace().eq(b.split_whitespace())
}

/// The figures of a question the copies answered, or `None` when none of them saw it.
pub fn for_question(results: &ClassResults, question: &Question) -> Option<QuestionResults> {
    let stats = results
        .report
        .questions
        .iter()
        .find(|q| q.id == question.id && q.asked > 0)?;
    let published = results.key.question(&question.id)?;
    let answers = question
        .options
        .iter()
        .map(|option| {
            let in_key = published
                .options
                .iter()
                .find(|o| same_text(&o.text, &option.text))?;
            let share = stats.options.iter().find(|s| s.id == in_key.id)?;
            Some(AnswerShare {
                picked: share.picked,
                percent: percent(share.rate),
                correct: share.is_correct,
            })
        })
        .collect();
    Some(QuestionResults {
        asked: stats.asked,
        blank: stats.blank,
        percent: percent(stats.facility),
        mean_seconds: stats.mean_seconds.round() as u32,
        small_cohort: results.report.small_cohort,
        answers,
    })
}
