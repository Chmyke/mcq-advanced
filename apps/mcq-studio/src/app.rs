//! The Studio: three spaces (Write, Settings, Publish), files dropped or pasted anywhere, and the
//! draft kept in the browser after every change.

use std::rc::Rc;

use dioxus::prelude::*;
use mcq_crypto::Identity;
use mcq_edit::{Project, qcmz};
use mcq_player::b64;

use crate::actions;
use crate::correct::CorrectSpace;
use crate::correction::{Correction, is_correction_file};
use crate::editor::write_space;
use crate::platform::{self, DROP_AND_PASTE_SCRIPT};
use crate::publish::{Done, PublishSpace};
use crate::settings::settings_space;
use crate::state::{Space, Studio};
use crate::texts::{Lang, texts};
use crate::{STUDIO_STYLE, STYLE};

/// How often the draft is saved when something changed, in milliseconds.
const AUTOSAVE_MS: u32 = 1500;

#[component]
pub fn App() -> Element {
    let mut lang = use_signal(Lang::default);
    let mut studio = use_signal(|| Studio::new(Project::starter("", "")));
    let mut ready = use_signal(|| false);
    let mut notice = use_signal(|| None::<String>);
    let mut saved = use_signal(|| false);
    let recording = use_signal(|| false);
    let mut dark = use_signal(|| None::<bool>);
    let phone = use_signal(|| false);
    let preview_dark = use_signal(|| false);
    let identity = use_signal(|| None::<Rc<Identity>>);
    let done = use_signal(|| None::<Rc<Done>>);
    let mut correction = use_signal(Correction::default);

    // Restores the draft, then saves it whenever it changed.
    use_future(move || async move {
        let language = Lang::from_tag(&platform::browser_language().await);
        lang.set(language);
        if dark().is_none() {
            dark.set(Some(platform::prefers_dark().await));
        }
        let t = texts(language);
        let restored = platform::load_draft()
            .await
            .and_then(|text| b64::decode(&text))
            .and_then(|bytes| qcmz::import(&bytes).ok());
        match restored {
            Some((project, issues)) => {
                studio.set(Studio::with_issues(project, issues));
                notice.set(Some(t.restored.to_string()));
            }
            None => {
                let code = if language == Lang::Fr { "fr" } else { "en" };
                studio.set(Studio::new(Project::starter(t.new_paper_title, code)));
            }
        }
        ready.set(true);
        let mut last_saved = studio.read().revision;
        loop {
            let _ = document::eval(&format!(
                "await new Promise((r) => setTimeout(r, {AUTOSAVE_MS})); return 0;"
            ))
            .join::<u32>()
            .await;
            let revision = studio.read().revision;
            if revision == last_saved {
                continue;
            }
            let bytes = qcmz::export(&studio.read().project);
            if let Ok(bytes) = bytes
                && platform::save_draft(b64::encode(&bytes)).await
            {
                last_saved = revision;
                saved.set(true);
            }
        }
    });

    // Files dropped on the page or pasted: papers open, media join the selected question.
    use_future(move || async move {
        let mut events = document::eval(DROP_AND_PASTE_SCRIPT);
        while let Ok((name, data)) = events.recv::<(String, String)>().await {
            let t = texts(lang());
            let Some(bytes) = b64::decode(&data) else {
                continue;
            };
            if actions::is_paper_file(&name) {
                actions::open_paper(studio, notice, t, &name, bytes).await;
                continue;
            }
            // Packages and copies go to the Correct space.
            if is_correction_file(&name) {
                let id = identity.read().clone();
                correction.write().add_file(&name, bytes, id.as_deref());
                studio.write().space = Space::Correct;
                continue;
            }
            let selected = studio.read().selected.clone();
            match selected {
                Some(id) => actions::add_media(studio, notice, t, &id, &name, bytes),
                None => notice.set(Some(t.no_question_for_media.to_string())),
            }
        }
    });

    let t = texts(lang());
    let theme = match dark() {
        Some(true) => "dark",
        Some(false) => "light",
        None => "auto",
    };
    let space = studio.read().space;
    let title = studio.read().project.paper.title.clone();
    let tabs = [
        (Space::Write, t.space_write),
        (Space::Settings, t.space_settings),
        (Space::Publish, t.space_publish),
        (Space::Correct, t.space_correct),
    ];
    rsx! {
        style { {STYLE} }
        style { {STUDIO_STYLE} }
        div { class: "app studio", "data-theme": "{theme}",
            header { class: "top studio-top",
                div { class: "brand", "{t.brand}" span { class: "muted", " · {title}" } }
                nav { class: "spaces segmented", "aria-label": "{t.brand}",
                    for (s, label) in tabs {
                        button {
                            key: "{label}",
                            "aria-pressed": "{space == s}",
                            onclick: move |_| studio.write().space = s,
                            "{label}"
                        }
                    }
                }
                div { class: "prefs",
                    button { onclick: move |_| async move {
                        let blank = studio.read().is_blank();
                        if blank || platform::confirm(t.confirm_replace).await {
                            let code = if lang() == Lang::Fr { "fr" } else { "en" };
                            studio.write().replace(Project::starter(t.new_paper_title, code), Vec::new());
                        }
                    }, "{t.new_paper}" }
                    label { class: "file",
                        "{t.import}"
                        input { r#type: "file", class: "sr-only", accept: "{actions::PAPER_FILES}",
                            onchange: move |e: FormEvent| async move {
                                if let Some(file) = e.files().into_iter().next()
                                    && let Ok(bytes) = file.read_bytes().await
                                {
                                    actions::open_paper(studio, notice, t, &file.name(), bytes.to_vec()).await;
                                }
                            },
                        }
                    }
                    button { onclick: move |_| actions::export_qcmz(studio), "{t.export_qcmz}" }
                    button { onclick: move |_| actions::export_source(studio), "{t.export_source}" }
                    button { onclick: move |_| actions::export_student(studio), "{t.export_student}" }
                    button { onclick: move |_| actions::export_moodle(studio, notice), "{t.export_moodle}" }
                    button {
                        "aria-pressed": if theme == "dark" { "true" } else { "false" },
                        onclick: move |_| dark.set(Some(!dark().unwrap_or(false))),
                        if theme == "dark" { "{t.theme_light}" } else { "{t.theme_dark}" }
                    }
                }
            }
            div { class: "status-line", role: "status", "aria-live": "polite",
                if let Some(message) = notice() {
                    span { "{message}" }
                    button { class: "link", "aria-label": "OK", onclick: move |_| notice.set(None), "✕" }
                }
                if saved() { span { class: "muted saved", "{t.saved_locally}" } }
            }
            if ready() {
                div { class: "studio-main",
                    match space {
                        Space::Write => write_space(studio, t, recording, notice, phone, preview_dark),
                        Space::Settings => settings_space(studio, t),
                        Space::Publish => rsx! { PublishSpace { studio, identity, done, t } },
                        Space::Correct => rsx! { CorrectSpace { correction, identity, done, lang: lang(), t } },
                    }
                }
            }
        }
    }
}
