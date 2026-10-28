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
    /// English text, used by the command line and the docs.
    pub message: String,
    /// The same text in French, for interfaces shown in French.
    pub fr: String,
}

impl Issue {
    /// Both languages are required so that no reported problem lacks its French text.
    pub fn new(
        severity: Severity,
        code: &'static str,
        location: Location,
        message: impl Into<String>,
        fr: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code,
            location,
            message: message.into(),
            fr: fr.into(),
        }
    }

    /// The text in French when asked and available, else in English.
    pub fn message_in(&self, french: bool) -> &str {
        if french && !self.fr.is_empty() {
            &self.fr
        } else {
            &self.message
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

/// The languages the student apps have interface texts for.
pub const INTERFACE_LANGUAGES: [&str; 2] = ["en", "fr"];

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Paper {
    pub title: String,
    pub description: String,
    /// Language of the paper text (`fr`, `en`...), empty when not stated.
    pub lang: String,
    /// Interface languages the student may switch between, as written (`fr`, `en`...); empty
    /// means no choice: the interface follows `lang`.
    pub languages: Vec<String>,
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
    /// Author notes written before the first theme: kept in the source, never shown to students.
    pub notes: String,
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
    /// Author notes written between the theme header and its first question.
    pub notes: String,
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
    /// Number of options to tick as announced to students (`answers N`), 0 to derive it from the
    /// options. Student copies carry it because they carry no correct flag.
    pub declared_answers: u32,
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

    /// How many options the student must tick: the declared count, else the correct options.
    pub fn answers_to_tick(&self) -> usize {
        if self.declared_answers > 0 {
            self.declared_answers as usize
        } else {
            self.correct_count()
        }
    }

    /// True when more than one option is to be ticked: the student ticks boxes, not a radio.
    pub fn is_multiple(&self) -> bool {
        self.answers_to_tick() > 1
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
            "mp3" | "ogg" | "oga" | "wav" | "m4a" | "opus" | "flac" | "weba" => {
                Some(MediaKind::Audio)
            }
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

/// Category of a piece of highlighted code; each theme gives each kind a colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Plain,
    Keyword,
    Type,
    Function,
    String,
    Number,
    Constant,
    Comment,
    Operator,
}

impl TokenKind {
    pub const ALL: [TokenKind; 9] = [
        TokenKind::Plain,
        TokenKind::Keyword,
        TokenKind::Type,
        TokenKind::Function,
        TokenKind::String,
        TokenKind::Number,
        TokenKind::Constant,
        TokenKind::Comment,
        TokenKind::Operator,
    ];

    pub fn code(self) -> u8 {
        match self {
            TokenKind::Plain => 0,
            TokenKind::Keyword => 1,
            TokenKind::Type => 2,
            TokenKind::Function => 3,
            TokenKind::String => 4,
            TokenKind::Number => 5,
            TokenKind::Constant => 6,
            TokenKind::Comment => 7,
            TokenKind::Operator => 8,
        }
    }

    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.code() == code)
    }

    /// Stable name, used as a CSS class suffix.
    pub fn name(self) -> &'static str {
        match self {
            TokenKind::Plain => "plain",
            TokenKind::Keyword => "keyword",
            TokenKind::Type => "type",
            TokenKind::Function => "function",
            TokenKind::String => "string",
            TokenKind::Number => "number",
            TokenKind::Constant => "constant",
            TokenKind::Comment => "comment",
            TokenKind::Operator => "operator",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenRun {
    pub kind: TokenKind,
    pub text: String,
}

/// The highlighted form of one code block, computed when the paper is published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeTokens {
    pub question_id: String,
    /// Index of the block among the question's code blocks, from 0.
    pub block: u32,
    /// One entry per source line.
    pub lines: Vec<Vec<TokenRun>>,
}

/// What the student ticked on one question of their session.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Answer {
    pub question_id: String,
    pub selected: Vec<String>,
    pub seconds_spent: u32,
    /// The student reported the question as unclear or flawed, for the teacher to check.
    /// Independent of the answer and of the "to review" flag, which stays on the student's side.
    pub reported: bool,
}

/// A finished copy, as sealed for the teacher.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Submission {
    /// Identifies the published package the copy answers.
    pub paper_id: [u8; 16],
    /// Declared by the student, as on paper.
    pub student_name: String,
    /// One per question of the student's session, in session order.
    pub answers: Vec<Answer>,
    pub elapsed_seconds: u32,
    /// Times the exam window lost focus, recorded and reported, never judged.
    pub focus_losses: u32,
    pub seconds_away: u32,
    /// Every answer change and focus event, in order.
    pub log: Vec<LogEntry>,
    /// Extra time declared by the student, shown to the teacher.
    pub extension: TimeExtension,
}

/// Extra time a student is entitled to (French "tiers-temps" and wider arrangements). The
/// student declares it; the teacher sees it on the copy and checks it against their own list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeExtension {
    #[default]
    None,
    /// One third more.
    Third,
    /// Half as much again.
    Half,
    /// Twice the time.
    Double,
    /// No time limit, no pace alerts (WCAG 2.2.1 "turn off").
    Untimed,
}

impl TimeExtension {
    pub const ALL: [TimeExtension; 5] = [
        TimeExtension::None,
        TimeExtension::Third,
        TimeExtension::Half,
        TimeExtension::Double,
        TimeExtension::Untimed,
    ];

    /// The student's limit from the paper's; 0 means no limit.
    pub fn apply(self, limit_seconds: u32) -> u32 {
        match self {
            TimeExtension::None => limit_seconds,
            TimeExtension::Third => limit_seconds.saturating_add(limit_seconds / 3),
            TimeExtension::Half => limit_seconds.saturating_add(limit_seconds / 2),
            TimeExtension::Double => limit_seconds.saturating_mul(2),
            TimeExtension::Untimed => 0,
        }
    }

    /// Short label for reports and CSV files, the same in every language.
    pub fn label(self) -> &'static str {
        match self {
            TimeExtension::None => "",
            TimeExtension::Third => "+1/3",
            TimeExtension::Half => "+1/2",
            TimeExtension::Double => "x2",
            TimeExtension::Untimed => "untimed",
        }
    }

    pub fn code(self) -> u8 {
        match self {
            TimeExtension::None => 0,
            TimeExtension::Third => 1,
            TimeExtension::Half => 2,
            TimeExtension::Double => 3,
            TimeExtension::Untimed => 4,
        }
    }

    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.code() == code)
    }
}

/// What happened, for the teacher's anomaly report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Tick,
    Untick,
    FocusLost,
    FocusBack,
    Submit,
}

impl LogKind {
    pub fn code(self) -> u8 {
        match self {
            LogKind::Tick => 1,
            LogKind::Untick => 2,
            LogKind::FocusLost => 3,
            LogKind::FocusBack => 4,
            LogKind::Submit => 5,
        }
    }

    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(LogKind::Tick),
            2 => Some(LogKind::Untick),
            3 => Some(LogKind::FocusLost),
            4 => Some(LogKind::FocusBack),
            5 => Some(LogKind::Submit),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    /// Exam seconds elapsed when it happened.
    pub seconds: u32,
    pub kind: LogKind,
    /// Empty for focus and submit events.
    pub question_id: String,
    pub option_id: String,
}

impl LogEntry {
    pub fn new(seconds: u32, kind: LogKind, question_id: &str, option_id: &str) -> Self {
        Self {
            seconds,
            kind,
            question_id: question_id.to_string(),
            option_id: option_id.to_string(),
        }
    }
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
