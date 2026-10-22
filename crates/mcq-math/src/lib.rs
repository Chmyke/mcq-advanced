//! ROLE:    formulas in a paper's text: `$...$` inline, `$$...$$` on their own, `\$` for a dollar
//!          sign. Finds them, converts them to MathML when a paper is published (feature
//!          `convert`), carries them in the package, and checks them again where they are shown.
//! OWNER:   pure functions; no I/O.
//! INVARIANTS: players never convert, they look formulas up; every MathML shown has gone through
//!             `sanitize`, so even a forged package cannot put a link, a script or an id on the page.
//!
//! Delimiters follow Pandoc's `tex_math_dollars`: an opening `$` is followed by a non-space, a
//! closing `$` follows a non-space and is not followed by a digit, so "$5 and $10" is no formula.

#[cfg(feature = "convert")]
mod convert;
mod sanitize;

use std::collections::BTreeMap;

use mcq_format::binary::{ByteReader, ByteWriter, DecodeError};
use mcq_model::{Block, Paper};

#[cfg(feature = "convert")]
pub use convert::{convert_texts, to_mathml};
pub use sanitize::sanitize;

/// The package file holding the converted formulas.
pub const FORMULAS_FILE: &str = "formulas.mathml";
const FORMULAS_VERSION: u8 = 1;

/// A piece of text: prose (Markdown, escapes kept) or a formula's LaTeX source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment<'a> {
    Text(&'a str),
    Math { latex: &'a str, display: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Split<'a> {
    pub segments: Vec<Segment<'a>>,
    /// Dollar signs that open no formula: shown as they are, better written `\$`.
    pub stray_dollars: usize,
}

impl Split<'_> {
    pub fn has_math(&self) -> bool {
        self.segments
            .iter()
            .any(|s| matches!(s, Segment::Math { .. }))
    }
}

/// End of a backtick run starting at `i`, and the end of its matching closing run if any.
fn code_span(b: &[u8], i: usize) -> usize {
    let run = b[i..].iter().take_while(|&&c| c == b'`').count();
    let mut j = i + run;
    while j < b.len() {
        if b[j] == b'`' {
            let close = b[j..].iter().take_while(|&&c| c == b'`').count();
            if close == run {
                return j + close;
            }
            j += close;
        } else {
            j += 1;
        }
    }
    i + run
}

/// Position of the `$` closing an inline formula opened at `open`.
fn inline_close(b: &[u8], open: usize) -> Option<usize> {
    let mut j = open + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'\n' if b.get(j + 1) == Some(&b'\n') => return None,
            b'$' if !b[j - 1].is_ascii_whitespace()
                && !b.get(j + 1).is_some_and(u8::is_ascii_digit) =>
            {
                return Some(j);
            }
            _ => j += 1,
        }
    }
    None
}

/// Position of the `$$` closing a display formula opened at `open`.
fn display_close(b: &[u8], open: usize) -> Option<usize> {
    let mut j = open + 2;
    while j + 1 < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'\n' if b[j + 1] == b'\n' => return None,
            b'$' if b[j + 1] == b'$' => return Some(j),
            _ => j += 1,
        }
    }
    None
}

/// Cuts a text into prose and formulas. Code spans keep their dollars; `\$` stays an escape for
/// the Markdown renderer.
pub fn split(text: &str) -> Split<'_> {
    // Every delimiter is ASCII, so byte positions of delimiters are char boundaries.
    let b = text.as_bytes();
    let mut out = Split::default();
    let (mut start, mut i) = (0, 0);
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'`' => i = code_span(b, i),
            b'$' if b.get(i + 1) == Some(&b'$') => match display_close(b, i) {
                Some(end) if !text[i + 2..end].trim().is_empty() => {
                    if start < i {
                        out.segments.push(Segment::Text(&text[start..i]));
                    }
                    out.segments.push(Segment::Math {
                        latex: &text[i + 2..end],
                        display: true,
                    });
                    i = end + 2;
                    start = i;
                }
                _ => {
                    out.stray_dollars += 2;
                    i += 2;
                }
            },
            b'$' => {
                let opens = b.get(i + 1).is_some_and(|c| !c.is_ascii_whitespace());
                match inline_close(b, i).filter(|_| opens) {
                    Some(end) => {
                        if start < i {
                            out.segments.push(Segment::Text(&text[start..i]));
                        }
                        out.segments.push(Segment::Math {
                            latex: &text[i + 1..end],
                            display: false,
                        });
                        i = end + 1;
                        start = i;
                    }
                    None => {
                        out.stray_dollars += 1;
                        i += 1;
                    }
                }
            }
            _ => i += 1,
        }
    }
    if start < b.len() {
        out.segments.push(Segment::Text(&text[start..]));
    }
    out
}

/// What students see before the correction: the description, the statements, the options.
pub fn student_texts(paper: &Paper) -> Vec<&str> {
    let mut texts = vec![paper.description.as_str()];
    for question in &paper.questions {
        for block in &question.body {
            if let Block::Text(text) = block {
                texts.push(text);
            }
        }
        texts.extend(question.options.iter().map(|o| o.text.as_str()));
    }
    texts
}

/// What only the answer key holds: the explanations.
pub fn key_texts(paper: &Paper) -> Vec<&str> {
    paper
        .questions
        .iter()
        .flat_map(|q| q.options.iter().map(|o| o.explanation.as_str()))
        .collect()
}

/// The formulas of these texts, each once, in reading order.
pub fn formulas_in<'a>(texts: &[&'a str]) -> Vec<(&'a str, bool)> {
    let mut out: Vec<(&str, bool)> = Vec::new();
    for text in texts {
        for segment in split(text).segments {
            if let Segment::Math { latex, display } = segment
                && !out.contains(&(latex, display))
            {
                out.push((latex, display));
            }
        }
    }
    out
}

/// A formula and its MathML, as a package carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formula {
    pub latex: String,
    pub display: bool,
    pub mathml: String,
}

pub fn encode(formulas: &[Formula]) -> Vec<u8> {
    let mut w = ByteWriter::new();
    w.u8(FORMULAS_VERSION)
        .u32(u32::try_from(formulas.len()).expect("fewer than 4 billion formulas"));
    for f in formulas {
        w.u8(u8::from(f.display)).str(&f.latex).str(&f.mathml);
    }
    w.into_vec()
}

pub fn decode(data: &[u8]) -> Result<Vec<Formula>, DecodeError> {
    let mut r = ByteReader::new(data);
    let version = r.u8()?;
    if version != FORMULAS_VERSION {
        return Err(DecodeError::UnsupportedVersion(version));
    }
    let count = r.count(9)?;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let display = match r.u8()? {
            0 => false,
            1 => true,
            _ => return Err(DecodeError::Invalid("formula display")),
        };
        out.push(Formula {
            display,
            latex: r.str()?.to_string(),
            mathml: r.str()?.to_string(),
        });
    }
    r.finish()?;
    Ok(out)
}

/// The formulas a page may show, by source, each MathML already sanitized.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Formulas(BTreeMap<(bool, String), String>);

impl Formulas {
    /// Keeps the formulas whose MathML passes `sanitize`; the others will show as source.
    pub fn from_package(formulas: Vec<Formula>) -> Self {
        let mut out = Self::default();
        out.add(formulas);
        out
    }

    pub fn add(&mut self, formulas: Vec<Formula>) {
        for f in formulas {
            if let Some(safe) = sanitize(&f.mathml) {
                self.0.insert((f.display, f.latex), safe);
            }
        }
    }

    pub fn get(&self, latex: &str, display: bool) -> Option<&str> {
        self.0
            .get(&(display, latex.to_string()))
            .map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn maths(text: &str) -> Vec<(&str, bool)> {
        split(text)
            .segments
            .into_iter()
            .filter_map(|s| match s {
                Segment::Math { latex, display } => Some((latex, display)),
                Segment::Text(_) => None,
            })
            .collect()
    }

    #[test]
    fn inline_and_display_formulas() {
        let s = split("Solve $x^2 = 4$, then $$\\frac{a}{b}$$ done.");
        assert_eq!(
            s.segments,
            vec![
                Segment::Text("Solve "),
                Segment::Math {
                    latex: "x^2 = 4",
                    display: false
                },
                Segment::Text(", then "),
                Segment::Math {
                    latex: "\\frac{a}{b}",
                    display: true
                },
                Segment::Text(" done."),
            ]
        );
        assert_eq!(s.stray_dollars, 0);
    }

    #[test]
    fn prices_are_not_formulas() {
        let s = split("It costs $5 and $10.");
        assert!(!s.has_math());
        assert_eq!(s.stray_dollars, 2);
        let s = split("Pay 5 $ now.");
        assert!(!s.has_math());
        assert_eq!(s.stray_dollars, 1);
    }

    #[test]
    fn escaped_dollars_stay_text() {
        let s = split(r"From \$5 to \$10, and $n$ items");
        assert_eq!(
            maths(r"From \$5 to \$10, and $n$ items"),
            vec![("n", false)]
        );
        assert_eq!(s.stray_dollars, 0);
        assert_eq!(s.segments[0], Segment::Text(r"From \$5 to \$10, and "));
        // A dollar inside a formula, escaped as LaTeX does.
        assert_eq!(maths(r"$\$5 + x$"), vec![(r"\$5 + x", false)]);
    }

    #[test]
    fn code_keeps_its_dollars() {
        assert!(maths("Use `$\"{name}\"` and ``a ` $b$ ``").is_empty());
        assert_eq!(maths("`x` then $y$"), vec![("y", false)]);
    }

    #[test]
    fn a_blank_line_ends_a_formula() {
        assert!(maths("$a\n\nb$").is_empty());
        assert_eq!(maths("$a\nb$"), vec![("a\nb", false)]);
        assert!(maths("$$ $$").is_empty());
    }

    #[test]
    fn non_ascii_text_around_formulas() {
        assert_eq!(maths("Été : $é^2$, fin"), vec![("é^2", false)]);
    }

    #[test]
    fn the_package_file_round_trips() {
        let formulas = vec![
            Formula {
                latex: "x".into(),
                display: false,
                mathml: "<math><mi>x</mi></math>".into(),
            },
            Formula {
                latex: "y".into(),
                display: true,
                mathml: "<math display=\"block\"><mi>y</mi></math>".into(),
            },
        ];
        let data = encode(&formulas);
        assert_eq!(decode(&data), Ok(formulas));
        for n in 0..data.len() {
            assert!(decode(&data[..n]).is_err(), "truncated at {n}");
        }
    }
}
