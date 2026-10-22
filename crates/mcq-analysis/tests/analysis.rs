//! The class analysis: the earlier tool's tests, carried over, and a synthetic class whose planted
//! flaws must be found, every one and nothing else.

use mcq_analysis::{
    Adjustment, Anomaly, Copy, Finding, Report, analyse, analyse_with, csv, sheet, synth,
};
use mcq_model::{Answer, AnswerOption, Level, Paper, Question, Submission, Theme, TimeExtension};

const ID: [u8; 16] = [1; 16];

/// `n` questions over `t` themes (`t<i % t>`), four options, the first correct.
fn paper(n: usize, t: usize) -> Paper {
    let mut paper = Paper::new();
    paper.title = "Fixture".into();
    for k in 0..t {
        paper.themes.push(Theme {
            id: format!("t{k}"),
            title: format!("Theme {k}"),
            ..Theme::default()
        });
    }
    for i in 0..n {
        let id = format!("q{i}");
        paper.questions.push(Question {
            options: (0..4)
                .map(|o| AnswerOption {
                    id: format!("{id}-o{}", o + 1),
                    text: format!("Option {o}"),
                    is_correct: o == 0,
                    ..AnswerOption::default()
                })
                .collect(),
            theme_id: format!("t{}", i % t),
            level: Level::Apply,
            id,
            ..Question::default()
        });
    }
    paper
}

/// A copy picking, per question, the option at that index (`None` for a blank).
fn copy(paper: &Paper, name: &str, picks: &[Option<usize>], elapsed: u32) -> Copy {
    Copy {
        submission: Submission {
            paper_id: ID,
            student_name: name.into(),
            answers: paper
                .questions
                .iter()
                .zip(picks)
                .map(|(q, pick)| Answer {
                    question_id: q.id.clone(),
                    selected: pick
                        .map(|k| vec![q.options[k].id.clone()])
                        .unwrap_or_default(),
                    seconds_spent: 30,
                })
                .collect(),
            elapsed_seconds: elapsed,
            ..Submission::default()
        },
        receipt: format!("R-{name}"),
    }
}

/// A copy with `right` correct answers then wrong ones (option 2).
fn scored(paper: &Paper, name: &str, right: usize) -> Copy {
    let picks: Vec<Option<usize>> = (0..paper.questions.len())
        .map(|i| Some(if i < right { 0 } else { 1 }))
        .collect();
    copy(paper, name, &picks, 600)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn copies_are_graded_again_from_the_key() {
    let p = paper(6, 2);
    let report = analyse(
        &p,
        ID,
        &[scored(&p, "A", 6), scored(&p, "B", 3), scored(&p, "C", 0)],
    );
    let marks: Vec<f32> = report.students.iter().map(|s| s.mark).collect();
    assert_eq!(marks, vec![20.0, 10.0, 0.0]);
    assert!(close(report.mean_mark.unwrap_or(0.0), 10.0));
    assert!(close(report.median_mark.unwrap_or(0.0), 10.0));
    assert!(report.small_cohort);
    assert!(!report.items_computed);
}

#[test]
fn a_theme_nobody_got_is_to_reteach() {
    let p = paper(6, 2);
    // Right on t0 (even questions), wrong on t1.
    let picks: Vec<Option<usize>> = (0..6)
        .map(|i| Some(if i % 2 == 0 { 0 } else { 1 }))
        .collect();
    let copies: Vec<Copy> = (0..4)
        .map(|k| copy(&p, &format!("S{k}"), &picks, 600))
        .collect();
    let report = analyse(&p, ID, &copies);
    let t0 = &report.themes[0];
    let t1 = &report.themes[1];
    assert!(close(t0.mean_rate, 1.0));
    assert_eq!(t0.acquired, 4);
    assert!(close(t1.mean_rate, 0.0));
    assert_eq!(t1.to_relearn, 4);
    assert!(t1.needs_reteaching() && !t0.needs_reteaching());
    let reteach: Vec<&Finding> = report
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::Reteach { .. }))
        .collect();
    assert_eq!(
        reteach,
        vec![&Finding::Reteach {
            theme: "t1".into(),
            mean_rate: 0.0,
            doubtful: Vec::new(),
        }]
    );
}

#[test]
fn a_distractor_chosen_by_many_is_a_shared_confusion() {
    let p = paper(1, 1);
    let mut copies = Vec::new();
    for k in 0..8 {
        let pick = if k < 6 { 2 } else { 0 };
        copies.push(copy(&p, &format!("S{k}"), &[Some(pick)], 600));
    }
    let report = analyse(&p, ID, &copies);
    assert!(close(report.questions[0].options[2].rate, 0.75));
    assert!(report.findings.iter().any(|f| matches!(f,
        Finding::Trap { option, rate, .. } if option == "q0-o3" && close(*rate, 0.75))));
}

#[test]
fn distractors_nobody_picks_are_dead() {
    let p = paper(1, 1);
    let copies: Vec<Copy> = (0..8)
        .map(|k| copy(&p, &format!("S{k}"), &[Some(0)], 600))
        .collect();
    let report = analyse(&p, ID, &copies);
    assert!(report.findings.contains(&Finding::DeadDistractors {
        question: "q0".into(),
        count: 3
    }));
    assert!(report.findings.contains(&Finding::TooEasy {
        question: "q0".into(),
        facility: 1.0
    }));
}

#[test]
fn discrimination_needs_eight_copies() {
    let p = paper(6, 2);
    let copies: Vec<Copy> = (0..10)
        .map(|k| scored(&p, &format!("S{k}"), if k < 5 { 6 } else { 0 }))
        .collect();
    let report = analyse(&p, ID, &copies);
    assert!(close(
        report.questions[0].discrimination.unwrap_or(0.0),
        1.0
    ));
    assert!(
        !report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::KeyDoubt { .. }))
    );

    let two = analyse(&p, ID, &copies[..2]);
    assert_eq!(two.questions[0].discrimination, None);
    assert_eq!(two.reliability, None);
}

#[test]
fn odd_copies_are_reported_never_judged() {
    let p = paper(20, 2);
    let mut other = scored(&p, "Other", 20);
    other.submission.paper_id = [9; 16];
    let fast = scored(&p, "Quick", 20);
    let mut fast = fast;
    fast.submission.elapsed_seconds = 20;
    let mut blank_fast = scored(&p, "Blank", 0);
    blank_fast.submission.elapsed_seconds = 20;
    let normal = scored(&p, "Normal", 20);
    let report = analyse(
        &p,
        ID,
        &[
            other,
            fast,
            blank_fast,
            normal,
            scored(&p, "dupont marie", 3),
            scored(&p, "Dupont  Marie", 3),
        ],
    );
    assert_eq!(report.students.len(), 5, "the other paper is left out");
    let a = &report.anomalies;
    assert!(a.contains(&Anomaly::OtherPaper {
        name: "Other".into(),
        receipt: "R-Other".into()
    }));
    assert!(a.contains(&Anomaly::Fast {
        name: "Quick".into(),
        elapsed_seconds: 20
    }));
    assert!(
        !a.iter()
            .any(|x| matches!(x, Anomaly::Fast { name, .. } if name != "Quick"))
    );
    assert!(
        a.iter()
            .any(|x| matches!(x, Anomaly::SameName { count: 2, .. }))
    );
}

#[test]
fn spreadsheets_open_in_french_spreadsheet_software() {
    let p = paper(4, 1);
    let report = analyse(&p, ID, &[scored(&p, "Dupont Marie", 3)]);
    let notes = csv::notes(&p, &report, &csv::FR);
    assert!(notes.starts_with("\u{feff}Nom;Note /20;"), "{notes}");
    assert!(notes.contains("\nDupont Marie;15,0;3;1;0;"), "{notes}");
    assert!(!notes.contains('\r'));
    let themes = csv::themes(&report, &csv::FR);
    assert!(themes.contains("Theme 0;4;1;75,0;1;0;0;0,0;;"), "{themes}");
    let questions = csv::questions(&p, &report, &csv::EN);
    assert!(
        questions.contains("QUESTION;q0;t0;3;100.0;0;"),
        "{questions}"
    );
    assert!(questions.contains("\n  RIGHT;q0;"), "{questions}");
}

#[test]
fn a_revision_sheet_lists_mistakes_and_where_to_start() {
    let p = paper(6, 2);
    let picks = [Some(0), Some(1), Some(0), None, Some(0), Some(2)];
    let c = copy(&p, "Alex", &picks, 600);
    let report = analyse(&p, ID, std::slice::from_ref(&c));
    let s = sheet::revision_sheet(&p, &report, &report.students[0], &c);
    let ids: Vec<&str> = s.mistakes.iter().map(|m| m.question.id.as_str()).collect();
    assert_eq!(ids, vec!["q1", "q3", "q5"]);
    assert_eq!(s.mistakes[1].chosen, Vec::<String>::new(), "a blank");
    assert_eq!(s.start_with, vec!["Theme 1".to_string()]);
    assert_eq!(s.themes.len(), 2);
}

fn planted_findings(report: &Report, planted: &synth::Planted) -> Vec<String> {
    let mut missing = Vec::new();
    if !report.findings.iter().any(|f| {
        matches!(f,
        Finding::KeyDoubt { question, rival, .. }
            if *question == planted.faulty_question
                && rival.as_deref() == Some(&*planted.faulty_answer))
    }) {
        missing.push("doubtful key".to_string());
    }
    if !report.findings.iter().any(|f| {
        matches!(f,
        Finding::Trap { question, option, .. }
            if *question == planted.trap_question && *option == planted.trap_option)
    }) {
        missing.push("trap".to_string());
    }
    if !report
        .findings
        .iter()
        .any(|f| matches!(f, Finding::Reteach { theme, .. } if *theme == planted.weak_theme))
    {
        missing.push("weak theme".to_string());
    }
    let (a, b) = &planted.copied;
    if !report.anomalies.iter().any(|x| {
        matches!(x,
        Anomaly::Identical { first, second, .. } if first == a && second == b)
    }) {
        missing.push("copied pair".to_string());
    }
    if !report.anomalies.iter().any(|x| {
        matches!(x,
        Anomaly::LeftWindow { name, .. } if *name == planted.distracted)
    }) {
        missing.push("left window".to_string());
    }
    if !report.anomalies.iter().any(|x| {
        matches!(x,
        Anomaly::Overtime { name, .. } if *name == planted.late)
    }) {
        missing.push("overtime".to_string());
    }
    missing
}

#[test]
fn a_synthetic_class_reveals_exactly_what_was_planted() {
    let (paper, submissions, planted) = synth::cohort(2026, 40);
    let copies: Vec<Copy> = submissions
        .into_iter()
        .map(|submission| Copy {
            receipt: submission.student_name.clone(),
            submission,
        })
        .collect();
    let report = analyse(&paper, synth::PAPER_ID, &copies);
    assert_eq!(planted_findings(&report, &planted), Vec::<String>::new());

    // Nothing else is accused: one doubtful key, one copied pair, one window, one overtime.
    let doubts = report
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::KeyDoubt { .. }))
        .count();
    assert_eq!(doubts, 1, "{:#?}", report.findings);
    let identical = report
        .anomalies
        .iter()
        .filter(|a| matches!(a, Anomaly::Identical { .. }))
        .count();
    assert_eq!(identical, 1, "{:#?}", report.anomalies);
    let reteach: Vec<&Finding> = report
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::Reteach { .. }))
        .collect();
    assert_eq!(reteach.len(), 1, "{reteach:#?}");
    // Hard questions spread their wrong answers: no confusion is claimed in the weak theme, and
    // the doubtful key is the only thing said about its question.
    assert!(!report.findings.iter().any(|f| matches!(f,
        Finding::Trap { question, .. } if question.starts_with(&planted.weak_theme))));
    let about_faulty = report
        .findings
        .iter()
        .filter(|f| format!("{f:?}").contains(&format!("\"{}\"", planted.faulty_question)))
        .count();
    assert_eq!(about_faulty, 1, "{:#?}", report.findings);

    // Revision sheets never assert a key the analysis doubts.
    for (student, copy) in report.students.iter().zip({
        let mut sorted: Vec<&Copy> = copies.iter().collect();
        sorted.sort_by_key(|c| c.submission.student_name.to_lowercase());
        sorted
    }) {
        let s = sheet::revision_sheet(&paper, &report, student, copy);
        for m in &s.mistakes {
            assert_eq!(m.key_in_doubt, m.question.id == planted.faulty_question);
        }
    }
    assert!(!report.small_cohort);
    let alpha = report.reliability.unwrap_or(0.0);
    assert!((0.5..1.0).contains(&alpha), "KR-20 {alpha}");

    // The same copies always give the same report.
    assert_eq!(analyse(&paper, synth::PAPER_ID, &copies), report);
}

#[test]
fn planted_flaws_are_found_whatever_the_seed() {
    for seed in [1, 7, 42, 1000, 31337] {
        let (paper, submissions, planted) = synth::cohort(seed, 36);
        let copies: Vec<Copy> = submissions
            .into_iter()
            .map(|submission| Copy {
                receipt: submission.student_name.clone(),
                submission,
            })
            .collect();
        let report = analyse(&paper, synth::PAPER_ID, &copies);
        assert_eq!(
            planted_findings(&report, &planted),
            Vec::<String>::new(),
            "seed {seed}"
        );
    }
}

#[test]
fn a_neutralized_question_leaves_the_marks_not_the_statistics() {
    let p = paper(6, 2);
    // Everyone misses q5: graded on the five others.
    let copies = [scored(&p, "A", 5), scored(&p, "B", 2)];
    let report = analyse_with(&p, ID, &copies, &[Adjustment::neutralize("q5")]);
    let marks: Vec<f32> = report.students.iter().map(|s| s.mark).collect();
    assert_eq!(marks, vec![20.0, 8.0]);
    assert_eq!(report.students[0].grade.total, 5);
    let q5 = &report.questions[5];
    assert_eq!((q5.asked, q5.facility), (2, 0.0), "statistics kept");
    assert!(
        !report
            .findings
            .iter()
            .any(|f| format!("{f:?}").contains("\"q5\"")),
        "{:#?}",
        report.findings
    );
    // Theme 1 holds q1, q3, q5: two questions left in the grading.
    assert_eq!(report.themes[1].questions, 2);
    let questions = csv::questions(&p, &report, &csv::FR);
    assert!(
        questions.contains("QUESTION;q5;t1;3;0,0;0;30,0;;;NEUTRALISÉE : hors barème;"),
        "{questions}"
    );
    let sheet = sheet::revision_sheet(&p, &report, &report.students[1], &copies[1]);
    assert!(sheet.mistakes.iter().all(|m| m.question.id != "q5"));

    // Undoing is dropping the decision: the report is the plain one again.
    assert_eq!(analyse_with(&p, ID, &copies, &[]), analyse(&p, ID, &copies));
}

#[test]
fn a_corrected_key_marks_every_copy_again() {
    let (paper, submissions, planted) = synth::cohort(2026, 40);
    let copies: Vec<Copy> = submissions
        .into_iter()
        .map(|submission| Copy {
            receipt: submission.student_name.clone(),
            submission,
        })
        .collect();
    let faulty = planted.faulty_question.as_str();
    let rival = format!("{faulty}-o2");
    let before = analyse(&paper, synth::PAPER_ID, &copies);
    let after = analyse_with(
        &paper,
        synth::PAPER_ID,
        &copies,
        &[Adjustment::rekey(faulty, &[&rival])],
    );
    assert_eq!(after.adjustments.len(), 1);
    // The strong students were right: with their answer as the key, the doubt is gone and the
    // class does better.
    assert!(
        !after
            .findings
            .iter()
            .any(|f| matches!(f, Finding::KeyDoubt { .. })),
        "{:#?}",
        after.findings
    );
    assert!(after.mean_mark > before.mean_mark);
    let stats = after.questions.iter().find(|q| q.id == faulty);
    let right: Vec<&str> = stats
        .map(|q| {
            q.options
                .iter()
                .filter(|o| o.is_correct)
                .map(|o| o.id.as_str())
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(right, vec![rival.as_str()]);
    assert!(
        csv::questions(&paper, &after, &csv::EN).contains("KEY CORRECTED"),
        "the spreadsheet says so"
    );

    // A sheet shows the key the copy was marked with, and no longer doubts it.
    for (student, copy) in after.students.iter().zip({
        let mut sorted: Vec<&Copy> = copies.iter().collect();
        sorted.sort_by_key(|c| c.submission.student_name.to_lowercase());
        sorted
    }) {
        let s = sheet::revision_sheet(&paper, &after, student, copy);
        for m in s.mistakes.iter().filter(|m| m.question.id == faulty) {
            assert!(!m.key_in_doubt);
            assert!(m.question.option(&rival).is_some_and(|o| o.is_correct));
        }
    }

    // Neutralizing it instead: no finding on it, and the paper stays reliable.
    let neutral = analyse_with(
        &paper,
        synth::PAPER_ID,
        &copies,
        &[Adjustment::neutralize(faulty)],
    );
    assert!(
        neutral
            .students
            .iter()
            .all(|s| s.grade.total as usize == paper.questions.len() - 1)
    );
    assert!(neutral.reliability.unwrap_or(0.0) >= before.reliability.unwrap_or(1.0));
}

#[test]
fn a_timeline_replays_answers_changes_and_absences() {
    use mcq_analysis::timeline::{Change, Event, timeline};
    use mcq_model::{LogEntry, LogKind};
    let p = paper(3, 1);
    let mut c = copy(&p, "Alex", &[Some(1), Some(0), None], 600);
    let e = |s, k, q: &str, o: &str| LogEntry::new(s, k, q, o);
    c.submission.log = vec![
        e(10, LogKind::Tick, "q0", "q0-o1"),
        // Right, then wrong: one click unticks and ticks in the same second.
        e(40, LogKind::Untick, "q0", "q0-o1"),
        e(40, LogKind::Tick, "q0", "q0-o2"),
        e(50, LogKind::FocusLost, "", ""),
        e(110, LogKind::FocusBack, "", ""),
        e(120, LogKind::Tick, "q1", "q1-o3"),
        e(130, LogKind::Untick, "q1", "q1-o3"),
        e(130, LogKind::Tick, "q1", "q1-o1"),
        e(140, LogKind::Tick, "q2", "q2-o2"),
        e(150, LogKind::Untick, "q2", "q2-o2"),
        e(590, LogKind::FocusLost, "", ""),
        e(600, LogKind::Submit, "", ""),
    ];
    let t = timeline(&p, &c.submission);
    assert_eq!(
        (t.right_to_wrong, t.wrong_to_right, t.wrong_to_wrong),
        (1, 1, 0)
    );
    assert_eq!((t.away_count, t.away_seconds), (2, 60));
    let kinds: Vec<String> = t
        .moments
        .iter()
        .map(|m| match &m.event {
            Event::Answer {
                question,
                right,
                change,
                ..
            } => format!("{}:Q{question}:{right}:{change:?}", m.seconds),
            Event::Cleared { question, .. } => format!("{}:Q{question}:cleared", m.seconds),
            Event::Away { seconds } => format!("{}:away {seconds}", m.seconds),
            Event::Submitted => format!("{}:submit", m.seconds),
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            "10:Q1:true:None",
            format!("40:Q1:false:{:?}", Some(Change::RightToWrong)).as_str(),
            "50:away 60",
            "120:Q2:false:None",
            format!("130:Q2:true:{:?}", Some(Change::WrongToRight)).as_str(),
            "140:Q3:false:None",
            "150:Q3:cleared",
            "590:away 0",
            "600:submit",
        ]
    );
}

#[test]
fn declared_extra_time_is_shown_and_moves_the_overtime() {
    let mut p = paper(4, 1);
    p.duration_minutes = 30;
    let mut third = copy(&p, "Alex Example", &[Some(0); 4], 2300);
    third.submission.extension = TimeExtension::Third;
    let mut late_third = copy(&p, "Sam Example", &[Some(0); 4], 2500);
    late_third.submission.extension = TimeExtension::Third;
    let plain = copy(&p, "Kim Example", &[Some(0); 4], 2300);
    let report = analyse(&p, ID, &[third, late_third, plain]);

    let overtime: Vec<(&str, u32)> = report
        .anomalies
        .iter()
        .filter_map(|a| match a {
            Anomaly::Overtime { name, seconds } => Some((name.as_str(), *seconds)),
            _ => None,
        })
        .collect();
    assert_eq!(overtime, [("Kim Example", 500), ("Sam Example", 100)]);
    let declared: Vec<&str> = report
        .anomalies
        .iter()
        .filter_map(|a| match a {
            Anomaly::ExtraTime { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(declared, ["Alex Example", "Sam Example"]);
    let alex = report.students.iter().find(|s| s.name == "Alex Example");
    assert_eq!(alex.map(|s| s.extension), Some(TimeExtension::Third));

    let notes = csv::notes(&p, &report, &csv::FR);
    assert!(notes.contains("temps majoré +1/3"), "{notes}");
}
