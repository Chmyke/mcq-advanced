//! The Correct space end to end: a paper published, taken by students, sealed, dropped back into
//! the Studio, opened with the teacher's identity and analysed; then the diagnosis as shown.

use std::convert::Infallible;

use dioxus::prelude::*;
use mcq_analysis::{Copy, analyse, synth};
use mcq_crypto::rand_core::{TryCryptoRng, TryRng};
use mcq_crypto::{Identity, KdfParams};
use mcq_engine::Session;
use mcq_publish::{Mode, publish};
use mcq_studio::correct::{report_view, sheet_view};
use mcq_studio::correction::{Correction, Refusal};
use mcq_studio::texts::{EN, FR, Lang};

const SAMPLE: &str = include_str!("../../../crates/mcq-format/tests/fixtures/sample.qcm.md");

/// SplitMix64 bytes: reproducible, long period, test use only.
struct TestRng(u64);

impl TryRng for TestRng {
    type Error = Infallible;
    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        Ok(self.try_next_u64()? as u32)
    }
    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Ok(z ^ (z >> 31))
    }
    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
        for chunk in dst.chunks_mut(8) {
            let bytes = self.try_next_u64()?.to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
        Ok(())
    }
}
impl TryCryptoRng for TestRng {}

/// A package of the sample paper and sealed copies of students who tick the first option shown.
fn exam(teacher: &Identity, rng: &mut TestRng, names: &[&str]) -> (Vec<u8>, Vec<Vec<u8>>) {
    let (paper, _) = mcq_format::read(SAMPLE);
    let published = publish(
        teacher,
        &paper,
        |path: &str| Ok(path.as_bytes().to_vec()),
        Mode::Exam { end_code: false },
        KdfParams::TESTING,
        rng,
    )
    .expect("publish");
    let code = published.opening_code.clone().expect("code");
    let student = mcq_crypto::open(&published.package, &code).expect("open");
    let copies = names
        .iter()
        .map(|name| {
            let mut session = Session::new(&student.paper, student.header.paper_id, name);
            for i in 0..session.items.len() {
                session.go_to(i);
                let first = session.items[i].option_order[0].clone();
                session.toggle(&student.paper, &first);
            }
            let submission = session.submit();
            mcq_crypto::seal_result(&student.header, &submission, rng)
                .expect("seal")
                .bytes
        })
        .collect();
    (published.package, copies)
}

#[test]
fn copies_wait_for_the_identity_then_open_once() {
    let mut rng = TestRng(11);
    let teacher = Identity::generate(&mut rng);
    let stranger = Identity::generate(&mut rng);
    assert_ne!(
        teacher.public().fingerprint(),
        stranger.public().fingerprint()
    );
    let (package, copies) = exam(&teacher, &mut rng, &["Alex Example", "Sam Sample"]);
    let (_, foreign) = exam(&stranger, &mut rng, &["Other Class"]);

    let mut c = Correction::default();
    c.add_file("a.mcqres", copies[0].clone(), None);
    c.add_file("exam.mcqpack", package.clone(), None);
    c.add_file("notes.txt", b"hello".to_vec(), None);
    assert_eq!(c.pending(), 2);
    assert_eq!(c.refused, vec![("notes.txt".to_string(), Refusal::Unknown)]);
    assert!(c.report().is_none());

    c.open_pending(&teacher);
    assert_eq!(c.pending(), 0);
    assert!(c.package.is_some());
    assert_eq!(c.copies.len(), 1);

    c.add_file("b.mcqres", copies[1].clone(), Some(&teacher));
    c.add_file("a-again.mcqres", copies[0].clone(), Some(&teacher));
    c.add_file("other.mcqres", foreign[0].clone(), Some(&teacher));
    let mut broken = copies[1].clone();
    let last = broken.len() - 1;
    broken[last] ^= 1;
    c.add_file("broken.mcqres", broken, Some(&teacher));
    assert_eq!(c.copies.len(), 2);
    assert!(
        c.refused
            .contains(&("a-again.mcqres".to_string(), Refusal::AlreadyIn))
    );
    assert!(
        c.refused
            .contains(&("other.mcqres".to_string(), Refusal::NotYours))
    );
    assert!(c.refused.iter().any(|(n, _)| n == "broken.mcqres"));

    let report = c.report().expect("report");
    assert_eq!(report.students.len(), 2);
    let receipt = report.students[0].receipt.clone();
    let sheet = c.sheet(&report, &receipt).expect("sheet");
    assert_eq!(sheet.name, report.students[0].name);
    assert_eq!(
        sheet.mistakes.len(),
        report.students[0].grade.to_review().count()
    );
}

#[test]
fn a_package_for_another_teacher_is_refused() {
    let mut rng = TestRng(13);
    let teacher = Identity::generate(&mut rng);
    let stranger = Identity::generate(&mut rng);
    assert_ne!(
        teacher.public().fingerprint(),
        stranger.public().fingerprint()
    );
    let (package, _) = exam(&stranger, &mut rng, &[]);
    let mut c = Correction::default();
    c.add_file("theirs.mcqpack", package, Some(&teacher));
    assert!(c.package.is_none());
    assert_eq!(
        c.refused,
        vec![("theirs.mcqpack".to_string(), Refusal::NotYours)]
    );
}

fn synthetic() -> (mcq_model::Paper, Vec<Copy>) {
    let (paper, submissions, _) = synth::cohort(2026, 40);
    let copies = submissions
        .into_iter()
        .map(|submission| Copy {
            receipt: format!("R-{}", submission.student_name),
            submission,
        })
        .collect();
    (paper, copies)
}

fn render(view: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(view);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn the_diagnosis_reads_as_sentences_in_the_teachers_order() {
    let html = render(|| {
        let (paper, copies) = synthetic();
        let report = analyse(&paper, synth::PAPER_ID, &copies);
        report_view(
            &report,
            &paper,
            &FR,
            Lang::Fr,
            EventHandler::new(|_: String| {}),
        )
    });
    let at = |needle: &str| {
        html.find(needle)
            .unwrap_or_else(|| panic!("{needle}: {html}"))
    };
    let order = [
        at("1 · À réenseigner"),
        at("2 · Idées fausses partagées"),
        at("3 · Questions à revoir"),
        at("4 · Copies à regarder de près"),
        at("5 · Notes"),
    ];
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
    assert!(html.contains("Pointers : "), "theme to reteach: {html}");
    assert!(html.contains("ont choisi «Choice 3 of arrays-2»"), "{html}");
    assert!(
        html.contains(
            "loops-3 · corrigé ou énoncé à vérifier : les meilleurs préfèrent «Choice 2 of loops-3»"
        ),
        "{html}"
    );
    assert!(
        html.contains("Student 02 et Student 03 : mêmes réponses"),
        "{html}"
    );
    assert!(html.contains("Fiabilité du sujet (KR-20) : 0,"), "{html}");
    assert!(!html.contains("Moins de 30 copies"), "40 copies: {html}");
    assert_eq!(html.matches("Fiche de révision").count(), 40);
}

#[test]
fn a_small_class_is_told_its_figures_are_trends() {
    let html = render(|| {
        let (paper, copies) = synthetic();
        let report = analyse(&paper, synth::PAPER_ID, &copies[..6]);
        report_view(
            &report,
            &paper,
            &EN,
            Lang::En,
            EventHandler::new(|_: String| {}),
        )
    });
    assert!(html.contains("Fewer than 30 copies"), "{html}");
    assert!(html.contains("Fewer than 8 copies"), "{html}");
    assert!(!html.contains("KR-20"), "{html}");
}

#[test]
fn a_revision_sheet_shows_each_mistake_with_the_right_answer() {
    let html = render(|| {
        let (paper, copies) = synthetic();
        let report = analyse(&paper, synth::PAPER_ID, &copies);
        let student = report
            .students
            .iter()
            .find(|s| s.grade.to_review().count() > 0)
            .expect("a student with mistakes");
        let copy = copies
            .iter()
            .find(|c| c.receipt == student.receipt)
            .expect("copy");
        let sheet = mcq_analysis::sheet::revision_sheet(&paper, &report, student, copy);
        sheet_view(&sheet, &FR, Lang::Fr)
    });
    assert!(html.contains("Ce que j&#39;ai à revoir"), "{html}");
    assert!(html.contains("Mes erreurs ("), "{html}");
    assert!(html.contains("La bonne réponse :"), "{html}");
    assert!(html.contains("printable"), "{html}");
}
