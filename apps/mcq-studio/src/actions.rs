//! What buttons and page events start: adding media, recording, opening and exporting papers.

use dioxus::prelude::*;
use mcq_edit::{Project, qcmz};
use mcq_player::b64;

use crate::platform::{self, RECORD_SCRIPT, STOP_RECORDING_SCRIPT};
use crate::state::Studio;
use crate::texts::Texts;

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

/// True for the files the Studio opens as a paper rather than adds as media.
pub fn is_paper_file(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.ends_with(".qcmz") || name.ends_with(".zip") || name.ends_with(".md")
}

/// Reads a `.qcmz` (or any zip of a paper folder) or a `.qcm.md`.
pub fn read_paper(bytes: Vec<u8>) -> Result<(Project, Vec<mcq_model::Issue>), String> {
    if bytes.starts_with(b"PK") {
        return qcmz::import(&bytes).map_err(|e| e.to_string());
    }
    let text = String::from_utf8(bytes).map_err(|_| "not UTF-8 text".to_string())?;
    let mut project = Project::default();
    let issues = project.set_source(&text);
    Ok((project, issues))
}

/// Replaces the current paper, after asking when something would be lost.
pub async fn open_paper(
    mut studio: Signal<Studio>,
    mut notice: Signal<Option<String>>,
    t: &'static Texts,
    bytes: Vec<u8>,
) {
    let (project, issues) = match read_paper(bytes) {
        Ok(read) => read,
        Err(e) => {
            notice.set(Some(format!("{}: {e}.", t.not_a_paper)));
            return;
        }
    };
    let blank = studio.read().is_blank();
    if !blank && !platform::confirm(t.confirm_replace).await {
        return;
    }
    studio.write().replace(project, issues);
    notice.set(Some(t.imported.to_string()));
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
    platform::download(&name, &b64::encode(s.current_source().as_bytes()));
}

/// The paper without correct flags, explanations, rules or notes, to print or hand out.
pub fn export_student(studio: Signal<Studio>) {
    let s = studio.read();
    let text = mcq_format::write(&mcq_format::student_view(&s.project.paper));
    let name = format!("{}-student.qcm.md", qcmz::file_stem(&s.project));
    platform::download(&name, &b64::encode(text.as_bytes()));
}
