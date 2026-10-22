//! The screens' content, as functions of plain data: no state, no platform, so tests render
//! them to HTML and check what a student would see.

use std::collections::BTreeMap;

use dioxus::prelude::*;
use mcq_crypto::{OpeningKind, PackageHeader};
use mcq_engine::{Session, SessionItem, TimeStatus, format_duration};
use mcq_grade::{GradeResult, GuessAdvice, Mastery, Status};
use mcq_model::{Block, CodeBlock, CodeTokens, MediaKind, Paper, Question, Scoring};

use mcq_format::binary::{CODE_TOKENS_FILE, decode_code_tokens};
use mcq_math::{FORMULAS_FILE, Formulas};

use crate::texts::Texts;
use crate::{b64, markdown};

/// A paper opened with its code: the student copy, its media, its coloured code and formulas.
pub struct Opened {
    pub header: PackageHeader,
    pub paper: Paper,
    /// Media path to `data:` URL.
    pub media: BTreeMap<String, String>,
    pub tokens: Vec<CodeTokens>,
    pub formulas: Formulas,
}

impl Opened {
    /// Opens a package with the code or phrase the student typed.
    pub fn from_package(bytes: &[u8], secret: &str) -> Result<Self, mcq_crypto::Error> {
        let student = mcq_crypto::open(bytes, secret)?;
        let mut media = BTreeMap::new();
        let mut tokens = Vec::new();
        let mut formulas = Formulas::default();
        for entry in student.media {
            if entry.path == CODE_TOKENS_FILE {
                tokens = decode_code_tokens(&entry.data).unwrap_or_default();
            } else if entry.path == FORMULAS_FILE {
                formulas =
                    Formulas::from_package(mcq_math::decode(&entry.data).unwrap_or_default());
            } else {
                let url = format!(
                    "data:{};base64,{}",
                    mime(&entry.path),
                    b64::encode(&entry.data)
                );
                media.insert(entry.path, url);
            }
        }
        Ok(Opened {
            header: student.header,
            paper: student.paper,
            media,
            tokens,
            formulas,
        })
    }

    pub fn content(&self) -> Content<'_> {
        Content {
            paper: &self.paper,
            media: &self.media,
            tokens: &self.tokens,
            formulas: &self.formulas,
        }
    }
}

/// The formulas of the correction: the student's, and those of the answer key's files.
pub fn correction_formulas(opened: &Formulas, key_files: &[mcq_format::binary::Entry]) -> Formulas {
    let mut all = opened.clone();
    for entry in key_files.iter().filter(|e| e.path == FORMULAS_FILE) {
        all.add(mcq_math::decode(&entry.data).unwrap_or_default());
    }
    all
}

/// What the question views draw from: a paper, its media as `data:` URLs, its coloured code and
/// its formulas. The player takes it from an opened package; the Studio's preview from the paper
/// being edited.
#[derive(Clone, Copy)]
pub struct Content<'a> {
    pub paper: &'a Paper,
    pub media: &'a BTreeMap<String, String>,
    pub tokens: &'a [CodeTokens],
    pub formulas: &'a Formulas,
}

impl Content<'_> {
    fn tokens(&self, question_id: &str, block: usize) -> Option<&CodeTokens> {
        self.tokens
            .iter()
            .find(|t| t.question_id == question_id && t.block as usize == block)
    }
}

/// `image/png` and the like, from the file extension.
pub fn mime(path: &str) -> &'static str {
    let ext = path
        .rsplit_once('.')
        .map_or("", |(_, e)| e)
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        "mp3" => "audio/mpeg",
        "ogg" | "oga" | "opus" => "audio/ogg",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "flac" => "audio/flac",
        "weba" => "audio/webm",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mov" => "video/quicktime",
        _ => "application/octet-stream",
    }
}

pub fn code_block(code: &CodeBlock, tokens: Option<&CodeTokens>) -> Element {
    let lines: Vec<Vec<(String, String)>> = match tokens {
        Some(t) => t
            .lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|r| (format!("tok-{}", r.kind.name()), r.text.clone()))
                    .collect()
            })
            .collect(),
        None => code
            .source
            .split('\n')
            .map(|l| vec![("tok-plain".to_string(), l.to_string())])
            .collect(),
    };
    let language = if code.language.is_empty() {
        "code".to_string()
    } else {
        code.language.clone()
    };
    rsx! {
        figure { class: "code",
            figcaption { class: "code-lang", "{language}" }
            pre {
                code {
                    for (n, line) in lines.into_iter().enumerate() {
                        span {
                            key: "{n}",
                            class: if code.highlighted_lines.contains(&(n as u32 + 1)) { "line hl" } else { "line" },
                            span { class: "ln", "aria-hidden": "true", "{n + 1}" }
                            for (k, (class, text)) in line.into_iter().enumerate() {
                                span { key: "{k}", class: "{class}", "{text}" }
                            }
                            "\n"
                        }
                    }
                }
            }
        }
    }
}

pub fn body(question: &Question, content: Content<'_>) -> Element {
    let mut code_index = 0;
    let blocks: Vec<Element> = question
        .body
        .iter()
        .map(|block| match block {
            Block::Text(text) => {
                let html = markdown::block(text, content.formulas);
                rsx! { div { class: "prose", dangerous_inner_html: "{html}" } }
            }
            Block::Code(code) => {
                let element = code_block(code, content.tokens(&question.id, code_index));
                code_index += 1;
                element
            }
            Block::Media(media) => {
                let src = content.media.get(&media.path).cloned().unwrap_or_default();
                let alt = media.alt.clone();
                match media.kind {
                    MediaKind::Image => rsx! { img { class: "media", src: "{src}", alt: "{alt}" } },
                    MediaKind::Audio => rsx! {
                        figure { class: "media",
                            audio { controls: true, src: "{src}", "aria-label": "{alt}" }
                            if !alt.is_empty() { figcaption { "{alt}" } }
                        }
                    },
                    MediaKind::Video => rsx! {
                        figure { class: "media",
                            video { controls: true, src: "{src}", "aria-label": "{alt}" }
                            if !alt.is_empty() { figcaption { "{alt}" } }
                        }
                    },
                }
            }
        })
        .collect();
    rsx! {
        for (i, element) in blocks.into_iter().enumerate() {
            div { key: "{i}", {element} }
        }
    }
}

#[allow(clippy::too_many_arguments)] // WHY: a flat view of one question; grouping would only rename.
pub fn question_view(
    question: &Question,
    item: &SessionItem,
    index: usize,
    total: usize,
    content: Content<'_>,
    t: &'static Texts,
    on_toggle: EventHandler<String>,
    on_flag: EventHandler<()>,
) -> Element {
    let multiple = question.is_multiple();
    let count = question.answers_to_tick();
    let badge = if multiple {
        if content.paper.show_answer_count {
            format!("{} · {count} {}", t.several_answers, t.tick)
        } else {
            t.several_answers.to_string()
        }
    } else {
        t.one_answer.to_string()
    };
    let theme = content
        .paper
        .theme(&question.theme_id)
        .map(|th| th.title.clone())
        .unwrap_or_default();
    let input_type = if multiple { "checkbox" } else { "radio" };
    let group = format!("q-{index}");
    let options: Vec<(usize, String, String, bool)> = item
        .option_order
        .iter()
        .enumerate()
        .filter_map(|(k, id)| {
            let option = question.option(id)?;
            Some((
                k,
                id.clone(),
                markdown::inline(&option.text, content.formulas),
                item.selected.contains(id),
            ))
        })
        .collect();
    rsx! {
        article { class: "question", "aria-labelledby": "q-title",
            header { class: "question-head",
                h2 { id: "q-title", "{t.question} {index + 1} {t.of} {total}" }
                if !theme.is_empty() { span { class: "theme", "{theme}" } }
                span { class: if multiple { "badge multi" } else { "badge" }, "{badge}" }
            }
            {body(question, content)}
            fieldset { class: "options",
                legend { class: "sr-only", "{badge}" }
                for (k, id, html, checked) in options {
                    label { key: "{id}", class: if checked { "option checked" } else { "option" },
                        input {
                            r#type: "{input_type}",
                            name: "{group}",
                            checked,
                            onchange: {
                                let id = id.clone();
                                move |_| on_toggle.call(id.clone())
                            },
                        }
                        span { class: "key", "aria-hidden": "true", "{k + 1}" }
                        span { class: "option-text", dangerous_inner_html: "{html}" }
                    }
                }
            }
            button {
                class: if item.flagged { "flag on" } else { "flag" },
                "aria-pressed": if item.flagged { "true" } else { "false" },
                onclick: move |_| on_flag.call(()),
                if item.flagged { "{t.unflag}" } else { "{t.flag}" }
            }
        }
    }
}

pub fn grid(session: &Session, t: &'static Texts, on_go: EventHandler<usize>) -> Element {
    let cells: Vec<(usize, &'static str)> = session
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let class = match (
                i == session.current,
                item.flagged,
                !item.selected.is_empty(),
            ) {
                (true, _, _) => "cell current",
                (_, true, _) => "cell flagged",
                (_, _, true) => "cell answered",
                _ => "cell",
            };
            (i, class)
        })
        .collect();
    rsx! {
        nav { class: "grid", "aria-label": "{t.overview}",
            for (i, class) in cells {
                button {
                    key: "{i}",
                    class: "{class}",
                    "aria-current": if i == session.current { "step" } else { "false" },
                    onclick: move |_| on_go.call(i),
                    "{i + 1}"
                }
            }
        }
    }
}

pub fn clock(status: &TimeStatus, elapsed: u32, t: &'static Texts) -> Element {
    let (label, value, class) = if status.limit_seconds == 0 {
        (t.elapsed, format_duration(elapsed), "clock")
    } else if status.overtime_seconds > 0 {
        (
            t.overtime,
            format_duration(status.overtime_seconds),
            "clock over",
        )
    } else if status.final_warning {
        (
            t.time_left,
            format_duration(status.remaining_seconds),
            "clock warn",
        )
    } else {
        (
            t.time_left,
            format_duration(status.remaining_seconds),
            "clock",
        )
    };
    rsx! {
        div { class: "{class}", role: "timer", "aria-live": "off",
            span { class: "clock-label", "{label}" }
            span { class: "clock-value", "{value}" }
        }
    }
}

/// Up to three decimals, without trailing zeros, with the language's decimal separator.
pub fn points(value: f32, t: &Texts) -> String {
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    text.replace('.', t.decimal)
}

pub fn intro(
    paper: &Paper,
    formulas: &Formulas,
    header: &PackageHeader,
    t: &'static Texts,
    on_start: EventHandler<()>,
    resumed: bool,
) -> Element {
    let n = paper.questions.len();
    let duration = if paper.duration_minutes == 0 {
        t.no_limit.to_string()
    } else {
        format!("{} {}", paper.duration_minutes, t.minutes)
    };
    let scale = 20.0 / n.max(1) as f32;
    let scoring = match paper.scoring {
        Scoring::AllOrNothing => t.scoring_all_or_nothing,
        Scoring::Partial => t.scoring_partial,
    };
    let costs = if paper.penalty == 0.0 && paper.unanswered_penalty == 0.0 {
        t.no_penalty.to_string()
    } else {
        format!(
            "{} {} / 20. {} {} / 20.",
            t.wrong_costs,
            points(paper.penalty * scale, t),
            t.blank_costs,
            points(paper.unanswered_penalty * scale, t)
        )
    };
    let advice = match mcq_grade::guess_advice(paper) {
        GuessAdvice::GuessingIsFree => t.advice_free,
        GuessAdvice::AnswerEverything => t.advice_everything,
        GuessAdvice::AnswerWhenYouHaveAnIdea => t.advice_with_idea,
        GuessAdvice::TickOnlyWhenYouHaveAnIdea => t.advice_tick_only,
    };
    let description = markdown::block(&paper.description, formulas);
    rsx! {
        section { class: "intro card",
            h1 { "{paper.title}" }
            div { class: "prose", dangerous_inner_html: "{description}" }
            if header.opening == OpeningKind::Open { p { class: "notice", "{t.practice_notice}" } }
            if resumed { p { class: "notice", "{t.resumed}" } }
            dl { class: "facts",
                dt { "{t.questions}" } dd { "{n}" }
                dt { "{t.duration}" } dd { "{duration}" }
                dt { "{t.scoring}" } dd { "{scoring} {costs}" }
            }
            p { "{advice}" }
            p { class: "muted", "{t.time_never_blocks}" }
            p { class: "muted", "{t.focus_notice}" }
            p { class: "muted", "{t.keyboard_hint}" }
            button { class: "primary", autofocus: true, onclick: move |_| on_start.call(()),
                if resumed { "{t.resume}" } else { "{t.start}" }
            }
        }
    }
}

pub fn review_view(
    full: &Paper,
    session: &Session,
    result: &GradeResult,
    t: &'static Texts,
    content: Content<'_>,
) -> Element {
    let mark = format!("{:.2}", result.mark(20.0)).replace('.', t.decimal);
    let themes: Vec<(String, u32, &'static str)> = result
        .themes
        .iter()
        .filter(|th| th.total > 0)
        .map(|th| {
            let title = full
                .theme(&th.theme_id)
                .map_or(th.theme_id.clone(), |x| x.title.clone());
            let label = match th.mastery() {
                Mastery::Acquired => t.acquired,
                Mastery::Fragile => t.fragile,
                Mastery::ToRelearn | Mastery::NotAssessed => t.to_relearn,
            };
            (title, (th.rate() * 100.0).round() as u32, label)
        })
        .collect();
    let items: Vec<Element> = session
        .items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            let question = full.question(&item.question_id)?;
            let outcome = result.outcomes.iter().find(|o| o.question_id == item.question_id)?;
            let (status, class) = match outcome.status {
                Status::Correct => (t.correct, "ok"),
                Status::Partial => (t.partial, "part"),
                Status::Wrong => (t.wrong, "ko"),
                Status::Unanswered => (t.unanswered, "blank"),
            };
            let options: Vec<(String, String, String, bool, bool)> = item
                .option_order
                .iter()
                .filter_map(|id| {
                    let o = question.option(id)?;
                    Some((
                        id.clone(),
                        markdown::inline(&o.text, content.formulas),
                        markdown::block(&o.explanation, content.formulas),
                        o.is_correct,
                        item.selected.contains(id),
                    ))
                })
                .collect();
            let rule = question.rule.clone();
            let course = question.course_ref.clone();
            Some(rsx! {
                article { class: "review-item {class}",
                    h3 { "{t.question} {i + 1}" span { class: "status", "{status}" } }
                    {body(question, content)}
                    ul { class: "review-options",
                        for (id, text, explanation, correct, chosen) in options {
                            li { key: "{id}", class: if correct { "right" } else { "" },
                                span { class: "mark", if correct { "✓" } else { "·" } }
                                span { dangerous_inner_html: "{text}" }
                                if chosen { em { class: "chosen", " ({t.your_answer})" } }
                                if !explanation.is_empty() { div { class: "explanation", dangerous_inner_html: "{explanation}" } }
                            }
                        }
                    }
                    if !rule.is_empty() { p { class: "rule", strong { "{t.rule}" } " {rule}" } }
                    if !course.is_empty() { p { class: "muted", strong { "{t.review}" } " {course}" } }
                }
            })
        })
        .collect();
    rsx! {
        section { class: "review",
            div { class: "card score",
                h2 { "{t.score}" }
                p { class: "big", "{mark} / 20" }
                h3 { "{t.mastery}" }
                ul { class: "themes",
                    for (k, (title, rate, label)) in themes.into_iter().enumerate() {
                        li { key: "{k}", strong { "{title}" } " {rate} % ({label})" }
                    }
                }
            }
            for (k, element) in items.into_iter().enumerate() {
                div { key: "{k}", {element} }
            }
        }
    }
}
