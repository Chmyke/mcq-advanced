//! The player: screens in order (open, identify, intro, exam, hand in, correction), the session
//! saved after every change, and the page events wired to the engine.

use std::rc::Rc;

use dioxus::prelude::*;
use mcq_crypto::{KeyUnlock, OpeningKind, PackageHeader, SealedResult};
use mcq_engine::{Session, decode_session, encode_session};
use mcq_grade::GradeResult;
use mcq_model::Paper;

use crate::platform::{self, EVENTS_SCRIPT};
use crate::render::{self, Opened};
use crate::texts::{Lang, Texts};
use crate::{STYLE, b64};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Welcome,
    Identify,
    Intro,
    Exam,
    Confirm,
    Submitted,
    Review,
}

struct Loaded {
    bytes: Vec<u8>,
    header: PackageHeader,
}

struct Graded {
    full: Paper,
    result: GradeResult,
    /// The student's formulas and those of the explanations.
    formulas: mcq_math::Formulas,
}

fn save(session: &Session) {
    platform::storage_set(
        &platform::save_key(&session.paper_id, &session.student_name),
        &b64::encode(&encode_session(session)),
    );
}

#[component]
pub fn App() -> Element {
    let mut lang = use_signal(Lang::default);
    let mut stage = use_signal(|| Stage::Welcome);
    let mut loaded = use_signal(|| None::<Rc<Loaded>>);
    let mut opened = use_signal(|| None::<Rc<Opened>>);
    let mut session = use_signal(|| None::<Session>);
    let mut sealed = use_signal(|| None::<SealedResult>);
    let mut graded = use_signal(|| None::<Rc<Graded>>);
    let mut error = use_signal(|| None::<&'static str>);
    let mut notice = use_signal(|| None::<&'static str>);
    let mut resumed = use_signal(|| false);
    let mut dark = use_signal(|| None::<bool>);
    let mut text_scale = use_signal(|| 100_u32);
    let mut name = use_signal(String::new);
    let mut code = use_signal(String::new);
    let mut end_code = use_signal(String::new);
    let mut away = use_signal(|| None::<u32>);

    use_future(move || async move {
        lang.set(Lang::from_tag(&platform::browser_language().await));
        // The theme starts as the system's, so the first click on the toggle switches it.
        if dark().is_none() {
            dark.set(Some(platform::prefers_dark().await));
        }
        // A paper given by link (`?paper=...`) opens without choosing a file.
        if let Some(bytes) = platform::package_from_link().await {
            match mcq_crypto::inspect(&bytes) {
                Ok(header) => {
                    loaded.set(Some(Rc::new(Loaded { bytes, header })));
                    stage.set(Stage::Identify);
                }
                Err(_) => error.set(Some(lang().texts().not_a_package)),
            }
        }
    });

    use_future(move || async move {
        let mut events = document::eval(EVENTS_SCRIPT);
        while let Ok(message) = events.recv::<String>().await {
            // Outside the exam, events change nothing (and must not re-render every second).
            if !matches!(stage(), Stage::Exam | Stage::Confirm) {
                continue;
            }
            let Some(paper) = opened.read().as_ref().map(Rc::clone) else {
                continue;
            };
            let mut guard = session.write();
            let Some(s) = guard.as_mut() else {
                continue;
            };
            match message.as_str() {
                "tick" => {
                    s.tick(1);
                    if let Some(seconds) = away() {
                        away.set(Some(seconds + 1));
                    }
                    let status = s.time_status(&paper.paper);
                    if status.show_pace_alert {
                        notice.set(Some(lang().texts().pace_alert));
                    }
                    if s.elapsed_seconds % 5 == 0 {
                        save(s);
                    }
                }
                "away" => {
                    s.focus_lost();
                    away.set(Some(0));
                    save(s);
                }
                "back" => {
                    s.focus_back(away().unwrap_or(0));
                    away.set(None);
                    save(s);
                }
                key if stage() == Stage::Exam => {
                    let key = key.trim_start_matches("k:");
                    match key {
                        "ArrowRight" => s.next(),
                        "ArrowLeft" => s.previous(),
                        "f" | "F" => s.toggle_flag(),
                        digit => {
                            let index = digit.parse::<usize>().ok().and_then(|n| n.checked_sub(1));
                            let option = index.and_then(|i| {
                                s.current_item()
                                    .and_then(|item| item.option_order.get(i).cloned())
                            });
                            if let Some(option) = option {
                                s.toggle(&paper.paper, &option);
                            }
                        }
                    }
                    save(s);
                }
                _ => {}
            }
        }
    });

    let t: &'static Texts = lang().texts();
    let theme = match dark() {
        Some(true) => "dark",
        Some(false) => "light",
        None => "auto",
    };
    let header_title = opened
        .read()
        .as_ref()
        .map(|o| o.paper.title.clone())
        .unwrap_or_else(|| "MCQ Advanced".to_string());

    let content = match stage() {
        Stage::Welcome => rsx! {
            section { class: "welcome card",
                h1 { "MCQ Advanced" }
                p { "{t.app_tagline}" }
                label { class: "file primary",
                    "{t.open_file}"
                    input {
                        r#type: "file",
                        accept: ".mcqpack",
                        class: "sr-only",
                        onchange: move |evt: FormEvent| async move {
                            let Some(file) = evt.files().into_iter().next() else { return };
                            let Ok(bytes) = file.read_bytes().await else {
                                error.set(Some(lang().texts().not_a_package));
                                return;
                            };
                            match mcq_crypto::inspect(&bytes) {
                                Ok(header) => {
                                    loaded.set(Some(Rc::new(Loaded { bytes: bytes.to_vec(), header })));
                                    error.set(None);
                                    stage.set(Stage::Identify);
                                }
                                Err(_) => error.set(Some(lang().texts().not_a_package)),
                            }
                        },
                    }
                }
                p { class: "muted", "{t.open_hint}" }
                if let Some(message) = error() { p { class: "error", role: "alert", "{message}" } }
                p { class: "muted", "{t.privacy}" }
            }
        },
        Stage::Identify => {
            let Some(package) = loaded.read().as_ref().map(Rc::clone) else {
                return rsx! {};
            };
            let fingerprint = package.header.teacher.fingerprint();
            let opening = package.header.opening;
            let code_label = if opening == OpeningKind::Passphrase {
                t.passphrase
            } else {
                t.opening_code
            };
            rsx! {
                form { class: "identify card",
                    onsubmit: move |evt: FormEvent| {
                        evt.prevent_default();
                        let texts = lang().texts();
                        let typed_name = mcq_engine::normalize_name(&name());
                        if typed_name.is_empty() {
                            error.set(Some(texts.name_required));
                            return;
                        }
                        let Some(package) = loaded.read().as_ref().map(Rc::clone) else { return };
                        let Ok(paper) = Opened::from_package(&package.bytes, &code()) else {
                            error.set(Some(texts.wrong_code));
                            return;
                        };
                        let paper_id = paper.header.paper_id;
                        let fresh = Session::new(&paper.paper, paper_id, &typed_name);
                        opened.set(Some(Rc::new(paper)));
                        error.set(None);
                        spawn(async move {
                            let key = platform::save_key(&paper_id, &fresh.student_name);
                            let restored = platform::storage_get(&key)
                                .await
                                .and_then(|text| b64::decode(&text))
                                .and_then(|bytes| decode_session(&bytes).ok())
                                .filter(|s| s.paper_id == paper_id && s.items.len() == fresh.items.len());
                            resumed.set(restored.is_some());
                            let current = restored.unwrap_or(fresh);
                            let done = current.submitted;
                            session.set(Some(current));
                            stage.set(if done { Stage::Submitted } else { Stage::Intro });
                        });
                    },
                    div { class: "fingerprint",
                        span { "{t.fingerprint}" }
                        strong { class: "mono", "{fingerprint}" }
                        p { class: "muted", "{t.fingerprint_hint}" }
                    }
                    if opening == OpeningKind::Open { p { class: "notice", "{t.practice_notice}" } }
                    label { r#for: "name", "{t.your_name}" }
                    input {
                        id: "name", r#type: "text", autocomplete: "name", autofocus: true,
                        placeholder: "{t.name_placeholder}", value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                    if opening != OpeningKind::Open {
                        label { r#for: "code", "{code_label}" }
                        input {
                            id: "code", r#type: "text", autocomplete: "off", spellcheck: "false",
                            class: if opening == OpeningKind::Code { "mono" } else { "" },
                            placeholder: if opening == OpeningKind::Code { t.code_placeholder } else { "" },
                            value: "{code}",
                            oninput: move |e| code.set(e.value()),
                        }
                    }
                    if let Some(message) = error() { p { class: "error", role: "alert", "{message}" } }
                    button { class: "primary", r#type: "submit", "{t.open_paper}" }
                }
            }
        }
        Stage::Intro => {
            let Some(o) = opened.read().as_ref().map(Rc::clone) else {
                return rsx! {};
            };
            render::intro(
                &o.paper,
                &o.formulas,
                &o.header,
                t,
                EventHandler::new(move |()| stage.set(Stage::Exam)),
                resumed(),
            )
        }
        Stage::Exam | Stage::Confirm => {
            let Some(o) = opened.read().as_ref().map(Rc::clone) else {
                return rsx! {};
            };
            let snapshot = session.read().clone();
            let Some(mut s) = snapshot else {
                return rsx! {};
            };
            let status = s.time_status(&o.paper);
            let total = s.items.len();
            let answered = s.answered_count();
            let unanswered = total - answered;
            let flagged = s.flagged_count();
            let current = s.current;
            let question_element = s.current_item().and_then(|item| {
                let question = o.paper.question(&item.question_id)?;
                let paper = Rc::clone(&o);
                Some(render::question_view(
                    question,
                    item,
                    current,
                    total,
                    o.content(),
                    t,
                    EventHandler::new(move |option: String| {
                        if let Some(s) = session.write().as_mut() {
                            s.toggle(&paper.paper, &option);
                            save(s);
                        }
                    }),
                    EventHandler::new(move |()| {
                        if let Some(s) = session.write().as_mut() {
                            s.toggle_flag();
                            save(s);
                        }
                    }),
                ))
            });
            let mut go = move |f: fn(&mut Session)| {
                if let Some(s) = session.write().as_mut() {
                    f(s);
                    save(s);
                }
            };
            rsx! {
                div { class: "exam",
                    div { class: "status-bar",
                        {render::clock(&status, s.elapsed_seconds, t)}
                        span { class: "progress", "{answered} / {total} {t.answered}" }
                    }
                    if status.overtime_seconds > 0 {
                        p { class: "notice warn", role: "status", "{t.time_up}" }
                    } else if status.final_warning {
                        p { class: "notice warn", role: "status", "{t.final_warning}" }
                    }
                    if let Some(message) = notice() {
                        p { class: "notice", role: "status",
                            "{message} "
                            button { class: "link", onclick: move |_| notice.set(None), "×" }
                        }
                    }
                    if stage() == Stage::Confirm {
                        div { class: "confirm card", role: "dialog", "aria-modal": "true", "aria-labelledby": "confirm-title",
                            h2 { id: "confirm-title", tabindex: "-1", "{t.confirm_title}" }
                            p { "{unanswered} {t.unanswered_left}, {flagged} {t.flagged_left}." }
                            div { class: "nav",
                                button { onclick: move |_| stage.set(Stage::Exam), "{t.keep_working}" }
                                button { class: "primary",
                                    onclick: move |_| {
                                        let Some(o) = opened.read().as_ref().map(Rc::clone) else { return };
                                        let copy = match session.write().as_mut() {
                                            Some(s) => {
                                                let copy = s.submit();
                                                save(s);
                                                copy
                                            }
                                            None => return,
                                        };
                                        if let Ok(result) = mcq_crypto::seal_result(&o.header, &copy, &mut platform::rng()) {
                                            sealed.set(Some(result));
                                        }
                                        stage.set(Stage::Submitted);
                                    },
                                    "{t.confirm_submit}"
                                }
                            }
                        }
                    }
                    {question_element}
                    div { class: "nav",
                        button { disabled: current == 0, onclick: move |_| go(Session::previous), "← {t.previous}" }
                        if flagged > 0 {
                            button { onclick: move |_| go(|s| { s.next_flagged(); }), "{t.next_flagged}" }
                        }
                        if current + 1 < total {
                            button { class: "primary", onclick: move |_| go(Session::next), "{t.next} →" }
                        } else {
                            button { class: "primary", onclick: move |_| open_confirm(stage), "{t.finish}" }
                        }
                    }
                    {render::grid(&s, t, EventHandler::new(move |i: usize| go_to(session, i)))}
                    if current + 1 < total {
                        button { class: "finish", onclick: move |_| open_confirm(stage), "{t.finish}" }
                    }
                }
            }
        }
        Stage::Submitted => {
            let Some(o) = opened.read().as_ref().map(Rc::clone) else {
                return rsx! {};
            };
            let practice = o.header.opening == OpeningKind::Open;
            let has_end_code = o.header.has_end_code;
            let receipt = sealed.read().as_ref().map(|r| r.receipt.clone());
            let file_name = session
                .read()
                .as_ref()
                .map(|s| format!("{}.mcqres", s.student_name))
                .unwrap_or_else(|| "copy.mcqres".to_string());
            let show = move |_| {
                let Some(package) = loaded.read().as_ref().map(Rc::clone) else {
                    return;
                };
                let typed = end_code();
                let unlock = if practice {
                    KeyUnlock::Practice
                } else {
                    KeyUnlock::EndCode(&typed)
                };
                let Ok((full, key_files)) =
                    mcq_crypto::open_answer_key_files(&package.bytes, unlock)
                else {
                    error.set(Some(lang().texts().wrong_code));
                    return;
                };
                let Some(s) = session.read().clone() else {
                    return;
                };
                let copy_answers: Vec<_> = s
                    .items
                    .iter()
                    .map(|i| mcq_model::Answer {
                        question_id: i.question_id.clone(),
                        selected: i.selected.clone(),
                        seconds_spent: i.seconds_spent,
                    })
                    .collect();
                let result = mcq_grade::grade(&full, &copy_answers, s.elapsed_seconds);
                error.set(None);
                let formulas = opened
                    .read()
                    .as_ref()
                    .map(|o| render::correction_formulas(&o.formulas, &key_files))
                    .unwrap_or_default();
                graded.set(Some(Rc::new(Graded {
                    full,
                    result,
                    formulas,
                })));
                stage.set(Stage::Review);
            };
            rsx! {
                section { class: "submitted card",
                    h1 { "{t.submitted_title}" }
                    if let Some(receipt) = receipt.clone() {
                        p { "{t.receipt}" }
                        p { class: "big mono", "{receipt}" }
                        p { class: "muted", "{t.receipt_hint}" }
                        button { class: "primary",
                            onclick: move |_| {
                                if let Some(r) = sealed.read().as_ref() {
                                    platform::download(&file_name, &b64::encode(&r.bytes));
                                }
                            },
                            "{t.save_copy}"
                        }
                    }
                    if practice {
                        button { onclick: show, "{t.show_correction}" }
                    } else if has_end_code {
                        label { r#for: "end", "{t.end_code}" }
                        input { id: "end", r#type: "text", class: "mono", autocomplete: "off",
                            placeholder: "{t.code_placeholder}", value: "{end_code}",
                            oninput: move |e| end_code.set(e.value()),
                        }
                        p { class: "muted", "{t.end_code_hint}" }
                        button { onclick: show, "{t.show_correction}" }
                    }
                    if let Some(message) = error() { p { class: "error", role: "alert", "{message}" } }
                }
            }
        }
        Stage::Review => {
            let (Some(o), Some(g), Some(s)) = (
                opened.read().as_ref().map(Rc::clone),
                graded.read().as_ref().map(Rc::clone),
                session.read().clone(),
            ) else {
                return rsx! {};
            };
            let content = render::Content {
                formulas: &g.formulas,
                ..o.content()
            };
            render::review_view(&g.full, &s, &g.result, t, content)
        }
    };

    rsx! {
        style { {STYLE} }
        div { class: "app", "data-theme": "{theme}", style: "font-size: {text_scale}%",
            header { class: "top",
                span { class: "brand", "{header_title}" }
                div { class: "prefs",
                    button { "aria-label": if dark() == Some(true) { t.theme_light } else { t.theme_dark },
                        title: if dark() == Some(true) { t.theme_light } else { t.theme_dark },
                        onclick: move |_| dark.set(Some(dark() != Some(true))),
                        if dark() == Some(true) { "☀" } else { "☾" }
                    }
                    button { "aria-label": "{t.text_smaller}", title: "{t.text_smaller}",
                        onclick: move |_| text_scale.set(text_scale().saturating_sub(10).max(80)), "A−" }
                    button { "aria-label": "{t.text_larger}", title: "{t.text_larger}",
                        onclick: move |_| text_scale.set((text_scale() + 10).min(160)), "A+" }
                    button { onclick: move |_| lang.set(if lang() == Lang::Fr { Lang::En } else { Lang::Fr }),
                        if lang() == Lang::Fr { "EN" } else { "FR" }
                    }
                }
            }
            main { {content} }
        }
    }
}

fn go_to(mut session: Signal<Option<Session>>, index: usize) {
    if let Some(s) = session.write().as_mut() {
        s.go_to(index);
        save(s);
    }
}

/// Shows the hand-in question at the top of the page, with the focus on it.
fn open_confirm(mut stage: Signal<Stage>) {
    stage.set(Stage::Confirm);
    let _ = document::eval(
        "setTimeout(() => { window.scrollTo(0, 0);          const h = document.getElementById('confirm-title'); if (h) h.focus(); }, 0);",
    );
}
