//! ROLE:    reads the `.qcm.md` source format into a `Paper`.
//! OWNER:   stateless; `read` owns its parsing state for the duration of one call.
//! INVARIANTS: never panics on any input; every problem becomes an `Issue`;
//!             option ids are positional (`<question id>-o<k>`).
//!
//! The grammar is specified in `docs/format.md`. It stays compatible with the papers written for
//! the earlier Unity tool (French keywords) and accepts English keywords as well.

use mcq_model::{
    AnswerOption, Block, CodeBlock, Issue, Level, Location, Media, MediaKind, Paper, Question,
    Scoring, Severity, Theme,
};

/// Parses a source file. Always returns a paper, possibly partial, with the problems found.
pub fn read(source: &str) -> (Paper, Vec<Issue>) {
    let mut reader = Reader::default();
    reader.run(source);
    (reader.paper, reader.issues)
}

#[derive(Default)]
struct Reader {
    paper: Paper,
    issues: Vec<Issue>,
    question: Option<Draft>,
    theme: Option<usize>,
    fence: Option<Fence>,
}

struct Draft {
    question: Question,
    text: Vec<String>,
    /// Index of the option the next `>` line explains; cleared by any other content line.
    last_option: Option<usize>,
}

struct Fence {
    start_line: usize,
    /// `None` when the fence is outside any question: its content is dropped.
    block: Option<CodeBlock>,
    lines: Vec<String>,
}

enum Keyword {
    Course,
    Review,
    Rule,
}

impl Reader {
    fn run(&mut self, source: &str) {
        self.paper = Paper::new();
        if source.trim().is_empty() {
            self.error("EMPTY_SOURCE", Location::Paper, "The file is empty.");
            return;
        }
        let source = source.strip_prefix('\u{feff}').unwrap_or(source);
        let normalised = source.replace("\r\n", "\n").replace('\r', "\n");
        let lines: Vec<&str> = normalised.split('\n').collect();
        let body_start = self.read_front_matter(&lines);
        for (index, raw) in lines.iter().enumerate().skip(body_start) {
            self.read_line(raw, index + 1);
        }
        if let Some(fence) = self.fence.take() {
            self.error(
                "UNCLOSED_CODE_FENCE",
                Location::Line(fence.start_line),
                "A code block is never closed: check the ``` fences.",
            );
            self.close_fence(fence);
        }
        self.flush_question();
    }

    /// Returns the index of the first body line.
    fn read_front_matter(&mut self, lines: &[&str]) -> usize {
        let Some(first) = lines.iter().position(|l| !l.trim().is_empty()) else {
            return lines.len();
        };
        if lines[first].trim() != "---" {
            self.error(
                "NO_FRONTMATTER",
                Location::Line(first + 1),
                "The file must start with a `---` header holding at least `title:`.",
            );
            return first;
        }
        for (index, raw) in lines.iter().enumerate().skip(first + 1) {
            let line = raw.trim();
            if line == "---" {
                return index + 1;
            }
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let key = key.trim().to_ascii_lowercase();
            if key.is_empty() {
                continue;
            }
            let value = value.trim().trim_matches('"');
            self.read_setting(&key, value, index + 1);
        }
        self.error(
            "UNCLOSED_FRONTMATTER",
            Location::Line(first + 1),
            "The `---` header is never closed.",
        );
        lines.len()
    }

    fn read_setting(&mut self, key: &str, value: &str, line: usize) {
        let ok = match key {
            "title" => {
                self.paper.title = value.to_string();
                true
            }
            "description" => {
                self.paper.description = value.to_string();
                true
            }
            "lang" => {
                self.paper.lang = value.to_ascii_lowercase();
                true
            }
            "duration" => set(&mut self.paper.duration_minutes, value.parse().ok()),
            "penalty" => set(&mut self.paper.penalty, parse_points(value)),
            "unansweredpenalty" => set(&mut self.paper.unanswered_penalty, parse_points(value)),
            "shufflequestions" => set(&mut self.paper.shuffle_questions, parse_bool(value)),
            "shuffleoptions" => set(&mut self.paper.shuffle_options, parse_bool(value)),
            "showanswercount" => set(&mut self.paper.show_answer_count, parse_bool(value)),
            "scoring" => set(&mut self.paper.scoring, parse_scoring(value)),
            _ => {
                self.warn(
                    "UNKNOWN_KEY",
                    Location::Line(line),
                    format!("Unknown header key `{key}`: ignored."),
                );
                return;
            }
        };
        if !ok {
            self.warn(
                "BAD_VALUE",
                Location::Line(line),
                format!("Value `{value}` is not valid for `{key}`: the default is kept."),
            );
        }
    }

    fn read_line(&mut self, raw: &str, line_no: usize) {
        let line = raw.trim();

        if let Some(fence) = self.fence.as_mut() {
            if line.starts_with("```") {
                if let Some(fence) = self.fence.take() {
                    self.close_fence(fence);
                }
            } else {
                fence.lines.push(raw.replace('\t', "    "));
            }
            return;
        }

        if let Some(info) = line.strip_prefix("```") {
            self.open_fence(info, line_no);
            return;
        }
        if let Some(rest) = strip_marker(line, "# THEME") {
            self.read_theme(rest, line_no);
            return;
        }
        if let Some(rest) = strip_marker(line, "## Q") {
            self.read_question_header(rest, line_no);
            return;
        }
        if let Some((keyword, value)) = keyword(line) {
            self.read_keyword(keyword, value);
            return;
        }
        if let Some((is_correct, text)) = option_line(line) {
            self.read_option(is_correct, text, line_no);
            return;
        }
        if let Some(fragment) = line.strip_prefix('>') {
            self.read_explanation(fragment.trim(), line_no);
            return;
        }
        if line.is_empty() {
            return;
        }
        if let Some((alt, path)) = media_line(line) {
            self.read_media(alt, path, line_no);
            return;
        }
        // Statement text inside a question; author notes outside, ignored.
        if let Some(draft) = self.question.as_mut() {
            draft.text.push(line.to_string());
            draft.last_option = None;
        }
    }

    fn open_fence(&mut self, info: &str, line_no: usize) {
        let block = if let Some(draft) = self.question.as_mut() {
            draft.flush_text();
            draft.last_option = None;
            let (language, highlighted_lines) = parse_fence_info(info);
            Some(CodeBlock {
                language,
                highlighted_lines,
                source: String::new(),
            })
        } else {
            self.warn(
                "CODE_OUTSIDE_QUESTION",
                Location::Line(line_no),
                "Code block outside a question: ignored.",
            );
            None
        };
        self.fence = Some(Fence {
            start_line: line_no,
            block,
            lines: Vec::new(),
        });
    }

    fn close_fence(&mut self, fence: Fence) {
        let (Some(mut block), Some(draft)) = (fence.block, self.question.as_mut()) else {
            return;
        };
        block.source = fence.lines.join("\n").trim_end_matches('\n').to_string();
        draft.question.body.push(Block::Code(block));
    }

    fn read_theme(&mut self, rest: &str, line_no: usize) {
        self.flush_question();
        let (id, title) = match rest.split_once('|') {
            Some((id, title)) => (id.trim(), title.trim()),
            None => (rest.trim(), ""),
        };
        if id.is_empty() {
            self.error(
                "THEME_WITHOUT_ID",
                Location::Line(line_no),
                "Expected `# THEME id | title`.",
            );
            self.theme = None;
            return;
        }
        if let Some(index) = self.paper.themes.iter().position(|t| t.id == id) {
            self.error(
                "DUPLICATE_THEME",
                Location::Line(line_no),
                format!("Theme `{id}` is declared twice."),
            );
            self.theme = Some(index);
            return;
        }
        self.paper.themes.push(Theme {
            id: id.to_string(),
            title: if title.is_empty() { id } else { title }.to_string(),
            course_ref: String::new(),
        });
        self.theme = Some(self.paper.themes.len() - 1);
    }

    fn read_question_header(&mut self, rest: &str, line_no: usize) {
        self.flush_question();
        let mut parts = rest.split('|');
        let id = parts.next().unwrap_or_default().trim().to_string();
        // A question before any theme keeps an empty theme id; the linter reports it.
        let theme_id = self
            .theme
            .map(|index| self.paper.themes[index].id.clone())
            .unwrap_or_default();
        let mut question = Question {
            id,
            theme_id,
            line: line_no,
            ..Question::default()
        };
        for attribute in parts {
            self.read_attribute(&mut question, attribute, line_no);
        }
        self.question = Some(Draft {
            question,
            text: Vec::new(),
            last_option: None,
        });
    }

    fn read_attribute(&mut self, question: &mut Question, attribute: &str, line_no: usize) {
        let attribute = attribute.trim().to_lowercase();
        if attribute.is_empty() {
            return;
        }
        let level_value = attribute
            .strip_prefix("niveau")
            .or_else(|| attribute.strip_prefix("level"));
        if let Some(value) = level_value {
            match value.trim().parse().ok().and_then(Level::from_number) {
                Some(level) => question.level = level,
                None => self.warn(
                    "BAD_LEVEL",
                    Location::Line(line_no),
                    format!("Level `{}` is not between 1 and 5.", value.trim()),
                ),
            }
            return;
        }
        if let Some(seconds) = attribute
            .strip_suffix('s')
            .and_then(|n| n.trim().parse().ok())
        {
            question.estimated_seconds = seconds;
            return;
        }
        self.warn(
            "UNKNOWN_ATTRIBUTE",
            Location::Line(line_no),
            format!("Attribute `{attribute}` is not understood: ignored."),
        );
    }

    fn read_keyword(&mut self, keyword: Keyword, value: &str) {
        let value = value.trim().to_string();
        match (keyword, self.question.as_mut()) {
            (Keyword::Course | Keyword::Review, Some(draft)) => {
                draft.question.course_ref = value;
                draft.last_option = None;
            }
            (Keyword::Course, None) => {
                if let Some(index) = self.theme {
                    self.paper.themes[index].course_ref = value;
                }
            }
            (Keyword::Rule, Some(draft)) => {
                draft.question.rule = value;
                draft.last_option = None;
            }
            (Keyword::Review | Keyword::Rule, None) => {}
        }
    }

    fn read_option(&mut self, is_correct: bool, text: &str, line_no: usize) {
        let Some(draft) = self.question.as_mut() else {
            self.error(
                "OPTION_WITHOUT_QUESTION",
                Location::Line(line_no),
                "Option outside a question.",
            );
            return;
        };
        let question = &mut draft.question;
        let id = format!("{}-o{}", question.id, question.options.len() + 1);
        question.options.push(AnswerOption {
            id,
            text: text.to_string(),
            is_correct,
            explanation: String::new(),
        });
        draft.last_option = Some(question.options.len() - 1);
    }

    fn read_explanation(&mut self, fragment: &str, line_no: usize) {
        let target = self.question.as_mut().and_then(|draft| {
            let index = draft.last_option?;
            draft.question.options.get_mut(index)
        });
        let Some(option) = target else {
            self.warn(
                "ORPHAN_EXPLANATION",
                Location::Line(line_no),
                "Explanation line `>` not directly under an option: ignored.",
            );
            return;
        };
        let text = &mut option.explanation;
        if fragment.is_empty() {
            // An empty `>` line starts a new paragraph.
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            return;
        }
        if !text.is_empty() && !text.ends_with('\n') {
            text.push(' ');
        }
        text.push_str(fragment);
    }

    fn read_media(&mut self, alt: &str, path: &str, line_no: usize) {
        let Some(draft) = self.question.as_mut() else {
            return;
        };
        let Some(kind) = MediaKind::from_path(path) else {
            self.warn(
                "UNKNOWN_MEDIA",
                Location::Line(line_no),
                format!("`{path}` is not a supported image, sound or video: ignored."),
            );
            return;
        };
        draft.flush_text();
        draft.last_option = None;
        draft.question.body.push(Block::Media(Media {
            kind,
            path: path.to_string(),
            alt: alt.trim().to_string(),
        }));
    }

    fn flush_question(&mut self) {
        let Some(mut draft) = self.question.take() else {
            return;
        };
        draft.flush_text();
        self.paper.questions.push(draft.question);
    }

    fn error(&mut self, code: &'static str, location: Location, message: impl Into<String>) {
        self.issues
            .push(Issue::new(Severity::Error, code, location, message));
    }

    fn warn(&mut self, code: &'static str, location: Location, message: impl Into<String>) {
        self.issues
            .push(Issue::new(Severity::Warning, code, location, message));
    }
}

impl Draft {
    fn flush_text(&mut self) {
        if self.text.is_empty() {
            return;
        }
        let text = self.text.join("\n").trim().to_string();
        self.text.clear();
        if !text.is_empty() {
            self.question.body.push(Block::Text(text));
        }
    }
}

fn set<T>(slot: &mut T, value: Option<T>) -> bool {
    match value {
        Some(v) => {
            *slot = v;
            true
        }
        None => false,
    }
}

/// Accepts a decimal comma (`0,25`).
fn parse_points(value: &str) -> Option<f32> {
    let v: f32 = value.replace(',', ".").parse().ok()?;
    (v.is_finite() && v >= 0.0).then_some(v)
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "oui" | "yes" | "1" => Some(true),
        "false" | "non" | "no" | "0" => Some(false),
        _ => None,
    }
}

fn parse_scoring(value: &str) -> Option<Scoring> {
    match value.to_ascii_lowercase().as_str() {
        "all-or-nothing" => Some(Scoring::AllOrNothing),
        "partial" => Some(Scoring::Partial),
        _ => None,
    }
}

/// `marker` alone or followed by whitespace; returns what follows.
fn strip_marker<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(marker)?;
    (rest.is_empty() || rest.starts_with(char::is_whitespace)).then_some(rest.trim())
}

fn keyword(line: &str) -> Option<(Keyword, &str)> {
    const KEYWORDS: [(&str, Keyword); 7] = [
        ("COURS:", Keyword::Course),
        ("COURSE:", Keyword::Course),
        ("REVOIR:", Keyword::Review),
        ("REVIEW:", Keyword::Review),
        ("REGLE:", Keyword::Rule),
        ("RULE:", Keyword::Rule),
        ("EXPLICATION:", Keyword::Rule),
    ];
    KEYWORDS
        .into_iter()
        .find_map(|(prefix, keyword)| line.strip_prefix(prefix).map(|value| (keyword, value)))
}

/// `- [x] text`, `- [ ] text` or `- [] text`.
fn option_line(line: &str) -> Option<(bool, &str)> {
    let rest = line.strip_prefix('-')?.trim_start();
    let (is_correct, text) = if let Some(t) = rest.strip_prefix("[x]") {
        (true, t)
    } else if let Some(t) = rest.strip_prefix("[X]") {
        (true, t)
    } else if let Some(t) = rest.strip_prefix("[ ]") {
        (false, t)
    } else {
        (false, rest.strip_prefix("[]")?)
    };
    Some((is_correct, text.trim()))
}

/// A whole line `![alt](path)`.
fn media_line(line: &str) -> Option<(&str, &str)> {
    let inner = line.strip_prefix("![")?.strip_suffix(')')?;
    let (alt, path) = inner.split_once("](")?;
    let path = path.trim();
    (!path.is_empty()).then_some((alt, path))
}

/// `cs {2,4-5}` gives (`cs`, [2, 4, 5]). Unreadable ranges are ignored.
fn parse_fence_info(info: &str) -> (String, Vec<u32>) {
    let info = info.trim();
    let (head, lines) = match info.split_once('{') {
        Some((head, rest)) => (head, rest.split_once('}').map_or("", |(inside, _)| inside)),
        None => (info, ""),
    };
    let language = head
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string();
    let mut highlighted = Vec::new();
    for part in lines.split(',') {
        let part = part.trim();
        match part.split_once('-') {
            Some((a, b)) => {
                if let (Ok(a), Ok(b)) = (a.trim().parse::<u32>(), b.trim().parse::<u32>()) {
                    if a <= b && b - a < 1000 {
                        highlighted.extend(a..=b);
                    }
                }
            }
            None => {
                if let Ok(n) = part.parse() {
                    highlighted.push(n);
                }
            }
        }
    }
    highlighted.retain(|&n| n > 0);
    highlighted.sort_unstable();
    highlighted.dedup();
    (language, highlighted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fence_info_reads_language_and_lines() {
        assert_eq!(parse_fence_info("cs"), ("cs".into(), vec![]));
        assert_eq!(parse_fence_info("cpp {3}"), ("cpp".into(), vec![3]));
        assert_eq!(
            parse_fence_info("python {5, 1-2}"),
            ("python".into(), vec![1, 2, 5])
        );
        assert_eq!(parse_fence_info("{2}"), (String::new(), vec![2]));
        assert_eq!(parse_fence_info("js {x,0,9-3}"), ("js".into(), vec![]));
    }

    #[test]
    fn option_line_accepts_the_three_box_forms() {
        assert_eq!(option_line("- [x] Yes"), Some((true, "Yes")));
        assert_eq!(option_line("- [X] Yes"), Some((true, "Yes")));
        assert_eq!(option_line("- [ ] No"), Some((false, "No")));
        assert_eq!(option_line("-[] No"), Some((false, "No")));
        assert_eq!(option_line("- plain item"), None);
    }

    #[test]
    fn markers_need_a_word_boundary() {
        assert_eq!(strip_marker("# THEME a | b", "# THEME"), Some("a | b"));
        assert_eq!(strip_marker("# THEMES", "# THEME"), None);
        assert_eq!(strip_marker("## Questions", "## Q"), None);
    }

    #[test]
    fn media_line_needs_the_whole_line() {
        assert_eq!(
            media_line("![A cat](media/cat.png)"),
            Some(("A cat", "media/cat.png"))
        );
        assert_eq!(media_line("See ![x](a.png) here"), None);
        assert_eq!(media_line("![x]()"), None);
    }
}
