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
    let (project, issues) = if bytes.starts_with(b"PK") {
        qcmz::import(&bytes).map_err(|e| e.to_string())?
    } else {
        read_text(name, bytes, lang)?
    };
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
        let mut project = Project::default();
        let issues = project.set_source(&text);
        return Ok((project, issues));
    }
    let source = Source::detect(name, &text).ok_or_else(|| "unknown format".to_string())?;
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    let options = ImportOptions {
        title: stem.to_string(),
        lang: lang.to_string(),
    };
    let imported = mcq_convert::import(source, &text, &options);
    Ok((imported.project, imported.issues))
}

/// Import warnings shown in the notice; the others are counted.
const REPORTED: usize = 4;

/// Replaces the current paper, after asking when something would be lost.
pub async fn open_paper(
    mut studio: Signal<Studio>,
    mut notice: Signal<Option<String>>,
    t: &'static Texts,
    name: &str,
    bytes: Vec<u8>,
) {
    let lang = studio.read().project.paper.lang.clone();
    let (project, issues) = match read_paper(name, bytes, &lang, t.lang == Lang::Fr) {
        Ok(read) => read,
        Err(e) => {
            notice.set(Some(format!("{}: {e}.", t.not_a_paper)));
            return;
        }
    };
    // Nothing to lose over a blank draft or a demo paper as it was opened.
    let replaceable = {
        let s = studio.read();
        s.is_blank() || crate::demo::is_untouched_demo(&s.project)
    };
    if !replaceable && !platform::confirm(t.confirm_replace).await {
        return;
    }
    // The import report says what stayed behind; it belongs to no question of the paper.
    let (report, issues): (Vec<_>, Vec<_>) = issues
        .into_iter()
        .partition(|i| i.code.starts_with("IMPORT_"));
    let said: Vec<&str> = report
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .map(|i| t.issue(i))
        .collect();
    let mut text = t.imported.to_string();
    for message in said.iter().take(REPORTED) {
        text.push(' ');
        text.push_str(message);
    }
    if said.len() > REPORTED {
        text.push_str(&format!(" (+{})", said.len() - REPORTED));
    }
    studio.write().replace(project, issues);
    notice.set(Some(text));
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
    let exported = moodle::export(&s.project);
    let name = format!("{}.xml", qcmz::file_stem(&s.project));
    platform::download(&name, &b64::encode(exported.xml.as_bytes()));
    let said: Vec<&str> = exported.issues.iter().map(|i| t.issue(i)).collect();
    if !said.is_empty() {
        notice.set(Some(said.join(" ")));
    }
}
