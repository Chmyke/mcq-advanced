//! The validator's links: each piece of writing advice opens its section of the website guide, in
//! the interface language; the count of errors and warnings reads as a sentence.

use mcq_model::{Issue, Location, Severity};
use mcq_studio::editor::{GUIDE_CODES, RULES_GUIDE, rule_link};
use mcq_studio::texts::{EN, FR, Lang};

const RULES: &str = include_str!("../../../docs/lint-rules.md");

/// The warning codes of docs/lint-rules.md, except those about reading the file.
fn writing_warnings() -> Vec<&'static str> {
    let mut section = "";
    let mut codes = Vec::new();
    for line in RULES.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            section = title;
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if section != "Reading the file" && cells.len() > 3 && cells[2] == "warning" {
            codes.push(cells[1].trim_matches('`'));
        }
    }
    codes
}

#[test]
fn the_guide_explains_every_piece_of_writing_advice() {
    let mut documented = writing_warnings();
    let mut guided = GUIDE_CODES.to_vec();
    documented.sort_unstable();
    guided.sort_unstable();
    assert_eq!(guided, documented);
}

#[test]
fn advice_opens_its_section_in_the_interface_language() {
    assert_eq!(
        rule_link("LENGTH_BIAS", Lang::Fr),
        "https://mickaelmancini.com/mcq-advanced/guide/#length-bias"
    );
    assert_eq!(
        rule_link("ANSWER_POSITION_BIAS", Lang::En),
        "https://mickaelmancini.com/mcq-advanced/en/guide/#answer-position-bias"
    );
    // Errors and file reading are listed with every other check.
    assert_eq!(rule_link("NO_CORRECT_OPTION", Lang::Fr), RULES_GUIDE);
    assert_eq!(rule_link("UNKNOWN_KEY", Lang::En), RULES_GUIDE);
}

#[test]
fn the_count_reads_as_a_sentence() {
    let issue = |severity, code| Issue::new(severity, code, Location::Paper, "en", "fr");
    let none: Vec<&Issue> = Vec::new();
    assert_eq!(FR.issue_count(&none), "Aucune erreur, aucun avertissement");
    assert_eq!(EN.issue_count(&none), "No error, no warning");
    let issues = [
        issue(Severity::Error, "EMPTY_OPTION"),
        issue(Severity::Warning, "NO_RULE"),
        issue(Severity::Warning, "NO_COURSE_REF"),
        issue(Severity::Info, "MULTIPLE_CORRECT_OPTIONS"),
    ];
    let some: Vec<&Issue> = issues.iter().collect();
    assert_eq!(FR.issue_count(&some), "1 erreur, 2 avertissements");
    assert_eq!(EN.issue_count(&some), "1 error, 2 warnings");
}
