//! The Write space, as on the validated board: the questions by theme with the validator's state,
//! the selected question in the middle (type, statement blocks, answers, advice), the student
//! preview under it, and the Source view on demand.

use dioxus::prelude::*;
use mcq_edit::ops::{self, IdError, Kind};
use mcq_model::{Block, Issue, Level, MediaKind, Question, Severity};

use crate::state::{self, Studio, View};
use crate::texts::Texts;
use crate::{actions, preview};

/// Where the rules are explained, for the validator's links.
pub const RULES_GUIDE: &str = "https://github.com/Chmyke/mcq-advanced/blob/main/docs/lint-rules.md";

fn level_number(level: Level) -> u32 {
    match level {
        Level::Unknown => 0,
        Level::Remember => 1,
        Level::Understand => 2,
        Level::Apply => 3,
        Level::Analyse => 4,
        Level::Evaluate => 5,
    }
}

fn severity_label(severity: Severity, t: &Texts) -> (&'static str, &'static str) {
    match severity {
        Severity::Error => ("error", t.severity_error),
        Severity::Warning => ("warning", t.severity_warning),
        Severity::Info => ("info", t.severity_info),
    }
}

pub fn issue_list(issues: &[&Issue], t: &'static Texts, with_location: bool) -> Element {
    if issues.is_empty() {
        return rsx! { p { class: "ok-note", "{t.no_issue}" } };
    }
    let rows: Vec<(String, &'static str, &'static str, &'static str, String)> = issues
        .iter()
        .map(|issue| {
            let (class, label) = severity_label(issue.severity, t);
            let place = if with_location {
                format!("{}: ", issue.location)
            } else {
                String::new()
            };
            (place, class, label, issue.code, issue.message.clone())
        })
        .collect();
    rsx! {
        ul { class: "issues",
            for (k, (place, class, label, code, message)) in rows.into_iter().enumerate() {
                li { key: "{k}", class: "issue {class}",
                    span { class: "issue-level", "{label}" }
                    span { "{place}{message} " }
                    a { class: "issue-code", href: "{RULES_GUIDE}", target: "_blank", rel: "noopener", "{code}" }
                }
            }
        }
    }
}

pub fn write_space(
    mut studio: Signal<Studio>,
    t: &'static Texts,
    recording: Signal<bool>,
    notice: Signal<Option<String>>,
    phone: Signal<bool>,
    dark: Signal<bool>,
) -> Element {
    let s = studio.read();
    let issues = s.issues();
    let view = s.view;
    rsx! {
        div { class: "write",
            {question_list(studio, &s, &issues, t)}
            div { class: "workspace",
                div { class: "view-switch segmented", role: "group", "aria-label": "{t.view_label}",
                    button { "aria-pressed": "{view == View::Visual}", onclick: move |_| studio.write().show_visual(), "{t.view_visual}" }
                    button { "aria-pressed": "{view == View::Source}", onclick: move |_| studio.write().show_source(), "{t.view_source}" }
                }
                if view == View::Source || !s.can_edit_visually() {
                    {source_view(studio, &s, &issues, t)}
                } else {
                    match s.selected_question() {
                        Some(question) => question_editor(studio, &s, question, &issues, t, recording, notice, phone, dark),
                        None => rsx! { p { class: "card muted", "{t.empty_paper}" } },
                    }
                }
            }
        }
    }
}

/// One line of the question list.
struct ListItem {
    /// Position in the paper, from 0.
    n: usize,
    id: String,
    text: String,
    class: &'static str,
    label: &'static str,
}

fn question_list(
    mut studio: Signal<Studio>,
    s: &Studio,
    issues: &[Issue],
    t: &'static Texts,
) -> Element {
    let paper = &s.project.paper;
    let selected = s.selected.clone().unwrap_or_default();
    let mut groups: Vec<(String, String)> = paper
        .themes
        .iter()
        .map(|th| (th.id.clone(), th.title.clone()))
        .collect();
    if paper.questions.iter().any(|q| q.theme_id.is_empty()) {
        groups.insert(0, (String::new(), "?".to_string()));
    }
    let editable = s.can_edit_visually();
    let groups: Vec<(String, String, Vec<ListItem>)> = groups
        .into_iter()
        .map(|(id, title)| {
            let items = paper
                .questions
                .iter()
                .enumerate()
                .filter(|(_, q)| q.theme_id == id)
                .map(|(n, q)| {
                    let mine = state::issues_for(paper, issues, &q.id);
                    let (class, label) = match state::worst(&mine) {
                        Some(Severity::Error) => ("dot error", t.state_error),
                        Some(_) => ("dot warning", t.state_warning),
                        None => ("dot ok", t.state_ok),
                    };
                    let text = match state::summary(q, 70) {
                        s if s.is_empty() => t.no_statement.to_string(),
                        s => s,
                    };
                    ListItem {
                        n,
                        id: q.id.clone(),
                        text,
                        class,
                        label,
                    }
                })
                .collect();
            (id, title, items)
        })
        .collect();
    let over = s.drag.as_ref().and_then(|d| d.over.clone());
    rsx! {
        nav { class: "question-list", "aria-label": "{t.questions}",
            for (theme_id, title, items) in groups {
                section {
                    key: "{theme_id}",
                    class: if over.as_deref() == Some(theme_id.as_str()) { "theme-group drop-target" } else { "theme-group" },
                    // A question dropped on the group goes to the end of its theme; the
                    // questions themselves catch the drops that land on them.
                    ondragover: {
                        let theme_id = theme_id.clone();
                        move |e: DragEvent| {
                            if editable && !theme_id.is_empty() && studio.peek().drag.is_some() {
                                e.prevent_default();
                                studio.write().drag_over(&theme_id);
                            }
                        }
                    },
                    ondrop: {
                        let theme_id = theme_id.clone();
                        move |e: DragEvent| {
                            e.prevent_default();
                            studio.write().drop_question(&theme_id, None);
                        }
                    },
                    div { class: "theme-head",
                        h2 { "{title}" }
                        if editable && !theme_id.is_empty() {
                            button {
                                class: "small",
                                onclick: {
                                    let theme_id = theme_id.clone();
                                    move |_| studio.write().add_question(&theme_id)
                                },
                                "{t.add_question}"
                            }
                        }
                    }
                    ol {
                        for ListItem { n, id, text, class, label } in items {
                            li {
                                key: "{id}",
                                ondrop: {
                                    let (theme_id, id) = (theme_id.clone(), id.clone());
                                    move |e: DragEvent| {
                                        e.prevent_default();
                                        e.stop_propagation();
                                        studio.write().drop_question(&theme_id, Some(&id));
                                    }
                                },
                                button {
                                    class: if id == selected { "q-item current" } else { "q-item" },
                                    "aria-current": if id == selected { "true" } else { "false" },
                                    draggable: if editable { "true" } else { "false" },
                                    title: if editable { t.drag_question } else { "" },
                                    ondragstart: {
                                        let id = id.clone();
                                        move |e: DragEvent| {
                                            // Firefox only starts a drag that carries data; a
                                            // private type keeps text fields from taking it.
                                            let data = e.data_transfer();
                                            let _ = data.set_data("application/x-mcq-question", &id);
                                            data.set_effect_allowed("move");
                                            studio.write().start_drag(&id);
                                        }
                                    },
                                    ondragend: move |_| studio.write().end_drag(),
                                    onclick: {
                                        let id = id.clone();
                                        move |_| studio.write().select(&id)
                                    },
                                    span { class: "q-num", "{n + 1}" }
                                    span { class: "q-text", "{text}" }
                                    span { class: "{class}", title: "{label}",
                                        span { class: "sr-only", "{label}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn source_view(
    mut studio: Signal<Studio>,
    s: &Studio,
    issues: &[Issue],
    t: &'static Texts,
) -> Element {
    let all: Vec<&Issue> = issues.iter().collect();
    let text = s.source.clone();
    rsx! {
        div { class: "source card",
            if !s.can_edit_visually() { p { class: "notice warn", "{t.source_broken}" } }
            label { r#for: "source-text", "{t.source_label}" }
            textarea {
                id: "source-text",
                class: "mono source-text",
                spellcheck: "false",
                value: "{text}",
                oninput: move |e| studio.write().edit_source(e.value()),
            }
            h2 { "{t.validator}" }
            {issue_list(&all, t, true)}
        }
    }
}

#[allow(clippy::too_many_arguments)] // WHY: one screen of one question; grouping would only rename.
fn question_editor(
    mut studio: Signal<Studio>,
    s: &Studio,
    question: &Question,
    issues: &[Issue],
    t: &'static Texts,
    recording: Signal<bool>,
    notice: Signal<Option<String>>,
    phone: Signal<bool>,
    dark: Signal<bool>,
) -> Element {
    let paper = &s.project.paper;
    let id = question.id.clone();
    let index = paper.questions.iter().position(|q| q.id == id).unwrap_or(0);
    let kind = s.kind(question);
    let mine = state::issues_for(paper, issues, &id);
    let themes: Vec<(String, String)> = paper
        .themes
        .iter()
        .map(|th| (th.id.clone(), th.title.clone()))
        .collect();
    let level = level_number(question.level);
    let seconds = if question.estimated_seconds > 0 {
        question.estimated_seconds.to_string()
    } else {
        String::new()
    };
    let kinds = [
        (Kind::Single, t.kind_single),
        (Kind::Multiple, t.kind_multiple),
        (Kind::TrueFalse, t.kind_true_false),
    ];
    let preview = preview::question_preview(s, question, index, t, phone, dark);
    rsx! {
        article { class: "editor card", "aria-labelledby": "editor-title",
            header { class: "editor-head",
                h1 { id: "editor-title", "{t.question} {index + 1}" }
                div { class: "editor-actions",
                    button { class: "small", onclick: { let id = id.clone(); move |_| { studio.write().edit_paper(|p| { ops::move_question(p, &id, true); }); } }, "{t.move_up}" }
                    button { class: "small", onclick: { let id = id.clone(); move |_| { studio.write().edit_paper(|p| { ops::move_question(p, &id, false); }); } }, "{t.move_down}" }
                    button { class: "small", onclick: { let id = id.clone(); move |_| studio.write().duplicate_question(&id) }, "{t.duplicate}" }
                    button { class: "small danger", onclick: { let id = id.clone(); move |_| studio.write().remove_question(&id) }, "{t.delete}" }
                }
            }
            div { class: "fields",
                IdField { key: "{id}", studio, id: id.clone(), t_id: t.id, t_empty: t.id_empty, t_bad: t.id_bad, t_taken: t.id_taken }
                div { class: "field",
                    label { r#for: "q-theme", "{t.theme}" }
                    select {
                        id: "q-theme",
                        onchange: { let id = id.clone(); move |e: FormEvent| { studio.write().edit_paper(|p| { ops::set_question_theme(p, &id, &e.value()); }); } },
                        for (theme_id, title) in themes {
                            option { key: "{theme_id}", value: "{theme_id}", selected: theme_id == question.theme_id, "{title}" }
                        }
                    }
                }
                div { class: "field",
                    label { r#for: "q-level", "{t.level}" }
                    select {
                        id: "q-level",
                        onchange: { let id = id.clone(); move |e: FormEvent| {
                            let level = e.value().parse().ok().and_then(Level::from_number).unwrap_or_default();
                            studio.write().edit_question(&id, |q| q.level = level);
                        } },
                        for (n, label) in t.levels.iter().enumerate() {
                            option { key: "{n}", value: "{n}", selected: n as u32 == level, "{label}" }
                        }
                    }
                }
                div { class: "field narrow",
                    label { r#for: "q-seconds", "{t.seconds}" }
                    input {
                        id: "q-seconds", r#type: "number", min: "0", placeholder: "{t.seconds_auto}", value: "{seconds}",
                        oninput: { let id = id.clone(); move |e: FormEvent| {
                            let seconds = e.value().trim().parse().unwrap_or(0);
                            studio.write().edit_question(&id, |q| q.estimated_seconds = seconds);
                        } },
                    }
                }
            }
            div { class: "segmented kinds", role: "group", "aria-label": "{t.kind_label}",
                for (k, label) in kinds {
                    button {
                        key: "{label}",
                        "aria-pressed": "{kind == k}",
                        onclick: { let id = id.clone(); move |_| studio.write().set_kind(&id, k) },
                        "{label}"
                    }
                }
            }

            section { class: "statement", "aria-labelledby": "statement-title",
                div { class: "statement-bar",
                    h2 { id: "statement-title", "{t.statement}" }
                    div { class: "toolbar",
                        button { class: "small", onclick: { let id = id.clone(); move |_| studio.write().add_text_block(&id) }, "{t.add_text}" }
                        button { class: "small", onclick: { let id = id.clone(); move |_| {
                            let lang = studio.read().project.paper.questions.iter().flat_map(|q| q.code_blocks()).last().map(|c| c.language.clone()).unwrap_or_default();
                            studio.write().add_code_block(&id, &lang);
                        } }, "{t.add_code}" }
                        button { class: "small", onclick: { let id = id.clone(); move |_| studio.write().add_formula_block(&id) }, "{t.add_formula}" }
                        label { class: "file small",
                            "{t.add_media}"
                            input {
                                r#type: "file", class: "sr-only", multiple: true,
                                accept: "image/*,audio/*,video/*",
                                onchange: { let id = id.clone(); move |e: FormEvent| {
                                    let id = id.clone();
                                    async move {
                                        for file in e.files() {
                                            if let Ok(bytes) = file.read_bytes().await {
                                                actions::add_media(studio, notice, t, &id, &file.name(), bytes.to_vec());
                                            }
                                        }
                                    }
                                } },
                            }
                        }
                        if recording() {
                            button { class: "small recording", onclick: move |_| actions::stop_recording(), "{t.stop_recording}" }
                        } else {
                            button { class: "small", onclick: { let id = id.clone(); move |_| actions::start_recording(studio, recording, notice, t, id.clone()) }, "{t.record}" }
                        }
                    }
                }
                p { class: "muted hint", "{t.drop_hint}" }
                p { class: "muted hint", "{t.formula_hint}" }
                for (k, block) in question.body.iter().enumerate() {
                    div { key: "{k}", class: "block",
                        {block_editor(studio, s, &id, k, block, t)}
                        div { class: "block-actions",
                            button { class: "icon", title: "{t.block_up}", "aria-label": "{t.block_up}", onclick: { let id = id.clone(); move |_| studio.write().edit_question(&id, |q| { ops::move_item(&mut q.body, k, true); }) }, "↑" }
                            button { class: "icon", title: "{t.block_down}", "aria-label": "{t.block_down}", onclick: { let id = id.clone(); move |_| studio.write().edit_question(&id, |q| { ops::move_item(&mut q.body, k, false); }) }, "↓" }
                            button { class: "icon", title: "{t.block_remove}", "aria-label": "{t.block_remove}", onclick: { let id = id.clone(); move |_| studio.write().remove_block(&id, k) }, "✕" }
                        }
                    }
                }
            }

            section { class: "answers", "aria-labelledby": "answers-title",
                h2 { id: "answers-title", "{t.answers}" }
                p { class: "muted hint", "{t.answers_hint}" }
                for (k, option) in question.options.iter().enumerate() {
                    div { key: "{k}", class: if option.is_correct { "answer correct" } else { "answer" },
                        input {
                            r#type: if kind == Kind::Multiple { "checkbox" } else { "radio" },
                            name: "correct-{id}",
                            checked: option.is_correct,
                            "aria-label": "{t.correct} {k + 1}",
                            onclick: { let id = id.clone(); move |_| studio.write().toggle_correct(&id, k) },
                        }
                        div { class: "answer-texts",
                            input {
                                r#type: "text", class: "answer-text",
                                "aria-label": "{t.answer_placeholder} {k + 1}",
                                placeholder: "{t.answer_placeholder} {k + 1}",
                                value: "{option.text}",
                                oninput: { let id = id.clone(); move |e: FormEvent| studio.write().edit_question(&id, |q| q.options[k].text = e.value()) },
                            }
                            textarea {
                                class: "explanation-text", rows: "1",
                                "aria-label": "{t.explanation_placeholder} ({k + 1})",
                                placeholder: "{t.explanation_placeholder}",
                                value: "{option.explanation}",
                                oninput: { let id = id.clone(); move |e: FormEvent| studio.write().edit_question(&id, |q| q.options[k].explanation = e.value()) },
                            }
                        }
                        div { class: "block-actions",
                            button { class: "icon", title: "{t.move_up}", "aria-label": "{t.move_up}", onclick: { let id = id.clone(); move |_| studio.write().edit_question(&id, |q| { ops::move_option(q, k, true); }) }, "↑" }
                            button { class: "icon", title: "{t.move_down}", "aria-label": "{t.move_down}", onclick: { let id = id.clone(); move |_| studio.write().edit_question(&id, |q| { ops::move_option(q, k, false); }) }, "↓" }
                            button { class: "icon", title: "{t.remove_answer}", "aria-label": "{t.remove_answer}", onclick: { let id = id.clone(); move |_| studio.write().edit_question(&id, |q| ops::remove_option(q, k)) }, "✕" }
                        }
                    }
                }
                if kind != Kind::TrueFalse {
                    button { class: "small", onclick: { let id = id.clone(); move |_| studio.write().edit_question(&id, ops::add_option) }, "{t.add_answer}" }
                }
            }

            div { class: "fields wide",
                div { class: "field",
                    label { r#for: "q-rule", "{t.rule}" }
                    input { id: "q-rule", r#type: "text", value: "{question.rule}",
                        oninput: { let id = id.clone(); move |e: FormEvent| studio.write().edit_question(&id, |q| q.rule = e.value()) } }
                }
                div { class: "field",
                    label { r#for: "q-review", "{t.review}" }
                    input { id: "q-review", r#type: "text", value: "{question.course_ref}",
                        oninput: { let id = id.clone(); move |e: FormEvent| studio.write().edit_question(&id, |q| q.course_ref = e.value()) } }
                }
            }

            section { class: "advice", "aria-labelledby": "advice-title",
                h2 { id: "advice-title", "{t.validator}" }
                {issue_list(&mine, t, false)}
            }
        }
        {preview}
    }
}

fn block_editor(
    mut studio: Signal<Studio>,
    s: &Studio,
    id: &str,
    k: usize,
    block: &Block,
    t: &'static Texts,
) -> Element {
    let id = id.to_string();
    match block {
        Block::Text(text) => rsx! {
            div { class: "block-body",
                label { r#for: "b-{k}", class: "block-label", "{t.text_block}" }
                textarea {
                    id: "b-{k}", rows: "3", value: "{text}",
                    oninput: move |e| studio.write().edit_question(&id, |q| if let Some(Block::Text(text)) = q.body.get_mut(k) { *text = e.value() }),
                }
            }
        },
        Block::Code(code) => {
            let lines: Vec<String> = code.highlighted_lines.iter().map(u32::to_string).collect();
            let lines = lines.join(",");
            let rows = code.source.lines().count().max(2) + 1;
            let (id_a, id_b) = (id.clone(), id.clone());
            rsx! {
                div { class: "block-body",
                    span { class: "block-label", "{t.code_block}" }
                    div { class: "code-meta",
                        label { "{t.language}"
                            input { r#type: "text", class: "mono", value: "{code.language}", placeholder: "cs, cpp, python...",
                                oninput: move |e| studio.write().edit_question(&id_a, |q| if let Some(Block::Code(c)) = q.body.get_mut(k) { c.language = e.value().trim().to_string() }) }
                        }
                        label { "{t.highlighted_lines}"
                            input { r#type: "text", class: "mono", value: "{lines}", placeholder: "3,5-6",
                                onchange: move |e| studio.write().edit_question(&id_b, |q| if let Some(Block::Code(c)) = q.body.get_mut(k) { c.highlighted_lines = mcq_format::line_ranges(&e.value()) }) }
                        }
                    }
                    textarea {
                        class: "mono code-text", rows: "{rows}", spellcheck: "false", value: "{code.source}",
                        "aria-label": "{t.code_block}",
                        oninput: move |e| studio.write().edit_question(&id, |q| if let Some(Block::Code(c)) = q.body.get_mut(k) { c.source = e.value().replace('\t', "    ").trim_end_matches('\n').to_string() }),
                    }
                }
            }
        }
        Block::Media(media) => {
            let src = s.media_urls().get(&media.path).cloned().unwrap_or_default();
            let path = media.path.clone();
            let alt = media.alt.clone();
            rsx! {
                div { class: "block-body media-block",
                    match media.kind {
                        MediaKind::Image => rsx! { img { class: "thumb", src: "{src}", alt: "{alt}" } },
                        MediaKind::Audio => rsx! { audio { controls: true, src: "{src}" } },
                        MediaKind::Video => rsx! { video { class: "thumb", controls: true, src: "{src}" } },
                    }
                    div { class: "media-meta",
                        span { class: "mono muted", "{path}" }
                        label { r#for: "alt-{k}", "{t.alt_text}" }
                        input { id: "alt-{k}", r#type: "text", value: "{alt}", placeholder: "{t.alt_hint}",
                            oninput: move |e| studio.write().edit_question(&id, |q| if let Some(Block::Media(m)) = q.body.get_mut(k) { m.alt = e.value() }) }
                    }
                }
            }
        }
    }
}

/// The question id, renamed when the field is left, with the reason when it cannot be.
#[component]
fn IdField(
    studio: Signal<Studio>,
    id: String,
    t_id: &'static str,
    t_empty: &'static str,
    t_bad: &'static str,
    t_taken: &'static str,
) -> Element {
    let mut error = use_signal(|| None::<&'static str>);
    let current = id.clone();
    rsx! {
        div { class: "field",
            label { r#for: "q-id", "{t_id}" }
            input {
                id: "q-id", r#type: "text", class: "mono", value: "{id}",
                "aria-invalid": if error().is_some() { "true" } else { "false" },
                "aria-describedby": error().is_some().then_some("q-id-error"),
                onchange: move |e: FormEvent| {
                    let wanted = e.value().trim().to_string();
                    match studio.write().rename_question(&current, &wanted) {
                        Ok(()) => error.set(None),
                        Err(IdError::Empty) => error.set(Some(t_empty)),
                        Err(IdError::BadCharacter) => error.set(Some(t_bad)),
                        Err(IdError::Taken) => error.set(Some(t_taken)),
                    }
                },
            }
            if let Some(message) = error() { p { id: "q-id-error", class: "error small-text", role: "alert", "{message}" } }
        }
    }
}
