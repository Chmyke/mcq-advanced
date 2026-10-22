//! Per-student order of questions and options, reproducible from the paper id and the name.

use mcq_model::{Paper, Question};
use sha2::{Digest, Sha256};

/// SplitMix64: small, fast, good enough to shuffle; seeded from SHA-256 so that close names
/// give unrelated orders. Not used for any secret.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn seeded(paper_id: &[u8; 16], student_name: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"mcq-advanced/order/v1");
        hasher.update(paper_id);
        hasher.update(student_name.to_uppercase().as_bytes());
        let hash: [u8; 32] = hasher.finalize().into();
        let mut seed = [0_u8; 8];
        seed.copy_from_slice(&hash[..8]);
        Self(u64::from_le_bytes(seed))
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..bound`, by rejection so no value is favoured.
    fn below(&mut self, bound: usize) -> usize {
        let bound = bound as u64;
        let zone = u64::MAX - (u64::MAX % bound);
        loop {
            let v = self.next();
            if v < zone {
                return (v % bound) as usize;
            }
        }
    }

    /// Fisher-Yates.
    pub(crate) fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }
}

/// Questions in source order, or, when shuffling: each theme shuffled, themes shuffled, then
/// interleaved one question per theme in turn, so no theme comes as one block.
pub(crate) fn questions<'a>(paper: &'a Paper, rng: &mut Rng) -> Vec<&'a Question> {
    if !paper.shuffle_questions {
        return paper.questions.iter().collect();
    }
    let mut buckets: Vec<(&str, Vec<&Question>)> = Vec::new();
    for question in &paper.questions {
        match buckets
            .iter_mut()
            .find(|(theme, _)| *theme == question.theme_id)
        {
            Some((_, bucket)) => bucket.push(question),
            None => buckets.push((&question.theme_id, vec![question])),
        }
    }
    for (_, bucket) in &mut buckets {
        rng.shuffle(bucket);
    }
    rng.shuffle(&mut buckets);
    let mut out = Vec::with_capacity(paper.questions.len());
    let longest = buckets.iter().map(|(_, b)| b.len()).max().unwrap_or(0);
    for round in 0..longest {
        for (_, bucket) in &buckets {
            if let Some(question) = bucket.get(round) {
                out.push(*question);
            }
        }
    }
    out
}
