//! Plain text and Markdown from other tools, made safe for the `.qcm.md` format: characters that
//! Markdown would read as formatting are escaped, and lines that the paper reader would take for
//! an option, a keyword or a header are neutralised.

use mcq_model::{Block, CodeBlock};

/// Plain text as Markdown that renders the same characters.
pub fn escape_inline(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let space = |c: Option<char>| c.is_none_or(char::is_whitespace);
    let word = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
    let mut k = 0;
    while k < chars.len() {
        let c = chars[k];
        if matches!(c, '*' | '_' | '~') {
            // Emphasis is decided per run of the same character, by its neighbours.
            let end = chars[k..]
                .iter()
                .position(|&d| d != c)
                .map_or(chars.len(), |n| k + n);
            let prev = k.checked_sub(1).map(|p| chars[p]);
            let next = chars.get(end).copied();
            let alone = space(prev) && space(next);
            let escape = !alone && !(c == '_' && word(prev) && word(next));
            for _ in k..end {
                if escape {
                    out.push('\\');
                }
                out.push(c);
            }
            k = end;
            continue;
        }
        let next = chars.get(k + 1).copied();
        let escape = match c {
            '`' => true,
            '\\' => next.is_some_and(|n| n.is_ascii_punctuation()),
            // Only `](` can make a link or an image.
            ']' => next == Some('('),
            '&' => looks_like_entity(&chars[k + 1..]),
            _ => false,
        };
        if escape {
            out.push('\\');
        }
        out.push(c);
        k += 1;
    }
    out
}

fn looks_like_entity(rest: &[char]) -> bool {
    let end = rest.iter().take(32).position(|&c| c == ';');
    end.is_some_and(|end| {
        end > 0
            && rest[..end]
                .iter()
                .enumerate()
                .all(|(k, c)| c.is_ascii_alphanumeric() || (k == 0 && *c == '#'))
    })
}

const KEYWORDS: [&str; 7] = [
    "COURS:",
    "COURSE:",
    "REVOIR:",
    "REVIEW:",
    "REGLE:",
    "RULE:",
    "EXPLICATION:",
];

/// One line of statement as the paper reader must take it: text, never an option, a `>`
/// explanation, a keyword, a header, a code fence or a media line.
pub fn neutral_line(line: &str) -> String {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix('-') {
        let rest = rest.trim_start();
        if ["[x]", "[X]", "[ ]", "[]"]
            .iter()
            .any(|m| rest.starts_with(m))
        {
            return format!("\\{line}");
        }
    }
    if let Some(keyword) = KEYWORDS.iter().find(|k| line.starts_with(*k)) {
        let name = &keyword[..keyword.len() - 1];
        return format!("{name}\\{}", &line[name.len()..]);
    }
    if line.starts_with('>') || line.starts_with('#') || line.starts_with("![") {
        return format!("\\{line}");
    }
    if let Some(rest) = line.strip_prefix("```") {
        return format!("\\`\\`\\`{rest}");
    }
    line.to_string()
}

/// Plain text as statement blocks: one text block, lines kept, empty lines dropped.
pub fn plain_blocks(text: &str) -> Vec<Block> {
    let lines: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| neutral_line(&escape_inline(l)))
        .collect();
    if lines.is_empty() {
        Vec::new()
    } else {
        vec![Block::Text(lines.join("\n"))]
    }
}

/// Plain text on one line, for an option.
pub fn plain_inline(text: &str) -> String {
    one_line(&escape_inline(text))
}

/// Markdown as statement blocks: fenced code becomes code blocks, the rest stays Markdown.
pub fn markdown_blocks(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    let mut fence: Option<(CodeBlock, Vec<String>)> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if let Some((block, code)) = fence.as_mut() {
            if line.starts_with("```") {
                block.source = code.join("\n").trim_end().to_string();
                if let Some((block, _)) = fence.take() {
                    blocks.push(Block::Code(block));
                }
            } else {
                code.push(raw.replace('\t', "    ").trim_end().to_string());
            }
            continue;
        }
        if let Some(info) = line.strip_prefix("```") {
            flush(&mut blocks, &mut lines);
            let language = info.split_whitespace().next().unwrap_or_default();
            let block = CodeBlock {
                language: language.to_string(),
                ..CodeBlock::default()
            };
            fence = Some((block, Vec::new()));
        } else if !line.is_empty() {
            lines.push(neutral_line(line));
        }
    }
    if let Some((mut block, code)) = fence {
        block.source = code.join("\n").trim_end().to_string();
        blocks.push(Block::Code(block));
    }
    flush(&mut blocks, &mut lines);
    blocks
}

/// Pushes the pending lines as one text block.
pub fn flush(blocks: &mut Vec<Block>, lines: &mut Vec<String>) {
    if !lines.is_empty() {
        blocks.push(Block::Text(lines.join("\n")));
        lines.clear();
    }
}

/// Line breaks and runs of spaces as single spaces.
pub fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Statement blocks on one line, for an option or an explanation built from them.
pub fn blocks_inline(blocks: &[Block]) -> String {
    let parts: Vec<String> = blocks
        .iter()
        .map(|b| match b {
            Block::Text(text) => one_line(text),
            Block::Code(code) => format!("`{}`", one_line(&code.source)),
            Block::Media(media) => media.alt.clone(),
        })
        .filter(|p| !p.is_empty())
        .collect();
    parts.join(" ")
}

/// Statement blocks as explanation paragraphs, one per line.
pub fn blocks_paragraphs(blocks: &[Block]) -> String {
    let mut out: Vec<String> = Vec::new();
    for block in blocks {
        match block {
            Block::Text(text) => out.extend(text.lines().map(str::to_string)),
            Block::Code(code) => out.push(format!("`{}`", one_line(&code.source))),
            Block::Media(media) if !media.alt.is_empty() => out.push(media.alt.clone()),
            Block::Media(_) => {}
        }
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_only_what_markdown_would_read() {
        assert_eq!(escape_inline("2*3 and x * y"), "2\\*3 and x * y");
        assert_eq!(escape_inline("max_hp and _init_"), "max_hp and \\_init\\_");
        assert_eq!(escape_inline("a _____ b and **x"), "a _____ b and \\*\\*x");
        assert_eq!(escape_inline("use `print`"), "use \\`print\\`");
        assert_eq!(escape_inline("C:\\Users and \\*"), "C:\\Users and \\\\\\*");
        assert_eq!(escape_inline("tab[0] or [a](b)"), "tab[0] or [a\\](b)");
        assert_eq!(escape_inline("AT&T &lt;"), "AT&T \\&lt;");
    }

    #[test]
    fn neutralises_lines_the_reader_would_misread() {
        assert_eq!(
            neutral_line("- [x] looks like an option"),
            "\\- [x] looks like an option"
        );
        assert_eq!(neutral_line("RULE: not a rule"), "RULE\\: not a rule");
        assert_eq!(neutral_line("> quote"), "\\> quote");
        assert_eq!(neutral_line("## Q heading"), "\\## Q heading");
        assert_eq!(neutral_line("```cs"), "\\`\\`\\`cs");
        assert_eq!(neutral_line("- a list item"), "- a list item");
    }

    #[test]
    fn markdown_keeps_its_code_blocks() {
        let blocks = markdown_blocks("Read:\n```py\nx = 1\n```\nThen?");
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[1], Block::Code(c) if c.language == "py" && c.source == "x = 1"));
    }
}
