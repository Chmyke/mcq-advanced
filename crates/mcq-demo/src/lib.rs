//! ROLE:    builds the website demo from `examples/demo`: per language, the practice package the
//!          player opens and the `.qcmz` the Studio opens, so a visitor sees how the paper is made.
//! OWNER:   stateless; the papers and their media are compiled in.
//! INVARIANTS: the output is the same byte for byte at every build (fixed seed, no clock), so a
//!             published demo can be checked against its source; the demo identity is public by
//!             design and signs practice packages only.

use std::convert::Infallible;

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

/// The media both papers use, by path relative to the paper.
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

/// Builds one language: the practice package and the `.qcmz`.
pub fn build(source: &Source) -> Result<Built, String> {
    let project = project(source);
    let mut rng = SeededRng::new("MCQ Advanced public demo identity");
    let identity = Identity::generate(&mut rng);
    let mut rng = SeededRng::new(&format!("MCQ Advanced demo package {}", source.lang));
    let published = mcq_publish::publish(
        &identity,
        &project.paper,
        |path: &str| {
            project
                .media
                .get(path)
                .cloned()
                .ok_or_else(|| "missing".to_string())
        },
        Mode::Practice,
        KdfParams::CODE,
        &mut rng,
    )
    .map_err(|e| e.to_string())?;
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
