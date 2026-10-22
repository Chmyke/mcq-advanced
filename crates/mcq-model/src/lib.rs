//! ROLE:    the data model of an MCQ paper, shared by every other crate.
//! OWNER:   plain data; whoever parses or builds a `Paper` owns it.
//! INVARIANTS: no I/O, no clock, no dependency; option ids are unique within a question.

/// How serious a reported problem is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Blocks publishing.
    Error,
    /// Worth fixing, never blocks.
    Warning,
    /// A neutral fact about the paper.
    Info,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }
}

/// Where a problem was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    Paper,
    /// 1-based line in the source file.
    Line(usize),
    Question(String),
    Theme(String),
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Location::Paper => f.write_str("paper"),
            Location::Line(n) => write!(f, "line {n}"),
            Location::Question(id) => write!(f, "question {id}"),
            Location::Theme(id) => write!(f, "theme {id}"),
        }
    }
}

/// One problem reported by the reader or the linter. `code` is stable and documented.
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub severity: Severity,
    pub code: &'static str,
    pub location: Location,
    pub message: String,
}

impl Issue {
    pub fn new(
        severity: Severity,
        code: &'static str,
        location: Location,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code,
            location,
            message: message.into(),
        }
    }
}

/// How a question earns its point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scoring {
    /// The exact set of correct options, or nothing.
    #[default]
    AllOrNothing,
    /// (correct ticked minus wrong ticked) / correct count, floored at zero.
    Partial,
}

/// Bloom-style cognitive level of a question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Level {
    #[default]
    Unknown,
    Remember,
    Understand,
    Apply,
    Analyse,
    Evaluate,
}

impl Level {
    pub fn from_number(n: u32) -> Option<Self> {
        match n {
            1 => Some(Level::Remember),
            2 => Some(Level::Understand),
            3 => Some(Level::Apply),
            4 => Some(Level::Analyse),
            5 => Some(Level::Evaluate),
            _ => None,
        }
    }

    /// Answering time assumed when a question does not state its own.
    pub fn default_seconds(self) -> u32 {
        match self {
            Level::Remember => 10,
            Level::Understand | Level::Unknown => 15,
            Level::Apply => 20,
            Level::Analyse => 30,
            Level::Evaluate => 40,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Paper {
    pub title: String,
    pub description: String,
    /// Language of the paper text (`fr`, `en`...), empty when not stated.
    pub lang: String,
    /// 0 means no time limit.
    pub duration_minutes: u32,
    /// Points lost per wrong answer; a question is worth 1 point.
    pub penalty: f32,
    /// Points lost per unanswered question.
    pub unanswered_penalty: f32,
    pub scoring: Scoring,
    pub shuffle_questions: bool,
    pub shuffle_options: bool,
    /// Tell the student how many options to tick on multiple-answer questions.
    pub show_answer_count: bool,
    pub themes: Vec<Theme>,
    pub questions: Vec<Question>,
}

impl Paper {
    /// An empty paper with the documented defaults.
    pub fn new() -> Self {
        Self {
            shuffle_questions: true,
            shuffle_options: true,
            show_answer_count: true,
            ..Self::default()
        }
    }

    pub fn theme(&self, id: &str) -> Option<&Theme> {
        self.themes.iter().find(|t| t.id == id)
    }

    pub fn question(&self, id: &str) -> Option<&Question> {
        self.questions.iter().find(|q| q.id == id)
    }

    /// Expected answering time with a 20 % margin, in seconds.
    pub fn estimated_seconds_with_margin(&self) -> u32 {
        let total: u32 = self.questions.iter().map(Question::effective_seconds).sum();
        total * 6 / 5
    }

    /// Largest option count of the paper, at least 2.
    pub fn max_option_count(&self) -> usize {
        self.questions
            .iter()
            .map(|q| q.options.len())
            .max()
            .unwrap_or(0)
            .max(2)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Theme {
    pub id: String,
    pub title: String,
    /// Where to revise in the course material.
    pub course_ref: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Question {
    pub id: String,
    pub theme_id: String,
    pub level: Level,
    /// 0 means derived from the level.
    pub estimated_seconds: u32,
    /// The statement, in reading order: text, code and media.
    pub body: Vec<Block>,
    pub options: Vec<AnswerOption>,
    /// The rule to remember, shown after correction.
    pub rule: String,
    pub course_ref: String,
    /// 1-based line of the question header in its source file, 0 when not read from a file.
    pub line: usize,
}

impl Question {
    pub fn effective_seconds(&self) -> u32 {
        if self.estimated_seconds > 0 {
            self.estimated_seconds
        } else {
            self.level.default_seconds()
        }
    }

    /// The prose of the statement, text blocks only, joined by newlines.
    pub fn statement_text(&self) -> String {
        let texts: Vec<&str> = self
            .body
            .iter()
            .filter_map(|b| match b {
                Block::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        texts.join("\n")
    }

    pub fn code_blocks(&self) -> impl Iterator<Item = &CodeBlock> {
        self.body.iter().filter_map(|b| match b {
            Block::Code(c) => Some(c),
            _ => None,
        })
    }

    pub fn media(&self) -> impl Iterator<Item = &Media> {
        self.body.iter().filter_map(|b| match b {
            Block::Media(m) => Some(m),
            _ => None,
        })
    }

    pub fn correct_count(&self) -> usize {
        self.options.iter().filter(|o| o.is_correct).count()
    }

    /// True when more than one option is correct: the student ticks boxes, not a radio.
    pub fn is_multiple(&self) -> bool {
        self.correct_count() > 1
    }

    pub fn first_correct(&self) -> Option<&AnswerOption> {
        self.options.iter().find(|o| o.is_correct)
    }

    pub fn option(&self, id: &str) -> Option<&AnswerOption> {
        self.options.iter().find(|o| o.id == id)
    }
}

/// A piece of a question statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    /// Markdown prose.
    Text(String),
    Code(CodeBlock),
    Media(Media),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CodeBlock {
    /// Fence language tag as written (`cs`, `cpp`, `python`...), empty when none.
    pub language: String,
    /// 1-based lines to emphasise, from a fence like ```` ```cs {2,4-5} ````.
    pub highlighted_lines: Vec<u32>,
    /// Tabs expanded to four spaces, no trailing newline.
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Audio,
    Video,
}

impl MediaKind {
    /// Infers the kind from a file extension; `None` when it is not a supported media file.
    pub fn from_path(path: &str) -> Option<Self> {
        let ext = path.rsplit_once('.')?.1.to_ascii_lowercase();
        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "avif" => Some(MediaKind::Image),
            "mp3" | "ogg" | "oga" | "wav" | "m4a" | "opus" | "flac" => Some(MediaKind::Audio),
            "mp4" | "webm" | "ogv" | "mov" => Some(MediaKind::Video),
            _ => None,
        }
    }
}

/// An image, sound or video referenced by the statement, relative to the paper folder.
#[derive(Debug, Clone, PartialEq)]
pub struct Media {
    pub kind: MediaKind,
    pub path: String,
    /// Text alternative, required for accessibility.
    pub alt: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnswerOption {
    /// Positional id: `<question id>-o<k>`, k from 1.
    pub id: String,
    pub text: String,
    pub is_correct: bool,
    /// Why this option is right or wrong.
    pub explanation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_seconds_falls_back_to_level() {
        let mut q = Question {
            level: Level::Analyse,
            ..Question::default()
        };
        assert_eq!(q.effective_seconds(), 30);
        q.estimated_seconds = 45;
        assert_eq!(q.effective_seconds(), 45);
    }

    #[test]
    fn estimated_time_has_a_twenty_percent_margin() {
        let mut paper = Paper::new();
        for _ in 0..5 {
            paper.questions.push(Question {
                level: Level::Apply,
                ..Question::default()
            });
        }
        assert_eq!(paper.estimated_seconds_with_margin(), 120);
    }

    #[test]
    fn media_kind_follows_extension() {
        assert_eq!(MediaKind::from_path("media/a.GIF"), Some(MediaKind::Image));
        assert_eq!(
            MediaKind::from_path("media/voice.ogg"),
            Some(MediaKind::Audio)
        );
        assert_eq!(MediaKind::from_path("clip.webm"), Some(MediaKind::Video));
        assert_eq!(MediaKind::from_path("notes.txt"), None);
        assert_eq!(MediaKind::from_path("noext"), None);
    }

    #[test]
    fn multiple_means_more_than_one_correct() {
        let opt = |c| AnswerOption {
            is_correct: c,
            ..AnswerOption::default()
        };
        let mut q = Question {
            options: vec![opt(true), opt(false), opt(false)],
            ..Question::default()
        };
        assert!(!q.is_multiple());
        q.options[1].is_correct = true;
        assert!(q.is_multiple());
    }
}
