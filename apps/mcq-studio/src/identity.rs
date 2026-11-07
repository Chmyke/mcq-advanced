//! The teacher's identity, created or unlocked once per session: Publish signs with it, Correct
//! opens copies with it.
//!
//! With an identity kept in the browser, unlocking it is the only way shown: creating another one
//! is folded behind a link and warned about, since the copies of the papers already published
//! open with the kept one only.

use std::rc::Rc;

use dioxus::prelude::*;
use mcq_crypto::{Identity, KdfParams, MIN_PASSPHRASE_CHARS};
use mcq_player::b64;

use crate::platform;
use crate::texts::Texts;

/// The name of the identity file the Studio downloads.
pub const IDENTITY_FILE: &str = "teacher.mcqid";

#[component]
pub fn IdentityCard(identity: Signal<Option<Rc<Identity>>>, t: &'static Texts) -> Element {
    let mut identity_file = use_signal(|| None::<Vec<u8>>);
    // The file came from this browser, not from a file the teacher chose.
    let mut kept = use_signal(|| false);
    let creating = use_signal(|| false);
    // The locked file of an identity created here, to download again.
    let created_file = use_signal(|| None::<Vec<u8>>);
    use_future(move || async move {
        if identity_file.peek().is_none()
            && let Some(bytes) = platform::kept_identity()
                .await
                .and_then(|text| b64::decode(&text))
        {
            identity_file.set(Some(bytes));
            kept.set(true);
        }
    });

    let ready = identity.read().as_ref().map(|i| i.public().fingerprint());
    rsx! {
        section { class: "card",
            h2 { "{t.identity}" }
            p { class: "muted", "{t.identity_hint}" }
            if let Some(fingerprint) = ready {
                p { class: "notice", "{t.identity_ready} " strong { class: "mono", "{fingerprint}" } }
                p { class: "muted small-text", "{t.fingerprint_hint}" }
                if let Some(file) = created_file() {
                    div { class: "notice warn",
                        p { "{t.identity_saved}" }
                        button { onclick: move |_| platform::download(IDENTITY_FILE, &b64::encode(&file)), "{t.identity_download_again}" }
                    }
                }
            } else {
                IdentityForms { identity, identity_file, kept, creating, created_file, t }
            }
        }
    }
}

/// The ways to an identity while none is unlocked: load or unlock one, or create one. `kept` is
/// true when `identity_file` came from the browser; `creating` unfolds the creation then.
#[component]
pub fn IdentityForms(
    identity: Signal<Option<Rc<Identity>>>,
    identity_file: Signal<Option<Vec<u8>>>,
    kept: Signal<bool>,
    creating: Signal<bool>,
    created_file: Signal<Option<Vec<u8>>>,
    t: &'static Texts,
) -> Element {
    let mut identity_file = identity_file;
    let mut kept = kept;
    let mut creating = creating;
    let mut created_file = created_file;
    let mut passphrase = use_signal(String::new);
    let mut new_phrase = use_signal(String::new);
    let mut new_phrase_again = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<&'static str>);
    // Kept in this browser after creating or unlocking it.
    let mut keep = use_signal(|| true);

    // Enter in the passphrase submits the form, as the button does.
    let unlock = move |e: FormEvent| {
        e.prevent_default();
        async move {
            let Some(file) = identity_file.read().clone() else {
                return;
            };
            busy.set(true);
            platform::next_frame().await;
            match Identity::unlock(&file, &passphrase()) {
                Ok(unlocked) => {
                    if keep() {
                        platform::keep_identity(b64::encode(&file)).await;
                    }
                    identity.set(Some(Rc::new(unlocked)));
                    error.set(None);
                }
                Err(_) => error.set(Some(t.wrong_passphrase)),
            }
            passphrase.set(String::new());
            busy.set(false);
        }
    };

    // Enter in either field submits; in the first one with the second still empty, it only moves
    // on to the second.
    let create = move |e: FormEvent| {
        e.prevent_default();
        async move {
            if new_phrase_again().is_empty() && !new_phrase().is_empty() {
                let _ = document::eval("document.getElementById('new-pass-2')?.focus();");
                return;
            }
            if new_phrase() != new_phrase_again() {
                error.set(Some(t.passphrases_differ));
                return;
            }
            if new_phrase().chars().count() < MIN_PASSPHRASE_CHARS {
                error.set(Some(t.passphrase_too_short));
                return;
            }
            busy.set(true);
            platform::next_frame().await;
            let mut rng = platform::rng();
            let created = Identity::generate(&mut rng);
            if let Ok(file) = created.lock(&new_phrase(), KdfParams::PASSPHRASE, &mut rng) {
                platform::download(IDENTITY_FILE, &b64::encode(&file));
                if keep() {
                    platform::keep_identity(b64::encode(&file)).await;
                }
                created_file.set(Some(file));
                identity.set(Some(Rc::new(created)));
                error.set(None);
            }
            new_phrase.set(String::new());
            new_phrase_again.set(String::new());
            busy.set(false);
        }
    };

    // A kept identity is unlocked, not replaced by mistake: creation waits behind its link.
    let show_create = !kept() || creating();
    rsx! {
        if kept() {
            p { class: "notice", "{t.identity_kept}" }
        }
        div { class: if show_create { "two-columns" } else { "" },
            div {
                label { class: "file",
                    "{t.load_identity}"
                    input { r#type: "file", accept: ".mcqid", class: "sr-only",
                        onchange: move |e: FormEvent| async move {
                            if let Some(file) = e.files().into_iter().next()
                                && let Ok(bytes) = file.read_bytes().await
                            {
                                identity_file.set(Some(bytes.to_vec()));
                            }
                        },
                    }
                }
                if identity_file.read().is_some() {
                    form { class: "unlock", onsubmit: unlock,
                        label { r#for: "id-pass", "{t.passphrase}" }
                        input { id: "id-pass", r#type: "password", autocomplete: "current-password", value: "{passphrase}",
                            oninput: move |e| passphrase.set(e.value()) }
                        button { r#type: "submit", class: "primary", disabled: busy(), "{t.unlock}" }
                    }
                }
                if kept() {
                    button { r#type: "button", class: "link",
                        onclick: move |_| async move {
                            platform::forget_identity().await;
                            identity_file.set(None);
                            kept.set(false);
                            creating.set(false);
                        },
                        "{t.identity_forget}"
                    }
                }
            }
            if show_create {
                div {
                    h3 { "{t.create_identity}" }
                    if kept() {
                        p { class: "notice warn", role: "note", "{t.create_other_warning}" }
                    }
                    form { class: "create", onsubmit: create,
                        label { r#for: "new-pass", "{t.new_passphrase}" }
                        input { id: "new-pass", r#type: "password", autocomplete: "new-password", value: "{new_phrase}",
                            oninput: move |e| new_phrase.set(e.value()) }
                        label { r#for: "new-pass-2", "{t.confirm_passphrase}" }
                        input { id: "new-pass-2", r#type: "password", autocomplete: "new-password", value: "{new_phrase_again}",
                            oninput: move |e| new_phrase_again.set(e.value()) }
                        button { r#type: "submit", disabled: busy(), "{t.create_and_download}" }
                    }
                }
            }
        }
        if !show_create {
            p {
                button { r#type: "button", class: "link create-other", "aria-expanded": "false",
                    onclick: move |_| creating.set(true),
                    "{t.create_other_identity}"
                }
            }
        }
        label { class: "check",
            input { r#type: "checkbox", checked: keep(), onchange: move |e| keep.set(e.checked()) }
            "{t.identity_keep}"
        }
        if busy() { p { class: "muted", role: "status", "{t.working}" } }
        if let Some(message) = error() { p { class: "error", role: "alert", "{message}" } }
    }
}
