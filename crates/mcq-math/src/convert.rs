//! LaTeX to MathML Core, with `math-core` (pure Rust, no script, output native to browsers).
//! Only the publishing side converts; the result goes through `sanitize` before it is kept.

use math_core::{LatexToMathML, MathCoreConfig, MathDisplay};

use crate::{Formula, formulas_in, sanitize};

/// The MathML of one formula, or why it cannot be shown, in words for the author.
pub fn to_mathml(latex: &str, display: bool) -> Result<String, String> {
    let converter =
        LatexToMathML::new(MathCoreConfig::default(), vec![]).map_err(|(e, _, _)| e.to_string())?;
    let mode = if display {
        MathDisplay::Block
    } else {
        MathDisplay::Inline
    };
    let converted = converter
        .convert_with_local_state(latex, mode)
        .map_err(|e| e.to_string())?;
    sanitize(&converted.mathml)
        .ok_or_else(|| "links, labels and references cannot go in a paper".to_string())
}

/// The formulas of these texts that convert, each once; the others are the linter's business.
pub fn convert_texts(texts: &[&str]) -> Vec<Formula> {
    formulas_in(texts)
        .into_iter()
        .filter_map(|(latex, display)| {
            to_mathml(latex, display).ok().map(|mathml| Formula {
                latex: latex.to_string(),
                display,
                mathml,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_school_formulas_convert() {
        for latex in [
            r"x^2 + y^2 = z^2",
            r"\frac{-b \pm \sqrt{b^2-4ac}}{2a}",
            r"\sum_{i=0}^{n-1} i = \frac{n(n-1)}{2}",
            r"O(n \log n)",
            r"\vec{v} \cdot \vec{w} = \|v\| \|w\| \cos\theta",
            r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}",
            r"\mathbb{R}^3 \to \mathbb{R}",
            r"\lim_{x \to 0} \frac{\sin x}{x} = 1",
            r"\int_0^1 f(x)\,dx",
            r"a < b \land b > c",
            r"\text{si } x \geq 0",
            r"\color{red}{x}",
            r"\$5",
        ] {
            let mathml = to_mathml(latex, false).unwrap_or_else(|e| panic!("{latex}: {e}"));
            assert!(mathml.starts_with("<math"), "{latex}: {mathml}");
        }
        let block = to_mathml("x", true).unwrap_or_default();
        assert!(block.contains("display=\"block\""), "{block}");
    }

    #[test]
    fn mistakes_are_explained() {
        for (latex, words) in [
            (r"\frac{1}{", "closing"),
            (r"\unknowncommand{x}", "unknowncommand"),
            (r"a & b", "&"),
        ] {
            let error = to_mathml(latex, false).expect_err(latex);
            assert!(error.contains(words), "{latex}: {error}");
        }
    }

    #[test]
    fn links_never_leave_the_converter() {
        for latex in [r"\href{javascript:alert(1)}{x}", r"x \tag{1}"] {
            if let Ok(mathml) = to_mathml(latex, true) {
                assert!(
                    !mathml.contains("href") && !mathml.contains(" id="),
                    "{latex}: {mathml}"
                );
            }
        }
    }

    #[test]
    fn text_is_escaped() {
        let mathml = to_mathml(r"\text{<script>alert(1)</script>}", false).unwrap_or_default();
        assert!(!mathml.contains("<script"), "{mathml}");
    }
}
