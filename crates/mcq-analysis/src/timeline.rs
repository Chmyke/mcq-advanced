//! What happened during one copy, minute by minute, from the journal the player keeps: answers
//! ticked and changed, exits from the exam window, hand-in. Facts for a conversation with the
//! student, never a verdict.

use mcq_grade::Status;
use mcq_model::{LogEntry, LogKind, Paper, Submission};

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The student settled on an answer to a question (single answers: the click that moved the
    /// tick; multiple answers: each box). `status` is how the answer, as it then stood, would be
    /// graded: right, partly right (some credit under partial scoring) or wrong.
    Answer {
        /// 1-based, in the paper's order (the teacher's numbering), never the shuffled order the
        /// student saw; 0 for a question missing from the key.
        question: usize,
        question_id: String,
        /// Option ids ticked once this moment is over.
        selected: Vec<String>,
        status: Status,
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

/// How the answer would be graded, with the same rule as the marks.
fn graded(paper: &Paper, question_id: &str, selected: &[String]) -> Status {
    paper.question(question_id).map_or(Status::Wrong, |q| {
        mcq_grade::score(q, selected, paper.scoring).0
    })
}

/// The end (exclusive) of the click whose journal entries start at `start`, a Tick or an Untick.
///
/// One click logs one entry, except on a single-answer question where ticking an option first
/// unticks the one held: the player logs those Unticks then the Tick, in the same second. That
/// pair is one step; anything else is a click of its own, even within the same second, so an
/// answer changed at once still counts as a change. A clear followed by a new tick in the same
/// second logs the same entries and reads as one step too: same outcome, without the blank.
fn click_end(paper: &Paper, log: &[LogEntry], start: usize) -> usize {
    let first = &log[start];
    let multiple = paper
        .question(&first.question_id)
        .is_some_and(|q| q.is_multiple());
    if first.kind == LogKind::Tick || multiple {
        return start + 1;
    }
    let same_click =
        |e: &LogEntry| e.seconds == first.seconds && e.question_id == first.question_id;
    let mut end = start + 1;
    while end < log.len() && log[end].kind == LogKind::Untick && same_click(&log[end]) {
        end += 1;
    }
    match log.get(end) {
        // The ticked option is never one the same click unticked.
        Some(e)
            if e.kind == LogKind::Tick
                && same_click(e)
                && !log[start..end].iter().any(|u| u.option_id == e.option_id) =>
        {
            end + 1
        }
        _ => end,
    }
}

/// The timeline of a copy, against the answer key in force.
pub fn timeline(paper: &Paper, copy: &Submission) -> Timeline {
    let mut out = Timeline {
        elapsed_seconds: copy.elapsed_seconds,
        ..Timeline::default()
    };
    let number = |id: &str| {
        paper
            .questions
            .iter()
            .position(|q| q.id == id)
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
                let slot = match held.iter().position(|(q, _)| *q == entry.question_id) {
                    Some(slot) => slot,
                    None => {
                        held.push((entry.question_id.clone(), Vec::new()));
                        held.len() - 1
                    }
                };
                let before = held[slot].1.clone();
                let j = click_end(paper, log, i);
                for e in &log[i..j] {
                    let selected = &mut held[slot].1;
                    if e.kind == LogKind::Tick {
                        if !selected.contains(&e.option_id) {
                            selected.push(e.option_id.clone());
                        }
                    } else {
                        selected.retain(|o| *o != e.option_id);
                    }
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
                    let status = graded(paper, &question_id, &after);
                    // A change is judged on entirely right answers: partly right counts as wrong.
                    let right = status == Status::Correct;
                    let change = (!before.is_empty()).then(|| {
                        let was_right = graded(paper, &question_id, &before) == Status::Correct;
                        match (was_right, right) {
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
                        status,
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
