//! ROLE:    what a class's copies say: every student's grade, how each question and each
//!          distractor behaved, which themes to reteach, the reliability of the paper, and the
//!          copies worth a look, as findings a teacher can act on.
//! OWNER:   pure functions; the caller provides the answer key and the opened copies.
//! INVARIANTS: no I/O, no clock, no randomness; the same copies give the same report; indices
//!             need `MIN_COPIES_FOR_ITEMS` copies, and below `SMALL_COHORT` they are trends.
//!
//! Thresholds follow the earlier tool, used in class; KR-20, the point-biserial, identical copies
//! and focus losses are additions.

pub mod csv;
pub mod decision;
pub mod sheet;
pub mod stats;
pub mod synth;
pub mod timeline;

use mcq_grade::{GradeResult, Mastery, Status};
use mcq_model::{Answer, LogKind, Paper, Submission};

pub use decision::{Adjustment, Decision};

/// Share of students in the upper and lower groups (Kelley's 27 %), rounded down.
pub const GROUP_FRACTION: f64 = 0.27;
/// Below this many copies, indices are trends to confirm, never verdicts.
pub const SMALL_COHORT: usize = 30;
/// Below this many copies, discrimination, reliability and dead distractors are not computed.
pub const MIN_COPIES_FOR_ITEMS: usize = 8;
/// A question succeeded by at least this share of the class teaches nothing new about it.
pub const TOO_EASY: f64 = 0.95;
/// A question succeeded by this share or less is to be read again.
pub const TOO_HARD: f64 = 0.15;
/// Discrimination under this value separates strong and weak students poorly.
pub const WEAK_DISCRIMINATION: f64 = 0.1;
/// A distractor that separates strong from weak students by at least this much, and better than
/// the key, is probably the right answer: the key is in doubt.
pub const RIVAL_DISCRIMINATION: f64 = 0.2;
/// A distractor picked in at least this share of the answers reveals a shared confusion.
pub const TRAP_RATE: f64 = 0.25;
/// ...and that chance would explain less often than this, were the wrong answers spread evenly
/// over the distractors: a hard question spreads its mistakes, one misleading idea gathers them.
pub const TRAP_CHANCE: f64 = 0.01;
/// A theme whose mean rate is under this value is to be taught again...
pub const RETEACH_RATE: f64 = 0.5;
/// ...as is a theme that at least this share of the class has to relearn.
pub const RETEACH_SHARE: f64 = 0.4;
/// Below this many questions a theme rate says little.
pub const MIN_THEME_QUESTIONS: usize = 3;
/// Leaving the exam window this many times is reported.
pub const FOCUS_LOSSES: u32 = 5;
/// Two copies sharing at least this many identical wrong answers are reported.
pub const SHARED_WRONG: usize = 4;
/// A copy handed in under this share of the estimated time...
pub const FAST_SHARE: f64 = 0.1;
/// ...with at least this success rate is reported.
pub const FAST_SUCCESS: f32 = 0.7;

/// A copy opened by the teacher, with the receipt code the student was shown.
#[derive(Debug, Clone, PartialEq)]
pub struct Copy {
    pub submission: Submission,
    pub receipt: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StudentResult {
    pub name: String,
    pub receipt: String,
    pub grade: GradeResult,
    /// Over 20.
    pub mark: f32,
    pub elapsed_seconds: u32,
    pub focus_losses: u32,
    pub seconds_away: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionStats {
    pub id: String,
    pub is_correct: bool,
    /// Students who ticked it.
    pub picked: u32,
    /// Share of the answers (blanks aside) that ticked it.
    pub rate: f64,
    /// Share of the upper and lower groups who ticked it.
    pub upper_rate: f64,
    pub lower_rate: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuestionStats {
    pub id: String,
    pub theme_id: String,
    /// Copies in which the question appears.
    pub asked: u32,
    pub blank: u32,
    /// Mean credit, from 0 to 1: the share of the class who got it.
    pub facility: f64,
    /// Upper group facility minus lower group facility, from -1 to 1.
    pub discrimination: Option<f64>,
    /// Correlation between the question and the rest of the paper.
    pub point_biserial: Option<f64>,
    /// Over the copies that spent time on it.
    pub mean_seconds: f64,
    pub options: Vec<OptionStats>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeStats {
    pub id: String,
    pub title: String,
    pub course_ref: String,
    /// Questions of the paper in the theme.
    pub questions: usize,
    /// Mean of the students' rates on the theme, from 0 to 1.
    pub mean_rate: f64,
    pub acquired: u32,
    pub fragile: u32,
    pub to_relearn: u32,
}

impl ThemeStats {
    pub fn assessed(&self) -> u32 {
        self.acquired + self.fragile + self.to_relearn
    }

    /// Enough questions for its rate to mean something.
    pub fn is_diagnostic(&self) -> bool {
        self.questions >= MIN_THEME_QUESTIONS
    }

    /// Fragile students are left out on purpose: counting them flags every theme of a class
    /// around 12/20.
    pub fn needs_reteaching(&self) -> bool {
        let n = self.assessed();
        self.is_diagnostic()
            && n > 0
            && (self.mean_rate < RETEACH_RATE
                || f64::from(self.to_relearn) / f64::from(n) >= RETEACH_SHARE)
    }
}

/// Something a teacher can act on, about a question or a theme.
#[derive(Debug, Clone, PartialEq)]
pub enum Finding {
    /// The class did not get the theme. `doubtful` lists its questions whose key is in doubt:
    /// check them first, the notion may be fine.
    Reteach {
        theme: String,
        mean_rate: f64,
        doubtful: Vec<String>,
    },
    /// One distractor gathers many answers, most of the wrong ones: a precise confusion to
    /// address in class.
    Trap {
        question: String,
        option: String,
        rate: f64,
    },
    /// The key may be wrong or the question ambiguous: weak students succeed more than strong
    /// ones, or a distractor attracts the strong students (`rival`), as a right answer would.
    KeyDoubt {
        question: String,
        rival: Option<String>,
        discrimination: f64,
    },
    TooHard {
        question: String,
        facility: f64,
    },
    /// Distractors nobody picked: they do not work as distractors.
    DeadDistractors {
        question: String,
        count: usize,
    },
    TooEasy {
        question: String,
        facility: f64,
    },
    /// Strong and weak students succeed alike: the question measures little.
    WeakDiscrimination {
        question: String,
        discrimination: f64,
    },
}

/// Something about a copy worth a question; never a verdict.
#[derive(Debug, Clone, PartialEq)]
pub enum Anomaly {
    /// The copy answers another package: left out of the analysis.
    OtherPaper {
        name: String,
        receipt: String,
    },
    /// Several copies under one name: all kept.
    SameName {
        name: String,
        count: u32,
    },
    /// The copy answers questions the answer key does not have.
    UnknownQuestions {
        name: String,
        count: usize,
    },
    /// Two copies with the same answers, wrong ones included.
    Identical {
        first: String,
        second: String,
        shared_wrong: usize,
    },
    LeftWindow {
        name: String,
        times: u32,
        seconds_away: u32,
    },
    Overtime {
        name: String,
        seconds: u32,
    },
    /// Very good and very quick.
    Fast {
        name: String,
        elapsed_seconds: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    /// In name order.
    pub students: Vec<StudentResult>,
    /// In paper order.
    pub questions: Vec<QuestionStats>,
    /// In paper order.
    pub themes: Vec<ThemeStats>,
    /// Themes to reteach (weakest first), shared confusions (most chosen first), then questions
    /// to review (most urgent first).
    pub findings: Vec<Finding>,
    pub anomalies: Vec<Anomaly>,
    /// KR-20 (alpha with partial credit), from 0 to 1 for a usable paper.
    pub reliability: Option<f64>,
    pub mean_mark: Option<f64>,
    pub median_mark: Option<f64>,
    pub mean_seconds: Option<f64>,
    /// Fewer than `SMALL_COHORT` copies: indices are trends to confirm.
    pub small_cohort: bool,
    /// At least `MIN_COPIES_FOR_ITEMS` copies: item indices were computed.
    pub items_computed: bool,
    /// The teacher's decisions that took effect, in the order given.
    pub adjustments: Vec<Adjustment>,
}

impl Report {
    pub fn decision(&self, question: &str) -> Option<&Decision> {
        self.adjustments
            .iter()
            .find(|a| a.question == question)
            .map(|a| &a.decision)
    }
}

/// Analyses the copies of one package against its answer key.
pub fn analyse(paper: &Paper, paper_id: [u8; 16], copies: &[Copy]) -> Report {
    analyse_with(paper, paper_id, copies, &[])
}

/// Analyses the copies once the teacher's decisions are applied to the answer key: a neutralized
/// question still has its statistics but counts in no mark, no total and no finding.
pub fn analyse_with(
    paper: &Paper,
    paper_id: [u8; 16],
    copies: &[Copy],
    adjustments: &[Adjustment],
) -> Report {
    let (key, adjustments) = decision::apply(paper, adjustments);
    let paper = &key;
    let graded: Vec<bool> = paper
        .questions
        .iter()
        .map(|q| !decision::is_neutralized(&adjustments, &q.id))
        .collect();
    let mut report = Report {
        adjustments,
        ..Report::default()
    };
    let mut kept: Vec<&Copy> = Vec::new();
    for copy in copies {
        if copy.submission.paper_id == paper_id {
            kept.push(copy);
        } else {
            report.anomalies.push(Anomaly::OtherPaper {
                name: copy.submission.student_name.clone(),
                receipt: copy.receipt.clone(),
            });
        }
    }
    kept.sort_by(|a, b| {
        a.submission
            .student_name
            .to_lowercase()
            .cmp(&b.submission.student_name.to_lowercase())
            .then_with(|| a.receipt.cmp(&b.receipt))
    });

    report.students = kept
        .iter()
        .map(|copy| {
            let s = &copy.submission;
            let answers: Vec<Answer> = s
                .answers
                .iter()
                .filter(|a| !decision::is_neutralized(&report.adjustments, &a.question_id))
                .cloned()
                .collect();
            let grade = mcq_grade::grade(paper, &answers, s.elapsed_seconds);
            StudentResult {
                name: s.student_name.clone(),
                receipt: copy.receipt.clone(),
                mark: grade.mark(20.0),
                grade,
                elapsed_seconds: s.elapsed_seconds,
                focus_losses: s.focus_losses,
                seconds_away: s.seconds_away,
            }
        })
        .collect();
    report.small_cohort = report.students.len() < SMALL_COHORT;
    report.items_computed = report.students.len() >= MIN_COPIES_FOR_ITEMS;
    let marks: Vec<f64> = report.students.iter().map(|s| f64::from(s.mark)).collect();
    report.mean_mark = stats::mean(&marks);
    report.median_mark = stats::median(&marks);
    let elapsed: Vec<f64> = report
        .students
        .iter()
        .map(|s| f64::from(s.elapsed_seconds))
        .collect();
    report.mean_seconds = stats::mean(&elapsed);

    // Item scores: credit per student and question, 0 when the question was not asked. A
    // neutralized question has its column, out of the totals and the reliability.
    let credits: Vec<Vec<f64>> = kept
        .iter()
        .map(|copy| {
            paper
                .questions
                .iter()
                .map(|q| {
                    copy.submission
                        .answers
                        .iter()
                        .find(|a| a.question_id == q.id)
                        .map_or(0.0, |a| {
                            f64::from(mcq_grade::score(q, &a.selected, paper.scoring).1)
                        })
                })
                .collect()
        })
        .collect();
    let graded_only = |row: &Vec<f64>| -> Vec<f64> {
        row.iter()
            .zip(&graded)
            .filter(|(_, g)| **g)
            .map(|(c, _)| *c)
            .collect()
    };
    let totals: Vec<f64> = credits
        .iter()
        .map(|row| graded_only(row).iter().sum())
        .collect();
    let (lower, upper) = stats::extreme_groups(&totals, GROUP_FRACTION);
    if report.items_computed {
        report.reliability = stats::kr20(&credits.iter().map(graded_only).collect::<Vec<_>>());
    }
    for (q, question) in paper.questions.iter().enumerate() {
        report.questions.push(question_stats(
            question,
            (q, graded[q]),
            &kept,
            &credits,
            &totals,
            (&lower, &upper),
            report.items_computed,
        ));
    }

    for theme in &paper.themes {
        let mut stats = ThemeStats {
            id: theme.id.clone(),
            title: theme.title.clone(),
            course_ref: theme.course_ref.clone(),
            questions: paper
                .questions
                .iter()
                .zip(&graded)
                .filter(|(q, g)| **g && q.theme_id == theme.id)
                .count(),
            mean_rate: 0.0,
            acquired: 0,
            fragile: 0,
            to_relearn: 0,
        };
        let mut rates = Vec::new();
        for student in &report.students {
            let Some(result) = student.grade.themes.iter().find(|t| t.theme_id == theme.id) else {
                continue;
            };
            match result.mastery() {
                Mastery::Acquired => stats.acquired += 1,
                Mastery::Fragile => stats.fragile += 1,
                Mastery::ToRelearn => stats.to_relearn += 1,
                Mastery::NotAssessed => continue,
            }
            rates.push(f64::from(result.rate()));
        }
        stats.mean_rate = stats::mean(&rates).unwrap_or(0.0);
        report.themes.push(stats);
    }

    report.findings = findings(&report);
    report.anomalies.extend(anomalies(paper, &kept, &report));
    report
}

fn share(count: u32, of: usize) -> f64 {
    if of == 0 {
        0.0
    } else {
        f64::from(count) / of as f64
    }
}

fn question_stats(
    question: &mcq_model::Question,
    (q, graded): (usize, bool),
    kept: &[&Copy],
    credits: &[Vec<f64>],
    totals: &[f64],
    (lower, upper): (&[usize], &[usize]),
    items: bool,
) -> QuestionStats {
    let n = question.options.len();
    let (mut asked, mut blank) = (0_u32, 0_u32);
    let mut seconds = Vec::new();
    let (mut picked, mut upper_picked, mut lower_picked) = (vec![0; n], vec![0; n], vec![0; n]);
    for (k, copy) in kept.iter().enumerate() {
        let Some(answer) = copy
            .submission
            .answers
            .iter()
            .find(|a| a.question_id == question.id)
        else {
            continue;
        };
        asked += 1;
        if answer.seconds_spent > 0 {
            seconds.push(f64::from(answer.seconds_spent));
        }
        if answer.selected.is_empty() {
            blank += 1;
        }
        for (o, option) in question.options.iter().enumerate() {
            if answer.selected.contains(&option.id) {
                picked[o] += 1;
                upper_picked[o] += u32::from(upper.contains(&k));
                lower_picked[o] += u32::from(lower.contains(&k));
            }
        }
    }
    let column: Vec<f64> = credits.iter().map(|row| row[q]).collect();
    let group_facility = |group: &[usize]| {
        if group.is_empty() {
            0.0
        } else {
            group.iter().map(|&i| column[i]).sum::<f64>() / group.len() as f64
        }
    };
    // The rest of the paper: a graded question is part of the total, a neutralized one is not.
    let rest: Vec<f64> = totals
        .iter()
        .zip(&column)
        .map(|(t, c)| if graded { t - c } else { *t })
        .collect();
    let answered = (asked - blank) as usize;
    QuestionStats {
        id: question.id.clone(),
        theme_id: question.theme_id.clone(),
        asked,
        blank,
        facility: if asked == 0 {
            0.0
        } else {
            column.iter().sum::<f64>() / f64::from(asked)
        },
        discrimination: items.then(|| group_facility(upper) - group_facility(lower)),
        point_biserial: if items {
            stats::correlation(&column, &rest)
        } else {
            None
        },
        mean_seconds: stats::mean(&seconds).unwrap_or(0.0),
        options: question
            .options
            .iter()
            .enumerate()
            .map(|(o, option)| OptionStats {
                id: option.id.clone(),
                is_correct: option.is_correct,
                picked: picked[o],
                rate: share(picked[o], answered),
                upper_rate: share(upper_picked[o], upper.len()),
                lower_rate: share(lower_picked[o], lower.len()),
            })
            .collect(),
    }
}

fn findings(report: &Report) -> Vec<Finding> {
    let mut traps: Vec<Finding> = Vec::new();
    let mut review: Vec<(u8, Finding)> = Vec::new();
    // A neutralized question is settled: nothing more to say about it.
    for q in report
        .questions
        .iter()
        .filter(|q| q.asked > 0 && !decision::is_neutralized(&report.adjustments, &q.id))
    {
        let mut doubt = false;
        if let Some(d) = q.discrimination {
            // A good distractor tempts weak students more than strong ones; one that does the
            // reverse, better than the key, is probably the right answer.
            let rival = q
                .options
                .iter()
                .filter(|o| !o.is_correct)
                .map(|o| (o, o.upper_rate - o.lower_rate))
                .filter(|(_, od)| *od >= RIVAL_DISCRIMINATION && *od > d)
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(o, _)| o);
            if d < 0.0 || rival.is_some() {
                doubt = true;
                review.push((
                    6,
                    Finding::KeyDoubt {
                        question: q.id.clone(),
                        rival: rival.map(|o| o.id.clone()),
                        discrimination: d,
                    },
                ));
            }
        }
        // A doubtful key makes everything else about the question meaningless.
        if doubt {
            continue;
        }
        // A precise confusion gathers the wrong answers beyond chance; a hard question spreads
        // them evenly. With a single distractor (true or false) the two cannot be told apart.
        let distractors = q.options.iter().filter(|o| !o.is_correct).count();
        let wrong_picks: u32 = q
            .options
            .iter()
            .filter(|o| !o.is_correct)
            .map(|o| o.picked)
            .sum();
        if let Some(trap) = q
            .options
            .iter()
            .filter(|o| {
                !o.is_correct
                    && distractors >= 2
                    && o.rate >= TRAP_RATE
                    && stats::binomial_tail(wrong_picks, o.picked, 1.0 / distractors as f64)
                        < TRAP_CHANCE
            })
            .max_by(|a, b| a.rate.total_cmp(&b.rate))
        {
            traps.push(Finding::Trap {
                question: q.id.clone(),
                option: trap.id.clone(),
                rate: trap.rate,
            });
        }
        if q.facility <= TOO_HARD {
            review.push((
                5,
                Finding::TooHard {
                    question: q.id.clone(),
                    facility: q.facility,
                },
            ));
        }
        let dead = q
            .options
            .iter()
            .filter(|o| !o.is_correct && o.picked == 0)
            .count();
        if report.items_computed && dead > 0 {
            review.push((
                4,
                Finding::DeadDistractors {
                    question: q.id.clone(),
                    count: dead,
                },
            ));
        }
        if q.facility >= TOO_EASY {
            review.push((
                3,
                Finding::TooEasy {
                    question: q.id.clone(),
                    facility: q.facility,
                },
            ));
        } else if let Some(d) = q
            .discrimination
            .filter(|d| (0.0..WEAK_DISCRIMINATION).contains(d))
        {
            review.push((
                2,
                Finding::WeakDiscrimination {
                    question: q.id.clone(),
                    discrimination: d,
                },
            ));
        }
    }
    traps.sort_by(|a, b| match (a, b) {
        (Finding::Trap { rate: x, .. }, Finding::Trap { rate: y, .. }) => y.total_cmp(x),
        _ => std::cmp::Ordering::Equal,
    });
    // Stable: same priority keeps paper order.
    review.sort_by_key(|(priority, _)| std::cmp::Reverse(*priority));

    let mut reteach: Vec<&ThemeStats> = report
        .themes
        .iter()
        .filter(|t| t.needs_reteaching())
        .collect();
    reteach.sort_by(|a, b| a.mean_rate.total_cmp(&b.mean_rate));
    let mut all: Vec<Finding> = reteach
        .into_iter()
        .map(|t| Finding::Reteach {
            theme: t.id.clone(),
            mean_rate: t.mean_rate,
            doubtful: review
                .iter()
                .filter_map(|(_, f)| match f {
                    Finding::KeyDoubt { question, .. }
                        if report
                            .questions
                            .iter()
                            .any(|q| q.id == *question && q.theme_id == t.id) =>
                    {
                        Some(question.clone())
                    }
                    _ => None,
                })
                .collect(),
        })
        .collect();
    all.extend(traps);
    all.extend(review.into_iter().map(|(_, f)| f));
    all
}

fn anomalies(paper: &Paper, kept: &[&Copy], report: &Report) -> Vec<Anomaly> {
    let mut out = Vec::new();
    let key = name_key;
    let mut reported: Vec<String> = Vec::new();
    for copy in kept {
        let name = key(&copy.submission.student_name);
        let count = kept
            .iter()
            .filter(|c| key(&c.submission.student_name) == name)
            .count() as u32;
        if count > 1 && !reported.contains(&name) {
            reported.push(name);
            out.push(Anomaly::SameName {
                name: copy.submission.student_name.clone(),
                count,
            });
        }
    }

    for copy in kept {
        let unknown = copy
            .submission
            .answers
            .iter()
            .filter(|a| paper.question(&a.question_id).is_none())
            .count();
        if unknown > 0 {
            out.push(Anomaly::UnknownQuestions {
                name: copy.submission.student_name.clone(),
                count: unknown,
            });
        }
    }

    for (i, a) in kept.iter().enumerate() {
        for b in &kept[i + 1..] {
            if let Some(shared_wrong) = identical(paper, &a.submission, &b.submission) {
                out.push(Anomaly::Identical {
                    first: a.submission.student_name.clone(),
                    second: b.submission.student_name.clone(),
                    shared_wrong,
                });
            }
        }
    }

    let limit = paper.duration_minutes * 60;
    let budget = f64::from(paper.estimated_seconds_with_margin());
    for (copy, student) in kept.iter().zip(&report.students) {
        let s = &copy.submission;
        let left = s
            .log
            .iter()
            .filter(|e| e.kind == LogKind::FocusLost)
            .count() as u32;
        let times = s.focus_losses.max(left);
        if times >= FOCUS_LOSSES {
            out.push(Anomaly::LeftWindow {
                name: s.student_name.clone(),
                times,
                seconds_away: s.seconds_away,
            });
        }
        if limit > 0 && s.elapsed_seconds > limit {
            out.push(Anomaly::Overtime {
                name: s.student_name.clone(),
                seconds: s.elapsed_seconds - limit,
            });
        }
        if f64::from(s.elapsed_seconds) < budget * FAST_SHARE
            && student.grade.success_rate() >= FAST_SUCCESS
        {
            out.push(Anomaly::Fast {
                name: s.student_name.clone(),
                elapsed_seconds: s.elapsed_seconds,
            });
        }
    }
    out
}

/// One student under several spellings of spaces and case.
fn name_key(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// The number of identical wrong answers when two copies answer every question alike.
fn identical(paper: &Paper, a: &Submission, b: &Submission) -> Option<usize> {
    let mut shared_wrong = 0;
    for question in &paper.questions {
        let pick = |s: &Submission| {
            let mut selected = s
                .answers
                .iter()
                .find(|x| x.question_id == question.id)
                .map(|x| x.selected.clone())
                .unwrap_or_default();
            selected.sort();
            selected
        };
        let (x, y) = (pick(a), pick(b));
        if x != y {
            return None;
        }
        if !x.is_empty() && mcq_grade::score(question, &x, paper.scoring).0 != Status::Correct {
            shared_wrong += 1;
        }
    }
    (shared_wrong >= SHARED_WRONG).then_some(shared_wrong)
}
