//! The Publish space: the teacher's identity, how students take the paper, the package and its
//! codes, and the sheet printed for the exam room.

use std::rc::Rc;

use dioxus::prelude::*;
use mcq_crypto::{Identity, KdfParams};
use mcq_edit::qcmz;
use mcq_player::b64;
use mcq_player::icon::{Glyph, Icon, IconButton};
use mcq_publish::{Mode, Published, RoomSheet};

use crate::identity::IdentityCard;
use crate::platform;
use crate::state::Studio;
use crate::texts::Texts;

/// A publication of this session: what to show and what to download again.
pub struct Done {
    pub published: Published,
    pub sheet: RoomSheet,
    pub file_name: String,
}

pub fn room_sheet(sheet: &RoomSheet, t: &'static Texts) -> Element {
    let duration = if sheet.duration_minutes == 0 {
        t.no_limit.to_string()
    } else {
        format!("{} {}", sheet.duration_minutes, t.minutes)
    };
    let last = if sheet.opening_code.is_none() {
        t.sheet_practice
    } else if sheet.correction_later {
        t.sheet_correction_later
    } else {
        ""
    };
    rsx! {
        section { class: "room-sheet card printable", "aria-label": "{t.print_sheet}",
            h1 { "{sheet.title}" }
            dl { class: "facts",
                dt { "{t.sheet_questions}" } dd { "{sheet.question_count}" }
                dt { "{t.sheet_duration}" } dd { "{duration}" }
                if let Some(code) = &sheet.opening_code {
                    dt { "{t.opening_code}" } dd { class: "mono big", "{code}" }
                }
                dt { "{t.fingerprint}" } dd { class: "mono", "{sheet.fingerprint}" }
            }
            ol {
                for (k, step) in t.sheet_steps.iter().enumerate() {
                    if sheet.opening_code.is_some() || k != 2 { li { key: "{k}", "{step}" } }
                }
            }
            if !last.is_empty() { p { "{last}" } }
        }
    }
}

#[component]
pub fn PublishSpace(
    studio: Signal<Studio>,
    identity: Signal<Option<Rc<Identity>>>,
    done: Signal<Option<Rc<Done>>>,
    t: &'static Texts,
) -> Element {
    let mut practice = use_signal(|| false);
    let mut end_code = use_signal(|| true);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let blocking = mcq_publish::blocking_issues(&studio.read().project.paper).len();

    let publish = move |_| async move {
        let Some(teacher) = identity.read().clone() else {
            return;
        };
        busy.set(true);
        platform::next_frame().await;
        let mode = if practice() {
            Mode::Practice
        } else {
            Mode::Exam {
                end_code: end_code(),
            }
        };
        let s = studio.read();
        let result = mcq_publish::publish(
            &teacher,
            &s.project.paper,
            |path: &str| {
                s.project
                    .media
                    .get(path)
                    .cloned()
                    .ok_or_else(|| "missing".to_string())
            },
            mode,
            KdfParams::CODE,
            &mut platform::rng(),
        );
        match result {
            Ok(published) => {
                let file_name = format!("{}.mcqpack", qcmz::file_stem(&s.project));
                platform::download(&file_name, &b64::encode(&published.package));
                let sheet = RoomSheet::new(&s.project.paper, &published);
                done.set(Some(Rc::new(Done {
                    published,
                    sheet,
                    file_name,
                })));
                error.set(None);
            }
            Err(e) => error.set(Some(e.to_string())),
        }
        busy.set(false);
    };

    let last = done.read().clone();
    rsx! {
        div { class: "publish",
            if blocking > 0 { p { class: "notice warn", role: "alert", "{t.publish_blocked} ({blocking})" } }
            IdentityCard { identity, t }
            section { class: "card",
                h2 { "{t.mode}" }
                label { class: "check",
                    input { r#type: "radio", name: "mode", checked: !practice(), onchange: move |_| practice.set(false) }
                    "{t.mode_exam}"
                }
                if !practice() {
                    label { class: "check indent",
                        input { r#type: "checkbox", checked: end_code(), onchange: move |e| end_code.set(e.checked()) }
                        "{t.with_end_code}"
                    }
                }
                label { class: "check",
                    input { r#type: "radio", name: "mode", checked: practice(), onchange: move |_| practice.set(true) }
                    "{t.mode_practice}"
                }
                // A decisive action: the word stays, the icon only echoes the Publish space.
                button {
                    class: "primary",
                    "data-icon": "text",
                    disabled: busy() || blocking > 0 || identity.read().is_none(),
                    onclick: publish,
                    Glyph { icon: Icon::Send }
                    "{t.publish}"
                }
                if busy() { p { class: "muted", role: "status", "{t.working}" } }
                if let Some(message) = error() { p { class: "error", role: "alert", "{message}" } }
            }
            if let Some(last) = last {
                section { class: "card result",
                    p { "{t.published} " strong { class: "mono", "{last.file_name}" } }
                    if let Some(code) = &last.published.end_code {
                        div { class: "notice warn",
                            strong { "{t.end_code}" } " " span { class: "mono big", "{code}" }
                            p { "{t.end_code_hint}" }
                        }
                    }
                    div { class: "actions",
                        IconButton { class: "primary", icon: Icon::Print, label: t.print, tip: t.print_sheet, onclick: move |_| platform::print() }
                        IconButton { icon: Icon::Download, label: t.download, tip: t.tip_download_again, onclick: {
                            let last = Rc::clone(&last);
                            move |_| platform::download(&last.file_name, &b64::encode(&last.published.package))
                        } }
                    }
                }
                {room_sheet(&last.sheet, t)}
            }
        }
    }
}
