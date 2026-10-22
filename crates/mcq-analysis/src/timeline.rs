//! What happened during one copy, minute by minute, from the journal the player keeps: answers
//! ticked and changed, exits from the exam window, hand-in. Facts for a conversation with the
//! student, never a verdict.

use mcq_grade::Status;
use mcq_model::{LogKind, Paper, Submission};

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The student settled on an answer to a question (single answers: the click that moved the
    /// tick; multiple answers: each box). `right` tells whether the answer, as it then stood,
    /// was entirely right.
    Answer {
        /// 1-based, in the student's order.
        question: usize,
        question_id: String,
        /// Option ids ticked once this moment is over.
        selected: Vec<String>,
        right: bool,
        /// The answer replaced one that was already given.
        change: Option<Change>,
    },
    /// The student cleared the question.
    Cleared {
        question: usize,
        question_id: String,
    },
    /// Left the exam window for `seconds` (0 when the copy was handed in before coming back).
    Away {
        seconds: u32,
    },
    Submitted,
}

/// How a given answer was replaced by another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    RightToWrong,
    WrongToRight,
    WrongToWrong,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Moment {
    /// Exam seconds elapsed.
    pub seconds: u32,
    pub event: Event,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Timeline {
    pub moments: Vec<Moment>,
    pub elapsed_seconds: u32,
    pub right_to_wrong: u32,
    pub wrong_to_right: u32,
    pub wrong_to_wrong: u32,
    pub away_count: u32,
    pub away_seconds: u32,
}

impl Timeline {
    pub fn changes(&self) -> u32 {
        self.right_to_wrong + self.wrong_to_right + self.wrong_to_wrong
    }
}

fn is_right(paper: &Paper, question_id: &str, selected: &[String]) -> bool {
    paper
        .question(question_id)
        .is_some_and(|q| mcq_grade::score(q, selected, paper.scoring).0 == Status::Correct)
}

/// The timeline of a copy, against the answer key in force.
pub fn timeline(paper: &Paper, copy: &Submission) -> Timeline {
    let mut out = Timeline {
        elapsed_seconds: copy.elapsed_seconds,
        ..Timeline::default()
    };
    let number = |id: &str| {
        copy.answers
            .iter()
            .position(|a| a.question_id == id)
            .map_or(0, |i| i + 1)
    };
    // What each question holds as the journal is replayed.
    let mut held: Vec<(String, Vec<String>)> = Vec::new();
    let mut away_since: Option<u32> = None;
    let log = &copy.log;
    let mut i = 0;
    while i < log.len() {
        let entry = &log[i];
        match entry.kind {
            LogKind::Tick | LogKind::Untick => {
                // One click may untick and tick in the same second: replay them together.
                let mut j = i;
                let slot = match held.iter().position(|(q, _)| *q == entry.question_id) {
                    Some(slot) => slot,
                    None => {
                        held.push((entry.question_id.clone(), Vec::new()));
                        held.len() - 1
                    }
                };
                let before = held[slot].1.clone();
                while j < log.len()
                    && matches!(log[j].kind, LogKind::Tick | LogKind::Untick)
                    && log[j].seconds == entry.seconds
                    && log[j].question_id == entry.question_id
                {
                    let selected = &mut held[slot].1;
                    if log[j].kind == LogKind::Tick {
                        if !selected.contains(&log[j].option_id) {
                            selected.push(log[j].option_id.clone());
                        }
                    } else {
                        selected.retain(|o| *o != log[j].option_id);
                    }
                    j += 1;
                }
                let after = held[slot].1.clone();
                i = j;
                if after == before {
                    continue;
                }
                let question = number(&entry.question_id);
                let question_id = entry.question_id.clone();
                let event = if after.is_empty() {
                    Event::Cleared {
                        question,
                        question_id,
                    }
                } else {
                    let right = is_right(paper, &question_id, &after);
                    let change = (!before.is_empty()).then(|| {
                        match (is_right(paper, &question_id, &before), right) {
                            (true, _) => Change::RightToWrong,
                            (false, true) => Change::WrongToRight,
                            (false, false) => Change::WrongToWrong,
                        }
                    });
                    match change {
                        Some(Change::RightToWrong) => out.right_to_wrong += 1,
                        Some(Change::WrongToRight) => out.wrong_to_right += 1,
                        Some(Change::WrongToWrong) => out.wrong_to_wrong += 1,
                        None => {}
                    }
                    Event::Answer {
                        question,
                        question_id,
                        selected: after,
                        right,
                        change,
                    }
                };
                out.moments.push(Moment {
                    seconds: entry.seconds,
                    event,
                });
                continue;
            }
            LogKind::FocusLost => {
                away_since.get_or_insert(entry.seconds);
            }
            LogKind::FocusBack => {
                if let Some(start) = away_since.take() {
                    let seconds = entry.seconds.saturating_sub(start);
                    out.away_count += 1;
                    out.away_seconds += seconds;
                    out.moments.push(Moment {
                        seconds: start,
                        event: Event::Away { seconds },
                    });
                }
            }
            LogKind::Submit => {
                if let Some(start) = away_since.take() {
                    out.away_count += 1;
                    out.moments.push(Moment {
                        seconds: start,
                        event: Event::Away { seconds: 0 },
                    });
                }
                out.moments.push(Moment {
                    seconds: entry.seconds,
                    event: Event::Submitted,
                });
            }
        }
        i += 1;
    }
    out.moments.sort_by_key(|m| m.seconds);
    out
}
