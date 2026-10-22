//! The Correct space: the package and the copies go in, the class's diagnosis comes out, in the
//! order a teacher acts on it (what to reteach, shared confusions, questions to review, copies to
//! look at), the marks last; revision sheets to print and spreadsheets to export.

use std::rc::Rc;

use dioxus::prelude::*;
use mcq_analysis::sheet::RevisionSheet;
use mcq_analysis::{Anomaly, Finding, Report, csv};
use mcq_crypto::Identity;
use mcq_grade::Mastery;
use mcq_model::{Block, Paper};
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

fn theme_title(paper: &Paper, id: &str) -> String {
    paper
        .theme(id)
        .map_or_else(|| id.to_string(), |t| t.title.clone())
}

/// The findings as sentences, by section.
struct Lines {
    /// Each theme to reteach, with the doubtful keys to check first.
    reteach: Vec<(String, Option<String>)>,
    traps: Vec<String>,
    review: Vec<String>,
}

/// One sentence per finding of a kind, in the report's order.
fn finding_lines(report: &Report, paper: &Paper, c: &CorrectTexts, lang: Lang) -> Lines {
    let d = |v: f64| decimal(v, 2, lang);
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
            } => traps.push(fill(
                c.trap,
                &[
                    ("rate", pct(*rate)),
                    ("option", option_text(paper, question, option)),
                    ("question", question.clone()),
                ],
            )),
            Finding::KeyDoubt {
                question,
                rival,
                discrimination,
            } => review.push(match rival {
                Some(rival) => fill(
                    c.key_doubt_rival,
                    &[
                        ("question", question.clone()),
                        ("rival", option_text(paper, question, rival)),
                        ("d", d(*discrimination)),
                    ],
                ),
                None => fill(
                    c.key_doubt,
                    &[("question", question.clone()), ("d", d(*discrimination))],
                ),
            }),
            Finding::TooHard { question, facility } => review.push(fill(
                c.too_hard,
                &[("question", question.clone()), ("pct", pct(1.0 - facility))],
            )),
            Finding::DeadDistractors { question, count } => review.push(fill(
                c.dead,
                &[("question", question.clone()), ("n", count.to_string())],
            )),
            Finding::TooEasy { question, .. } => {
                review.push(fill(c.too_easy, &[("question", question.clone())]));
            }
            Finding::WeakDiscrimination {
                question,
                discrimination,
            } => review.push(fill(
                c.weak,
                &[("question", question.clone()), ("d", d(*discrimination))],
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

/// The whole diagnosis, as text lines a test can read and a teacher can act on.
pub fn report_view(
    report: &Report,
    paper: &Paper,
    t: &'static Texts,
    lang: Lang,
    on_sheet: EventHandler<String>,
) -> Element {
    let c = t.correction;
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
            ul { for (k, line) in traps.into_iter().enumerate() { li { key: "{k}", "{line}" } } }
        }
        section { class: "card finding-group",
            h2 { "{c.review_title}" }
            if review.is_empty() { p { class: "ok-note", "{c.review_none}" } }
            ul { for (k, line) in review.into_iter().enumerate() { li { key: "{k}", "{line}" } } }
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
                    thead { tr { for (k, col) in c.columns.iter().enumerate() { th { key: "{k}", scope: "col", "{col}" } } th {} } }
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
                                td { button { class: "small", onclick: { let receipt = receipt.clone(); move |_| on_sheet.call(receipt.clone()) }, "{c.open_sheet}" } }
                            }
                        }
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
            let mut code_index = 0;
            let body: Vec<Element> = q
                .body
                .iter()
                .filter_map(|block| match block {
                    Block::Text(text) => {
                        let html = markdown::block(text);
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
                .map(|o| markdown::inline(&o.text))
                .collect();
            let right: Vec<(String, String)> = q
                .options
                .iter()
                .filter(|o| o.is_correct)
                .map(|o| (markdown::inline(&o.text), markdown::block(&o.explanation)))
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
                    button { onclick: move |_| { correction.set(Correction::default()); sheet_for.set(None); }, "{c.clear}" }
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
            } else if let (Some(report), Some(paper)) = (report, paper) {
                div { class: "actions",
                    button { onclick: { let (r, p) = (report.clone(), paper.clone()); move |_| download_text("notes.csv", &csv::notes(&p, &r, labels)) }, "{c.export_notes}" }
                    button { onclick: { let r = report.clone(); move |_| download_text("themes.csv", &csv::themes(&r, labels)) }, "{c.export_themes}" }
                    button { onclick: { let (r, p) = (report.clone(), paper.clone()); move |_| download_text("questions.csv", &csv::questions(&p, &r, labels)) }, "{c.export_questions}" }
                }
                {report_view(&report, &paper, t, lang, EventHandler::new(move |receipt: String| sheet_for.set(Some(receipt))))}
            }
        }
    }
}
