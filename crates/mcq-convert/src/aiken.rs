//! Aiken, the simplest bank format: a question, options `A)` or `A.`, then `ANSWER: B`.
//!
//! Moodle's reader is followed, with two tolerances: the question may span several lines, and
//! `ANSWER: A, C` marks several correct options.

use mcq_model::{Location, Question};

use crate::build::{Builder, Draft};
use crate::text;
use crate::{ImportOptions, Imported};

#[derive(Default)]
struct Pending {
    line: usize,
    text: Vec<String>,
    options: Vec<String>,
}

pub fn import(source: &str, options: &ImportOptions) -> Imported {
    let mut b = Builder::new(options);
    let mut pending = Pending::default();
    let source = source.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    for (index, raw) in source.split(['\n', '\r']).enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let at = Location::Line(index + 1);
        if let Some(answer) = line.strip_prefix("ANSWER:") {
            let question = std::mem::take(&mut pending);
            finish_question(&mut b, question, answer, at);
        } else if let Some(option) = option_text(line) {
            if pending.text.is_empty() {
                b.skip(
                    at,
                    format!("Line {}: an option before any question.", index + 1),
                    format!("Ligne {} : une réponse avant toute question.", index + 1),
                );
            } else {
                pending.options.push(option.to_string());
            }
        } else {
            if !pending.options.is_empty() {
                let line = pending.line;
                b.skip(
                    Location::Line(line),
                    format!("The question of line {line} has no `ANSWER:` line."),
                    format!("La question de la ligne {line} n'a pas de ligne `ANSWER:`."),
                );
                pending = Pending::default();
            }
            if pending.text.is_empty() {
                pending.line = index + 1;
            }
            pending.text.push(line.to_string());
        }
    }
    if !pending.text.is_empty() {
        let line = pending.line;
        b.skip(
            Location::Line(line),
            format!("The question of line {line} has no `ANSWER:` line."),
            format!("La question de la ligne {line} n'a pas de ligne `ANSWER:`."),
        );
    }
    b.finish()
}

/// `A) text` or `A. text`: an uppercase letter, then `)` or `.`.
fn option_text(line: &str) -> Option<&str> {
    let mut chars = line.chars();
    let letter = chars.next()?;
    let mark = chars.next()?;
    (letter.is_ascii_uppercase() && (mark == ')' || mark == '.')).then(|| line[2..].trim())
}

fn finish_question(b: &mut Builder, pending: Pending, answer: &str, at: Location) {
    if pending.options.len() < 2 {
        b.skip(
            at,
            format!(
                "The question of line {} has fewer than two options.",
                pending.line
            ),
            format!(
                "La question de la ligne {} a moins de deux réponses.",
                pending.line
            ),
        );
        return;
    }
    let letters: Vec<usize> = answer
        .chars()
        .filter(char::is_ascii_alphabetic)
        .map(|c| (c.to_ascii_uppercase() as u8 - b'A') as usize)
        .collect();
    if letters.is_empty() || letters.iter().any(|&k| k >= pending.options.len()) {
        b.skip(
            at,
            format!(
                "The question of line {}: `ANSWER:{answer}` names no option of the question.",
                pending.line
            ),
            format!(
                "La question de la ligne {} : `ANSWER:{answer}` ne désigne aucune de ses réponses.",
                pending.line
            ),
        );
        return;
    }
    let mut draft = Draft {
        id_hint: String::new(),
        question: Question {
            body: text::plain_blocks(&pending.text.join("\n")),
            ..Question::default()
        },
    };
    for (k, option) in pending.options.iter().enumerate() {
        draft.option(
            text::plain_inline(option),
            letters.contains(&k),
            String::new(),
        );
    }
    let theme = b.default_theme();
    b.question(&theme, draft);
}
