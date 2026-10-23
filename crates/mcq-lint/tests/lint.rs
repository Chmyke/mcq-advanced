//! One scenario per rule: a clean fictional paper, damaged on purpose.

use mcq_lint::{find_duplicates, find_duplicates_within, has_errors, lint};
use mcq_model::{
    AnswerOption, Block, Level, Location, Media, MediaKind, Paper, Question, Severity, Theme,
};

/// `n` well-written questions in one theme: no warning expected.
fn clean(n: usize) -> Paper {
    let mut paper = Paper::new();
    paper.title = "Clean paper".into();
    paper.themes.push(Theme {
        id: "fruit".into(),
        title: "Fruit".into(),
        course_ref: "Fruit, chapter 1".into(),
        ..Theme::default()
    });
    let fruits = [
        "banana", "apple", "cherry", "lemon", "grape", "melon", "peach", "plum", "kiwi", "mango",
        "pear", "fig",
    ];
    for i in 0..n {
        let fruit = fruits[i % fruits.len()];
        let id = format!("fruit-{i}");
        let texts = ["Yellow", "Blue", "Green", "Black"];
        // Rotate the correct answer so no position dominates.
        let options = texts
            .iter()
            .enumerate()
            .map(|(k, t)| AnswerOption {
                id: format!("{id}-o{}", k + 1),
                text: (*t).into(),
                is_correct: k == i % 4,
                explanation: "Because of its pigments.".into(),
            })
            .collect();
        paper.questions.push(Question {
            id,
            theme_id: "fruit".into(),
            level: Level::Apply,
            body: vec![Block::Text(format!(
                "Variant {i}: which colour is a ripe {fruit} number {i}?"
            ))],
            options,
            rule: "Colour follows ripeness.".into(),
            ..Question::default()
        });
    }
    paper
}

fn codes(paper: &Paper) -> Vec<&'static str> {
    lint(paper)
        .into_iter()
        .filter(|i| i.code != "DURATION")
        .map(|i| i.code)
        .collect()
}

#[test]
fn clean_paper_has_only_the_duration_info() {
    let issues = lint(&clean(12));
    assert_eq!(issues.len(), 1, "{issues:#?}");
    assert_eq!(issues[0].code, "DURATION");
    assert_eq!(issues[0].severity, Severity::Info);
    assert!(!has_errors(&issues));
}

#[test]
fn no_title_and_no_question() {
    let mut paper = Paper::new();
    assert_eq!(codes(&paper), ["NO_TITLE", "NO_QUESTION"]);
    paper.title = "T".into();
    assert_eq!(codes(&paper), ["NO_QUESTION"]);
    assert!(has_errors(&lint(&paper)));
}

#[test]
fn structure_errors() {
    let mut paper = clean(4);
    paper.questions[1].id = paper.questions[0].id.clone();
    paper.questions[2].theme_id = "ghost".into();
    assert_eq!(codes(&paper), ["DUPLICATE_QUESTION_ID", "UNKNOWN_THEME"]);

    let mut paper = clean(3);
    paper.questions[0].id.clear();
    paper.questions[0].line = 12;
    paper.questions[1].body.clear();
    paper.questions[2].theme_id.clear();
    let issues = lint(&paper);
    let found: Vec<_> = issues
        .iter()
        .map(|i| (i.code, i.location.clone()))
        .collect();
    assert!(found.contains(&("QUESTION_WITHOUT_ID", Location::Line(12))));
    assert!(found.contains(&("EMPTY_STATEMENT", Location::Question("fruit-1".into()))));
    assert!(found.contains(&(
        "QUESTION_WITHOUT_THEME",
        Location::Question("fruit-2".into())
    )));
}

#[test]
fn option_errors() {
    let mut paper = clean(3);
    paper.questions[0].options.truncate(1);
    assert!(codes(&paper).contains(&"TOO_FEW_OPTIONS"));

    let mut paper = clean(3);
    for o in &mut paper.questions[0].options {
        o.is_correct = false;
    }
    for o in &mut paper.questions[1].options {
        o.is_correct = true;
    }
    paper.questions[2].options[3].text = "yellow ".into();
    paper.questions[2].options[2].text = " ".into();
    let found = codes(&paper);
    for code in [
        "NO_CORRECT_OPTION",
        "ALL_OPTIONS_CORRECT",
        "DUPLICATE_OPTION",
        "EMPTY_OPTION",
    ] {
        assert!(found.contains(&code), "{code} missing from {found:?}");
    }
}

#[test]
fn multiple_correct_options_is_information() {
    let mut paper = clean(3);
    paper.questions[0].options[0].is_correct = true;
    paper.questions[0].options[1].is_correct = true;
    let issues = lint(&paper);
    let multi = issues
        .iter()
        .find(|i| i.code == "MULTIPLE_CORRECT_OPTIONS")
        .expect("reported");
    assert_eq!(multi.severity, Severity::Info);
    assert!(!has_errors(&issues));
}

#[test]
fn announced_answer_count_must_match() {
    let mut paper = clean(3);
    paper.questions[0].declared_answers = 2;
    assert_eq!(codes(&paper), ["ANSWER_COUNT_MISMATCH"]);
    paper.questions[0].declared_answers = 1;
    assert!(codes(&paper).is_empty());
}

#[test]
fn too_many_and_meta_options() {
    let mut paper = clean(3);
    let q = &mut paper.questions[0];
    let mut extra = q.options[1].clone();
    extra.text = "None of the above".into();
    q.options.push(extra);
    assert_eq!(codes(&paper), ["TOO_MANY_OPTIONS", "META_OPTION"]);
}

fn with_stem(stem: &str) -> Paper {
    let mut paper = clean(3);
    paper.questions[0].body = vec![Block::Text(stem.into())];
    paper
}

#[test]
fn stem_rules() {
    assert_eq!(
        codes(&with_stem("Which fruit is not yellow?")),
        ["NEGATIVE_STEM"]
    );
    assert!(codes(&with_stem("Which fruit is NOT yellow?")).is_empty());
    assert!(codes(&with_stem("Quel fruit n'est **pas** jaune ?")).is_empty());
    assert_eq!(
        codes(&with_stem("Bananas are yellow.")),
        ["MISSING_QUESTION"]
    );
    assert!(codes(&with_stem("Pick the yellow fruit:")).is_empty());
}

#[test]
fn vague_stem_needs_mixed_compile_options() {
    let mut paper = with_stem("Here is the code. What happens?");
    paper.questions[0].options[1].text = "It does not compile".into();
    assert_eq!(codes(&paper), ["VAGUE_STEM"]);
    paper.questions[0].body = vec![Block::Text(
        "Does it compile, and what does it print?".into(),
    )];
    assert!(codes(&paper).is_empty());
}

#[test]
fn media_needs_a_text_alternative() {
    let mut paper = clean(3);
    paper.questions[0].body.push(Block::Media(Media {
        kind: MediaKind::Audio,
        path: "media/voice.ogg".into(),
        alt: String::new(),
    }));
    assert_eq!(codes(&paper), ["MEDIA_WITHOUT_ALT"]);
}

#[test]
fn duration_rules() {
    let mut paper = clean(3);
    paper.duration_minutes = 1;
    paper.questions[0].estimated_seconds = 120;
    assert_eq!(codes(&paper), ["DURATION_MISMATCH"]);
    paper.duration_minutes = 0;
    paper.questions[0].estimated_seconds = 100 * 60;
    assert_eq!(codes(&paper), ["TOO_LONG"]);
}

#[test]
fn level_rules() {
    let mut paper = clean(4);
    paper.questions[0].level = Level::Unknown;
    assert_eq!(codes(&paper), ["LEVEL_MISSING"]);
    paper.questions[0].level = Level::Remember;
    assert_eq!(codes(&paper), ["TOO_MUCH_RECALL"]);
    paper.questions[1].level = Level::Understand;
    paper.questions[2].level = Level::Understand;
    assert_eq!(codes(&paper), ["TOO_MUCH_RECALL", "NOT_ENOUGH_APPLY"]);
}

#[test]
fn theme_rules() {
    let mut paper = clean(2);
    paper.themes.push(Theme {
        id: "empty".into(),
        ..Theme::default()
    });
    let issues = lint(&paper);
    let found: Vec<_> = issues
        .iter()
        .map(|i| (i.code, i.location.clone()))
        .collect();
    assert!(found.contains(&("THIN_THEME", Location::Theme("fruit".into()))));
    assert!(found.contains(&("EMPTY_THEME", Location::Theme("empty".into()))));
}

#[test]
fn feedback_rules() {
    let mut paper = clean(3);
    for o in &mut paper.questions[0].options {
        o.explanation.clear();
    }
    paper.questions[1].options[2].explanation.clear();
    paper.questions[2].rule.clear();
    assert_eq!(
        codes(&paper),
        ["NO_EXPLANATION", "PARTIAL_EXPLANATION", "NO_RULE"]
    );

    let mut paper = clean(3);
    paper.themes[0].course_ref.clear();
    paper.questions[0].course_ref = "Chapter 2".into();
    assert_eq!(codes(&paper), ["NO_COURSE_REF", "NO_COURSE_REF"]);
}

#[test]
fn length_bias_per_question() {
    let mut paper = clean(3);
    paper.questions[0].options[0].text = "Yellow, because ripening reveals pigments".into();
    assert_eq!(codes(&paper), ["LENGTH_BIAS"]);
}

#[test]
fn paper_wide_length_and_position_patterns() {
    let mut paper = clean(10);
    for q in &mut paper.questions {
        for (k, o) in q.options.iter_mut().enumerate() {
            o.is_correct = k == 0;
            o.text = if k == 0 {
                "Bright yellow".into()
            } else {
                format!("Hue {k}")
            };
        }
    }
    let found = codes(&paper);
    assert!(found.contains(&"LENGTH_RANK_BIAS"), "{found:?}");
    assert!(found.contains(&"ANSWER_POSITION_BIAS"), "{found:?}");
    let issues = lint(&paper);
    let position = issues
        .iter()
        .find(|i| i.code == "ANSWER_POSITION_BIAS")
        .expect("reported");
    assert!(position.message.contains("100 %") && position.message.contains("position A"));

    paper.questions.truncate(9);
    let found = codes(&paper);
    assert!(!found.contains(&"LENGTH_RANK_BIAS") && !found.contains(&"ANSWER_POSITION_BIAS"));
}

#[test]
fn duplicates_inside_and_across_papers() {
    let mut paper = clean(3);
    paper.questions[1].body = vec![Block::Text(
        "How many seeds does a cherry stone hold?".into(),
    )];
    paper.questions[2].body = paper.questions[0].body.clone();
    assert_eq!(find_duplicates_within(&paper).len(), 1);
    let other = clean(1);
    let issues = find_duplicates(&paper, &other, "other.qcm.md");
    assert_eq!(issues.len(), 2);
    assert!(issues[0].message.contains("other.qcm.md"));
}

#[test]
fn sample_fixture_lints_without_errors() {
    let source = include_str!("../../mcq-format/tests/fixtures/sample.qcm.md");
    let (paper, read_issues) = mcq_format::read(source);
    assert!(read_issues.is_empty());
    let issues = lint(&paper);
    assert!(!has_errors(&issues), "{issues:#?}");
}

#[test]
fn formulas_must_convert_and_dollars_must_be_meant() {
    let mut paper = clean(4);
    paper.questions[0].body = vec![Block::Text(
        r"Which value has $\frac{a}{b}$ when $a = 2b$?".into(),
    )];
    assert_eq!(codes(&paper), Vec::<&str>::new(), "a good formula");

    paper.questions[1].options[0].explanation = r"Since $\frac{1}{$ is open.".into();
    paper.questions[2].body = vec![Block::Text(
        "Which ripe fruit costs $5 at the market?".into(),
    )];
    paper.questions[3].body = vec![Block::Text(
        r"Which ripe fruit costs \$5 at the market?".into(),
    )];
    let issues = lint(&paper);
    let bad: Vec<_> = issues.iter().filter(|i| i.code == "BAD_FORMULA").collect();
    assert_eq!(bad.len(), 1, "{issues:#?}");
    assert_eq!(bad[0].severity, Severity::Error);
    assert_eq!(bad[0].location, Location::Question("fruit-1".into()));
    assert!(!bad[0].message.contains(".."), "{}", bad[0].message);
    assert!(
        bad[0].message.contains(r"`$\frac{1}{$`"),
        "{}",
        bad[0].message
    );
    let stray: Vec<_> = issues.iter().filter(|i| i.code == "STRAY_DOLLAR").collect();
    assert_eq!(stray.len(), 1, "an escaped dollar is fine: {issues:#?}");
    assert_eq!(stray[0].severity, Severity::Warning);
    assert_eq!(stray[0].location, Location::Question("fruit-2".into()));
    assert!(has_errors(&issues));
}

/// A paper damaged in every way the linter knows, written for this test.
const DAMAGED: &str = r"---
duration: 1
---
## Q
Before any theme, without an id?
- [x] Yes
- [ ] No

# THEME fruit | Fruit
# THEME empty | Empty

## Q dup | level 1 | 6000s | answers 3
Which fruit is not yellow? It costs $5.
- [x] Banana
- [x] Lemon
- [ ] Banana
- [ ] Cherry
- [ ] All of the above

## Q dup
- [ ]

## Q vague | level 2
Here is some code. What happens?
- [x] It compiles
> Nothing stops it.
- [ ] It prints 3
RULE: Read before running.
REVIEW: Chapter 2

## Q mute | level 3
A picture of the orchard.
![](media/orchard.png)
- [x] Apples $\frac{1}{$
- [x] Pears

## Q none | level 3
Which tree gives figs?
- [ ] Oak
- [ ] Pine

# THEME veg | Vegetables

## Q long | level 1
Which vegetable is a root?
- [x] The carrot, because we eat the part that grows underground
- [ ] Leek
";

#[test]
fn every_issue_has_its_french_text() {
    let (mut paper, _) = mcq_format::read(DAMAGED);
    if let Some(q) = paper.questions.iter_mut().find(|q| q.id == "none") {
        q.theme_id = "ghost".into();
    }
    let mut issues = lint(&paper);
    issues.extend(find_duplicates_within(&paper));
    issues.extend(find_duplicates(&paper, &paper.clone(), "orchard.qcm.md"));

    // Ten questions or more for the paper-wide patterns: the correct answer first and longest.
    let mut biased = clean(12);
    for question in &mut biased.questions {
        for (k, option) in question.options.iter_mut().enumerate() {
            option.is_correct = k == 0;
        }
        question.options[0].text = "Golden yellow".into();
    }
    issues.extend(lint(&biased));

    let mut found: Vec<&str> = issues.iter().map(|i| i.code).collect();
    found.sort_unstable();
    found.dedup();
    assert!(found.len() >= 35, "{} codes: {found:?}", found.len());
    for issue in &issues {
        assert!(!issue.fr.trim().is_empty(), "{issue:#?}");
        assert_ne!(issue.fr, issue.message, "{issue:#?}");
        assert_eq!(issue.message_in(true), issue.fr);
        assert_eq!(issue.message_in(false), issue.message);
    }
}
