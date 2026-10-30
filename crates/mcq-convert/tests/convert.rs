//! Imports of every format from fictional banks, and the Moodle round trip.

use mcq_convert::{ImportOptions, Imported, Source, handout, import, moodle};
use mcq_edit::Project;
use mcq_model::{Block, Issue, Level, MediaKind, Paper, Scoring, Severity};

const SAMPLE: &str = include_str!("../../mcq-format/tests/fixtures/sample.qcm.md");
const GIFT: &str = include_str!("fixtures/bank.gift");
const AIKEN: &str = include_str!("fixtures/bank-aiken.txt");
const CSV: &str = include_str!("fixtures/bank.csv");
const MOODLE: &str = include_str!("fixtures/moodle-bank.xml");

fn options(lang: &str) -> ImportOptions {
    ImportOptions {
        title: "Imported bank".to_string(),
        lang: lang.to_string(),
    }
}

fn codes(issues: &[Issue], severity: Severity) -> Vec<&'static str> {
    issues
        .iter()
        .filter(|i| i.severity == severity)
        .map(|i| i.code)
        .collect()
}

/// The Moodle export with code coloured, as the CLI and the Studio call it.
fn export(project: &Project) -> moodle::Exported {
    moodle::export(
        project,
        &mcq_highlight::Highlighter::new().paper(&project.paper),
    )
}

/// What the `.qcm.md` reader gives back from the written paper: the import must survive it.
fn reread(paper: &Paper) -> Paper {
    let (mut again, issues) = mcq_format::read(&mcq_format::write(paper));
    assert!(
        !issues.iter().any(|i| i.severity == Severity::Error),
        "{issues:?}"
    );
    for q in &mut again.questions {
        q.line = 0;
    }
    again
}

fn statement(imported: &Imported, id: &str) -> Vec<Block> {
    imported
        .project
        .paper
        .question(id)
        .unwrap_or_else(|| panic!("no question {id}"))
        .body
        .clone()
}

#[test]
fn gift_brings_single_multiple_and_true_false() {
    let imported = import(Source::Gift, GIFT, &options("en"));
    let paper = &imported.project.paper;
    let ids: Vec<&str> = paper.questions.iter().map(|q| q.id.as_str()).collect();
    assert_eq!(
        ids,
        ["planet-red", "moons", "sun", "pluto", "escapes", "gap"],
        "{:?}",
        imported.issues
    );
    let themes: Vec<&str> = paper.themes.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(themes, ["Planets", "Symbols"]);

    let red = paper.question("planet-red").expect("red");
    assert_eq!(red.level, Level::Remember);
    assert_eq!(red.estimated_seconds, 20);
    assert_eq!(red.rule, "Mars owes its colour to rust.");
    assert_eq!(red.options[0].text, "Mars");
    assert!(red.options[0].is_correct);
    assert_eq!(red.options[0].explanation, "Iron oxide colours its soil.");
    assert_eq!(red.correct_count(), 1);

    let moons = paper.question("moons").expect("moons");
    assert_eq!(moons.correct_count(), 2);
    assert_eq!(moons.options[2].explanation, "Venus has no moon.");
    assert_eq!(paper.scoring, Scoring::Partial);

    // `{TRUE#wrong#right}`: the right feedback goes with the true option.
    let sun = paper.question("sun").expect("sun");
    assert_eq!(
        (sun.options[0].text.as_str(), sun.options[0].is_correct),
        ("True", true)
    );
    assert_eq!(sun.options[0].explanation, "Right.");
    assert_eq!(sun.options[1].explanation, "Wrong, it is a star.");
    assert!(paper.question("pluto").expect("pluto").options[1].is_correct);

    let escapes = paper.question("escapes").expect("escapes");
    assert_eq!(
        escapes.statement_text(),
        "In GIFT, which character writes = literally?"
    );
    assert_eq!(escapes.options[0].text, "A backslash \\ before it");
    assert_eq!(escapes.options[1].text, "A colon : after it");
    assert_eq!(
        statement(&imported, "gap"),
        vec![Block::Text(
            "Light from the Sun reaches us in about _____ , not instantly.".to_string()
        )]
    );

    // Numerical, essay, matching, short answer and the description are reported, not imported;
    // `-100%` on a wrong answer of a two-answer question is harsher than partial credit.
    let warnings = codes(&imported.issues, Severity::Warning);
    assert_eq!(
        warnings.iter().filter(|c| **c == "IMPORT_SKIPPED").count(),
        5
    );
    assert_eq!(
        warnings
            .iter()
            .filter(|c| **c == "IMPORT_SIMPLIFIED")
            .count(),
        1
    );
    assert_eq!(reread(paper).questions, paper.questions);
}

#[test]
fn aiken_reads_options_and_answer_letters() {
    let imported = import(Source::Aiken, AIKEN, &options("en"));
    let paper = &imported.project.paper;
    assert_eq!(paper.questions.len(), 2, "{:?}", imported.issues);
    let gas = &paper.questions[0];
    assert_eq!(gas.statement_text(), "Which gas do plants take in?");
    assert_eq!(gas.options[2].text, "Nitrogen");
    assert!(gas.options[1].is_correct);
    let even = &paper.questions[1];
    assert_eq!(
        even.statement_text(),
        "Which numbers are even?\nPick every one."
    );
    assert_eq!(even.correct_count(), 2);
    // No `ANSWER:` line, then a letter outside the options.
    assert_eq!(
        codes(&imported.issues, Severity::Warning),
        ["IMPORT_SKIPPED"; 2]
    );
    assert_eq!(paper.themes[0].title, "Imported bank");
    assert_eq!(reread(paper).questions, paper.questions);
}

#[test]
fn csv_finds_columns_by_their_headers() {
    let imported = import(Source::Csv, CSV, &options("en"));
    let paper = &imported.project.paper;
    assert_eq!(paper.questions.len(), 3, "{:?}", imported.issues);
    let primary = &paper.questions[0];
    assert_eq!(
        primary.statement_text(),
        "Which of these are primary colours? (pick \"all\")"
    );
    assert_eq!(primary.correct_count(), 2);
    assert!(primary.options[0].is_correct && primary.options[2].is_correct);
    assert_eq!(primary.estimated_seconds, 30);
    assert_eq!(primary.rule, "Red and blue are primary.");
    assert_eq!(paper.questions[1].options.len(), 2);
    assert!(paper.questions[1].options[0].is_correct);
    assert!(paper.questions[2].options[1].is_correct);
    let themes: Vec<&str> = paper.themes.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(themes, ["Colours", "Animals"]);
    // The open question and the one with a single option.
    assert_eq!(codes(&imported.issues, Severity::Warning).len(), 2);
}

#[test]
fn csv_without_a_question_column_is_refused() {
    let imported = import(Source::Csv, "a;b\n1;2\n", &options("en"));
    assert_eq!(
        codes(&imported.issues, Severity::Error),
        ["IMPORT_UNREADABLE"]
    );
}

#[test]
fn moodle_bank_comes_in_with_media_and_categories() {
    let imported = import(Source::MoodleXml, MOODLE, &options("fr"));
    let paper = &imported.project.paper;
    let ids: Vec<&str> = paper.questions.iter().map(|q| q.id.as_str()).collect();
    assert_eq!(
        ids,
        ["boiling-water", "knife-habits", "yeast"],
        "{:?}",
        imported.issues
    );
    let themes: Vec<(&str, &str)> = paper
        .themes
        .iter()
        .map(|t| (t.id.as_str(), t.title.as_str()))
        .collect();
    assert_eq!(themes, [("kitchen", "Kitchen"), ("bread", "Bread")]);
    assert_eq!(paper.themes[0].course_ref, "Cooking, **chapter 3**");
    // Moodle's own default category does not name the paper.
    assert_eq!(paper.title, "Imported bank");

    let boiling = statement(&imported, "boiling-water");
    assert_eq!(
        boiling[0],
        Block::Text("At sea level, water boils at which temperature?".to_string())
    );
    match &boiling[1] {
        Block::Media(m) => {
            assert_eq!(
                (m.kind, m.path.as_str()),
                (MediaKind::Image, "media/pot-photo.png")
            );
            assert_eq!(m.alt, "A pot on a stove");
        }
        other => panic!("{other:?}"),
    }
    assert!(imported.project.media.contains_key("media/pot-photo.png"));
    let q = paper.question("boiling-water").expect("boiling");
    assert_eq!(q.options[0].text, "100 °C");
    assert_eq!(
        q.options[1].explanation,
        "That is the boiling point *at altitude*.\nAround 3000 m."
    );
    assert_eq!(q.rule, "Pure water boils at 100 °C under 1 atm.");

    let knife = paper.question("knife-habits").expect("knife");
    assert_eq!(
        knife.statement_text(),
        "Which habits are safe with a knife?\nTick every one."
    );
    assert_eq!(knife.correct_count(), 2);
    let yeast = paper.question("yeast").expect("yeast");
    assert_eq!(yeast.options[0].text, "Vrai");
    assert!(yeast.options[0].is_correct);
    assert_eq!(
        yeast.options[0].explanation,
        "Right: a single-celled fungus."
    );

    // The knife question was worth 2 points, with its own weights; short answer and
    // description skipped.
    assert_eq!(
        codes(&imported.issues, Severity::Warning),
        [
            "IMPORT_SIMPLIFIED",
            "IMPORT_SIMPLIFIED",
            "IMPORT_SKIPPED",
            "IMPORT_SKIPPED"
        ]
    );
    assert_eq!(reread(paper).questions, paper.questions);
    assert!(
        !mcq_lint::has_errors(&imported.project.issues()),
        "{:?}",
        imported.project.issues()
    );
}

#[test]
fn broken_xml_is_reported_not_panicked() {
    for text in [
        "",
        "<quiz>",
        "<other/>",
        "<quiz><question type=\"multichoice\"/></quiz>",
    ] {
        let imported = import(Source::MoodleXml, text, &options("en"));
        assert!(
            imported
                .issues
                .iter()
                .any(|i| i.severity == Severity::Error),
            "{text}"
        );
    }
}

/// The sample paper with its two media files.
fn sample_project() -> Project {
    let (paper, issues) = mcq_format::read(SAMPLE);
    assert!(issues.is_empty(), "{issues:?}");
    let mut project = Project {
        paper,
        ..Project::default()
    };
    project.media.insert(
        "media/apple.png".into(),
        b"\x89PNG fictional apple".to_vec(),
    );
    project
        .media
        .insert("media/crunch.ogg".into(), b"OggS fictional crunch".to_vec());
    project
}

/// What a question bank keeps of a paper (`docs/exchange.md`).
fn bank_view(paper: &Paper, lang: &str) -> Paper {
    let mut expected = paper.clone();
    expected.duration_minutes = 0;
    expected.penalty = 0.0;
    expected.unanswered_penalty = 0.0;
    expected.shuffle_questions = true;
    expected.show_answer_count = true;
    expected.notes.clear();
    expected.lang = lang.to_string();
    expected.scoring = if paper.questions.iter().any(|q| q.correct_count() > 1) {
        Scoring::Partial
    } else {
        Scoring::AllOrNothing
    };
    for theme in &mut expected.themes {
        theme.notes.clear();
    }
    for question in &mut expected.questions {
        question.line = 0;
        question.declared_answers = 0;
    }
    expected
}

#[test]
fn moodle_round_trip_keeps_everything_a_bank_can_hold() {
    let project = sample_project();
    let exported = export(&project);
    let back = import(Source::MoodleXml, &exported.xml, &options("en"));
    assert!(
        !back.issues.iter().any(|i| i.severity != Severity::Info),
        "{:?}",
        back.issues
    );
    assert_eq!(back.project.paper, bank_view(&project.paper, "en"));
    assert_eq!(back.project.media, project.media);

    // Exporting what was imported gives the same bank, byte for byte.
    let again = export(&back.project);
    assert_eq!(again.xml, exported.xml);
    // The duration and penalty stay behind, and the export says so.
    assert_eq!(
        codes(&exported.issues, Severity::Info),
        ["EXPORT_SETTINGS", "EXPORT_ANSWER_COUNT"]
    );
}

#[test]
fn moodle_export_has_the_shape_moodle_reads() {
    let xml = export(&sample_project()).xml;
    let doc = roxmltree::Document::parse(&xml).expect("well-formed XML");
    let questions: Vec<_> = doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("question"))
        .collect();
    let kinds: Vec<&str> = questions
        .iter()
        .filter_map(|q| q.attribute("type"))
        .collect();
    assert_eq!(
        kinds,
        [
            "category",
            "category",
            "multichoice",
            "category",
            "multichoice",
            "multichoice"
        ]
    );
    assert!(xml.contains("<text>$course$/top/Sample paper: fruit and code/Fruit colours</text>"));
    // Two correct answers out of four: 50 each, -50 for each wrong one.
    assert!(xml.contains("<answer fraction=\"50\" format=\"html\">"));
    assert!(xml.contains("<answer fraction=\"-50\" format=\"html\">"));
    assert!(xml.contains("<single>false</single>"));
    assert!(xml.contains("<file name=\"apple.png\" path=\"/\" encoding=\"base64\">"));
    assert!(xml.contains("<img src=\"@@PLUGINFILE@@/apple.png\" alt=\"A red apple on a table\">"));
    assert!(xml.contains("<pre data-lines=\"3\"><code class=\"language-cs\">"));
}

/// The paper behind the file a teacher imports into a real Moodle site to check the export.
fn moodle_check_project() -> Project {
    let (paper, issues) = mcq_format::read(include_str!(
        "../../../examples/moodle-check/moodle-check.qcm.md"
    ));
    assert!(
        !issues.iter().any(|i| i.severity == Severity::Error),
        "{issues:?}"
    );
    let mut project = Project {
        paper,
        ..Project::default()
    };
    for (name, data) in [
        (
            "triangle.svg",
            &include_bytes!("../../../examples/moodle-check/media/triangle.svg")[..],
        ),
        (
            "tone.mp3",
            &include_bytes!("../../../examples/moodle-check/media/tone.mp3")[..],
        ),
        (
            "pattern.mp4",
            &include_bytes!("../../../examples/moodle-check/media/pattern.mp4")[..],
        ),
    ] {
        project.media.insert(format!("media/{name}"), data.to_vec());
    }
    project
}

#[test]
fn moodle_export_writes_formulas_for_mathjax_and_colours_code() {
    let xml = export(&moodle_check_project()).xml;
    // Inline and display formulas in MathJax's delimiters, LaTeX untouched by Markdown.
    assert!(xml.contains(r"\(\frac{1}{2} + \frac{1}{3}\)"), "{xml}");
    assert!(xml.contains(r"\(a &lt; b\)"));
    assert!(xml.contains(r"\(\left\{ x \mid x &gt; 0 \right\}\)"));
    assert!(xml.contains(r"\[\int_0^1 x^2 \, dx\]"));
    assert!(xml.contains(r"\(\frac{5}{6}\)</text>"));
    assert!(xml.contains(r"\[\int_0^1 x^n \, dx = \frac{1}{n+1}\]"));
    // Prices stay text.
    assert!(xml.contains("the book costs $5, the bag $10."));
    // Code: coloured tokens, line 2 marked, signs escaped, the text intact.
    assert!(xml.contains("<pre data-lines=\"2\"><code class=\"language-python\">"));
    assert!(xml.contains("<span style=\"color: #a626a4\">def</span>"));
    assert!(xml.contains("<span style=\"background-color: #fff1c2\">"));
    assert!(xml.contains("&lt;"));
    assert!(xml.contains("&amp;&amp;"));
    // Moodle's count of correct answers comes after the attempt: never written.
    assert!(!xml.contains("shownumcorrect"));
}

#[test]
fn moodle_round_trip_keeps_formulas_code_and_media() {
    let project = moodle_check_project();
    let exported = export(&project);
    let back = import(Source::MoodleXml, &exported.xml, &options("en"));
    assert!(
        !back.issues.iter().any(|i| i.severity != Severity::Info),
        "{:?}",
        back.issues
    );
    let expected = bank_view(&project.paper, "en");
    for (got, want) in back.project.paper.questions.iter().zip(&expected.questions) {
        assert_eq!(got, want);
    }
    assert_eq!(back.project.paper, expected);
    assert_eq!(back.project.media, project.media);
    assert_eq!(export(&back.project).xml, exported.xml);
    assert_eq!(
        codes(&exported.issues, Severity::Info),
        ["EXPORT_SETTINGS", "EXPORT_ANSWER_COUNT"]
    );
}

#[test]
fn moodle_import_reads_mathjax_formulas() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<quiz>
  <question type="multichoice">
    <name><text>sum</text></name>
    <questiontext format="html"><text><![CDATA[<p>What is \( \frac{1}{2} + \frac{1}{3} \) if \(a_1 &lt; b\)?</p><p>$$\sqrt{x}$$ and \[ y^2 \]</p>]]></text></questiontext>
    <single>true</single>
    <answer fraction="100" format="html"><text>\(\frac{5}{6}\)</text></answer>
    <answer fraction="0" format="html"><text>\(\frac{2}{5}\)</text></answer>
  </question>
</quiz>"#;
    let imported = import(Source::MoodleXml, xml, &options("en"));
    let question = imported.project.paper.question("sum").expect("sum");
    assert_eq!(
        question.body,
        [Block::Text(
            "What is $\\frac{1}{2} + \\frac{1}{3}$ if $a_1 < b$?\n$$\\sqrt{x}$$ and $$y^2$$"
                .to_string()
        )]
    );
    assert_eq!(question.options[0].text, "$\\frac{5}{6}$");
}

#[test]
fn every_format_is_detected() {
    assert_eq!(Source::detect("bank.gift", ""), Some(Source::Gift));
    assert_eq!(Source::detect("bank.xml", ""), Some(Source::MoodleXml));
    assert_eq!(Source::detect("bank.CSV", ""), Some(Source::Csv));
    assert_eq!(Source::detect("bank.txt", AIKEN), Some(Source::Aiken));
    assert_eq!(Source::detect("bank.txt", GIFT), Some(Source::Gift));
    assert_eq!(Source::detect("bank.txt", "just words"), None);
    assert_eq!(Source::from_name("MOODLE"), Some(Source::MoodleXml));
}

#[test]
fn handout_hides_every_answer() {
    let (paper, _) = mcq_format::read(SAMPLE);
    let text = handout::handout(&paper);
    assert!(text.starts_with("<!-- Generated by `mcq handout`"));
    assert!(text.contains("> 3 questions, some questions have several correct answers."));
    assert!(text.contains("Duration: 10 min."));
    assert!(text.contains("A wrong answer costs 0.25 point."));
    assert!(text.contains("> Themes: Fruit colours (1) · Counting loops (2)"));
    assert!(text.contains("### Question 2\n\n*Several answers possible (2 to tick)*"));
    assert!(text.contains("- **A.** It prints `14`"));
    assert!(text.contains("```cs\nint total = 0;"));
    assert!(text.contains("![A red apple on a table](media/apple.png)"));
    for secret in [
        "[x]",
        "Ripening",
        "RULE",
        "Author notes",
        "Colour follows",
        "REVIEW",
        "COURSE",
    ] {
        assert!(!text.contains(secret), "{secret} leaked");
    }

    let mut french = paper.clone();
    french.lang = "fr".to_string();
    french.unanswered_penalty = 0.5;
    let text = handout::handout(&french);
    assert!(text.contains("Une mauvaise réponse coûte 0,25 point."));
    assert!(text.contains("Une question laissée sans réponse coûte 0,5 point."));
    assert!(text.contains("*Plusieurs réponses possibles (2 à cocher)*"));
}
