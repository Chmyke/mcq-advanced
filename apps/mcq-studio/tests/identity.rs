//! The identity card while no identity is unlocked: unlock the kept one, or create one.

use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::*;
use mcq_crypto::Identity;
use mcq_studio::identity::IdentityForms;
use mcq_studio::texts::FR;

thread_local! {
    /// Whether the identity file came from the browser, and whether creation is unfolded.
    static STATE: Cell<(bool, bool)> = const { Cell::new((false, false)) };
}

#[allow(non_snake_case)]
fn Harness() -> Element {
    let (kept_now, creating_now) = STATE.with(Cell::get);
    let identity = use_signal(|| None::<Rc<Identity>>);
    let identity_file = use_signal(|| kept_now.then(|| vec![1, 2, 3]));
    let kept = use_signal(|| kept_now);
    let creating = use_signal(|| creating_now);
    let created_file = use_signal(|| None::<Vec<u8>>);
    rsx! { IdentityForms { identity, identity_file, kept, creating, created_file, t: &FR } }
}

fn render(kept: bool, creating: bool) -> String {
    STATE.with(|s| s.set((kept, creating)));
    let mut dom = VirtualDom::new(Harness);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn attr(text: &str) -> String {
    text.replace('\'', "&#39;")
}

#[test]
fn a_kept_identity_is_unlocked_and_creating_another_is_folded() {
    let html = render(true, false);
    assert!(html.contains(&attr(FR.unlock)), "{html}");
    assert!(html.contains(&attr(FR.identity_forget)), "{html}");
    assert!(html.contains(&attr(FR.create_other_identity)), "{html}");
    // No second set of passphrase fields side by side with the first.
    assert!(!html.contains("id=\"new-pass\""), "{html}");
    assert!(!html.contains(&attr(FR.create_and_download)), "{html}");
}

#[test]
fn creating_another_identity_unfolds_with_its_warning() {
    let html = render(true, true);
    assert!(html.contains("id=\"new-pass-2\""), "{html}");
    assert!(html.contains(&attr(FR.create_other_warning)), "{html}");
    assert!(!html.contains(&attr(FR.create_other_identity)), "{html}");
}

#[test]
fn without_a_kept_identity_creating_is_offered_at_once_without_warning() {
    let html = render(false, false);
    assert!(html.contains("id=\"new-pass\""), "{html}");
    assert!(!html.contains(&attr(FR.create_other_warning)), "{html}");
    assert!(!html.contains(&attr(FR.create_other_identity)), "{html}");
}

#[test]
fn enter_in_a_passphrase_submits_its_form() {
    // Each passphrase field sits in a form whose submit button is its action: Enter submits it.
    for (kept, creating, fields) in [
        (true, false, &["id-pass"][..]),
        (false, false, &["new-pass", "new-pass-2"][..]),
    ] {
        let html = render(kept, creating);
        for field in fields {
            let at = html.find(&format!("id=\"{field}\"")).expect(field);
            let form = html[..at].rfind("<form").expect("inside a form");
            assert!(
                html[form..at].find("</form>").is_none(),
                "{field} is outside its form: {html}"
            );
            let end = at + html[at..].find("</form>").expect("form closes");
            assert!(
                html[at..end].contains("type=\"submit\""),
                "{field}: no submit button in its form: {html}"
            );
        }
    }
}
