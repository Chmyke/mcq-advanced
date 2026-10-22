//! The correction, once the answer key is open: the result first (mark, a word on it, themes,
//! where to start), then one question at a time, the mistakes first, as the earlier tool did.
//! What to show is decided by plain functions, so tests check it without a screen.

use std::collections::BTreeSet;

use dioxus::prelude::*;
use mcq_engine::{Session, format_duration};
use mcq_grade::{GradeResult, MIN_DIAGNOSTIC_QUESTIONS, Mastery, Status};
use mcq_model::{Block, Level, Paper};

use crate::markdown;
use crate::render::{self, Content};
use crate::texts::Texts;

/// Which questions the review walks through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    /// Anything short of full credit, blanks included.
    #[default]
    Mistakes,
    All,
    Right,
}

/// Where the student is in the review. "Understood" marks last for the visit only.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReviewState {
    pub filter: Filter,
    /// Position in the filtered list.
    pub position: usize,
    /// Session indices the student marked as understood.
    pub understood: BTreeSet<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewAction {
    Filter(Filter),
    /// A question of the session, by index: opens it, under "All" if the filter hides it.
    Go(usize),
    Previous,
    Next,
    /// Marks the question understood and moves on.
    Understood,
    DownloadSheet,
    Print,
}

fn status(result: &GradeResult, question_id: &str) -> Status {
    result
        .outcomes
        .iter()
        .find(|o| o.question_id == question_id)
        .map_or(Status::Unanswered, |o| o.status)
}

/// The session indices a filter keeps, in the student's order.
pub fn indices(session: &Session, result: &GradeResult, filter: Filter) -> Vec<usize> {
    session
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            let right = status(result, &item.question_id) == Status::Correct;
            match filter {
                Filter::Mistakes => !right,
                Filter::All => true,
                Filter::Right => right,
            }
        })
        .map(|(i, _)| i)
        .collect()
}

impl ReviewState {
    /// The session index on screen, if the filter keeps any question.
    pub fn current(&self, session: &Session, result: &GradeResult) -> Option<usize> {
        let list = indices(session, result, self.filter);
        list.get(self.position.min(list.len().saturating_sub(1)))
            .copied()
    }

    /// Applies a navigation action; downloads and printing are the caller's.
    pub fn apply(&mut self, action: ReviewAction, session: &Session, result: &GradeResult) {
        let len = indices(session, result, self.filter).len();
        match action {
            ReviewAction::Filter(filter) => {
                self.filter = filter;
                self.position = 0;
            }
            ReviewAction::Go(index) => {
                let list = indices(session, result, self.filter);
                match list.iter().position(|&i| i == index) {
                    Some(position) => self.position = position,
                    None => {
                        self.filter = Filter::All;
                        self.position = index;
                    }
                }
            }
            ReviewAction::Previous => self.position = self.position.saturating_sub(1),
            ReviewAction::Next => self.position = (self.position + 1).min(len.saturating_sub(1)),
            ReviewAction::Understood => {
                if let Some(index) = self.current(session, result) {
                    self.understood.insert(index);
                }
                self.position = (self.position + 1).min(len.saturating_sub(1));
            }
            ReviewAction::DownloadSheet | ReviewAction::Print => {}
        }
    }
}

/// A word on the mark, from the earlier tool: 16, 13, 10 and 6 out of 20 are the steps.
pub fn appreciation(mark: f32, t: &Texts) -> &'static str {
    let step = match mark {
        m if m >= 16.0 => 0,
        m if m >= 13.0 => 1,
        m if m >= 10.0 => 2,
        m if m >= 6.0 => 3,
        _ => 4,
    };
    t.appreciations[step]
}

/// A theme to review: title, success rate from 0 to 1, where to revise.
#[derive(Debug, Clone, PartialEq)]
pub struct WeakTheme {
    pub title: String,
    pub rate: f32,
    pub course_ref: String,
}

/// At most this many themes in "to review".
pub const WEAK_THEMES: usize = 5;

/// Fragile and to-relearn themes with enough questions to say so, weakest first.
pub fn weak_themes(full: &Paper, result: &GradeResult) -> Vec<WeakTheme> {
    let mut weak: Vec<WeakTheme> = result
        .themes
        .iter()
        .filter(|th| {
            th.total >= MIN_DIAGNOSTIC_QUESTIONS
                && matches!(th.mastery(), Mastery::Fragile | Mastery::ToRelearn)
        })
        .map(|th| {
            let theme = full.theme(&th.theme_id);
            WeakTheme {
                title: theme.map_or_else(|| th.theme_id.clone(), |x| x.title.clone()),
                rate: th.rate(),
                course_ref: theme.map(|x| x.course_ref.clone()).unwrap_or_default(),
            }
        })
        .collect();
    weak.sort_by(|a, b| a.rate.total_cmp(&b.rate));
    weak.truncate(WEAK_THEMES);
    weak
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pace {
    /// Under a third of the expected time.
    Quick,
    /// Over twice the expected time.
    Long,
}

/// How the time spent compares with the time the author expected.
pub fn pace(spent: u32, expected: u32) -> Option<Pace> {
    if spent == 0 || expected == 0 {
        None
    } else if spent * 3 < expected {
        Some(Pace::Quick)
    } else if spent > expected * 2 {
        Some(Pace::Long)
    } else {
        None
    }
}

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

fn mark_text(result: &GradeResult, t: &Texts) -> String {
    format!("{:.2}", result.mark(20.0)).replace('.', t.decimal)
}

/// The student's revision sheet in Markdown: where they stand, each mistake with the right
/// answer and why, where to start. Formulas stay in `$...$`, as note-taking apps read them.
pub fn revision_markdown(
    full: &Paper,
    session: &Session,
    result: &GradeResult,
    t: &Texts,
) -> String {
    let mut out = String::new();
    let mark = mark_text(result, t);
    out.push_str(&format!("# {} : {}\n\n", t.sheet_title, full.title));
    out.push_str(&format!(
        "{} · {mark} / 20 · {}\n\n",
        session.student_name,
        appreciation(result.mark(20.0), t)
    ));
    out.push_str(&format!("## {}\n\n", t.sheet_where));
    out.push_str(&format!(
        "| {} | {} | {} |\n|---|---|---|\n",
        t.sheet_theme, t.sheet_success, t.sheet_state
    ));
    for th in result.themes.iter().filter(|th| th.total > 0) {
        let title = full
            .theme(&th.theme_id)
            .map_or_else(|| th.theme_id.clone(), |x| x.title.clone());
        let state = match th.mastery() {
            Mastery::Acquired => t.acquired,
            Mastery::Fragile => t.fragile,
            Mastery::ToRelearn | Mastery::NotAssessed => t.to_relearn,
        };
        out.push_str(&format!(
            "| {title} | {} % | {state} |\n",
            (th.rate() * 100.0).round()
        ));
    }
    let mistakes = indices(session, result, Filter::Mistakes);
    out.push_str(&format!(
        "\n## {} ({})\n\n",
        t.sheet_mistakes,
        mistakes.len()
    ));
    if mistakes.is_empty() {
        out.push_str(&format!("{}\n\n", t.sheet_none));
    }
    for i in mistakes {
        let item = &session.items[i];
        let Some(q) = full.question(&item.question_id) else {
            continue;
        };
        out.push_str(&format!("### {} {}\n\n", t.question, i + 1));
        for block in &q.body {
            match block {
                Block::Text(text) => out.push_str(&format!("{}\n\n", text.trim())),
                Block::Code(code) => {
                    out.push_str(&format!("```{}\n{}\n```\n\n", code.language, code.source))
                }
                Block::Media(media) => out.push_str(&format!("*({})*\n\n", media.alt)),
            }
        }
        let chosen: Vec<_> = item
            .option_order
            .iter()
            .filter(|id| item.selected.contains(id))
            .filter_map(|id| q.option(id))
            .collect();
        out.push_str(&format!("**{}**", t.your_answer_title));
        if chosen.is_empty() {
            out.push_str(&format!(" {}\n\n", t.no_answer));
        } else {
            out.push_str("\n\n");
            for o in chosen {
                out.push_str(&format!("- {}\n", o.text));
                if !o.explanation.trim().is_empty() {
                    out.push_str(&format!("  > {}\n", o.explanation.trim()));
                }
            }
            out.push('\n');
        }
        out.push_str(&format!("**{}**\n\n", t.right_answer_title));
        for o in q.options.iter().filter(|o| o.is_correct) {
            out.push_str(&format!("- {}\n", o.text));
            if !o.explanation.trim().is_empty() {
                out.push_str(&format!("  > {}\n", o.explanation.trim()));
            }
        }
        out.push('\n');
        if !q.rule.is_empty() {
            out.push_str(&format!("**{}** {}\n\n", t.rule, q.rule));
        }
        if !q.course_ref.is_empty() {
            out.push_str(&format!("*{} {}*\n\n", t.review, q.course_ref));
        }
    }
    let weak = weak_themes(full, result);
    if !weak.is_empty() {
        out.push_str(&format!("## {}\n\n", t.sheet_start));
        for (k, th) in weak.iter().enumerate() {
            out.push_str(&format!("{}. {}", k + 1, th.title));
            if !th.course_ref.is_empty() {
                out.push_str(&format!(" ({})", th.course_ref));
            }
            out.push('\n');
        }
    }
    out
}

fn status_text(status: Status, t: &Texts) -> (&'static str, &'static str) {
    match status {
        Status::Correct => (t.correct, "ok"),
        Status::Partial => (t.partial, "part"),
        Status::Wrong => (t.wrong, "ko"),
        Status::Unanswered => (t.unanswered, "blank"),
    }
}

/// One question of the correction: what was asked, the options marked, the student's answer
/// and the right one with their explanations, the rule and where to revise.
pub fn question_review(
    full: &Paper,
    session: &Session,
    result: &GradeResult,
    index: usize,
    t: &'static Texts,
    content: Content<'_>,
) -> Option<Element> {
    let item = session.items.get(index)?;
    let question = full.question(&item.question_id)?;
    let (status_label, class) = status_text(status(result, &item.question_id), t);
    let theme = full
        .theme(&question.theme_id)
        .map(|th| th.title.clone())
        .unwrap_or_default();
    let level = level_number(question.level);
    let expected = question.effective_seconds();
    let spent = item.seconds_spent;
    let time_line = crate::texts::fill(
        t.time_line,
        &[
            ("spent", format_duration(spent)),
            ("expected", format_duration(expected)),
        ],
    );
    let note = match pace(spent, expected) {
        Some(Pace::Quick) => Some(t.time_quick),
        Some(Pace::Long) => Some(t.time_long),
        None => None,
    };
    let f = content.formulas;
    let options: Vec<(String, String, bool, bool)> = item
        .option_order
        .iter()
        .filter_map(|id| {
            let o = question.option(id)?;
            Some((
                id.clone(),
                markdown::inline(&o.text, f),
                o.is_correct,
                item.selected.contains(id),
            ))
        })
        .collect();
    let explained = |o: &mcq_model::AnswerOption| {
        (
            markdown::inline(&o.text, f),
            markdown::block(&o.explanation, f),
        )
    };
    let chosen: Vec<(String, String)> = item
        .option_order
        .iter()
        .filter(|id| item.selected.contains(id))
        .filter_map(|id| question.option(id))
        .map(explained)
        .collect();
    let right: Vec<(String, String)> = item
        .option_order
        .iter()
        .filter_map(|id| question.option(id))
        .filter(|o| o.is_correct)
        .map(explained)
        .collect();
    let rule = question.rule.clone();
    let course = question.course_ref.clone();
    Some(rsx! {
        article { key: "{index}", class: "review-item {class}",
            h2 { "{t.question} {index + 1}" span { class: "status-badge {class}", "{status_label}" } }
            p { class: "context muted",
                if !theme.is_empty() { "{theme} · " }
                if level > 0 { "{t.level} {level} · " }
                span { class: if note.is_some() { "time flagged" } else { "time" }, "{time_line}" }
                if let Some(note) = note { " ({note})" }
            }
            {render::body(question, content)}
            ul { class: "review-options",
                for (id, text, correct, picked) in options {
                    li { key: "{id}",
                        class: match (correct, picked) { (true, _) => "right", (false, true) => "wrong", _ => "" },
                        span { class: "mark", "aria-hidden": "true", match (correct, picked) { (true, _) => "✓", (false, true) => "✗", _ => "·" } }
                        span { dangerous_inner_html: "{text}" }
                        if correct { em { class: "tag", " ({t.right_answer_tag})" } }
                        if picked { em { class: "chosen", " ({t.your_answer})" } }
                    }
                }
            }
            div { class: "answer-block",
                h3 { "{t.your_answer_title}" }
                if chosen.is_empty() { p { em { "{t.no_answer}" } } }
                for (k, (text, why)) in chosen.into_iter().enumerate() {
                    div { key: "{k}", p { span { dangerous_inner_html: "{text}" } }
                        if !why.is_empty() { div { class: "explanation", dangerous_inner_html: "{why}" } }
                    }
                }
            }
            div { class: "answer-block right",
                h3 { "{t.right_answer_title}" }
                for (k, (text, why)) in right.into_iter().enumerate() {
                    div { key: "{k}", p { span { dangerous_inner_html: "{text}" } }
                        if !why.is_empty() { div { class: "explanation", dangerous_inner_html: "{why}" } }
                    }
                }
            }
            if !rule.is_empty() { p { class: "rule", strong { "{t.rule}" } " {rule}" } }
            if !course.is_empty() { p { class: "muted", strong { "{t.review}" } " {course}" } }
        }
    })
}

/// The result and the guided correction.
pub fn review_screen(
    full: &Paper,
    session: &Session,
    result: &GradeResult,
    state: &ReviewState,
    t: &'static Texts,
    content: Content<'_>,
    on: EventHandler<ReviewAction>,
) -> Element {
    let mark = mark_text(result, t);
    let word = appreciation(result.mark(20.0), t);
    let minutes = crate::texts::fill(
        t.minutes_taken,
        &[("m", (session.elapsed_seconds / 60).to_string())],
    );
    let overtime = (result.overtime_seconds > 0).then(|| {
        crate::texts::fill(
            t.overtime_note,
            &[("m", result.overtime_seconds.div_ceil(60).to_string())],
        )
    });
    let themes: Vec<(String, u32, &'static str, &'static str)> = result
        .themes
        .iter()
        .filter(|th| th.total > 0)
        .map(|th| {
            let title = full
                .theme(&th.theme_id)
                .map_or_else(|| th.theme_id.clone(), |x| x.title.clone());
            let (label, class) = match th.mastery() {
                Mastery::Acquired => (t.acquired, "acquired"),
                Mastery::Fragile => (t.fragile, "fragile"),
                Mastery::ToRelearn | Mastery::NotAssessed => (t.to_relearn, "relearn"),
            };
            (title, (th.rate() * 100.0).round() as u32, label, class)
        })
        .collect();
    let weak = weak_themes(full, result);
    let current = state.current(session, result);
    let list = indices(session, result, state.filter);
    let position = current
        .and_then(|c| list.iter().position(|&i| i == c))
        .unwrap_or(0);
    let count = list.len();
    let segments: Vec<(usize, &'static str, bool, String)> = session
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let (word, class) = status_text(status(result, &item.question_id), t);
            let name = format!("{} {}, {word}", t.question, i + 1);
            (i, class, Some(i) == current, name)
        })
        .collect();
    let question = current.and_then(|i| question_review(full, session, result, i, t, content));
    let understood = current.is_some_and(|i| state.understood.contains(&i));
    let filters = [
        (Filter::Mistakes, t.filter_mistakes),
        (Filter::All, t.filter_all),
        (Filter::Right, t.filter_right),
    ];
    // Printed, the whole correction: every question one after the other.
    let printed: Vec<Element> = (0..session.items.len())
        .filter_map(|i| question_review(full, session, result, i, t, content))
        .collect();
    rsx! {
        section { class: "review", lang: (!full.lang.is_empty()).then(|| full.lang.clone()),
            h1 { class: "sr-only", "{full.title}" }
            div { class: "card score",
                p { class: "big score-mark", "{mark} / 20" }
                p { class: "appreciation", "{word}" }
                p { class: "counts",
                    span { class: "ok", "{result.correct} {t.correct}" } " · "
                    if result.partial > 0 { span { class: "part", "{result.partial} {t.partial}" } " · " }
                    span { class: "ko", "{result.wrong} {t.wrong}" } " · "
                    span { "{result.unanswered} {t.unanswered}" } " · "
                    span { "{minutes}" }
                    if let Some(over) = overtime { " ({over})" }
                }
                h2 { "{t.mastery}" }
                ul { class: "theme-bars",
                    for (k, (title, rate, label, class)) in themes.into_iter().enumerate() {
                        li { key: "{k}",
                            span { class: "theme-name", "{title}" }
                            span { class: "bar", span { class: "fill {class}", style: "width: {rate}%" } }
                            span { class: "theme-rate", "{rate} % ({label})" }
                        }
                    }
                }
                if !weak.is_empty() {
                    h2 { "{t.to_review_title}" }
                    ol { class: "weak-themes",
                        for (k, th) in weak.into_iter().enumerate() {
                            li { key: "{k}", strong { "{th.title}" }
                                if !th.course_ref.is_empty() { " → {th.course_ref}" }
                            }
                        }
                    }
                }
                div { class: "actions no-print",
                    button { onclick: move |_| on.call(ReviewAction::DownloadSheet), "{t.download_sheet}" }
                    button { onclick: move |_| on.call(ReviewAction::Print), "{t.print_correction}" }
                }
            }
            div { class: "review-guide no-print",
                div { class: "segmented", role: "group", "aria-label": "{t.show}",
                    for (filter, label) in filters {
                        button { key: "{label}", "aria-pressed": "{state.filter == filter}",
                            onclick: move |_| on.call(ReviewAction::Filter(filter)), "{label}" }
                    }
                }
                nav { class: "segments", "aria-label": "{t.overview}",
                    for (i, class, here, name) in segments {
                        button { key: "{i}", class: if here { "segment {class} current" } else { "segment {class}" },
                            "aria-current": if here { "step" } else { "false" },
                            title: "{name}",
                            "aria-label": "{name}",
                            onclick: move |_| on.call(ReviewAction::Go(i)),
                        }
                    }
                }
                if let Some(question) = question {
                    p { class: "muted position", "{position + 1} / {count}" }
                    {question}
                    div { class: "nav",
                        button { disabled: position == 0, onclick: move |_| on.call(ReviewAction::Previous), "← {t.previous}" }
                        button { class: if understood { "understood on" } else { "understood" }, "aria-pressed": "{understood}",
                            onclick: move |_| on.call(ReviewAction::Understood), "{t.understood}" }
                        button { class: "primary", disabled: position + 1 >= count, onclick: move |_| on.call(ReviewAction::Next), "{t.next} →" }
                    }
                    p { class: "muted", "{t.review_keys}" }
                } else {
                    p { class: "ok-note", "{t.filter_empty}" }
                }
            }
            div { class: "print-only",
                for (k, element) in printed.into_iter().enumerate() { div { key: "{k}", {element} } }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texts::Lang;
    use mcq_engine::SessionItem;
    use mcq_grade::{Outcome, ThemeResult};

    fn session(n: usize) -> Session {
        Session {
            items: (0..n)
                .map(|i| SessionItem {
                    question_id: format!("q{i}"),
                    ..SessionItem::default()
                })
                .collect(),
            ..Session::default()
        }
    }

    fn result(statuses: &[Status]) -> GradeResult {
        GradeResult {
            outcomes: statuses
                .iter()
                .enumerate()
                .map(|(i, s)| Outcome {
                    question_id: format!("q{i}"),
                    status: *s,
                    credit: f32::from(u8::from(*s == Status::Correct)),
                    seconds_spent: 0,
                })
                .collect(),
            ..GradeResult::default()
        }
    }

    #[test]
    fn mistakes_come_first_and_filters_walk_their_questions() {
        use Status::*;
        let s = session(5);
        let r = result(&[Correct, Wrong, Unanswered, Correct, Partial]);
        assert_eq!(indices(&s, &r, Filter::Mistakes), vec![1, 2, 4]);
        assert_eq!(indices(&s, &r, Filter::Right), vec![0, 3]);
        let mut state = ReviewState::default();
        assert_eq!(state.current(&s, &r), Some(1));
        state.apply(ReviewAction::Understood, &s, &r);
        assert_eq!(state.current(&s, &r), Some(2));
        assert!(state.understood.contains(&1));
        state.apply(ReviewAction::Next, &s, &r);
        state.apply(ReviewAction::Next, &s, &r);
        assert_eq!(state.current(&s, &r), Some(4), "stops at the last");
        // A right answer clicked on the strip opens under "All".
        state.apply(ReviewAction::Go(3), &s, &r);
        assert_eq!(
            (state.filter, state.current(&s, &r)),
            (Filter::All, Some(3))
        );
        state.apply(ReviewAction::Filter(Filter::Right), &s, &r);
        assert_eq!(state.current(&s, &r), Some(0));
        let none = result(&[Correct; 5]);
        let state = ReviewState::default();
        assert_eq!(state.current(&s, &none), None, "no mistake to show");
    }

    #[test]
    fn words_and_paces() {
        let t = Lang::Fr.texts();
        assert_eq!(appreciation(16.0, t), "Solide");
        assert_eq!(appreciation(12.9, t), "Des bases à consolider");
        assert_eq!(appreciation(5.9, t), "À reprendre depuis le début");
        assert_eq!(pace(5, 30), Some(Pace::Quick));
        assert_eq!(pace(10, 30), None);
        assert_eq!(pace(61, 30), Some(Pace::Long));
        assert_eq!(pace(0, 30), None, "never visited");
    }

    #[test]
    fn weak_themes_need_three_questions_and_come_weakest_first() {
        let mut paper = Paper::new();
        for (id, course) in [("a", "Ch. 1"), ("b", ""), ("c", "Ch. 3")] {
            paper.themes.push(mcq_model::Theme {
                id: id.into(),
                title: id.to_uppercase(),
                course_ref: course.into(),
                ..mcq_model::Theme::default()
            });
        }
        let theme = |id: &str, total, credit| ThemeResult {
            theme_id: id.into(),
            total,
            credit,
            ..ThemeResult::default()
        };
        let r = GradeResult {
            themes: vec![theme("a", 4, 2.0), theme("b", 2, 0.0), theme("c", 3, 0.0)],
            ..GradeResult::default()
        };
        let weak = weak_themes(&paper, &r);
        let titles: Vec<&str> = weak.iter().map(|w| w.title.as_str()).collect();
        assert_eq!(titles, vec!["C", "A"]);
        assert_eq!(weak[0].course_ref, "Ch. 3");
    }
}
