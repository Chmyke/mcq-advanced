//! Time left, pace and overtime. Time informs; it never blocks or changes the mark.

use mcq_model::Paper;

use crate::Session;

/// The final warning shows during the last five minutes.
pub const WARNING_SECONDS: u32 = 300;
/// The pace alert fires when the student is this many question budgets behind...
pub const PACE_ALERT_BUDGETS: u32 = 3;
/// ...and re-arms once back within this many.
pub const PACE_REARM_BUDGETS: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TimeStatus {
    /// 0 when the paper has no limit.
    pub limit_seconds: u32,
    /// Seconds left, 0 once the time is up or without a limit.
    pub remaining_seconds: u32,
    /// Seconds past the limit, written on the copy without changing the mark.
    pub overtime_seconds: u32,
    /// True during the last `WARNING_SECONDS`.
    pub final_warning: bool,
    /// Seconds ahead (positive) or behind (negative) the even pace; 0 without a limit.
    pub lead_seconds: i64,
    /// True the moment the pace alert should be shown (once per lag).
    pub show_pace_alert: bool,
    pub(crate) pace_alerted: bool,
}

impl TimeStatus {
    pub(crate) fn compute(paper: &Paper, session: &Session, alerted: bool) -> Self {
        let limit = paper.duration_minutes.saturating_mul(60);
        let elapsed = session.elapsed_seconds;
        let mut status = TimeStatus {
            limit_seconds: limit,
            pace_alerted: alerted,
            ..TimeStatus::default()
        };
        if limit == 0 || session.items.is_empty() {
            return status;
        }
        status.remaining_seconds = limit.saturating_sub(elapsed);
        status.overtime_seconds = elapsed.saturating_sub(limit);
        status.final_warning =
            status.remaining_seconds > 0 && status.remaining_seconds <= WARNING_SECONDS;

        if session.submitted || status.remaining_seconds == 0 {
            return status;
        }
        let budget = f64::from(limit) / session.items.len() as f64;
        let answered = session.answered_count() as f64;
        let lead = budget * answered - f64::from(elapsed);
        status.lead_seconds = lead.round() as i64;
        let behind = -lead;
        if !alerted && behind >= budget * f64::from(PACE_ALERT_BUDGETS) {
            status.show_pace_alert = true;
            status.pace_alerted = true;
        } else if alerted && behind <= budget * f64::from(PACE_REARM_BUDGETS) {
            status.pace_alerted = false;
        }
        status
    }
}

/// `3 min 05 s`, or `42 s` under a minute.
pub fn format_duration(seconds: u32) -> String {
    if seconds < 60 {
        format!("{seconds} s")
    } else {
        format!("{} min {:02} s", seconds / 60, seconds % 60)
    }
}

/// Where the student stands against an even share of the time per question.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaceGauge {
    /// Each question's share of the time.
    pub share_seconds: u32,
    /// Time spent so far on the question on screen.
    pub current_seconds: u32,
    /// Ahead (positive) or behind (negative) the even pace.
    pub lead_seconds: i64,
    /// Share of the time limit gone, from 0 to 1.
    pub time_fraction: f64,
}

impl Session {
    /// The pace gauge, while it means something: a time limit, not handed in, not past the limit.
    pub fn pace(&self, paper: &Paper) -> Option<PaceGauge> {
        let status = TimeStatus::compute(paper, self, true);
        if status.limit_seconds == 0 || status.remaining_seconds == 0 || self.submitted {
            return None;
        }
        let share = status.limit_seconds / self.items.len().max(1) as u32;
        Some(PaceGauge {
            share_seconds: share,
            current_seconds: self.current_item().map_or(0, |i| i.seconds_spent),
            lead_seconds: status.lead_seconds,
            time_fraction: f64::from(self.elapsed_seconds) / f64::from(status.limit_seconds),
        })
    }

    /// Seconds a question saved (positive) or took beyond its share (negative), once visited.
    pub fn share_delta(&self, paper: &Paper, index: usize) -> Option<i64> {
        let gauge = self.pace(paper)?;
        let spent = self.items.get(index)?.seconds_spent;
        (spent > 0).then(|| i64::from(gauge.share_seconds) - i64::from(spent))
    }
}
