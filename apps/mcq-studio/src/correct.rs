//! The Correct space: the package and the copies go in, the class's diagnosis comes out, in the
//! order a teacher acts on it (what to reteach, shared confusions, questions to review, copies to
//! look at), the marks last; revision sheets to print and spreadsheets to export.

use std::rc::Rc;

use dioxus::prelude::*;
use mcq_analysis::sheet::RevisionSheet;
use mcq_analysis::timeline::{Change, Event, Timeline};
use mcq_analysis::{Adjustment, Anomaly, Decision, Finding, Report, csv};
use mcq_crypto::Identity;
use mcq_grade::Mastery;
use mcq_math::Formulas;
use mcq_model::{Block, Paper, TimeExtension};
use mcq_player::{b64, markdown, render};

use crate::correction::{Correction, Refusal};
use crate::identity::IdentityCard;
use crate::platform;
use crate::preview;
use crate::publish::Done;
use crate::texts::{CorrectTexts, Lang, Texts, fill};

fn pct(value: f64) -> String {
    format!("{:.0}", value * 100.0)
}

fn decimal(value: f64, digits: usize, lang: Lang) -> String {
    let text = format!("{value:.digits$}");
    if lang == Lang::Fr {
        text.replace('.', ",")
    } else {
        text
    }
}

fn minutes(seconds: u32) -> String {
    (seconds / 60).to_string()
}

fn option_text(paper: &Paper, question: &str, option: &str) -> String {
    paper
        .question(question)
        .and_then(|q| q.option(option))
        .map(|o| o.text.clone())
        .unwrap_or_else(|| option.to_string())
}

/// The formulas of every option of the paper, for the sentences that quote them.
fn option_formulas(paper: &Paper) -> Formulas {
    let texts: Vec<&str> = paper
        .questions
        .iter()
        .flat_map(|q| q.options.iter().map(|o| o.text.as_str()))
        .collect();
    preview::formulas(&texts)
}

/// An option as the student saw it: inline Markdown, code and formulas, as safe HTML.
fn quoted(paper: &Paper, formulas: &Formulas, question: &str, option: &str) -> String {
    markdown::inline(&option_text(paper, question, option), formulas)
}

/// A piece of a report sentence: words as text, or a quoted option already rendered.
#[derive(Debug, Clone, PartialEq)]
enum Piece {
    Text(String),
    Html(String),
}

/// A report sentence that quotes options.
type Sentence = Vec<Piece>;

/// `fill` for sentences: `values` go in as text, `quotes` as rendered HTML.
fn fill_quoted(template: &str, values: &[(&str, String)], quotes: &[(&str, String)]) -> Sentence {
    let mut pieces = vec![Piece::Text(template.to_string())];
    for (name, html) in quotes {
        let mark = format!("{{{name}}}");
        pieces = pieces
            .into_iter()
            .flat_map(|piece| match piece {
                Piece::Text(text) => {
                    let mut out = Vec::new();
                    for (k, part) in text.split(&mark).enumerate() {
                        if k > 0 {
                            out.push(Piece::Html(html.clone()));
                        }
                        out.push(Piece::Text(part.to_string()));
                    }
                    out
                }
                html => vec![html],
            })
            .collect();
    }
    pieces
        .into_iter()
        .filter_map(|piece| match piece {
            Piece::Text(text) if text.is_empty() => None,
            Piece::Text(text) => Some(Piece::Text(fill(&text, values))),
            html => Some(html),
        })
        .collect()
}

fn sentence_view(sentence: Sentence) -> Element {
    rsx! {
        for (k, piece) in sentence.into_iter().enumerate() {
            match piece {
                Piece::Text(text) => rsx! { "{text}" },
                Piece::Html(html) => rsx! { span { key: "{k}", class: "quoted", dangerous_inner_html: "{html}" } },
            }
        }
    }
}

fn theme_title(paper: &Paper, id: &str) -> String {
    paper
        .theme(id)
        .map_or_else(|| id.to_string(), |t| t.title.clone())
}

/// What the teacher can decide about a doubtful key, ready to click.
struct Choices {
    question: String,
    /// Each distractor rendered, the rival first, with the key it would give.
    accept: Vec<(String, Vec<String>)>,
}

/// One button per distractor: on a single-answer question it replaces the key, on a multiple
/// answer one it joins it.
fn key_choices(paper: &Paper, formulas: &Formulas, question: &str, rival: Option<&str>) -> Choices {
    let mut accept = Vec::new();
    if let Some(q) = paper.question(question) {
        let key: Vec<String> = q
            .options
            .iter()
            .filter(|o| o.is_correct)
            .map(|o| o.id.clone())
            .collect();
        let mut distractors: Vec<_> = q.options.iter().filter(|o| !o.is_correct).collect();
        distractors.sort_by_key(|o| Some(o.id.as_str()) != rival);
        for option in distractors {
            let new_key = if key.len() > 1 {
                let mut joined = key.clone();
                joined.push(option.id.clone());
                joined
            } else {
                vec![option.id.clone()]
            };
            accept.push((markdown::inline(&option.text, formulas), new_key));
        }
    }
    Choices {
        question: question.to_string(),
        accept,
    }
}

/// The findings as sentences, by section.
struct Lines {
    /// Each theme to reteach, with the doubtful keys to check first.
    reteach: Vec<(String, Option<String>)>,
    traps: Vec<Sentence>,
    /// A doubtful key comes with its choices.
    review: Vec<(Sentence, Option<Choices>)>,
}

/// One sentence per finding of a kind, in the report's order.
fn finding_lines(report: &Report, paper: &Paper, c: &CorrectTexts, lang: Lang) -> Lines {
    let d = |v: f64| decimal(v, 2, lang);
    let formulas = option_formulas(paper);
    let text = |line: String| vec![Piece::Text(line)];
    let (mut reteach, mut traps, mut review) = (Vec::new(), Vec::new(), Vec::new());
    for finding in &report.findings {
        match finding {
            Finding::Reteach {
                theme,
                mean_rate,
                doubtful,
            } => {
                let stats = report.themes.iter().find(|t| t.id == *theme);
                let share = stats.map_or(0.0, |t| {
                    let n = f64::from(t.assessed().max(1));
                    f64::from(t.fragile + t.to_relearn) / n
                });
                let mut line = fill(
                    c.reteach,
                    &[
                        ("theme", theme_title(paper, theme)),
                        ("rate", pct(*mean_rate)),
                        ("share", pct(share)),
                    ],
                );
                if let Some(course) = stats
                    .map(|t| t.course_ref.clone())
                    .filter(|r| !r.is_empty())
                {
                    line = format!("{line} → {course}");
                }
                let doubt = (!doubtful.is_empty()).then(|| {
                    fill(
                        c.reteach_doubtful,
                        &[
                            ("n", doubtful.len().to_string()),
                            ("ids", doubtful.join(", ")),
                        ],
                    )
                });
                reteach.push((line, doubt));
            }
            Finding::Trap {
                question,
                option,
                rate,
            } => traps.push(fill_quoted(
                c.trap,
                &[("rate", pct(*rate)), ("question", question.clone())],
                &[("option", quoted(paper, &formulas, question, option))],
            )),
            Finding::KeyDoubt {
                question,
                rival,
                discrimination,
            } => review.push((
                match rival {
                    Some(rival) => fill_quoted(
                        c.key_doubt_rival,
                        &[("question", question.clone()), ("d", d(*discrimination))],
                        &[("rival", quoted(paper, &formulas, question, rival))],
                    ),
                    None => text(fill(
                        c.key_doubt,
                        &[("question", question.clone()), ("d", d(*discrimination))],
                    )),
                },
                Some(key_choices(paper, &formulas, question, rival.as_deref())),
            )),
            Finding::Reported { question, students } => review.push((
                text(fill(
                    c.reported,
                    &[("question", question.clone()), ("n", students.to_string())],
                )),
                None,
            )),
            Finding::TooHard { question, facility } => review.push((
                text(fill(
                    c.too_hard,
                    &[("question", question.clone()), ("pct", pct(1.0 - facility))],
                )),
                None,
            )),
            Finding::DeadDistractors { question, count } => review.push((
                text(fill(
                    c.dead,
                    &[("question", question.clone()), ("n", count.to_string())],
                )),
                None,
            )),
            Finding::TooEasy { question, .. } => {
                review.push((
                    text(fill(c.too_easy, &[("question", question.clone())])),
                    None,
                ));
            }
            Finding::WeakDiscrimination {
                question,
                discrimination,
            } => review.push((
                text(fill(
                    c.weak,
                    &[("question", question.clone()), ("d", d(*discrimination))],
                )),
                None,
            )),
        }
    }
    Lines {
        reteach,
        traps,
        review,
    }
}

fn anomaly_line(anomaly: &Anomaly, c: &CorrectTexts) -> String {
    match anomaly {
        Anomaly::OtherPaper { name, .. } => fill(c.other_paper, &[("name", name.clone())]),
        Anomaly::SameName { name, count } => fill(
            c.same_name,
            &[("name", name.clone()), ("n", count.to_string())],
        ),
        Anomaly::UnknownQuestions { name, count } => fill(
            c.unknown_questions,
            &[("name", name.clone()), ("n", count.to_string())],
        ),
        Anomaly::Identical {
            first,
            second,
            shared_wrong,
        } => fill(
            c.identical,
            &[
                ("first", first.clone()),
                ("second", second.clone()),
                ("n", shared_wrong.to_string()),
            ],
        ),
        Anomaly::LeftWindow {
            name,
            times,
            seconds_away,
        } => fill(
            c.left_window,
            &[
                ("name", name.clone()),
                ("n", times.to_string()),
                ("minutes", minutes(*seconds_away)),
            ],
        ),
        Anomaly::ExtraTime {
            name,
            extension: TimeExtension::Untimed,
        } => fill(c.untimed, &[("name", name.clone())]),
        Anomaly::ExtraTime { name, extension } => fill(
            c.extra_time,
            &[
                ("name", name.clone()),
                ("extension", extension.label().to_string()),
            ],
        ),
        Anomaly::Overtime { name, seconds } => fill(
            c.overtime,
            &[("name", name.clone()), ("minutes", minutes(*seconds))],
        ),
        Anomaly::Fast {
            name,
            elapsed_seconds,
        } => fill(
            c.fast,
            &[
                ("name", name.clone()),
                ("minutes", minutes(*elapsed_seconds)),
            ],
        ),
    }
}

fn reliability_line(value: f64, c: &CorrectTexts, lang: Lang) -> String {
    let level = match value {
        v if v < 0.5 => c.reliability_levels[0],
        v if v < 0.7 => c.reliability_levels[1],
        v if v < 0.8 => c.reliability_levels[2],
        _ => c.reliability_levels[3],
    };
    fill(
        c.reliability,
        &[
            ("value", decimal(value, 2, lang)),
            ("level", level.to_string()),
        ],
    )
}

/// The decisions in force, one sentence each with the question they are about.
fn decision_lines(report: &Report, paper: &Paper, c: &CorrectTexts) -> Vec<(String, Sentence)> {
    let formulas = option_formulas(paper);
    let neutralized = report
        .adjustments
        .iter()
        .filter(|a| a.decision == Decision::Neutralize)
        .count();
    let graded = paper.questions.len().saturating_sub(neutralized);
    report
        .adjustments
        .iter()
        .map(|a| {
            let line = match &a.decision {
                Decision::Neutralize => vec![Piece::Text(fill(
                    c.decided_neutral,
                    &[("question", a.question.clone()), ("n", graded.to_string())],
                ))],
                Decision::Rekey(options) => fill_quoted(
                    c.decided_rekey,
                    &[("question", a.question.clone())],
                    &[(
                        "options",
                        options
                            .iter()
                            .map(|o| quoted(paper, &formulas, &a.question, o))
                            .collect::<Vec<_>>()
                            .join("», «"),
                    )],
                ),
            };
            (a.question.clone(), line)
        })
        .collect()
}

/// What the report view hands back: a revision sheet to open, or a decision on the key.
#[derive(Debug, Clone, PartialEq)]
pub enum ReportAction {
    Sheet(String),
    Timeline(String),
    Decide(Adjustment),
    Undo(String),
}

/// The whole diagnosis, as text lines a test can read and a teacher can act on.
pub fn report_view(
    report: &Report,
    paper: &Paper,
    t: &'static Texts,
    lang: Lang,
    on_action: EventHandler<ReportAction>,
) -> Element {
    let c = t.correction;
    let decisions = decision_lines(report, paper, c);
    let summary = fill(
        c.summary,
        &[
            ("n", report.students.len().to_string()),
            ("mean", decimal(report.mean_mark.unwrap_or(0.0), 1, lang)),
            (
                "median",
                decimal(report.median_mark.unwrap_or(0.0), 1, lang),
            ),
            (
                "minutes",
                format!("{:.0}", report.mean_seconds.unwrap_or(0.0) / 60.0),
            ),
        ],
    );
    let reliability = report.reliability.map(|r| reliability_line(r, c, lang));
    let Lines {
        reteach,
        traps,
        review,
    } = finding_lines(report, paper, c, lang);
    let thin: Vec<String> = report
        .themes
        .iter()
        .filter(|th| th.questions > 0 && !th.is_diagnostic())
        .map(|th| {
            fill(
                c.thin_theme,
                &[("theme", th.title.clone()), ("n", th.questions.to_string())],
            )
        })
        .collect();
    let anomalies: Vec<String> = report
        .anomalies
        .iter()
        .map(|a| anomaly_line(a, c))
        .collect();
    let rows: Vec<(String, String, u32, u32, u32, String, String)> = report
        .students
        .iter()
        .map(|s| {
            (
                s.name.clone(),
                decimal(f64::from(s.mark), 1, lang),
                s.grade.correct,
                s.grade.wrong,
                s.grade.unanswered,
                minutes(s.elapsed_seconds),
                s.receipt.clone(),
            )
        })
        .collect();
    rsx! {
        section { class: "card report-head",
            p { class: "big-line", "{summary}" }
            if let Some(line) = reliability { p { "{line}" } }
            if report.small_cohort { p { class: "notice warn", "{c.small_cohort}" } }
            if !report.items_computed { p { class: "muted", "{c.items_not_computed}" } }
        }
        section { class: "card finding-group",
            h2 { "{c.reteach_title}" }
            if reteach.is_empty() { p { class: "ok-note", "{c.reteach_none}" } }
            ul {
                for (k, (line, doubt)) in reteach.into_iter().enumerate() {
                    li { key: "{k}", "{line}"
                        if let Some(doubt) = doubt { p { class: "notice warn", "{doubt}" } }
                    }
                }
            }
            for (k, line) in thin.into_iter().enumerate() { p { key: "{k}", class: "muted", "{line}" } }
        }
        section { class: "card finding-group",
            h2 { "{c.traps_title}" }
            if traps.is_empty() { p { class: "ok-note", "{c.traps_none}" } }
            ul { for (k, line) in traps.into_iter().enumerate() { li { key: "{k}", {sentence_view(line)} } } }
        }
        section { class: "card finding-group",
            h2 { "{c.review_title}" }
            if !decisions.is_empty() {
                div { class: "decisions",
                    h3 { "{c.decisions_title}" }
                    ul {
                        for (question, line) in decisions {
                            li { key: "{question}", class: "decision", {sentence_view(line)} " "
                                button { class: "small", onclick: { let q = question.clone(); move |_| on_action.call(ReportAction::Undo(q.clone())) }, "{c.undo}" }
                            }
                        }
                    }
                }
            }
            if review.is_empty() { p { class: "ok-note", "{c.review_none}" } }
            ul {
                for (k, (line, choices)) in review.into_iter().enumerate() {
                    li { key: "{k}", {sentence_view(line)}
                        if let Some(choices) = choices {
                            div { class: "key-choices",
                                p { class: "muted", "{c.decide_hint}" }
                                div { class: "actions",
                                    button { class: "small", onclick: { let q = choices.question.clone(); move |_| on_action.call(ReportAction::Decide(Adjustment::neutralize(&q))) }, "{c.neutralize}" }
                                    for (j, (text, key)) in choices.accept.into_iter().enumerate() {
                                        button { key: "{j}", class: "small",
                                            onclick: {
                                                let q = choices.question.clone();
                                                move |_| on_action.call(ReportAction::Decide(Adjustment { question: q.clone(), decision: Decision::Rekey(key.clone()) }))
                                            },
                                            {sentence_view(fill_quoted(c.accept, &[], &[("option", text)]))}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if !anomalies.is_empty() {
            section { class: "card finding-group",
                h2 { "{c.anomalies_title}" }
                p { class: "muted", "{c.anomalies_hint}" }
                ul { for (k, line) in anomalies.into_iter().enumerate() { li { key: "{k}", "{line}" } } }
            }
        }
        section { class: "card",
            h2 { "{c.marks_title}" }
            div { class: "table-scroll",
                table { class: "marks",
                    thead { tr { for (k, col) in c.columns.iter().enumerate() { th { key: "{k}", scope: "col", "{col}" } } td {} } }
                    tbody {
                        for (name, mark, right, wrong, blank, mins, receipt) in rows {
                            tr { key: "{receipt}",
                                th { scope: "row", "{name}" }
                                td { class: "num", "{mark}" }
                                td { class: "num", "{right}" }
                                td { class: "num", "{wrong}" }
                                td { class: "num", "{blank}" }
                                td { class: "num", "{mins}" }
                                td { class: "mono", "{receipt}" }
                                td { class: "row-actions",
                                    button { class: "small", onclick: { let receipt = receipt.clone(); move |_| on_action.call(ReportAction::Sheet(receipt.clone())) }, "{c.open_sheet}" }
                                    button { class: "small", onclick: { let receipt = receipt.clone(); move |_| on_action.call(ReportAction::Timeline(receipt.clone())) }, "{c.open_timeline}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn clock(seconds: u32) -> String {
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

/// One copy's timeline: a summary, a strip along the exam's duration, then each moment.
pub fn timeline_view(
    name: &str,
    timeline: &Timeline,
    paper: &Paper,
    t: &'static Texts,
    lang: Lang,
) -> Element {
    let c = t.correction;
    let summary = fill(
        c.timeline_summary,
        &[
            ("changes", timeline.changes().to_string()),
            ("rw", timeline.right_to_wrong.to_string()),
            ("wr", timeline.wrong_to_right.to_string()),
            ("ww", timeline.wrong_to_wrong.to_string()),
            ("away", timeline.away_count.to_string()),
            (
                "minutes",
                decimal(f64::from(timeline.away_seconds) / 60.0, 1, lang),
            ),
        ],
    );
    let formulas = option_formulas(paper);
    let text = |line: String| vec![Piece::Text(line)];
    let span = f64::from(timeline.elapsed_seconds.max(1));
    let at = |seconds: u32| (f64::from(seconds) / span * 100.0).min(100.0);
    // Marks on the strip: left %, width %, class.
    let mut marks: Vec<(String, String, &'static str)> = Vec::new();
    let mut rows: Vec<(String, Sentence, &'static str)> = Vec::new();
    for moment in &timeline.moments {
        let (line, class) = match &moment.event {
            Event::Answer {
                question,
                question_id,
                selected,
                right,
                change,
            } => {
                let options: Vec<String> = selected
                    .iter()
                    .map(|o| quoted(paper, &formulas, question_id, o))
                    .collect();
                let mut line = fill_quoted(
                    c.tl_answer,
                    &[("q", question.to_string())],
                    &[("options", options.join("», «"))],
                );
                let mut tail = format!(" ({})", if *right { c.tl_right } else { c.tl_wrong });
                let class = match change {
                    Some(change) => {
                        let k = match change {
                            Change::RightToWrong => 0,
                            Change::WrongToRight => 1,
                            Change::WrongToWrong => 2,
                        };
                        tail.push_str(&format!(" · {}", c.tl_changes[k]));
                        match change {
                            Change::RightToWrong => "tl-change bad",
                            Change::WrongToRight => "tl-change good",
                            Change::WrongToWrong => "tl-change",
                        }
                    }
                    None if *right => "tl-answer good",
                    None => "tl-answer bad",
                };
                line.push(Piece::Text(tail));
                (line, class)
            }
            Event::Cleared { question, .. } => (
                text(fill(c.tl_cleared, &[("q", question.to_string())])),
                "tl-answer",
            ),
            Event::Away { seconds: 0 } => (text(c.tl_away_open.to_string()), "tl-away"),
            Event::Away { seconds } => (
                text(fill(
                    c.tl_away,
                    &[("duration", mcq_engine::format_duration(*seconds))],
                )),
                "tl-away",
            ),
            Event::Submitted => (text(c.tl_submitted.to_string()), "tl-submit"),
        };
        let width = match moment.event {
            Event::Away { seconds: 0 } => 100.0 - at(moment.seconds),
            Event::Away { seconds } => at(seconds),
            _ => 0.0,
        };
        marks.push((
            format!("{:.2}", at(moment.seconds)),
            format!("{width:.2}"),
            class,
        ));
        rows.push((clock(moment.seconds), line, class));
    }
    let total = clock(timeline.elapsed_seconds);
    rsx! {
        section { class: "card printable timeline",
            h1 { "{c.timeline_title}" }
            p { strong { "{name}" } " · {total}" }
            p { class: "muted", "{c.timeline_hint}" }
            p { "{summary}" }
            if rows.is_empty() { p { "{c.timeline_empty}" } } else {
                div { class: "timeline-strip", "aria-hidden": "true",
                    for (k, (left, width, class)) in marks.into_iter().enumerate() {
                        span { key: "{k}", class: "mark {class}", style: "left: {left}%; width: {width}%" }
                    }
                }
                ol { class: "timeline-list",
                    for (k, (time, line, class)) in rows.into_iter().enumerate() {
                        li { key: "{k}", class: "{class}", span { class: "mono time", "{time}" } " " {sentence_view(line)} }
                    }
                }
            }
        }
    }
}

/// A student's revision sheet, ready to print.
pub fn sheet_view(sheet: &RevisionSheet, t: &'static Texts, lang: Lang) -> Element {
    let c = t.correction;
    let mark = decimal(f64::from(sheet.mark), 1, lang);
    let themes: Vec<(String, String, &'static str, bool)> = sheet
        .themes
        .iter()
        .map(|th| {
            let state = match th.mastery {
                Mastery::Acquired => c.mastery[0],
                Mastery::Fragile => c.mastery[1],
                Mastery::ToRelearn => c.mastery[2],
                Mastery::NotAssessed => "",
            };
            (th.title.clone(), pct(f64::from(th.rate)), state, th.too_few)
        })
        .collect();
    let mistakes_title = fill(c.sheet_mistakes, &[("n", sheet.mistakes.len().to_string())]);
    let mistakes: Vec<Element> = sheet
        .mistakes
        .iter()
        .map(|m| {
            let q = &m.question;
            let tokens = preview::tokens(q);
            let mut texts = preview::student_texts(q);
            texts.extend(q.options.iter().map(|o| o.explanation.as_str()));
            let formulas = preview::formulas(&texts);
            let mut code_index = 0;
            let body: Vec<Element> = q
                .body
                .iter()
                .filter_map(|block| match block {
                    Block::Text(text) => {
                        let html = markdown::block(text, &formulas);
                        Some(rsx! { div { class: "prose", dangerous_inner_html: "{html}" } })
                    }
                    Block::Code(code) => {
                        let tok = tokens.get(code_index);
                        code_index += 1;
                        Some(render::code_block(code, tok))
                    }
                    Block::Media(_) => None,
                })
                .collect();
            let chosen: Vec<String> = m
                .chosen
                .iter()
                .filter_map(|id| q.option(id))
                .map(|o| markdown::inline(&o.text, &formulas))
                .collect();
            let right: Vec<(String, String)> = q
                .options
                .iter()
                .filter(|o| o.is_correct)
                .map(|o| {
                    (
                        markdown::inline(&o.text, &formulas),
                        markdown::block(&o.explanation, &formulas),
                    )
                })
                .collect();
            let rule = q.rule.clone();
            let review = q.course_ref.clone();
            let doubt = m.key_in_doubt;
            rsx! {
                article { class: "mistake",
                    for (k, element) in body.into_iter().enumerate() { div { key: "{k}", {element} } }
                    p { strong { "{c.sheet_yours}" } " "
                        if chosen.is_empty() { em { "{c.sheet_blank}" } }
                        for (k, html) in chosen.into_iter().enumerate() { span { key: "{k}", class: "chosen-answer", dangerous_inner_html: "{html}" } }
                    }
                    if doubt { p { class: "notice warn", "{c.sheet_key_in_doubt}" } } else {
                    div { strong { "{c.sheet_right}" }
                        ul { for (k, (html, why)) in right.into_iter().enumerate() {
                            li { key: "{k}", span { dangerous_inner_html: "{html}" }
                                if !why.is_empty() { div { class: "explanation", dangerous_inner_html: "{why}" } }
                            }
                        } }
                    }
                    if !rule.is_empty() { p { class: "rule", strong { "{c.sheet_remember}" } " {rule}" } }
                    }
                    if !review.is_empty() { p { class: "muted", em { "{c.sheet_revise} {review}" } } }
                }
            }
        })
        .collect();
    rsx! {
        section { class: "card printable revision-sheet",
            h1 { "{c.sheet_title}" }
            p { strong { "{sheet.name}" } " · {sheet.paper_title} · {mark} / 20" }
            h2 { "{c.sheet_where}" }
            table { class: "marks",
                tbody {
                    for (k, (title, rate, state, too_few)) in themes.into_iter().enumerate() {
                        tr { key: "{k}",
                            th { scope: "row", "{title}" }
                            td { class: "num", "{rate} %" }
                            td { "{state}" if too_few { " {c.sheet_too_few}" } }
                        }
                    }
                }
            }
            h2 { "{mistakes_title}" }
            if mistakes.is_empty() { p { "{c.sheet_none}" } }
            for (k, element) in mistakes.into_iter().enumerate() { div { key: "{k}", {element} } }
            if !sheet.start_with.is_empty() {
                h2 { "{c.sheet_start}" }
                ol { for (k, title) in sheet.start_with.iter().enumerate() { li { key: "{k}", "{title}" } } }
            }
        }
    }
}

fn refusal_text(refusal: Refusal, c: &CorrectTexts) -> &'static str {
    match refusal {
        Refusal::Unknown => c.refused_unknown,
        Refusal::NotYours => c.refused_not_yours,
        Refusal::AlreadyIn => c.refused_already_in,
        Refusal::Damaged => c.refused_damaged,
    }
}

fn download_text(name: &str, text: &str) {
    platform::download(name, &b64::encode(text.as_bytes()));
}

#[component]
pub fn CorrectSpace(
    correction: Signal<Correction>,
    identity: Signal<Option<Rc<Identity>>>,
    done: Signal<Option<Rc<Done>>>,
    lang: Lang,
    t: &'static Texts,
) -> Element {
    let c = t.correction;
    let mut sheet_for = use_signal(|| None::<String>);
    let mut timeline_for = use_signal(|| None::<String>);

    // Files that arrived before the identity open as soon as it is unlocked.
    use_effect(move || {
        if let Some(id) = identity()
            && correction.peek().pending() > 0
        {
            correction.write().open_pending(&id);
        }
    });

    let state = correction.read();
    let report = state.report();
    let paper = state.package.as_ref().map(|p| p.key.clone());
    let refused: Vec<(String, &'static str)> = state
        .refused
        .iter()
        .map(|(name, why)| (name.clone(), refusal_text(*why, c)))
        .collect();
    let waiting = state.pending();
    let copies = state.copies.len();
    let package_line = paper
        .as_ref()
        .map(|p| fill(c.package_ready, &[("title", p.title.clone())]));
    let sheet = match (&report, sheet_for()) {
        (Some(report), Some(receipt)) => state.sheet(report, &receipt),
        _ => None,
    };
    let timeline = match (&report, timeline_for()) {
        (Some(report), Some(receipt)) => state.timeline(report, &receipt),
        _ => None,
    };
    let published = done.read().clone();
    let labels = if lang == Lang::Fr { &csv::FR } else { &csv::EN };
    drop(state);

    let mut add = move |name: String, bytes: Vec<u8>| {
        let id = identity.read().clone();
        correction.write().add_file(&name, bytes, id.as_deref());
    };

    rsx! {
        div { class: "correct",
            IdentityCard { identity, t }
            section { class: "card",
                p { "{c.intro}" }
                div { class: "actions",
                    label { class: "file",
                        "{c.load_package}"
                        input { r#type: "file", accept: ".mcqpack", class: "sr-only",
                            onchange: move |e: FormEvent| async move {
                                for file in e.files() {
                                    if let Ok(bytes) = file.read_bytes().await {
                                        add(file.name(), bytes.to_vec());
                                    }
                                }
                            },
                        }
                    }
                    if let Some(last) = published {
                        button { onclick: move |_| add(last.file_name.clone(), last.published.package.clone()), "{c.use_published}" }
                    }
                    label { class: "file",
                        "{c.add_copies}"
                        input { r#type: "file", accept: ".mcqres", multiple: true, class: "sr-only",
                            onchange: move |e: FormEvent| async move {
                                for file in e.files() {
                                    if let Ok(bytes) = file.read_bytes().await {
                                        add(file.name(), bytes.to_vec());
                                    }
                                }
                            },
                        }
                    }
                    button { onclick: move |_| { correction.set(Correction::default()); sheet_for.set(None); timeline_for.set(None); }, "{c.clear}" }
                }
                p { class: "muted", "{c.drop_hint}" }
                if let Some(line) = package_line { p { strong { "{line}" } } }
                p { "{fill(c.copies_in, &[(\"n\", copies.to_string())])}" }
                if waiting > 0 { p { class: "notice warn", "{fill(c.waiting, &[(\"n\", waiting.to_string())])}" } }
                if !refused.is_empty() {
                    h3 { "{c.refused}" }
                    ul { for (k, (name, why)) in refused.into_iter().enumerate() { li { key: "{k}", span { class: "mono", "{name}" } " ({why})" } } }
                }
            }
            if let Some(sheet) = sheet {
                div { class: "actions",
                    button { class: "primary", onclick: move |_| platform::print(), "{c.print_student_sheet}" }
                    button { onclick: move |_| sheet_for.set(None), "{c.close_sheet}" }
                }
                {sheet_view(&sheet, t, lang)}
            } else if let (Some((name, timeline)), Some(paper)) = (timeline, paper.as_ref()) {
                div { class: "actions",
                    button { class: "primary", onclick: move |_| platform::print(), "{c.print_student_sheet}" }
                    button { onclick: move |_| timeline_for.set(None), "{c.close_sheet}" }
                }
                {timeline_view(&name, &timeline, paper, t, lang)}
            } else if let (Some(report), Some(paper)) = (report, paper) {
                div { class: "actions",
                    button { onclick: { let (r, p) = (report.clone(), paper.clone()); move |_| download_text("notes.csv", &csv::notes(&p, &r, labels)) }, "{c.export_notes}" }
                    button { onclick: { let r = report.clone(); move |_| download_text("themes.csv", &csv::themes(&r, labels)) }, "{c.export_themes}" }
                    button { onclick: { let (r, p) = (report.clone(), paper.clone()); move |_| download_text("questions.csv", &csv::questions(&p, &r, labels)) }, "{c.export_questions}" }
                }
                {report_view(&report, &paper, t, lang, EventHandler::new(move |action: ReportAction| match action {
                    ReportAction::Sheet(receipt) => sheet_for.set(Some(receipt)),
                    ReportAction::Timeline(receipt) => timeline_for.set(Some(receipt)),
                    ReportAction::Decide(adjustment) => correction.write().decide(adjustment),
                    ReportAction::Undo(question) => correction.write().undo(&question),
                }))}
            }
        }
    }
}
