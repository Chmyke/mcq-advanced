//! What a teacher sees in the Studio, rendered to HTML from the fictional sample paper.

use std::cell::RefCell;

use dioxus::prelude::*;
use mcq_edit::Project;
use mcq_publish::{Published, RoomSheet};
use mcq_studio::state::{Space, Studio};
use mcq_studio::texts::{EN, FR};
use mcq_studio::{editor, publish, settings};

const SAMPLE: &str = include_str!("../../../crates/mcq-format/tests/fixtures/sample.qcm.md");

/// What a test changes in the sample before it is rendered.
type Setup = Box<dyn Fn(&mut Studio)>;

thread_local! {
    static SETUP: RefCell<Option<Setup>> = const { RefCell::new(None) };
}

/// The sample paper with stand-ins for its media files.
fn sample() -> Studio {
    let mut project = Project::default();
    let issues = project.set_source(SAMPLE);
    for path in project.used_media() {
        project.media.insert(path, vec![0]);
    }
    Studio::with_issues(project, issues)
}

#[allow(non_snake_case)]
fn Harness() -> Element {
    let studio = use_signal(|| {
        let mut studio = sample();
        SETUP.with(|s| {
            if let Some(setup) = s.borrow().as_ref() {
                setup(&mut studio);
            }
        });
        studio
    });
    let recording = use_signal(|| false);
    let notice = use_signal(|| None::<String>);
    let phone = use_signal(|| false);
    let dark = use_signal(|| false);
    let space = studio.read().space;
    match space {
        Space::Write => editor::write_space(studio, &EN, recording, notice, phone, dark, None),
        Space::Settings => settings::settings_space(studio, &FR),
        Space::Publish | Space::Correct => rsx! {},
    }
}

/// Renders the Write or Settings space of the sample after `setup`.
fn render(setup: impl Fn(&mut Studio) + 'static) -> String {
    SETUP.with(|s| *s.borrow_mut() = Some(Box::new(setup)));
    let mut dom = VirtualDom::new(Harness);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn the_list_groups_questions_by_theme_with_their_state() {
    let html = render(|_| {});
    let colours = html.find("Fruit colours").expect("first theme");
    let loops = html.find("Counting loops").expect("second theme");
    assert!(colours < loops, "themes in paper order");
    assert!(html.contains("q-item current"), "{html}");
    // The sample has no error: every question shows a state, none in red.
    assert_eq!(html.matches("class=\"dot ").count(), 3, "{html}");
    assert!(!html.contains("dot error"), "{html}");
}

#[test]
fn a_question_dragged_onto_another_theme_changes_theme_and_stays_open() {
    let html = render(|s| {
        s.start_drag("fruit-banana-colour");
        s.drag_over("loops");
    });
    assert!(html.contains("draggable=\"true\""), "{html}");
    assert!(html.contains("theme-group drop-target"), "{html}");

    let html = render(|s| {
        s.start_drag("fruit-banana-colour");
        s.drop_question("loops", Some("code-sum-of-squares"));
        assert!(s.drag.is_none());
        let paper = &s.project.paper;
        let banana = paper.question("fruit-banana-colour").expect("kept");
        assert_eq!(banana.theme_id, "loops");
        assert_eq!(
            paper.questions[0].id, "fruit-banana-colour",
            "before the target"
        );
        assert_eq!(s.selected.as_deref(), Some("fruit-banana-colour"));
    });
    assert!(!html.contains("drop-target"), "{html}");
    // The editor's theme menu follows.
    assert!(html.contains("value=\"loops\" selected"), "{html}");
}

#[test]
fn a_multiple_answer_question_edits_with_checkboxes_and_previews_as_students_see_it() {
    let html = render(|s| s.select("code-sum-of-squares"));
    let editor = &html[..html.find("class=\"preview\"").expect("preview")];
    assert!(
        editor.contains("aria-pressed=\"true\">Multiple answers"),
        "{editor}"
    );
    assert_eq!(editor.matches("type=\"checkbox\"").count(), 4, "{editor}");
    assert!(editor.contains("Counting loops"));
    assert!(editor.contains("value=\"cs\""), "the code block's language");

    let preview = &html[html.find("class=\"preview\"").expect("preview")..];
    assert!(preview.contains("Several answers · 2 to tick"), "{preview}");
    assert!(
        preview.contains("tok-keyword"),
        "code coloured live: {preview}"
    );
    assert!(preview.contains("line hl"), "highlighted line: {preview}");
}

#[test]
fn the_validator_speaks_next_to_the_question() {
    let html = render(|s| {
        s.edit_question("fruit-banana-colour", |q| {
            for option in &mut q.options {
                option.is_correct = false;
            }
        });
    });
    assert!(html.contains("NO_CORRECT_OPTION"), "{html}");
    assert!(html.contains("dot error"), "{html}");
    assert!(html.contains("lint-rules.md"), "link to the guide");
}

#[test]
fn the_editor_has_no_source_view() {
    let html = render(|_| {});
    assert!(html.contains("class=\"editor card\""), "{html}");
    assert!(!html.contains("source-text"), "{html}");
    assert!(!html.contains("view-switch"), "{html}");
}

#[test]
fn settings_show_the_header_values() {
    let html = render(|s| {
        s.space = Space::Settings;
        s.edit_paper(|p| p.penalty = 0.25);
    });
    assert!(
        html.contains("value=\"Sample paper: fruit and code\""),
        "{html}"
    );
    assert!(html.contains("value=\"0.25\""), "{html}");
    assert!(html.contains(FR.shuffle_questions), "{html}");
    assert!(html.contains("Fruit colours"), "{html}");
}

#[test]
fn the_room_sheet_shows_the_opening_code_and_never_the_end_code() {
    let sheet = RoomSheet::new(
        &sample().project.paper,
        &Published {
            package: Vec::new(),
            paper_id: [0; 16],
            opening_code: Some("AAAA-BBBB-CCCC".into()),
            end_code: Some("ZZZZ-YYYY-XXXX".into()),
            fingerprint: "FING-ERPR-INT0-0000".into(),
        },
    );
    let mut dom =
        VirtualDom::new_with_props(|sheet: RoomSheet| publish::room_sheet(&sheet, &FR), sheet);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("AAAA-BBBB-CCCC"), "{html}");
    assert!(html.contains("FING-ERPR-INT0-0000"), "{html}");
    let later = FR.sheet_correction_later.replace('\'', "&#39;");
    assert!(html.contains(&later), "{html}");
    assert!(!html.contains("ZZZZ"), "{html}");
}

#[test]
fn the_formula_button_adds_a_formula_the_preview_shows_at_once() {
    let html = render(|s| {
        s.select("fruit-banana-colour");
        s.add_formula_block("fruit-banana-colour");
    });
    let editor = &html[..html.find("class=\"preview\"").expect("preview")];
    assert!(editor.contains("aria-label=\"Add a formula\""), "{editor}");
    assert!(editor.contains("$$x^2$$ on their own line"), "{editor}");
    assert!(
        editor.contains(r"$$\frac{a}{b}$$"),
        "the example to replace"
    );
    let preview = &html[html.find("class=\"preview\"").expect("preview")..];
    assert!(
        preview.contains("<math display=\"block\"><mfrac><mi>a</mi><mi>b</mi></mfrac></math>"),
        "{preview}"
    );
}

#[test]
fn a_formula_that_does_not_convert_is_explained_and_shown_as_source() {
    let html = render(|s| {
        s.select("fruit-banana-colour");
        s.edit_question("fruit-banana-colour", |q| {
            q.body
                .push(mcq_model::Block::Text(r"Is $\frac{1}{$ right?".into()));
        });
    });
    assert!(html.contains("BAD_FORMULA"), "{html}");
    assert!(html.contains("formula-source"), "{html}");
}

#[test]
fn every_icon_of_write_and_settings_is_named_after_its_tip() {
    // Write renders in English, Settings in French: both tables are checked.
    for html in [render(|_| {}), render(|s| s.space = Space::Settings)] {
        assert!(html.contains("data-icon"), "the screen shows icons: {html}");
        let faults = mcq_player::icon::contract_faults(&html);
        assert!(faults.is_empty(), "{faults:#?}");
    }
}

/// The icons outside Write and Settings (top bar, spaces, Publish, Correct): each short label
/// is part of its tip, in both languages, so the name read aloud matches the word shown.
#[test]
fn the_other_tips_contain_their_labels() {
    for t in [&EN, &FR] {
        let c = t.correction;
        let pairs = [
            (t.new_paper, t.tip_new),
            (t.import, t.tip_import),
            (t.export, t.tip_export),
            (t.theme_light, t.tip_theme_light),
            (t.theme_dark, t.tip_theme_dark),
            (t.space_write, t.tip_space_write),
            (t.space_settings, t.tip_space_settings),
            (t.space_publish, t.tip_space_publish),
            (t.space_correct, t.tip_space_correct),
            (t.print, t.print_sheet),
            (t.download, t.tip_download_again),
            (c.undo, c.tip_undo),
            (c.open_sheet, c.tip_open_sheet),
            (c.open_timeline, c.tip_open_timeline),
            (c.print, c.print_student_sheet),
            (c.export, c.tip_export),
        ];
        for (label, tip) in pairs {
            assert!(
                tip.to_lowercase().contains(&label.to_lowercase()),
                "\"{tip}\" lacks \"{label}\""
            );
        }
    }
}
