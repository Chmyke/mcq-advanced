//! A student's revision sheet: where they stand per theme, each mistake with the right answer and
//! why, and where to start. The teacher prints it or hands it out once the exam is over.

use mcq_grade::Mastery;
use mcq_model::{Paper, Question};

use crate::{Copy, Finding, MIN_THEME_QUESTIONS, Report, StudentResult};

/// At most this many themes in "where to start".
pub const START_WITH: usize = 3;

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeLine {
    pub title: String,
    /// From 0 to 1.
    pub rate: f32,
    pub mastery: Mastery,
    /// Fewer than `MIN_THEME_QUESTIONS` questions: too few to conclude.
    pub too_few: bool,
    /// Where to revise the theme, as the teacher wrote it in the settings; may be empty.
    pub course_ref: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mistake {
    /// The question from the answer key: statement, options, explanations, rule.
    pub question: Question,
    /// Option ids the student ticked; empty for a blank.
    pub chosen: Vec<String>,
    /// The analysis doubts the key: the sheet must not assert the answer before the teacher
    /// has checked it.
    pub key_in_doubt: bool,
    /// Where to revise: the question's own reference, otherwise its theme's (the player's
    /// rule); empty when neither has one.
    pub revise: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RevisionSheet {
    pub name: String,
    pub paper_title: String,
    pub mark: f32,
    pub overtime_seconds: u32,
    pub themes: Vec<ThemeLine>,
    /// In the order the student met them.
    pub mistakes: Vec<Mistake>,
    /// The weakest fragile or to-relearn themes, weakest first, each with where to revise it.
    pub start_with: Vec<ThemeLine>,
}

/// The sheet of one student; `report` tells which keys are in doubt and which the teacher
/// corrected, so the sheet shows the key the copies were marked with.
pub fn revision_sheet(
    paper: &Paper,
    report: &Report,
    student: &StudentResult,
    copy: &Copy,
) -> RevisionSheet {
    let (key, _) = crate::decision::apply(paper, &report.adjustments);
    let paper = &key;
    let themes: Vec<ThemeLine> = student
        .grade
        .themes
        .iter()
        .filter(|t| t.total > 0)
        .map(|t| {
            let theme = paper.theme(&t.theme_id);
            ThemeLine {
                title: theme.map_or_else(|| t.theme_id.clone(), |th| th.title.clone()),
                rate: t.rate(),
                mastery: t.mastery(),
                too_few: (t.total as usize) < MIN_THEME_QUESTIONS,
                course_ref: theme.map(|th| th.course_ref.clone()).unwrap_or_default(),
            }
        })
        .collect();
    let mut weak: Vec<&ThemeLine> = themes
        .iter()
        .filter(|t| matches!(t.mastery, Mastery::Fragile | Mastery::ToRelearn))
        .collect();
    weak.sort_by(|a, b| a.rate.total_cmp(&b.rate));
    let start_with = weak.into_iter().take(START_WITH).cloned().collect();
    let mistakes = student
        .grade
        .to_review()
        .filter_map(|outcome| {
            let question = paper.question(&outcome.question_id)?.clone();
            let chosen = copy
                .submission
                .answers
                .iter()
                .find(|a| a.question_id == outcome.question_id)
                .map(|a| a.selected.clone())
                .unwrap_or_default();
            let key_in_doubt = report
                .findings
                .iter()
                .any(|f| matches!(f, Finding::KeyDoubt { question: id, .. } if *id == question.id));
            let revise = if question.course_ref.trim().is_empty() {
                paper
                    .theme(&question.theme_id)
                    .map(|th| th.course_ref.clone())
                    .unwrap_or_default()
            } else {
                question.course_ref.clone()
            };
            Some(Mistake {
                question,
                chosen,
                key_in_doubt,
                revise,
            })
        })
        .collect();
    RevisionSheet {
        name: student.name.clone(),
        paper_title: paper.title.clone(),
        mark: student.mark,
        overtime_seconds: student.grade.overtime_seconds,
        themes,
        mistakes,
        start_with,
    }
}
