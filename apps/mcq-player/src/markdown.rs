//! Markdown of statements, options and explanations, rendered to safe HTML.
//!
//! Raw HTML is shown as text, links keep their text only, and Markdown images are dropped (media
//! come as their own blocks): even a malicious paper cannot inject markup or scripts.
//!
//! Formulas (`$...$`, `$$...$$`) are looked up in the package's table, never converted here: the
//! MathML was made when the paper was published and sanitized when the package was opened. A
//! formula missing from the table shows as its LaTeX source.

use mcq_math::{Formulas, Segment};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, html};

fn safe_events(text: &str) -> impl Iterator<Item = Event<'_>> {
    let mut in_image = 0_usize;
    Parser::new_ext(text, Options::ENABLE_STRIKETHROUGH).filter_map(move |event| match event {
        Event::Html(raw) | Event::InlineHtml(raw) => Some(Event::Text(raw)),
        Event::Start(Tag::Link { .. }) | Event::End(TagEnd::Link) => None,
        Event::Start(Tag::Image { .. }) => {
            in_image += 1;
            None
        }
        Event::End(TagEnd::Image) => {
            in_image = in_image.saturating_sub(1);
            None
        }
        _ if in_image > 0 => None,
        // Lines typed one under the other stay one under the other, as on Discord.
        Event::SoftBreak => Some(Event::HardBreak),
        other => Some(other),
    })
}

/// Marks where a formula goes while Markdown renders the prose around it. Private-use characters
/// pass through Markdown untouched.
fn placeholder(k: usize) -> String {
    format!("\u{E000}{k}\u{E001}")
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Renders the prose with `render`, then puts each formula in its place.
fn with_formulas(text: &str, formulas: &Formulas, render: impl Fn(&str) -> String) -> String {
    let split = mcq_math::split(text);
    if !split.has_math() {
        return render(text);
    }
    let mut source = String::with_capacity(text.len());
    let mut pieces: Vec<String> = Vec::new();
    for segment in split.segments {
        match segment {
            Segment::Text(prose) => source.push_str(prose),
            Segment::Math { latex, display } => {
                source.push_str(&placeholder(pieces.len()));
                pieces.push(match formulas.get(latex, display) {
                    Some(mathml) => mathml.to_string(),
                    None => {
                        let mark = if display { "$$" } else { "$" };
                        format!(
                            "<code class=\"formula-source\">{mark}{}{mark}</code>",
                            escape(latex)
                        )
                    }
                });
            }
        }
    }
    let mut out = render(&source);
    for (k, piece) in pieces.iter().enumerate() {
        out = out.replacen(&placeholder(k), piece, 1);
    }
    out
}

fn prose_block(text: &str) -> String {
    let mut out = String::new();
    html::push_html(&mut out, safe_events(text));
    out
}

fn prose_inline(text: &str) -> String {
    let html = prose_block(text);
    let trimmed = html.trim();
    trimmed
        .strip_prefix("<p>")
        .and_then(|t| t.strip_suffix("</p>"))
        .unwrap_or(trimmed)
        .to_string()
}

/// Paragraphs, lists, inline code, emphasis, formulas.
pub fn block(text: &str, formulas: &Formulas) -> String {
    with_formulas(text, formulas, prose_block)
}

/// One line of inline content, without the paragraph around it (options).
pub fn inline(text: &str, formulas: &Formulas) -> String {
    with_formulas(text, formulas, prose_inline)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcq_math::Formula;

    fn block(text: &str) -> String {
        super::block(text, &Formulas::default())
    }

    fn inline(text: &str) -> String {
        super::inline(text, &Formulas::default())
    }

    fn table() -> Formulas {
        Formulas::from_package(vec![
            Formula {
                latex: "x_1".into(),
                display: false,
                mathml: "<math><msub><mi>x</mi><mn>1</mn></msub></math>".into(),
            },
            Formula {
                latex: "e".into(),
                display: true,
                mathml: "<math display=\"block\"><mi>e</mi></math>".into(),
            },
            Formula {
                latex: "evil".into(),
                display: false,
                mathml: "<math><mtext><a href=\"javascript:alert(1)\">x</a></mtext></math>".into(),
            },
        ])
    }

    #[test]
    fn formulas_come_from_the_table() {
        let f = table();
        assert_eq!(
            super::inline("If $x_1$ is *odd*", &f),
            "If <math><msub><mi>x</mi><mn>1</mn></msub></math> is <em>odd</em>"
        );
        // Markdown never sees the LaTeX: `_1` is no emphasis.
        assert_eq!(
            super::block("So\n$$e$$", &f),
            "<p>So<br />\n<math display=\"block\"><mi>e</mi></math></p>\n"
        );
        // Unknown or unsafe formulas show as source, escaped.
        assert_eq!(
            super::inline("$a<b$ and $evil$", &f),
            "<code class=\"formula-source\">$a&lt;b$</code> and <code class=\"formula-source\">$evil$</code>"
        );
        // Prices stay prices; an escaped dollar loses its backslash.
        assert_eq!(super::inline(r"From \$5 to $10", &f), "From $5 to $10");
        assert_eq!(super::inline("`$x_1$`", &f), "<code>$x_1$</code>");
    }

    #[test]
    fn formats_like_chat() {
        assert_eq!(inline("It prints `14`"), "It prints <code>14</code>");
        assert_eq!(
            block("**not**\nnext line"),
            "<p><strong>not</strong><br />\nnext line</p>\n"
        );
    }

    #[test]
    fn markup_and_scripts_cannot_get_in() {
        let evil = "<script>alert(1)</script> <img src=x onerror=alert(1)> [click](javascript:alert(1)) ![x](javascript:alert(2))";
        let html = block(evil);
        assert!(!html.contains("<script"), "{html}");
        assert!(!html.contains("<img"), "{html}");
        assert!(!html.contains("href="), "{html}");
        assert!(!html.contains("<a "), "{html}");
        assert!(html.contains("&lt;script&gt;"), "{html}");
        assert!(html.contains("click"), "{html}");
        let link = block("[click](javascript:alert(1)) and ![pic](x.png)");
        assert_eq!(
            link,
            "<p>click and </p>
"
        );
    }
}
