//! What a student sees, rendered to HTML from a real package of the fictional sample paper.

use std::cell::RefCell;
use std::convert::Infallible;

use dioxus::prelude::*;
use mcq_crypto::rand_core::{TryCryptoRng, TryRng};
use mcq_crypto::{Identity, KdfParams, Opening, PublishOptions};
use mcq_engine::Session;
use mcq_format::binary::{CODE_TOKENS_FILE, Entry, encode_code_tokens};
use mcq_model::TimeExtension;
use mcq_player::render::{self, Opened};
use mcq_player::texts::Lang;

const SAMPLE: &str = include_str!("../../../crates/mcq-format/tests/fixtures/sample.qcm.md");
const CODE: &str = "0000-0000-0000";

/// Counter-mode bytes: reproducible, test use only.
struct CountingRng(u8);

impl TryRng for CountingRng {
    type Error = Infallible;
    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        let mut b = [0; 4];
        self.try_fill_bytes(&mut b)?;
        Ok(u32::from_le_bytes(b))
    }
    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        let mut b = [0; 8];
        self.try_fill_bytes(&mut b)?;
        Ok(u64::from_le_bytes(b))
    }
    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
        for byte in dst {
            self.0 = self.0.wrapping_mul(31).wrapping_add(7);
            *byte = self.0;
        }
        Ok(())
    }
}
impl TryCryptoRng for CountingRng {}

/// The sample paper, published with its media and coloured code, as a player receives it.
fn package(practice: bool) -> Vec<u8> {
    let (mut paper, _) = mcq_format::read(SAMPLE);
    paper.shuffle_options = false;
    let mut rng = CountingRng(1);
    let teacher = Identity::generate(&mut rng);
    let tokens = mcq_highlight::Highlighter::new().paper(&paper);
    let files = vec![
        Entry {
            path: "media/apple.png".into(),
            data: vec![0x89, b'P', b'N', b'G'],
        },
        Entry {
            path: "media/crunch.ogg".into(),
            data: b"OggS".to_vec(),
        },
        Entry {
            path: CODE_TOKENS_FILE.into(),
            data: encode_code_tokens(&tokens),
        },
    ];
    let options = PublishOptions {
        opening: if practice {
            Opening::Open
        } else {
            Opening::Code(CODE.into())
        },
        end_code: None,
        kdf: KdfParams::TESTING,
    };
    mcq_crypto::publish(&teacher, &paper, &files, &options, &mut rng).expect("publish")
}

fn opened() -> Opened {
    Opened::from_package(&package(false), CODE).expect("open")
}

thread_local! {
    static VIEW: RefCell<Option<Box<dyn Fn() -> Element>>> = const { RefCell::new(None) };
}

#[allow(non_snake_case)]
fn Harness() -> Element {
    VIEW.with(|v| v.borrow().as_ref().map_or_else(|| rsx! {}, |f| f()))
}

/// Renders `view` inside a component, where event handlers can exist.
fn render(view: impl Fn() -> Element + 'static) -> String {
    VIEW.with(|v| *v.borrow_mut() = Some(Box::new(view)));
    let mut dom = VirtualDom::new(Harness);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn question_html(question_id: &'static str) -> String {
    render(move || {
        let opened = opened();
        let mut session = Session::new(&opened.paper, opened.header.paper_id, "Alex Example");
        let index = session
            .items
            .iter()
            .position(|i| i.question_id == question_id)
            .expect("question in session");
        session.go_to(index);
        let item = session.current_item().expect("item").clone();
        let question = opened
            .paper
            .question(question_id)
            .expect("question")
            .clone();
        render::question_view(
            &question,
            &item,
            index,
            session.items.len(),
            opened.content(),
            Lang::En.texts(),
            EventHandler::new(|_: String| {}),
            EventHandler::new(|()| {}),
            EventHandler::new(|()| {}),
        )
    })
}

#[test]
fn multiple_answer_question_is_announced_with_checkboxes() {
    let html = question_html("code-sum-of-squares");
    assert!(html.contains("Several answers · 2 to tick"), "{html}");
    let (options, _) = html.split_once("</fieldset>").expect("options");
    assert_eq!(options.matches("type=\"checkbox\"").count(), 4, "{html}");
    assert!(!html.contains("type=\"radio\""), "{html}");
    assert!(html.contains("It prints <code>14</code>"), "{html}");
}

#[test]
fn code_is_coloured_with_its_highlighted_line() {
    let html = question_html("code-sum-of-squares");
    assert!(html.contains("class=\"tok-keyword\""), "{html}");
    assert!(html.contains("class=\"tok-number\""), "{html}");
    assert_eq!(html.matches("class=\"line hl\"").count(), 1, "{html}");
    assert!(html.contains(">cs<"), "language caption: {html}");
}

#[test]
fn single_answer_question_uses_radios_and_no_answer_leaks() {
    let html = question_html("fruit-banana-colour");
    assert!(html.contains("One answer"), "{html}");
    assert_eq!(html.matches("type=\"radio\"").count(), 3, "{html}");
    assert!(
        !html.contains("Ripening reveals"),
        "no explanation before hand-in: {html}"
    );
    assert!(
        !html.contains("Colour follows ripeness"),
        "no rule before hand-in: {html}"
    );
}

#[test]
fn media_are_embedded_with_their_alternative() {
    let html = question_html("fruit-apple-picture");
    assert!(
        html.contains("src=\"data:image/png;base64,iVBORw==\""),
        "{html}"
    );
    assert!(html.contains("alt=\"A red apple on a table\""), "{html}");
    assert!(html.contains("<audio"), "{html}");
}

#[test]
fn intro_explains_time_scoring_and_guessing() {
    let html = render(|| {
        let opened = opened();
        render::intro(
            &opened.paper,
            &opened.formulas,
            &opened.header,
            Lang::Fr.texts(),
            EventHandler::new(|()| {}),
            false,
            no_extra_time(),
        )
    });
    assert!(html.contains("10 min"), "{html}");
    assert!(
        html.contains("Une réponse fausse coûte 1,667 / 20"),
        "{html}"
    );
    assert!(html.contains("Commencer"), "{html}");
}

#[test]
fn practice_and_correction_show_the_answers_after_hand_in() {
    let html = render(|| {
        let bytes = package(true);
        let opened = Opened::from_package(&bytes, "").expect("practice opens without code");
        let full =
            mcq_crypto::open_answer_key(&bytes, mcq_crypto::KeyUnlock::Practice).expect("key");
        let mut session = Session::new(&opened.paper, opened.header.paper_id, "Alex Example");
        let banana = session
            .items
            .iter()
            .position(|i| i.question_id == "fruit-banana-colour")
            .expect("q");
        session.go_to(banana);
        session.toggle(&opened.paper, "fruit-banana-colour-o1");
        let copy = session.submit();
        let result = mcq_grade::grade(&full, &copy.answers, copy.elapsed_seconds, copy.extension);
        // The right answer is not among the mistakes shown first: the student opens it.
        let mut state = mcq_player::review::ReviewState::default();
        state.apply(
            mcq_player::review::ReviewAction::Go(banana),
            &session,
            &result,
        );
        mcq_player::review::review_screen(
            &full,
            &session,
            &result,
            &state,
            Lang::En.texts(),
            opened.content(),
            EventHandler::new(|_| {}),
        )
    });
    assert!(html.contains("6.67 / 20"), "one of three right: {html}");
    assert!(
        html.contains("Ripening reveals the yellow pigments."),
        "{html}"
    );
    assert!(html.contains("(your answer)"), "{html}");
    assert!(html.contains("Colour follows ripeness."), "{html}");
}

#[test]
fn overview_marks_current_answered_and_flagged() {
    let html = render(|| {
        let opened = opened();
        let mut session = Session::new(&opened.paper, opened.header.paper_id, "Alex Example");
        let first = session.items[0].option_order[0].clone();
        session.toggle(&opened.paper, &first);
        session.next();
        session.toggle_flag();
        session.next();
        render::segments(
            &session,
            None,
            Lang::En.texts(),
            EventHandler::new(|_: usize| {}),
        )
    });
    assert!(html.contains("segment answered"), "{html}");
    assert!(html.contains("segment flagged"), "{html}");
    assert!(html.contains("segment current"), "{html}");
    // The state is spoken, not only coloured (WCAG 1.4.1).
    assert!(
        html.contains("aria-label=\"Question 1, answered\""),
        "{html}"
    );
    assert!(
        html.contains("aria-label=\"Question 2, not answered, flagged for review\""),
        "{html}"
    );
    assert!(
        html.contains("aria-label=\"Question 3, not answered\""),
        "{html}"
    );
}

#[test]
fn wrong_code_opens_nothing() {
    assert!(Opened::from_package(&package(false), "1111-1111-1111").is_err());
}

#[test]
fn welcome_screen_renders() {
    let mut dom = VirtualDom::new(mcq_player::App);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("Open a paper"), "{html}");
    assert!(html.contains("accept=\".mcqpack\""), "{html}");
}

#[test]
fn the_player_starts_dark_with_dark_left_of_light() {
    let mut dom = VirtualDom::new(mcq_player::App);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("data-theme=\"dark\""), "{html}");
    let dark = html
        .find("aria-label=\"Switch to the dark theme\" aria-pressed=\"true\"")
        .expect("dark, chosen");
    let light = html
        .find("aria-label=\"Switch to the light theme\" aria-pressed=\"false\"")
        .expect("light, not chosen");
    assert!(dark < light, "{html}");
}

#[test]
fn formulas_show_as_native_mathml_and_explanations_after_hand_in() {
    let (mut paper, _) = mcq_format::read(SAMPLE);
    paper.shuffle_options = false;
    let banana = paper
        .questions
        .iter_mut()
        .find(|q| q.id == "fruit-banana-colour")
        .expect("q");
    banana.body.push(mcq_model::Block::Text(
        r"A ripe banana is $\frac{3}{4}$ yellow, for \$2.".into(),
    ));
    banana.options[0].explanation = r"Pigments: $$\sum_i p_i$$".into();
    let mut rng = CountingRng(9);
    let teacher = Identity::generate(&mut rng);
    let bytes = mcq_publish::publish(
        &teacher,
        &paper,
        |path: &str| Ok(path.as_bytes().to_vec()),
        mcq_publish::Mode::Practice,
        KdfParams::TESTING,
        &mut rng,
    )
    .expect("publish")
    .package;

    let html = render(move || {
        let opened = Opened::from_package(&bytes, "").expect("open");
        let (full, key_files) =
            mcq_crypto::open_answer_key_files(&bytes, mcq_crypto::KeyUnlock::Practice)
                .expect("key");
        let mut session = Session::new(&opened.paper, opened.header.paper_id, "Alex Example");
        let index = session
            .items
            .iter()
            .position(|i| i.question_id == "fruit-banana-colour")
            .expect("q");
        session.go_to(index);
        let item = session.current_item().expect("item").clone();
        let question = opened.paper.question("fruit-banana-colour").expect("q");
        let exam = render::question_view(
            question,
            &item,
            index,
            session.items.len(),
            opened.content(),
            Lang::En.texts(),
            EventHandler::new(|_: String| {}),
            EventHandler::new(|()| {}),
            EventHandler::new(|()| {}),
        );
        let copy = session.submit();
        let result = mcq_grade::grade(&full, &copy.answers, copy.elapsed_seconds, copy.extension);
        let formulas = render::correction_formulas(&opened.formulas, &key_files);
        let content = render::Content {
            formulas: &formulas,
            ..opened.content()
        };
        let review = mcq_player::review::review_screen(
            &full,
            &session,
            &result,
            &Default::default(),
            Lang::En.texts(),
            content,
            EventHandler::new(|_| {}),
        );
        // Before the correction, the explanation's formula is not in the student's table.
        let early = mcq_player::review::review_screen(
            &full,
            &session,
            &result,
            &Default::default(),
            Lang::En.texts(),
            opened.content(),
            EventHandler::new(|_| {}),
        );
        rsx! { div { id: "exam", {exam} } div { id: "review", {review} } div { id: "early", {early} } }
    });
    let part = |id: &str| {
        let start = html.find(&format!("id=\"{id}\"")).expect(id);
        html[start..].to_string()
    };
    let exam = part("exam");
    assert!(
        exam.contains("<math><mfrac><mn>3</mn><mn>4</mn></mfrac></math> yellow, for $2."),
        "{exam}"
    );
    let review = part("review");
    assert!(
        review.contains("<math display=\"block\"><munder>"),
        "{review}"
    );
    let early = part("early");
    assert!(
        early.contains(r#"<code class="formula-source">$$\sum_i p_i$$</code>"#),
        "{early}"
    );
}

#[test]
fn the_correction_opens_on_the_mistakes_one_at_a_time() {
    let html = render(|| {
        let bytes = package(true);
        let opened = Opened::from_package(&bytes, "").expect("open");
        let full =
            mcq_crypto::open_answer_key(&bytes, mcq_crypto::KeyUnlock::Practice).expect("key");
        let mut session = Session::new(&opened.paper, opened.header.paper_id, "Alex Example");
        let banana = session
            .items
            .iter()
            .position(|i| i.question_id == "fruit-banana-colour")
            .expect("q");
        session.go_to(banana);
        session.toggle(&opened.paper, "fruit-banana-colour-o1");
        session.tick(3);
        let copy = session.submit();
        let result = mcq_grade::grade(&full, &copy.answers, copy.elapsed_seconds, copy.extension);
        let state = mcq_player::review::ReviewState::default();
        let first = state.current(&session, &result).expect("a mistake");
        assert_ne!(first, banana, "the right answer is not shown first");
        mcq_player::review::review_screen(
            &full,
            &session,
            &result,
            &state,
            Lang::Fr.texts(),
            opened.content(),
            EventHandler::new(|_| {}),
        )
    });
    let guide = &html[html.find("review-guide").expect("guide")..];
    assert!(html.contains("6,67 / 20"), "{html}");
    assert!(!html.contains("Imprimer"), "nothing to print: {html}");
    assert!(html.contains("À reprendre"), "the word on the mark: {html}");
    assert!(html.contains("1 Juste"), "{html}");
    assert!(
        guide.contains("aria-pressed=\"true\">Mes erreurs"),
        "{guide}"
    );
    assert!(
        guide.contains("1 / 2"),
        "two mistakes, the first shown: {guide}"
    );
    assert_eq!(
        guide.matches("class=\"review-item").count(),
        1,
        "one at a time"
    );
    assert!(guide.contains("Votre réponse"), "{guide}");
    assert!(guide.contains("La bonne réponse"), "{guide}");
    assert!(
        guide.contains("prévues"),
        "time spent against expected: {guide}"
    );
    assert!(guide.contains("J&#39;ai compris"), "{guide}");
    assert_eq!(guide.matches("class=\"segment ").count(), 3, "{guide}");
    let faults = mcq_player::icon::contract_faults(&html);
    assert!(faults.is_empty(), "{faults:#?}");
}

#[test]
fn the_flag_is_an_icon_that_keeps_its_name_when_pressed() {
    let html = question_html("fruit-banana-colour");
    assert!(html.contains("aria-label=\"Flag for review\""), "{html}");
    assert!(html.contains("aria-pressed=\"false\""), "{html}");
    let faults = mcq_player::icon::contract_faults(&html);
    assert!(faults.is_empty(), "{faults:#?}");
}

/// The rendered correction, written for `tools/layout-check.mjs`: the browser lays out the real
/// markup with the real stylesheet, so the check never drifts from the screen.
#[test]
fn the_correction_markup_is_handed_to_the_layout_check() {
    let html = render(|| {
        let bytes = package(true);
        let opened = Opened::from_package(&bytes, "").expect("open");
        let full =
            mcq_crypto::open_answer_key(&bytes, mcq_crypto::KeyUnlock::Practice).expect("key");
        let mut session = Session::new(&opened.paper, opened.header.paper_id, "Alex Example");
        let copy = session.submit();
        let result = mcq_grade::grade(&full, &copy.answers, copy.elapsed_seconds, copy.extension);
        mcq_player::review::review_screen(
            &full,
            &session,
            &result,
            &mcq_player::review::ReviewState::default(),
            Lang::Fr.texts(),
            opened.content(),
            EventHandler::new(|_| {}),
        )
    });
    assert_eq!(html.matches("class=\"theme-name\"").count(), 2, "{html}");
    let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("player-review.html");
    std::fs::write(&file, html).expect("write the layout fixture");
}

/// The exam screen, written for `tools/motion-check.mjs`: every control of the real markup must
/// answer the pointer.
#[test]
fn the_exam_markup_is_handed_to_the_motion_check() {
    let html = render(|| {
        let opened = opened();
        let mut session = Session::new(&opened.paper, opened.header.paper_id, "Alex Example");
        session.tick(150);
        let gauge = session.pace(&opened.paper).expect("ten minutes");
        let item = session.current_item().expect("item").clone();
        let question = opened.paper.question(&item.question_id).expect("q");
        rsx! {
            {render::segments(&session, Some(gauge.time_fraction), Lang::Fr.texts(), EventHandler::new(|_: usize| {}))}
            {render::question_view(question, &item, 0, 3, opened.content(), Lang::Fr.texts(), EventHandler::new(|_: String| {}), EventHandler::new(|()| {}), EventHandler::new(|()| {}))}
        }
    });
    assert!(html.contains("class=\"option"), "{html}");
    let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("player-exam.html");
    std::fs::write(&file, html).expect("write the motion fixture");
}

#[test]
fn the_revision_sheet_downloads_as_markdown() {
    let bytes = package(true);
    let opened = Opened::from_package(&bytes, "").expect("open");
    let full = mcq_crypto::open_answer_key(&bytes, mcq_crypto::KeyUnlock::Practice).expect("key");
    let mut session = Session::new(&opened.paper, opened.header.paper_id, "Alex Example");
    let copy = session.submit();
    let result = mcq_grade::grade(&full, &copy.answers, copy.elapsed_seconds, copy.extension);
    let sheet = mcq_player::review::revision_markdown(&full, &session, &result, Lang::Fr.texts());
    assert!(sheet.starts_with("# Fiche de révision : "), "{sheet}");
    assert!(sheet.contains("| Thème | Réussite | État |"), "{sheet}");
    assert!(sheet.contains("## Mes erreurs (3)"), "{sheet}");
    assert!(
        sheet.contains("**Votre réponse** Pas de réponse."),
        "{sheet}"
    );
    assert!(sheet.contains("**La bonne réponse**"), "{sheet}");
    assert!(sheet.contains("```cs"), "code stays code: {sheet}");
}

#[test]
fn the_exam_shows_segments_and_a_calm_pace_gauge_with_letter_keys() {
    let html = render(|| {
        let opened = opened();
        let mut session = Session::new(&opened.paper, opened.header.paper_id, "Alex Example");
        session.tick(150);
        let gauge = session.pace(&opened.paper).expect("ten minutes");
        let item = session.current_item().expect("item").clone();
        let question = opened.paper.question(&item.question_id).expect("q");
        rsx! {
            {render::segments(&session, Some(gauge.time_fraction), Lang::Fr.texts(), EventHandler::new(|_: usize| {}))}
            {render::pace_view(&gauge, Some((0, -30)), Lang::Fr.texts())}
            {render::question_view(question, &item, 0, 3, opened.content(), Lang::Fr.texts(), EventHandler::new(|_: String| {}), EventHandler::new(|()| {}), EventHandler::new(|()| {}))}
        }
    });
    assert_eq!(html.matches("class=\"segment ").count(), 3, "{html}");
    assert!(
        html.contains("time-cursor\" aria-hidden=\"true\" style=\"left: 25.00%"),
        "{html}"
    );
    assert!(
        html.contains("Cette question : 2 min 30 s sur 3 min 20 s"),
        "{html}"
    );
    assert!(html.contains("de retard"), "{html}");
    assert!(
        html.contains("Question 1 : 30 s de plus que sa part"),
        "{html}"
    );
    assert!(
        html.contains(">A</span>"),
        "options keyed by letter: {html}"
    );
}

#[test]
fn a_demo_link_leads_to_the_editor_in_a_new_tab() {
    let html = render(|| render::editor_link("/mcq-advanced/demo/fr.qcmz", Lang::Fr.texts()));
    assert!(
        html.contains("href=\"../studio/?open=/mcq-advanced/demo/fr.qcmz\""),
        "{html}"
    );
    assert!(html.contains("target=\"_blank\""), "{html}");
    assert!(html.contains("rel=\"noopener\""), "{html}");
    assert!(html.contains("Ouvrir ce QCM dans l"), "{html}");
    // Without a source, or with one from elsewhere, nothing is shown.
    for source in ["", "https://evil.example/x.qcmz"] {
        let html = render(move || render::editor_link(source, Lang::En.texts()));
        assert!(!html.contains("href"), "{html}");
    }
}

fn no_extra_time() -> render::ExtraTime {
    render::ExtraTime {
        chosen: TimeExtension::None,
        locked: false,
        on_change: EventHandler::new(|_| {}),
    }
}

#[test]
fn intro_offers_extra_time_and_shows_the_stretched_duration() {
    let html = render(|| {
        let opened = opened();
        render::intro(
            &opened.paper,
            &opened.formulas,
            &opened.header,
            Lang::Fr.texts(),
            EventHandler::new(|()| {}),
            false,
            render::ExtraTime {
                chosen: TimeExtension::Third,
                ..no_extra_time()
            },
        )
    });
    assert!(html.contains("<legend>Temps majoré</legend>"), "{html}");
    assert!(html.contains("13 min 20 s · Tiers-temps (+1/3)"), "{html}");
    assert_eq!(html.matches("type=\"radio\"").count(), 5, "{html}");
    assert!(!html.contains("disabled"), "{html}");
}

#[test]
fn started_exam_shows_extra_time_locked() {
    let html = render(|| {
        let opened = opened();
        render::intro(
            &opened.paper,
            &opened.formulas,
            &opened.header,
            Lang::En.texts(),
            EventHandler::new(|()| {}),
            true,
            render::ExtraTime {
                chosen: TimeExtension::Untimed,
                locked: true,
                ..no_extra_time()
            },
        )
    });
    assert!(html.contains("no time limit"), "{html}");
    assert!(html.contains("it can no longer change"), "{html}");
    assert_eq!(html.matches("disabled").count(), 5, "{html}");
}

#[test]
fn a_question_can_be_reported_apart_from_its_answers_and_its_flag() {
    let html = question_html("fruit-banana-colour");
    let (options, after) = html.split_once("</fieldset>").expect("options");
    assert!(!options.contains("report"), "outside the answers: {html}");
    assert!(
        after.contains("This question seems unclear or wrong"),
        "{html}"
    );
    assert!(
        after.contains("Your teacher will see it. It does not affect your mark."),
        "{html}"
    );
    assert!(after.contains("class=\"report\""), "{html}");
    assert!(after.contains("type=\"checkbox\""), "{html}");
    assert!(!after.contains("checked"), "not ticked by default: {html}");
    assert!(after.contains("Flag for review"), "the flag stays: {html}");

    let reported = render(|| {
        let opened = opened();
        let mut session = Session::new(&opened.paper, opened.header.paper_id, "Alex Example");
        session.toggle_report();
        let item = session.current_item().expect("item").clone();
        let question = opened.paper.question(&item.question_id).expect("q").clone();
        render::question_view(
            &question,
            &item,
            0,
            session.items.len(),
            opened.content(),
            Lang::Fr.texts(),
            EventHandler::new(|_: String| {}),
            EventHandler::new(|()| {}),
            EventHandler::new(|()| {}),
        )
    });
    assert!(reported.contains("class=\"report on\""), "{reported}");
    assert!(reported.contains("checked"), "{reported}");
    assert!(
        reported.contains("Cette question me semble floue ou erronée"),
        "{reported}"
    );
    assert!(
        reported.contains("Marquer à revoir"),
        "still not flagged: {reported}"
    );
}

#[test]
fn overview_shows_a_reported_glyph_apart_from_the_flag_colour() {
    let html = render(|| {
        let opened = opened();
        let mut session = Session::new(&opened.paper, opened.header.paper_id, "Alex Example");
        session.toggle_report();
        session.next();
        session.toggle_flag();
        session.toggle_report();
        session.next();
        session.toggle_flag();
        render::segments(
            &session,
            None,
            Lang::En.texts(),
            EventHandler::new(|_: usize| {}),
        )
    });
    assert_eq!(
        html.matches("class=\"segment-report\"").count(),
        2,
        "{html}"
    );
    assert!(
        html.contains("aria-label=\"Question 1, not answered, reported as unclear\""),
        "{html}"
    );
    assert!(
        html.contains(
            "aria-label=\"Question 2, not answered, flagged for review, reported as unclear\""
        ),
        "{html}"
    );
    // Flagging the question on screen colours its segment at once.
    assert!(html.contains("segment current flagged"), "{html}");
}

#[test]
fn practice_hands_in_straight_to_the_correction_and_an_exam_waits_for_its_code() {
    use mcq_crypto::OpeningKind;
    assert!(mcq_player::review::correction_on_hand_in(OpeningKind::Open));
    let practice = mcq_crypto::inspect(&package(true)).expect("practice");
    let exam = mcq_crypto::inspect(&package(false)).expect("exam");
    assert!(mcq_player::review::correction_on_hand_in(practice.opening));
    assert!(!mcq_player::review::correction_on_hand_in(exam.opening));
}

#[test]
fn the_correction_opens_with_save_and_share_first() {
    use mcq_player::review::{ResultActions, result_actions};
    let both = render(|| {
        result_actions(
            &ResultActions {
                receipt: Some("AB12-CD34".into()),
                can_share: true,
                shared: Some(Lang::Fr.texts().share_copied),
            },
            Lang::Fr.texts(),
            EventHandler::new(|_| {}),
        )
    });
    assert!(both.contains("Enregistrer mon résultat"), "{both}");
    assert!(both.contains("Partager ce QCM"), "{both}");
    assert!(both.contains("AB12-CD34"), "{both}");
    assert!(both.contains("Lien copié"), "{both}");
    // A paper opened from a file has no address to pass on.
    let save_only = render(|| {
        result_actions(
            &ResultActions {
                receipt: Some("AB12-CD34".into()),
                ..ResultActions::default()
            },
            Lang::En.texts(),
            EventHandler::new(|_| {}),
        )
    });
    assert!(save_only.contains("Save my result"), "{save_only}");
    assert!(!save_only.contains("Share this paper"), "{save_only}");
    let nothing = render(|| {
        result_actions(
            &ResultActions::default(),
            Lang::En.texts(),
            EventHandler::new(|_| {}),
        )
    });
    assert!(!nothing.contains("result-actions"), "{nothing}");
}
