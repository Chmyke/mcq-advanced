//! Builds a paper question by question, whatever the source format: unique ids, themes found by
//! title, positional option ids, media stored once, and the report of what was left out.

use std::collections::BTreeMap;

use mcq_edit::{Project, ops, slug};
use mcq_model::{AnswerOption, Issue, Location, Paper, Question, Scoring, Severity, Theme};

use crate::{ImportOptions, Imported, Note};

/// The file cannot be read at all.
pub const UNREADABLE: &str = "IMPORT_UNREADABLE";
/// Not a single question could be imported.
pub const EMPTY: &str = "IMPORT_EMPTY";
/// One question was left out: unsupported type or broken syntax.
pub const SKIPPED: &str = "IMPORT_SKIPPED";
/// A question was imported with something simplified or dropped.
pub const SIMPLIFIED: &str = "IMPORT_SIMPLIFIED";
/// A neutral fact about the import.
pub const SUMMARY: &str = "IMPORT_SUMMARY";

/// Options of a question being imported, before ids are given.
#[derive(Debug, Clone, Default)]
pub struct Draft {
    /// Preferred id: an id from the source, kept when it fits a header, or a slug of the
    /// question name. Made unique.
    pub id_hint: String,
    pub question: Question,
}

/// An answer as banks write it: a weight in percent, positive when it earns credit.
#[derive(Debug, Clone, Default)]
pub struct Choice {
    pub fraction: f64,
    pub text: String,
    pub feedback: String,
}

impl Draft {
    /// Options from weighted answers. A single-answer question keeps its best answers as correct;
    /// a multiple-answer one every answer that earns credit. What partial credit cannot keep
    /// goes to `lost`.
    pub fn choices(&mut self, choices: &[Choice], single: bool, lost: &mut Vec<Note>) {
        let best = choices.iter().map(|c| c.fraction).fold(0.0, f64::max);
        for choice in choices {
            let is_correct = if single {
                choice.fraction > 0.0 && (choice.fraction - best).abs() < 1e-6
            } else {
                choice.fraction > 0.0
            };
            self.option(choice.text.clone(), is_correct, choice.feedback.clone());
        }
        let correct = self.question.correct_count();
        if single {
            if choices
                .iter()
                .any(|c| c.fraction > 0.0 && (c.fraction - best).abs() >= 1e-6)
            {
                lost.push(Note::new(
                    "partial credit on some answers is dropped: only the best answer counts",
                    "le crédit partiel de certaines réponses est abandonné : seule la meilleure \
                     réponse compte",
                ));
            }
            if correct > 1 {
                lost.push(Note::new(
                    "the source accepts any one of its best answers; here they must all be ticked",
                    "la source accepte n'importe laquelle de ses meilleures réponses ; ici, il \
                     faut toutes les cocher",
                ));
            }
        } else if correct > 0 {
            let share = 100.0 / correct as f64;
            let standard = choices.iter().all(|c| {
                let f = c.fraction;
                f.abs() < 1e-6 || (f - share).abs() < 0.01 || (f + share).abs() < 0.01
            });
            if !standard {
                lost.push(Note::new(
                    "its own answer weights become standard partial credit",
                    "ses propres pondérations deviennent un crédit partiel standard",
                ));
            }
        }
    }

    pub fn option(&mut self, text: String, is_correct: bool, explanation: String) {
        self.question.options.push(AnswerOption {
            id: String::new(),
            text,
            is_correct,
            explanation,
        });
    }
}

pub struct Builder {
    paper: Paper,
    media: BTreeMap<String, Vec<u8>>,
    issues: Vec<Issue>,
    lang: String,
    default_theme: String,
}

impl Builder {
    pub fn new(options: &ImportOptions) -> Self {
        let mut paper = Paper::new();
        paper.title = options.title.trim().to_string();
        paper.lang = options.lang.trim().to_ascii_lowercase();
        Self {
            paper,
            media: BTreeMap::new(),
            issues: Vec::new(),
            lang: options.lang.clone(),
            default_theme: String::new(),
        }
    }

    pub fn paper_mut(&mut self) -> &mut Paper {
        &mut self.paper
    }

    /// The true and false option texts in the paper language.
    pub fn true_false(&self) -> (&'static str, &'static str) {
        if self.lang.starts_with("fr") {
            ("Vrai", "Faux")
        } else {
            ("True", "False")
        }
    }

    /// The id of the theme with this title, created when new. `id_hint` names a new theme.
    pub fn theme(&mut self, title: &str, id_hint: &str) -> String {
        let title = title.trim();
        if let Some(theme) = self.paper.themes.iter().find(|t| t.title == title) {
            return theme.id.clone();
        }
        let base = match safe_id(id_hint) {
            s if s.is_empty() => match slug(title) {
                s if s.is_empty() => "theme".to_string(),
                s => s,
            },
            s => s,
        };
        let id = unique(&base, |id| self.paper.theme(id).is_some());
        self.paper.themes.push(Theme {
            id: id.clone(),
            title: if title.is_empty() {
                id.clone()
            } else {
                title.to_string()
            },
            ..Theme::default()
        });
        id
    }

    pub fn theme_mut(&mut self, id: &str) -> Option<&mut Theme> {
        self.paper.themes.iter_mut().find(|t| t.id == id)
    }

    /// The theme of questions whose source names none: one per import, titled like the paper.
    pub fn default_theme(&mut self) -> String {
        if self.default_theme.is_empty() {
            let title = if self.paper.title.is_empty() {
                "Questions".to_string()
            } else {
                self.paper.title.clone()
            };
            self.default_theme = self.theme(&title, "");
        }
        self.default_theme.clone()
    }

    /// Adds a question at the end of its theme and returns its id.
    pub fn question(&mut self, theme_id: &str, draft: Draft) -> String {
        let id = match safe_id(&draft.id_hint) {
            s if s.is_empty() => ops::next_question_id(&self.paper, theme_id),
            s => unique(&s, |id| self.paper.question(id).is_some()),
        };
        let mut question = draft.question;
        question.id = id.clone();
        question.theme_id = theme_id.to_string();
        question.line = 0;
        ops::renumber_options(&mut question);
        let at = self
            .paper
            .questions
            .iter()
            .rposition(|q| q.theme_id == theme_id)
            .map_or(self.paper.questions.len(), |k| k + 1);
        self.paper.questions.insert(at, question);
        id
    }

    /// Stores a media file and returns its path in the paper, `None` when it is not a media file.
    pub fn media(&mut self, file_name: &str, data: Vec<u8>) -> Option<String> {
        let mut project = Project {
            paper: Paper::default(),
            media: std::mem::take(&mut self.media),
        };
        let path = project.add_media(file_name, data);
        self.media = project.media;
        path
    }

    pub fn report(&mut self, severity: Severity, code: &'static str, at: Location, text: Note) {
        self.issues
            .push(Issue::new(severity, code, at, text.en, text.fr));
    }

    pub fn skip(&mut self, at: Location, en: String, fr: String) {
        self.report(Severity::Warning, SKIPPED, at, Note { en, fr });
    }

    /// `what` was lost on the question named `label`.
    pub fn simplified(&mut self, at: Location, label: &str, what: Note) {
        let text = Note::new(
            format!("`{label}`: {}.", what.en),
            format!("`{label}` : {}.", what.fr),
        );
        self.report(Severity::Warning, SIMPLIFIED, at, text);
    }

    pub fn unreadable(mut self, en: String, fr: String) -> Imported {
        self.report(
            Severity::Error,
            UNREADABLE,
            Location::Paper,
            Note { en, fr },
        );
        self.finish()
    }

    /// Partial credit as soon as one question has several answers: that is how Moodle, GIFT and
    /// most banks grade them. The paper is otherwise left with the documented defaults.
    pub fn finish(mut self) -> Imported {
        let multiple = self.paper.questions.iter().any(|q| q.correct_count() > 1);
        if multiple && self.paper.scoring != Scoring::Partial {
            self.paper.scoring = Scoring::Partial;
            self.report(
                Severity::Info,
                SUMMARY,
                Location::Paper,
                Note::new(
                    "Some questions have several correct answers: the paper uses partial credit, \
                     as Moodle does.",
                    "Certaines questions ont plusieurs bonnes réponses : le sujet utilise le \
                     crédit partiel, comme Moodle.",
                ),
            );
        }
        let count = self.paper.questions.len();
        if count == 0 {
            if !self.issues.iter().any(|i| i.code == UNREADABLE) {
                self.report(
                    Severity::Error,
                    EMPTY,
                    Location::Paper,
                    Note::new(
                        "No question could be imported from this file.",
                        "Aucune question n'a pu être importée depuis ce fichier.",
                    ),
                );
            }
        } else {
            let themes = self.paper.themes.len();
            self.report(
                Severity::Info,
                SUMMARY,
                Location::Paper,
                Note::new(
                    format!("{count} question(s) imported in {themes} theme(s)."),
                    format!("{count} question(s) importée(s) dans {themes} thème(s)."),
                ),
            );
        }
        // Themes created for questions that were skipped afterwards would stay empty.
        let used: Vec<String> = self
            .paper
            .questions
            .iter()
            .map(|q| q.theme_id.clone())
            .collect();
        self.paper.themes.retain(|t| used.contains(&t.id));
        Imported {
            project: Project {
                paper: self.paper,
                media: self.media,
            },
            issues: self.issues,
        }
    }
}

/// The names of a Moodle category path under its context and `top`, outermost first. A `/`
/// inside a name is written `//`.
pub fn category_path(raw: &str) -> Vec<String> {
    let mut path: Vec<String> = raw
        .replace("//", "\u{0}")
        .split('/')
        .map(|s| s.replace('\u{0}', "/").trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if path.first().is_some_and(|s| s.starts_with('$')) {
        path.remove(0);
    }
    if path.first().is_some_and(|s| s == "top") {
        path.remove(0);
    }
    path
}

/// The hint itself when it can be written in a header (exports of this tool keep their ids),
/// else its slug.
fn safe_id(hint: &str) -> String {
    let hint = hint.trim();
    if ops::check_id(hint).is_ok() {
        hint.to_string()
    } else {
        slug(hint)
    }
}

/// `base`, or `base-2`, `base-3`... the first one not taken.
fn unique(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_string();
    }
    let mut n = 2;
    loop {
        let id = format!("{base}-{n}");
        if !taken(&id) {
            return id;
        }
        n += 1;
    }
}
