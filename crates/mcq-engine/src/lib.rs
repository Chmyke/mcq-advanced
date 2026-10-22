//! ROLE:    runs one student's exam: per-student order, ticking, navigation, time, autosave, submission.
//! OWNER:   the player owns one `Session`; the paper is passed in, never stored, so the session
//!          stays small enough to autosave after every click.
//! INVARIANTS: deterministic: the same paper id and name always give the same order;
//!             a submitted session never changes; time never blocks anything.

mod order;
mod save;
mod time;

pub use save::{decode_session, encode_session};
pub use time::{
    PACE_ALERT_BUDGETS, PACE_REARM_BUDGETS, PaceGauge, TimeStatus, WARNING_SECONDS, format_duration,
};

use mcq_model::{Answer, LogEntry, LogKind, Paper, Submission};

/// One question as this student sees it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionItem {
    pub question_id: String,
    /// Option ids in display order.
    pub option_order: Vec<String>,
    pub selected: Vec<String>,
    /// Marked "to review" by the student.
    pub flagged: bool,
    pub seconds_spent: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Session {
    pub paper_id: [u8; 16],
    pub student_name: String,
    pub items: Vec<SessionItem>,
    pub current: usize,
    pub elapsed_seconds: u32,
    pub submitted: bool,
    pub focus_losses: u32,
    pub seconds_away: u32,
    /// Every answer change and focus event, for the teacher's anomaly report.
    pub log: Vec<LogEntry>,
    /// The pace alert fired and has not been re-armed yet.
    pub pace_alerted: bool,
}

impl Session {
    /// A fresh session: questions and options ordered for this student.
    pub fn new(paper: &Paper, paper_id: [u8; 16], student_name: &str) -> Self {
        let student_name = normalize_name(student_name);
        let mut rng = order::Rng::seeded(&paper_id, &student_name);
        let items = order::questions(paper, &mut rng)
            .into_iter()
            .map(|question| {
                let mut option_order: Vec<String> =
                    question.options.iter().map(|o| o.id.clone()).collect();
                if paper.shuffle_options {
                    rng.shuffle(&mut option_order);
                }
                SessionItem {
                    question_id: question.id.clone(),
                    option_order,
                    ..SessionItem::default()
                }
            })
            .collect();
        Self {
            paper_id,
            student_name,
            items,
            ..Self::default()
        }
    }

    pub fn current_item(&self) -> Option<&SessionItem> {
        self.items.get(self.current)
    }

    /// Ticks or unticks an option of the current question. A single-answer question keeps one
    /// option at most; a multiple-answer one adds or removes. Returns false when nothing changed.
    pub fn toggle(&mut self, paper: &Paper, option_id: &str) -> bool {
        if self.submitted {
            return false;
        }
        let at = self.elapsed_seconds;
        let Some(item) = self.items.get_mut(self.current) else {
            return false;
        };
        if !item.option_order.iter().any(|o| o == option_id) {
            return false;
        }
        let multiple = paper
            .question(&item.question_id)
            .is_some_and(|q| q.is_multiple());
        let question_id = item.question_id.clone();
        if let Some(index) = item.selected.iter().position(|o| o == option_id) {
            item.selected.remove(index);
            self.log
                .push(LogEntry::new(at, LogKind::Untick, &question_id, option_id));
        } else {
            if !multiple {
                for previous in item.selected.drain(..) {
                    self.log
                        .push(LogEntry::new(at, LogKind::Untick, &question_id, &previous));
                }
            }
            item.selected.push(option_id.to_string());
            self.log
                .push(LogEntry::new(at, LogKind::Tick, &question_id, option_id));
        }
        true
    }

    pub fn go_to(&mut self, index: usize) {
        if index < self.items.len() {
            self.current = index;
        }
    }

    pub fn next(&mut self) {
        self.go_to(self.current + 1);
    }

    pub fn previous(&mut self) {
        self.current = self.current.saturating_sub(1);
    }

    pub fn toggle_flag(&mut self) {
        if self.submitted {
            return;
        }
        if let Some(item) = self.items.get_mut(self.current) {
            item.flagged = !item.flagged;
        }
    }

    /// Goes to the next flagged question after the current one, wrapping around.
    pub fn next_flagged(&mut self) -> bool {
        let n = self.items.len();
        let found = (1..=n)
            .map(|step| (self.current + step) % n.max(1))
            .find(|&i| self.items.get(i).is_some_and(|item| item.flagged));
        if let Some(index) = found {
            self.current = index;
        }
        found.is_some()
    }

    /// Advances the clock; the seconds count for the question on screen.
    pub fn tick(&mut self, seconds: u32) {
        if self.submitted {
            return;
        }
        self.elapsed_seconds = self.elapsed_seconds.saturating_add(seconds);
        if let Some(item) = self.items.get_mut(self.current) {
            item.seconds_spent = item.seconds_spent.saturating_add(seconds);
        }
    }

    /// The exam window lost focus (another tab, another program).
    pub fn focus_lost(&mut self) {
        if self.submitted {
            return;
        }
        self.focus_losses += 1;
        self.log.push(LogEntry::new(
            self.elapsed_seconds,
            LogKind::FocusLost,
            "",
            "",
        ));
    }

    /// The window is back after `seconds_away`.
    pub fn focus_back(&mut self, seconds_away: u32) {
        if self.submitted {
            return;
        }
        self.seconds_away = self.seconds_away.saturating_add(seconds_away);
        self.log.push(LogEntry::new(
            self.elapsed_seconds,
            LogKind::FocusBack,
            "",
            "",
        ));
    }

    pub fn answered_count(&self) -> usize {
        self.items.iter().filter(|i| !i.selected.is_empty()).count()
    }

    pub fn flagged_count(&self) -> usize {
        self.items.iter().filter(|i| i.flagged).count()
    }

    /// Ends the exam; later calls return the same copy.
    pub fn submit(&mut self) -> Submission {
        if !self.submitted {
            self.submitted = true;
            self.log
                .push(LogEntry::new(self.elapsed_seconds, LogKind::Submit, "", ""));
        }
        Submission {
            paper_id: self.paper_id,
            student_name: self.student_name.clone(),
            answers: self
                .items
                .iter()
                .map(|item| Answer {
                    question_id: item.question_id.clone(),
                    selected: item.selected.clone(),
                    seconds_spent: item.seconds_spent,
                })
                .collect(),
            elapsed_seconds: self.elapsed_seconds,
            focus_losses: self.focus_losses,
            seconds_away: self.seconds_away,
            log: self.log.clone(),
        }
    }

    /// Time left, pace and overtime; also arms or re-arms the pace alert.
    pub fn time_status(&mut self, paper: &Paper) -> TimeStatus {
        let status = TimeStatus::compute(paper, self, self.pace_alerted);
        self.pace_alerted = status.pace_alerted;
        status
    }
}

/// Collapses spaces and trims, so "  Alex   Example " and "Alex Example" are the same student.
pub fn normalize_name(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}
