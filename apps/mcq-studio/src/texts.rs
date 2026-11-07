//! Interface texts of the Studio, in English and French. The paper's own text is never translated;
//! the validator's messages come from the core crates in both languages, shown in the interface
//! language with a link to the rules guide.

use mcq_model::{Issue, Location, Paper, Severity};
pub use mcq_player::texts::Lang;

#[derive(PartialEq)]
pub struct Texts {
    pub brand: &'static str,
    pub space_write: &'static str,
    pub space_settings: &'static str,
    pub space_publish: &'static str,
    pub new_paper: &'static str,
    pub new_paper_title: &'static str,
    pub confirm_replace: &'static str,
    pub import: &'static str,
    pub paste: &'static str,
    pub tip_paste: &'static str,
    /// The browser kept the clipboard to itself.
    pub paste_refused: &'static str,
    pub paste_not_paper: &'static str,
    /// Pasted text in a known format that cannot be read; the reason follows.
    pub paste_failed: &'static str,
    /// The title of a paper pasted without one, and of the theme of its questions without a
    /// category.
    pub pasted_title: &'static str,
    /// The help menu: the website's tutorial, its writing guide, and the AI prompt.
    pub help: &'static str,
    pub tip_help: &'static str,
    pub help_tutorial: &'static str,
    pub help_guide: &'static str,
    /// Importing over a paper that has questions: `{mine}` and `{theirs}` count them.
    pub import_choice: &'static str,
    /// The same question when the questions come from pasted text.
    pub import_choice_pasted: &'static str,
    pub import_add: &'static str,
    pub import_replace: &'static str,
    pub cancel: &'static str,
    /// After an import added to the paper; `{n}`.
    pub imported_added: &'static str,
    /// The themes a bank's questions went to, to rename if needed; `{themes}`.
    pub imported_themes: &'static str,
    /// The link from a validator message to its explanation; the code is its tip.
    pub why: &'static str,
    pub welcome_title: &'static str,
    pub welcome_text: &'static str,
    pub welcome_write: &'static str,
    pub welcome_write_hint: &'static str,
    pub welcome_import_hint: &'static str,
    pub copy_prompt: &'static str,
    pub welcome_ai_hint: &'static str,
    pub welcome_demo: &'static str,
    pub welcome_demo_hint: &'static str,
    pub prompt_copied: &'static str,
    pub prompt_saved: &'static str,
    pub prompt_file: &'static str,
    /// In place of the validator's list while the question is still empty.
    pub untouched_hint: &'static str,
    pub export_qcmz: &'static str,
    pub export_source: &'static str,
    pub export_student: &'static str,
    pub export_moodle: &'static str,
    pub theme_light: &'static str,
    pub theme_dark: &'static str,
    pub restored: &'static str,
    pub imported: &'static str,
    pub link_failed: &'static str,
    pub not_a_paper: &'static str,
    pub not_a_media: &'static str,
    pub no_question_for_media: &'static str,
    pub mic_unavailable: &'static str,
    pub saved_locally: &'static str,
    pub close: &'static str,
    pub tip_new: &'static str,
    pub tip_import: &'static str,
    pub export: &'static str,
    pub tip_export: &'static str,
    pub tip_theme_light: &'static str,
    pub tip_theme_dark: &'static str,
    pub tip_space_write: &'static str,
    pub tip_space_settings: &'static str,
    pub tip_space_publish: &'static str,
    pub tip_space_correct: &'static str,
    pub tip_add_question: &'static str,
    pub tip_add_answer: &'static str,
    pub tip_add_theme: &'static str,
    pub tip_add_text: &'static str,
    pub tip_add_code: &'static str,
    pub tip_add_formula: &'static str,
    pub tip_add_media: &'static str,
    pub tip_record: &'static str,
    pub tip_question_up: &'static str,
    pub tip_question_down: &'static str,
    pub tip_duplicate: &'static str,
    pub tip_delete: &'static str,
    pub tip_answer_up: &'static str,
    pub tip_answer_down: &'static str,
    pub remove: &'static str,
    pub stop: &'static str,
    pub tip_theme_up: &'static str,
    pub tip_theme_down: &'static str,
    pub tip_preview_computer: &'static str,
    pub tip_preview_phone: &'static str,
    pub tip_preview_light: &'static str,
    pub tip_preview_dark: &'static str,
    pub print: &'static str,
    pub download: &'static str,
    pub tip_download_again: &'static str,

    pub questions: &'static str,
    pub add_question: &'static str,
    pub drag_question: &'static str,
    pub no_statement: &'static str,
    pub state_error: &'static str,
    pub state_warning: &'static str,
    pub state_ok: &'static str,
    pub kind_label: &'static str,
    pub empty_paper: &'static str,

    pub question: &'static str,
    pub id: &'static str,
    pub id_empty: &'static str,
    pub id_bad: &'static str,
    pub id_taken: &'static str,
    pub theme: &'static str,
    pub level: &'static str,
    /// What the levels mean (Bloom's), in one sentence: the tip of the level menu.
    pub level_hint: &'static str,
    pub level_short: &'static str,
    pub levels: [&'static str; 6],
    pub seconds: &'static str,
    pub seconds_auto: &'static str,
    pub kind_single: &'static str,
    pub kind_multiple: &'static str,
    pub kind_true_false: &'static str,
    pub move_up: &'static str,
    pub move_down: &'static str,
    pub duplicate: &'static str,
    pub delete: &'static str,

    pub statement: &'static str,
    pub add_text: &'static str,
    pub add_code: &'static str,
    pub add_formula: &'static str,
    pub formula_hint: &'static str,
    pub add_media: &'static str,
    pub record: &'static str,
    pub stop_recording: &'static str,
    pub drop_hint: &'static str,
    pub text_block: &'static str,
    pub code_block: &'static str,
    pub language: &'static str,
    pub highlighted_lines: &'static str,
    pub alt_text: &'static str,
    pub alt_hint: &'static str,
    pub block_up: &'static str,
    pub block_down: &'static str,
    pub block_remove: &'static str,

    pub answers: &'static str,
    /// Above the answers of a multiple-answer question.
    pub answers_hint: &'static str,
    /// Above the answers of a single-answer or true-or-false question.
    pub answers_hint_single: &'static str,
    pub answer_placeholder: &'static str,
    pub explanation_placeholder: &'static str,
    pub correct: &'static str,
    pub add_answer: &'static str,
    pub remove_answer: &'static str,
    pub rule: &'static str,
    pub review: &'static str,

    pub validator: &'static str,
    pub class_results: &'static str,
    /// `{percent}`, `{asked}`, `{blank}`, `{seconds}`.
    pub class_summary: &'static str,
    pub class_small: &'static str,
    /// `{percent}`, `{picked}`.
    pub class_answer: &'static str,
    pub class_correct: &'static str,
    pub class_unmatched: &'static str,
    pub no_issue: &'static str,
    pub severity_error: &'static str,
    pub severity_warning: &'static str,
    pub severity_info: &'static str,
    /// Names of the places an issue points to: the paper, a line, a question, a theme.
    pub at_paper: &'static str,
    pub at_line: &'static str,
    pub at_question: &'static str,
    pub at_theme: &'static str,
    pub rules_guide: &'static str,

    pub preview: &'static str,
    pub preview_computer: &'static str,
    pub preview_phone: &'static str,

    pub settings_paper: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub paper_lang: &'static str,
    pub paper_languages: &'static str,
    pub paper_languages_hint: &'static str,
    pub notes: &'static str,
    pub settings_time: &'static str,
    pub duration: &'static str,
    pub estimated: &'static str,
    /// How the estimate is made, and how to correct it.
    pub estimated_hint: &'static str,
    /// The time limit never closes a copy: said to the teacher as to the students.
    pub duration_soft: &'static str,
    /// The paper's language when none is written.
    pub lang_unset: &'static str,
    pub minutes: &'static str,
    pub settings_scoring: &'static str,
    pub scoring_all_or_nothing: &'static str,
    pub scoring_partial: &'static str,
    pub penalty: &'static str,
    pub unanswered_penalty: &'static str,
    pub settings_exam: &'static str,
    pub shuffle_questions: &'static str,
    pub shuffle_options: &'static str,
    pub show_answer_count: &'static str,
    pub allow_extra_time: &'static str,
    /// The fold holding a question's id and time.
    pub advanced: &'static str,
    pub themes: &'static str,
    pub theme_title: &'static str,
    pub course_ref: &'static str,
    pub add_theme: &'static str,
    pub new_theme: &'static str,
    pub remove_theme: &'static str,
    pub paper_findings: &'static str,

    pub publish_blocked: &'static str,
    pub identity: &'static str,
    pub identity_hint: &'static str,
    pub load_identity: &'static str,
    pub passphrase: &'static str,
    pub unlock: &'static str,
    pub create_identity: &'static str,
    pub new_passphrase: &'static str,
    pub confirm_passphrase: &'static str,
    pub create_and_download: &'static str,
    pub identity_ready: &'static str,
    /// What the fingerprint is for, under it.
    pub fingerprint_hint: &'static str,
    pub identity_saved: &'static str,
    pub identity_download_again: &'static str,
    pub identity_kept: &'static str,
    pub identity_forget: &'static str,
    /// With an identity kept in the browser, creating another one is folded behind this link.
    pub create_other_identity: &'static str,
    /// Shown once unfolded: the copies of papers published so far open with the kept one only.
    pub create_other_warning: &'static str,
    pub identity_keep: &'static str,
    /// `{n}`: warnings left when publishing is allowed.
    pub publish_warnings: &'static str,
    pub publish_advice: &'static str,
    pub severity_advice: &'static str,
    /// The repeated advice summed up for the paper, in the order of `state::REPEATED_ADVICE`.
    pub advice_summaries: [&'static str; 4],
    pub distribute_title: &'static str,
    /// `{file}` is the package's name.
    pub distribute_steps: [&'static str; 4],
    pub wrong_passphrase: &'static str,
    pub passphrases_differ: &'static str,
    pub passphrase_too_short: &'static str,
    pub mode: &'static str,
    pub mode_exam: &'static str,
    pub mode_practice: &'static str,
    /// Checkbox, then what the code is for, under it.
    pub with_opening_code: &'static str,
    pub opening_code_help: &'static str,
    pub with_end_code: &'static str,
    pub end_code_help: &'static str,
    pub publish: &'static str,
    pub working: &'static str,
    pub published: &'static str,
    pub download_again: &'static str,
    pub opening_code: &'static str,
    pub end_code: &'static str,
    pub end_code_hint: &'static str,
    pub fingerprint: &'static str,
    pub print_sheet: &'static str,
    pub sheet_questions: &'static str,
    pub sheet_duration: &'static str,
    pub no_limit: &'static str,
    pub sheet_steps: [&'static str; 4],
    /// Replaces the third step when the paper opens without a code.
    pub sheet_name_only: &'static str,
    pub sheet_correction_later: &'static str,
    pub sheet_practice: &'static str,
    pub space_correct: &'static str,
    pub correction: &'static CorrectTexts,
    pub demo: &'static DemoTexts,
    /// Picks the language of the validator's messages.
    pub lang: Lang,
}

impl Texts {
    /// A validator message in the interface language.
    /// A message of the core as a teacher reads it: names written as code (`Q7`) are quoted
    /// instead, as in the rest of the interface.
    pub fn issue(&self, issue: &Issue) -> String {
        self.quoted(issue.message_in(self.lang == Lang::Fr))
    }

    /// `text` between the language's quotation marks.
    pub fn quote(&self, text: &str) -> String {
        match self.lang {
            Lang::Fr => format!("« {text} »"),
            Lang::En => format!("“{text}”"),
        }
    }

    /// Every `code` span of `text` between the language's quotation marks.
    pub fn quoted(&self, text: &str) -> String {
        text.split('`')
            .enumerate()
            .map(|(k, part)| {
                if k % 2 == 1 {
                    self.quote(part)
                } else {
                    part.to_string()
                }
            })
            .collect()
    }

    /// A validator message as the editor shows it: where the core names the file's syntax
    /// (`RULE:`, `[x]`...), the editor names the field to fill instead.
    pub fn form_issue(&self, issue: &Issue) -> String {
        let fields: [(&str, String); 5] = match self.lang {
            Lang::Fr => [
                (
                    "marquez la bonne avec `[x]`",
                    "cochez le rond ou la case de la bonne réponse".to_string(),
                ),
                ("ajoutez `RULE:`", format!("remplissez « {} »", self.rule)),
                (
                    "ajoutez `COURSE:` au thème ou `REVIEW:` à la question",
                    format!(
                        "remplissez « {} » sous la question, ou celui du thème dans {}",
                        self.review, self.space_settings
                    ),
                ),
                (
                    "ajoutez de `level 1` à `level 5`",
                    format!("choisissez-le dans « {} »", self.level),
                ),
                (
                    "ajoutez `title:` dans l'en-tête",
                    format!("remplissez « {} » dans {}", self.title, self.space_settings),
                ),
            ],
            Lang::En => [
                (
                    "mark it with `[x]`",
                    "tick the circle or box beside the right answer".to_string(),
                ),
                ("add `RULE:`", format!("fill in “{}”", self.rule)),
                (
                    "add `COURSE:` to the theme or `REVIEW:` to the question",
                    format!(
                        "fill in “{}” under the question, or the theme's in {}",
                        self.review, self.space_settings
                    ),
                ),
                (
                    "add `level 1` to `level 5`",
                    format!("choose one in “{}”", self.level),
                ),
                (
                    "add `title:` to the header",
                    format!("fill in “{}” in {}", self.title, self.space_settings),
                ),
            ],
        };
        let mut text = issue.message_in(self.lang == Lang::Fr).to_string();
        for (syntax, field) in &fields {
            text = text.replace(syntax, field);
        }
        self.quoted(&text)
    }

    /// How many errors and warnings the validator reports, as a screen reader announces it.
    pub fn issue_count(&self, issues: &[&Issue]) -> String {
        let count = |severity| issues.iter().filter(|i| i.severity == severity).count();
        let (errors, warnings) = (count(Severity::Error), count(Severity::Warning));
        let words = match self.lang {
            Lang::Fr => [
                ("aucune erreur", "erreur", "erreurs"),
                ("aucun avertissement", "avertissement", "avertissements"),
            ],
            Lang::En => [
                ("no error", "error", "errors"),
                ("no warning", "warning", "warnings"),
            ],
        };
        let say = |n: usize, (none, one, many): (&str, &str, &str)| match n {
            0 => none.to_string(),
            1 => format!("1 {one}"),
            n => format!("{n} {many}"),
        };
        let text = format!("{}, {}", say(errors, words[0]), say(warnings, words[1]));
        // A sentence starts with a capital.
        let mut chars = text.chars();
        chars.next().map_or(text.clone(), |first| {
            first.to_uppercase().chain(chars).collect()
        })
    }

    /// The sentence that sums up a repeated advice for the whole paper.
    pub fn advice_summary(&self, code: &str) -> &'static str {
        crate::state::REPEATED_ADVICE
            .iter()
            .position(|c| *c == code)
            .map_or("", |k| self.advice_summaries[k])
    }

    /// Where an issue was found, in the interface language, as the prefix of its message.
    /// A theme is named by its title, as the teacher wrote it, not by its id in the file.
    pub fn place(&self, location: &Location, paper: &Paper) -> String {
        let place = match location {
            Location::Paper => self.at_paper.to_string(),
            Location::Line(n) => format!("{} {n}", self.at_line),
            Location::Question(id) => format!("{} {id}", self.at_question),
            Location::Theme(id) => {
                let title = paper
                    .themes
                    .iter()
                    .find(|theme| &theme.id == id && !theme.title.trim().is_empty())
                    .map_or(id.as_str(), |theme| theme.title.as_str());
                match self.lang {
                    Lang::Fr => format!("{} « {title} »", self.at_theme),
                    Lang::En => format!("{} “{title}”", self.at_theme),
                }
            }
        };
        // French puts a space before the colon.
        let colon = if self.lang == Lang::Fr { " :" } else { ":" };
        format!("{place}{colon} ")
    }
}

/// The desktop window keeps the draft in its own web view's storage, on this computer: no browser
/// for the teacher to think of.
const ON_DESKTOP: bool = cfg!(feature = "desktop");

/// The Studio's texts in the interface language.
pub fn texts(lang: Lang) -> &'static Texts {
    match lang {
        Lang::En => &EN,
        Lang::Fr => &FR,
    }
}

pub static EN: Texts = Texts {
    brand: "MCQ Advanced Studio",
    space_write: "Write",
    space_settings: "Settings",
    space_publish: "Publish",
    new_paper: "New",
    new_paper_title: "New paper",
    confirm_replace: "Replace the current paper? Export it first to keep it.",
    import: "Import",
    paste: "Paste",
    tip_paste: "Paste a paper copied from an AI's answer or a text editor",
    paste_refused: "The browser did not give access to the clipboard: click an empty part of the page and press Ctrl+V.",
    paste_not_paper: "The clipboard holds no paper the Studio can read. Pasted as text, it reads: the whole answer of an AI given the Studio's prompt (Help menu), a .qcm.md paper, and Moodle XML, GIFT, Aiken or CSV banks (CSV with its header row). For a file, use Import.",
    paste_failed: "The pasted text could not be read",
    pasted_title: "Imported questions",
    help: "Help",
    tip_help: "Help: getting started, writing guide, AI prompt",
    help_tutorial: "Getting started (website)",
    help_guide: "Writing a good quiz (website)",
    import_choice: "Your paper has {mine} {mine:question|questions}; the file brings {theirs}. Add them after yours, or replace your paper?",
    import_choice_pasted: "Your paper has {mine} {mine:question|questions}; the pasted text brings {theirs}. Add them after yours, or replace your paper?",
    import_add: "Add to my paper",
    import_replace: "Replace my paper",
    cancel: "Cancel",
    imported_added: "{n} {n:question added|questions added} at the end of your paper.",
    imported_themes: "{count:Theme|Themes} of the imported questions: {themes}. Rename in Settings if needed.",
    why: "Why?",
    welcome_title: "Write a quiz",
    welcome_text: "Four ways to start. Whatever you pick, everything stays in this browser: no account, no server.",
    welcome_write: "Write a question",
    welcome_write_hint: "Type it here: the validator comments as you write and the student preview follows below.",
    welcome_import_hint: "A paper (.qcmz, .qcm.md) or a Moodle XML, GIFT, Aiken or CSV bank.",
    copy_prompt: "Copy the AI prompt",
    welcome_ai_hint: "Paste it into your AI with your course at the end, copy its whole answer, then press Paste.",
    welcome_demo: "See a sample class",
    welcome_demo_hint: "A fictional paper and 32 copies already corrected, with a short tour.",
    prompt_copied: "Prompt copied: paste it into your AI, add your course at the end, then press Paste here with its answer.",
    prompt_saved: "The browser kept the clipboard to itself: the prompt was saved as a text file to open and copy.",
    prompt_file: "mcq-advanced-ai-prompt.txt",
    untouched_hint: "The validator comments on the question as soon as you write it.",
    export_qcmz: "Export .qcmz",
    export_source: "Export .qcm.md",
    export_student: "Paper without answers",
    export_moodle: "Export Moodle XML",
    theme_light: "Light",
    theme_dark: "Dark",
    restored: if ON_DESKTOP {
        "Your draft was restored from this computer."
    } else {
        "Your draft was restored from this browser."
    },
    imported: "Paper imported.",
    link_failed: "The paper named by the link could not be loaded.",
    not_a_paper: "This file is neither a paper (.qcmz, .qcm.md) nor a bank (GIFT, Moodle XML, Aiken, CSV)",
    not_a_media: "Only images, sounds and videos can be added to a statement.",
    no_question_for_media: "Select a question before adding media.",
    mic_unavailable: "The microphone is not available (refused or missing).",
    saved_locally: if ON_DESKTOP {
        "Saved on this computer"
    } else {
        "Saved in this browser"
    },
    close: "Close",
    tip_new: "Start a new, empty paper",
    tip_import: "Import a paper (.qcmz, .qcm.md, Moodle XML, GIFT, Aiken, CSV)",
    export: "Export",
    tip_export: "Export your paper to a file",
    tip_theme_light: "Switch to the light theme",
    tip_theme_dark: "Switch to the dark theme",
    tip_space_write: "Write and edit your questions",
    tip_space_settings: "Settings of the paper: title, time, scoring, themes",
    tip_space_publish: "Publish your paper for your students",
    tip_space_correct: "Correct your students' copies",
    tip_add_question: "Add a question to this theme",
    tip_add_answer: "Add an answer",
    tip_add_theme: "Add a theme",
    tip_add_text: "Add a text paragraph",
    tip_add_code: "Add a code block",
    tip_add_formula: "Add a formula",
    tip_add_media: "Add a media: image, sound or video",
    tip_record: "Record a sound with the microphone",
    tip_question_up: "Move this question up",
    tip_question_down: "Move this question down",
    tip_duplicate: "Duplicate this question",
    tip_delete: "Delete this question",
    tip_answer_up: "Move this answer up",
    tip_answer_down: "Move this answer down",
    remove: "Remove",
    stop: "Stop",
    tip_theme_up: "Move this theme up",
    tip_theme_down: "Move this theme down",
    tip_preview_computer: "Preview on a computer",
    tip_preview_phone: "Preview on a phone",
    tip_preview_light: "Preview in the light theme",
    tip_preview_dark: "Preview in the dark theme",
    print: "Print",
    download: "Download",
    tip_download_again: "Download the package again",

    questions: "Questions",
    add_question: "Question",
    drag_question: "Drag onto another theme to move it there.",
    no_statement: "(no statement yet)",
    state_error: "error",
    state_warning: "to check",
    state_ok: "ready",
    kind_label: "Question type",
    empty_paper: "No question yet: add one from the list.",

    question: "Question",
    id: "Question reference",
    id_empty: "A reference cannot be empty.",
    id_bad: "No space and no | in a reference.",
    id_taken: "Another question already has this reference.",
    theme: "Theme",
    level: "Level",
    level_short: "What the question asks of the student, from remembering a fact to judging; it also sets the estimated time.",
    level_hint: "Bloom's levels: what the question asks of the student, from 1 (remember a fact) to 5 (evaluate, judge); it also sets the estimated time.",
    levels: [
        "Not set",
        "1 Remember",
        "2 Understand",
        "3 Apply",
        "4 Analyse",
        "5 Evaluate",
    ],
    seconds: "Time (s)",
    seconds_auto: "from level",
    kind_single: "Single answer",
    kind_multiple: "Multiple answers",
    kind_true_false: "True or false",
    move_up: "Up",
    move_down: "Down",
    duplicate: "Duplicate",
    delete: "Delete",

    statement: "Statement",
    add_text: "Text",
    add_code: "Code",
    add_formula: "Formula",
    formula_hint: "Formulas: $x^2$ inside a sentence, $$x^2$$ on their own line, \\$ for a dollar sign.",
    add_media: "Media",
    record: "Record",
    stop_recording: "Stop recording",
    drop_hint: "Drop or paste an image, a sound or a video anywhere.",
    text_block: "Text",
    code_block: "Code",
    language: "Language (optional, colours computer code)",
    highlighted_lines: "Lines to highlight (optional)",
    alt_text: "Text alternative",
    alt_hint: "What a student who cannot see or hear it should know.",
    block_up: "Move this block up",
    block_down: "Move this block down",
    block_remove: "Remove this block",

    answers: "Answers",
    answers_hint: "Tick the correct answers.",
    answers_hint_single: "Tick the correct answer.",
    answer_placeholder: "Answer",
    explanation_placeholder: "Why this answer is right or wrong",
    correct: "Correct answer",
    add_answer: "Answer",
    remove_answer: "Remove this answer",
    rule: "Rule to remember",
    review: "Where to revise",

    validator: "Validator",
    class_results: "The class's results",
    class_summary: "Got right by {percent} % of {asked} copies · {blank} blank · {seconds} s on average",
    class_small: "Small class: a tendency to confirm.",
    class_answer: "{percent} % ({picked})",
    class_correct: "correct in the key",
    class_unmatched: "new answer, no figure",
    no_issue: "Nothing to fix.",
    severity_error: "Error",
    severity_warning: "Warning",
    severity_info: "Info",
    at_paper: "paper",
    at_line: "line",
    at_question: "question",
    at_theme: "theme",
    rules_guide: "Rules guide",

    preview: "Student preview",
    preview_computer: "Computer",
    preview_phone: "Phone",

    settings_paper: "Paper",
    title: "Title of the paper",
    description: "Introduction shown to students",
    paper_lang: "Language of the paper",
    paper_languages: "Interface languages offered to students",
    paper_languages_hint: "None ticked: no language switch, the interface follows the paper's language.",
    notes: "Author notes (never shown to students)",
    settings_time: "Time",
    duration: "Duration in minutes (0: no limit)",
    estimated: "Estimated answering time: at least",
    estimated_hint: "From each question's level, plus a fifth for margin; the length of a statement, a calculation or code is not counted. A question that takes longer gets its own time in its More settings.",
    duration_soft: "The time never closes a copy: going over is only noted on it, the mark does not change.",
    lang_unset: "Not set",
    minutes: "min",
    settings_scoring: "Scoring",
    scoring_all_or_nothing: "All or nothing",
    scoring_partial: "Partial credit",
    penalty: "Points lost per wrong answer",
    unanswered_penalty: "Points lost per blank question",
    settings_exam: "Exam",
    shuffle_questions: "Shuffle the questions",
    shuffle_options: "Shuffle the answers",
    show_answer_count: "Tell how many answers to tick",
    allow_extra_time: "Let students declare extra time (a third, a half, double, untimed); their copy shows it",
    advanced: "More settings: reference and time",
    themes: "Themes",
    theme_title: "Name of the theme",
    course_ref: "Where to revise",
    add_theme: "Theme",
    new_theme: "New theme",
    remove_theme: "Delete this empty theme",
    paper_findings: "About the whole paper",

    publish_blocked: "Fix the errors shown in Write before publishing.",
    identity: "Teacher identity",
    identity_hint: "It signs your papers and is the only key to your students' copies. Keep the file and its passphrase: without them, copies cannot be read.",
    load_identity: "Load my identity (.mcqid)",
    passphrase: "Passphrase",
    unlock: "Unlock",
    create_identity: "Create an identity",
    new_passphrase: "New passphrase (12 characters or more)",
    confirm_passphrase: "Same passphrase again",
    create_and_download: "Create and download",
    identity_ready: "Identity ready. Fingerprint:",
    fingerprint_hint: "The fingerprint is a short code that names your identity: students see it when they open your paper and can check it against the room sheet, so a paper signed by someone else shows.",
    identity_saved: if ON_DESKTOP {
        "Your identity was saved as teacher.mcqid, in the folder you chose. Keep it somewhere safe (a USB key, your cloud storage): with its passphrase, it is the only key to your students' copies."
    } else {
        "Your identity was saved as teacher.mcqid, usually in your Downloads folder. Put it somewhere safe (a USB key, your cloud storage): with its passphrase, it is the only key to your students' copies."
    },
    identity_download_again: "Download my identity again",
    identity_kept: "Your identity is kept in this browser: type its passphrase to unlock it.",
    identity_forget: "Forget the identity kept in this browser",
    create_other_identity: "Create another identity",
    create_other_warning: "Careful: the copies of the papers published with your current identity open with it alone. Create another one only if you have lost it for good.",
    identity_keep: "Keep my identity in this browser (still locked by its passphrase)",
    publish_warnings: "The validator still has {n} {n:warning|warnings}: you can publish, but read them in Write first.",
    publish_advice: "{n} {n:question|questions} could get a fuller correction (explanations, rule to remember, where to revise): optional, it helps students learn from their mistakes.",
    severity_advice: "Advice",
    advice_summaries: [
        "{n} {n:question explains|questions explain} none of the answers.",
        "{n} {n:question has|questions have} answers without an explanation.",
        "{n} {n:question has|questions have} no rule to remember.",
        "{n} {n:question says|questions say} nowhere where to revise.",
    ],
    distribute_title: "How students take it",
    distribute_steps: [
        "Send {file} to your students (school platform, email, USB key) or put it online.",
        "They open it in the player, in any browser, nothing to install: https://mickaelmancini.com/mcq-advanced/player/ (or in the MCQ Advanced app).",
        "When the exam starts, give the opening code: it is on the room sheet below.",
        "At the end, each student saves their copy (a .mcqres file) and sends it to you: open them in Correct.",
    ],
    wrong_passphrase: "Wrong passphrase, or not an identity file.",
    passphrases_differ: "The two passphrases differ.",
    passphrase_too_short: "12 characters or more.",
    mode: "How students take it",
    mode_exam: "Exam: answers hidden, each student hands in a copy",
    mode_practice: "Practice: no code, the correction shows as soon as the student finishes",
    with_opening_code: "Opening code",
    opening_code_help: "The paper stays unreadable until you give this code, on the board when the exam starts. Useful when the file reaches students beforehand, or when another class takes the same paper later.",
    with_end_code: "End code",
    end_code_help: "Once every copy is in, you reveal this code and each student reads their detailed correction. Without it, students only hand in and never see the correction.",
    publish: "Publish",
    working: "Encrypting...",
    published: "Package downloaded:",
    download_again: "Download again",
    opening_code: "Opening code",
    end_code: "End code",
    end_code_hint: "Keep it secret until every copy is in.",
    fingerprint: "Teacher fingerprint",
    print_sheet: "Print the room sheet",
    sheet_questions: "Questions",
    sheet_duration: "Duration",
    no_limit: "no limit",
    sheet_steps: [
        "Open the paper in the player: https://mickaelmancini.com/mcq-advanced/player/ (or the MCQ Advanced app).",
        "Check that the fingerprint shown is the one on this sheet.",
        "Type your name, then the opening code.",
        "When done, hand in, save your copy and send the file to your teacher.",
    ],
    sheet_name_only: "Type your name.",
    sheet_correction_later: "The correction opens with a code given after the exam.",
    sheet_practice: "Practice paper: no code, the correction shows at the end.",
    space_correct: "Correct",
    correction: &CORRECT_EN,
    demo: &DEMO_EN,
    lang: Lang::En,
};

pub static FR: Texts = Texts {
    brand: "MCQ Advanced Studio",
    space_write: "Écrire",
    space_settings: "Réglages",
    space_publish: "Publier",
    new_paper: "Nouveau",
    new_paper_title: "Nouveau sujet",
    confirm_replace: "Remplacer le sujet en cours ? Exportez-le d'abord pour le garder.",
    import: "Importer",
    paste: "Coller",
    tip_paste: "Coller un QCM copié depuis la réponse d'une IA ou un éditeur de texte",
    paste_refused: "Le navigateur n'a pas donné accès au presse-papiers : cliquez sur une zone vide de la page et appuyez sur Ctrl+V.",
    paste_not_paper: "Le presse-papiers ne contient pas de QCM lisible. Collé en texte, le Studio lit : la réponse entière d'une IA à qui l'on a donné son prompt (menu Aide), un sujet .qcm.md, et les banques Moodle XML, GIFT, Aiken ou CSV (le CSV avec sa ligne d'en-tête). Pour un fichier, utilisez Importer.",
    paste_failed: "Le texte collé n'a pas pu être lu",
    pasted_title: "Questions importées",
    help: "Aide",
    tip_help: "Aide : prise en main, guide d'écriture, prompt pour l'IA",
    help_tutorial: "Prise en main (site)",
    help_guide: "Écrire un bon QCM (site)",
    import_choice: "Votre sujet compte {mine} {mine:question|question|questions} ; le fichier en apporte {theirs}. Les ajouter après les vôtres, ou remplacer votre sujet ?",
    import_choice_pasted: "Votre sujet compte {mine} {mine:question|question|questions} ; le texte collé en apporte {theirs}. Les ajouter après les vôtres, ou remplacer votre sujet ?",
    import_add: "Ajouter à mon sujet",
    import_replace: "Remplacer mon sujet",
    cancel: "Annuler",
    imported_added: "{n} {n:question ajoutée|questions ajoutées} à la fin de votre sujet.",
    imported_themes: "{count:Thème|Thèmes} des questions importées : {themes}. Renommez dans Réglages si besoin.",
    why: "Pourquoi ?",
    welcome_title: "Écrire un QCM",
    welcome_text: "Quatre façons de commencer. Quel que soit votre choix, tout reste dans ce navigateur : ni compte, ni serveur.",
    welcome_write: "Écrire une question",
    welcome_write_hint: "Tapez-la ici : le validateur la commente au fil de l'écriture et l'aperçu candidat suit en dessous.",
    welcome_import_hint: "Un sujet (.qcmz, .qcm.md) ou une banque Moodle XML, GIFT, Aiken ou CSV.",
    copy_prompt: "Copier le prompt pour l'IA",
    welcome_ai_hint: "Collez-le dans votre IA avec votre cours à la fin, copiez toute sa réponse, puis appuyez sur Coller.",
    welcome_demo: "Voir une classe d'exemple",
    welcome_demo_hint: "Un sujet fictif et 32 copies déjà corrigées, avec une courte visite.",
    prompt_copied: "Prompt copié : collez-le dans votre IA, ajoutez votre cours à la fin, puis appuyez ici sur Coller avec sa réponse.",
    prompt_saved: "Le navigateur a gardé le presse-papiers pour lui : le prompt est enregistré dans un fichier texte à ouvrir et copier.",
    prompt_file: "mcq-advanced-prompt-ia.txt",
    untouched_hint: "Le validateur commente la question dès que vous l'écrivez.",
    export_qcmz: "Exporter .qcmz",
    export_source: "Exporter .qcm.md",
    export_student: "Sujet sans réponses",
    export_moodle: "Exporter Moodle XML",
    theme_light: "Clair",
    theme_dark: "Sombre",
    restored: if ON_DESKTOP {
        "Votre brouillon a été restauré depuis cet ordinateur."
    } else {
        "Votre brouillon a été restauré depuis ce navigateur."
    },
    imported: "Sujet importé.",
    link_failed: "Le sujet indiqué par le lien n'a pas pu être chargé.",
    not_a_paper: "Ce fichier n'est ni un sujet (.qcmz, .qcm.md) ni une banque (GIFT, Moodle XML, Aiken, CSV)",
    not_a_media: "Seuls les images, sons et vidéos s'ajoutent à un énoncé.",
    no_question_for_media: "Choisissez une question avant d'ajouter un média.",
    mic_unavailable: "Le micro n'est pas disponible (refusé ou absent).",
    saved_locally: if ON_DESKTOP {
        "Enregistré sur cet ordinateur"
    } else {
        "Enregistré dans ce navigateur"
    },
    close: "Fermer",
    tip_new: "Commencer un nouveau QCM vide",
    tip_import: "Importer un QCM (.qcmz, .qcm.md, Moodle XML, GIFT, Aiken, CSV)",
    export: "Exporter",
    tip_export: "Exporter votre QCM dans un fichier",
    tip_theme_light: "Passer en thème clair",
    tip_theme_dark: "Passer en thème sombre",
    tip_space_write: "Écrire et modifier vos questions",
    tip_space_settings: "Réglages du QCM : titre, durée, barème, thèmes",
    tip_space_publish: "Publier votre QCM pour vos candidats",
    tip_space_correct: "Corriger les copies de vos candidats",
    tip_add_question: "Ajouter une question à ce thème",
    tip_add_answer: "Ajouter une réponse",
    tip_add_theme: "Ajouter un thème",
    tip_add_text: "Ajouter un paragraphe de texte",
    tip_add_code: "Ajouter un bloc de code",
    tip_add_formula: "Ajouter une formule",
    tip_add_media: "Ajouter un média : image, son ou vidéo",
    tip_record: "Enregistrer un son au micro",
    tip_question_up: "Monter cette question",
    tip_question_down: "Descendre cette question",
    tip_duplicate: "Dupliquer cette question",
    tip_delete: "Supprimer cette question",
    tip_answer_up: "Monter cette réponse",
    tip_answer_down: "Descendre cette réponse",
    remove: "Retirer",
    stop: "Arrêter",
    tip_theme_up: "Monter ce thème",
    tip_theme_down: "Descendre ce thème",
    tip_preview_computer: "Aperçu sur ordinateur",
    tip_preview_phone: "Aperçu sur téléphone",
    tip_preview_light: "Aperçu en thème clair",
    tip_preview_dark: "Aperçu en thème sombre",
    print: "Imprimer",
    download: "Télécharger",
    tip_download_again: "Télécharger à nouveau le paquet",

    questions: "Questions",
    add_question: "Question",
    drag_question: "Glissez sur un autre thème pour l'y déplacer.",
    no_statement: "(pas encore d'énoncé)",
    state_error: "erreur",
    state_warning: "à revoir",
    state_ok: "prête",
    kind_label: "Type de question",
    empty_paper: "Aucune question : ajoutez-en une depuis la liste.",

    question: "Question",
    id: "Référence de la question",
    id_empty: "Une référence ne peut pas être vide.",
    id_bad: "Ni espace ni | dans une référence.",
    id_taken: "Une autre question porte déjà cette référence.",
    theme: "Thème",
    level: "Niveau",
    level_short: "Ce que la question demande au candidat, de se souvenir d'un fait à juger ; il fixe aussi le temps estimé.",
    level_hint: "Niveaux de Bloom : ce que la question demande au candidat, de 1 (se souvenir d'un fait) à 5 (évaluer, juger) ; ils fixent aussi le temps estimé.",
    levels: [
        "Non précisé",
        "1 Se souvenir",
        "2 Comprendre",
        "3 Appliquer",
        "4 Analyser",
        "5 Évaluer",
    ],
    seconds: "Temps (s)",
    seconds_auto: "selon le niveau",
    kind_single: "Choix unique",
    kind_multiple: "Choix multiples",
    kind_true_false: "Vrai ou faux",
    move_up: "Monter",
    move_down: "Descendre",
    duplicate: "Dupliquer",
    delete: "Supprimer",

    statement: "Énoncé",
    add_text: "Texte",
    add_code: "Code",
    add_formula: "Formule",
    formula_hint: "Formule : $x^2$ dans une phrase, $$x^2$$ sur sa propre ligne, \\$ pour un signe dollar.",
    add_media: "Média",
    record: "Enregistrer",
    stop_recording: "Arrêter l'enregistrement",
    drop_hint: "Glissez ou collez une image, un son ou une vidéo n'importe où.",
    text_block: "Texte",
    code_block: "Code",
    language: "Langage (facultatif, pour colorer du code informatique)",
    highlighted_lines: "Lignes à surligner (facultatif)",
    alt_text: "Texte alternatif",
    alt_hint: "Ce que doit savoir un candidat qui ne peut ni le voir ni l'entendre.",
    block_up: "Monter ce bloc",
    block_down: "Descendre ce bloc",
    block_remove: "Retirer ce bloc",

    answers: "Réponses",
    answers_hint: "Cochez les bonnes réponses.",
    answers_hint_single: "Cochez la bonne réponse.",
    answer_placeholder: "Réponse",
    explanation_placeholder: "Pourquoi cette réponse est juste ou fausse",
    correct: "Bonne réponse",
    add_answer: "Réponse",
    remove_answer: "Retirer cette réponse",
    rule: "Règle à retenir",
    review: "Où réviser",

    validator: "Validateur",
    class_results: "Résultats de la classe",
    class_summary: "Réussie par {percent} % de {asked} copies · {blank} sans réponse · {seconds} s en moyenne",
    class_small: "Petite classe : tendance à confirmer.",
    class_answer: "{percent} % ({picked})",
    class_correct: "juste dans le corrigé",
    class_unmatched: "réponse nouvelle, sans chiffre",
    no_issue: "Rien à corriger.",
    severity_error: "Erreur",
    severity_warning: "Avertissement",
    severity_info: "Info",
    at_paper: "sujet",
    at_line: "ligne",
    at_question: "question",
    at_theme: "thème",
    rules_guide: "Guide des règles",

    preview: "Aperçu candidat",
    preview_computer: "Ordinateur",
    preview_phone: "Téléphone",

    settings_paper: "Sujet",
    title: "Titre du sujet",
    description: "Introduction montrée aux candidats",
    paper_lang: "Langue du sujet",
    paper_languages: "Langues d'interface proposées aux candidats",
    paper_languages_hint: "Aucune cochée : pas de choix de langue, l'interface suit la langue du sujet.",
    notes: "Notes d'auteur (jamais montrées aux candidats)",
    settings_time: "Temps",
    duration: "Durée en minutes (0 : sans limite)",
    estimated: "Temps de réponse estimé : au moins",
    estimated_hint: "D'après le niveau de chaque question, plus un cinquième de marge ; la longueur d'un énoncé, un calcul ou du code ne sont pas comptés. Une question plus longue reçoit son propre temps dans ses réglages avancés.",
    duration_soft: "Le temps ne ferme jamais une copie : un dépassement est seulement noté dessus, la note ne change pas.",
    lang_unset: "Non précisée",
    minutes: "min",
    settings_scoring: "Barème",
    scoring_all_or_nothing: "Tout ou rien",
    scoring_partial: "Crédit partiel",
    penalty: "Points perdus par réponse fausse",
    unanswered_penalty: "Points perdus par question sans réponse",
    settings_exam: "Passation",
    shuffle_questions: "Mélanger les questions",
    shuffle_options: "Mélanger les réponses",
    show_answer_count: "Annoncer le nombre de réponses à cocher",
    allow_extra_time: "Laisser les candidats déclarer un temps majoré (tiers-temps, moitié, double, sans limite) ; leur copie l'indique",
    advanced: "Plus de réglages : référence et temps",
    themes: "Thèmes",
    theme_title: "Nom du thème",
    course_ref: "Où réviser",
    add_theme: "Thème",
    new_theme: "Nouveau thème",
    remove_theme: "Supprimer ce thème vide",
    paper_findings: "Sur l'ensemble du sujet",

    publish_blocked: "Corrigez les erreurs signalées dans Écrire avant de publier.",
    identity: "Identité du professeur",
    identity_hint: "Elle signe vos sujets et elle seule ouvre les copies. Gardez le fichier et sa phrase de passe : sans eux, les copies sont illisibles.",
    load_identity: "Charger mon identité (.mcqid)",
    passphrase: "Phrase de passe",
    unlock: "Déverrouiller",
    create_identity: "Créer une identité",
    new_passphrase: "Nouvelle phrase de passe (12 caractères au moins)",
    confirm_passphrase: "La même phrase, encore",
    create_and_download: "Créer et télécharger",
    identity_ready: "Identité prête. Empreinte :",
    fingerprint_hint: "L'empreinte est un code court qui désigne votre identité : les candidats la voient en ouvrant votre sujet et la comparent à la fiche de salle, ce qui trahit un sujet signé par quelqu'un d'autre.",
    identity_saved: if ON_DESKTOP {
        "Votre identité est enregistrée sous teacher.mcqid, dans le dossier choisi. Gardez-la en lieu sûr (clé USB, stockage en ligne) : avec sa phrase de passe, elle seule ouvre les copies de vos candidats."
    } else {
        "Votre identité est enregistrée sous teacher.mcqid, en général dans vos Téléchargements. Rangez-la en lieu sûr (clé USB, stockage en ligne) : avec sa phrase de passe, elle seule ouvre les copies de vos candidats."
    },
    identity_download_again: "Télécharger à nouveau mon identité",
    identity_kept: "Votre identité est gardée dans ce navigateur : tapez sa phrase de passe pour la déverrouiller.",
    identity_forget: "Oublier l'identité gardée dans ce navigateur",
    create_other_identity: "Créer une autre identité",
    create_other_warning: "Attention : les copies des sujets publiés avec votre identité actuelle ne s'ouvriront qu'avec elle. N'en créez une autre que si vous l'avez perdue pour de bon.",
    identity_keep: "Garder mon identité dans ce navigateur (toujours verrouillée par sa phrase de passe)",
    publish_warnings: "Le validateur signale encore {n} {n:avertissement|avertissements} : vous pouvez publier, mais relisez-les d'abord dans Écrire.",
    publish_advice: "{n} {n:question gagnerait|questions gagneraient} à une correction plus complète (explications, règle à retenir, où réviser) : facultatif, cela aide les candidats à apprendre de leurs erreurs.",
    severity_advice: "Conseil",
    advice_summaries: [
        "{n} {n:question n'explique|questions n'expliquent} aucune réponse.",
        "{n} {n:question a|questions ont} des réponses sans explication.",
        "{n} {n:question n'a|questions n'ont} pas de règle à retenir.",
        "{n} {n:question n'indique|questions n'indiquent} pas où réviser.",
    ],
    distribute_title: "Comment les candidats le passent",
    distribute_steps: [
        "Envoyez {file} à vos candidats (ENT, messagerie, clé USB) ou mettez-le en ligne.",
        "Ils l'ouvrent dans le lecteur, dans n'importe quel navigateur, sans rien installer : https://mickaelmancini.com/mcq-advanced/player/ (ou dans l'application MCQ Advanced).",
        "Au début de l'épreuve, donnez le code d'ouverture : il est sur la fiche de salle ci-dessous.",
        "À la fin, chaque candidat enregistre sa copie (un fichier .mcqres) et vous l'envoie : ouvrez-les dans Corriger.",
    ],
    wrong_passphrase: "Phrase de passe incorrecte, ou ce n'est pas un fichier d'identité.",
    passphrases_differ: "Les deux phrases diffèrent.",
    passphrase_too_short: "12 caractères au moins.",
    mode: "Passation",
    mode_exam: "Examen : corrigé caché, chaque candidat rend une copie",
    mode_practice: "Entraînement : sans code, la correction s'affiche dès que le candidat a fini",
    with_opening_code: "Code d'ouverture",
    opening_code_help: "Le sujet reste illisible tant que vous n'avez pas donné ce code, au tableau au début de l'épreuve. Utile si le fichier arrive chez les candidats avant, ou si une autre classe passe le même sujet plus tard.",
    with_end_code: "Code de fin",
    end_code_help: "Quand toutes les copies sont rendues, vous révélez ce code et chaque candidat lit sa correction détaillée. Sans lui, les candidats rendent leur copie et ne voient jamais la correction.",
    publish: "Publier",
    working: "Chiffrement en cours...",
    published: "Paquet téléchargé :",
    download_again: "Télécharger à nouveau",
    opening_code: "Code d'ouverture",
    end_code: "Code de fin",
    end_code_hint: "Gardez-le secret jusqu'à la remise de toutes les copies.",
    fingerprint: "Empreinte du professeur",
    print_sheet: "Imprimer la fiche de salle",
    sheet_questions: "Questions",
    sheet_duration: "Durée",
    no_limit: "sans limite",
    sheet_steps: [
        "Ouvrez le sujet dans le lecteur : https://mickaelmancini.com/mcq-advanced/player/ (ou l'application MCQ Advanced).",
        "Vérifiez que l'empreinte affichée est celle de cette fiche.",
        "Saisissez votre nom, puis le code d'ouverture.",
        "À la fin, rendez votre copie, enregistrez-la et envoyez le fichier à votre enseignant.",
    ],
    sheet_name_only: "Saisissez votre nom.",
    sheet_correction_later: "La correction s'ouvrira avec un code donné après l'épreuve.",
    sheet_practice: "Sujet d'entraînement : aucun code, la correction s'affiche à la fin.",
    space_correct: "Corriger",
    correction: &CORRECT_FR,
    demo: &DEMO_FR,
    lang: Lang::Fr,
};

/// Puts values in a template: `{name}` becomes the value given for `name`.
/// `{name:one|many}` becomes `one` when `name` is 1, `many` otherwise.
/// `{name:zero|one|many}` also names the form for 0, which French puts in the singular.
pub fn fill(template: &str, values: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    for (name, value) in values {
        out = out.replace(&format!("{{{name}}}"), value);
        let open = format!("{{{name}:");
        while let Some(start) = out.find(&open) {
            let Some(len) = out[start..].find('}') else {
                break;
            };
            let forms = out[start + open.len()..start + len].to_string();
            let parts: Vec<&str> = forms.split('|').collect();
            let count = value.trim().parse::<u64>().ok();
            let chosen = match (parts.as_slice(), count) {
                ([zero, _, _], Some(0)) => zero,
                ([_, one, _] | [one, _], Some(1)) => one,
                ([.., many], _) => many,
                ([], _) => "",
            }
            .to_string();
            out.replace_range(start..=start + len, &chosen);
        }
    }
    out
}

/// The Correct space: sentences with `{placeholders}` filled by `fill`.
#[derive(PartialEq)]
pub struct CorrectTexts {
    pub intro: &'static str,
    pub load_package: &'static str,
    pub use_published: &'static str,
    pub add_copies: &'static str,
    pub drop_hint: &'static str,
    pub waiting: &'static str,
    pub package_ready: &'static str,
    pub copies_in: &'static str,
    pub refused: &'static str,
    pub refused_unknown: &'static str,
    pub refused_not_yours: &'static str,
    pub refused_already_in: &'static str,
    pub refused_damaged: &'static str,
    pub clear: &'static str,
    pub summary: &'static str,
    pub reliability: &'static str,
    pub reliability_levels: [&'static str; 4],
    /// Under the class figures: the words of the analysis, in plain language.
    pub glossary_title: &'static str,
    pub glossary: [&'static str; 3],
    pub small_cohort: &'static str,
    pub items_not_computed: &'static str,
    pub reteach_title: &'static str,
    pub reteach: &'static str,
    pub reteach_doubtful: &'static str,
    pub reteach_none: &'static str,
    pub thin_theme: &'static str,
    pub traps_title: &'static str,
    pub trap: &'static str,
    pub traps_none: &'static str,
    pub review_title: &'static str,
    pub key_doubt_rival: &'static str,
    pub key_doubt: &'static str,
    /// Students ticked "this question seems unclear or wrong".
    pub reported: &'static str,
    pub too_hard: &'static str,
    /// Some answered and missed it, others left it empty.
    pub too_hard_blank: &'static str,
    /// Nobody who answered missed it: every miss is an empty answer.
    pub too_hard_all_blank: &'static str,
    pub dead: &'static str,
    pub too_easy: &'static str,
    pub weak: &'static str,
    pub review_none: &'static str,
    pub neutralize: &'static str,
    pub accept: &'static str,
    pub decide_hint: &'static str,
    pub decisions_title: &'static str,
    pub decided_neutral: &'static str,
    pub decided_rekey: &'static str,
    pub undo: &'static str,
    pub anomalies_title: &'static str,
    pub anomalies_hint: &'static str,
    pub other_paper: &'static str,
    pub same_name: &'static str,
    pub unknown_questions: &'static str,
    pub identical: &'static str,
    pub left_window: &'static str,
    pub overtime: &'static str,
    pub fast: &'static str,
    pub hurried: &'static str,
    pub extra_time: &'static str,
    pub untimed: &'static str,
    pub marks_title: &'static str,
    pub columns: [&'static str; 7],
    pub open_sheet: &'static str,
    pub open_timeline: &'static str,
    pub timeline_title: &'static str,
    pub timeline_hint: &'static str,
    pub timeline_summary: &'static str,
    pub timeline_empty: &'static str,
    pub tl_answer: &'static str,
    pub tl_right: &'static str,
    /// Some credit, not all, as the marks count it.
    pub tl_partly: &'static str,
    pub tl_wrong: &'static str,
    pub tl_changes: [&'static str; 3],
    pub tl_cleared: &'static str,
    pub tl_away: &'static str,
    pub tl_away_open: &'static str,
    pub tl_submitted: &'static str,
    pub export_notes: &'static str,
    pub export_themes: &'static str,
    pub export_questions: &'static str,
    pub sheet_title: &'static str,
    pub sheet_where: &'static str,
    pub sheet_mistakes: &'static str,
    pub sheet_none: &'static str,
    pub sheet_yours: &'static str,
    pub sheet_blank: &'static str,
    pub sheet_right: &'static str,
    pub sheet_key_in_doubt: &'static str,
    pub sheet_remember: &'static str,
    pub sheet_revise: &'static str,
    pub sheet_start: &'static str,
    pub sheet_too_few: &'static str,
    pub mastery: [&'static str; 3],
    pub print_student_sheet: &'static str,
    pub export: &'static str,
    pub tip_export: &'static str,
    pub tip_open_sheet: &'static str,
    pub tip_open_timeline: &'static str,
    pub tip_undo: &'static str,
    pub print: &'static str,
    pub close_sheet: &'static str,
}

pub static CORRECT_EN: CorrectTexts = CorrectTexts {
    intro: "The paper is the last one you published from this browser, or the package you load. Then add the candidates' copies: they open with your identity only. Everything stays on this device.",
    load_package: "Load a package (.mcqpack)",
    use_published: "Use the last paper published",
    add_copies: "Add copies (.mcqres)",
    drop_hint: "All the copies at once: select them together in their folder (Ctrl+A), or drop the whole lot anywhere on the page, the package with them.",
    waiting: "{n} {n:file|files} waiting for your identity.",
    package_ready: "Paper: {title}",
    copies_in: "{n} {n:copy|copies} opened.",
    refused: "Not taken",
    refused_unknown: "not a package or a copy",
    refused_not_yours: "made for another teacher's identity",
    refused_already_in: "this copy is already in",
    refused_damaged: "damaged or altered file",
    clear: "Start again",
    summary: "{n} {n:copy|copies} · mean {mean}/20 · median {median}/20 · mean time {minutes}",
    reliability: "Reliability of the paper (KR-20): {value}, {level}.",
    reliability_levels: [
        "very low: marks say little",
        "low: read the marks with care",
        "fair",
        "good",
    ],
    glossary_title: "What do these figures mean?",
    glossary: [
        "D, from -1 to 1: how much better the strongest copies do on a question than the weakest. Near 0, the question hardly separates them; below 0, the strongest miss it more, often the sign of a doubtful key.",
        "Reliability (KR-20), from 0 to 1: whether the questions measure the same thing together. From 0.7, the marks can support a decision.",
        "A wrong option is one of the answers offered that are not right (also called a distractor).",
    ],
    small_cohort: "Fewer than 30 copies: the figures below are trends to confirm, not verdicts.",
    items_not_computed: "Fewer than 8 copies: how well each question separates stronger from weaker students (D) and the reliability of the marks are not computed.",
    reteach_title: "To teach again",
    reteach: "{theme}: {rate} % success, {share} % of the class not there yet",
    reteach_doubtful: "Check first: {n} {n:question of this theme has|questions of this theme have} a doubtful key or were reported by students ({ids}); the notion may be fine.",
    reteach_none: "No theme is under the threshold for this class.",
    thin_theme: "{theme} has only {n} {n:question|questions}: its rate is not usable.",
    traps_title: "Shared confusions",
    trap: "{rate} % chose «{option}» ({question})",
    traps_none: "No wrong option drew a quarter of the answers.",
    review_title: "Questions to review",
    key_doubt_rival: "{question} · key or wording to check: the strongest students prefer «{rival}»",
    key_doubt: "{question} · the strongest students miss it more than the others (D = {d}): probably ambiguous",
    reported: "{question} · reported as unclear or wrong by {n} {n:student|students}: reread the statement and the key",
    too_hard: "{question} · missed by {pct} % of the class",
    too_hard_blank: "{question} · missed by {pct} %, left blank by {blank} % of the class",
    too_hard_all_blank: "{question} · left blank by {blank} % of the class",
    dead: "{question} · {n} {n:wrong option|wrong options} never chosen",
    too_easy: "{question} · succeeded by everyone",
    weak: "{question} · separates strong and weak students poorly (D = {d})",
    review_none: "No question needs rewriting.",
    neutralize: "Neutralize",
    accept: "Accept «{option}»",
    decide_hint: "Neutralize takes the question out of the marks; Accept makes that option the right answer. Every copy is marked again at once, and you can undo.",
    decisions_title: "Your decisions on the key",
    decided_neutral: "{question} neutralized: out of the marks, every copy is marked on {n} questions",
    decided_rekey: "{question}: key corrected, right answer «{options}»",
    undo: "Undo",
    anomalies_title: "Copies to look at closely",
    anomalies_hint: "An anomaly is a reason to ask a question, never a verdict.",
    other_paper: "{name}: a copy of another paper, left out",
    same_name: "{name}: {n} copies under this name",
    unknown_questions: "{name}: {n} {n:answer|answers} to questions missing from the key",
    identical: "{first} and {second}: the same answers, {n} identical mistakes among them",
    left_window: "{name}: left the exam window {n} {n:time|times} ({minutes} away)",
    overtime: "{name}: {minutes} over the time limit (the mark ignores it)",
    fast: "{name}: a very good copy handed in after {minutes}",
    hurried: "{name}: copy handed in after {minutes}, for about {planned} planned",
    extra_time: "{name}: extra time declared ({extension}), to check against your list of arrangements",
    untimed: "{name}: extra time without limit declared, to check against your list of arrangements",
    marks_title: "Marks",
    columns: [
        "Name",
        "Mark /20",
        "Correct",
        "Partly correct",
        "Wrong",
        "Blank",
        "Time",
    ],
    open_sheet: "Sheet",
    open_timeline: "Timeline",
    timeline_title: "Timeline of the copy",
    timeline_hint: "Facts to talk about with the student, never a verdict.",
    timeline_summary: "{changes} answer {changes:change|changes}: {rw} from right to wrong, {wr} from wrong to right, {ww} from wrong to wrong. {away} {away:exit|exits} from the exam window, {minutes} away.",
    timeline_empty: "Nothing was recorded for this copy.",
    tl_answer: "{question}: «{options}»",
    tl_right: "right",
    tl_partly: "partly right",
    tl_wrong: "wrong",
    tl_changes: [
        "replaces a right answer",
        "corrects a wrong answer",
        "changes a wrong answer",
    ],
    tl_cleared: "{question}: answer cleared",
    tl_away: "leaves the exam window ({duration})",
    tl_away_open: "leaves the exam window (not back before hand-in)",
    tl_submitted: "hands in the copy",
    export_notes: "Export notes.csv",
    export_themes: "Export themes.csv",
    export_questions: "Export questions.csv",
    sheet_title: "What to review",
    sheet_where: "Where I stand",
    sheet_mistakes: "My mistakes ({n})",
    sheet_none: "None. Nothing to review on this paper.",
    sheet_yours: "My answer:",
    sheet_blank: "I did not answer",
    sheet_right: "The right answer:",
    sheet_key_in_doubt: "Your teacher is checking the key of this question: ask in class.",
    sheet_remember: "To remember:",
    sheet_revise: "Revise:",
    sheet_start: "Where to start",
    sheet_too_few: "(too few questions to conclude)",
    mastery: ["acquired", "fragile", "to relearn"],
    print_student_sheet: "Print this sheet",
    export: "Export",
    tip_export: "Export the class results",
    tip_open_sheet: "Revision sheet of this student",
    tip_open_timeline: "Timeline of this copy",
    tip_undo: "Undo this decision",
    print: "Print",
    close_sheet: "Close",
};

pub static CORRECT_FR: CorrectTexts = CorrectTexts {
    intro: "Le sujet est le dernier que vous avez publié depuis ce navigateur, ou le paquet que vous chargez. Ajoutez ensuite les copies des candidats : elles ne s'ouvrent qu'avec votre identité. Tout reste sur cet appareil.",
    load_package: "Charger un paquet (.mcqpack)",
    use_published: "Reprendre le dernier sujet publié",
    add_copies: "Ajouter des copies (.mcqres)",
    drop_hint: "Toutes les copies d'un coup : sélectionnez-les ensemble dans leur dossier (Ctrl+A), ou glissez le lot entier n'importe où sur la page, le paquet avec.",
    waiting: "{n} {n:fichier|fichiers} en attente de votre identité.",
    package_ready: "Sujet : {title}",
    copies_in: "{n} {n:copie ouverte|copie ouverte|copies ouvertes}.",
    refused: "Non pris",
    refused_unknown: "ni un paquet ni une copie",
    refused_not_yours: "fait pour l'identité d'un autre professeur",
    refused_already_in: "cette copie est déjà là",
    refused_damaged: "fichier abîmé ou modifié",
    clear: "Recommencer",
    summary: "{n} {n:copie|copies} · moyenne {mean}/20 · médiane {median}/20 · durée moyenne {minutes}",
    reliability: "Fiabilité du sujet (KR-20) : {value}, {level}.",
    reliability_levels: [
        "très faible : les notes disent peu",
        "faible : notes à lire avec prudence",
        "correcte",
        "bonne",
    ],
    glossary_title: "Que veulent dire ces chiffres ?",
    glossary: [
        "D, de -1 à 1 : à quel point les meilleures copies réussissent mieux la question que les plus faibles. Proche de 0, la question les sépare mal ; sous 0, les meilleurs la ratent davantage, souvent le signe d'un corrigé douteux.",
        "Fiabilité (KR-20), de 0 à 1 : les questions mesurent-elles ensemble la même chose ? À partir de 0,7, les notes peuvent appuyer une décision.",
        "Une mauvaise réponse est une des options proposées qui ne sont pas justes (on dit aussi « distracteur »).",
    ],
    small_cohort: "Moins de 30 copies : les chiffres ci-dessous sont des tendances à confirmer, pas des verdicts.",
    items_not_computed: "Moins de 8 copies : la capacité de chaque question à distinguer les candidats forts des plus faibles (D) et la fiabilité de la note ne sont pas calculées.",
    reteach_title: "À réenseigner",
    reteach: "{theme} : {rate} % de réussite, {share} % de la classe non acquis",
    reteach_doubtful: "À vérifier d'abord : {n} {n:question de ce thème a un corrigé douteux ou a été signalée|questions de ce thème ont un corrigé douteux ou ont été signalées} par des candidats ({ids}) ; la notion est peut-être acquise.",
    reteach_none: "Aucun thème n'est sous le seuil pour cette classe.",
    thin_theme: "{theme} n'a que {n} {n:question|questions} : son taux n'est pas exploitable.",
    traps_title: "Idées fausses partagées",
    trap: "{rate} % ont choisi «{option}» ({question})",
    traps_none: "Aucune mauvaise réponse n'a attiré le quart des réponses.",
    review_title: "Questions à revoir",
    key_doubt_rival: "{question} · corrigé ou énoncé à vérifier : les meilleurs préfèrent «{rival}»",
    key_doubt: "{question} · les meilleurs la ratent plus que les autres (D = {d}) : probablement ambiguë",
    reported: "{question} · signalée floue ou erronée par {n} {n:candidat|candidats} : relire l'énoncé et le corrigé",
    too_hard: "{question} · ratée par {pct} % de la classe",
    too_hard_blank: "{question} · ratée par {pct} %, laissée vide par {blank} % de la classe",
    too_hard_all_blank: "{question} · laissée vide par {blank} % de la classe",
    dead: "{question} · {n} {n:mauvaise réponse jamais choisie|mauvaises réponses jamais choisies}",
    too_easy: "{question} · réussie par tout le monde",
    weak: "{question} · sépare mal les forts et les faibles (D = {d})",
    review_none: "Aucune question ne demande de réécriture.",
    neutralize: "Neutraliser",
    accept: "Accepter «{option}»",
    decide_hint: "Neutraliser retire la question du barème ; Accepter fait de cette option la bonne réponse. Toutes les copies sont re-notées aussitôt, et c'est annulable.",
    decisions_title: "Vos décisions sur le corrigé",
    decided_neutral: "{question} neutralisée : hors barème, chaque copie est notée sur {n} questions",
    decided_rekey: "{question} : corrigé rectifié, bonne réponse «{options}»",
    undo: "Annuler",
    anomalies_title: "Copies à regarder de près",
    anomalies_hint: "Une anomalie est une raison de poser une question, jamais un verdict.",
    other_paper: "{name} : copie d'un autre sujet, écartée",
    same_name: "{name} : {n} copies sous ce nom",
    unknown_questions: "{name} : {n} {n:réponse|réponses} à des questions absentes du corrigé",
    identical: "{first} et {second} : mêmes réponses, dont {n} erreurs identiques",
    left_window: "{name} : {n} {n:sortie|sorties} de la fenêtre d'épreuve ({minutes} dehors)",
    overtime: "{name} : temps dépassé de {minutes} (la note n'en tient pas compte)",
    fast: "{name} : très bonne copie rendue en {minutes}",
    hurried: "{name} : copie rendue en {minutes}, pour environ {planned} prévues",
    extra_time: "{name} : temps majoré déclaré ({extension}), à vérifier avec votre liste d'aménagements",
    untimed: "{name} : temps majoré sans limite déclaré, à vérifier avec votre liste d'aménagements",
    marks_title: "Notes",
    columns: [
        "Nom",
        "Note /20",
        "Justes",
        "En partie justes",
        "Fausses",
        "Sans réponse",
        "Durée",
    ],
    open_sheet: "Fiche",
    open_timeline: "Chronologie",
    timeline_title: "Chronologie de la copie",
    timeline_hint: "Des faits pour en parler avec le candidat, jamais un verdict.",
    timeline_summary: "{changes} {changes:changement|changement|changements} de réponse : {rw} de juste à faux, {wr} de faux à juste, {ww} de faux à faux. {away} {away:sortie|sortie|sorties} de la fenêtre d'épreuve, {minutes} dehors.",
    timeline_empty: "Rien n'a été enregistré pour cette copie.",
    tl_answer: "{question} : «{options}»",
    tl_right: "juste",
    tl_partly: "en partie juste",
    tl_wrong: "faux",
    tl_changes: [
        "remplace une réponse juste",
        "corrige une réponse fausse",
        "change une réponse fausse",
    ],
    tl_cleared: "{question} : réponse effacée",
    tl_away: "quitte la fenêtre d'épreuve ({duration})",
    tl_away_open: "quitte la fenêtre d'épreuve (pas revenu avant la remise)",
    tl_submitted: "rend sa copie",
    export_notes: "Exporter notes.csv",
    export_themes: "Exporter themes.csv",
    export_questions: "Exporter questions.csv",
    sheet_title: "Ce que j'ai à revoir",
    sheet_where: "Où j'en suis",
    sheet_mistakes: "Mes erreurs ({n})",
    sheet_none: "Aucune. Rien à revoir sur ce sujet.",
    sheet_yours: "Ma réponse :",
    sheet_blank: "Je n'ai pas répondu",
    sheet_right: "La bonne réponse :",
    sheet_key_in_doubt: "Le corrigé de cette question est en cours de vérification : posez la question en classe.",
    sheet_remember: "À retenir :",
    sheet_revise: "Réviser :",
    sheet_start: "Par quoi commencer",
    sheet_too_few: "(trop peu de questions pour conclure)",
    mastery: ["acquis", "fragile", "à reprendre"],
    print_student_sheet: "Imprimer cette fiche",
    export: "Exporter",
    tip_export: "Exporter les résultats de la classe",
    tip_open_sheet: "Fiche de révision de ce candidat",
    tip_open_timeline: "Chronologie de cette copie",
    tip_undo: "Annuler cette décision",
    print: "Imprimer",
    close_sheet: "Fermer",
};

/// The teacher demo: what the status line says and the tour of what to try.
#[derive(PartialEq)]
pub struct DemoTexts {
    pub preparing: &'static str,
    pub ready: &'static str,
    pub failed: &'static str,
    pub title: &'static str,
    pub intro: &'static str,
    /// Title and text of each step, in the order of `demo::STEP_SPACES`.
    pub steps: [(&'static str, &'static str); 8],
    pub identity_note: &'static str,
    /// Asked before the demo paper takes the place of a draft that holds work.
    pub confirm: &'static str,
    /// The answer of `confirm` that opens the demo.
    pub open: &'static str,
    /// The tour's button that ends the visit and brings the draft back.
    pub leave: &'static str,
    /// Asked on leaving when the demo paper was changed during the visit.
    pub leave_confirm: &'static str,
    /// Said once the visit is over.
    pub left: &'static str,
}

pub static DEMO_EN: DemoTexts = DemoTexts {
    preparing: "Preparing the sample class: paper, package and 32 fictional copies...",
    ready: "Sample class ready: the paper is in the editor, 32 fictional copies wait in Correct.",
    failed: "The sample class could not be prepared.",
    title: "Sample class tour",
    intro: "A fictional paper and class. Click a step to go there.",
    steps: [
        (
            "Write.",
            "Edit a question; the preview shows it as a student will, on a phone or in the dark theme, with its formulas, picture and coloured code.",
        ),
        (
            "Check.",
            "The live check flags every problem, question by question, before anything is published.",
        ),
        (
            "Set up.",
            "Duration, scoring, penalties, shuffling, and the themes with their course references.",
        ),
        (
            "Publish.",
            "Create your identity, then publish: a signed package, its opening code and a room sheet to print.",
        ),
        (
            "Correct.",
            "32 fictional copies are already in: what to reteach, shared confusions, questions to read again, then the marks.",
        ),
        (
            "Settle a doubtful key.",
            "A box ticked by mistake puts the key in doubt: accept the answer the strongest students chose, and every mark is computed again.",
        ),
        (
            "Look at a copy.",
            "Identical copies, exits from the window, overtime: open a student's timeline or revision sheet, export the marks as CSV.",
        ),
        (
            "Import and export.",
            "In the top bar: import a GIFT, Moodle XML, Aiken or CSV bank, export to Moodle or as a paper without answers.",
        ),
    ],
    identity_note: "The copies are opened with a public sample identity that serves them only. To publish a real paper, create your own identity.",
    confirm: "Open the sample class? Your draft is set aside and comes back when you leave the tour.",
    open: "Open the sample class",
    leave: "Leave the sample class",
    leave_confirm: "Leave the tour? Your changes to the sample paper are dropped and your draft comes back.",
    left: "Tour over: your draft is back in the editor.",
};

pub static DEMO_FR: DemoTexts = DemoTexts {
    preparing: "Préparation de la classe d'exemple : sujet, paquet et 32 copies fictives...",
    ready: "Classe d'exemple prête : le sujet est dans l'éditeur, 32 copies fictives attendent dans Corriger.",
    failed: "La classe d'exemple n'a pas pu être préparée.",
    title: "Visite de la classe d'exemple",
    intro: "Un sujet et une classe fictifs. Cliquez une étape pour y aller.",
    steps: [
        (
            "Rédiger.",
            "Modifiez une question ; l'aperçu la montre comme le candidat la verra, sur téléphone ou en thème sombre, avec ses formules, son image et son code coloré.",
        ),
        (
            "Vérifier.",
            "Le contrôle en direct signale chaque problème, question par question, avant toute publication.",
        ),
        (
            "Régler.",
            "Durée, barème, pénalités, mélange, et les thèmes avec leur renvoi de cours.",
        ),
        (
            "Publier.",
            "Créez votre identité, puis publiez : un paquet signé, son code d'ouverture et une fiche de salle à imprimer.",
        ),
        (
            "Corriger.",
            "32 copies fictives sont déjà là : ce qu'il faut retravailler, les confusions partagées, les questions à relire, puis les notes.",
        ),
        (
            "Trancher une clé douteuse.",
            "Une case cochée par erreur fait douter de la clé : acceptez la réponse choisie par les meilleurs candidats, et toutes les notes sont recalculées.",
        ),
        (
            "Examiner une copie.",
            "Copies identiques, sorties de fenêtre, dépassement de temps : ouvrez la chronologie d'un candidat ou sa fiche de révision, exportez les notes en CSV.",
        ),
        (
            "Importer et exporter.",
            "Dans la barre du haut : importez une banque GIFT, Moodle XML, Aiken ou CSV, exportez vers Moodle ou en sujet sans réponses.",
        ),
    ],
    identity_note: "Les copies sont ouvertes avec une identité d'exemple publique, qui ne sert qu'à elles. Pour publier un vrai sujet, créez votre propre identité.",
    confirm: "Ouvrir la classe d'exemple ? Votre brouillon est mis de côté et revient quand vous quittez la visite.",
    open: "Ouvrir la classe d'exemple",
    leave: "Quitter la classe d'exemple",
    leave_confirm: "Quitter la visite ? Vos modifications du sujet d'exemple sont abandonnées et votre brouillon revient.",
    left: "Visite terminée : votre brouillon est revenu dans l'éditeur.",
};

#[cfg(test)]
mod tests {
    use mcq_model::{Issue, Location, Severity};

    use super::{EN, FR, fill};

    #[test]
    fn counts_pick_their_singular_or_plural() {
        let one = fill("{n} {n:copie|copies}", &[("n", "1".to_string())]);
        let many = fill("{n} {n:copie|copies}", &[("n", "3".to_string())]);
        assert_eq!((one.as_str(), many.as_str()), ("1 copie", "3 copies"));
        assert_eq!(
            fill(
                "{a} {a:x|y}, {b} {b:x|y}",
                &[("a", "1".into()), ("b", "0".into())]
            ),
            "1 x, 0 y"
        );
        let zero = "{n} {n:copie|copie|copies}";
        let say = |n: &str| fill(zero, &[("n", n.to_string())]);
        assert_eq!(
            [say("0"), say("1"), say("2")],
            ["0 copie", "1 copie", "2 copies"]
        );
    }

    #[test]
    fn the_editor_names_fields_where_the_core_names_syntax() {
        let issue = Issue::new(
            Severity::Warning,
            "NO_RULE",
            Location::Paper,
            "No rule to remember: add `RULE:`.",
            "Aucune règle à retenir : ajoutez `RULE:`.",
        );
        assert_eq!(
            FR.form_issue(&issue),
            "Aucune règle à retenir : remplissez « Règle à retenir »."
        );
        assert_eq!(
            EN.form_issue(&issue),
            "No rule to remember: fill in “Rule to remember”."
        );
    }

    #[test]
    fn no_core_message_reaches_the_editor_with_its_syntax() {
        let mut project = mcq_edit::Project::default();
        project.paper.questions.push(mcq_model::Question {
            id: "q1".into(),
            options: vec![mcq_model::AnswerOption::default(); 2],
            ..mcq_model::Question::default()
        });
        for issue in project.issues() {
            for text in [FR.form_issue(&issue), EN.form_issue(&issue)] {
                for syntax in ["`RULE:`", "`[x]`", "`COURSE:`", "`level 1`", "`title:`"] {
                    assert!(!text.contains(syntax), "{text}");
                }
            }
        }
    }
}
