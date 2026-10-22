//! Publishing produces a package students open with the code, and refuses unfinished papers.

use std::convert::Infallible;

use mcq_crypto::rand_core::{TryCryptoRng, TryRng};
use mcq_crypto::{Identity, KdfParams, KeyUnlock, OpeningKind};
use mcq_format::binary::CODE_TOKENS_FILE;
use mcq_publish::{Mode, PublishError, RoomSheet, publish};

const SAMPLE: &str = include_str!("../../mcq-format/tests/fixtures/sample.qcm.md");

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

fn media(path: &str) -> Result<Vec<u8>, String> {
    Ok(path.as_bytes().to_vec())
}

#[test]
fn an_exam_opens_with_its_code_and_the_sheet_keeps_the_end_code_secret() {
    let (paper, _) = mcq_format::read(SAMPLE);
    let mut rng = CountingRng(3);
    let teacher = Identity::generate(&mut rng);
    let published = publish(
        &teacher,
        &paper,
        media,
        Mode::Exam { end_code: true },
        KdfParams::TESTING,
        &mut rng,
    )
    .expect("publish");

    let code = published.opening_code.clone().expect("opening code");
    let end_code = published.end_code.clone().expect("end code");
    assert_ne!(code, end_code);
    let student = mcq_crypto::open(&published.package, &code).expect("open");
    assert_eq!(student.header.paper_id, published.paper_id);
    assert_eq!(student.paper.questions.len(), paper.questions.len());
    assert!(
        student
            .paper
            .questions
            .iter()
            .all(|q| q.correct_count() == 0)
    );
    assert!(student.media.iter().any(|f| f.path == CODE_TOKENS_FILE));
    assert!(student.media.iter().any(|f| f.path == "media/apple.png"));
    let key = mcq_crypto::open_answer_key(&published.package, KeyUnlock::EndCode(&end_code))
        .expect("answer key");
    assert_eq!(key.questions[0].correct_count(), 1);

    let sheet = RoomSheet::new(&paper, &published);
    assert_eq!(sheet.opening_code.as_deref(), Some(code.as_str()));
    assert_eq!(sheet.fingerprint, teacher.public().fingerprint());
    assert!(sheet.correction_later);
    assert!(!format!("{sheet:?}").contains(&end_code));
}

#[test]
fn practice_needs_no_code() {
    let (paper, _) = mcq_format::read(SAMPLE);
    let mut rng = CountingRng(5);
    let teacher = Identity::generate(&mut rng);
    let published = publish(
        &teacher,
        &paper,
        media,
        Mode::Practice,
        KdfParams::TESTING,
        &mut rng,
    )
    .expect("publish");
    assert_eq!(published.opening_code, None);
    assert_eq!(published.end_code, None);
    let header = mcq_crypto::inspect(&published.package).expect("inspect");
    assert_eq!(header.opening, OpeningKind::Open);
}

#[test]
fn unfinished_papers_and_missing_media_are_refused() {
    let (mut paper, _) = mcq_format::read(SAMPLE);
    let mut rng = CountingRng(7);
    let teacher = Identity::generate(&mut rng);
    let missing = publish(
        &teacher,
        &paper,
        |path: &str| Err(format!("{path} is gone")),
        Mode::Practice,
        KdfParams::TESTING,
        &mut rng,
    );
    assert!(
        matches!(missing, Err(PublishError::Media { .. })),
        "{missing:?}"
    );

    for option in &mut paper.questions[0].options {
        option.is_correct = false;
    }
    let refused = publish(
        &teacher,
        &paper,
        media,
        Mode::Practice,
        KdfParams::TESTING,
        &mut rng,
    );
    match refused {
        Err(PublishError::HasErrors(issues)) => {
            assert!(
                issues.iter().any(|i| i.code == "NO_CORRECT_OPTION"),
                "{issues:#?}"
            );
        }
        other => panic!("expected errors, got {other:?}"),
    }
}

#[test]
fn formulas_are_converted_once_and_explanations_wait_for_the_end_code() {
    let (mut paper, _) = mcq_format::read(SAMPLE);
    paper.questions[0]
        .body
        .push(mcq_model::Block::Text(r"Given $x^2 = 4$:".into()));
    paper.questions[0].options[0].explanation = r"Because $$x = \pm 2$$.".into();
    let mut rng = CountingRng(5);
    let teacher = Identity::generate(&mut rng);
    let published = publish(
        &teacher,
        &paper,
        media,
        Mode::Exam { end_code: true },
        KdfParams::TESTING,
        &mut rng,
    )
    .expect("publish");
    let student = mcq_crypto::open(
        &published.package,
        published.opening_code.as_deref().expect("code"),
    )
    .expect("open");
    let file = student
        .media
        .iter()
        .find(|f| f.path == mcq_math::FORMULAS_FILE)
        .expect("formulas for students");
    let seen = mcq_math::decode(&file.data).expect("decode");
    assert_eq!(seen.len(), 1);
    assert_eq!(
        (seen[0].latex.as_str(), seen[0].display),
        ("x^2 = 4", false)
    );
    assert!(seen[0].mathml.starts_with("<math"));
    // The explanation's formula is nowhere in what the opening code opens.
    let needle = r"\pm".as_bytes();
    assert!(
        student
            .media
            .iter()
            .all(|f| !f.data.windows(needle.len()).any(|w| w == needle))
    );

    let (_, files) = mcq_crypto::open_answer_key_files(
        &published.package,
        KeyUnlock::EndCode(published.end_code.as_deref().expect("end code")),
    )
    .expect("answer key");
    let key = mcq_math::decode(&files[0].data).expect("decode");
    assert_eq!(key.len(), 1);
    assert_eq!(
        (key[0].latex.as_str(), key[0].display),
        (r"x = \pm 2", true)
    );
}
