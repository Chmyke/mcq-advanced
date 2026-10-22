//! What the Studio opens: its own papers, and the banks of other tools.

use mcq_studio::actions::{is_paper_file, read_paper};

const SAMPLE: &str = include_str!("../../../crates/mcq-format/tests/fixtures/sample.qcm.md");
const GIFT: &str = include_str!("../../../crates/mcq-convert/tests/fixtures/bank.gift");
const MOODLE: &str = include_str!("../../../crates/mcq-convert/tests/fixtures/moodle-bank.xml");

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
        read_paper("sample.qcm.md", SAMPLE.as_bytes().to_vec(), "en").expect("paper");
    assert_eq!(project.paper.questions.len(), 3);

    let (project, issues) = read_paper("bank.gift", GIFT.as_bytes().to_vec(), "fr").expect("gift");
    assert_eq!(project.paper.title, "bank");
    assert_eq!(
        project.paper.question("sun").expect("sun").options[0].text,
        "Vrai"
    );
    assert!(issues.iter().any(|i| i.code == "IMPORT_SKIPPED"));

    let (project, _) = read_paper("moodle.xml", MOODLE.as_bytes().to_vec(), "en").expect("xml");
    assert_eq!(project.media.len(), 1);
}

#[test]
fn unreadable_banks_say_why() {
    let err = read_paper("notes.txt", b"just words".to_vec(), "en").expect_err("no format");
    assert_eq!(err, "unknown format");
    let err = read_paper("bank.xml", b"<quiz>".to_vec(), "en").expect_err("broken");
    assert!(err.contains("XML"), "{err}");
}

#[test]
fn the_website_demo_opens_with_its_picture() {
    for built in mcq_demo::build_all().expect("demo") {
        let name = format!("{}.qcmz", built.lang);
        assert!(is_paper_file(&name));
        let (project, issues) = read_paper(&name, built.qcmz, "en").expect("demo paper");
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
