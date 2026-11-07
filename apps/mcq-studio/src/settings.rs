//! The Settings space: what the paper's header holds, and its themes.

use dioxus::prelude::*;
use mcq_edit::ops;
use mcq_model::{INTERFACE_LANGUAGES, Issue, Scoring};

use mcq_player::icon::{Icon, IconButton};

use crate::editor::issue_list;
use crate::state::{self, Studio};
use crate::texts::Texts;

/// `0,25` and `0.25` both read as a quarter; anything else as zero.
fn number(text: &str) -> f32 {
    text.trim()
        .replace(',', ".")
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .unwrap_or(0.0)
}

/// The languages a paper is most often written in, each in its own words as language pickers
/// show them.
const PAPER_LANGUAGES: [(&str, &str); 8] = [
    ("fr", "Français"),
    ("en", "English"),
    ("de", "Deutsch"),
    ("es", "Español"),
    ("it", "Italiano"),
    ("pt", "Português"),
    ("nl", "Nederlands"),
    ("ca", "Català"),
];

/// Each language in its own words; a tag outside the list as written.
fn endonym(tag: &str) -> &str {
    PAPER_LANGUAGES
        .iter()
        .find(|(t, _)| *t == tag)
        .map_or(tag, |(_, name)| name)
}

/// The choices of the paper's language: the usual ones, and the paper's own when it is another.
fn language_choices(current: &str) -> Vec<String> {
    let mut tags: Vec<String> = PAPER_LANGUAGES.iter().map(|(t, _)| t.to_string()).collect();
    if !current.is_empty() && !tags.iter().any(|t| t == current) {
        tags.push(current.to_string());
    }
    tags
}

/// Ticks or unticks one language; languages this Studio has no box for stay as written.
fn offer_language(languages: &mut Vec<String>, tag: &str, offered: bool) {
    languages.retain(|l| l != tag);
    if offered {
        languages.push(tag.to_string());
    }
}

pub fn settings_space(mut studio: Signal<Studio>, t: &'static Texts) -> Element {
    let s = studio.read();
    let paper = &s.project.paper;
    let estimated = paper.estimated_seconds_with_margin().div_ceil(60);
    let issues = s.issues();
    let general: Vec<&Issue> = state::paper_issues(paper, &issues);
    let themes: Vec<(String, String, String, bool)> = paper
        .themes
        .iter()
        .map(|th| {
            let used = paper.questions.iter().any(|q| q.theme_id == th.id);
            (th.id.clone(), th.title.clone(), th.course_ref.clone(), used)
        })
        .collect();
    let partial = paper.scoring == Scoring::Partial;
    rsx! {
        div { class: "settings",
            fieldset { class: "card",
                legend { "{t.settings_paper}" }
                label { r#for: "p-title", "{t.title}" }
                input { id: "p-title", r#type: "text", value: "{paper.title}",
                    // The starter title is selected on entry: typing replaces it.
                    onfocus: move |_| {
                        if studio.peek().project.paper.title == t.new_paper_title {
                            let _ = document::eval("setTimeout(() => { const i = document.getElementById('p-title'); if (i) i.select(); }, 0);");
                        }
                    },
                    oninput: move |e| studio.write().edit_paper(|p| p.title = e.value()) }
                label { r#for: "p-description", "{t.description}" }
                textarea { id: "p-description", rows: "3", value: "{paper.description}",
                    oninput: move |e| studio.write().edit_paper(|p| p.description = e.value().replace('\n', " ")) }
                label { r#for: "p-lang", "{t.paper_lang}" }
                select { id: "p-lang",
                    onchange: move |e| studio.write().edit_paper(|p| p.lang = e.value()),
                    option { value: "", selected: paper.lang.is_empty(), "{t.lang_unset}" }
                    for tag in language_choices(&paper.lang) {
                        option { key: "{tag}", value: "{tag}", lang: "{tag}", selected: tag == paper.lang, "{endonym(&tag)}" }
                    }
                }
                div { role: "group", "aria-labelledby": "p-languages", "aria-describedby": "p-languages-hint",
                    p { id: "p-languages", class: "group-label", "{t.paper_languages}" }
                    for tag in INTERFACE_LANGUAGES {
                        label { key: "{tag}", class: "check", lang: tag,
                            input { r#type: "checkbox", checked: paper.languages.iter().any(|l| l == tag),
                                onchange: move |e| studio.write().edit_paper(|p| offer_language(&mut p.languages, tag, e.checked())) }
                            "{endonym(tag)}"
                        }
                    }
                    p { id: "p-languages-hint", class: "muted", "{t.paper_languages_hint}" }
                }
                label { r#for: "p-notes", "{t.notes}" }
                textarea { id: "p-notes", rows: "3", value: "{paper.notes}",
                    oninput: move |e| studio.write().edit_paper(|p| p.notes = e.value().lines().filter(|l| !l.trim().is_empty()).collect::<Vec<_>>().join("\n")) }
            }
            fieldset { class: "card",
                legend { "{t.settings_time}" }
                label { r#for: "p-duration", "{t.duration}" }
                input { id: "p-duration", r#type: "number", min: "0", class: "short", value: "{paper.duration_minutes}",
                    oninput: move |e| studio.write().edit_paper(|p| p.duration_minutes = e.value().trim().parse().unwrap_or(0)) }
                p { class: "muted", "{t.estimated} {estimated} {t.minutes}" }
                p { class: "muted small-text", "{t.estimated_hint}" }
                if paper.duration_minutes > 0 { p { class: "muted small-text", "{t.duration_soft}" } }
            }
            fieldset { class: "card",
                legend { "{t.settings_scoring}" }
                div { class: "segmented", role: "group", "aria-label": "{t.settings_scoring}",
                    button { "aria-pressed": "{!partial}", onclick: move |_| studio.write().edit_paper(|p| p.scoring = Scoring::AllOrNothing), "{t.scoring_all_or_nothing}" }
                    button { "aria-pressed": "{partial}", onclick: move |_| studio.write().edit_paper(|p| p.scoring = Scoring::Partial), "{t.scoring_partial}" }
                }
                label { r#for: "p-penalty", "{t.penalty}" }
                input { id: "p-penalty", r#type: "number", min: "0", step: "0.05", class: "short", value: "{paper.penalty}",
                    onchange: move |e| studio.write().edit_paper(|p| p.penalty = number(&e.value())) }
                label { r#for: "p-blank", "{t.unanswered_penalty}" }
                input { id: "p-blank", r#type: "number", min: "0", step: "0.05", class: "short", value: "{paper.unanswered_penalty}",
                    onchange: move |e| studio.write().edit_paper(|p| p.unanswered_penalty = number(&e.value())) }
            }
            fieldset { class: "card",
                legend { "{t.settings_exam}" }
                label { class: "check",
                    input { r#type: "checkbox", checked: paper.shuffle_questions,
                        onchange: move |e| studio.write().edit_paper(|p| p.shuffle_questions = e.checked()) }
                    "{t.shuffle_questions}"
                }
                label { class: "check",
                    input { r#type: "checkbox", checked: paper.shuffle_options,
                        onchange: move |e| studio.write().edit_paper(|p| p.shuffle_options = e.checked()) }
                    "{t.shuffle_options}"
                }
                label { class: "check",
                    input { r#type: "checkbox", checked: paper.show_answer_count,
                        onchange: move |e| studio.write().edit_paper(|p| p.show_answer_count = e.checked()) }
                    "{t.show_answer_count}"
                }
                label { class: "check",
                    input { r#type: "checkbox", checked: paper.allow_extra_time,
                        onchange: move |e| studio.write().edit_paper(|p| p.allow_extra_time = e.checked()) }
                    "{t.allow_extra_time}"
                }
            }
            fieldset { class: "card",
                legend { "{t.themes}" }
                for (k, (id, title, course, used)) in themes.into_iter().enumerate() {
                    div { key: "{id}", class: "theme-row",
                        span { class: "mono muted", "{id}" }
                        label { "{t.theme_title}"
                            input { r#type: "text", value: "{title}",
                                oninput: move |e| studio.write().edit_paper(|p| p.themes[k].title = e.value()) }
                        }
                        label { "{t.course_ref}"
                            input { r#type: "text", value: "{course}",
                                oninput: move |e| studio.write().edit_paper(|p| p.themes[k].course_ref = e.value()) }
                        }
                        div { class: "block-actions",
                            IconButton { icon: Icon::ArrowUp, label: t.move_up, tip: t.tip_theme_up,
                                onclick: { let id = id.clone(); move |_| studio.write().edit_paper(|p| { ops::move_theme(p, &id, true); }) } }
                            IconButton { icon: Icon::ArrowDown, label: t.move_down, tip: t.tip_theme_down,
                                onclick: { let id = id.clone(); move |_| studio.write().edit_paper(|p| { ops::move_theme(p, &id, false); }) } }
                            IconButton { icon: Icon::Trash, label: t.delete, tip: t.remove_theme, disabled: used,
                                onclick: { let id = id.clone(); move |_| studio.write().edit_paper(|p| { ops::remove_theme(p, &id); }) } }
                        }
                    }
                }
                IconButton { icon: Icon::Plus, label: t.add_theme, tip: t.tip_add_theme, onclick: move |_| studio.write().edit_paper(|p| { ops::add_theme(p, t.new_theme); }) }
            }
            section { class: "card",
                h2 { "{t.paper_findings}" }
                {issue_list(&general, t, true)}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{endonym, language_choices, number, offer_language};

    #[test]
    fn the_paper_language_is_picked_and_an_unusual_one_kept() {
        assert_eq!(endonym("fr"), "Français");
        assert_eq!(endonym("eo"), "eo");
        assert!(language_choices("").iter().all(|t| !t.is_empty()));
        let tags = language_choices("eo");
        assert_eq!(tags.last().map(String::as_str), Some("eo"));
        assert_eq!(tags.iter().filter(|t| *t == "fr").count(), 1);
        assert_eq!(language_choices("fr").len(), language_choices("").len());
    }

    #[test]
    fn ticking_languages_keeps_the_others() {
        let mut languages = vec!["de".to_string()];
        offer_language(&mut languages, "fr", true);
        offer_language(&mut languages, "fr", true);
        offer_language(&mut languages, "en", true);
        assert_eq!(languages, ["de", "fr", "en"]);
        offer_language(&mut languages, "fr", false);
        assert_eq!(languages, ["de", "en"]);
    }

    #[test]
    fn decimal_commas_are_read() {
        assert_eq!(number("0,25"), 0.25);
        assert_eq!(number(" 0.5 "), 0.5);
        assert_eq!(number("-1"), 0.0);
        assert_eq!(number("abc"), 0.0);
    }
}
