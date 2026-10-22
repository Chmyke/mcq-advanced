//! What a student sees, rendered to HTML from a real package of the fictional sample paper.

use std::cell::RefCell;
use std::convert::Infallible;

use dioxus::prelude::*;
use mcq_crypto::rand_core::{TryCryptoRng, TryRng};
use mcq_crypto::{Identity, KdfParams, Opening, PublishOptions};
use mcq_engine::Session;
use mcq_format::binary::{CODE_TOKENS_FILE, Entry, encode_code_tokens};
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
        )
    })
}

#[test]
fn multiple_answer_question_is_announced_with_checkboxes() {
    let html = question_html("code-sum-of-squares");
    assert!(html.contains("Several answers · 2 to tick"), "{html}");
    assert_eq!(html.matches("type=\"checkbox\"").count(), 4, "{html}");
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
        let result = mcq_grade::grade(&full, &copy.answers, copy.elapsed_seconds);
        render::review_view(&full, &session, &result, Lang::En.texts(), opened.content())
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
        render::grid(&session, Lang::En.texts(), EventHandler::new(|_: usize| {}))
    });
    assert!(html.contains("cell answered"), "{html}");
    assert!(html.contains("cell flagged"), "{html}");
    assert!(html.contains("cell current"), "{html}");
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
        );
        let copy = session.submit();
        let result = mcq_grade::grade(&full, &copy.answers, copy.elapsed_seconds);
        let formulas = render::correction_formulas(&opened.formulas, &key_files);
        let content = render::Content {
            formulas: &formulas,
            ..opened.content()
        };
        let review = render::review_view(&full, &session, &result, Lang::En.texts(), content);
        // Before the correction, the explanation's formula is not in the student's table.
        let early =
            render::review_view(&full, &session, &result, Lang::En.texts(), opened.content());
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
