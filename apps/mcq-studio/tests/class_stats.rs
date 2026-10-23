//! The class's results in the Write space, from the teacher demo's class: each answer keeps its
//! figure when answers are reordered, a reworded answer shows none, a question no copy saw shows
//! nothing.

use mcq_studio::class_stats::{ClassResults, for_question};
use mcq_studio::demo::teacher;
use mcq_studio::texts::Lang;

fn demo() -> (mcq_edit::Project, ClassResults) {
    let (project, correction) = teacher(Lang::Fr).expect("demo");
    let class = ClassResults::from_correction(&correction).expect("copies");
    (project, class)
}

#[test]
fn the_planted_key_error_shows_beside_its_answers() {
    let (project, class) = demo();
    let question = project.paper.question("somme-fractions").expect("question");
    let results = for_question(&class, question).expect("results");
    assert_eq!(results.asked as usize, mcq_demo::STUDENTS.len());
    assert_eq!(results.answers.len(), question.options.len());
    let shares: Vec<_> = results
        .answers
        .iter()
        .map(|a| a.expect("matched"))
        .collect();
    // The key ticks 1/6 by mistake; the class mostly answers 5/6.
    let (sum, mistake) = (shares[0], shares[1]);
    assert!(mistake.correct && !sum.correct);
    assert!(sum.percent > mistake.percent, "{shares:?}");
    assert!(shares.iter().map(|s| s.percent).sum::<u32>() <= 101);
}

#[test]
fn figures_follow_their_answer_not_its_place() {
    let (project, class) = demo();
    let question = project.paper.question("somme-fractions").expect("question");
    let before = for_question(&class, question).expect("results");
    let mut moved = question.clone();
    moved.options.reverse();
    let after = for_question(&class, &moved).expect("results");
    let mut reversed = before.answers.clone();
    reversed.reverse();
    assert_eq!(after.answers, reversed);

    let mut reworded = question.clone();
    reworded.options[2].text = "un tout autre résultat".into();
    let changed = for_question(&class, &reworded).expect("results");
    assert_eq!(changed.answers[2], None);
    assert_eq!(changed.answers[0], before.answers[0]);
}

#[test]
fn a_question_no_copy_saw_shows_nothing() {
    let (project, class) = demo();
    let mut new = project.paper.questions[0].clone();
    new.id = "question-ecrite-apres".into();
    assert!(for_question(&class, &new).is_none());
}

#[test]
fn the_editor_shows_them_under_the_advice() {
    use dioxus::prelude::*;
    use mcq_studio::state::Studio;

    #[component]
    fn Harness() -> Element {
        let (project, class) = demo();
        let studio = use_signal(move || {
            let mut studio = Studio::new(project.clone());
            studio.selected = Some("somme-fractions".into());
            studio
        });
        let off = use_signal(|| false);
        let notice = use_signal(|| None::<String>);
        let t = mcq_studio::texts::texts(Lang::Fr);
        mcq_studio::editor::write_space(studio, t, off, notice, off, off, Some(&class))
    }
    let mut dom = VirtualDom::new(Harness);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    let section = html
        .find("Résultats de la classe")
        .expect("results section");
    assert!(html.find("Validateur").expect("advice") < section);
    assert!(
        html[section..].contains("Réussie par"),
        "{}",
        &html[section..]
    );
    assert!(html[section..].contains("juste dans le corrigé"));
    // The answers are fractions: drawn as formulas, not shown as LaTeX source.
    assert!(html[section..].contains("<math"));
    assert!(!html[section..].contains(r"\frac"));
}
