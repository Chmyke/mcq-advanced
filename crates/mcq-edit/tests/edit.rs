//! Editing operations keep the paper writable and lossless; `.qcmz` files carry it with its media.

use std::io::{Cursor, Write};

use mcq_edit::ops::{self, IdError, Kind};
use mcq_edit::qcmz::{self, QcmzError};
use mcq_edit::{HEAVY_MEDIA_BYTES, Project};
use mcq_model::{Block, Level, Media, MediaKind, Paper, Severity};
use zip::write::SimpleFileOptions;

const SAMPLE: &str = include_str!("../../mcq-format/tests/fixtures/sample.qcm.md");

fn without_lines(mut paper: Paper) -> Paper {
    for q in &mut paper.questions {
        q.line = 0;
    }
    paper
}

/// What the editor holds must survive being written and read back.
fn assert_lossless(paper: &Paper) {
    let text = mcq_format::write(paper);
    let (again, issues) = mcq_format::read(&text);
    assert!(
        issues.iter().all(|i| i.severity != Severity::Error),
        "{issues:#?}\n{text}"
    );
    assert_eq!(
        without_lines(again),
        without_lines(paper.clone()),
        "\n{text}"
    );
}

fn sample() -> Paper {
    without_lines(mcq_format::read(SAMPLE).0)
}

fn filled(paper: &mut Paper, id: &str) {
    let q = paper
        .questions
        .iter_mut()
        .find(|q| q.id == id)
        .expect("question");
    q.body = vec![Block::Text("Which one?".into())];
    for (k, option) in q.options.iter_mut().enumerate() {
        option.text = format!("Choice {k}");
    }
    q.options[0].is_correct = true;
}

#[test]
fn a_starter_paper_round_trips_once_filled() {
    let mut project = Project::starter("Week 1", "en");
    let id = project.paper.questions[0].id.clone();
    assert_eq!(id, "theme-1-1");
    assert_eq!(project.paper.questions[0].options[3].id, "theme-1-1-o4");
    filled(&mut project.paper, &id);
    assert_lossless(&project.paper);
    let errors: Vec<_> = project
        .issues()
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn questions_are_added_moved_and_removed_inside_their_theme() {
    let mut paper = sample();
    let theme = paper.themes[0].id.clone();
    let other = paper.themes[1].id.clone();
    let first = paper.questions[0].id.clone();

    let added = ops::add_question(&mut paper, &theme);
    let last_of_theme = paper
        .questions
        .iter()
        .rposition(|q| q.theme_id == theme)
        .expect("theme");
    assert_eq!(paper.questions[last_of_theme].id, added);
    assert!(ops::next_question_id(&paper, &theme) != added);

    let copy = ops::duplicate_question(&mut paper, &first).expect("copy");
    assert_eq!(paper.questions[1].id, copy);
    assert_eq!(paper.questions[1].body, paper.questions[0].body);
    assert!(paper.questions[1].options[0].id.starts_with(&copy));

    assert!(
        !ops::move_question(&mut paper, &first, true),
        "already first"
    );
    assert!(ops::move_question(&mut paper, &first, false));
    assert_eq!(paper.questions[1].id, first);

    assert!(ops::set_question_theme(&mut paper, &added, &other));
    assert_eq!(
        paper.questions.last().map(|q| q.id.as_str()),
        Some(added.as_str())
    );
    assert!(!ops::set_question_theme(&mut paper, &added, "nowhere"));

    filled(&mut paper, &added);
    assert_lossless(&paper);
    assert!(ops::remove_question(&mut paper, &copy));
    assert!(!ops::remove_question(&mut paper, &copy));
    assert_lossless(&paper);
}

#[test]
fn options_keep_positional_ids() {
    let mut paper = sample();
    let q = &mut paper.questions[0];
    let id = q.id.clone();
    ops::add_option(q);
    let n = q.options.len();
    assert_eq!(q.options[n - 1].id, format!("{id}-o{n}"));
    let second = q.options[1].text.clone();
    assert!(ops::move_option(q, 1, true));
    assert_eq!(q.options[0].text, second);
    assert_eq!(q.options[0].id, format!("{id}-o1"));
    ops::remove_option(q, 0);
    assert_eq!(q.options.len(), n - 1);
    assert!(
        q.options
            .iter()
            .enumerate()
            .all(|(k, o)| o.id == format!("{id}-o{}", k + 1))
    );
}

#[test]
fn types_follow_the_options() {
    let mut paper = sample();
    let q = &mut paper.questions[1];
    assert_eq!(ops::kind(q), Kind::Multiple);
    q.declared_answers = 2;
    ops::set_kind(q, Kind::Single, "en");
    assert_eq!(q.correct_count(), 1);
    assert_eq!(q.declared_answers, 0);

    ops::toggle_correct(q, 2, true);
    assert_eq!(q.correct_count(), 1);
    assert!(q.options[2].is_correct);
    ops::toggle_correct(q, 2, true);
    assert_eq!(q.correct_count(), 0, "ticking the ticked option unticks it");
    ops::toggle_correct(q, 0, false);
    ops::toggle_correct(q, 1, false);
    assert_eq!(ops::kind(q), Kind::Multiple);

    ops::set_kind(q, Kind::TrueFalse, "fr");
    assert_eq!(ops::kind(q), Kind::TrueFalse);
    assert_eq!(q.options[0].text, "Vrai");
    assert_eq!(q.options[1].id, format!("{}-o2", q.id));
    ops::toggle_correct(q, 1, true);
    ops::set_kind(q, Kind::TrueFalse, "fr");
    assert!(q.options[1].is_correct, "already true or false: kept");
    assert_lossless(&paper);
}

#[test]
fn ids_are_checked_before_renaming() {
    let mut paper = sample();
    let a = paper.questions[0].id.clone();
    let b = paper.questions[1].id.clone();
    assert_eq!(
        ops::rename_question(&mut paper, &a, ""),
        Err(IdError::Empty)
    );
    assert_eq!(
        ops::rename_question(&mut paper, &a, "x y"),
        Err(IdError::BadCharacter)
    );
    assert_eq!(
        ops::rename_question(&mut paper, &a, "x|y"),
        Err(IdError::BadCharacter)
    );
    assert_eq!(
        ops::rename_question(&mut paper, &a, &b),
        Err(IdError::Taken)
    );
    assert_eq!(ops::rename_question(&mut paper, &a, "fresh"), Ok(()));
    assert_eq!(paper.questions[0].options[0].id, "fresh-o1");
    assert_lossless(&paper);
}

#[test]
fn themes_get_ids_from_their_titles() {
    let mut paper = Paper::new();
    assert_eq!(
        ops::add_theme(&mut paper, "Boucles et tableaux"),
        "boucles-et-tableaux"
    );
    assert_eq!(
        ops::add_theme(&mut paper, "Boucles & tableaux"),
        "boucles-tableaux"
    );
    assert_eq!(
        ops::add_theme(&mut paper, "Boucles et tableaux"),
        "boucles-et-tableaux-2"
    );
    assert_eq!(ops::add_theme(&mut paper, "  "), "theme");
    let q = ops::add_question(&mut paper, "boucles-tableaux");
    assert!(
        !ops::remove_theme(&mut paper, "boucles-tableaux"),
        "holds a question"
    );
    assert!(ops::move_theme(&mut paper, "boucles-tableaux", true));
    assert_eq!(paper.themes[0].id, "boucles-tableaux");
    assert!(ops::remove_question(&mut paper, &q));
    assert!(ops::remove_theme(&mut paper, "boucles-tableaux"));
}

#[test]
fn every_level_and_time_round_trips() {
    let mut paper = sample();
    for (k, level) in [Level::Remember, Level::Evaluate, Level::Unknown]
        .into_iter()
        .enumerate()
    {
        paper.questions[0].level = level;
        paper.questions[0].estimated_seconds = 15 * k as u32;
        assert_lossless(&paper);
    }
}

fn project_with_media() -> Project {
    let mut project = Project::default();
    project.set_source(SAMPLE);
    project.paper = without_lines(project.paper);
    for path in project.used_media() {
        project.media.insert(path.clone(), path.into_bytes());
    }
    project
}

#[test]
fn qcmz_round_trips_the_paper_and_its_media() {
    let project = project_with_media();
    assert!(!project.media.is_empty());
    let bytes = qcmz::export(&project).expect("export");
    let (back, issues) = qcmz::import(&bytes).expect("import");
    assert!(issues.is_empty(), "{issues:#?}");
    assert_eq!(without_lines(back.paper.clone()), project.paper);
    assert_eq!(back.media, project.media);
    assert_eq!(
        qcmz::export(&back).expect("export"),
        bytes,
        "export is stable"
    );
}

fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in files {
        zip.start_file(*name, SimpleFileOptions::default())
            .expect("start");
        zip.write_all(data).expect("write");
    }
    zip.finish().expect("finish").into_inner()
}

#[test]
fn a_zipped_folder_opens_and_nothing_escapes_it() {
    let paper =
        b"---\ntitle: T\n---\n# THEME t\n## Q a\nSee?\n![A cat](media/cat.png)\n- [x] Y\n- [ ] N\n";
    let bytes = zip_of(&[
        ("My paper/my paper.qcm.md", paper),
        ("My paper/media/cat.png", b"png"),
        ("My paper/media/readme.txt", b"not a media"),
        ("My paper/other/dog.png", b"outside media"),
        ("../evil/media/x.png", b"escape"),
        ("/abs/media/y.png", b"absolute"),
    ]);
    let (project, issues) = qcmz::import(&bytes).expect("import");
    assert!(issues.is_empty(), "{issues:#?}");
    assert_eq!(project.paper.title, "T");
    assert_eq!(
        project.media.keys().cloned().collect::<Vec<_>>(),
        vec!["media/cat.png".to_string()]
    );
    assert_eq!(project.paper.questions[0].media().count(), 1);
}

#[test]
fn ambiguous_or_broken_files_are_refused() {
    assert_eq!(qcmz::import(b"not a zip").err(), Some(QcmzError::NotAZip));
    assert_eq!(
        qcmz::import(&zip_of(&[("media/a.png", b"x")])).err(),
        Some(QcmzError::NoPaper)
    );
    assert_eq!(
        qcmz::import(&zip_of(&[("a.qcm.md", b"x"), ("b.qcm.md", b"y")])).err(),
        Some(QcmzError::SeveralPapers)
    );
    assert_eq!(
        qcmz::import(&zip_of(&[("a.qcm.md", &[0xff, 0xfe, 0x00])])).err(),
        Some(QcmzError::NotText)
    );
    // A plain `.md` is accepted when it is the only paper.
    assert!(qcmz::import(&zip_of(&[("notes.md", b"---\ntitle: X\n---\n")])).is_ok());
}

#[test]
fn missing_and_heavy_media_are_reported() {
    let mut project = project_with_media();
    let used = project.used_media();
    project.media.remove(&used[0]);
    project
        .media
        .insert("media/big.mp4".into(), vec![0; HEAVY_MEDIA_BYTES + 1]);
    let codes: Vec<&str> = project.issues().iter().map(|i| i.code).collect();
    assert!(codes.contains(&"MEDIA_NOT_FOUND"), "{codes:?}");
    assert!(codes.contains(&"HEAVY_MEDIA"), "{codes:?}");

    let q = &mut project.paper.questions[0];
    q.body.push(Block::Media(Media {
        kind: MediaKind::Image,
        path: "media/x.png".into(),
        alt: String::new(),
    }));
    let codes: Vec<&str> = project.issues().iter().map(|i| i.code).collect();
    assert!(codes.contains(&"MEDIA_WITHOUT_ALT"), "{codes:?}");
}
