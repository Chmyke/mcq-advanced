//! The teacher demo: the paper and every copy open with the demo identity, the analysis has
//! something to show, and the tour leads to each space in both languages.

use dioxus::prelude::*;
use mcq_analysis::Finding;
use mcq_edit::Project;
use mcq_studio::correction::Correction;
use mcq_studio::demo::{
    Entry, STEP_SPACES, Tour, entry, is_untouched_demo, leaving_drops_changes, teacher,
};
use mcq_studio::state::Studio;
use mcq_studio::texts::{Lang, texts};

const SAMPLE: &str = include_str!("../../../crates/mcq-format/tests/fixtures/sample.qcm.md");

#[test]
fn every_copy_opens_and_the_doubtful_key_shows() {
    for lang in [Lang::En, Lang::Fr] {
        let (project, correction) = teacher(lang).expect("demo");
        assert!(!project.paper.questions.is_empty());
        assert!(correction.package.is_some(), "{lang:?}");
        assert_eq!(correction.copies.len(), mcq_demo::STUDENTS.len());
        assert!(correction.refused.is_empty(), "{:?}", correction.refused);
        assert_eq!(correction.pending(), 0);
        let report = correction.report().expect("report");
        assert!(
            report
                .findings
                .iter()
                .any(|f| matches!(f, Finding::KeyDoubt { .. }))
        );
    }
    let code = |lang| teacher(lang).expect("demo").0.paper.lang;
    assert_eq!(code(Lang::Fr), "fr");
    assert_eq!(code(Lang::En), "en");
}

#[test]
fn only_an_untouched_demo_is_replaced_without_asking() {
    let (french, _) = teacher(Lang::Fr).expect("demo");
    assert!(is_untouched_demo(&french));
    // The student demo, as the player's "open in the editor" link imports it.
    for source in &mcq_demo::SOURCES {
        let built = mcq_demo::build(source).expect("student demo");
        let (student, _) = mcq_edit::qcmz::import(&built.qcmz).expect("import");
        assert!(is_untouched_demo(&student), "{}", source.lang);
    }
    // A gallery model, as its "open in the editor" link imports it.
    for model in &mcq_demo::GALLERY {
        let built = mcq_demo::build_model(model).expect("gallery model");
        let (opened, _) = mcq_edit::qcmz::import(&built.qcmz).expect("import");
        assert!(is_untouched_demo(&opened), "{}", model.stem);
    }
    let mut edited = french.clone();
    edited.paper.title.push('!');
    assert!(!is_untouched_demo(&edited));
    assert!(!is_untouched_demo(&Project::default()));
}

#[test]
fn the_tour_leads_to_each_space_and_out() {
    for lang in [Lang::En, Lang::Fr] {
        #[component]
        fn Harness(lang: Lang) -> Element {
            let studio = use_signal(|| Studio::new(Project::default()));
            let correction = use_signal(Correction::default);
            let notice = use_signal(|| None::<String>);
            let tour = use_signal(|| true);
            rsx! { Tour { studio, correction, notice, tour, lang, t: texts(lang) } }
        }
        let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { lang });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        let t = texts(lang).demo;
        for (title, _) in t.steps {
            assert!(html.contains(title), "{lang:?}: {title}");
        }
        // One button per step with a space, and the way out of the visit.
        assert!(html.contains(&t.leave.replace('\'', "&#39;")), "{html}");
        let buttons = html.matches("<button").count();
        assert_eq!(buttons, STEP_SPACES.iter().flatten().count() + 1, "{html}");
    }
}

#[test]
fn a_draft_with_work_is_asked_for_then_set_aside() {
    let (demo, _) = teacher(Lang::Fr).expect("demo");
    let blank = Studio::new(Project::starter("Nouveau sujet", "fr"));
    assert!(blank.is_blank());
    assert_eq!(entry(&blank, false), Entry::Replace);
    assert_eq!(entry(&Studio::new(demo.clone()), false), Entry::Replace);

    let mut mine = Project::default();
    mine.set_source(SAMPLE);
    let mine = Studio::new(mine);
    assert_eq!(
        entry(&mine, false),
        Entry::Ask,
        "a draft with work is never replaced silently"
    );
    // Reloaded during the visit: the paper in the editor is the demo's, changed or not, and the
    // draft is already aside.
    let mut changed = demo.clone();
    changed.paper.title.push('!');
    assert_eq!(entry(&Studio::new(changed.clone()), true), Entry::Resume);

    // Leaving asks only when the demo paper was changed.
    assert!(!leaving_drops_changes(&Studio::new(demo)));
    assert!(leaving_drops_changes(&Studio::new(changed)));
    assert!(!leaving_drops_changes(&blank));
}
