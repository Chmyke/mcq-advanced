//! The question as a student will see it, drawn by the player's own views and stylesheet.

use std::cell::OnceCell;

use dioxus::prelude::*;
use mcq_engine::SessionItem;
use mcq_highlight::Highlighter;
use mcq_model::{CodeTokens, Question};
use mcq_player::render::{self, Content};

use crate::state::Studio;
use crate::texts::{Lang, Texts};

thread_local! {
    /// Loading the grammars takes a moment: once per run, on first use.
    static HIGHLIGHTER: OnceCell<Highlighter> = const { OnceCell::new() };
}

/// The coloured lines of a question's code blocks, as the package will carry them.
pub fn tokens(question: &Question) -> Vec<CodeTokens> {
    HIGHLIGHTER.with(|cell| {
        let highlighter = cell.get_or_init(Highlighter::new);
        question
            .code_blocks()
            .enumerate()
            .map(|(block, code)| CodeTokens {
                question_id: question.id.clone(),
                block: u32::try_from(block).unwrap_or(u32::MAX),
                lines: highlighter.highlight(&code.language, &code.source),
            })
            .collect()
    })
}

pub fn question_preview(
    studio: &Studio,
    question: &Question,
    index: usize,
    t: &'static Texts,
    mut phone: Signal<bool>,
    mut dark: Signal<bool>,
) -> Element {
    let paper = &studio.project.paper;
    let tokens = tokens(question);
    let content = Content {
        paper,
        media: studio.media_urls(),
        tokens: &tokens,
    };
    let item = SessionItem {
        question_id: question.id.clone(),
        option_order: question.options.iter().map(|o| o.id.clone()).collect(),
        ..SessionItem::default()
    };
    // The student's interface speaks the paper's language.
    let student = Lang::from_tag(&paper.lang).texts();
    let view = render::question_view(
        question,
        &item,
        index,
        paper.questions.len(),
        content,
        student,
        EventHandler::new(|_: String| {}),
        EventHandler::new(|_: ()| {}),
    );
    let theme = if dark() { "dark" } else { "light" };
    rsx! {
        section { class: "preview", "aria-label": "{t.preview}",
            div { class: "preview-bar",
                h2 { "{t.preview}" }
                div { class: "segmented",
                    button { "aria-pressed": "{!phone()}", onclick: move |_| phone.set(false), "{t.preview_computer}" }
                    button { "aria-pressed": "{phone()}", onclick: move |_| phone.set(true), "{t.preview_phone}" }
                }
                div { class: "segmented",
                    button { "aria-pressed": "{!dark()}", onclick: move |_| dark.set(false), "{t.theme_light}" }
                    button { "aria-pressed": "{dark()}", onclick: move |_| dark.set(true), "{t.theme_dark}" }
                }
            }
            div { class: if phone() { "app preview-frame phone" } else { "app preview-frame" }, "data-theme": "{theme}",
                {view}
            }
        }
    }
}
