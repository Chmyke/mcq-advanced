//! The AI prompt offered on the website: the example it shows the AI is a clean paper that follows
//! the prompt's own rules, so an AI imitating it starts from something the validator accepts.

use mcq_model::Severity;

/// Each prompt with the lines around its example.
const PROMPTS: [(&str, &str, &str); 2] = [
    (
        include_str!("../../../examples/ai-prompt/en.txt"),
        "=== EXAMPLE ===\n",
        "=== END OF EXAMPLE ===\n",
    ),
    (
        include_str!("../../../examples/ai-prompt/fr.txt"),
        "=== EXEMPLE ===\n",
        "=== FIN DE L'EXEMPLE ===\n",
    ),
];

fn examples() -> impl Iterator<Item = &'static str> {
    PROMPTS.iter().map(|(prompt, open, close)| {
        let start = prompt.find(open).expect("example opens") + open.len();
        let end = prompt.find(close).expect("example closes");
        &prompt[start..end]
    })
}

#[test]
fn the_example_reads_without_any_warning() {
    for example in examples() {
        let (paper, mut issues) = mcq_format::read(example);
        issues.extend(mcq_lint::lint(&paper));
        issues.retain(|i| i.severity != Severity::Info);
        assert!(issues.is_empty(), "{issues:?}");
        assert!(paper.questions.len() >= 3);
    }
}

#[test]
fn the_example_follows_the_duration_rule_it_teaches() {
    // "Add up the times of all questions, add 20 %, round up to the minute."
    for example in examples() {
        let paper = mcq_format::read(example).0;
        let seconds: u32 = paper.questions.iter().map(|q| q.estimated_seconds).sum();
        assert_eq!(
            paper.duration_minutes,
            (seconds * 12).div_ceil(600),
            "{}",
            paper.title
        );
    }
}

#[test]
fn the_example_is_what_pasting_it_keeps() {
    for example in examples() {
        assert_eq!(mcq_format::paper_in_pasted(example), Some(example));
    }
}

#[test]
fn both_languages_ask_for_the_same_settings() {
    let [en, fr] = PROMPTS.map(|(prompt, _, _)| prompt);
    for (english, french) in [
        ("Number of questions: 12", "Nombre de questions : 12"),
        ("scoring: all-or-nothing", "scoring: all-or-nothing"),
        ("penalty: 0", "penalty: 0"),
        ("[paste your course here]", "[colle ton cours ici]"),
    ] {
        assert!(en.contains(english), "{english}");
        assert!(fr.contains(french), "{french}");
    }
}
