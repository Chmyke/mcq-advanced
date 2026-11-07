//! What buttons and page events start: adding media, recording, opening, importing and exporting
//! papers.

use dioxus::prelude::*;
use mcq_convert::{ImportOptions, Source, handout, moodle};
use mcq_edit::{Project, qcmz};
use mcq_model::{Location, Severity};
use mcq_player::b64;

use crate::platform::{self, RECORD_SCRIPT, STOP_RECORDING_SCRIPT};
use crate::state::Studio;
use crate::texts::{Lang, Texts};

pub fn add_media(
    mut studio: Signal<Studio>,
    mut notice: Signal<Option<String>>,
    t: &'static Texts,
    question_id: &str,
    file_name: &str,
    bytes: Vec<u8>,
) {
    if studio
        .write()
        .add_media(question_id, file_name, bytes)
        .is_none()
    {
        notice.set(Some(t.not_a_media.to_string()));
    }
}

pub fn start_recording(
    studio: Signal<Studio>,
    mut recording: Signal<bool>,
    mut notice: Signal<Option<String>>,
    t: &'static Texts,
    question_id: String,
) {
    recording.set(true);
    spawn(async move {
        let result = document::eval(RECORD_SCRIPT)
            .join::<(String, String)>()
            .await;
        recording.set(false);
        match result {
            Ok((ext, data)) if !ext.is_empty() => match b64::decode(&data) {
                Some(bytes) => add_media(
                    studio,
                    notice,
                    t,
                    &question_id,
                    &format!("recording.{ext}"),
                    bytes,
                ),
                None => notice.set(Some(t.mic_unavailable.to_string())),
            },
            _ => notice.set(Some(t.mic_unavailable.to_string())),
        }
    });
}

pub fn stop_recording() {
    let _ = document::eval(STOP_RECORDING_SCRIPT);
}

/// What the import button accepts: papers, and the banks of other tools.
pub const PAPER_FILES: &str = ".qcmz,.zip,.md,.gift,.xml,.txt,.csv,.tsv";

/// True for the files the Studio opens as a paper rather than adds as media.
pub fn is_paper_file(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    PAPER_FILES.split(',').any(|ext| name.ends_with(ext))
}

/// Reads a `.qcmz` (or any zip of a paper folder), a `.qcm.md`, or a GIFT, Moodle XML, Aiken or
/// CSV bank, whose import report comes back as the issues. `lang` is the paper's language;
/// `french` picks the language of the reason given when a file cannot be read.
/// WHY: the Studio only edits visually, so a paper the reader cannot fully understand is refused
/// with its first error and line, to fix in the file, instead of being opened in part.
pub fn read_paper(
    name: &str,
    bytes: Vec<u8>,
    lang: &str,
    french: bool,
) -> Result<(Project, Vec<mcq_model::Issue>), String> {
    let read = if bytes.starts_with(b"PK") {
        qcmz::import(&bytes).map_err(|e| e.to_string())?
    } else {
        read_text(name, bytes, lang)?
    };
    refuse_errors(read, french)
}

/// The paper read, or its first error with its line, in the interface language.
fn refuse_errors(
    (project, issues): (Project, Vec<mcq_model::Issue>),
    french: bool,
) -> Result<(Project, Vec<mcq_model::Issue>), String> {
    match issues.iter().find(|i| i.severity == Severity::Error) {
        Some(error) => {
            let message = error.message_in(french).trim_end_matches('.');
            Err(match &error.location {
                Location::Line(n) if french => format!("ligne {n} : {message}"),
                Location::Line(n) => format!("line {n}: {message}"),
                _ => message.to_string(),
            })
        }
        None => Ok((project, issues)),
    }
}

fn read_text(
    name: &str,
    bytes: Vec<u8>,
    lang: &str,
) -> Result<(Project, Vec<mcq_model::Issue>), String> {
    let text = String::from_utf8(bytes).map_err(|_| "not UTF-8 text".to_string())?;
    if name.to_ascii_lowercase().ends_with(".md") {
        return Ok(read_source(&text));
    }
    let source = Source::detect(name, &text).ok_or_else(|| "unknown format".to_string())?;
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    Ok(read_bank(source, &text, &readable_title(stem), lang))
}

fn read_source(text: &str) -> (Project, Vec<mcq_model::Issue>) {
    let mut project = Project::default();
    let issues = project.set_source(text);
    (project, issues)
}

/// A bank in `source`'s format; `title` names the paper, and the theme of the questions that
/// come without a category.
fn read_bank(
    source: Source,
    text: &str,
    title: &str,
    lang: &str,
) -> (Project, Vec<mcq_model::Issue>) {
    let options = ImportOptions {
        title: title.to_string(),
        lang: lang.to_string(),
    };
    let imported = mcq_convert::import(source, text, &options);
    (imported.project, imported.issues)
}

/// Reads pasted text as the import reads a file, its format told by its content alone: the
/// paper inside an AI's answer, or a Moodle XML, CSV, Aiken or GIFT bank. `None` when the text
/// holds none of them; otherwise as `read_paper`. `title` names what came without a title or
/// a category.
pub fn read_pasted(
    text: &str,
    title: &str,
    lang: &str,
    french: bool,
) -> Option<Result<(Project, Vec<mcq_model::Issue>), String>> {
    let read = if let Some(paper) = mcq_format::paper_in_pasted(text) {
        read_source(paper)
    } else {
        read_bank(Source::detect_text(text)?, text, title, lang)
    };
    Some(refuse_errors(read, french))
}

/// Import warnings shown in the notice; the others are counted.
const REPORTED: usize = 4;

/// A file name as a title: `les_solutions-aqueuses` gives `Les solutions aqueuses`.
/// WHY: a bank without categories puts its questions in a theme named after the paper, and the
/// students read that theme on every question.
pub fn readable_title(stem: &str) -> String {
    let words: Vec<&str> = stem
        .split(['_', '-', ' '])
        .filter(|w| !w.is_empty())
        .collect();
    let text = words.join(" ");
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => text,
    }
}

/// What to do with a paper read from a file or the clipboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opening {
    Replace,
    Append,
}

/// The question asked before questions join a paper that has some: `mine` already there,
/// `theirs` brought by a file or, when `pasted`, by pasted text.
pub fn import_question(t: &Texts, pasted: bool, mine: usize, theirs: usize) -> String {
    crate::texts::fill(
        if pasted {
            t.import_choice_pasted
        } else {
            t.import_choice
        },
        &[("mine", mine.to_string()), ("theirs", theirs.to_string())],
    )
}

/// Opens a paper file: in place of a blank draft, or, when the draft holds questions, added
/// after them or in their place as the teacher picks.
pub async fn open_paper(
    studio: Signal<Studio>,
    mut notice: Signal<Option<String>>,
    t: &'static Texts,
    name: &str,
    bytes: Vec<u8>,
) {
    let lang = studio.read().project.paper.lang.clone();
    // A bank names its themes after its categories or the file: worth saying where they went.
    let bank = !bytes.starts_with(b"PK") && !name.to_ascii_lowercase().ends_with(".md");
    match read_paper(name, bytes, &lang, t.lang == Lang::Fr) {
        Ok(read) => open_read(studio, notice, t, read, bank, false).await,
        Err(e) => notice.set(Some(format!("{}: {e}.", t.not_a_paper))),
    }
}

/// Opens what was read from a file or from pasted text (`pasted`).
async fn open_read(
    mut studio: Signal<Studio>,
    mut notice: Signal<Option<String>>,
    t: &'static Texts,
    (project, issues): (Project, Vec<mcq_model::Issue>),
    bank: bool,
    pasted: bool,
) {
    // Nothing to lose over a blank draft or a demo paper as it was opened.
    let (replaceable, mine) = {
        let s = studio.read();
        (
            s.is_blank() || crate::demo::is_untouched_demo(&s.project),
            s.project.paper.questions.len(),
        )
    };
    let opening = if replaceable {
        Opening::Replace
    } else {
        let question = import_question(t, pasted, mine, project.paper.questions.len());
        match platform::choose(&question, &[t.import_add, t.import_replace, t.cancel]).await {
            Some(0) => Opening::Append,
            Some(1) => Opening::Replace,
            _ => return,
        }
    };
    // The import report says what stayed behind; it belongs to no question of the paper.
    let (report, issues): (Vec<_>, Vec<_>) = issues
        .into_iter()
        .partition(|i| i.code.starts_with("IMPORT_"));
    let said: Vec<String> = report
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .map(|i| t.issue(i))
        .collect();
    let themes: Vec<String> = project
        .paper
        .themes
        .iter()
        .map(|th| t.quote(&th.title))
        .collect();
    let mut text = match opening {
        Opening::Replace => {
            studio.write().replace(project, issues);
            t.imported.to_string()
        }
        Opening::Append => {
            let added = studio.write().append(project);
            crate::texts::fill(t.imported_added, &[("n", added.to_string())])
        }
    };
    if bank && !themes.is_empty() {
        text.push(' ');
        text.push_str(&crate::texts::fill(
            t.imported_themes,
            &[
                ("count", themes.len().to_string()),
                ("themes", themes.join(", ")),
            ],
        ));
    }
    for message in said.iter().take(REPORTED) {
        text.push(' ');
        text.push_str(message);
    }
    if said.len() > REPORTED {
        text.push_str(&format!(" (+{})", said.len() - REPORTED));
    }
    notice.set(Some(text));
}

/// Opens the paper inside pasted text, typically an AI's answer to the prompt on the website,
/// whatever it wrote around the file.
pub async fn open_pasted(
    studio: Signal<Studio>,
    mut notice: Signal<Option<String>>,
    t: &'static Texts,
    text: &str,
) {
    let lang = studio.read().project.paper.lang.clone();
    // A bank names its themes after its categories: worth saying where they went.
    let bank = mcq_format::paper_in_pasted(text).is_none();
    match read_pasted(text, t.pasted_title, &lang, t.lang == Lang::Fr) {
        Some(Ok(read)) => open_read(studio, notice, t, read, bank, true).await,
        Some(Err(e)) => notice.set(Some(format!("{}: {e}.", t.paste_failed))),
        None => notice.set(Some(t.paste_not_paper.to_string())),
    }
}

pub fn export_qcmz(studio: Signal<Studio>) {
    let s = studio.read();
    if let Ok(bytes) = qcmz::export(&s.project) {
        let name = format!("{}.qcmz", qcmz::file_stem(&s.project));
        platform::download(&name, &b64::encode(&bytes));
    }
}

pub fn export_source(studio: Signal<Studio>) {
    let s = studio.read();
    let name = format!("{}.qcm.md", qcmz::file_stem(&s.project));
    platform::download(&name, &b64::encode(s.project.source().as_bytes()));
}

/// The paper as a Markdown handout without answers, explanations, rules or notes, to print or
/// hand out.
pub fn export_student(studio: Signal<Studio>) {
    let s = studio.read();
    let text = handout::handout(&s.project.paper);
    let name = format!("{}-handout.md", qcmz::file_stem(&s.project));
    platform::download(&name, &b64::encode(text.as_bytes()));
}

/// A Moodle XML question bank; what a bank cannot hold is said in the notice.
pub fn export_moodle(
    studio: Signal<Studio>,
    mut notice: Signal<Option<String>>,
    t: &'static Texts,
) {
    let s = studio.read();
    let code: Vec<_> = s
        .project
        .paper
        .questions
        .iter()
        .flat_map(crate::preview::tokens)
        .collect();
    let exported = moodle::export(&s.project, &code);
    let name = format!("{}.xml", qcmz::file_stem(&s.project));
    platform::download(&name, &b64::encode(exported.xml.as_bytes()));
    let said: Vec<String> = exported.issues.iter().map(|i| t.issue(i)).collect();
    if !said.is_empty() {
        notice.set(Some(said.join(" ")));
    }
}
