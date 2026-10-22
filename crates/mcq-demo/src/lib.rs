//! ROLE:    builds the website demos from `examples/demo`. Student demo: per language, the
//!          practice package the player opens and the `.qcmz` the Studio opens. Teacher demo: a
//!          paper with an exam package and the sealed copies of a fictional class, which the
//!          Studio builds in the browser so a visitor can try writing and correcting at once.
//! OWNER:   stateless; the papers and their media are compiled in.
//! INVARIANTS: the output is the same byte for byte at every build (fixed seed, no clock), so a
//!             published demo can be checked against its source; the demo identity is public by
//!             design: it signs demo packages and opens demo copies, never a real paper.

use std::convert::Infallible;

use mcq_analysis::synth::{self, Planted};
use mcq_crypto::rand_core::{TryCryptoRng, TryRng};
use mcq_crypto::{Identity, KdfParams};
use mcq_edit::{Project, qcmz};
use mcq_publish::Mode;
use sha2::{Digest, Sha256};

/// One language of the demo.
pub struct Source {
    /// Two-letter code, also the stem of the output files (`fr.mcqpack`, `fr.qcmz`).
    pub lang: &'static str,
    pub paper: &'static str,
}

pub const SOURCES: [Source; 2] = [
    Source {
        lang: "en",
        paper: include_str!("../../../examples/demo/en.qcm.md"),
    },
    Source {
        lang: "fr",
        paper: include_str!("../../../examples/demo/fr.qcm.md"),
    },
];

/// The teacher demo, per language: its author notes say what the copies of the class reveal.
pub const TEACHER: [Source; 2] = [
    Source {
        lang: "en",
        paper: include_str!("../../../examples/demo/teacher-en.qcm.md"),
    },
    Source {
        lang: "fr",
        paper: include_str!("../../../examples/demo/teacher-fr.qcm.md"),
    },
];

/// The media the papers use, by path relative to the paper.
pub const MEDIA: [(&str, &[u8]); 1] = [(
    "media/triangle.svg",
    include_bytes!("../../../examples/demo/media/triangle.svg"),
)];

/// A built language of the demo.
pub struct Built {
    pub lang: &'static str,
    pub package: Vec<u8>,
    pub qcmz: Vec<u8>,
    pub fingerprint: String,
}

/// SHA-256 in counter mode from a public label: reproducible bytes, never a secret.
struct SeededRng {
    seed: [u8; 32],
    counter: u64,
    block: [u8; 32],
    used: usize,
}

impl SeededRng {
    fn new(label: &str) -> Self {
        Self {
            seed: Sha256::digest(label.as_bytes()).into(),
            counter: 0,
            block: [0; 32],
            used: 32,
        }
    }
}

impl TryRng for SeededRng {
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
            if self.used == self.block.len() {
                let mut h = Sha256::new();
                h.update(self.seed);
                h.update(self.counter.to_le_bytes());
                self.counter += 1;
                self.block = h.finalize().into();
                self.used = 0;
            }
            *byte = self.block[self.used];
            self.used += 1;
        }
        Ok(())
    }
}

impl TryCryptoRng for SeededRng {}

/// The project the Studio opens: the paper and the media it uses.
pub fn project(source: &Source) -> Project {
    let mut project = Project::default();
    project.set_source(source.paper);
    let used = project.used_media();
    for (path, data) in MEDIA {
        if used.iter().any(|u| u == path) {
            project.media.insert(path.to_string(), data.to_vec());
        }
    }
    project
}

/// The public demo identity: anyone can rebuild it, so it must never sign a real paper.
pub fn identity() -> Identity {
    Identity::generate(&mut SeededRng::new("MCQ Advanced public demo identity"))
}

fn publish(
    project: &Project,
    identity: &Identity,
    mode: Mode,
    label: &str,
) -> Result<mcq_publish::Published, String> {
    let mut rng = SeededRng::new(label);
    mcq_publish::publish(
        identity,
        &project.paper,
        |path: &str| {
            project
                .media
                .get(path)
                .cloned()
                .ok_or_else(|| "missing".to_string())
        },
        mode,
        KdfParams::CODE,
        &mut rng,
    )
    .map_err(|e| e.to_string())
}

/// Builds one language: the practice package and the `.qcmz`.
pub fn build(source: &Source) -> Result<Built, String> {
    let project = project(source);
    let label = format!("MCQ Advanced demo package {}", source.lang);
    let published = publish(&project, &identity(), Mode::Practice, &label)?;
    let qcmz = qcmz::export(&project).map_err(|e| e.to_string())?;
    Ok(Built {
        lang: source.lang,
        package: published.package,
        qcmz,
        fingerprint: published.fingerprint,
    })
}

/// Every language of the demo.
pub fn build_all() -> Result<Vec<Built>, String> {
    SOURCES.iter().map(build).collect()
}

/// The class of the teacher demo: fictional first names, the same in both languages.
pub const STUDENTS: [&str; 32] = [
    "Adam B.",
    "Alice M.",
    "Amir K.",
    "Anna L.",
    "Arthur P.",
    "Ines D.",
    "Chloe R.",
    "Daniel S.",
    "Elena V.",
    "Emma G.",
    "Ethan C.",
    "Eva N.",
    "Gabriel T.",
    "Hugo F.",
    "Iris H.",
    "Jade W.",
    "Jonas A.",
    "Julia E.",
    "Leo O.",
    "Lina Z.",
    "Louis Y.",
    "Maya J.",
    "Milo Q.",
    "Nina U.",
    "Noah I.",
    "Oscar X.",
    "Paul B.",
    "Rose C.",
    "Sacha D.",
    "Sofia F.",
    "Tom G.",
    "Zoe H.",
];

/// How hard each question of the teacher paper is for the class, in the paper's order: the
/// programming theme is hard throughout, so the analysis asks to reteach it.
const DIFFICULTY: [f64; 12] = [
    -1.4, -0.3, -0.7, 0.3, -1.2, -0.9, -0.8, -1.8, 0.2, 0.3, 0.5, 0.7,
];
/// The question whose key is wrong on purpose, and the option that is right in truth.
const FAULTY: (usize, usize) = (1, 0);
/// The question with a tempting distractor, and that distractor.
const TRAP: (usize, usize) = (3, 1);
const CLASS_SEED: u64 = 2026;

/// The teacher demo, ready for the Studio's Correct space.
pub struct Class {
    /// The paper as the Studio opens it, with its media.
    pub project: Project,
    /// The exam package, signed by the demo identity, with its file name.
    pub package: (String, Vec<u8>),
    /// One sealed copy per student, named by its receipt.
    pub copies: Vec<(String, Vec<u8>)>,
    /// What the copies were made to reveal.
    pub planted: Planted,
}

/// What the copies of the class are made to reveal on this paper.
pub fn planted(project: &Project) -> Result<Planted, String> {
    let paper = &project.paper;
    let pick = |(q, o): (usize, usize)| {
        let question = paper.questions.get(q).ok_or("question missing")?;
        let option = question.options.get(o).ok_or("option missing")?;
        Ok::<_, String>((question.id.clone(), option.id.clone()))
    };
    let (faulty_question, faulty_answer) = pick(FAULTY)?;
    let (trap_question, trap_option) = pick(TRAP)?;
    let weak_theme = paper.themes.last().ok_or("theme missing")?.id.clone();
    Ok(Planted {
        faulty_question,
        faulty_answer,
        trap_question,
        trap_option,
        weak_theme,
        copied: (STUDENTS[8].into(), STUDENTS[22].into()),
        distracted: STUDENTS[13].into(),
        late: STUDENTS[27].into(),
    })
}

/// Builds the teacher demo in one language: the exam package and the sealed copies of the class.
pub fn class(source: &Source) -> Result<Class, String> {
    let project = project(source);
    if project.paper.questions.len() != DIFFICULTY.len() {
        return Err("the teacher paper needs one difficulty per question".into());
    }
    let label = format!("MCQ Advanced teacher demo package {}", source.lang);
    let published = publish(
        &project,
        &identity(),
        Mode::Exam { end_code: false },
        &label,
    )?;
    let header = mcq_crypto::inspect(&published.package).map_err(|e| e.to_string())?;
    let planted = planted(&project)?;
    let names: Vec<String> = STUDENTS.iter().map(|n| (*n).to_string()).collect();
    let submissions = synth::simulate(
        &synth::Class {
            paper: &project.paper,
            paper_id: header.paper_id,
            difficulty: &DIFFICULTY,
            names: &names,
            planted: &planted,
        },
        CLASS_SEED,
    );
    let mut rng = SeededRng::new(&format!("MCQ Advanced teacher demo copies {}", source.lang));
    let copies = submissions
        .iter()
        .map(|s| {
            mcq_crypto::seal_result(&header, s, &mut rng)
                .map(|sealed| (format!("{}.mcqres", sealed.receipt), sealed.bytes))
                .map_err(|e| e.to_string())
        })
        .collect::<Result<_, _>>()?;
    Ok(Class {
        project,
        package: (
            format!("teacher-demo-{}.mcqpack", source.lang),
            published.package,
        ),
        copies,
        planted,
    })
}

/// The teacher demo in this language, English when there is none.
pub fn teacher_source(lang: &str) -> &'static Source {
    TEACHER
        .iter()
        .find(|s| s.lang == lang)
        .unwrap_or(&TEACHER[0])
}
