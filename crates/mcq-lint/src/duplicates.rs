//! Near-duplicate questions, by word overlap (Jaccard index) of statement and code.

use std::collections::BTreeSet;

use mcq_model::{Issue, Location, Paper, Question, Severity};

/// Similarity from which two questions are reported as near duplicates.
pub const DUPLICATE_THRESHOLD: f32 = 0.72;

/// Below this many significant words, short words are kept too.
const MIN_WORDS: usize = 5;

/// Jaccard index of the two questions' word sets, 0 when either is empty.
pub fn similarity(a: &Question, b: &Question) -> f32 {
    let (a, b) = (words(a), words(b));
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let shared = a.intersection(&b).count();
    let union = a.union(&b).count();
    shared as f32 / union as f32
}

/// Pairs inside one paper; the later question carries the warning.
pub fn find_duplicates_within(paper: &Paper) -> Vec<Issue> {
    let mut issues = Vec::new();
    for (j, later) in paper.questions.iter().enumerate() {
        for earlier in &paper.questions[..j] {
            if let Some(issue) = compare(later, earlier, "this paper") {
                issues.push(issue);
            }
        }
    }
    issues
}

/// Questions of `paper` close to a question of `other`, named `other_name` in messages.
pub fn find_duplicates(paper: &Paper, other: &Paper, other_name: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    for question in &paper.questions {
        for candidate in &other.questions {
            if let Some(issue) = compare(question, candidate, other_name) {
                issues.push(issue);
            }
        }
    }
    issues
}

fn compare(question: &Question, candidate: &Question, source: &str) -> Option<Issue> {
    let score = similarity(question, candidate);
    (score >= DUPLICATE_THRESHOLD).then(|| {
        Issue::new(
            Severity::Warning,
            "DUPLICATE_QUESTION",
            Location::Question(question.id.clone()),
            format!(
                "Very close to question `{}` in {source} ({} % shared words).",
                candidate.id,
                (score * 100.0).round() as u32
            ),
        )
    })
}

fn words(question: &Question) -> BTreeSet<String> {
    let statement = question.statement_text();
    let mut set: BTreeSet<String> = tokens(&statement)
        .filter(|t| t.chars().count() >= 4)
        .collect();
    for code in question.code_blocks() {
        set.extend(tokens(&code.source).filter(|t| t.chars().count() >= 2));
    }
    if set.len() < MIN_WORDS {
        set.extend(tokens(&statement));
    }
    set
}

fn tokens(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcq_model::Block;

    fn question(id: &str, text: &str) -> Question {
        Question {
            id: id.into(),
            body: vec![Block::Text(text.into())],
            ..Question::default()
        }
    }

    #[test]
    fn identical_statements_are_duplicates() {
        let a = question("a", "Which colour does a ripe banana usually have?");
        let b = question("b", "Which colour does a ripe banana usually have?");
        assert!((similarity(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn unrelated_statements_are_not() {
        let a = question("a", "Which colour does a ripe banana usually have?");
        let b = question("b", "How many strings does a standard violin carry?");
        assert!(similarity(&a, &b) < DUPLICATE_THRESHOLD);
    }

    #[test]
    fn short_statements_fall_back_to_all_words() {
        let a = question("a", "Is a kiwi a fruit?");
        assert!(words(&a).contains("is"));
    }

    #[test]
    fn within_reports_the_later_question_once() {
        let mut paper = Paper::new();
        paper.questions.push(question(
            "first",
            "Which colour does a ripe banana usually have?",
        ));
        paper.questions.push(question(
            "second",
            "Which colour does a ripe banana usually have?",
        ));
        let issues = find_duplicates_within(&paper);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].location, Location::Question("second".into()));
    }
}
