//! GIFT, Moodle's text format: single, multiple and true or false questions are imported, the
//! other types are reported and left out. The grammar follows Moodle's own reader: questions
//! are separated by empty lines, `//` lines are comments, `\` escapes `~ = # { } :`.

use mcq_model::{Block, Level, Location, Question};

use crate::build::{Builder, Choice, Draft, category_path};
use crate::html;
use crate::text;
use crate::{ImportOptions, Imported};

/// Escaped characters stand in the private use area while the question is cut up.
const ESCAPES: [(&str, char, &str); 8] = [
    ("\\\\", '\u{e000}', "\\"),
    ("\\:", '\u{e001}', ":"),
    ("\\#", '\u{e002}', "#"),
    ("\\=", '\u{e003}', "="),
    ("\\{", '\u{e004}', "{"),
    ("\\}", '\u{e005}', "}"),
    ("\\~", '\u{e006}', "~"),
    ("\\n", '\u{e007}', "\n"),
];

fn protect(text: &str) -> String {
    let mut out = text.to_string();
    for (escaped, mark, _) in ESCAPES {
        out = out.replace(escaped, mark.encode_utf8(&mut [0; 4]));
    }
    out
}

fn restore(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match ESCAPES.iter().find(|(_, mark, _)| *mark == c) {
            Some((_, _, plain)) => out.push_str(plain),
            None => out.push(c),
        }
    }
    out
}

struct Chunk {
    line: usize,
    comments: Vec<String>,
    lines: Vec<String>,
}

/// Questions are the runs of non-empty lines; a run whose `{` is still open goes on past the
/// empty line, which Moodle would reject.
fn chunks(source: &str) -> Vec<Chunk> {
    let source = source.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let mut out = Vec::new();
    let mut current: Option<Chunk> = None;
    for (index, raw) in source.split('\n').enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            let open = current.as_ref().is_some_and(|c| {
                let text = protect(&c.lines.join("\n"));
                text.matches('{').count() > text.matches('}').count()
            });
            if !open && let Some(chunk) = current.take() {
                out.push(chunk);
            }
            continue;
        }
        let chunk = current.get_or_insert_with(|| Chunk {
            line: index + 1,
            comments: Vec::new(),
            lines: Vec::new(),
        });
        if let Some(comment) = line.strip_prefix("//") {
            chunk.comments.push(comment.to_string());
        } else {
            chunk.lines.push(line.to_string());
        }
    }
    out.extend(current);
    out.into_iter().filter(|c| !c.lines.is_empty()).collect()
}

pub fn import(source: &str, options: &ImportOptions) -> Imported {
    let mut b = Builder::new(options);
    let mut theme: Option<String> = None;
    for chunk in chunks(source) {
        let at = Location::Line(chunk.line);
        let text = protect(&chunk.lines.join("\n"));
        if let Some(path) = text.strip_prefix("$CATEGORY:") {
            let path = category_path(&restore(path));
            theme = path.last().map(|title| b.theme(title, ""));
            continue;
        }
        let theme_id = match &theme {
            Some(id) => id.clone(),
            None => b.default_theme(),
        };
        import_question(&mut b, &chunk, &text, &theme_id, at);
    }
    b.finish()
}

fn import_question(b: &mut Builder, chunk: &Chunk, text: &str, theme_id: &str, at: Location) {
    let (name, text) = match text.strip_prefix("::").and_then(|t| t.split_once("::")) {
        Some((name, rest)) => (restore(name).trim().to_string(), rest.trim()),
        None => (String::new(), text),
    };
    let label = if name.is_empty() {
        let start: String = restore(text).chars().take(40).collect();
        text::one_line(&start)
    } else {
        name.clone()
    };
    let (open, close) = (text.find('{'), text.find('}'));
    let (open, close) = match (open, close) {
        (None, None) => {
            b.skip(
                at,
                format!("`{label}` has no answers: a description, not a question."),
            );
            return;
        }
        (Some(open), Some(close)) if close > open => (open, close),
        _ => {
            b.skip(
                at,
                format!("`{label}`: the answer braces `{{ }}` do not match."),
            );
            return;
        }
    };
    let block = &text[open + 1..close];
    let after = text[close + 1..].trim();
    let mut statement = text[..open].trim_end().to_string();
    if !after.is_empty() {
        statement.push_str(" _____ ");
        statement.push_str(after);
    }
    let (format, statement) = split_format(&statement, "moodle");
    let (block, general) = match block.rfind("####") {
        Some(k) => (&block[..k], Some(&block[k + 4..])),
        None => (block, None),
    };

    let mut lost = Vec::new();
    let mut question = Question {
        body: blocks(statement, format, &mut lost),
        ..Question::default()
    };
    if let Some(general) = general {
        let (format, general) = split_format(general, format);
        question.rule = text::blocks_inline(&blocks(general, format, &mut lost));
    }
    let (id, tags) = id_and_tags(&chunk.comments);
    for tag in tags {
        if let Some(n) = tag.strip_prefix("mcq-level-").and_then(|n| n.parse().ok()) {
            question.level = Level::from_number(n).unwrap_or_default();
        } else if let Some(n) = tag.strip_prefix("mcq-time-").and_then(|n| n.parse().ok()) {
            question.estimated_seconds = n;
        }
    }
    let mut draft = Draft {
        id_hint: if id.is_empty() {
            mcq_edit::slug(&name)
        } else {
            id
        },
        question,
    };

    let trimmed = block.trim();
    let head = trimmed.split('#').next().unwrap_or_default().trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || (!trimmed.contains('~') && trimmed.contains("->"))
    {
        let kind = if trimmed.is_empty() {
            "an essay"
        } else if trimmed.starts_with('#') {
            "a numerical question"
        } else {
            "a matching question"
        };
        b.skip(
            at,
            format!(
                "`{label}` is {kind}: only single, multiple and true or false questions come in."
            ),
        );
        return;
    }
    if trimmed.contains('~') {
        let single = trimmed.contains('=');
        let choices: Vec<Choice> = trimmed
            .replace('=', "~=")
            .split('~')
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .map(|answer| choice(answer, format, &mut lost))
            .collect();
        if choices.len() < 2 {
            b.skip(at, format!("`{label}` has fewer than two answers."));
            return;
        }
        draft.choices(&choices, single, &mut lost);
    } else if ["T", "TRUE", "F", "FALSE"].contains(&head) {
        let mut parts = trimmed.splitn(3, '#').skip(1);
        let wrong = parts.next().unwrap_or_default();
        let right = parts.next().unwrap_or_default();
        let is_true = head.starts_with('T');
        let (on_true, on_false) = if is_true {
            (right, wrong)
        } else {
            (wrong, right)
        };
        let mut feedback = |raw: &str| {
            let (fmt, raw) = split_format(raw, format);
            text::blocks_paragraphs(&blocks(raw, fmt, &mut lost))
        };
        let (on_true, on_false) = (feedback(on_true), feedback(on_false));
        let (yes, no) = b.true_false();
        draft.option(yes.to_string(), is_true, on_true);
        draft.option(no.to_string(), !is_true, on_false);
    } else {
        b.skip(at, format!("`{label}` is a short answer question: only single, multiple and true or false questions come in."));
        return;
    }
    b.question(theme_id, draft);
    for what in lost {
        b.simplified(at.clone(), format!("`{label}`: {what}."));
    }
}

/// `=answer#feedback`, `~%50%answer`, `~answer`.
fn choice(answer: &str, format: &str, lost: &mut Vec<String>) -> Choice {
    let (fraction, rest) = if let Some(rest) = answer.strip_prefix('=') {
        (100.0, rest)
    } else {
        weight(answer)
    };
    let (text_part, feedback) = match rest.split_once('#') {
        Some((t, f)) => (t, f),
        None => (rest, ""),
    };
    let (text_format, text_part) = split_format(text_part.trim(), format);
    let (feedback_format, feedback) = split_format(feedback.trim(), format);
    Choice {
        fraction,
        text: text::blocks_inline(&blocks(text_part, text_format, lost)),
        feedback: text::blocks_paragraphs(&blocks(feedback, feedback_format, lost)),
    }
}

/// `%50%rest` or `%-33.333%rest`; no weight means 0.
fn weight(answer: &str) -> (f64, &str) {
    let parsed = answer.strip_prefix('%').and_then(|rest| {
        let (number, rest) = rest.split_once('%')?;
        Some((number.trim().parse::<f64>().ok()?, rest))
    });
    parsed.unwrap_or((0.0, answer))
}

/// `[html]text` gives (`html`, `text`); without a known prefix the default format applies.
fn split_format<'a>(text: &'a str, default: &'a str) -> (&'a str, &'a str) {
    for name in ["moodle", "html", "plain", "markdown"] {
        if let Some(rest) = text
            .strip_prefix('[')
            .and_then(|t| t.strip_prefix(name))
            .and_then(|t| t.strip_prefix(']'))
        {
            return (name, rest);
        }
    }
    (default, text)
}

fn blocks(raw: &str, format: &str, lost: &mut Vec<String>) -> Vec<Block> {
    let plain = restore(raw);
    match format {
        "plain" => text::plain_blocks(&plain),
        "markdown" => text::markdown_blocks(&plain),
        other => {
            let converted = html::to_blocks(&plain, other == "moodle", &mut |_: &str| None);
            lost.extend(converted.lost);
            converted.blocks
        }
    }
}

/// `[id:x]` (the first) and every `[tag:y]` of the comment lines; `\]` is an escaped `]`.
fn id_and_tags(comments: &[String]) -> (String, Vec<String>) {
    let mut id = String::new();
    let mut tags = Vec::new();
    for comment in comments {
        let comment = comment.replace("\\]", "\u{e010}");
        let mut rest = comment.as_str();
        while let Some(start) = rest.find('[') {
            let Some(end) = rest[start..].find(']') else {
                break;
            };
            let inner = rest[start + 1..start + end].replace('\u{e010}', "]");
            if let Some(value) = inner.strip_prefix("id:") {
                if id.is_empty() {
                    id = value.trim().to_string();
                }
            } else if let Some(value) = inner.strip_prefix("tag:") {
                tags.push(value.trim().to_string());
            }
            rest = &rest[start + end + 1..];
        }
    }
    (id, tags)
}
