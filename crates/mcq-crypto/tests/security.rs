//! Security properties on a fictional paper, with a deterministic RNG and the cheapest key
//! derivation settings so the suite stays fast.

use std::convert::Infallible;

use mcq_crypto::rand_core::{TryCryptoRng, TryRng};
use mcq_crypto::{
    Error, Identity, KdfParams, KeyUnlock, Opening, OpeningKind, PAPER_FILE, PublishOptions,
    generate_code, inspect, normalize_code, open, open_answer_key, open_result, publish,
    receipt_code, seal_result,
};
use mcq_format::binary::Entry;
use mcq_model::{Answer, Paper, Submission};
use sha2::{Digest, Sha256};

const SAMPLE: &str = include_str!("../../mcq-format/tests/fixtures/sample.qcm.md");
const PASSPHRASE: &str = "correct horse battery staple";

/// SHA-256 in counter mode: reproducible bytes, test use only.
struct TestRng {
    seed: [u8; 32],
    counter: u64,
    buffer: Vec<u8>,
}

impl TestRng {
    fn new(label: &str) -> Self {
        Self {
            seed: Sha256::digest(label.as_bytes()).into(),
            counter: 0,
            buffer: Vec::new(),
        }
    }
}

impl TryRng for TestRng {
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
            if self.buffer.is_empty() {
                let mut h = Sha256::new();
                h.update(self.seed);
                h.update(self.counter.to_le_bytes());
                self.counter += 1;
                self.buffer = h.finalize().to_vec();
            }
            *byte = self.buffer.remove(0);
        }
        Ok(())
    }
}

impl TryCryptoRng for TestRng {}

fn sample_paper() -> Paper {
    let (mut paper, issues) = mcq_format::read(SAMPLE);
    assert!(issues.is_empty());
    for q in &mut paper.questions {
        q.line = 0;
    }
    paper
}

/// What an answer key holds: the paper without the author's notes.
fn without_notes(mut paper: Paper) -> Paper {
    paper.notes.clear();
    for theme in &mut paper.themes {
        theme.notes.clear();
    }
    paper
}

fn media() -> Vec<Entry> {
    vec![
        Entry {
            path: "media/apple.png".into(),
            data: vec![0x89, b'P', b'N', b'G'],
        },
        Entry {
            path: "media/crunch.ogg".into(),
            data: b"OggS".to_vec(),
        },
    ]
}

fn options(opening: Opening, end_code: Option<&str>) -> PublishOptions {
    PublishOptions {
        opening,
        end_code: end_code.map(str::to_string),
        kdf: KdfParams::TESTING,
    }
}

struct Exam {
    teacher: Identity,
    code: String,
    end_code: String,
    package: Vec<u8>,
}

fn exam(label: &str) -> Exam {
    let mut rng = TestRng::new(label);
    let teacher = Identity::generate(&mut rng);
    let code = generate_code(&mut rng);
    let end_code = generate_code(&mut rng);
    let package = publish(
        &teacher,
        &sample_paper(),
        &media(),
        &options(Opening::Code(code.clone()), Some(&end_code)),
        &mut rng,
    )
    .expect("publish");
    Exam {
        teacher,
        code,
        end_code,
        package,
    }
}

#[test]
fn identity_locks_and_unlocks() {
    let mut rng = TestRng::new("identity");
    let teacher = Identity::generate(&mut rng);
    let file = teacher
        .lock(PASSPHRASE, KdfParams::TESTING, &mut rng)
        .expect("lock");
    let again = Identity::unlock(&file, &format!("  {PASSPHRASE} ")).expect("unlock");
    assert_eq!(again.public(), teacher.public());
    assert_eq!(Identity::read_public(&file), Ok(teacher.public()));
    assert_eq!(
        Identity::unlock(&file, "correct horse battery stapler").map(|i| i.public()),
        Err(Error::WrongSecret)
    );
    assert_eq!(
        teacher.lock("too short", KdfParams::TESTING, &mut rng),
        Err(Error::WeakPassphrase)
    );
    let fingerprint = teacher.public().fingerprint();
    assert_eq!(fingerprint.len(), 19, "{fingerprint}");
    assert_ne!(
        Identity::generate(&mut rng).public().fingerprint(),
        fingerprint
    );
}

#[test]
fn every_altered_identity_byte_is_refused() {
    let mut rng = TestRng::new("identity tamper");
    let teacher = Identity::generate(&mut rng);
    let file = teacher
        .lock(PASSPHRASE, KdfParams::TESTING, &mut rng)
        .expect("lock");
    for i in 0..file.len() {
        let mut bad = file.clone();
        bad[i] ^= 0x01;
        assert!(Identity::unlock(&bad, PASSPHRASE).is_err(), "byte {i}");
    }
    for len in 0..file.len() {
        assert!(
            Identity::unlock(&file[..len], PASSPHRASE).is_err(),
            "length {len}"
        );
    }
}

#[test]
fn students_open_the_paper_without_its_answers() {
    let exam = exam("student");
    let header = inspect(&exam.package).expect("inspect");
    assert_eq!(header.opening, OpeningKind::Code);
    assert!(header.has_end_code);
    assert_eq!(header.teacher, exam.teacher.public());

    let typed = exam.code.to_lowercase().replace('-', " ");
    let student = open(&exam.package, &typed).expect("open");
    assert_eq!(student.media, media());
    let paper = &student.paper;
    assert_eq!(paper.title, sample_paper().title);
    for q in &paper.questions {
        assert_eq!(q.correct_count(), 0, "{}", q.id);
        assert!(q.rule.is_empty());
        assert!(q.options.iter().all(|o| o.explanation.is_empty()));
    }
    assert!(paper.questions[1].is_multiple());
    assert_eq!(paper.questions[1].answers_to_tick(), 2);
}

#[test]
fn wrong_codes_open_nothing() {
    let exam = exam("wrong code");
    assert!(matches!(
        open(&exam.package, &exam.end_code),
        Err(Error::WrongSecret)
    ));
    assert!(matches!(
        open(&exam.package, "not a code"),
        Err(Error::WrongSecret)
    ));
    assert!(matches!(
        open_answer_key(&exam.package, KeyUnlock::EndCode(&exam.code)),
        Err(Error::WrongSecret)
    ));
    assert!(matches!(
        open_answer_key(&exam.package, KeyUnlock::Practice),
        Err(Error::NotAllowed(_))
    ));
    let stranger = Identity::generate(&mut TestRng::new("stranger"));
    assert!(matches!(
        open_answer_key(&exam.package, KeyUnlock::Teacher(&stranger)),
        Err(Error::WrongSecret)
    ));
}

#[test]
fn teacher_and_end_code_open_the_full_paper() {
    let exam = exam("key");
    // Author notes never travel: students read the answer key once the end code is out.
    assert!(!sample_paper().notes.is_empty());
    let expected = without_notes(sample_paper());
    let by_teacher =
        open_answer_key(&exam.package, KeyUnlock::Teacher(&exam.teacher)).expect("teacher");
    assert_eq!(by_teacher, expected);
    let by_end_code =
        open_answer_key(&exam.package, KeyUnlock::EndCode(&exam.end_code)).expect("end code");
    assert_eq!(by_end_code, expected);
}

#[test]
fn passphrase_and_practice_packages() {
    let mut rng = TestRng::new("modes");
    let teacher = Identity::generate(&mut rng);
    let paper = sample_paper();

    let phrase = "our class motto, spring term";
    let package = publish(
        &teacher,
        &paper,
        &[],
        &options(Opening::Passphrase(phrase.into()), None),
        &mut rng,
    )
    .expect("publish");
    assert_eq!(
        inspect(&package).map(|h| h.opening),
        Ok(OpeningKind::Passphrase)
    );
    assert!(open(&package, phrase).is_ok());
    assert!(matches!(
        open(&package, "our class motto"),
        Err(Error::WrongSecret)
    ));
    assert!(matches!(
        open_answer_key(&package, KeyUnlock::EndCode("0000-0000-0000")),
        Err(Error::NotAllowed(_))
    ));
    assert_eq!(
        publish(
            &teacher,
            &paper,
            &[],
            &options(Opening::Passphrase("short".into()), None),
            &mut rng
        ),
        Err(Error::WeakPassphrase)
    );

    let practice = publish(
        &teacher,
        &paper,
        &[],
        &options(Opening::Open, None),
        &mut rng,
    )
    .expect("publish");
    assert!(open(&practice, "").is_ok());
    assert_eq!(
        open_answer_key(&practice, KeyUnlock::Practice),
        Ok(without_notes(paper.clone()))
    );
    assert_eq!(
        publish(
            &teacher,
            &paper,
            &[],
            &options(Opening::Open, Some("0000-0000-0000")),
            &mut rng
        ),
        Err(Error::NotAllowed("an end code on a practice package"))
    );
    assert_eq!(
        publish(
            &teacher,
            &paper,
            &[],
            &options(Opening::Code("bad".into()), None),
            &mut rng
        ),
        Err(Error::BadCode)
    );
}

#[test]
fn every_altered_package_byte_is_refused() {
    let exam = exam("tamper");
    for i in 0..exam.package.len() {
        let mut bad = exam.package.clone();
        bad[i] ^= 0x01;
        assert!(inspect(&bad).is_err(), "byte {i}");
    }
    for len in 0..exam.package.len() {
        assert!(inspect(&exam.package[..len]).is_err(), "length {len}");
    }
    let mut longer = exam.package.clone();
    longer.push(0);
    assert!(inspect(&longer).is_err());
}

#[test]
fn a_package_signed_by_someone_else_shows_another_fingerprint() {
    // The signature proves the package matches the key it carries; the fingerprint, compared
    // with the teacher's, is what tells a forgery apart.
    let exam = exam("genuine");
    let mut rng = TestRng::new("forger");
    let forger = Identity::generate(&mut rng);
    let forged = publish(
        &forger,
        &sample_paper(),
        &[],
        &options(Opening::Code(exam.code.clone()), None),
        &mut rng,
    )
    .expect("publish");
    let genuine = inspect(&exam.package)
        .expect("genuine")
        .teacher
        .fingerprint();
    let shown = inspect(&forged).expect("forged").teacher.fingerprint();
    assert_ne!(genuine, shown);
}

fn submission(paper_id: [u8; 16]) -> Submission {
    Submission {
        paper_id,
        student_name: "Alex Example".into(),
        answers: vec![Answer {
            question_id: "fruit-banana-colour".into(),
            selected: vec!["fruit-banana-colour-o1".into()],
            seconds_spent: 9,
        }],
        elapsed_seconds: 120,
        focus_losses: 0,
        seconds_away: 0,
        log: Vec::new(),
    }
}

#[test]
fn copies_are_sealed_for_the_teacher_only() {
    let exam = exam("result");
    let header = inspect(&exam.package).expect("inspect");
    let mut rng = TestRng::new("student device");
    let copy = submission(header.paper_id);
    let sealed = seal_result(&header, &copy, &mut rng).expect("seal");
    assert_eq!(sealed.receipt.len(), 11);
    assert_eq!(receipt_code(&sealed.bytes), sealed.receipt);

    let (opened, receipt) = open_result(&sealed.bytes, &exam.teacher).expect("open");
    assert_eq!(opened, copy);
    assert_eq!(receipt, sealed.receipt);

    let stranger = Identity::generate(&mut rng);
    assert_eq!(
        open_result(&sealed.bytes, &stranger),
        Err(Error::WrongSecret)
    );
    assert_eq!(
        seal_result(&header, &submission([0; 16]), &mut rng),
        Err(Error::NotAllowed("the copy answers another package"))
    );

    for i in 0..sealed.bytes.len() {
        let mut bad = sealed.bytes.clone();
        bad[i] ^= 0x01;
        assert!(open_result(&bad, &exam.teacher).is_err(), "byte {i}");
    }
    for len in 0..sealed.bytes.len() {
        assert!(
            open_result(&sealed.bytes[..len], &exam.teacher).is_err(),
            "length {len}"
        );
    }
}

#[test]
fn codes_have_sixty_bits_in_the_board_format() {
    let mut rng = TestRng::new("codes");
    let code = generate_code(&mut rng);
    assert_eq!(code.len(), 14, "{code}");
    assert_eq!(code.matches('-').count(), 2);
    assert!(normalize_code(&code).is_some());
    assert_ne!(generate_code(&mut rng), code);
}

#[test]
fn package_holds_the_paper_file_name() {
    assert_eq!(PAPER_FILE, "paper.qcm.md");
}

/// Reproducible outputs for other implementations: same seed, same bytes. A change here breaks
/// every file already published, so it must come with a new format version.
#[test]
fn test_vectors_are_stable() {
    let mut rng = TestRng::new("mcq-advanced test vectors v1");
    let teacher = Identity::generate(&mut rng);
    let identity_file = teacher
        .lock(PASSPHRASE, KdfParams::TESTING, &mut rng)
        .expect("lock");
    let code = generate_code(&mut rng);
    let end_code = generate_code(&mut rng);
    let package = publish(
        &teacher,
        &sample_paper(),
        &media(),
        &options(Opening::Code(code.clone()), Some(&end_code)),
        &mut rng,
    )
    .expect("publish");
    let header = inspect(&package).expect("inspect");
    let result = seal_result(&header, &submission(header.paper_id), &mut rng).expect("seal");

    let actual = format!(
        "fingerprint {}\nopening code {code}\nend code {end_code}\nreceipt {}\n\
         identity sha256 {}\npackage sha256 {}\nresult sha256 {}\n",
        teacher.public().fingerprint(),
        result.receipt,
        hex(&Sha256::digest(&identity_file)),
        hex(&Sha256::digest(&package)),
        hex(&Sha256::digest(&result.bytes)),
    );
    let expected = include_str!("vectors.txt").replace("\r\n", "\n");
    assert_eq!(actual, expected, "\nactual vectors:\n{actual}");
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
