//! The teacher demo: the paper and every copy open with the demo identity, the analysis has
//! something to show, and the tour leads to each space in both languages.

use dioxus::prelude::*;
use mcq_analysis::Finding;
use mcq_edit::Project;
use mcq_studio::demo::{STEP_SPACES, Tour, is_untouched_demo, teacher};
use mcq_studio::state::Studio;
use mcq_studio::texts::{Lang, texts};

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
    let mut edited = french.clone();
    edited.paper.title.push('!');
    assert!(!is_untouched_demo(&edited));
    assert!(!is_untouched_demo(&Project::default()));
}

#[test]
fn the_tour_leads_to_each_space() {
    for lang in [Lang::En, Lang::Fr] {
        #[component]
        fn Harness(lang: Lang) -> Element {
            let studio = use_signal(|| Studio::new(Project::default()));
            rsx! { Tour { studio, t: texts(lang) } }
        }
        let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { lang });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        let t = texts(lang).demo;
        for (title, _) in t.steps {
            assert!(html.contains(title), "{lang:?}: {title}");
        }
        let buttons = html.matches("<button").count();
        assert_eq!(buttons, STEP_SPACES.iter().flatten().count(), "{html}");
    }
}
