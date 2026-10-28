//! A fictional class taking a fictional paper, generated from a seed, with known flaws planted on
//! purpose: tests check that the analysis finds each of them, and only them, every time.
//! `cohort` builds its own paper; `simulate` has a class sit any paper, such as the website demo.
//!
//! Students answer by a logistic model: the stronger the student and the easier the question, the
//! likelier the right answer; otherwise a distractor, some more tempting than others.

use mcq_model::{
    AnswerOption, Block, Level, LogEntry, LogKind, Paper, Question, Submission, Theme,
};

/// What was planted, to compare with what the analysis reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planted {
    /// Marked with the wrong key: strong students pick another option.
    pub faulty_question: String,
    /// The option strong students pick on it, the right one in truth.
    pub faulty_answer: String,
    /// A distractor that pulls far more students than the others.
    pub trap_question: String,
    pub trap_option: String,
    /// A theme whose questions are all hard.
    pub weak_theme: String,
    /// Two students with the same answers, the second a copy of the first.
    pub copied: (String, String),
    /// A student who left the exam window many times.
    pub distracted: String,
    /// A student far past the time limit.
    pub late: String,
}

/// A class sitting a paper, with what is planted in its copies.
pub struct Class<'a> {
    pub paper: &'a Paper,
    pub paper_id: [u8; 16],
    /// One per question, in the paper's order: 0 is middling, higher is harder.
    pub difficulty: &'a [f64],
    /// One per student, in the order of the copies.
    pub names: &'a [String],
    pub planted: &'a Planted,
}

/// SplitMix64: small, fast, reproducible on every platform.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1_u64 << 53) as f64
    }

    /// Roughly normal, mean 0, standard deviation 1 (sum of twelve uniforms).
    fn normal(&mut self) -> f64 {
        (0..12).map(|_| self.unit()).sum::<f64>() - 6.0
    }

    /// An index drawn with the given weights.
    fn weighted(&mut self, weights: &[f64]) -> usize {
        let total: f64 = weights.iter().sum();
        let mut x = self.unit() * total;
        for (i, w) in weights.iter().enumerate() {
            if x < *w {
                return i;
            }
            x -= w;
        }
        weights.len() - 1
    }
}

const THEMES: [(&str, &str); 3] = [
    ("loops", "Loops"),
    ("arrays", "Arrays"),
    ("pointers", "Pointers"),
];
const PER_THEME: usize = 6;
const OPTIONS: usize = 4;
pub const PAPER_ID: [u8; 16] = [7; 16];
pub const DURATION_MINUTES: u32 = 20;

/// The paper and the copies of `students` students (at least 4).
pub fn cohort(seed: u64, students: usize) -> (Paper, Vec<Submission>, Planted) {
    let students = students.max(4);
    let mut rng = Rng(seed);
    let mut paper = Paper::new();
    paper.title = "Synthetic paper".into();
    paper.lang = "en".into();
    paper.duration_minutes = DURATION_MINUTES;
    paper.penalty = 0.25;

    // Difficulty per question; the weak theme is hard throughout.
    let mut difficulty = Vec::new();
    for (t, (theme_id, title)) in THEMES.iter().enumerate() {
        paper.themes.push(Theme {
            id: (*theme_id).into(),
            title: (*title).into(),
            course_ref: format!("Chapter {}", t + 1),
            ..Theme::default()
        });
        for k in 0..PER_THEME {
            let id = format!("{theme_id}-{}", k + 1);
            let options = (0..OPTIONS)
                .map(|o| AnswerOption {
                    id: format!("{id}-o{}", o + 1),
                    text: format!("Choice {} of {id}", o + 1),
                    is_correct: o == 0,
                    explanation: "Because.".into(),
                })
                .collect();
            paper.questions.push(Question {
                id,
                theme_id: (*theme_id).into(),
                level: Level::Apply,
                body: vec![Block::Text(format!("Question {k} about {title}?"))],
                options,
                rule: "A rule.".into(),
                ..Question::default()
            });
            let base = rng.unit() * 2.2 - 1.8;
            difficulty.push(if t == 2 { base + 1.6 } else { base });
        }
    }
    // The planted questions get a set, middling difficulty: enough wrong answers for the trap to
    // gather, enough right ones for the doubtful key to show, whatever the seed.
    difficulty[2] = -0.3; // loops-3
    difficulty[PER_THEME + 1] = 0.4; // arrays-2
    let planted = Planted {
        faulty_question: "loops-3".into(),
        faulty_answer: "loops-3-o2".into(),
        trap_question: "arrays-2".into(),
        trap_option: "arrays-2-o3".into(),
        weak_theme: "pointers".into(),
        copied: ("Student 02".into(), "Student 03".into()),
        distracted: "Student 05".into(),
        late: "Student 06".into(),
    };
    let names: Vec<String> = (0..students)
        .map(|s| format!("Student {:02}", s + 1))
        .collect();
    let class = Class {
        paper: &paper,
        paper_id: PAPER_ID,
        difficulty: &difficulty,
        names: &names,
        planted: &planted,
    };
    let copies = sit(&class, &mut rng);
    (paper, copies, planted)
}

/// The copies of a class, the same for the same seed.
pub fn simulate(class: &Class<'_>, seed: u64) -> Vec<Submission> {
    sit(class, &mut Rng(seed))
}

fn sit(class: &Class<'_>, rng: &mut Rng) -> Vec<Submission> {
    let (paper, planted) = (class.paper, class.planted);
    let duration = paper.duration_minutes * 60;
    let mut copies: Vec<Submission> = Vec::new();
    for name in class.names {
        // The copied pair are weak students, so their shared mistakes stand out.
        let ability = if *name == planted.copied.0 || *name == planted.copied.1 {
            -1.2
        } else {
            rng.normal()
        };
        let mut answers = Vec::new();
        let mut log = Vec::new();
        let mut clock = 0;
        for (q, question) in paper.questions.iter().enumerate() {
            let planned = question.estimated_seconds;
            let seconds = if planned == 0 {
                20 + (rng.unit() * 60.0) as u32
            } else {
                planned / 2 + (rng.unit() * f64::from(planned)) as u32
            };
            clock += seconds;
            let p_right = 1.0 / (1.0 + (-1.7 * (ability - class.difficulty[q])).exp());
            let blank = rng.unit() < 0.04 * (1.0 - p_right);
            // The faulty question's real answer is another option, marked wrong in the key.
            let right: Vec<&str> = if question.id == planted.faulty_question {
                vec![planted.faulty_answer.as_str()]
            } else {
                question
                    .options
                    .iter()
                    .filter(|o| o.is_correct)
                    .map(|o| o.id.as_str())
                    .collect()
            };
            let weights: Vec<f64> = question
                .options
                .iter()
                .map(|o| {
                    if right.contains(&o.id.as_str()) {
                        0.0
                    } else if o.id == planted.trap_option {
                        8.0
                    } else {
                        1.0
                    }
                })
                .collect();
            let picked: Vec<String> = if rng.unit() < p_right {
                right.iter().map(|id| (*id).to_string()).collect()
            } else {
                vec![question.options[rng.weighted(&weights)].id.clone()]
            };
            let selected = if blank { Vec::new() } else { picked };
            for option in &selected {
                log.push(LogEntry::new(clock, LogKind::Tick, &question.id, option));
            }
            answers.push(mcq_model::Answer {
                question_id: question.id.clone(),
                selected,
                seconds_spent: seconds,
                reported: false,
            });
        }
        let mut copy = Submission {
            paper_id: class.paper_id,
            student_name: name.clone(),
            answers,
            elapsed_seconds: clock.min(duration - 30),
            log,
            ..Submission::default()
        };
        if *name == planted.copied.1
            && let Some(first) = copies.iter().find(|c| c.student_name == planted.copied.0)
        {
            copy.answers = first.answers.clone();
            copy.log = first.log.clone();
        }
        if *name == planted.distracted {
            copy.focus_losses = 9;
            copy.seconds_away = 240;
            // Nine exits spread over the exam, 240 s away in all, as the player journals them.
            let step = copy.elapsed_seconds / 10;
            for k in 0..9_u32 {
                let left = step * (k + 1);
                let away = if k == 8 { 240 - 26 * 8 } else { 26 };
                copy.log
                    .push(LogEntry::new(left, LogKind::FocusLost, "", ""));
                copy.log
                    .push(LogEntry::new(left + away, LogKind::FocusBack, "", ""));
            }
        }
        if *name == planted.late {
            copy.elapsed_seconds = duration + 420;
        }
        copy.log
            .push(LogEntry::new(copy.elapsed_seconds, LogKind::Submit, "", ""));
        copies.push(copy);
    }
    copies
}
