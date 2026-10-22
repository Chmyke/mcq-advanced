//! Markdown of statements, options and explanations, rendered to safe HTML.
//!
//! Raw HTML is shown as text, links keep their text only, and Markdown images are dropped (media
//! come as their own blocks): even a malicious paper cannot inject markup or scripts.

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

/// Paragraphs, lists, inline code, emphasis.
pub fn block(text: &str) -> String {
    let mut out = String::new();
    html::push_html(&mut out, safe_events(text));
    out
}

/// One line of inline content, without the paragraph around it (options).
pub fn inline(text: &str) -> String {
    let html = block(text);
    let trimmed = html.trim();
    trimmed
        .strip_prefix("<p>")
        .and_then(|t| t.strip_suffix("</p>"))
        .unwrap_or(trimmed)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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
