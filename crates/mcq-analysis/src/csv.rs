//! The three spreadsheets of a class, as the earlier tool wrote them: `notes.csv`, `themes.csv`
//! and `questions.csv`. UTF-8 with a byte order mark, `;` between fields, `\n` between lines,
//! numbers with one decimal, so spreadsheet software opens them as they are.
//!
//! Statements and options stay as written in the paper, Markdown and LaTeX included: a cell
//! cannot draw a formula, and `$\frac{5}{6}$` or `` `7` `` says more than `\frac{5}{6}` or `7`
//! with its delimiters stripped.

use mcq_model::{Level, Paper, TimeExtension};

use crate::{Anomaly, Decision, Finding, Report, TOO_EASY, TOO_HARD};

/// The words of the spreadsheets, in one language.
pub struct Labels {
    /// `,` in French, `.` in English.
    pub decimal: char,
    pub notes_header: [&'static str; 10],
    pub themes_header: [&'static str; 10],
    pub questions_header: [&'static str; 11],
    pub yes: &'static str,
    pub right_row: &'static str,
    pub distractor_row: &'static str,
    /// Followed by the count of students who reported the question as unclear or flawed.
    pub reported: &'static str,
    /// The key may be wrong: see `Finding::KeyDoubt`.
    pub key_doubt: &'static str,
    /// The teacher took the question out of the grading.
    pub neutralized: &'static str,
    /// The teacher replaced the key; the right rows show the new one.
    pub rekeyed: &'static str,
    pub too_hard: &'static str,
    pub too_easy: &'static str,
    /// Followed by the count.
    pub dead_distractors: &'static str,
    pub shared_confusion: &'static str,
    pub never_chosen: &'static str,
    pub other_paper: &'static str,
    pub same_name: &'static str,
    pub unknown_questions: &'static str,
    pub identical: &'static str,
    pub left_window: &'static str,
    pub overtime: &'static str,
    pub fast: &'static str,
    /// Followed by the extension, such as "+1/3".
    pub extra_time: &'static str,
    pub untimed: &'static str,
}

pub const EN: Labels = Labels {
    decimal: '.',
    notes_header: [
        "Name",
        "Mark /20",
        "Correct",
        "Wrong",
        "Blank",
        "Minutes",
        "Overtime (min)",
        "Window exits",
        "Receipt",
        "Anomalies",
    ],
    themes_header: [
        "Theme",
        "Questions",
        "Students",
        "Mean success %",
        "Acquired",
        "Fragile",
        "To relearn",
        "Not acquired %",
        "Reteach",
        "Where to revise",
    ],
    questions_header: [
        "Type",
        "Question",
        "Theme",
        "Level",
        "Success %",
        "Blank",
        "Mean seconds",
        "Discrimination",
        "Point-biserial",
        "Alert",
        "Text",
    ],
    yes: "YES",
    right_row: "  RIGHT",
    distractor_row: "  distractor",
    reported: "REPORTED AS UNCLEAR by",
    key_doubt: "KEY TO CHECK: the strongest students avoid it",
    neutralized: "NEUTRALIZED: out of the marks",
    rekeyed: "KEY CORRECTED: marked with the right rows below",
    too_hard: "missed by almost everyone",
    too_easy: "succeeded by everyone",
    dead_distractors: "dead distractor(s):",
    shared_confusion: "SHARED CONFUSION",
    never_chosen: "never chosen",
    other_paper: "another paper",
    same_name: "same name",
    unknown_questions: "unknown questions",
    identical: "identical to",
    left_window: "left the window",
    overtime: "overtime",
    fast: "unusually fast",
    extra_time: "extra time",
    untimed: "extra time without limit",
};

pub const FR: Labels = Labels {
    decimal: ',',
    notes_header: [
        "Nom",
        "Note /20",
        "Justes",
        "Fausses",
        "Sans réponse",
        "Minutes",
        "Dépassement (min)",
        "Sorties de fenêtre",
        "Code de remise",
        "Anomalies",
    ],
    themes_header: [
        "Thème",
        "Questions",
        "Étudiants",
        "Réussite moyenne %",
        "Acquis",
        "Fragile",
        "À reprendre",
        "Non acquis %",
        "À réenseigner",
        "Où réviser",
    ],
    questions_header: [
        "Type",
        "Question",
        "Thème",
        "Niveau",
        "Réussite %",
        "Sans réponse",
        "Secondes moy.",
        "Discrimination",
        "Point-bisérial",
        "Alerte",
        "Texte",
    ],
    yes: "OUI",
    right_row: "  BONNE",
    distractor_row: "  distracteur",
    reported: "SIGNALÉE COMME DOUTEUSE par",
    key_doubt: "CORRIGÉ À VÉRIFIER : les meilleurs l'évitent",
    neutralized: "NEUTRALISÉE : hors barème",
    rekeyed: "CORRIGÉ RECTIFIÉ : noté selon les lignes BONNE ci-dessous",
    too_hard: "ratée par presque tout le monde",
    too_easy: "réussie par tout le monde",
    dead_distractors: "distracteur(s) mort(s) :",
    shared_confusion: "IDÉE FAUSSE PARTAGÉE",
    never_chosen: "jamais choisi",
    other_paper: "autre sujet",
    same_name: "doublon de nom",
    unknown_questions: "questions inconnues du sujet",
    identical: "identique à",
    left_window: "sorties de fenêtre",
    overtime: "temps dépassé",
    fast: "terminé anormalement vite",
    extra_time: "temps majoré",
    untimed: "temps majoré sans limite",
};

/// Text longer than this is cut, with an ellipsis.
pub const TEXT_CHARS: usize = 90;

/// A field, quoted when it holds a separator, a quote or a line break.
fn field(text: &str) -> String {
    if text.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

fn number(value: f64, labels: &Labels) -> String {
    format!("{value:.1}").replace('.', &labels.decimal.to_string())
}

fn line(out: &mut String, fields: &[String]) {
    let row: Vec<String> = fields.iter().map(|f| field(f)).collect();
    out.push_str(&row.join(";"));
    out.push('\n');
}

fn start() -> String {
    "\u{feff}".to_string()
}

fn short(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > TEXT_CHARS {
        let cut: String = flat.chars().take(TEXT_CHARS).collect();
        format!("{cut}…")
    } else {
        flat
    }
}

/// What the notes sheet says about one student's copy.
fn anomalies_of(report: &Report, name: &str, labels: &Labels) -> String {
    let mut words: Vec<String> = Vec::new();
    for anomaly in &report.anomalies {
        let word = match anomaly {
            Anomaly::SameName { name: n, .. } if n.eq_ignore_ascii_case(name) => {
                labels.same_name.to_string()
            }
            Anomaly::UnknownQuestions { name: n, count } if n == name => {
                format!("{} ({count})", labels.unknown_questions)
            }
            Anomaly::Identical { first, second, .. } if first == name => {
                format!("{} {second}", labels.identical)
            }
            Anomaly::Identical { first, second, .. } if second == name => {
                format!("{} {first}", labels.identical)
            }
            Anomaly::LeftWindow { name: n, times, .. } if n == name => {
                format!("{} ({times})", labels.left_window)
            }
            Anomaly::Overtime { name: n, .. } if n == name => labels.overtime.to_string(),
            Anomaly::Fast { name: n, .. } if n == name => labels.fast.to_string(),
            Anomaly::ExtraTime { name: n, extension } if n == name => match extension {
                TimeExtension::Untimed => labels.untimed.to_string(),
                _ => format!("{} {}", labels.extra_time, extension.label()),
            },
            _ => continue,
        };
        words.push(word);
    }
    words.join(", ")
}

/// One row per student, in name order, then one column per theme.
pub fn notes(paper: &Paper, report: &Report, labels: &Labels) -> String {
    let mut out = start();
    let mut header: Vec<String> = labels.notes_header.iter().map(|h| h.to_string()).collect();
    header.extend(paper.themes.iter().map(|t| format!("{} %", t.title)));
    line(&mut out, &header);
    for s in &report.students {
        let mut row = vec![
            s.name.clone(),
            number(f64::from(s.mark), labels),
            s.grade.correct.to_string(),
            s.grade.wrong.to_string(),
            s.grade.unanswered.to_string(),
            (s.elapsed_seconds / 60).to_string(),
            (s.grade.overtime_seconds / 60).to_string(),
            s.focus_losses.to_string(),
            s.receipt.clone(),
            anomalies_of(report, &s.name, labels),
        ];
        for theme in &paper.themes {
            let rate = s
                .grade
                .themes
                .iter()
                .find(|t| t.theme_id == theme.id && t.total > 0)
                .map(|t| number(f64::from(t.rate()) * 100.0, labels))
                .unwrap_or_default();
            row.push(rate);
        }
        line(&mut out, &row);
    }
    out
}

pub fn themes(report: &Report, labels: &Labels) -> String {
    let mut out = start();
    let header: Vec<String> = labels.themes_header.iter().map(|h| h.to_string()).collect();
    line(&mut out, &header);
    for t in &report.themes {
        let n = t.assessed();
        let not_acquired = if n == 0 {
            0.0
        } else {
            f64::from(t.fragile + t.to_relearn) / f64::from(n) * 100.0
        };
        line(
            &mut out,
            &[
                t.title.clone(),
                t.questions.to_string(),
                n.to_string(),
                number(t.mean_rate * 100.0, labels),
                t.acquired.to_string(),
                t.fragile.to_string(),
                t.to_relearn.to_string(),
                number(not_acquired, labels),
                if t.needs_reteaching() {
                    labels.yes.to_string()
                } else {
                    String::new()
                },
                t.course_ref.clone(),
            ],
        );
    }
    out
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

/// One row per question, then one sub-row per option.
pub fn questions(paper: &Paper, report: &Report, labels: &Labels) -> String {
    let mut out = start();
    let header: Vec<String> = labels
        .questions_header
        .iter()
        .map(|h| h.to_string())
        .collect();
    line(&mut out, &header);
    let optional = |value: Option<f64>| {
        value
            .map(|v| format!("{v:.2}").replace('.', &labels.decimal.to_string()))
            .unwrap_or_default()
    };
    for (q, stats) in paper.questions.iter().zip(&report.questions) {
        let dead = stats
            .options
            .iter()
            .filter(|o| !o.is_correct && o.picked == 0)
            .count();
        let doubtful = report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::KeyDoubt { question, .. } if *question == q.id));
        let alert = if let Some(decision) = report.decision(&q.id) {
            match decision {
                Decision::Neutralize => labels.neutralized,
                Decision::Rekey(_) => labels.rekeyed,
            }
            .to_string()
        } else if doubtful {
            labels.key_doubt.to_string()
        } else if stats.asked > 0 && stats.facility <= TOO_HARD {
            labels.too_hard.to_string()
        } else if stats.facility >= TOO_EASY {
            labels.too_easy.to_string()
        } else if report.items_computed && dead > 0 {
            format!("{} {dead}", labels.dead_distractors)
        } else {
            String::new()
        };
        // The students' own report comes first: the alert above says what the figures add.
        let alert = match (stats.reported, alert.is_empty()) {
            (0, _) => alert,
            (n, true) => format!("{} {n}", labels.reported),
            (n, false) => format!("{} {n} · {alert}", labels.reported),
        };
        line(
            &mut out,
            &[
                "QUESTION".to_string(),
                q.id.clone(),
                q.theme_id.clone(),
                level_number(q.level).to_string(),
                number(stats.facility * 100.0, labels),
                stats.blank.to_string(),
                number(stats.mean_seconds, labels),
                optional(stats.discrimination),
                optional(stats.point_biserial),
                alert,
                short(&q.statement_text()),
            ],
        );
        for (option, o) in q.options.iter().zip(&stats.options) {
            let alert = if o.is_correct {
                String::new()
            } else if report
                .findings
                .iter()
                .any(|f| matches!(f, Finding::Trap { option, .. } if *option == o.id))
            {
                labels.shared_confusion.to_string()
            } else if report.items_computed && o.picked == 0 {
                labels.never_chosen.to_string()
            } else {
                String::new()
            };
            line(
                &mut out,
                &[
                    if o.is_correct {
                        labels.right_row
                    } else {
                        labels.distractor_row
                    }
                    .to_string(),
                    q.id.clone(),
                    String::new(),
                    String::new(),
                    number(o.rate * 100.0, labels),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    alert,
                    short(&option.text),
                ],
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_are_quoted_only_when_needed() {
        assert_eq!(field("plain"), "plain");
        assert_eq!(field("a;b"), "\"a;b\"");
        assert_eq!(field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(field("two\nlines"), "\"two\nlines\"");
    }

    #[test]
    fn numbers_use_the_language_decimal() {
        assert_eq!(number(15.0, &FR), "15,0");
        assert_eq!(number(12.345, &EN), "12.3");
    }

    #[test]
    fn long_text_is_cut() {
        let text = "word ".repeat(40);
        let cut = short(&text);
        assert_eq!(cut.chars().count(), TEXT_CHARS + 1);
        assert!(cut.ends_with('…'));
        assert_eq!(short("a\n  b"), "a b");
    }
}
