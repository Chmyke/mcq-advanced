//! ROLE:    exchange with other tools: imports GIFT, Moodle XML, Aiken and CSV question banks,
//!          exports Moodle XML, and prints a paper as a Markdown handout without its answers.
//! OWNER:   stateless; each call owns what it builds and returns it.
//! INVARIANTS: no I/O, no clock; never panics on any input; every question that cannot be
//!             imported, and every detail lost on the way, is reported as an `Issue`.
//!
//! The mapping between formats is specified in `docs/exchange.md`.

pub mod aiken;
pub mod csv;
pub mod gift;
pub mod handout;
pub mod moodle;

mod base64;
mod build;
mod html;
mod text;

use mcq_edit::Project;
use mcq_model::Issue;

/// A question bank format that can be imported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Gift,
    MoodleXml,
    Aiken,
    Csv,
}

impl Source {
    pub const ALL: [Source; 4] = [Source::Gift, Source::MoodleXml, Source::Aiken, Source::Csv];

    /// Command line name.
    pub fn name(self) -> &'static str {
        match self {
            Source::Gift => "gift",
            Source::MoodleXml => "moodle",
            Source::Aiken => "aiken",
            Source::Csv => "csv",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|s| s.name().eq_ignore_ascii_case(name))
    }

    /// Guesses the format from the file extension, then from the content for `.txt` files,
    /// which hold GIFT or Aiken alike.
    pub fn detect(file_name: &str, text: &str) -> Option<Self> {
        let ext = file_name
            .rsplit_once('.')
            .map(|(_, ext)| ext.to_ascii_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            "gift" => Some(Source::Gift),
            "xml" => Some(Source::MoodleXml),
            "csv" | "tsv" => Some(Source::Csv),
            "txt" => {
                let aiken = text.lines().any(|l| l.trim_start().starts_with("ANSWER:"));
                if aiken {
                    Some(Source::Aiken)
                } else if text.contains('{') {
                    Some(Source::Gift)
                } else {
                    None
                }
            }
            _ if text.trim_start().starts_with("<?xml") || text.contains("<quiz") => {
                Some(Source::MoodleXml)
            }
            _ => None,
        }
    }
}

/// What every import needs to know beyond the file itself.
#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    /// Paper title when the file does not carry one, usually the file name without extension.
    pub title: String,
    /// Language of the paper (`fr`, `en`...): it names the true and false options.
    pub lang: String,
}

/// A paper built from another format, with what was skipped or simplified on the way.
#[derive(Debug, Clone, PartialEq)]
pub struct Imported {
    pub project: Project,
    pub issues: Vec<Issue>,
}

/// One sentence of a report, in English and French, before it becomes part of an `Issue`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Note {
    pub en: String,
    pub fr: String,
}

impl Note {
    pub fn new(en: impl Into<String>, fr: impl Into<String>) -> Self {
        Self {
            en: en.into(),
            fr: fr.into(),
        }
    }
}

/// Imports a question bank.
pub fn import(source: Source, text: &str, options: &ImportOptions) -> Imported {
    match source {
        Source::Gift => gift::import(text, options),
        Source::MoodleXml => moodle::import(text, options),
        Source::Aiken => aiken::import(text, options),
        Source::Csv => csv::import(text, options),
    }
}
