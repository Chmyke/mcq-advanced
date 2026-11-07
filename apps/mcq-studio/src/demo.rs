//! The teacher demo (`?demo=teacher`): the demo paper in the editor, the copies of a fictional class
//! already in the Correct space, and a short tour of what to try.
//!
//! The demo identity is public: it opens the demo's own copies here and never becomes the
//! teacher's identity, so nothing real can be published with it.

use dioxus::prelude::*;
use mcq_edit::{Project, qcmz};
use mcq_player::b64;

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

/// What opening the demo does with the draft in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// Nothing to lose (a blank draft, a demo paper as it was opened): the demo paper replaces it.
    Replace,
    /// A visit is under way, the teacher's draft already set aside (a reload): the paper in the
    /// editor is the demo's, kept as it is, changes included.
    Resume,
    /// The draft holds work: ask first, then set it aside for the time of the visit.
    Ask,
}

/// What opening the demo does with `draft`; `visiting` when a draft is already set aside.
pub fn entry(draft: &Studio, visiting: bool) -> Entry {
    if draft.is_blank() || is_untouched_demo(&draft.project) {
        Entry::Replace
    } else if visiting {
        Entry::Resume
    } else {
        Entry::Ask
    }
}

/// True when leaving the visit would drop work: the demo paper was changed during it.
pub fn leaving_drops_changes(draft: &Studio) -> bool {
    !draft.is_blank() && !is_untouched_demo(&draft.project)
}

/// Opens the teacher demo: the demo paper in the editor, the teacher's draft set aside until the
/// visit ends (after asking, when it holds work), the copies in the Correct space, the tour
/// shown. Declined, nothing changes. Nothing is published or kept as published: Correct takes the
/// last published paper again once the visit is over.
pub async fn open(
    mut studio: Signal<Studio>,
    mut correction: Signal<Correction>,
    mut notice: Signal<Option<String>>,
    mut tour: Signal<bool>,
    t: &'static Texts,
    lang: Lang,
) {
    let visiting = platform::aside_draft().await.is_some();
    let entering = entry(&studio.read(), visiting);
    if entering == Entry::Ask
        && platform::choose(t.demo.confirm, &[t.demo.open, t.cancel]).await != Some(0)
    {
        platform::forget_demo_link();
        return;
    }
    notice.set(Some(t.demo.preparing.to_string()));
    platform::next_frame().await;
    let Ok((project, opened)) = teacher(lang) else {
        notice.set(Some(t.demo.failed.to_string()));
        return;
    };
    match entering {
        Entry::Ask => {
            // WHY first and checked: the draft must be safe before the demo paper replaces it.
            let Ok(bytes) = qcmz::export(&studio.read().project) else {
                notice.set(Some(t.demo.failed.to_string()));
                return;
            };
            if !platform::set_aside_draft(b64::encode(&bytes)).await {
                notice.set(Some(t.demo.failed.to_string()));
                return;
            }
            studio.write().replace(project, Vec::new());
        }
        Entry::Replace if studio.read().project != project => {
            studio.write().replace(project, Vec::new());
        }
        Entry::Replace | Entry::Resume => {}
    }
    correction.set(opened);
    tour.set(true);
    notice.set(Some(t.demo.ready.to_string()));
}

/// Ends the visit: the draft set aside comes back (a blank paper when there was none), the demo
/// copies leave Correct, the tour closes. Asks first when the demo paper was changed.
pub async fn leave(
    mut studio: Signal<Studio>,
    mut correction: Signal<Correction>,
    mut notice: Signal<Option<String>>,
    mut tour: Signal<bool>,
    t: &'static Texts,
    lang: Lang,
) {
    if leaving_drops_changes(&studio.read()) && !platform::confirm(t.demo.leave_confirm).await {
        return;
    }
    let aside = platform::aside_draft()
        .await
        .and_then(|text| b64::decode(&text))
        .and_then(|bytes| qcmz::import(&bytes).ok());
    {
        let mut s = studio.write();
        match aside {
            Some((project, issues)) => s.replace(project, issues),
            None => {
                let code = if lang == Lang::Fr { "fr" } else { "en" };
                s.replace(Project::starter(t.new_paper_title, code), Vec::new());
            }
        }
        s.space = Space::Write;
    }
    // Cleared once the draft is back in the editor, which the autosave keeps from now on.
    platform::set_aside_draft(String::new()).await;
    platform::forget_demo_link();
    correction.set(Correction::default());
    tour.set(false);
    notice.set(Some(t.demo.left.to_string()));
}

/// The tour: one step per thing to try, each leading to its space, and the way out.
#[component]
pub fn Tour(
    studio: Signal<Studio>,
    correction: Signal<Correction>,
    notice: Signal<Option<String>>,
    tour: Signal<bool>,
    lang: Lang,
    t: &'static Texts,
) -> Element {
    let d = t.demo;
    let current = studio.read().space;
    rsx! {
        details { class: "card tour", open: true,
            summary { strong { "{d.title}" } }
            button { class: "leave-tour",
                onclick: move |_| async move { leave(studio, correction, notice, tour, t, lang).await },
                "{d.leave}"
            }
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
