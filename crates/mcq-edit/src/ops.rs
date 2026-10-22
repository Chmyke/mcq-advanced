//! Editing operations on a paper, as the visual editor applies them. Each keeps the paper
//! writable: ids unique, option ids positional, questions grouped by theme.

use mcq_model::{AnswerOption, Paper, Question, Theme};

/// What the editor offers as the question type; the format itself only knows options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// One correct option: students see radio buttons.
    Single,
    /// Several correct options: students see check boxes.
    Multiple,
    /// Two options, true and false.
    TrueFalse,
}

const TRUE_WORDS: [&str; 2] = ["true", "vrai"];
const FALSE_WORDS: [&str; 2] = ["false", "faux"];

/// The type a question shows in the editor, read from its options.
pub fn kind(question: &Question) -> Kind {
    let word = |o: &AnswerOption| o.text.trim().to_lowercase();
    if let [a, b] = question.options.as_slice()
        && TRUE_WORDS.contains(&word(a).as_str())
        && FALSE_WORDS.contains(&word(b).as_str())
    {
        return Kind::TrueFalse;
    }
    if question.correct_count() > 1 {
        Kind::Multiple
    } else {
        Kind::Single
    }
}

/// Changes the type. Single keeps the first correct option; true or false replaces the options.
pub fn set_kind(question: &mut Question, kind: Kind, lang: &str) {
    match kind {
        Kind::Single => {
            let mut seen = false;
            for option in &mut question.options {
                if option.is_correct && seen {
                    option.is_correct = false;
                }
                seen |= option.is_correct;
            }
        }
        Kind::Multiple => {}
        Kind::TrueFalse => {
            if self::kind(question) != Kind::TrueFalse {
                let (yes, no) = if lang.starts_with("fr") {
                    ("Vrai", "Faux")
                } else {
                    ("True", "False")
                };
                question.options = [yes, no]
                    .into_iter()
                    .map(|text| AnswerOption {
                        text: text.to_string(),
                        ..AnswerOption::default()
                    })
                    .collect();
            }
        }
    }
    question.declared_answers = 0;
    renumber_options(question);
}

/// Ticks or unticks an option as correct. With `single`, ticking one unticks the others.
pub fn toggle_correct(question: &mut Question, index: usize, single: bool) {
    let Some(was) = question.options.get(index).map(|o| o.is_correct) else {
        return;
    };
    if single {
        for option in &mut question.options {
            option.is_correct = false;
        }
    }
    question.options[index].is_correct = !was;
    // The count announced to students follows the options in a paper with its answers.
    question.declared_answers = 0;
}

pub fn add_option(question: &mut Question) {
    question.options.push(AnswerOption::default());
    renumber_options(question);
}

pub fn remove_option(question: &mut Question, index: usize) {
    if index < question.options.len() {
        question.options.remove(index);
        renumber_options(question);
    }
}

pub fn move_option(question: &mut Question, index: usize, up: bool) -> bool {
    let moved = move_item(&mut question.options, index, up);
    renumber_options(question);
    moved
}

/// Option ids are positional: `<question id>-o1`, `-o2`...
pub fn renumber_options(question: &mut Question) {
    for (k, option) in question.options.iter_mut().enumerate() {
        option.id = format!("{}-o{}", question.id, k + 1);
    }
}

/// Swaps an item with its neighbour; false at either end.
pub fn move_item<T>(items: &mut [T], index: usize, up: bool) -> bool {
    let other = if up {
        index.checked_sub(1)
    } else {
        Some(index + 1)
    };
    match other {
        Some(other) if index < items.len() && other < items.len() => {
            items.swap(index, other);
            true
        }
        _ => false,
    }
}

/// An unused id like `<theme id>-<n>`, `n` above every number already used with that prefix.
pub fn next_question_id(paper: &Paper, theme_id: &str) -> String {
    let prefix = if theme_id.is_empty() { "q" } else { theme_id };
    let highest = paper
        .questions
        .iter()
        .filter_map(|q| {
            q.id.strip_prefix(prefix)?
                .strip_prefix('-')?
                .parse::<u32>()
                .ok()
        })
        .max()
        .unwrap_or(0);
    let mut n = highest + 1;
    loop {
        let id = format!("{prefix}-{n}");
        if paper.question(&id).is_none() {
            return id;
        }
        n += 1;
    }
}

/// Adds an empty question with four options at the end of a theme and returns its id.
pub fn add_question(paper: &mut Paper, theme_id: &str) -> String {
    let mut question = Question {
        id: next_question_id(paper, theme_id),
        theme_id: theme_id.to_string(),
        options: vec![AnswerOption::default(); 4],
        ..Question::default()
    };
    renumber_options(&mut question);
    let id = question.id.clone();
    insert_in_theme(paper, question);
    id
}

/// Copies a question right after itself, under a new id, and returns that id.
pub fn duplicate_question(paper: &mut Paper, id: &str) -> Option<String> {
    let index = paper.questions.iter().position(|q| q.id == id)?;
    let mut copy = paper.questions[index].clone();
    copy.id = next_question_id(paper, &copy.theme_id);
    copy.line = 0;
    renumber_options(&mut copy);
    let new_id = copy.id.clone();
    paper.questions.insert(index + 1, copy);
    Some(new_id)
}

pub fn remove_question(paper: &mut Paper, id: &str) -> bool {
    let before = paper.questions.len();
    paper.questions.retain(|q| q.id != id);
    paper.questions.len() != before
}

/// Moves a question before or after its neighbour in the same theme.
pub fn move_question(paper: &mut Paper, id: &str, up: bool) -> bool {
    let Some(index) = paper.questions.iter().position(|q| q.id == id) else {
        return false;
    };
    let theme = paper.questions[index].theme_id.clone();
    let neighbour = if up {
        paper.questions[..index]
            .iter()
            .rposition(|q| q.theme_id == theme)
    } else {
        paper.questions[index + 1..]
            .iter()
            .position(|q| q.theme_id == theme)
            .map(|k| index + 1 + k)
    };
    match neighbour {
        Some(other) => {
            paper.questions.swap(index, other);
            true
        }
        None => false,
    }
}

/// Moves a question to the end of another theme.
pub fn set_question_theme(paper: &mut Paper, id: &str, theme_id: &str) -> bool {
    if paper.theme(theme_id).is_none() {
        return false;
    }
    let Some(index) = paper.questions.iter().position(|q| q.id == id) else {
        return false;
    };
    let mut question = paper.questions.remove(index);
    question.theme_id = theme_id.to_string();
    insert_in_theme(paper, question);
    true
}

fn insert_in_theme(paper: &mut Paper, question: Question) {
    let at = paper
        .questions
        .iter()
        .rposition(|q| q.theme_id == question.theme_id)
        .map_or(paper.questions.len(), |k| k + 1);
    paper.questions.insert(at, question);
}

/// Why an id cannot be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdError {
    Empty,
    /// Spaces and `|` would break the `## Q` header.
    BadCharacter,
    Taken,
}

/// Ids are written in headers: no space, no `|`.
pub fn check_id(id: &str) -> Result<(), IdError> {
    if id.is_empty() {
        Err(IdError::Empty)
    } else if id.chars().any(|c| c.is_whitespace() || c == '|') {
        Err(IdError::BadCharacter)
    } else {
        Ok(())
    }
}

/// Renames a question; its option ids follow.
pub fn rename_question(paper: &mut Paper, id: &str, new_id: &str) -> Result<(), IdError> {
    check_id(new_id)?;
    if new_id != id && paper.question(new_id).is_some() {
        return Err(IdError::Taken);
    }
    if let Some(question) = paper.questions.iter_mut().find(|q| q.id == id) {
        question.id = new_id.to_string();
        renumber_options(question);
    }
    Ok(())
}

/// Adds a theme at the end and returns its id, made from the title.
pub fn add_theme(paper: &mut Paper, title: &str) -> String {
    let base = match crate::slug(title) {
        s if s.is_empty() => "theme".to_string(),
        s => s,
    };
    let mut id = base.clone();
    let mut n = 2;
    while paper.theme(&id).is_some() {
        id = format!("{base}-{n}");
        n += 1;
    }
    paper.themes.push(Theme {
        id: id.clone(),
        title: title.trim().to_string(),
        ..Theme::default()
    });
    id
}

/// Removes a theme that holds no question.
pub fn remove_theme(paper: &mut Paper, id: &str) -> bool {
    if paper.questions.iter().any(|q| q.theme_id == id) {
        return false;
    }
    let before = paper.themes.len();
    paper.themes.retain(|t| t.id != id);
    paper.themes.len() != before
}

pub fn move_theme(paper: &mut Paper, id: &str, up: bool) -> bool {
    match paper.themes.iter().position(|t| t.id == id) {
        Some(index) => move_item(&mut paper.themes, index, up),
        None => false,
    }
}
