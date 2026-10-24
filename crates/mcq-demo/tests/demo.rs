//! The website demos: clean papers, the same in both languages, rebuilt identical, and readable
//! by the player and the Studio, the gallery's models as much as the student demo; the teacher
//! demo's class reveals what was planted, and only that.

use mcq_analysis::{Adjustment, Anomaly, Copy, Decision, Finding, analyse, analyse_with};
use mcq_crypto::{KeyUnlock, OpeningKind};
use mcq_demo::{
    GALLERY, SOURCES, STUDENTS, TEACHER, build, build_all, build_model, class, gallery_index,
    identity, project,
};
use mcq_edit::qcmz;
use mcq_model::{Level, Paper, Severity};

fn paper(source: &mcq_demo::Source) -> Paper {
    mcq_format::read(source.paper).0
}

/// Every paper with a name for messages: the demos by language, the gallery by file stem.
fn every_paper() -> impl Iterator<Item = (String, &'static mcq_demo::Source)> {
    let demos = SOURCES.iter().chain(TEACHER.iter());
    demos.map(|s| (s.lang.to_string(), s)).chain(
        GALLERY
            .iter()
            .map(|m| (format!("{}-{}", m.stem, m.source.lang), &m.source)),
    )
}

/// What both languages of a paper share: per theme, each question's level, time, key and media.
type Shape = Vec<Vec<(Level, u32, Vec<bool>, usize)>>;

fn shape(p: &Paper) -> Shape {
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
                .collect()
        })
        .collect()
}

#[test]
fn papers_have_no_warning() {
    for (name, source) in every_paper() {
        let issues: Vec<_> = mcq_lint::lint(&paper(source))
            .into_iter()
            .filter(|i| i.severity != Severity::Info)
            .collect();
        assert!(issues.is_empty(), "{name}: {issues:?}");
    }
}

#[test]
fn sources_are_written_as_the_studio_writes_them() {
    // The visitor opens the paper in the Studio: its `.qcm.md` export must be this very file.
    for (name, source) in every_paper() {
        let canonical = mcq_format::write(&paper(source));
        let differs = canonical
            .lines()
            .zip(source.paper.lines())
            .enumerate()
            .find(|(_, (a, b))| a != b);
        assert!(differs.is_none(), "{name}: {differs:?}");
        assert_eq!(canonical, source.paper, "{name}");
    }
}

#[test]
fn languages_share_one_structure() {
    for sources in [&SOURCES, &TEACHER] {
        let papers: Vec<Paper> = sources.iter().map(paper).collect();
        assert_eq!(shape(&papers[0]), shape(&papers[1]));
        assert_eq!(papers[0].duration_minutes, papers[1].duration_minutes);
        for (source, p) in sources.iter().zip(&papers) {
            assert_eq!(p.lang, source.lang);
        }
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

#[test]
fn gallery_models_come_in_both_languages_with_one_structure() {
    let mut stems: Vec<&str> = GALLERY.iter().map(|m| m.stem).collect();
    stems.dedup();
    assert_eq!(
        stems.len() * 2,
        GALLERY.len(),
        "one English and one French per stem"
    );
    for pair in GALLERY.chunks(2) {
        let [en, fr] = pair else { unreachable!() };
        assert_eq!((en.stem, en.source.lang), (fr.stem, "en"), "{}", fr.stem);
        assert_eq!(fr.source.lang, "fr", "{}", fr.stem);
        let (a, b) = (paper(&en.source), paper(&fr.source));
        assert_eq!(shape(&a), shape(&b), "{}", en.stem);
        assert_eq!(a.duration_minutes, b.duration_minutes, "{}", en.stem);
        assert_eq!(
            (a.lang.as_str(), b.lang.as_str()),
            ("en", "fr"),
            "{}",
            en.stem
        );
        for p in [&a, &b] {
            assert!(!p.notes.trim().is_empty(), "{}: author notes", en.stem);
            assert!(p.duration_minutes > 0, "{}: duration", en.stem);
        }
    }
}

#[test]
fn gallery_is_rebuilt_identical_and_opens_without_a_code() {
    let fingerprint = build(&SOURCES[0]).expect("build").fingerprint;
    for model in &GALLERY {
        let name = format!("{}-{}", model.stem, model.source.lang);
        let (built, again) = (
            build_model(model).expect("build"),
            build_model(model).expect("build"),
        );
        assert!(built.package == again.package, "{name} package differs");
        assert!(built.qcmz == again.qcmz, "{name} qcmz differs");
        assert_eq!(
            built.fingerprint, fingerprint,
            "{name}: signed by the demo identity"
        );

        let header = mcq_crypto::inspect(&built.package).expect("signature");
        assert_eq!(header.opening, OpeningKind::Open, "{name}");
        mcq_crypto::open(&built.package, "").expect("open without code");
        let key = mcq_crypto::open_answer_key(&built.package, KeyUnlock::Practice).expect("key");
        assert_eq!(key.questions.len(), paper(&model.source).questions.len());

        let (imported, issues) = qcmz::import(&built.qcmz).expect("import");
        assert!(issues.is_empty(), "{name}: {issues:?}");
        assert_eq!(imported, project(&model.source), "{name}");
        assert_eq!(imported.source(), model.source.paper, "{name}");
    }
}

#[test]
fn gallery_index_lists_every_model_in_order() {
    let index = gallery_index();
    assert_eq!(index, gallery_index());
    let lines: Vec<&str> = index.lines().collect();
    assert_eq!(lines.len(), GALLERY.len() + 2);
    assert_eq!((lines[0], lines[lines.len() - 1]), ("[", "]"));
    let mut keys: Vec<(&str, &str)> = GALLERY.iter().map(|m| (m.stem, m.source.lang)).collect();
    let listed = keys.clone();
    keys.sort_unstable();
    assert_eq!(keys, listed, "GALLERY is sorted by stem then language");
    for (line, model) in lines[1..].iter().zip(&GALLERY) {
        let p = paper(&model.source);
        let expected = format!(
            "  {{\"stem\": \"{}\", \"lang\": \"{}\", \"title\": \"{}\", ",
            model.stem, model.source.lang, p.title
        );
        assert!(line.starts_with(&expected), "{line}");
        let tail = format!(
            "\"questions\": {}, \"duration\": {}}}",
            p.questions.len(),
            p.duration_minutes
        );
        assert!(line.trim_end_matches(',').ends_with(&tail), "{line}");
    }
}

/// The copies of the teacher demo, opened with the demo identity, and the key of its package.
fn opened(source: &mcq_demo::Source) -> (Paper, [u8; 16], Vec<Copy>, mcq_demo::Class) {
    let built = class(source).expect("class");
    let identity = identity();
    let header = mcq_crypto::inspect(&built.package.1).expect("signature");
    assert!(matches!(header.opening, OpeningKind::Code));
    let key = mcq_crypto::open_answer_key(&built.package.1, KeyUnlock::Teacher(&identity))
        .expect("the demo identity opens the key");
    let copies = built
        .copies
        .iter()
        .map(|(_, bytes)| {
            let (submission, receipt) =
                mcq_crypto::open_result(bytes, &identity).expect("the demo identity opens copies");
            Copy {
                submission,
                receipt,
            }
        })
        .collect();
    (key, header.paper_id, copies, built)
}

#[test]
fn teacher_demo_is_rebuilt_identical() {
    for source in &TEACHER {
        let (a, b) = (class(source).expect("class"), class(source).expect("class"));
        assert!(a.package == b.package, "{} package differs", source.lang);
        assert!(a.copies == b.copies, "{} copies differ", source.lang);
        assert_eq!(a.copies.len(), STUDENTS.len());
    }
}

#[test]
fn teacher_demo_reveals_what_was_planted_and_nothing_else() {
    for source in &TEACHER {
        let (key, paper_id, copies, built) = opened(source);
        let planted = &built.planted;
        let report = analyse(&key, paper_id, &copies);
        let lang = source.lang;
        assert!(!report.small_cohort, "{lang}");

        let doubts: Vec<_> = report
            .findings
            .iter()
            .filter_map(|f| match f {
                Finding::KeyDoubt {
                    question, rival, ..
                } => Some((question.clone(), rival.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(
            doubts,
            vec![(
                planted.faulty_question.clone(),
                Some(planted.faulty_answer.clone())
            )],
            "{lang}: {:#?}",
            report.findings
        );
        assert!(
            report.findings.iter().any(|f| matches!(f,
                Finding::Trap { question, option, .. }
                    if *question == planted.trap_question && *option == planted.trap_option)),
            "{lang}: {:#?}",
            report.findings
        );
        let reteach: Vec<&String> = report
            .findings
            .iter()
            .filter_map(|f| match f {
                Finding::Reteach { theme, .. } => Some(theme),
                _ => None,
            })
            .collect();
        assert_eq!(reteach, vec![&planted.weak_theme], "{lang}");

        let identical: Vec<(&String, &String)> = report
            .anomalies
            .iter()
            .filter_map(|a| match a {
                Anomaly::Identical { first, second, .. } => Some((first, second)),
                _ => None,
            })
            .collect();
        let (a, b) = &planted.copied;
        assert!(
            identical == vec![(a, b)] || identical == vec![(b, a)],
            "{lang}: {identical:?}"
        );
        let left: Vec<&String> = report
            .anomalies
            .iter()
            .filter_map(|a| match a {
                Anomaly::LeftWindow { name, .. } => Some(name),
                _ => None,
            })
            .collect();
        assert_eq!(left, vec![&planted.distracted], "{lang}");
        let late: Vec<&String> = report
            .anomalies
            .iter()
            .filter_map(|a| match a {
                Anomaly::Overtime { name, .. } => Some(name),
                _ => None,
            })
            .collect();
        assert_eq!(late, vec![&planted.late], "{lang}");

        // Accepting the rival as the key, as the Studio offers, lifts the doubt and the marks.
        let rekey = Adjustment {
            question: planted.faulty_question.clone(),
            decision: Decision::Rekey(vec![planted.faulty_answer.clone()]),
        };
        let fixed = analyse_with(&key, paper_id, &copies, &[rekey]);
        assert!(
            !fixed
                .findings
                .iter()
                .any(|f| matches!(f, Finding::KeyDoubt { .. })),
            "{lang}: {:#?}",
            fixed.findings
        );
        let mean = |r: &mcq_analysis::Report| {
            r.students.iter().map(|s| f64::from(s.mark)).sum::<f64>() / r.students.len() as f64
        };
        assert!(mean(&fixed) > mean(&report), "{lang}");
    }
}

#[test]
fn teacher_demo_opens_in_the_studio_unchanged() {
    for source in &TEACHER {
        let built = class(source).expect("class");
        assert_eq!(built.project, project(source));
        assert!(!built.project.media.is_empty());
    }
}
