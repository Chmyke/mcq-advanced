//! The player: screens in order (open, identify, intro, exam, hand in, correction), the session
//! saved after every change, and the page events wired to the engine.

use std::rc::Rc;

use dioxus::prelude::*;
use mcq_crypto::{KeyUnlock, OpeningKind, PackageHeader, SealedResult};
use mcq_engine::{Session, decode_session, encode_session};
use mcq_grade::GradeResult;
use mcq_model::{Paper, TimeExtension};

use crate::icon::{Icon, IconButton};
use crate::platform::{self, EVENTS_SCRIPT};
use crate::render::{self, Opened};
use crate::review::{ResultAction, ReviewAction, ReviewState};
use crate::texts::{Lang, Texts, interface_for};
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
    let key = platform::save_key(&session.paper_id, &session.student_name);
    platform::storage_set(&key, &b64::encode(&encode_session(session)));
    platform::storage_stamp(&key);
}

#[component]
pub fn App() -> Element {
    let mut lang = use_signal(Lang::default);
    // From the link or the browser; an open paper may override it.
    let mut asked = use_signal(Lang::default);
    // Languages the switch cycles through: all of them until a paper is open, then what the
    // paper offers.
    let mut choices = use_signal(|| Lang::ALL.to_vec());
    let mut stage = use_signal(|| Stage::Welcome);
    let mut loaded = use_signal(|| None::<Rc<Loaded>>);
    let mut opened = use_signal(|| None::<Rc<Opened>>);
    let mut session = use_signal(|| None::<Session>);
    let mut sealed = use_signal(|| None::<SealedResult>);
    let mut graded = use_signal(|| None::<Rc<Graded>>);
    let mut error = use_signal(|| None::<&'static str>);
    let mut notice = use_signal(|| None::<&'static str>);
    let mut resumed = use_signal(|| false);
    // The sealed copy went to a file this time; until then the teacher does not have it.
    let mut copy_saved = use_signal(|| false);
    let mut erased = use_signal(|| false);
    // Dark unless the student picks light; the pick is remembered on this device.
    let mut dark = use_signal(|| true);
    let mut text_scale = use_signal(|| 100_u32);
    // Single-key shortcuts can be turned off (WCAG 2.1.4): speech input then ticks nothing by mistake.
    let mut shortcuts = use_signal(|| true);
    let mut name = use_signal(String::new);
    let mut code = use_signal(String::new);
    let mut end_code = use_signal(String::new);
    let mut away = use_signal(|| None::<u32>);
    let mut review = use_signal(ReviewState::default);
    // The paper's source on this site, for "open in the editor"; empty when the link gave none.
    let mut source = use_signal(String::new);
    // The paper came from a `?paper=` link, so the page's address can be passed on.
    let mut shareable = use_signal(|| false);
    let mut shared = use_signal(|| None::<&'static str>);
    // The title of a copy left unfinished on this device, for the welcome screen.
    let mut unfinished = use_signal(|| None::<String>);
    let mut can_share = use_signal(|| false);
    // Grades the handed-in copy with the unlocked key and opens the correction; false when the
    // key stays locked.
    let mut open_correction = move |unlock: KeyUnlock| -> bool {
        let Some(package) = loaded.read().as_ref().map(Rc::clone) else {
            return false;
        };
        let Ok((full, key_files)) = mcq_crypto::open_answer_key_files(&package.bytes, unlock)
        else {
            return false;
        };
        let Some(s) = session.read().clone() else {
            return false;
        };
        let copy_answers: Vec<_> = s
            .items
            .iter()
            .map(|i| mcq_model::Answer {
                question_id: i.question_id.clone(),
                selected: i.selected.clone(),
                seconds_spent: i.seconds_spent,
                reported: i.reported,
            })
            .collect();
        let result = mcq_grade::grade(&full, &copy_answers, s.elapsed_seconds, s.extension);
        review.set(ReviewState::default());
        shared.set(None);
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
        let _ = document::eval("setTimeout(() => window.scrollTo(0, 0), 0);");
        true
    };
    let mut last_current = use_signal(|| 0_usize);
    use_effect(move || {
        let current = session.read().as_ref().map_or(0, |s| s.current);
        if current != *last_current.peek() {
            last_current.set(current);
            focus_question();
        }
    });
    // Every new screen takes the focus to its title, so the keyboard and the screen reader start
    // from its top instead of the page's; the hand-in question and its way back place it themselves.
    let mut last_stage = use_signal(|| Stage::Welcome);
    use_effect(move || {
        let now = stage();
        let before = *last_stage.peek();
        if now != before {
            last_stage.set(now);
            if now != Stage::Confirm && before != Stage::Confirm {
                focus_screen();
            }
        }
    });
    use_effect(move || {
        let tag = lang().tag();
        let _ = document::eval(&format!("document.documentElement.lang = '{tag}';"));
    });

    use_future(move || async move {
        dark.set(platform::remembered_dark().await);
        can_share.set(platform::can_share_files().await);
        unfinished.set(unfinished_copy().await);
        let options = platform::link_options().await;
        let tag = if options.lang.is_empty() {
            platform::browser_language().await
        } else {
            options.lang
        };
        asked.set(Lang::from_tag(&tag));
        lang.set(Lang::from_tag(&tag));
        if name.peek().is_empty() {
            name.set(options.name);
        }
        source.set(options.source);
        // A paper given by link (`?paper=...`) opens without choosing a file.
        if let Some(bytes) = platform::package_from_link().await {
            match mcq_crypto::inspect(&bytes) {
                Ok(header) => {
                    loaded.set(Some(Rc::new(Loaded { bytes, header })));
                    shareable.set(true);
                    stage.set(Stage::Identify);
                }
                Err(_) => error.set(Some(lang().texts().not_a_package)),
            }
        }
    });

    crate::icon::use_keep_in_view();
    use_future(move || async move {
        let mut events = document::eval(EVENTS_SCRIPT);
        while let Ok(message) = events.recv::<String>().await {
            if stage() == Stage::Review {
                let key = message.trim_start_matches("k:");
                let action = match key {
                    "ArrowLeft" | "PageUp" => Some(ReviewAction::Previous),
                    "ArrowRight" | "PageDown" | "Enter" => Some(ReviewAction::Next),
                    " " => Some(ReviewAction::Understood),
                    _ => None,
                };
                let graded_now = graded.read().as_ref().map(Rc::clone);
                let session_now = session.read().clone();
                if let (Some(action), Some(g), Some(s)) = (action, graded_now, session_now) {
                    review.write().apply(action, &s, &g.result);
                    keep_review_in_view();
                }
                continue;
            }
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
            if let Some(seconds) = tick_seconds(&message) {
                let before = s.elapsed_seconds;
                s.tick(seconds);
                if let Some(gone) = away() {
                    away.set(Some(gone.saturating_add(seconds)));
                }
                let status = s.time_status(&paper.paper);
                if status.show_pace_alert {
                    notice.set(Some(lang().texts().pace_alert));
                }
                if before / 5 != s.elapsed_seconds / 5 {
                    save(s);
                }
                continue;
            }
            match message.as_str() {
                "k:Escape" if stage() == Stage::Confirm => close_confirm(stage),
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
                        "ArrowRight" | "PageDown" | "Enter" => s.next(),
                        "ArrowLeft" | "PageUp" => s.previous(),
                        "m" | "M" | " " => s.toggle_flag(),
                        other => {
                            // A to Z as printed next to the options, or their numbers.
                            let letter = match other.as_bytes() {
                                [c] if c.is_ascii_alphabetic() => {
                                    Some(usize::from(c.to_ascii_uppercase() - b'A'))
                                }
                                _ => None,
                            };
                            let index = letter.or_else(|| {
                                other.parse::<usize>().ok().and_then(|n| n.checked_sub(1))
                            });
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
    // The switch shows only when there is more than one language to go to.
    let next_lang = {
        let list = choices.read();
        let at = list.iter().position(|l| *l == lang()).unwrap_or(0);
        (list.len() > 1).then(|| list[(at + 1) % list.len()])
    };
    let theme = if dark() { "dark" } else { "light" };
    // The paper's own language, for its text read aloud (WCAG 3.1.2).
    let paper_lang = opened
        .read()
        .as_ref()
        .map(|o| o.paper.lang.clone())
        .filter(|l| !l.is_empty());
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
                                    shareable.set(false);
                                    error.set(None);
                                    stage.set(Stage::Identify);
                                }
                                Err(_) => error.set(Some(lang().texts().not_a_package)),
                            }
                        },
                    }
                }
                p { class: "muted", "{t.open_hint}" }
                if let Some(title) = unfinished() {
                    p { class: "notice", role: "status", {crate::texts::fill(t.in_progress, &[("title", title)])} }
                }
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
                        let (chosen, offered) =
                            interface_for(&paper.paper.lang, &paper.paper.languages, asked());
                        lang.set(chosen);
                        choices.set(offered);
                        opened.set(Some(Rc::new(paper)));
                        error.set(None);
                        spawn(async move {
                            let key = platform::save_key(&paper_id, &fresh.student_name);
                            let mut restored = platform::storage_get(&key)
                                .await
                                .and_then(|text| b64::decode(&text))
                                .and_then(|bytes| decode_session(&bytes).ok())
                                .filter(|s| s.paper_id == paper_id && s.items.len() == fresh.items.len());
                            // The time the page stayed closed counts, and shows on the copy.
                            if let Some(s) = restored.as_mut() {
                                s.reopened_after(platform::seconds_since_stamp(&key).await);
                                save(s);
                            }
                            resumed.set(restored.is_some());
                            let title = paper_title(opened);
                            copy_saved.set(false);
                            erased.set(false);
                            let current = restored.unwrap_or(fresh);
                            let done = current.submitted;
                            if !done {
                                platform::remember_in_progress(&key, &title);
                            }
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
            let (chosen, locked) = session
                .read()
                .as_ref()
                .map_or((TimeExtension::None, true), |s| {
                    (s.extension, s.elapsed_seconds > 0 || s.submitted)
                });
            render::intro(
                &o.paper,
                &o.formulas,
                &o.header,
                t,
                EventHandler::new(move |()| stage.set(Stage::Exam)),
                resumed(),
                render::ExtraTime {
                    chosen,
                    locked,
                    on_change: EventHandler::new(move |extension| {
                        let mut slot = session.write();
                        if let Some(s) = slot.as_mut()
                            && s.set_extension(extension)
                        {
                            save(s);
                        }
                    }),
                },
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
            let gauge = s.pace(&o.paper);
            let total = s.items.len();
            let answered = s.answered_count();
            let unanswered = total - answered;
            let incomplete = s.incomplete_count(&o.paper);
            let complete = answered - incomplete;
            let flagged = s.flagged_count();
            // The incomplete questions by position, to go and finish them from the hand-in question.
            let incomplete_at = s.incomplete_positions(&o.paper);
            let incomplete_word = if incomplete == 1 {
                t.incomplete_left_one
            } else {
                t.incomplete_left
            };
            let flagged_word = if flagged == 1 {
                t.flagged_left_one
            } else {
                t.flagged_left
            };
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
                    EventHandler::new(move |()| {
                        if let Some(s) = session.write().as_mut() {
                            s.toggle_report();
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
                    h1 { class: "sr-only", "{o.paper.title}" }
                    div { class: "status-bar",
                        {render::clock(&status, s.elapsed_seconds, t)}
                        span { class: "progress", "{complete} / {total} {t.answered}" }
                    }
                    {render::segments(&s, &o.paper, gauge.map(|g| g.time_fraction), t, EventHandler::new(move |i: usize| go_to(session, i)))}
                    if let Some(gauge) = gauge { {render::pace_view(&gauge, t)} }
                    // Always in the page, so screen readers announce what appears inside it.
                    div { class: "live", role: "status",
                        if status.overtime_seconds > 0 {
                            p { class: "notice warn", "{t.time_up}" }
                        } else if status.final_warning {
                            p { class: "notice warn", "{t.final_warning}" }
                        }
                        if let Some(message) = notice() {
                            p { class: "notice",
                                "{message} "
                                IconButton { class: "link", icon: Icon::Close, label: t.close, onclick: move |_| notice.set(None) }
                            }
                        }
                    }
                    if stage() == Stage::Confirm {
                        div { class: "confirm card", role: "dialog", "aria-labelledby": "confirm-title",
                            h2 { id: "confirm-title", tabindex: "-1", "{t.confirm_title}" }
                            p {
                                "{unanswered} {t.unanswered_left}, "
                                if incomplete > 0 { strong { "{incomplete} {incomplete_word}" } ", " }
                                "{flagged} {flagged_word}."
                            }
                            if !incomplete_at.is_empty() {
                                p { class: "incomplete-links",
                                    "{t.incomplete_which} "
                                    for i in incomplete_at {
                                        button { key: "{i}", class: "link",
                                            onclick: move |_| { go_to(session, i); stage.set(Stage::Exam); },
                                            "{t.question} {i + 1}"
                                        }
                                        " "
                                    }
                                }
                            }
                            div { class: "nav",
                                button { onclick: move |_| close_confirm(stage), "{t.keep_working}" }
                                button { class: "primary",
                                    onclick: move |_| {
                                        let Some(o) = opened.read().as_ref().map(Rc::clone) else { return };
                                        let copy = match session.write().as_mut() {
                                            Some(s) => {
                                                let copy = s.submit();
                                                save(s);
                                                platform::forget_in_progress();
                                                unfinished.set(None);
                                                copy
                                            }
                                            None => return,
                                        };
                                        if let Ok(result) = mcq_crypto::seal_result(&o.header, &copy, &mut platform::rng()) {
                                            sealed.set(Some(result));
                                        }
                                        // Practice opens on the correction; an exam waits for its end code.
                                        if !(crate::review::correction_on_hand_in(o.header.opening)
                                            && open_correction(KeyUnlock::Practice))
                                        {
                                            stage.set(Stage::Submitted);
                                        }
                                    },
                                    "{t.confirm_submit}"
                                }
                            }
                        }
                    }
                    {question_element}
                    // Fixed to the bottom of the window: the buttons never move with the question's length.
                    div { class: "exam-nav",
                        div { class: "exam-nav-inner",
                            IconButton { disabled: current == 0, icon: Icon::ArrowLeft, label: t.previous, tip: t.previous_tip, shown: true,
                                onclick: move |_| go(Session::previous) }
                            div { class: "exam-nav-middle",
                                if flagged > 0 {
                                    IconButton { class: "jump", icon: Icon::Flag, label: t.flagged_short, tip: t.next_flagged,
                                        onclick: move |_| go(|s| { s.next_flagged(); }) }
                                }
                                button { id: "finish", class: if current + 1 == total { "end primary" } else { "end" },
                                    onclick: move |_| open_confirm(stage), "{t.finish}"
                                }
                            }
                            IconButton { class: "primary next", disabled: current + 1 == total, icon: Icon::ArrowRight, label: t.next, tip: t.next_tip, shown: true,
                                onclick: move |_| go(Session::next) }
                        }
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
            let file_name = copy_file_name(session);
            let mut show = move |()| {
                let typed = end_code();
                let unlock = if practice {
                    KeyUnlock::Practice
                } else {
                    KeyUnlock::EndCode(&typed)
                };
                if open_correction(unlock) {
                    error.set(None);
                } else {
                    error.set(Some(lang().texts().wrong_code));
                }
            };
            let saved_line =
                crate::texts::fill(t.saved_to, &[("file", platform::file_name(&file_name))]);
            // An exam's copy reaches the teacher only as a file: saving it comes first.
            let has_copy = receipt.is_some();
            let ready = practice || !has_copy || copy_saved();
            rsx! {
                section { class: "submitted card",
                    h1 { "{t.submitted_title}" }
                    if let Some(receipt) = receipt.clone() {
                        if !copy_saved() { p { class: "notice warn", "{t.not_sent_yet}" } }
                        button { class: if copy_saved() { "" } else { "primary" },
                            onclick: move |_| {
                                if let Some(r) = sealed.read().as_ref() {
                                    platform::download(&file_name, &b64::encode(&r.bytes));
                                    copy_saved.set(true);
                                }
                            },
                            "{t.save_copy}"
                        }
                        if can_share() {
                            button {
                                onclick: {
                                    let file_name = file_name.clone();
                                    move |_| {
                                        let bytes = sealed.read().as_ref().map(|r| b64::encode(&r.bytes));
                                        let file_name = file_name.clone();
                                        async move {
                                            if let Some(bytes) = bytes
                                                && platform::share_file(&file_name, &bytes).await == platform::Shared::Sent
                                            {
                                                copy_saved.set(true);
                                            }
                                        }
                                    }
                                },
                                "{t.share_copy}"
                            }
                        }
                        if copy_saved() { p { class: "notice", role: "status", "{saved_line}" } }
                        p { "{t.receipt}" }
                        p { class: "big mono", "{receipt}" }
                        p { class: "muted", "{t.receipt_hint}" }
                    }
                    if ready && !erased() {
                        if practice {
                            button { onclick: move |_| show(()), "{t.show_correction}" }
                        } else if has_end_code {
                            form {
                                onsubmit: move |evt: FormEvent| {
                                    evt.prevent_default();
                                    show(());
                                },
                                label { r#for: "end", "{t.end_code}" }
                                input { id: "end", r#type: "text", class: "mono", autocomplete: "off",
                                    placeholder: "{t.code_placeholder}", value: "{end_code}",
                                    oninput: move |e| end_code.set(e.value()),
                                }
                                p { class: "muted", "{t.end_code_hint}" }
                                button { r#type: "submit", "{t.show_correction}" }
                            }
                        }
                    }
                    if has_copy && copy_saved() {
                        if erased() {
                            p { class: "notice", role: "status", "{t.erased}" }
                        } else {
                            button { class: "link",
                                onclick: move |_| {
                                    if let Some(s) = session.read().as_ref() {
                                        platform::storage_remove(&platform::save_key(&s.paper_id, &s.student_name));
                                    }
                                    erased.set(true);
                                },
                                "{t.erase_copy}"
                            }
                            p { class: "muted", "{t.erase_hint}" }
                        }
                    }
                    if let Some(message) = error() { p { class: "error", role: "alert", "{message}" } }
                }
                {render::editor_link(&source.read(), t)}
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
            let state = review.read().clone();
            let on = {
                let g = Rc::clone(&g);
                let s = s.clone();
                EventHandler::new(move |action: ReviewAction| match action {
                    ReviewAction::DownloadSheet => {
                        let sheet = crate::review::revision_markdown(
                            &g.full,
                            &s,
                            &g.result,
                            lang().texts(),
                        );
                        let name =
                            format!("{} - {}.md", s.student_name, lang().texts().sheet_title);
                        platform::download(&name, &b64::encode(sheet.as_bytes()));
                    }
                    other => {
                        review.write().apply(other, &s, &g.result);
                        keep_review_in_view();
                    }
                })
            };
            let actions = crate::review::ResultActions {
                receipt: sealed.read().as_ref().map(|r| r.receipt.clone()),
                can_share: shareable(),
                shared: shared(),
            };
            let act = EventHandler::new(move |action: ResultAction| match action {
                ResultAction::SaveCopy => {
                    if let Some(r) = sealed.read().as_ref() {
                        platform::download(&copy_file_name(session), &b64::encode(&r.bytes));
                        copy_saved.set(true);
                    }
                }
                ResultAction::Share => {
                    spawn(async move {
                        let texts = lang().texts();
                        shared.set(match platform::share_link().await {
                            platform::Shared::Sent => Some(texts.share_sent),
                            platform::Shared::Cancelled => None,
                            platform::Shared::Copied => Some(texts.share_copied),
                            platform::Shared::Failed => Some(texts.share_failed),
                        });
                    });
                }
            });
            rsx! {
                {crate::review::result_actions(&actions, t, act)}
                {crate::review::review_screen(&g.full, &s, &g.result, &state, t, content, on)}
                {render::editor_link(&source.read(), t)}
            }
        }
    };

    rsx! {
        style { {STYLE} }
        div { class: "app", "data-theme": "{theme}", "data-shortcuts": if shortcuts() { "on" } else { "off" },
            style: "font-size: {text_scale}%",
            if stage() == Stage::Exam {
                button { class: "skip-link", onclick: move |_| skip_to_question(), "{t.skip_to_question}" }
            }
            header { class: "top",
                span { class: "brand", "{header_title}" }
                div { class: "prefs",
                    {render::theme_switch(dark(), [t.dark_short, t.light_short], [t.theme_dark, t.theme_light],
                        EventHandler::new(move |pick| { dark.set(pick); platform::remember_dark(pick); }))}
                    IconButton { icon: Icon::TextSmaller, label: t.smaller_short, tip: t.text_smaller,
                        onclick: move |_| text_scale.set(text_scale().saturating_sub(10).max(80)) }
                    IconButton { icon: Icon::TextLarger, label: t.larger_short, tip: t.text_larger,
                        onclick: move |_| text_scale.set((text_scale() + 10).min(160)) }
                    IconButton { icon: Icon::Keyboard, label: t.shortcuts_short, tip: t.shortcuts, aria_pressed: "{shortcuts()}",
                        onclick: move |_| shortcuts.set(!shortcuts()) }
                    if let Some(next) = next_lang {
                        button { "aria-label": "{t.language_name}", title: "{t.language_name}",
                            lang: next.tag(),
                            onclick: move |_| lang.set(next),
                            {next.tag().to_ascii_uppercase()}
                        }
                    }
                }
            }
            main { lang: paper_lang, {content} }
        }
    }
}

/// The paper's title once open, empty before.
fn paper_title(opened: Signal<Option<Rc<Opened>>>) -> String {
    opened
        .read()
        .as_ref()
        .map(|o| o.paper.title.clone())
        .unwrap_or_default()
}

/// The title of the copy noted unfinished on this device, while it is still there and not
/// handed in.
async fn unfinished_copy() -> Option<String> {
    let (key, title) = platform::in_progress().await?;
    let unfinished = platform::storage_get(&key)
        .await
        .and_then(|text| b64::decode(&text))
        .and_then(|bytes| decode_session(&bytes).ok())
        .is_some_and(|s| !s.submitted);
    if !unfinished {
        platform::forget_in_progress();
    }
    unfinished.then_some(title)
}

/// The seconds a `tick:<n>` message of the page carries; `None` for any other message.
pub fn tick_seconds(message: &str) -> Option<u32> {
    message.strip_prefix("tick:")?.parse().ok()
}

/// The sealed copy's file name, after the student.
fn copy_file_name(session: Signal<Option<Session>>) -> String {
    session
        .read()
        .as_ref()
        .map(|s| format!("{}.mcqres", s.student_name))
        .unwrap_or_else(|| "copy.mcqres".to_string())
}

fn go_to(mut session: Signal<Option<Session>>, index: usize) {
    if let Some(s) = session.write().as_mut() {
        s.go_to(index);
        save(s);
    }
}

/// After moving in the correction, brings the question back up if the student had scrolled
/// past its start.
fn keep_review_in_view() {
    let _ = document::eval(
        "setTimeout(() => { const g = document.querySelector('.review-guide');          if (g && g.getBoundingClientRect().top < 0) g.scrollIntoView(); }, 0);",
    );
}

/// Back to the exam, the focus on the button that opened the hand-in question.
fn close_confirm(mut stage: Signal<Stage>) {
    stage.set(Stage::Exam);
    let _ = document::eval(
        "setTimeout(() => { const b = document.getElementById('finish'); if (b) b.focus(); }, 0);",
    );
}

/// After moving to another question, the focus goes to its title when it was lost with the
/// previous question's options, so keyboard and screen reader users start from the top.
fn focus_question() {
    let _ = document::eval(
        "setTimeout(() => { const a = document.activeElement; \
         if (!a || a === document.body) { const h = document.getElementById('q-title'); if (h) h.focus({ preventScroll: true }); } }, 0);",
    );
}

/// The focus on the new screen's title (the question's during the exam), unless a field of the
/// screen took it already (the name, on the identification screen).
fn focus_screen() {
    let _ = document::eval(
        "setTimeout(() => { const a = document.activeElement; \
         if (a && a !== document.body && document.querySelector('main').contains(a)) return; \
         const h = document.getElementById('q-title') || document.getElementById('name') || document.querySelector('main h1, main h2'); \
         if (!h) return; if (h.tagName !== 'INPUT' && !h.hasAttribute('tabindex')) h.setAttribute('tabindex', '-1'); \
         h.focus({ preventScroll: true }); }, 0);",
    );
}

/// Takes the keyboard straight to the question, past the header and the overview.
fn skip_to_question() {
    let _ = document::eval(
        "setTimeout(() => { const h = document.getElementById('q-title'); if (h) h.focus(); }, 0);",
    );
}

/// Shows the hand-in question at the top of the page, with the focus on it.
fn open_confirm(mut stage: Signal<Stage>) {
    stage.set(Stage::Confirm);
    let _ = document::eval(
        "setTimeout(() => { window.scrollTo(0, 0);          const h = document.getElementById('confirm-title'); if (h) h.focus(); }, 0);",
    );
}
