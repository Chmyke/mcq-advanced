//! Grading scenarios, ported from the Unity tool's tests and extended to partial scoring.

use mcq_grade::{Answer, GuessAdvice, Mastery, Status, grade, guess_advice, score};
use mcq_model::{AnswerOption, Level, Paper, Question, Scoring, Theme, TimeExtension};

const EPS: f32 = 1e-5;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < EPS
}

/// `n` questions `q<i>` spread over `t` themes `t<i % t>`, 4 options, the first one correct.
fn paper(n: usize, t: usize) -> Paper {
    let mut paper = Paper::new();
    for i in 0..t {
        paper.themes.push(Theme {
            id: format!("t{i}"),
            title: format!("Theme {i}"),
            course_ref: "Chapter".into(),
            ..Theme::default()
        });
    }
    for i in 0..n {
        let id = format!("q{i}");
        let options = (0..4)
            .map(|k| AnswerOption {
                id: format!("{id}-o{}", k + 1),
                text: format!("Option {k}"),
                is_correct: k == 0,
                explanation: "Because.".into(),
            })
            .collect();
        paper.questions.push(Question {
            id,
            theme_id: format!("t{}", i % t),
            level: Level::Apply,
            options,
            ..Question::default()
        });
    }
    paper
}

fn tick(q: usize, option: usize) -> Answer {
    Answer {
        question_id: format!("q{q}"),
        selected: vec![format!("q{q}-o{}", option + 1)],
        seconds_spent: 10,
    }
}

fn blank(q: usize) -> Answer {
    Answer {
        question_id: format!("q{q}"),
        ..Answer::default()
    }
}

#[test]
fn all_correct_is_twenty() {
    let p = paper(4, 2);
    let answers: Vec<_> = (0..4).map(|i| tick(i, 0)).collect();
    let r = grade(&p, &answers, 0, TimeExtension::None);
    assert_eq!((r.correct, r.wrong, r.unanswered), (4, 0, 0));
    assert!(close(r.mark(20.0), 20.0));
}

#[test]
fn all_wrong_is_zero() {
    let p = paper(4, 2);
    let answers: Vec<_> = (0..4).map(|i| tick(i, 1)).collect();
    let r = grade(&p, &answers, 0, TimeExtension::None);
    assert_eq!(r.wrong, 4);
    assert!(close(r.mark(20.0), 0.0));
}

#[test]
fn unanswered_counts_separately() {
    let p = paper(4, 2);
    let r = grade(
        &p,
        &[tick(0, 0), tick(1, 1), blank(2), blank(3)],
        0,
        TimeExtension::None,
    );
    assert_eq!((r.correct, r.wrong, r.unanswered, r.total), (1, 1, 2, 4));
}

#[test]
fn penalty_only_bites_wrong_answers() {
    let mut p = paper(4, 2);
    p.penalty = 0.5;
    let r = grade(
        &p,
        &[tick(0, 0), tick(1, 0), tick(2, 1), blank(3)],
        0,
        TimeExtension::None,
    );
    assert!(close(r.net_score, 1.5));
}

#[test]
fn negative_net_is_floored_at_zero() {
    let mut p = paper(4, 2);
    p.penalty = 2.0;
    let answers: Vec<_> = (0..4).map(|i| tick(i, 1)).collect();
    let r = grade(&p, &answers, 0, TimeExtension::None);
    assert!(close(r.net_score, -8.0));
    assert!(close(r.mark(20.0), 0.0));
}

#[test]
fn blank_penalty_bites_only_blanks() {
    let mut p = paper(4, 2);
    p.unanswered_penalty = 0.25;
    let r = grade(
        &p,
        &[tick(0, 0), tick(1, 1), blank(2), blank(3)],
        0,
        TimeExtension::None,
    );
    assert!(close(r.net_score, 0.5));
    assert!(close(r.mark(20.0), 2.5));
}

#[test]
fn both_penalties_add_up() {
    let mut p = paper(4, 2);
    p.penalty = 0.5;
    p.unanswered_penalty = 0.25;
    let r = grade(
        &p,
        &[tick(0, 0), tick(1, 0), tick(2, 1), blank(3)],
        0,
        TimeExtension::None,
    );
    assert!(close(r.net_score, 1.25));
}

#[test]
fn removed_question_counts_as_unpenalised_blank() {
    let mut p = paper(4, 2);
    p.unanswered_penalty = 1.0;
    let answers: Vec<_> = (0..4).map(|i| tick(i, 0)).collect();
    p.questions.remove(3);
    let r = grade(&p, &answers, 0, TimeExtension::None);
    assert_eq!((r.correct, r.unanswered, r.total), (3, 1, 4));
    assert!(close(r.net_score, 3.0));
    assert_eq!(r.outcomes.len(), 3);
}

#[test]
fn overtime_is_recorded_but_never_changes_the_mark() {
    let mut p = paper(4, 2);
    p.duration_minutes = 60;
    p.penalty = 0.5;
    p.unanswered_penalty = 0.25;
    let answers = [tick(0, 0), tick(1, 0), tick(2, 1), blank(3)];
    let on_time = grade(&p, &answers, 3000, TimeExtension::None);
    let late = grade(&p, &answers, 6000, TimeExtension::None);
    assert_eq!(on_time.overtime_seconds, 0);
    assert_eq!(late.overtime_seconds, 2400);
    assert_eq!(late.time_limit_seconds, 3600);
    assert!(close(on_time.mark(20.0), late.mark(20.0)));
}

#[test]
fn extra_time_moves_the_overtime_not_the_mark() {
    let mut p = paper(4, 2);
    p.duration_minutes = 60;
    let answers = [tick(0, 0), tick(1, 0), tick(2, 1), blank(3)];
    let plain = grade(&p, &answers, 4500, TimeExtension::None);
    let third = grade(&p, &answers, 4500, TimeExtension::Third);
    let untimed = grade(&p, &answers, 99_000, TimeExtension::Untimed);
    assert_eq!(
        (plain.time_limit_seconds, plain.overtime_seconds),
        (3600, 900)
    );
    assert_eq!(
        (third.time_limit_seconds, third.overtime_seconds),
        (4800, 0)
    );
    assert_eq!(
        (untimed.time_limit_seconds, untimed.overtime_seconds),
        (0, 0)
    );
    assert!(close(plain.mark(20.0), third.mark(20.0)));
}

#[test]
fn no_limit_means_no_overtime() {
    let p = paper(1, 1);
    let r = grade(&p, &[tick(0, 0)], 99_999, TimeExtension::None);
    assert_eq!((r.time_limit_seconds, r.overtime_seconds), (0, 0));
}

#[test]
fn themes_follow_declaration_order_with_mastery() {
    let p = paper(6, 2);
    let answers: Vec<_> = (0..6)
        .map(|i| if i % 2 == 0 { tick(i, 0) } else { tick(i, 1) })
        .collect();
    let r = grade(&p, &answers, 0, TimeExtension::None);
    assert_eq!(r.themes[0].theme_id, "t0");
    assert_eq!(r.themes[0].correct, 3);
    assert!(close(r.themes[0].rate(), 1.0));
    assert_eq!(r.themes[0].mastery(), Mastery::Acquired);
    assert!(close(r.themes[1].rate(), 0.0));
    assert_eq!(r.themes[1].mastery(), Mastery::ToRelearn);
    assert!(r.themes[0].is_diagnostic());
}

#[test]
fn undeclared_theme_is_appended() {
    let mut p = paper(2, 1);
    p.questions[1].theme_id = "ghost".into();
    let r = grade(&p, &[tick(0, 0), tick(1, 0)], 0, TimeExtension::None);
    let ids: Vec<_> = r.themes.iter().map(|t| t.theme_id.as_str()).collect();
    assert_eq!(ids, ["t0", "ghost"]);
}

#[test]
fn mastery_thresholds() {
    let p = paper(4, 1);
    let expect = [
        (0, Mastery::ToRelearn),
        (1, Mastery::ToRelearn),
        (2, Mastery::Fragile),
        (3, Mastery::Acquired),
        (4, Mastery::Acquired),
    ];
    for (right, mastery) in expect {
        let answers: Vec<_> = (0..4).map(|i| tick(i, usize::from(i >= right))).collect();
        let r = grade(&p, &answers, 0, TimeExtension::None);
        assert_eq!(r.themes[0].mastery(), mastery, "{right}/4");
    }
    let r = grade(&p, &[], 0, TimeExtension::None);
    assert_eq!(r.themes[0].mastery(), Mastery::NotAssessed);
}

#[test]
fn review_list_holds_everything_not_fully_correct() {
    let p = paper(4, 2);
    let r = grade(
        &p,
        &[tick(0, 0), tick(1, 1), blank(2), blank(3)],
        0,
        TimeExtension::None,
    );
    assert_eq!(r.to_review().count(), 3);
}

#[test]
fn key_without_correct_option_and_unknown_id_are_wrong() {
    let mut p = paper(2, 1);
    p.questions[0].options[0].is_correct = false;
    let bogus = Answer {
        question_id: "q1".into(),
        selected: vec!["nope".into()],
        seconds_spent: 0,
    };
    let r = grade(&p, &[tick(0, 0), bogus], 0, TimeExtension::None);
    assert_eq!((r.correct, r.wrong), (0, 2));
}

#[test]
fn empty_inputs_do_not_panic() {
    let r = grade(&Paper::new(), &[], 0, TimeExtension::None);
    assert_eq!(r.total, 0);
    assert!(close(r.success_rate(), 0.0));
}

fn two_of_four() -> Question {
    let mut p = paper(1, 1);
    let mut q = p.questions.remove(0);
    q.options[1].is_correct = true;
    q
}

fn ids(options: &[usize]) -> Vec<String> {
    options.iter().map(|k| format!("q0-o{}", k + 1)).collect()
}

#[test]
fn multiple_answers_all_or_nothing() {
    let q = two_of_four();
    let s = Scoring::AllOrNothing;
    assert_eq!(score(&q, &ids(&[0, 1]), s), (Status::Correct, 1.0));
    assert_eq!(score(&q, &ids(&[0]), s), (Status::Wrong, 0.0));
    assert_eq!(score(&q, &ids(&[0, 1, 2, 3]), s), (Status::Wrong, 0.0));
    assert_eq!(score(&q, &ids(&[0, 0, 1]), s), (Status::Correct, 1.0));
}

#[test]
fn multiple_answers_partial_credit() {
    let q = two_of_four();
    let s = Scoring::Partial;
    assert_eq!(score(&q, &ids(&[0, 1]), s), (Status::Correct, 1.0));
    assert_eq!(score(&q, &ids(&[0]), s), (Status::Partial, 0.5));
    assert_eq!(score(&q, &ids(&[0, 2]), s), (Status::Wrong, 0.0));
    // Ticking everything must earn nothing.
    assert_eq!(score(&q, &ids(&[0, 1, 2, 3]), s), (Status::Wrong, 0.0));
    assert_eq!(score(&q, &[], s), (Status::Unanswered, 0.0));
}

#[test]
fn partial_credit_counts_in_mark_and_theme_without_penalty() {
    let mut p = paper(2, 1);
    p.scoring = Scoring::Partial;
    p.penalty = 1.0;
    p.questions[0].options[1].is_correct = true;
    let half = Answer {
        question_id: "q0".into(),
        selected: vec!["q0-o1".into()],
        seconds_spent: 0,
    };
    let r = grade(&p, &[half, tick(1, 0)], 0, TimeExtension::None);
    assert_eq!((r.correct, r.partial, r.wrong), (1, 1, 0));
    assert!(close(r.net_score, 1.5));
    assert!(close(r.themes[0].rate(), 0.75));
}

#[test]
fn true_false_question_grades_like_any_other() {
    let mut p = paper(1, 1);
    p.questions[0].options.truncate(2);
    let r = grade(&p, &[tick(0, 0)], 0, TimeExtension::None);
    assert!(close(r.mark(20.0), 20.0));
}

#[test]
fn one_right_out_of_forty_is_half_a_point() {
    let p = paper(40, 4);
    let mut answers = vec![tick(0, 0)];
    answers.extend((1..40).map(|i| tick(i, 1)));
    let r = grade(&p, &answers, 0, TimeExtension::None);
    assert!(close(r.mark(20.0), 0.5));
}

#[test]
fn blank_penalty_over_forty_questions() {
    let mut p = paper(40, 4);
    p.unanswered_penalty = 0.25;
    let mut answers: Vec<_> = (0..10).map(|i| tick(i, 0)).collect();
    answers.extend((10..40).map(blank));
    let r = grade(&p, &answers, 0, TimeExtension::None);
    assert!(close(r.mark(20.0), 1.25));
}

#[test]
fn guess_advice_follows_expected_value() {
    let mut p = paper(40, 4);
    assert_eq!(guess_advice(&p), GuessAdvice::GuessingIsFree);
    p.unanswered_penalty = 0.25;
    assert_eq!(guess_advice(&p), GuessAdvice::AnswerEverything);
    p.unanswered_penalty = 0.0;
    p.penalty = 1.0 / 3.0;
    assert_eq!(guess_advice(&p), GuessAdvice::AnswerEverything);
    p.penalty = 0.5;
    assert_eq!(guess_advice(&p), GuessAdvice::TickOnlyWhenYouHaveAnIdea);
    p.unanswered_penalty = 0.05;
    assert_eq!(guess_advice(&p), GuessAdvice::AnswerWhenYouHaveAnIdea);
    p.unanswered_penalty = 0.25;
    assert_eq!(guess_advice(&p), GuessAdvice::AnswerEverything);
}
