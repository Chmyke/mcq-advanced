//! The writer gives back what the reader read, and student copies hide the answers.

use mcq_format::{read, student_view, write};
use mcq_model::Paper;

const SAMPLE: &str = include_str!("fixtures/sample.qcm.md");

/// Source lines are the only thing a rewrite may change.
fn without_lines(mut paper: Paper) -> Paper {
    for q in &mut paper.questions {
        q.line = 0;
    }
    paper
}

fn round_trip(source: &str) {
    let (paper, issues) = read(source);
    assert!(issues.is_empty(), "{issues:#?}");
    let text = write(&paper);
    let (again, issues) = read(&text);
    assert!(issues.is_empty(), "{issues:#?}\n{text}");
    assert_eq!(without_lines(again), without_lines(paper), "\n{text}");
    assert_eq!(write(&read(&text).0), text, "writing is stable");
}

#[test]
fn sample_round_trips() {
    round_trip(SAMPLE);
}

#[test]
fn edge_cases_round_trip() {
    round_trip(
        "---\ntitle: \"Edges\"\npenalty: 0,333\nshuffleOptions: no\nshowAnswerCount: false\n---\n\
         # THEME empty\n\
         # THEME t | Topic\n\
         ## Q q1 | level 5 | 7s | answers 1\nLine one\nLine two?\n\
         ```\n```\n\
         ``` {2}\n  indented\n\n  after blank\n```\n\
         - [x] A\n  > one\n  >\n  > two\n- [ ] B\n  > trailing\n  >\n\
         REVIEW: here\n\
         ## Q q2\nNo level?\n- [] X\n- [X] Y\n",
    );
}

#[test]
fn student_copy_keeps_the_count_but_not_the_answers() {
    let (paper, _) = read(SAMPLE);
    let copy = student_view(&paper);
    let text = write(&copy);
    assert!(!text.contains("[x]"), "{text}");
    assert!(!text.contains("RULE:"), "{text}");
    assert!(!text.contains("  >"), "{text}");
    assert!(
        text.contains("## Q code-sum-of-squares | level 3 | answers 2"),
        "{text}"
    );

    let (read_back, issues) = read(&text);
    assert!(issues.is_empty(), "{issues:#?}");
    let q = &read_back.questions[1];
    assert!(q.is_multiple());
    assert_eq!(q.answers_to_tick(), 2);
    assert_eq!(q.correct_count(), 0);
    assert_eq!(read_back.questions[0].answers_to_tick(), 1);
    assert_eq!(q.course_ref, paper.questions[1].course_ref);
    assert_eq!(q.options[0].id, paper.questions[1].options[0].id);
}

#[test]
fn author_notes_survive_a_rewrite_but_never_reach_students() {
    let source = "---\ntitle: Notes\n---\nDraft: ask Sam about Q2.\nSecond note line\n\
                  # THEME t | Topic\nCOURSE: ch. 1\nTheme note\n## Q q1\nWhy?\n- [x] A\n- [ ] B\n";
    let (paper, issues) = read(source);
    assert!(issues.is_empty(), "{issues:#?}");
    assert_eq!(paper.notes, "Draft: ask Sam about Q2.\nSecond note line");
    assert_eq!(paper.themes[0].notes, "Theme note");
    round_trip(source);

    let copy = write(&student_view(&paper));
    assert!(!copy.contains("Sam"), "{copy}");
    assert!(!copy.contains("Theme note"), "{copy}");
}
