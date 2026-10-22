//! MathML from a package is data, not markup: it is parsed, checked against the elements and
//! attributes formulas need, and written again by this code, so what reaches the page is exactly
//! what was checked.

use roxmltree::{Document, Node, NodeType};

const MATHML_NS: &str = "http://www.w3.org/1998/Math/MathML";

/// MathML Core presentation elements, plus `span` (small capitals inside `mtext`).
const ELEMENTS: &[&str] = &[
    "math",
    "mrow",
    "mi",
    "mn",
    "mo",
    "ms",
    "mtext",
    "mspace",
    "msqrt",
    "mroot",
    "mfrac",
    "msub",
    "msup",
    "msubsup",
    "munder",
    "mover",
    "munderover",
    "mmultiscripts",
    "mprescripts",
    "none",
    "mtable",
    "mtr",
    "mtd",
    "mstyle",
    "mpadded",
    "mphantom",
    "merror",
    "menclose",
    "semantics",
    "annotation",
    "span",
];

/// Layout attributes only: no `id`, no `href`, no event handler.
const ATTRIBUTES: &[&str] = &[
    "display",
    "displaystyle",
    "scriptlevel",
    "lspace",
    "rspace",
    "stretchy",
    "symmetric",
    "largeop",
    "movablelimits",
    "accent",
    "accentunder",
    "mathvariant",
    "mathcolor",
    "mathbackground",
    "width",
    "height",
    "depth",
    "voffset",
    "linethickness",
    "minsize",
    "maxsize",
    "form",
    "fence",
    "separator",
    "columnalign",
    "rowalign",
    "columnspan",
    "rowspan",
    "notation",
    "dir",
    "encoding",
    "class",
    "style",
];

/// Named entities the converter writes; XML knows only five.
const ENTITIES: &[(&str, &str)] = &[
    ("&ApplyFunction;", "&#x2061;"),
    ("&InvisibleTimes;", "&#x2062;"),
    ("&InvisibleComma;", "&#x2063;"),
    ("&nbsp;", "&#xA0;"),
];

/// Sizes, spacing and colours; nothing that loads a resource.
fn safe_style(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || " .:;%#,()+-".contains(c))
        && !["url", "image", "expression", "var("]
            .iter()
            .any(|word| lower.contains(word))
}

fn escape(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

fn write(node: Node<'_, '_>, out: &mut String) -> Option<()> {
    match node.node_type() {
        NodeType::Text => {
            escape(node.text().unwrap_or_default(), out);
            Some(())
        }
        NodeType::Element => {
            let tag = node.tag_name();
            let name = tag.name();
            if !matches!(tag.namespace(), None | Some(MATHML_NS)) || !ELEMENTS.contains(&name) {
                return None;
            }
            out.push('<');
            out.push_str(name);
            for attr in node.attributes() {
                let key = attr.name();
                if attr.namespace().is_some()
                    || !ATTRIBUTES.contains(&key)
                    || (key == "style" && !safe_style(attr.value()))
                {
                    return None;
                }
                out.push(' ');
                out.push_str(key);
                out.push_str("=\"");
                escape(attr.value(), out);
                out.push('"');
            }
            out.push('>');
            for child in node.children() {
                write(child, out)?;
            }
            out.push_str("</");
            out.push_str(name);
            out.push('>');
            Some(())
        }
        // Comments and processing instructions carry nothing a formula needs.
        _ => Some(()),
    }
}

/// The MathML rewritten from what was checked, or `None` when it holds anything else than a
/// formula's layout: a link, an id, a script, a foreign element, a style that loads something.
pub fn sanitize(mathml: &str) -> Option<String> {
    let mut xml = mathml.to_string();
    for (named, numeric) in ENTITIES {
        xml = xml.replace(named, numeric);
    }
    // No DTD is accepted (the parser's default): no entity can be declared.
    let doc = Document::parse(&xml).ok()?;
    let root = doc.root_element();
    if root.tag_name().name() != "math" {
        return None;
    }
    let mut out = String::with_capacity(xml.len());
    write(root, &mut out)?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_formula_is_written_again_as_it_was() {
        let mathml = r#"<math display="block"><mfrac><mi>a</mi><mn>2</mn></mfrac><mo lspace="0">&ApplyFunction;</mo><mspace width="0.1667em"/></math>"#;
        assert_eq!(
            sanitize(mathml).as_deref(),
            Some(
                "<math display=\"block\"><mfrac><mi>a</mi><mn>2</mn></mfrac><mo lspace=\"0\">\u{2061}</mo><mspace width=\"0.1667em\"></mspace></math>"
            )
        );
        assert_eq!(
            sanitize("<math><mo>&lt;</mo></math>").as_deref(),
            Some("<math><mo>&lt;</mo></math>")
        );
    }

    #[test]
    fn anything_but_layout_is_refused() {
        for evil in [
            r#"<math><mtext><a href="javascript:alert(1)">x</a></mtext></math>"#,
            r#"<math><mi onclick="alert(1)">x</mi></math>"#,
            r#"<math><mi id="main">x</mi></math>"#,
            r#"<math><mi style="background:url(https://x.test/)">x</mi></math>"#,
            r#"<math><script>alert(1)</script></math>"#,
            r#"<math xmlns:x="http://www.w3.org/1999/xlink"><mi x:href="https://x.test">x</mi></math>"#,
            r#"<svg><mi>x</mi></svg>"#,
            r#"<!DOCTYPE math [<!ENTITY e "x">]><math><mi>&e;</mi></math>"#,
            r#"<math><mi>x</mi>"#,
            "<math><mi>&unknown;</mi></math>",
        ] {
            assert_eq!(sanitize(evil), None, "{evil}");
        }
    }

    #[test]
    fn styles_may_size_and_colour() {
        assert!(safe_style("font-size:1.2em; color: #c00"));
        assert!(!safe_style("background-image: url(x)"));
        assert!(!safe_style("width: var(--x)"));
        assert!(!safe_style("content:\"x\""));
    }
}
