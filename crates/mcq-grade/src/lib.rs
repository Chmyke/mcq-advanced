//! ROLE:    grades a submitted copy against its paper: mark, per-question outcome, per-theme mastery.
//! OWNER:   pure functions; the caller owns the paper and the answers.
//! INVARIANTS: grading works on option ids, never on display positions;
//!             time never changes the mark; the success rate is never negative.

use mcq_model::{Paper, Question, Scoring};

/// What the student ticked on one question of their session.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Answer {
    pub question_id: String,
    pub selected: Vec<String>,
    pub seconds_spent: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Correct,
    /// Some credit, not all (partial scoring only).
    Partial,
    Wrong,
    Unanswered,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub question_id: String,
    pub status: Status,
    /// Between 0 and 1.
    pub credit: f32,
    pub seconds_spent: u32,
}

impl Outcome {
    /// Anything short of full credit is worth reviewing.
    pub fn needs_review(&self) -> bool {
        self.status != Status::Correct
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mastery {
    NotAssessed,
    ToRelearn,
    Fragile,
    Acquired,
}

/// Below this many questions a theme rate says little.
pub const MIN_DIAGNOSTIC_QUESTIONS: u32 = 3;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ThemeResult {
    pub theme_id: String,
    pub correct: u32,
    pub partial: u32,
    pub wrong: u32,
    pub unanswered: u32,
    pub total: u32,
    pub credit: f32,
    pub seconds: u32,
}

impl ThemeResult {
    /// Credit earned over questions asked, without penalties.
    pub fn rate(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.credit / self.total as f32
        }
    }

    pub fn mastery(&self) -> Mastery {
        if self.total == 0 {
            return Mastery::NotAssessed;
        }
        let rate = self.rate();
        if rate >= 0.75 {
            Mastery::Acquired
        } else if rate >= 0.5 {
            Mastery::Fragile
        } else {
            Mastery::ToRelearn
        }
    }

    pub fn is_diagnostic(&self) -> bool {
        self.total >= MIN_DIAGNOSTIC_QUESTIONS
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GradeResult {
    pub correct: u32,
    pub partial: u32,
    pub wrong: u32,
    pub unanswered: u32,
    pub total: u32,
    /// Earned credit minus penalties, in question points; may be negative.
    pub net_score: f32,
    /// 0 when the paper has no time limit.
    pub time_limit_seconds: u32,
    pub overtime_seconds: u32,
    /// One per answered-or-blank question that exists in the paper, in session order.
    pub outcomes: Vec<Outcome>,
    /// Declared themes first, in paper order, then any undeclared theme met.
    pub themes: Vec<ThemeResult>,
}

impl GradeResult {
    /// Net score over question count, floored at 0.
    pub fn success_rate(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.net_score / self.total as f32).max(0.0)
        }
    }

    /// The mark on a scale, such as 20 or 100.
    pub fn mark(&self, scale: f32) -> f32 {
        self.success_rate() * scale
    }

    pub fn to_review(&self) -> impl Iterator<Item = &Outcome> {
        self.outcomes.iter().filter(|o| o.needs_review())
    }
}

/// Grades a copy. `answers` holds one entry per question of the student's session.
pub fn grade(paper: &Paper, answers: &[Answer], elapsed_seconds: u32) -> GradeResult {
    let mut result = GradeResult {
        time_limit_seconds: paper.duration_minutes * 60,
        themes: paper
            .themes
            .iter()
            .map(|t| ThemeResult {
                theme_id: t.id.clone(),
                ..ThemeResult::default()
            })
            .collect(),
        ..GradeResult::default()
    };
    if result.time_limit_seconds > 0 {
        result.overtime_seconds = elapsed_seconds.saturating_sub(result.time_limit_seconds);
    }

    let mut earned = 0.0_f32;
    let mut penalised_blanks = 0_u32;
    for answer in answers {
        result.total += 1;
        let Some(question) = paper.question(&answer.question_id) else {
            // The question left the paper after the copy was taken: counted, never penalised.
            result.unanswered += 1;
            continue;
        };
        let (status, credit) = score(question, &answer.selected, paper.scoring);
        match status {
            Status::Correct => result.correct += 1,
            Status::Partial => result.partial += 1,
            Status::Wrong => result.wrong += 1,
            Status::Unanswered => {
                result.unanswered += 1;
                penalised_blanks += 1;
            }
        }
        earned += credit;

        let theme = theme_slot(&mut result.themes, &question.theme_id);
        theme.total += 1;
        theme.credit += credit;
        theme.seconds += answer.seconds_spent;
        match status {
            Status::Correct => theme.correct += 1,
            Status::Partial => theme.partial += 1,
            Status::Wrong => theme.wrong += 1,
            Status::Unanswered => theme.unanswered += 1,
        }

        result.outcomes.push(Outcome {
            question_id: question.id.clone(),
            status,
            credit,
            seconds_spent: answer.seconds_spent,
        });
    }

    result.net_score = earned
        - result.wrong as f32 * paper.penalty
        - penalised_blanks as f32 * paper.unanswered_penalty;
    result
}

/// Status and credit of one question. Unknown or repeated ids count once, unknown ones as wrong.
pub fn score(question: &Question, selected: &[String], scoring: Scoring) -> (Status, f32) {
    if selected.is_empty() {
        return (Status::Unanswered, 0.0);
    }
    let mut seen: Vec<&str> = Vec::with_capacity(selected.len());
    let (mut right, mut wrong) = (0_usize, 0_usize);
    for id in selected {
        if seen.contains(&id.as_str()) {
            continue;
        }
        seen.push(id);
        match question.option(id) {
            Some(option) if option.is_correct => right += 1,
            _ => wrong += 1,
        }
    }
    let expected = question.correct_count();
    let credit = if expected == 0 {
        0.0
    } else {
        match scoring {
            Scoring::AllOrNothing => {
                if wrong == 0 && right == expected {
                    1.0
                } else {
                    0.0
                }
            }
            Scoring::Partial => (right as f32 - wrong as f32).max(0.0) / expected as f32,
        }
    };
    let status = if credit >= 1.0 {
        Status::Correct
    } else if credit > 0.0 {
        Status::Partial
    } else {
        Status::Wrong
    };
    (status, credit)
}

fn theme_slot<'a>(themes: &'a mut Vec<ThemeResult>, theme_id: &str) -> &'a mut ThemeResult {
    let index = match themes.iter().position(|t| t.theme_id == theme_id) {
        Some(index) => index,
        None => {
            themes.push(ThemeResult {
                theme_id: theme_id.to_string(),
                ..ThemeResult::default()
            });
            themes.len() - 1
        }
    };
    &mut themes[index]
}

/// What a student should do when unsure, given the penalties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuessAdvice {
    /// No penalty at all: guessing costs nothing.
    GuessingIsFree,
    /// A blind guess is worth at least as much as a blank.
    AnswerEverything,
    /// Blanks cost something, blind guesses cost more: answer with any idea.
    AnswerWhenYouHaveAnIdea,
    /// Blanks are free, blind guesses cost: tick only with an idea.
    TickOnlyWhenYouHaveAnIdea,
}

/// Compares the expected value of a blind guess with the cost of a blank.
pub fn guess_advice(paper: &Paper) -> GuessAdvice {
    let wrong = paper.penalty;
    let blank = paper.unanswered_penalty;
    if wrong == 0.0 && blank == 0.0 {
        return GuessAdvice::GuessingIsFree;
    }
    if wrong == 0.0 {
        return GuessAdvice::AnswerEverything;
    }
    let n = paper.max_option_count() as f32;
    let blind_guess = (1.0 - (n - 1.0) * wrong) / n;
    // The tolerance keeps 1/3 with four options neutral despite float rounding.
    if blind_guess >= -blank - 0.002 {
        GuessAdvice::AnswerEverything
    } else if blank > 0.0 {
        GuessAdvice::AnswerWhenYouHaveAnIdea
    } else {
        GuessAdvice::TickOnlyWhenYouHaveAnIdea
    }
}
