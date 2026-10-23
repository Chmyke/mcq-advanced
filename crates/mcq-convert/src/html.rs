//! HTML of other tools to statement blocks, and Markdown of statements to HTML.
//!
//! The reader is tolerant: unknown tags keep their text, broken markup never fails. It knows what
//! question banks hold: paragraphs, line breaks, emphasis, inline code, `<pre>` code, lists,
//! images, sounds and videos. Everything else becomes plain text.

use mcq_model::{Block, CodeBlock, Media, MediaKind};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, html};

use crate::Note;
use crate::text::{escape_inline, flush, neutral_line};

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Open {
        name: String,
        attrs: Vec<(String, String)>,
    },
    Close(String),
    Text(String),
}

fn tokenize(source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut rest = source;
    while !rest.is_empty() {
        let Some(at) = rest.find('<') else {
            tokens.push(Token::Text(decode_entities(rest)));
            break;
        };
        if at > 0 {
            tokens.push(Token::Text(decode_entities(&rest[..at])));
        }
        rest = &rest[at..];
        let after = &rest[1..];
        if let Some(comment) = rest.strip_prefix("<!--") {
            rest = comment.find("-->").map_or("", |end| &comment[end + 3..]);
        } else if after.starts_with(['!', '?']) {
            rest = after.find('>').map_or("", |end| &after[end + 1..]);
        } else if let Some(close) = after.strip_prefix('/') {
            let end = close.find('>').unwrap_or(close.len());
            let name = close[..end].trim().to_ascii_lowercase();
            tokens.push(Token::Close(name));
            rest = close.get(end + 1..).unwrap_or("");
        } else if after.starts_with(|c: char| c.is_ascii_alphabetic()) {
            let (name, attrs, used) = read_tag(after);
            rest = &after[used..];
            let raw_text = name == "script" || name == "style";
            tokens.push(Token::Open {
                name: name.clone(),
                attrs,
            });
            if raw_text {
                let closing = format!("</{name}");
                let lower = rest.to_ascii_lowercase();
                let end = lower.find(&closing).unwrap_or(rest.len());
                rest = &rest[end..];
            }
        } else {
            tokens.push(Token::Text("<".to_string()));
            rest = after;
        }
    }
    tokens
}

/// Reads `name attr="v" ...>` and returns the name, the attributes and the bytes used.
fn read_tag(text: &str) -> (String, Vec<(String, String)>, usize) {
    let bytes = text.as_bytes();
    let mut k = 0;
    while k < bytes.len() && (bytes[k].is_ascii_alphanumeric() || bytes[k] == b'-') {
        k += 1;
    }
    let name = text[..k].to_ascii_lowercase();
    let mut attrs = Vec::new();
    loop {
        while k < bytes.len() && (bytes[k].is_ascii_whitespace() || bytes[k] == b'/') {
            k += 1;
        }
        if k >= bytes.len() {
            return (name, attrs, k);
        }
        if bytes[k] == b'>' {
            return (name, attrs, k + 1);
        }
        let start = k;
        while k < bytes.len()
            && !matches!(bytes[k], b'=' | b'>' | b'/')
            && !bytes[k].is_ascii_whitespace()
        {
            k += 1;
        }
        let key = text[start..k].to_ascii_lowercase();
        while k < bytes.len() && bytes[k].is_ascii_whitespace() {
            k += 1;
        }
        let mut value = String::new();
        if k < bytes.len() && bytes[k] == b'=' {
            k += 1;
            while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                k += 1;
            }
            if k < bytes.len() && (bytes[k] == b'"' || bytes[k] == b'\'') {
                let quote = bytes[k];
                let from = k + 1;
                k = from;
                while k < bytes.len() && bytes[k] != quote {
                    k += 1;
                }
                value = decode_entities(&text[from..k]);
                k = (k + 1).min(bytes.len());
            } else {
                let from = k;
                while k < bytes.len() && bytes[k] != b'>' && !bytes[k].is_ascii_whitespace() {
                    k += 1;
                }
                value = decode_entities(&text[from..k]);
            }
        }
        if key.is_empty() {
            k += 1;
        } else {
            attrs.push((key, value));
        }
    }
}

const ENTITIES: [(&str, &str); 30] = [
    ("amp", "&"),
    ("lt", "<"),
    ("gt", ">"),
    ("quot", "\""),
    ("apos", "'"),
    ("nbsp", " "),
    ("laquo", "«"),
    ("raquo", "»"),
    ("lsquo", "‘"),
    ("rsquo", "’"),
    ("ldquo", "“"),
    ("rdquo", "”"),
    ("hellip", "…"),
    ("ndash", "\u{2013}"),
    ("mdash", "\u{2014}"), // dash-ok: decodes what the source wrote
    ("eacute", "é"),
    ("egrave", "è"),
    ("ecirc", "ê"),
    ("agrave", "à"),
    ("acirc", "â"),
    ("ccedil", "ç"),
    ("ugrave", "ù"),
    ("ocirc", "ô"),
    ("icirc", "î"),
    ("euro", "€"),
    ("deg", "°"),
    ("times", "×"),
    ("divide", "÷"),
    ("le", "≤"),
    ("ge", "≥"),
];

pub fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let end = rest.find(';').filter(|&e| e <= 12);
        let decoded = end.and_then(|end| {
            let name = &rest[1..end];
            let c = if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                u32::from_str_radix(hex, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .map(String::from)
            } else if let Some(dec) = name.strip_prefix('#') {
                dec.parse().ok().and_then(char::from_u32).map(String::from)
            } else {
                ENTITIES
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, v)| (*v).to_string())
            };
            c.map(|c| (c, end))
        });
        match decoded {
            Some((c, end)) => {
                out.push_str(&c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Text for HTML and XML content and attribute values.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Finds the paper path of a media `src`; `None` when the file is not in the bank.
pub type Resolve<'a> = &'a mut dyn FnMut(&str) -> Option<String>;

/// What became of the HTML: the blocks and what could not be kept.
#[derive(Debug, Default)]
pub struct Converted {
    pub blocks: Vec<Block>,
    /// Plain sentences, one per thing lost.
    pub lost: Vec<Note>,
}

struct Mark {
    tag: String,
    marker: &'static str,
    emitted: bool,
}

struct Pre {
    language: String,
    lines: Vec<u32>,
    text: String,
}

struct Embed {
    /// `img`, `audio`, `video` or `a`.
    tag: String,
    src: String,
    title: String,
    text: String,
}

struct Converter<'a> {
    resolve: Resolve<'a>,
    newline_is_break: bool,
    out: Converted,
    lines: Vec<String>,
    line: String,
    marks: Vec<Mark>,
    code: usize,
    pre: Option<Pre>,
    embed: Option<Embed>,
}

const BLOCK_TAGS: [&str; 22] = [
    "p",
    "div",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "ul",
    "ol",
    "li",
    "table",
    "tr",
    "blockquote",
    "section",
    "article",
    "center",
    "dl",
    "dt",
    "dd",
    "hr",
    "figure",
];

/// HTML to statement blocks. With `newline_is_break`, line ends are line breaks, as in Moodle's
/// auto format; otherwise they are spaces, as in HTML.
pub fn to_blocks(source: &str, newline_is_break: bool, resolve: Resolve<'_>) -> Converted {
    let mut c = Converter {
        resolve,
        newline_is_break,
        out: Converted::default(),
        lines: Vec::new(),
        line: String::new(),
        marks: Vec::new(),
        code: 0,
        pre: None,
        embed: None,
    };
    for token in tokenize(source) {
        match token {
            Token::Text(text) => c.text(&text),
            Token::Open { name, attrs } => c.open(&name, &attrs),
            Token::Close(name) => c.close(&name),
        }
    }
    if let Some(pre) = c.pre.take() {
        c.push_code(pre);
    }
    c.end_line();
    flush(&mut c.out.blocks, &mut c.lines);
    c.out
}

fn attr<'v>(attrs: &'v [(String, String)], key: &str) -> &'v str {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map_or("", |(_, v)| v.as_str())
}

fn language_of(class: &str) -> Option<String> {
    class.split_whitespace().find_map(|c| {
        c.strip_prefix("language-")
            .or_else(|| c.strip_prefix("lang-"))
            .map(str::to_string)
    })
}

impl Converter<'_> {
    fn text(&mut self, text: &str) {
        if let Some(pre) = self.pre.as_mut() {
            pre.text.push_str(text);
            return;
        }
        if let Some(embed) = self.embed.as_mut() {
            embed.text.push_str(text);
            return;
        }
        if self.newline_is_break {
            for (k, piece) in text.split('\n').enumerate() {
                if k > 0 {
                    self.end_line();
                }
                self.append(piece);
            }
        } else {
            self.append(text);
        }
    }

    /// Appends text with HTML spacing: a run of white space is one space, and the text is
    /// escaped as a whole so that Markdown sees the same neighbours as the reader.
    fn append(&mut self, piece: &str) {
        let mut normal = String::with_capacity(piece.len());
        for c in piece.chars() {
            if !c.is_whitespace() {
                normal.push(c);
            } else if !normal.ends_with(' ') {
                normal.push(' ');
            }
        }
        if normal.starts_with(' ') && !self.line.is_empty() && !self.line.ends_with(' ') {
            self.line.push(' ');
        }
        let body = normal.trim();
        if body.is_empty() {
            return;
        }
        self.emit_marks();
        if self.code > 0 {
            self.line.push_str(body);
        } else {
            self.line.push_str(&escape_inline(body));
        }
        if normal.ends_with(' ') {
            self.line.push(' ');
        }
    }

    fn emit_marks(&mut self) {
        for mark in self.marks.iter_mut().filter(|m| !m.emitted) {
            self.line.push_str(mark.marker);
            mark.emitted = true;
        }
    }

    fn open(&mut self, name: &str, attrs: &[(String, String)]) {
        if let Some(pre) = self.pre.as_mut() {
            match name {
                "br" => pre.text.push('\n'),
                "code" if pre.language.is_empty() => {
                    pre.language = language_of(attr(attrs, "class")).unwrap_or_default();
                }
                _ => {}
            }
            return;
        }
        if let Some(embed) = self.embed.as_mut() {
            if name == "source" && embed.src.is_empty() {
                embed.src = attr(attrs, "src").to_string();
            }
            return;
        }
        match name {
            "br" => self.end_line(),
            "strong" | "b" => self.marks.push(Mark {
                tag: name.to_string(),
                marker: "**",
                emitted: false,
            }),
            "em" | "i" => self.marks.push(Mark {
                tag: name.to_string(),
                marker: "*",
                emitted: false,
            }),
            "code" | "tt" | "kbd" => {
                self.emit_marks();
                self.line.push('`');
                self.code += 1;
            }
            "pre" => {
                self.end_line();
                flush(&mut self.out.blocks, &mut self.lines);
                self.pre = Some(Pre {
                    language: language_of(attr(attrs, "class")).unwrap_or_default(),
                    lines: mcq_format::line_ranges(attr(attrs, "data-lines")),
                    text: String::new(),
                });
            }
            "img" => {
                let embed = Embed {
                    tag: name.to_string(),
                    src: attr(attrs, "src").to_string(),
                    title: attr(attrs, "alt").to_string(),
                    text: String::new(),
                };
                self.push_embed(embed);
            }
            "audio" | "video" => {
                self.embed = Some(Embed {
                    tag: name.to_string(),
                    src: attr(attrs, "src").to_string(),
                    title: attr(attrs, "title").to_string(),
                    text: String::new(),
                });
            }
            "a" => {
                let href = attr(attrs, "href");
                if MediaKind::from_path(href).is_some() && (self.resolve)(href).is_some() {
                    self.embed = Some(Embed {
                        tag: name.to_string(),
                        src: href.to_string(),
                        title: String::new(),
                        text: String::new(),
                    });
                }
            }
            "li" => {
                self.end_line();
                self.line.push_str("- ");
            }
            "td" | "th" => {
                if !self.line.is_empty() && !self.line.ends_with(' ') {
                    self.line.push(' ');
                }
            }
            _ if BLOCK_TAGS.contains(&name) => self.end_line(),
            _ => {}
        }
    }

    fn close(&mut self, name: &str) {
        if self.pre.is_some() {
            if name == "pre"
                && let Some(pre) = self.pre.take()
            {
                self.push_code(pre);
            }
            return;
        }
        if let Some(embed) = self.embed.as_ref() {
            if embed.tag == name
                && let Some(embed) = self.embed.take()
            {
                self.push_embed(embed);
            }
            return;
        }
        match name {
            "strong" | "b" | "em" | "i" => {
                let Some(at) = self.marks.iter().rposition(|m| m.tag == name) else {
                    return;
                };
                let mark = self.marks.remove(at);
                if mark.emitted {
                    let trailing = self.line.ends_with(' ');
                    self.line.truncate(self.line.trim_end().len());
                    self.line.push_str(mark.marker);
                    if trailing {
                        self.line.push(' ');
                    }
                }
            }
            "code" | "tt" | "kbd" if self.code > 0 => {
                self.code -= 1;
                if self.line.ends_with('`') {
                    self.line.pop();
                } else {
                    self.line.push('`');
                }
            }
            _ if BLOCK_TAGS.contains(&name) => self.end_line(),
            _ => {}
        }
    }

    /// Ends the current line: emphasis is closed on it and reopened on the next.
    fn end_line(&mut self) {
        for mark in self.marks.iter_mut().rev().filter(|m| m.emitted) {
            self.line.truncate(self.line.trim_end().len());
            self.line.push_str(mark.marker);
            mark.emitted = false;
        }
        let line = self.line.trim();
        if !line.is_empty() && line != "-" {
            self.lines.push(neutral_line(line));
        }
        self.line.clear();
    }

    fn push_code(&mut self, pre: Pre) {
        let source = pre.text.replace("\r\n", "\n").replace('\t', "    ");
        let source = source.trim_start_matches('\n').trim_end();
        self.out.blocks.push(Block::Code(CodeBlock {
            language: pre.language,
            highlighted_lines: pre.lines,
            source: source.to_string(),
        }));
    }

    fn push_embed(&mut self, embed: Embed) {
        let alt = if embed.title.trim().is_empty() {
            embed.text.trim().to_string()
        } else {
            embed.title.trim().to_string()
        };
        match (self.resolve)(&embed.src) {
            Some(path) => {
                self.end_line();
                flush(&mut self.out.blocks, &mut self.lines);
                let kind = MediaKind::from_path(&path).unwrap_or(MediaKind::Image);
                self.out
                    .blocks
                    .push(Block::Media(Media { kind, path, alt }));
            }
            None => {
                let image = embed.tag == "img";
                self.out.lost.push(Note::new(
                    format!(
                        "the {} `{}` is not inside the file: its text alternative is kept",
                        if image { "image" } else { "media" },
                        embed.src
                    ),
                    format!(
                        "{} `{}` n'est pas dans le fichier : son texte alternatif est conservé",
                        if image { "l'image" } else { "le média" },
                        embed.src
                    ),
                ));
                if !alt.is_empty() {
                    self.text(&format!("[{alt}] "));
                }
            }
        }
    }
}

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
        Event::SoftBreak => Some(Event::HardBreak),
        other => Some(other),
    })
}

/// Statement Markdown as HTML, rendered as the Player renders it: no raw HTML, no link.
pub fn from_markdown(text: &str) -> String {
    let mut out = String::new();
    html::push_html(&mut out, safe_events(text));
    out.trim_end().to_string()
}

/// One line of Markdown as inline HTML, without the paragraph around it.
pub fn from_markdown_inline(text: &str) -> String {
    let html = from_markdown(text);
    html.strip_prefix("<p>")
        .and_then(|t| t.strip_suffix("</p>"))
        .unwrap_or(&html)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocks(html: &str) -> Vec<Block> {
        to_blocks(html, false, &mut |src: &str| {
            src.strip_prefix("@@PLUGINFILE@@/")
                .map(|n| format!("media/{n}"))
        })
        .blocks
    }

    fn text(html: &str) -> String {
        match blocks(html).as_slice() {
            [Block::Text(t)] => t.clone(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn keeps_emphasis_code_and_lines() {
        assert_eq!(
            text("<p>Read <strong>this </strong>and <code>x*2</code>.</p><p>Then 2*3?<br>Yes</p>"),
            "Read **this** and `x*2`.\nThen 2\\*3?\nYes"
        );
        assert_eq!(text("<ul><li>one</li><li>two</li></ul>"), "- one\n- two");
        assert_eq!(text("a &lt; b &amp;&amp; c&#233;&nbsp;!"), "a < b && cé !");
        assert_eq!(text("x < 3 <span>ok</span>"), "x < 3 ok");
    }

    #[test]
    fn splits_code_and_media() {
        let b = blocks(
            "<p>Look:</p><pre data-lines=\"2\"><code class=\"language-cs\">int a;\n\tint b;</code></pre>\
             <p><img src=\"@@PLUGINFILE@@/a.png\" alt=\"A cat\"></p>\
             <p><a href=\"@@PLUGINFILE@@/hi.mp3\">Listen</a> then answer.</p>",
        );
        assert_eq!(b.len(), 5, "{b:?}");
        assert!(
            matches!(&b[1], Block::Code(c) if c.language == "cs" && c.source == "int a;\n    int b;" && c.highlighted_lines == vec![2])
        );
        assert!(matches!(&b[2], Block::Media(m) if m.path == "media/a.png" && m.alt == "A cat"));
        assert!(
            matches!(&b[3], Block::Media(m) if m.kind == MediaKind::Audio && m.alt == "Listen")
        );
        assert_eq!(b[4], Block::Text("then answer.".to_string()));
    }

    #[test]
    fn external_media_keep_their_text() {
        let c = to_blocks(
            "<img src=\"http://x/y.png\" alt=\"Map\">",
            false,
            &mut |_: &str| None,
        );
        assert_eq!(c.blocks, vec![Block::Text("[Map]".to_string())]);
        assert_eq!(c.lost.len(), 1);
    }

    #[test]
    fn scripts_and_comments_vanish() {
        assert_eq!(
            text("<script>alert(1)</script><!-- note -->ok<style>p{}</style>"),
            "ok"
        );
    }

    #[test]
    fn markdown_renders_like_the_player() {
        assert_eq!(
            from_markdown_inline("It prints `14`"),
            "It prints <code>14</code>"
        );
        assert_eq!(from_markdown("a\nb"), "<p>a<br />\nb</p>");
        assert_eq!(from_markdown_inline("<b>x</b>"), "&lt;b&gt;x&lt;/b&gt;");
    }
}
