//! The question as a student will see it, drawn by the player's own views and stylesheet.

use std::cell::OnceCell;

use dioxus::prelude::*;
use mcq_engine::SessionItem;
use mcq_highlight::Highlighter;
use mcq_math::Formulas;
use mcq_model::{Block, CodeTokens, Question};
use mcq_player::icon::{Icon, IconButton};
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

/// The formulas of these texts as the package will carry them; those that do not convert show
/// as source, and the validator says why.
pub fn formulas(texts: &[&str]) -> Formulas {
    Formulas::from_package(mcq_math::convert_texts(texts))
}

/// The texts of a question a student sees: statement and options.
pub fn student_texts(question: &Question) -> Vec<&str> {
    let mut texts: Vec<&str> = question
        .body
        .iter()
        .filter_map(|b| match b {
            Block::Text(text) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    texts.extend(question.options.iter().map(|o| o.text.as_str()));
    texts
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
    let formulas = formulas(&student_texts(question));
    let content = Content {
        paper,
        media: studio.media_urls(),
        tokens: &tokens,
        formulas: &formulas,
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
        EventHandler::new(|_: ()| {}),
    );
    let theme = if dark() { "dark" } else { "light" };
    rsx! {
        section { class: "preview", "aria-label": "{t.preview}",
            div { class: "preview-bar",
                h2 { "{t.preview}" }
                div { class: "segmented",
                    IconButton { icon: Icon::Monitor, label: t.preview_computer, tip: t.tip_preview_computer, aria_pressed: "{!phone()}", onclick: move |_| phone.set(false) }
                    IconButton { icon: Icon::Phone, label: t.preview_phone, tip: t.tip_preview_phone, aria_pressed: "{phone()}", onclick: move |_| phone.set(true) }
                }
                // The preview's own theme, not remembered: the Studio's stays as picked.
                {render::theme_switch(dark(), [t.theme_dark, t.theme_light], [t.tip_preview_dark, t.tip_preview_light],
                    EventHandler::new(move |pick| dark.set(pick)))}
            }
            div { class: if phone() { "app preview-frame phone" } else { "app preview-frame" }, "data-theme": "{theme}",
                {view}
            }
        }
    }
}
