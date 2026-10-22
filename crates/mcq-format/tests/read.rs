//! Reader scenarios on fictional papers.

use mcq_format::read;
use mcq_model::{Block, Level, Location, MediaKind, Scoring, Severity};

const SAMPLE: &str = include_str!("fixtures/sample.qcm.md");

fn codes(issues: &[mcq_model::Issue]) -> Vec<&'static str> {
    issues.iter().map(|i| i.code).collect()
}

#[test]
fn sample_reads_without_issue() {
    let (paper, issues) = read(SAMPLE);
    assert!(issues.is_empty(), "{issues:#?}");
    assert_eq!(paper.title, "Sample paper: fruit and code");
    assert_eq!(paper.lang, "en");
    assert_eq!(paper.duration_minutes, 10);
    assert_eq!(paper.penalty, 0.25);
    assert_eq!(paper.unanswered_penalty, 0.0);
    assert_eq!(paper.scoring, Scoring::Partial);
    assert!(!paper.shuffle_questions);
    assert!(paper.shuffle_options);
    assert_eq!(paper.themes.len(), 2);
    assert_eq!(paper.themes[0].course_ref, "Fruit, chapter 1");
    assert_eq!(paper.questions.len(), 3);
}

#[test]
fn question_header_options_and_explanations() {
    let (paper, _) = read(SAMPLE);
    let q = &paper.questions[0];
    assert_eq!(q.id, "fruit-banana-colour");
    assert_eq!(q.theme_id, "colours");
    assert_eq!(q.level, Level::Understand);
    assert_eq!(q.estimated_seconds, 15);
    assert_eq!(q.statement_text(), "A ripe banana is usually which colour?");
    assert_eq!(q.options.len(), 3);
    assert_eq!(q.options[0].id, "fruit-banana-colour-o1");
    assert!(q.options[0].is_correct);
    assert_eq!(
        q.options[0].explanation,
        "Ripening reveals the yellow pigments.\nNew paragraph."
    );
    assert_eq!(q.options[1].explanation, "No banana pigment is blue.");
    assert_eq!(q.rule, "Colour follows ripeness.");
    assert_eq!(q.course_ref, "Fruit, chapter 1, ripening");
}

#[test]
fn code_blocks_keep_order_language_and_highlights() {
    let (paper, _) = read(SAMPLE);
    let q = &paper.questions[1];
    assert!(q.is_multiple());
    assert_eq!(q.body.len(), 3);
    assert!(matches!(&q.body[0], Block::Text(t) if t == "Read this program."));
    let Block::Code(code) = &q.body[1] else {
        panic!("expected code, got {:?}", q.body[1]);
    };
    assert_eq!(code.language, "cs");
    assert_eq!(code.highlighted_lines, vec![3]);
    assert_eq!(
        code.source,
        "int total = 0;\nfor (int i = 1; i <= 3; i++)\n    total += i * i;\nConsole.WriteLine(total);"
    );
    assert!(matches!(&q.body[2], Block::Text(t) if t == "Which statements are true?"));
}

#[test]
fn media_lines_become_media_blocks() {
    let (paper, _) = read(SAMPLE);
    let q = &paper.questions[2];
    let media: Vec<_> = q.media().collect();
    assert_eq!(media.len(), 2);
    assert_eq!(media[0].kind, MediaKind::Image);
    assert_eq!(media[0].path, "media/apple.png");
    assert_eq!(media[0].alt, "A red apple on a table");
    assert_eq!(media[1].kind, MediaKind::Audio);
    assert_eq!(media[1].alt, "");
}

#[test]
fn crlf_and_french_keywords_are_accepted() {
    let src = "---\r\ntitle: \"T\"\r\nshuffleQuestions: non\r\npenalty: 0,5\r\n---\r\n\
        # THEME t | Topic\r\nCOURS: Ch 1\r\n## Q q1 | niveau 3 | 20s\r\nWhy?\r\n\
        - [x] A\r\n  > because\r\n- [ ] B\r\nREGLE: R\r\nREVOIR: Ch 2\r\n";
    let (paper, issues) = read(src);
    assert!(issues.is_empty(), "{issues:#?}");
    assert!(!paper.shuffle_questions);
    assert_eq!(paper.penalty, 0.5);
    assert_eq!(paper.themes[0].course_ref, "Ch 1");
    let q = &paper.questions[0];
    assert_eq!(q.level, Level::Apply);
    assert_eq!(q.estimated_seconds, 20);
    assert_eq!(q.options[0].explanation, "because");
    assert_eq!(q.rule, "R");
    assert_eq!(q.course_ref, "Ch 2");
}

#[test]
fn tabs_in_code_become_four_spaces() {
    let src =
        "---\ntitle: T\n---\n# THEME t\n## Q q1\nWhat?\n```c\n\tx++;\n```\n- [x] A\n- [ ] B\n";
    let (paper, _) = read(src);
    let code = paper.questions[0]
        .code_blocks()
        .next()
        .expect("one code block");
    assert_eq!(code.source, "    x++;");
}

#[test]
fn empty_source_is_an_error() {
    let (paper, issues) = read("  \n");
    assert_eq!(codes(&issues), vec!["EMPTY_SOURCE"]);
    assert!(paper.questions.is_empty());
}

#[test]
fn missing_and_unclosed_front_matter() {
    let (_, issues) = read("# THEME t\n");
    assert_eq!(codes(&issues), vec!["NO_FRONTMATTER"]);
    assert_eq!(issues[0].location, Location::Line(1));

    let (_, issues) = read("---\ntitle: T\n");
    assert_eq!(codes(&issues), vec!["UNCLOSED_FRONTMATTER"]);
}

#[test]
fn unknown_keys_and_bad_values_warn() {
    let (paper, issues) = read("---\ntitle: T\nduraton: 5\nduration: soon\n---\n");
    assert_eq!(codes(&issues), vec!["UNKNOWN_KEY", "BAD_VALUE"]);
    assert_eq!(paper.duration_minutes, 0);
    assert!(issues.iter().all(|i| i.severity == Severity::Warning));
}

#[test]
fn structural_errors_are_reported() {
    let src = "---\ntitle: T\n---\n\
        - [x] stray\n\
        ## Q early\nNo theme yet?\n\
        # THEME | nameless\n\
        # THEME a\n# THEME a\n\
        ## Q\n\
        ## Q q2 | niveau 9 | fast\n\n- [x]\n> lost?\n";
    let (paper, issues) = read(src);
    assert_eq!(
        codes(&issues),
        vec![
            "OPTION_WITHOUT_QUESTION",
            "THEME_WITHOUT_ID",
            "DUPLICATE_THEME",
            "BAD_LEVEL",
            "UNKNOWN_ATTRIBUTE",
        ]
    );
    // Meaning-level problems (no theme, no id, empty text) are left to the linter.
    let ids: Vec<_> = paper.questions.iter().map(|q| q.id.as_str()).collect();
    assert_eq!(ids, ["early", "", "q2"]);
    assert_eq!(paper.questions[0].theme_id, "");
    assert_eq!(paper.questions[2].line, 11);
    assert_eq!(paper.questions[2].options[0].text, "");
}

#[test]
fn orphan_explanation_and_stray_code_warn() {
    let src = "---\ntitle: T\n---\n```\nnotes\n```\n# THEME t\n## Q q1\nWhat?\n> orphan\n- [x] A\n";
    let (paper, issues) = read(src);
    assert_eq!(
        codes(&issues),
        vec!["CODE_OUTSIDE_QUESTION", "ORPHAN_EXPLANATION"]
    );
    assert_eq!(paper.questions[0].code_blocks().count(), 0);
}

#[test]
fn unclosed_fence_is_an_error_but_keeps_the_question() {
    let src = "---\ntitle: T\n---\n# THEME t\n## Q q1\nWhat?\n```py\nprint(1)\n";
    let (paper, issues) = read(src);
    assert_eq!(codes(&issues), vec!["UNCLOSED_CODE_FENCE"]);
    assert_eq!(issues[0].location, Location::Line(7));
    assert_eq!(paper.questions.len(), 1);
    assert_eq!(paper.questions[0].code_blocks().count(), 1);
}

#[test]
fn unknown_media_is_ignored_with_a_warning() {
    let src =
        "---\ntitle: T\n---\n# THEME t\n## Q q1\nWhat?\n![doc](notes.pdf)\n- [x] A\n- [ ] B\n";
    let (paper, issues) = read(src);
    assert_eq!(codes(&issues), vec!["UNKNOWN_MEDIA"]);
    assert_eq!(paper.questions[0].media().count(), 0);
}

#[test]
fn byte_order_mark_is_skipped() {
    let (paper, issues) = read("\u{feff}---\ntitle: T\n---\n");
    assert!(issues.is_empty(), "{issues:#?}");
    assert_eq!(paper.title, "T");
}

#[test]
fn reader_never_panics_on_odd_input() {
    for src in [
        "---",
        "---\n---",
        "```",
        "## Q",
        "# THEME",
        ">",
        "- [x]",
        "![](",
        "---\n:\n---\n## Q |||",
        "\u{feff}---\ntitle: x\n---",
        "```cs {99999-1}\n",
    ] {
        let _ = read(src);
    }
}
