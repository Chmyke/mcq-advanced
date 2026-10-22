//! The teacher's verdict on a question whose key is in doubt, applied before grading: the
//! question leaves the grading (every copy is marked on the others), or its key is replaced by
//! the options that were right after all. Decisions are data: undoing one is dropping it.

use mcq_model::Paper;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Out of the grading: a paper of N questions is marked on N-1.
    Neutralize,
    /// These options are the right ones, in place of the key.
    Rekey(Vec<String>),
}

/// A decision about one question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adjustment {
    pub question: String,
    pub decision: Decision,
}

impl Adjustment {
    pub fn neutralize(question: &str) -> Self {
        Self {
            question: question.to_string(),
            decision: Decision::Neutralize,
        }
    }

    pub fn rekey(question: &str, correct: &[&str]) -> Self {
        Self {
            question: question.to_string(),
            decision: Decision::Rekey(correct.iter().map(|id| id.to_string()).collect()),
        }
    }
}

/// The answer key once the decisions are applied, with the decisions that took effect. A
/// decision is dropped when its question is unknown, when a later one decides the same question,
/// or when a new key is empty, names an unknown option, or equals the current key.
pub fn apply(paper: &Paper, adjustments: &[Adjustment]) -> (Paper, Vec<Adjustment>) {
    let mut key = paper.clone();
    let mut applied: Vec<Adjustment> = Vec::new();
    for (i, adjustment) in adjustments.iter().enumerate() {
        if adjustments[i + 1..]
            .iter()
            .any(|later| later.question == adjustment.question)
        {
            continue;
        }
        let Some(question) = key
            .questions
            .iter_mut()
            .find(|q| q.id == adjustment.question)
        else {
            continue;
        };
        match &adjustment.decision {
            Decision::Neutralize => {}
            Decision::Rekey(correct) => {
                let known = correct.iter().all(|id| question.option(id).is_some());
                let same = question
                    .options
                    .iter()
                    .all(|o| o.is_correct == correct.contains(&o.id));
                if correct.is_empty() || !known || same {
                    continue;
                }
                for option in &mut question.options {
                    option.is_correct = correct.contains(&option.id);
                }
            }
        }
        applied.push(adjustment.clone());
    }
    (key, applied)
}

/// True when the question is out of the grading.
pub fn is_neutralized(adjustments: &[Adjustment], question: &str) -> bool {
    adjustments
        .iter()
        .any(|a| a.question == question && a.decision == Decision::Neutralize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcq_model::{AnswerOption, Question};

    fn paper() -> Paper {
        let mut paper = Paper::new();
        paper.questions.push(Question {
            id: "q1".into(),
            options: ["a", "b", "c"]
                .iter()
                .map(|id| AnswerOption {
                    id: (*id).into(),
                    is_correct: *id == "a",
                    ..AnswerOption::default()
                })
                .collect(),
            ..Question::default()
        });
        paper
    }

    fn correct(paper: &Paper) -> Vec<String> {
        paper.questions[0]
            .options
            .iter()
            .filter(|o| o.is_correct)
            .map(|o| o.id.clone())
            .collect()
    }

    #[test]
    fn a_new_key_replaces_the_old_one() {
        let (key, applied) = apply(&paper(), &[Adjustment::rekey("q1", &["b"])]);
        assert_eq!(correct(&key), vec!["b"]);
        assert_eq!(applied.len(), 1);
    }

    #[test]
    fn the_last_decision_on_a_question_wins() {
        let decisions = [
            Adjustment::rekey("q1", &["b"]),
            Adjustment::neutralize("q1"),
        ];
        let (key, applied) = apply(&paper(), &decisions);
        assert_eq!(correct(&key), vec!["a"]);
        assert_eq!(applied, vec![Adjustment::neutralize("q1")]);
    }

    #[test]
    fn decisions_that_change_nothing_are_dropped() {
        for useless in [
            Adjustment::neutralize("q9"),
            Adjustment::rekey("q1", &[]),
            Adjustment::rekey("q1", &["z"]),
            Adjustment::rekey("q1", &["a"]),
        ] {
            let (key, applied) = apply(&paper(), std::slice::from_ref(&useless));
            assert_eq!(key, paper(), "{useless:?}");
            assert!(applied.is_empty(), "{useless:?}");
        }
    }
}
