//! The website demo: clean papers, the same in both languages, rebuilt identical, and readable by
//! the player and the Studio.

use mcq_crypto::{KeyUnlock, OpeningKind};
use mcq_demo::{SOURCES, build, build_all, project};
use mcq_edit::qcmz;
use mcq_model::{Paper, Severity};

fn paper(source: &mcq_demo::Source) -> Paper {
    mcq_format::read(source.paper).0
}

#[test]
fn papers_have_no_warning() {
    for source in &SOURCES {
        let issues: Vec<_> = mcq_lint::lint(&paper(source))
            .into_iter()
            .filter(|i| i.severity != Severity::Info)
            .collect();
        assert!(issues.is_empty(), "{}: {issues:?}", source.lang);
    }
}

#[test]
fn sources_are_written_as_the_studio_writes_them() {
    // The visitor opens the paper in the Studio: its source view must show this very file.
    for source in &SOURCES {
        let canonical = mcq_format::write(&paper(source));
        let differs = canonical
            .lines()
            .zip(source.paper.lines())
            .enumerate()
            .find(|(_, (a, b))| a != b);
        assert!(differs.is_none(), "{}: {differs:?}", source.lang);
        assert_eq!(canonical, source.paper, "{}", source.lang);
    }
}

#[test]
fn languages_share_one_structure() {
    let shape = |p: &Paper| {
        p.themes
            .iter()
            .map(|t| {
                p.questions
                    .iter()
                    .filter(|q| q.theme_id == t.id)
                    .map(|q| {
                        let correct: Vec<bool> = q.options.iter().map(|o| o.is_correct).collect();
                        (q.level, q.estimated_seconds, correct, q.media().count())
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    let papers: Vec<Paper> = SOURCES.iter().map(paper).collect();
    assert_eq!(shape(&papers[0]), shape(&papers[1]));
    assert_eq!(papers[0].duration_minutes, papers[1].duration_minutes);
    for (source, p) in SOURCES.iter().zip(&papers) {
        assert_eq!(p.lang, source.lang);
    }
}

#[test]
fn builds_are_identical() {
    let first = build_all().expect("build");
    let second = build_all().expect("build");
    for (a, b) in first.iter().zip(&second) {
        assert!(a.package == b.package, "{} package differs", a.lang);
        assert!(a.qcmz == b.qcmz, "{} qcmz differs", a.lang);
    }
    assert_eq!(first[0].fingerprint, first[1].fingerprint);
}

#[test]
fn package_opens_as_practice_with_its_correction() {
    for source in &SOURCES {
        let built = build(source).expect("build");
        let header = mcq_crypto::inspect(&built.package).expect("signature");
        assert_eq!(header.opening, OpeningKind::Open);
        assert!(!header.has_end_code);
        let opened = mcq_crypto::open(&built.package, "").expect("open without code");
        let full = mcq_crypto::open_answer_key(&built.package, KeyUnlock::Practice).expect("key");
        let original = paper(source);
        assert_eq!(full.questions.len(), original.questions.len());
        for (a, b) in full.questions.iter().zip(&original.questions) {
            assert_eq!(a.id, b.id);
            assert_eq!(
                a.options.iter().map(|o| o.is_correct).collect::<Vec<_>>(),
                b.options.iter().map(|o| o.is_correct).collect::<Vec<_>>()
            );
        }
        // Every picture travels in the package.
        for media in original.questions.iter().flat_map(|q| q.media()) {
            assert!(
                opened.media.iter().any(|f| f.path == media.path),
                "{}: {} missing",
                source.lang,
                media.path
            );
        }
    }
}

#[test]
fn qcmz_opens_in_the_studio_unchanged() {
    for source in &SOURCES {
        let built = build(source).expect("build");
        let (imported, issues) = qcmz::import(&built.qcmz).expect("import");
        assert!(issues.is_empty(), "{issues:?}");
        let expected = project(source);
        assert_eq!(imported, expected);
        assert!(!imported.media.is_empty());
        assert_eq!(imported.source(), source.paper);
    }
}
