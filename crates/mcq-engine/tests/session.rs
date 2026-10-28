//! Exam scenarios on fictional papers.

use mcq_engine::{Session, decode_session, encode_session, format_duration, normalize_name};
use mcq_model::{AnswerOption, Block, LogKind, Paper, Question, Theme, TimeExtension};

const ID: [u8; 16] = [3; 16];

/// `n` questions over `t` themes, 4 options each; question 0 has two correct options.
fn paper(n: usize, t: usize) -> Paper {
    let mut paper = Paper::new();
    paper.title = "Engine paper".into();
    for i in 0..t {
        paper.themes.push(Theme {
            id: format!("t{i}"),
            title: format!("Theme {i}"),
            ..Theme::default()
        });
    }
    for i in 0..n {
        let id = format!("q{i}");
        paper.questions.push(Question {
            options: (0..4)
                .map(|k| AnswerOption {
                    id: format!("{id}-o{}", k + 1),
                    text: format!("Option {k}"),
                    is_correct: k == 0 || (i == 0 && k == 1),
                    ..AnswerOption::default()
                })
                .collect(),
            id,
            theme_id: format!("t{}", i % t),
            body: vec![Block::Text(format!("Question {i}?"))],
            ..Question::default()
        });
    }
    paper
}

fn order(session: &Session) -> Vec<&str> {
    session
        .items
        .iter()
        .map(|i| i.question_id.as_str())
        .collect()
}

#[test]
fn same_student_same_order_other_student_other_order() {
    let p = paper(12, 3);
    let a = Session::new(&p, ID, "Alex Example");
    let again = Session::new(&p, ID, "  alex   EXAMPLE ");
    let b = Session::new(&p, ID, "Sam Sample");
    assert_eq!(order(&a), order(&again));
    assert_eq!(a.items[0].option_order, again.items[0].option_order);
    assert_ne!(order(&a), order(&b));
    let mut sorted = order(&a);
    sorted.sort_unstable();
    let mut expected: Vec<_> = p.questions.iter().map(|q| q.id.as_str()).collect();
    expected.sort_unstable();
    assert_eq!(sorted, expected, "every question exactly once");
}

#[test]
fn shuffled_themes_are_interleaved() {
    let p = paper(9, 3);
    let s = Session::new(&p, ID, "Alex Example");
    let themes: Vec<_> = s
        .items
        .iter()
        .map(|i| p.question(&i.question_id).expect("exists").theme_id.clone())
        .collect();
    for window in themes.chunks(3) {
        let mut w = window.to_vec();
        w.sort();
        w.dedup();
        assert_eq!(w.len(), 3, "each round shows each theme once: {themes:?}");
    }
}

#[test]
fn no_shuffle_keeps_source_order() {
    let mut p = paper(5, 2);
    p.shuffle_questions = false;
    p.shuffle_options = false;
    let s = Session::new(&p, ID, "Alex Example");
    assert_eq!(order(&s), ["q0", "q1", "q2", "q3", "q4"]);
    assert_eq!(
        s.items[1].option_order,
        ["q1-o1", "q1-o2", "q1-o3", "q1-o4"]
    );
}

fn ordered(n: usize) -> (Paper, Session) {
    let mut p = paper(n, 1);
    p.shuffle_questions = false;
    p.shuffle_options = false;
    let s = Session::new(&p, ID, "Alex Example");
    (p, s)
}

#[test]
fn single_answer_replaces_multiple_answer_adds() {
    let (p, mut s) = ordered(2);
    assert!(s.toggle(&p, "q0-o1"));
    assert!(s.toggle(&p, "q0-o3"));
    assert_eq!(
        s.items[0].selected,
        ["q0-o1", "q0-o3"],
        "q0 has two correct options"
    );
    assert!(s.toggle(&p, "q0-o1"));
    assert_eq!(s.items[0].selected, ["q0-o3"]);

    s.next();
    assert!(s.toggle(&p, "q1-o1"));
    assert!(s.toggle(&p, "q1-o2"));
    assert_eq!(s.items[1].selected, ["q1-o2"]);
    assert!(
        !s.toggle(&p, "q0-o1"),
        "options of another question are ignored"
    );
    let kinds: Vec<_> = s.log.iter().map(|e| e.kind).collect();
    assert_eq!(
        kinds,
        [
            LogKind::Tick,
            LogKind::Tick,
            LogKind::Untick,
            LogKind::Tick,
            LogKind::Untick,
            LogKind::Tick
        ]
    );
}

#[test]
fn navigation_and_flags() {
    let (_, mut s) = ordered(4);
    s.previous();
    assert_eq!(s.current, 0);
    s.go_to(9);
    assert_eq!(s.current, 0);
    s.go_to(3);
    s.next();
    assert_eq!(s.current, 3);
    assert!(!s.next_flagged());
    s.toggle_flag();
    s.go_to(1);
    s.toggle_flag();
    s.go_to(2);
    assert!(s.next_flagged());
    assert_eq!(s.current, 3);
    assert!(s.next_flagged());
    assert_eq!(s.current, 1, "wraps around");
    assert_eq!(s.flagged_count(), 2);
}

#[test]
fn time_goes_to_the_question_on_screen() {
    let (_, mut s) = ordered(2);
    s.tick(5);
    s.next();
    s.tick(7);
    assert_eq!(s.elapsed_seconds, 12);
    assert_eq!((s.items[0].seconds_spent, s.items[1].seconds_spent), (5, 7));
}

#[test]
fn deadline_warning_and_overtime_never_block() {
    let (mut p, mut s) = ordered(2);
    p.duration_minutes = 10;
    s.tick(299);
    let t = s.time_status(&p);
    assert_eq!((t.limit_seconds, t.remaining_seconds), (600, 301));
    assert!(!t.final_warning);
    s.tick(1);
    assert!(s.time_status(&p).final_warning);
    s.tick(400);
    let t = s.time_status(&p);
    assert_eq!((t.remaining_seconds, t.overtime_seconds), (0, 100));
    assert!(!t.final_warning);
    assert!(s.toggle(&p, "q0-o1"), "still answerable after the time");

    p.duration_minutes = 0;
    let t = s.time_status(&p);
    assert_eq!(
        (t.limit_seconds, t.overtime_seconds, t.lead_seconds),
        (0, 0, 0)
    );
}

#[test]
fn pace_alert_fires_once_and_rearms() {
    let (mut p, mut s) = ordered(10);
    p.duration_minutes = 10; // 60 s per question
    s.tick(179);
    assert!(!s.time_status(&p).show_pace_alert);
    s.tick(1);
    let t = s.time_status(&p);
    assert!(t.show_pace_alert);
    assert_eq!(t.lead_seconds, -180);
    s.tick(1);
    assert!(!s.time_status(&p).show_pace_alert, "once per lag");
    for (i, q) in ["q0", "q1", "q2"].iter().enumerate() {
        s.go_to(i);
        s.toggle(&p, &format!("{q}-o2"));
    }
    s.time_status(&p);
    assert!(!s.pace_alerted, "re-armed within one budget");
}

#[test]
fn focus_events_are_counted_and_logged() {
    let (_, mut s) = ordered(1);
    s.focus_lost();
    s.focus_back(12);
    assert_eq!((s.focus_losses, s.seconds_away), (1, 12));
    assert_eq!(s.log.len(), 2);
}

#[test]
fn submission_freezes_the_session() {
    let (p, mut s) = ordered(2);
    s.toggle(&p, "q0-o1");
    s.tick(30);
    let copy = s.submit();
    assert_eq!(copy.answers.len(), 2);
    assert_eq!(copy.answers[0].selected, ["q0-o1"]);
    assert_eq!(copy.elapsed_seconds, 30);
    assert_eq!(copy.log.last().map(|e| e.kind), Some(LogKind::Submit));
    assert!(!s.toggle(&p, "q0-o2"));
    s.tick(10);
    s.toggle_flag();
    assert_eq!(s.submit(), copy, "a second submit gives the same copy");
}

#[test]
fn autosave_round_trips_and_refuses_damage() {
    let (p, mut s) = ordered(3);
    s.toggle(&p, "q0-o2");
    s.toggle_flag();
    s.next();
    s.tick(42);
    s.focus_lost();
    let bytes = encode_session(&s);
    assert_eq!(decode_session(&bytes), Ok(s.clone()));
    for len in 0..bytes.len() {
        assert!(decode_session(&bytes[..len]).is_err(), "length {len}");
    }
}

#[test]
fn helpers() {
    assert_eq!(format_duration(42), "42 s");
    assert_eq!(format_duration(185), "3 min 05 s");
    assert_eq!(normalize_name("  Alex \t Example "), "Alex Example");
}

#[test]
fn the_pace_gauge_shares_the_time_evenly_and_goes_away_past_the_limit() {
    let mut p = paper(10, 2);
    p.duration_minutes = 10;
    let mut s = Session::new(&p, [7; 16], "Alex Example");
    let gauge = s.pace(&p).expect("a limit");
    assert_eq!(gauge.share_seconds, 60);
    s.tick(20);
    let first = s.items[0].option_order[0].clone();
    s.toggle(&p, &first);
    s.next();
    s.tick(90);
    let gauge = s.pace(&p).expect("gauge");
    assert_eq!(gauge.current_seconds, 90);
    // One answer is worth 60 s of the pace; 110 s have gone.
    assert_eq!(gauge.lead_seconds, -50);
    assert!((gauge.time_fraction - 110.0 / 600.0).abs() < 1e-9);
    assert_eq!(
        s.share_delta(&p, 0),
        Some(40),
        "the first question saved 40 s"
    );
    assert_eq!(s.share_delta(&p, 1), Some(-30));
    assert_eq!(s.share_delta(&p, 5), None, "never visited");
    s.tick(600);
    assert_eq!(s.pace(&p), None, "past the limit");
    let mut free = paper(3, 1);
    free.duration_minutes = 0;
    assert_eq!(Session::new(&free, [7; 16], "A").pace(&free), None);
}

#[test]
fn extra_time_stretches_the_limit_the_pace_and_the_overtime() {
    let (mut p, mut s) = ordered(10);
    p.duration_minutes = 30;
    assert!(s.set_extension(TimeExtension::Third));
    let t = s.time_status(&p);
    assert_eq!((t.limit_seconds, t.remaining_seconds), (2400, 2400));
    assert_eq!(s.pace(&p).map(|g| g.share_seconds), Some(240));
    s.tick(1900);
    assert!(
        !s.time_status(&p).final_warning,
        "warned on the student's own limit"
    );
    s.tick(600);
    assert_eq!(s.time_status(&p).overtime_seconds, 100);
    assert_eq!(s.submit().extension, TimeExtension::Third);

    for (extension, limit) in [
        (TimeExtension::None, 1800),
        (TimeExtension::Half, 2700),
        (TimeExtension::Double, 3600),
        (TimeExtension::Untimed, 0),
    ] {
        let (_, mut s) = ordered(10);
        s.set_extension(extension);
        assert_eq!(s.limit_seconds(&p), limit, "{extension:?}");
    }
}

#[test]
fn untimed_shows_no_clock_pace_or_alert() {
    let (mut p, mut s) = ordered(4);
    p.duration_minutes = 10;
    s.set_extension(TimeExtension::Untimed);
    s.tick(5000);
    let t = s.time_status(&p);
    assert_eq!((t.limit_seconds, t.overtime_seconds), (0, 0));
    assert!(!t.show_pace_alert && !t.final_warning);
    assert!(s.pace(&p).is_none());
}

#[test]
fn extra_time_is_declared_before_the_start_only() {
    let (_, mut s) = ordered(2);
    s.tick(1);
    assert!(!s.set_extension(TimeExtension::Double));
    assert_eq!(s.extension, TimeExtension::None);
    let (_, mut s) = ordered(2);
    s.submit();
    assert!(!s.set_extension(TimeExtension::Double));
}

#[test]
fn autosave_keeps_the_extension_and_old_saves_resume_without_one() {
    let (_, mut s) = ordered(2);
    s.set_extension(TimeExtension::Half);
    let bytes = encode_session(&s);
    assert_eq!(decode_session(&bytes), Ok(s.clone()));

    // A version 1 save: no extension byte after the pace flag, no reported flags.
    let mut old = version_2_save(&s);
    let at = 4 + 1 + 16 + 4 + s.student_name.len() + 4 + 4 + 1 + 4 + 4 + 1;
    old.remove(at);
    old[4] = 1;
    let resumed = decode_session(&old).expect("version 1");
    assert_eq!(resumed.extension, TimeExtension::None);
    assert_eq!(resumed.items, s.items);
}

/// The session as a version 2 build saved it: no reported flag after each item's `flagged`.
fn version_2_save(s: &Session) -> Vec<u8> {
    let mut bytes = encode_session(s);
    let mut at = 4 + 1 + 16 + 4 + s.student_name.len() + 4 + 4 + 1 + 4 + 4 + 1 + 1 + 4;
    for item in &s.items {
        at += 4 + item.question_id.len() + 1;
        bytes.remove(at);
        at += 4 + 4;
        at += item.option_order.iter().map(|o| 4 + o.len()).sum::<usize>();
        at += 4 + item.selected.iter().map(|o| 4 + o.len()).sum::<usize>();
    }
    bytes[4] = 2;
    bytes
}

#[test]
fn a_report_is_independent_of_the_flag_and_never_moves() {
    let (p, mut s) = ordered(4);
    s.go_to(1);
    s.toggle(&p, "q1-o2");
    s.toggle_report();
    assert_eq!(s.current, 1, "reporting stays on the question");
    assert!(s.items[1].reported);
    assert!(!s.items[1].flagged, "a report is not a flag");
    assert_eq!(s.items[1].selected, ["q1-o2"], "nor an answer");
    assert!(
        !s.next_flagged(),
        "a reported question is not a flagged one"
    );
    assert_eq!(s.current, 1);

    s.toggle_flag();
    assert!(s.items[1].reported && s.items[1].flagged);
    s.toggle_report();
    assert!(!s.items[1].reported);
    assert!(s.items[1].flagged, "withdrawing the report keeps the flag");
    s.toggle_report();
    s.toggle_flag();
    assert!(
        s.items[1].reported && !s.items[1].flagged,
        "and unflagging keeps the report"
    );
    assert_eq!((s.reported_count(), s.flagged_count()), (1, 0));
    assert_eq!(s.current, 1);
}

#[test]
fn the_copy_carries_the_reports_and_keeps_the_flags_private() {
    let (_, mut s) = ordered(3);
    s.toggle_report();
    s.go_to(2);
    s.toggle_flag();
    let copy = s.submit();
    let reported: Vec<bool> = copy.answers.iter().map(|a| a.reported).collect();
    assert_eq!(reported, [true, false, false]);
    s.go_to(1);
    s.toggle_report();
    assert!(!s.items[1].reported, "a handed-in copy no longer changes");
    assert_eq!(s.submit(), copy);
}

#[test]
fn autosave_keeps_the_reports_and_old_saves_resume_without_them() {
    let (_, mut s) = ordered(3);
    s.go_to(1);
    s.toggle_report();
    s.toggle_flag();
    let bytes = encode_session(&s);
    assert_eq!(decode_session(&bytes), Ok(s.clone()));
    let resumed = decode_session(&version_2_save(&s)).expect("version 2");
    assert!(resumed.items.iter().all(|i| !i.reported));
    assert!(resumed.items[1].flagged);
    assert_eq!(resumed.current, 1);
}
