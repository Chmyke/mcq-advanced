//! The teacher demo (`?demo=teacher`): the demo paper in the editor, the copies of a fictional class
//! already in the Correct space, and a short tour of what to try.
//!
//! The demo identity is public: it opens the demo's own copies here and never becomes the
//! teacher's identity, so nothing real can be published with it.

use dioxus::prelude::*;
use mcq_edit::Project;

use crate::correction::Correction;
use crate::platform;
use crate::state::{Space, Studio};
use crate::texts::{Lang, Texts};

/// Where each step of the tour leads, in the order of `DemoTexts::steps`; `None` for the top bar.
pub const STEP_SPACES: [Option<Space>; 8] = [
    Some(Space::Write),
    Some(Space::Write),
    Some(Space::Settings),
    Some(Space::Publish),
    Some(Space::Correct),
    Some(Space::Correct),
    Some(Space::Correct),
    None,
];

/// The demo paper in this language and its class's copies, opened with the demo identity.
pub fn teacher(lang: Lang) -> Result<(Project, Correction), String> {
    let code = if lang == Lang::Fr { "fr" } else { "en" };
    let class = mcq_demo::class(mcq_demo::teacher_source(code))?;
    let identity = mcq_demo::identity();
    let mut correction = Correction::default();
    let (name, bytes) = class.package;
    correction.add_file(&name, bytes, Some(&identity));
    for (name, bytes) in class.copies {
        correction.add_file(&name, bytes, Some(&identity));
    }
    Ok((class.project, correction))
}

/// True when the draft is a demo paper, student, teacher or gallery model, in any language, as it
/// was opened: a visitor who tried one first loses nothing by opening another.
pub fn is_untouched_demo(draft: &Project) -> bool {
    mcq_demo::SOURCES
        .iter()
        .chain(&mcq_demo::TEACHER)
        .chain(mcq_demo::GALLERY.iter().map(|m| &m.source))
        .any(|s| mcq_demo::project(s) == *draft)
}

/// Opens the teacher demo:the paper over the draft (after the usual question, unless the draft
/// is blank or an untouched demo paper, as after a reload or a change of language), the copies in
/// the Correct space, the tour shown.
pub async fn open(
    mut studio: Signal<Studio>,
    mut correction: Signal<Correction>,
    mut notice: Signal<Option<String>>,
    mut tour: Signal<bool>,
    t: &'static Texts,
    lang: Lang,
) {
    notice.set(Some(t.demo.preparing.to_string()));
    platform::next_frame().await;
    let Ok((project, opened)) = teacher(lang) else {
        notice.set(Some(t.demo.failed.to_string()));
        return;
    };
    let (same, nothing_to_lose) = {
        let draft = studio.read();
        (
            draft.project == project,
            draft.is_blank() || is_untouched_demo(&draft.project),
        )
    };
    if !same && (nothing_to_lose || platform::confirm(t.confirm_replace).await) {
        studio.write().replace(project, Vec::new());
    }
    correction.set(opened);
    tour.set(true);
    notice.set(Some(t.demo.ready.to_string()));
}

/// The tour: one step per thing to try, each leading to its space.
#[component]
pub fn Tour(studio: Signal<Studio>, t: &'static Texts) -> Element {
    let d = t.demo;
    let current = studio.read().space;
    rsx! {
        details { class: "card tour", open: true,
            summary { strong { "{d.title}" } }
            p { class: "muted", "{d.intro}" }
            ol { class: "tour-steps",
                for (k, ((title, text), space)) in d.steps.iter().zip(STEP_SPACES).enumerate() {
                    li { key: "{k}",
                        if let Some(space) = space {
                            button { class: "link",
                                "aria-pressed": "{current == space}",
                                onclick: move |_| studio.write().space = space,
                                "{title}"
                            }
                        } else {
                            strong { "{title}" }
                        }
                        " "
                        span { "{text}" }
                    }
                }
            }
            p { class: "muted", "{d.identity_note}" }
        }
    }
}
