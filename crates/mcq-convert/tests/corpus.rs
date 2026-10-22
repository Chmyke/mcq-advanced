//! The Moodle round trip on the real papers of the author, when they are on this machine.
//!
//! Real papers never enter the repository: `_prive/corpus-local.txt` (private, absent on CI)
//! lists their folders. Without it this test checks nothing and says so.

use std::path::{Path, PathBuf};

use mcq_convert::{ImportOptions, Source, import, moodle};
use mcq_edit::Project;
use mcq_model::{Paper, Scoring, Severity};

fn corpus() -> Vec<PathBuf> {
    let list = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../_prive/corpus-local.txt");
    let Ok(text) = std::fs::read_to_string(list) else {
        return Vec::new();
    };
    let mut papers = Vec::new();
    for folder in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let Ok(entries) = std::fs::read_dir(folder) else {
            continue;
        };
        for path in entries.filter_map(Result::ok).map(|e| e.path()) {
            let name = path.to_string_lossy().into_owned();
            if name.ends_with(".md") && !name.ends_with(" - Sujet.md") {
                papers.push(path);
            }
        }
    }
    papers
}

/// What a question bank keeps of a paper (`docs/exchange.md`).
fn bank_view(paper: &Paper) -> Paper {
    let mut expected = paper.clone();
    expected.duration_minutes = 0;
    expected.penalty = 0.0;
    expected.unanswered_penalty = 0.0;
    expected.shuffle_questions = true;
    expected.show_answer_count = true;
    expected.notes.clear();
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
fn real_papers_survive_the_moodle_round_trip() {
    let papers = corpus();
    if papers.is_empty() {
        eprintln!("no local corpus: nothing checked");
        return;
    }
    for path in papers {
        let source = std::fs::read_to_string(&path).expect("readable paper");
        let (paper, _) = mcq_format::read(&source);
        let project = Project {
            paper,
            ..Project::default()
        };
        let exported = moodle::export(&project);
        let options = ImportOptions {
            title: String::new(),
            lang: project.paper.lang.clone(),
        };
        let back = import(Source::MoodleXml, &exported.xml, &options);
        assert!(
            !back.issues.iter().any(|i| i.severity != Severity::Info),
            "{}: {:?}",
            path.display(),
            back.issues
        );
        let expected = bank_view(&project.paper);
        for (got, want) in back.project.paper.questions.iter().zip(&expected.questions) {
            assert_eq!(got, want, "{}", path.display());
        }
        assert_eq!(back.project.paper, expected, "{}", path.display());
        assert_eq!(moodle::export(&back.project).xml, exported.xml);
    }
}
