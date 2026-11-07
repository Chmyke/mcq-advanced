//! What the Studio opens: its own papers, and the banks of other tools.

use mcq_studio::actions::{
    import_question, is_paper_file, read_paper, read_pasted, readable_title,
};
use mcq_studio::texts::{EN, FR};

const SAMPLE: &str = include_str!("../../../crates/mcq-format/tests/fixtures/sample.qcm.md");
const GIFT: &str = include_str!("../../../crates/mcq-convert/tests/fixtures/bank.gift");
const MOODLE: &str = include_str!("../../../crates/mcq-convert/tests/fixtures/moodle-bank.xml");
const AIKEN: &str = include_str!("../../../crates/mcq-convert/tests/fixtures/bank-aiken.txt");
const CSV: &str = include_str!("../../../crates/mcq-convert/tests/fixtures/bank.csv");

#[test]
fn pasted_text_is_read_as_the_import_reads_its_file() {
    // Every bank the import button opens also opens from the clipboard, Moodle XML included.
    for (what, text, file) in [
        ("moodle", MOODLE, "bank.xml"),
        ("gift", GIFT, "bank.gift"),
        ("aiken", AIKEN, "bank.txt"),
        ("csv", CSV, "bank.csv"),
    ] {
        let (pasted, _) = read_pasted(text, FR.pasted_title, "fr", true)
            .expect(what)
            .expect(what);
        let (imported, _) = read_paper(file, text.as_bytes().to_vec(), "fr", true).expect(what);
        assert_eq!(
            pasted.paper.questions.len(),
            imported.paper.questions.len(),
            "{what}"
        );
        assert!(!pasted.paper.questions.is_empty(), "{what}");
    }
    // The paper inside an AI's answer, whatever surrounds it.
    let answer = format!("Here is your quiz:\n\n```markdown\n{SAMPLE}```\nGood luck!");
    let (project, _) = read_pasted(&answer, EN.pasted_title, "en", false)
        .expect("a paper")
        .expect("readable");
    assert_eq!(project.paper.questions.len(), 3);
    assert!(read_pasted("just words, nothing more", EN.pasted_title, "en", false).is_none());
}

#[test]
fn questions_without_a_category_go_to_a_theme_named_after_their_origin() {
    // Pasted: "Imported questions", never the title of a new paper.
    let (project, _) = read_pasted(AIKEN, FR.pasted_title, "fr", true)
        .expect("aiken")
        .expect("readable");
    let titles: Vec<&str> = project
        .paper
        .themes
        .iter()
        .map(|t| t.title.as_str())
        .collect();
    assert_eq!(titles, ["Questions importées"]);
    assert!(!titles.contains(&FR.new_paper_title));
    // From a file: the file's name.
    let (project, _) = read_paper(
        "chimie_organique.txt",
        AIKEN.as_bytes().to_vec(),
        "fr",
        true,
    )
    .expect("aiken");
    assert_eq!(project.paper.themes[0].title, "Chimie organique");
}

#[test]
fn the_import_question_says_where_the_questions_come_from() {
    let file = import_question(&FR, false, 4, 6);
    assert!(file.contains("le fichier en apporte 6"), "{file}");
    let pasted = import_question(&FR, true, 4, 6);
    assert!(pasted.contains("le texte collé en apporte 6"), "{pasted}");
    assert!(import_question(&EN, true, 1, 2).contains("the pasted text brings 2"));
}

#[test]
fn the_paste_refusal_lists_every_format_and_points_to_import() {
    for t in [&EN, &FR] {
        for format in ["Moodle XML", "GIFT", "Aiken", "CSV", ".qcm.md", t.import] {
            assert!(
                t.paste_not_paper.contains(format),
                "{format}: {}",
                t.paste_not_paper
            );
        }
    }
}

#[test]
fn banks_open_like_papers() {
    for name in [
        "a.qcmz",
        "a.qcm.md",
        "bank.GIFT",
        "bank.xml",
        "bank.txt",
        "bank.csv",
    ] {
        assert!(is_paper_file(name), "{name}");
    }
    assert!(!is_paper_file("photo.png"));

    let (project, _) =
        read_paper("sample.qcm.md", SAMPLE.as_bytes().to_vec(), "en", false).expect("paper");
    assert_eq!(project.paper.questions.len(), 3);

    let (project, issues) =
        read_paper("bank.gift", GIFT.as_bytes().to_vec(), "fr", false).expect("gift");
    // Named after the file, readable: students see it as the theme of every question.
    assert_eq!(project.paper.title, "Bank");
    assert!(!project.paper.allow_extra_time);
    assert_eq!(
        project.paper.question("sun").expect("sun").options[0].text,
        "Vrai"
    );
    assert!(issues.iter().any(|i| i.code == "IMPORT_SKIPPED"));

    let (project, _) =
        read_paper("moodle.xml", MOODLE.as_bytes().to_vec(), "en", false).expect("xml");
    assert_eq!(project.media.len(), 1);
}

#[test]
fn unreadable_banks_say_why() {
    let err = read_paper("notes.txt", b"just words".to_vec(), "en", false).expect_err("no format");
    assert_eq!(err, "unknown format");
    let err = read_paper("bank.xml", b"<quiz>".to_vec(), "en", false).expect_err("broken");
    assert!(err.contains("XML"), "{err}");
    let err = read_paper("bank.xml", b"<quiz>".to_vec(), "en", true).expect_err("broken");
    assert!(err.starts_with("Ce fichier XML"), "{err}");
}

#[test]
fn a_broken_paper_is_refused_with_its_line() {
    let broken = format!("{SAMPLE}\n## Q late\n```cs\nnever closed");
    let line = broken.lines().count() - 1;
    let err = read_paper("a.qcm.md", broken.clone().into_bytes(), "en", false).expect_err("broken");
    assert!(err.starts_with(&format!("line {line}: ")), "{err}");
    let err = read_paper("a.qcm.md", broken.into_bytes(), "en", true).expect_err("broken");
    assert!(err.starts_with(&format!("ligne {line} : ")), "{err}");
}

#[test]
fn the_website_demo_opens_with_its_picture() {
    for built in mcq_demo::build_all().expect("demo") {
        let name = format!("{}.qcmz", built.lang);
        assert!(is_paper_file(&name));
        let (project, issues) = read_paper(&name, built.qcmz, "en", false).expect("demo paper");
        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!(project.paper.lang, built.lang);
        assert_eq!(project.paper.questions.len(), 6);
        assert!(project.media.contains_key("media/triangle.svg"));
        assert!(
            project
                .issues()
                .iter()
                .all(|i| i.severity == mcq_model::Severity::Info)
        );
    }
}

#[test]
fn a_file_name_reads_as_a_title() {
    assert_eq!(
        readable_title("les_solutions-aqueuses"),
        "Les solutions aqueuses"
    );
    assert_eq!(readable_title("solutions"), "Solutions");
    assert_eq!(readable_title("élèves 2nde"), "Élèves 2nde");
    assert_eq!(readable_title("__"), "");
}
