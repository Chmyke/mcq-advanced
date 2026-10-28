//! The Studio: three spaces (Write, Settings, Publish), files dropped or pasted anywhere, and the
//! draft kept in the browser after every change.

use std::rc::Rc;

use dioxus::prelude::*;
use mcq_crypto::Identity;
use mcq_edit::{Project, qcmz};
use mcq_player::icon::{Icon, IconButton, IconFile, IconMenu};
use mcq_player::{b64, render};

use crate::actions;
use crate::class_stats::ClassResults;
use crate::correct::CorrectSpace;
use crate::correction::{Correction, is_correction_file};
use crate::demo::{self, Tour};
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
    // Dark unless the teacher picks light, for the Studio (remembered on this device) and for its
    // student preview (not remembered).
    let mut dark = use_signal(|| true);
    let phone = use_signal(|| false);
    let preview_dark = use_signal(|| true);
    let identity = use_signal(|| None::<Rc<Identity>>);
    let done = use_signal(|| None::<Rc<Done>>);
    let mut correction = use_signal(Correction::default);
    let tour = use_signal(|| false);
    // Analysed again only when the copies or the decisions change, not at every keystroke.
    let class = use_memo(move || ClassResults::from_correction(&correction.read()));

    use_effect(move || {
        let tag = if lang() == Lang::Fr { "fr" } else { "en" };
        let _ = document::eval(&format!("document.documentElement.lang = '{tag}';"));
    });

    // Restores the draft, then saves it whenever it changed.
    use_future(move || async move {
        let language = Lang::from_tag(&platform::interface_language().await);
        lang.set(language);
        dark.set(platform::remembered_dark().await);
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
        // A paper given by link (`?open=...`, such as the website demo) opens over the draft,
        // after the usual question when the draft would be lost, and becomes the draft.
        match platform::paper_from_link().await {
            Some(Some((name, bytes))) => actions::open_paper(studio, notice, t, &name, bytes).await,
            Some(None) => notice.set(Some(t.link_failed.to_string())),
            None => {}
        }
        if platform::teacher_demo_from_link().await {
            demo::open(studio, correction, notice, tour, t, language).await;
        }
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

    mcq_player::icon::use_keep_in_view();
    // Files dropped on the page or pasted: papers open, media join the selected question. Text
    // pasted outside a field opens as a paper when it holds one.
    use_future(move || async move {
        let mut events = document::eval(DROP_AND_PASTE_SCRIPT);
        while let Ok((name, data)) = events.recv::<(String, String)>().await {
            let t = texts(lang());
            if name == platform::PASTED_TEXT {
                actions::open_pasted(studio, notice, t, &data).await;
                continue;
            }
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
    let theme = if dark() { "dark" } else { "light" };
    let space = studio.read().space;
    let title = studio.read().project.paper.title.clone();
    let tabs = [
        (Space::Write, Icon::Pen, t.space_write, t.tip_space_write),
        (
            Space::Settings,
            Icon::Sliders,
            t.space_settings,
            t.tip_space_settings,
        ),
        (
            Space::Publish,
            Icon::Send,
            t.space_publish,
            t.tip_space_publish,
        ),
        (
            Space::Correct,
            Icon::Correct,
            t.space_correct,
            t.tip_space_correct,
        ),
    ];
    rsx! {
        style { {STYLE} }
        style { {STUDIO_STYLE} }
        div { class: "app studio", "data-theme": "{theme}",
            header { class: "top studio-top",
                div { class: "brand", "{t.brand}" span { class: "muted", " · {title}" } }
                nav { class: "spaces segmented", "aria-label": "{t.brand}",
                    for (s, icon, label, tip) in tabs {
                        IconButton {
                            key: "{label}",
                            icon, label, tip, shown: true,
                            aria_pressed: "{space == s}",
                            onclick: move |_| studio.write().space = s,
                        }
                    }
                }
                div { class: "prefs",
                    IconButton { icon: Icon::FileNew, label: t.new_paper, tip: t.tip_new,
                        onclick: move |_| async move {
                            let blank = studio.read().is_blank();
                            if blank || platform::confirm(t.confirm_replace).await {
                                let code = if lang() == Lang::Fr { "fr" } else { "en" };
                                studio.write().replace(Project::starter(t.new_paper_title, code), Vec::new());
                            }
                        },
                    }
                    IconFile { icon: Icon::Open, label: t.import, tip: t.tip_import, accept: actions::PAPER_FILES,
                        onchange: move |e: FormEvent| async move {
                            if let Some(file) = e.files().into_iter().next()
                                && let Ok(bytes) = file.read_bytes().await
                            {
                                actions::open_paper(studio, notice, t, &file.name(), bytes.to_vec()).await;
                            }
                        },
                    }
                    IconButton { icon: Icon::Paste, label: t.paste, tip: t.tip_paste,
                        onclick: move |_| async move {
                            match platform::read_clipboard().await {
                                Some(text) => actions::open_pasted(studio, notice, t, &text).await,
                                None => notice.set(Some(t.paste_refused.to_string())),
                            }
                        },
                    }
                    IconMenu { icon: Icon::Download, label: t.export, tip: t.tip_export,
                        button { onclick: move |_| actions::export_qcmz(studio), "{t.export_qcmz}" }
                        button { onclick: move |_| actions::export_source(studio), "{t.export_source}" }
                        button { onclick: move |_| actions::export_student(studio), "{t.export_student}" }
                        button { onclick: move |_| actions::export_moodle(studio, notice, t), "{t.export_moodle}" }
                    }
                    {render::theme_switch(dark(), [t.theme_dark, t.theme_light], [t.tip_theme_dark, t.tip_theme_light],
                        EventHandler::new(move |pick| { dark.set(pick); platform::remember_dark(pick); }))}
                }
            }
            div { class: "status-line",
                // Only the notices are announced, not the "saved" mark after every keystroke.
                span { role: "status",
                    if let Some(message) = notice() {
                        span { "{message}" }
                        IconButton { class: "link", icon: Icon::Close, label: t.close, onclick: move |_| notice.set(None) }
                    }
                }
                if saved() { span { class: "muted saved", "{t.saved_locally}" } }
            }
            if tour() { Tour { studio, t } }
            if ready() {
                div { class: "studio-main",
                    match space {
                        Space::Write => write_space(studio, t, recording, notice, phone, preview_dark, class.read().as_ref()),
                        Space::Settings => settings_space(studio, t),
                        Space::Publish => rsx! { PublishSpace { studio, identity, done, t } },
                        Space::Correct => rsx! { CorrectSpace { correction, identity, done, lang: lang(), t } },
                    }
                }
            }
        }
    }
}
