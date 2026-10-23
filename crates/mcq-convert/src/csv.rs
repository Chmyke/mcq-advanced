//! CSV and spreadsheet exports: one question per row, columns found by their header.
//!
//! Headers are matched loosely (case, accents, spaces and punctuation ignored), in English and
//! French, so the templates of common quiz tools come in as they are. The separator is `;`, `,`
//! or a tab, whichever the header uses most. `docs/exchange.md` lists the recognised headers.

use mcq_model::{Level, Location, Question};

use crate::build::{Builder, Draft};
use crate::text;
use crate::{ImportOptions, Imported};

/// Splits CSV text into rows of fields (RFC 4180 quotes, `""` inside quotes), with the 1-based
/// line where each row starts.
fn rows(source: &str, separator: char) -> Vec<(usize, Vec<String>)> {
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut line = 1;
    let mut row_line = 1;
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                if c == '\n' {
                    line += 1;
                }
                field.push(c);
            }
            continue;
        }
        match c {
            '"' if field.trim().is_empty() => {
                field.clear();
                quoted = true;
            }
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push((row_line, std::mem::take(&mut row)));
                line += 1;
                row_line = line;
            }
            c if c == separator => row.push(std::mem::take(&mut field)),
            c => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push((row_line, row));
    }
    rows.retain(|(_, r)| r.iter().any(|f| !f.trim().is_empty()));
    rows
}

fn guess_separator(source: &str) -> char {
    let header = source.lines().next().unwrap_or_default();
    [';', '\t', ',']
        .into_iter()
        .max_by_key(|&s| header.matches(s).count())
        .unwrap_or(';')
}

/// Lowercase ASCII letters and digits only.
fn key(header: &str) -> String {
    mcq_edit::slug(header).replace(['-', '_'], "")
}

#[derive(Default)]
struct Columns {
    question: Option<usize>,
    options: Vec<usize>,
    correct: Option<usize>,
    theme: Option<usize>,
    id: Option<usize>,
    explanation: Option<usize>,
    review: Option<usize>,
    level: Option<usize>,
    time: Option<usize>,
    kind: Option<usize>,
}

const QUESTION: [&str; 7] = [
    "question",
    "questiontext",
    "enonce",
    "statement",
    "intitule",
    "texte",
    "text",
];
const CORRECT: [&str; 11] = [
    "correct",
    "correctanswer",
    "correctanswers",
    "answer",
    "bonnereponse",
    "bonnesreponses",
    "reponsecorrecte",
    "reponsescorrectes",
    "solution",
    "key",
    "corrige",
];
const THEME: [&str; 5] = ["theme", "category", "categorie", "topic", "chapitre"];
const EXPLANATION: [&str; 7] = [
    "explanation",
    "explication",
    "feedback",
    "rule",
    "regle",
    "answerexplanation",
    "justification",
];
const REVIEW: [&str; 3] = ["review", "revoir", "course"];
const LEVEL: [&str; 2] = ["level", "niveau"];
const TIME: [&str; 6] = [
    "time",
    "timelimit",
    "timeinseconds",
    "temps",
    "duree",
    "seconds",
];
const KIND: [&str; 3] = ["questiontype", "type", "typedequestion"];
const OPTION_WORDS: [&str; 6] = [
    "option",
    "answer",
    "reponse",
    "choice",
    "choix",
    "proposition",
];

impl Columns {
    fn find(header: &[String]) -> Self {
        let mut columns = Columns::default();
        for (k, name) in header.iter().enumerate() {
            let name = key(name);
            let slot = if QUESTION.contains(&name.as_str()) {
                &mut columns.question
            } else if CORRECT.contains(&name.as_str()) {
                &mut columns.correct
            } else if THEME.contains(&name.as_str()) {
                &mut columns.theme
            } else if name == "id" {
                &mut columns.id
            } else if EXPLANATION.contains(&name.as_str()) {
                &mut columns.explanation
            } else if REVIEW.contains(&name.as_str()) {
                &mut columns.review
            } else if LEVEL.contains(&name.as_str()) {
                &mut columns.level
            } else if TIME.contains(&name.as_str()) || name.starts_with("timelimit") {
                &mut columns.time
            } else if KIND.contains(&name.as_str()) {
                &mut columns.kind
            } else {
                if is_option_header(&name) {
                    columns.options.push(k);
                }
                continue;
            };
            slot.get_or_insert(k);
        }
        columns
    }
}

/// `option1`, `answer 2`, `reponseC`, `A`...`H`.
fn is_option_header(name: &str) -> bool {
    if name.len() == 1 && name.as_bytes()[0].is_ascii_lowercase() && name <= "h" {
        return true;
    }
    OPTION_WORDS.iter().any(|word| {
        name.strip_prefix(word).is_some_and(|rest| {
            !rest.is_empty()
                && (rest.chars().all(|c| c.is_ascii_digit())
                    || (rest.len() == 1 && (b'a'..=b'h').contains(&rest.as_bytes()[0])))
        })
    })
}

pub fn import(source: &str, options: &ImportOptions) -> Imported {
    let mut b = Builder::new(options);
    let source = source.trim_start_matches('\u{feff}');
    let all = rows(source, guess_separator(source));
    let Some(((_, header), data)) = all.split_first() else {
        return b.unreadable("The file is empty.".into(), "Le fichier est vide.".into());
    };
    let columns = Columns::find(header);
    let Some(question_column) = columns.question else {
        return b.unreadable(
            "No `question` column: the first row must name the columns (see the import guide)."
                .into(),
            "Pas de colonne `question` : la première ligne doit nommer les colonnes (voir le \
             guide d'import)."
                .into(),
        );
    };
    if columns.options.len() < 2 {
        return b.unreadable(
            "Fewer than two option columns: name them `option 1`, `option 2`... or `A`, `B`..."
                .into(),
            "Moins de deux colonnes de réponses : nommez-les `option 1`, `option 2`... ou `A`, \
             `B`..."
                .into(),
        );
    }
    let cell = |row: &[String], column: Option<usize>| -> String {
        column
            .and_then(|k| row.get(k))
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    for (line, row) in data {
        let at = Location::Line(*line);
        let statement = cell(row, Some(question_column));
        let label: String = text::one_line(&statement).chars().take(40).collect();
        let kind = key(&cell(row, columns.kind));
        if !kind.is_empty() && !supported_kind(&kind) {
            b.skip(
                at,
                format!("Line {line}, `{label}`: the type `{kind}` is not imported."),
                format!("Ligne {line}, `{label}` : le type `{kind}` n'est pas importé."),
            );
            continue;
        }
        let option_texts: Vec<String> = columns
            .options
            .iter()
            .map(|&k| cell(row, Some(k)))
            .filter(|t| !t.is_empty())
            .collect();
        if statement.is_empty() || option_texts.len() < 2 {
            b.skip(
                at,
                format!("Line {line}: a question and at least two options are needed."),
                format!("Ligne {line} : il faut une question et au moins deux réponses."),
            );
            continue;
        }
        let correct = correct_indexes(&cell(row, columns.correct), &option_texts);
        if correct.is_empty() {
            b.skip(
                at,
                format!("Line {line}, `{label}`: the correct answer column names no option."),
                format!(
                    "Ligne {line}, `{label}` : la colonne de la bonne réponse ne désigne aucune \
                     réponse."
                ),
            );
            continue;
        }
        let mut question = Question {
            body: text::plain_blocks(&statement),
            rule: text::plain_inline(&cell(row, columns.explanation)),
            course_ref: text::plain_inline(&cell(row, columns.review)),
            ..Question::default()
        };
        if let Ok(n) = cell(row, columns.level).parse() {
            question.level = Level::from_number(n).unwrap_or_default();
        }
        if let Ok(n) = cell(row, columns.time).trim_end_matches('s').trim().parse() {
            question.estimated_seconds = n;
        }
        let mut draft = Draft {
            id_hint: cell(row, columns.id),
            question,
        };
        for (k, option) in option_texts.iter().enumerate() {
            draft.option(
                text::plain_inline(option),
                correct.contains(&k),
                String::new(),
            );
        }
        let theme_title = cell(row, columns.theme);
        let theme = if theme_title.is_empty() {
            b.default_theme()
        } else {
            b.theme(&theme_title, "")
        };
        b.question(&theme, draft);
    }
    b.finish()
}

fn supported_kind(kind: &str) -> bool {
    [
        "multiple", "choice", "checkbox", "qcm", "single", "unique", "vrai", "true", "mcq",
    ]
    .iter()
    .any(|word| kind.contains(word))
}

/// The options named by the correct answer cell: letters (`B`, `A,C`), numbers from 1
/// (`2`, `1;3`), or the exact text of an option.
fn correct_indexes(cell: &str, options: &[String]) -> Vec<usize> {
    if let Some(k) = options.iter().position(|o| o.eq_ignore_ascii_case(cell)) {
        return vec![k];
    }
    let parts: Vec<&str> = cell
        .split([',', ';', '|', ' ', '/'])
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let mut found = Vec::new();
    for part in parts {
        let index = if let Ok(n) = part.parse::<usize>() {
            n.checked_sub(1)
        } else if part.len() == 1 && part.as_bytes()[0].is_ascii_alphabetic() {
            Some((part.as_bytes()[0].to_ascii_uppercase() - b'A') as usize)
        } else {
            options.iter().position(|o| o.eq_ignore_ascii_case(part))
        };
        match index {
            Some(k) if k < options.len() => {
                if !found.contains(&k) {
                    found.push(k);
                }
            }
            _ => return Vec::new(),
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_quotes_and_separators() {
        let r = rows("a;\"b;c\";\"say \"\"hi\"\"\"\n1;2\n", ';');
        assert_eq!(r[0].1, vec!["a", "b;c", "say \"hi\""]);
        assert_eq!(r[1], (2, vec!["1".to_string(), "2".to_string()]));
        assert_eq!(guess_separator("a,b,c\n"), ',');
        assert_eq!(guess_separator("a\tb\tc\n"), '\t');
    }

    #[test]
    fn names_correct_options_every_way() {
        let options: Vec<String> = ["Red", "Green", "Blue"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(correct_indexes("B", &options), vec![1]);
        assert_eq!(correct_indexes("a, c", &options), vec![0, 2]);
        assert_eq!(correct_indexes("3", &options), vec![2]);
        assert_eq!(correct_indexes("green", &options), vec![1]);
        assert_eq!(correct_indexes("D", &options), Vec::<usize>::new());
    }

    #[test]
    fn recognises_option_headers() {
        for h in ["Option 1", "Answer 4", "Réponse C", "A", "choice2"] {
            assert!(is_option_header(&key(h)), "{h}");
        }
        for h in ["Question", "Correct Answer", "Answers", "I"] {
            assert!(!is_option_header(&key(h)), "{h}");
        }
    }
}
