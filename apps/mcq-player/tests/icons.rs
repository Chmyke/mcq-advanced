//! The icons: drawn once each, and every icon button named, with a tip that says what it does.

use std::cell::RefCell;

use dioxus::prelude::*;
use mcq_player::icon::{Icon, IconButton, IconFile, contract_faults};
use mcq_player::texts::Lang;

type View = Box<dyn Fn() -> Element>;

thread_local! {
    static VIEW: RefCell<Option<View>> = const { RefCell::new(None) };
}

#[allow(non_snake_case)]
fn Harness() -> Element {
    VIEW.with(|v| v.borrow().as_ref().map_or_else(|| rsx! {}, |f| f()))
}

fn render(view: impl Fn() -> Element + 'static) -> String {
    VIEW.with(|v| *v.borrow_mut() = Some(Box::new(view)));
    let mut dom = VirtualDom::new(Harness);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn every_icon_is_its_own_drawing_on_the_grid() {
    for (k, icon) in Icon::ALL.iter().enumerate() {
        let path = icon.path();
        assert!(path.starts_with('M'), "{icon:?}");
        assert!(
            path.chars()
                .all(|c| "MmLlHhVvCcSsQqTtAaZz0123456789.,- ".contains(c)),
            "{icon:?}: {path}"
        );
        for other in &Icon::ALL[k + 1..] {
            assert_ne!(icon, other, "listed twice");
            assert_ne!(path, other.path(), "{icon:?} and {other:?} look the same");
        }
    }
}

#[test]
fn an_icon_button_is_named_by_its_tip_and_hides_its_drawing() {
    let html = render(|| {
        rsx! {
            IconButton { icon: Icon::Trash, label: "Delete", tip: "Delete this question" }
        }
    });
    assert!(
        html.contains("aria-label=\"Delete this question\""),
        "{html}"
    );
    assert!(html.contains("<svg class=\"glyph\""), "{html}");
    assert!(html.contains("aria-hidden=\"true\""), "{html}");
    assert!(contract_faults(&html).is_empty(), "{html}");
}

#[test]
fn without_a_tip_the_label_is_the_name() {
    let html = render(|| rsx! { IconButton { icon: Icon::Close, label: "Close" } });
    assert!(html.contains("aria-label=\"Close\""), "{html}");
    assert!(contract_faults(&html).is_empty(), "{html}");
}

#[test]
fn a_file_picker_names_its_hidden_input() {
    let html = render(|| {
        rsx! {
            IconFile { icon: Icon::Open, label: "Import", tip: "Import a paper", accept: ".md", onchange: |_| {} }
        }
    });
    assert!(html.contains("type=\"file\""), "{html}");
    assert!(contract_faults(&html).is_empty(), "{html}");
}

#[test]
fn a_name_that_drops_the_shown_word_is_a_fault() {
    let html = render(
        || rsx! { IconButton { icon: Icon::ArrowUp, label: "Up", tip: "Move this question" } },
    );
    assert_eq!(contract_faults(&html).len(), 1, "{html}");
    let unnamed = "<button data-icon=\"tip\"><svg></svg><span class=\"label\">Up</span></button>";
    assert!(!contract_faults(unnamed).is_empty());
}

/// The icons of the main screen, rendered by the App only: each label is part of its tip.
#[test]
fn the_tips_of_the_exam_screen_contain_their_labels() {
    for lang in [Lang::En, Lang::Fr] {
        let t = lang.texts();
        let pairs = [
            (t.light_short, t.theme_light),
            (t.dark_short, t.theme_dark),
            (t.smaller_short, t.text_smaller),
            (t.larger_short, t.text_larger),
            (t.shortcuts_short, t.shortcuts),
            (t.previous, t.previous_tip),
            (t.next, t.next_tip),
            (t.flagged_short, t.next_flagged),
            (t.flag_short, t.flag),
            (t.sheet_short, t.download_sheet),
            (t.print_short, t.print_correction),
        ];
        for (label, tip) in pairs {
            assert!(
                tip.to_lowercase().contains(&label.to_lowercase()),
                "\"{tip}\" lacks \"{label}\""
            );
        }
    }
}
