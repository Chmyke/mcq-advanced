//! Moodle XML question banks: import of single, multiple and true or false questions, and export
//! of a paper as a bank that Moodle imports into its question bank.
//!
//! The export keeps what a bank can hold, and the import reads it back the same (`docs/exchange.md`
//! lists what a bank cannot hold). Ids travel as Moodle id numbers, themes as categories, levels
//! and times as tags, the rule and the review pointer as the general feedback.

use std::collections::HashMap;
use std::fmt::Write as _;

use mcq_edit::{Project, slug};
use mcq_model::{
    Block, CodeBlock, CodeTokens, Issue, Level, Location, Paper, Question, Scoring, Severity,
    TokenKind,
};
use roxmltree::{Document, Node, ParsingOptions};

use crate::build::{Builder, Choice, Draft, category_path};
use crate::html::{self, Converted};
use crate::text;
use crate::{ImportOptions, Imported, Note};

const PLUGINFILE: &str = "@@PLUGINFILE@@/";
/// Marks the general feedback paragraph that holds the review pointer.
const REVIEW_PARAGRAPH: &str = "<p class=\"mcq-review\">";
const LEVEL_TAG: &str = "mcq-level-";
const TIME_TAG: &str = "mcq-time-";

pub fn import(source: &str, options: &ImportOptions) -> Imported {
    let mut b = Builder::new(options);
    let parsing = ParsingOptions {
        allow_dtd: true,
        ..ParsingOptions::default()
    };
    let doc = match Document::parse_with_options(source.trim_start_matches('\u{feff}'), parsing) {
        Ok(doc) => doc,
        Err(e) => {
            return b.unreadable(
                format!("This is not a readable XML file: {e}."),
                format!("Ce fichier XML est illisible : {e}."),
            );
        }
    };
    let root = doc.root_element();
    if root.tag_name().name() != "quiz" {
        return b.unreadable(
            "This XML file is not a Moodle question bank: no <quiz> root.".into(),
            "Ce fichier XML n'est pas une banque de questions Moodle : pas de racine <quiz>."
                .into(),
        );
    }

    let mut categories: Vec<Category> = Vec::new();
    let mut current: Option<usize> = None;
    let mut used: Vec<Vec<String>> = Vec::new();
    let mut shuffles: Vec<bool> = Vec::new();
    for node in root.children().filter(|n| n.has_tag_name("question")) {
        let kind = node.attribute("type").unwrap_or_default();
        if kind == "category" {
            categories.push(Category::read(node));
            current = Some(categories.len() - 1);
            continue;
        }
        let name = child_text(node, &["name", "text"]);
        if kind != "multichoice" && kind != "truefalse" {
            let (en, fr) = if kind == "description" {
                (
                    format!("`{name}` is a description, not a question."),
                    format!("`{name}` est une description, pas une question."),
                )
            } else {
                (
                    format!(
                        "`{name}` is of type `{kind}`: only single, multiple and true or false \
                         questions come in."
                    ),
                    format!(
                        "`{name}` est de type `{kind}` : seules les questions à choix unique, à \
                         choix multiples et vrai ou faux sont importées."
                    ),
                )
            };
            b.skip(Location::Question(name.clone()), en, fr);
            continue;
        }
        let theme_id = match current.map(|k| &categories[k]) {
            Some(category) if !category.path.is_empty() => {
                used.push(category.path.clone());
                let title = category.path.last().cloned().unwrap_or_default();
                let id = b.theme(&title, &category.idnumber);
                let course = category.info.clone();
                if let Some(theme) = b.theme_mut(&id)
                    && theme.course_ref.is_empty()
                {
                    theme.course_ref = course;
                }
                id
            }
            _ => b.default_theme(),
        };
        import_question(&mut b, node, kind, &theme_id, &mut shuffles);
    }

    // The common parent of the categories in use names the paper, unless it is Moodle's own
    // default category.
    if let Some(first) = used.first() {
        let mut parent: Vec<String> = first[..first.len().saturating_sub(1)].to_vec();
        for path in &used {
            let shared = parent.iter().zip(path).take_while(|(a, b)| a == b).count();
            parent.truncate(shared.min(path.len().saturating_sub(1)));
        }
        if let Some(title) = parent.last().filter(|t| !is_default_category(t)) {
            let description = categories
                .iter()
                .find(|c| c.path == parent)
                .map(|c| c.info.clone())
                .unwrap_or_default();
            let paper = b.paper_mut();
            paper.title = title.clone();
            paper.description = description;
        }
    }
    if !shuffles.is_empty() && shuffles.iter().all(|s| !s) {
        b.paper_mut().shuffle_options = false;
    }
    b.finish()
}

/// Moodle 1.9 kept one image beside the text: `<image>` names it, `<image_base64>` holds it.
fn legacy_image(b: &mut Builder, node: Node) -> Option<Block> {
    let name = child_text(node, &["image"]);
    let data = crate::base64::decode(&child_text(node, &["image_base64"]))?;
    let file = name.rsplit('/').next().filter(|n| !n.is_empty())?;
    let path = b.media(file, data)?;
    Some(Block::Media(mcq_model::Media {
        kind: mcq_model::MediaKind::from_path(&path)?,
        path,
        alt: String::new(),
    }))
}

struct Category {
    /// Names under the context and `top`, outermost first.
    path: Vec<String>,
    info: String,
    idnumber: String,
}

impl Category {
    fn read(node: Node) -> Self {
        let path = category_path(&child_text(node, &["category", "text"]));
        let info = child(node, "info")
            .map(|info| inline_of(rich(info, "html", &mut |_: &str| None), &mut Vec::new()))
            .unwrap_or_default();
        Self {
            path,
            info,
            idnumber: child_text(node, &["idnumber"]),
        }
    }
}

fn is_default_category(name: &str) -> bool {
    name == "top"
        || name == "Default"
        || name.starts_with("Default for ")
        || name.starts_with("Défaut pour ")
}

fn import_question(
    b: &mut Builder,
    node: Node,
    kind: &str,
    theme_id: &str,
    shuffles: &mut Vec<bool>,
) {
    let name = child_text(node, &["name", "text"]);
    let idnumber = child_text(node, &["idnumber"]);
    let at = Location::Question(name.clone());
    let mut lost: Vec<Note> = Vec::new();

    let question_text = child(node, "questiontext");
    let format = question_text
        .and_then(|n| n.attribute("format"))
        .unwrap_or("html")
        .to_string();
    let mut files = question_text.map(files_of).unwrap_or_default();
    let body = match question_text {
        Some(qt) => {
            let mut resolve = |src: &str| media_path(b, &mut files, src);
            let converted = rich(qt, "html", &mut resolve);
            lost.extend(converted.lost);
            converted.blocks
        }
        None => Vec::new(),
    };

    let mut question = Question {
        body,
        ..Question::default()
    };
    if let Some(block) = legacy_image(b, node) {
        question.body.push(block);
    }
    if let Some(feedback) = child(node, "generalfeedback") {
        let html = child_text(feedback, &["text"]);
        let (rest, review) = take_review(&html);
        let feedback_format = feedback.attribute("format").unwrap_or(&format);
        question.rule = inline_of(
            convert(&rest, feedback_format, &mut |_: &str| None),
            &mut lost,
        );
        question.course_ref = review;
    }
    for tag in descendants_text(node, "tags") {
        if let Some(level) = tag.strip_prefix(LEVEL_TAG).and_then(|n| n.parse().ok()) {
            question.level = Level::from_number(level).unwrap_or_default();
        } else if let Some(seconds) = tag.strip_prefix(TIME_TAG).and_then(|n| n.parse().ok()) {
            question.estimated_seconds = seconds;
        }
    }
    let grade = child_text(node, &["defaultgrade"]);
    if let Ok(grade) = grade.parse::<f64>()
        && (grade - 1.0).abs() > 1e-6
    {
        lost.push(Note::new(
            format!("it is worth {grade} points in Moodle, 1 here"),
            format!(
                "elle vaut {} points dans Moodle, 1 ici",
                grade.to_string().replace('.', ",")
            ),
        ));
    }

    let mut draft = Draft {
        id_hint: if idnumber.is_empty() {
            slug(&name)
        } else {
            idnumber
        },
        question,
    };
    let answers: Vec<Choice> = node
        .children()
        .filter(|n| n.has_tag_name("answer"))
        .map(|n| read_choice(n, &format, &mut lost))
        .collect();
    if kind == "truefalse" {
        true_false(b, &mut draft, &answers);
    } else {
        let single = !matches!(child_text(node, &["single"]).as_str(), "false" | "" | "0")
            || child(node, "single").is_none();
        let shuffle = matches!(child_text(node, &["shuffleanswers"]).as_str(), "true" | "1");
        shuffles.push(shuffle);
        draft.choices(&answers, single, &mut lost);
    }
    if draft.question.options.len() < 2 {
        b.skip(
            at,
            format!("`{name}` has fewer than two answers."),
            format!("`{name}` a moins de deux réponses."),
        );
        return;
    }
    b.question(theme_id, draft);
    for what in lost {
        b.simplified(at.clone(), &name, what);
    }
}

fn read_choice(node: Node, question_format: &str, lost: &mut Vec<Note>) -> Choice {
    let format = node.attribute("format").unwrap_or(question_format);
    let text = child_text(node, &["text"]);
    let feedback = child(node, "feedback")
        .map(|f| {
            let format = f.attribute("format").unwrap_or(format);
            paragraphs_of(
                convert(&child_text(f, &["text"]), format, &mut |_: &str| None),
                lost,
            )
        })
        .unwrap_or_default();
    Choice {
        fraction: node
            .attribute("fraction")
            .and_then(|f| f.trim().parse().ok())
            .unwrap_or(0.0),
        text: inline_of(convert(&text, format, &mut |_: &str| None), lost),
        feedback,
    }
}

/// Moodle's rule: true is correct when the true answer is worth 100 %, or the false one is not.
fn true_false(b: &Builder, draft: &mut Draft, answers: &[Choice]) {
    let find = |word: &str| {
        answers
            .iter()
            .find(|a| a.text.trim().eq_ignore_ascii_case(word))
    };
    let (yes, no) = match (find("true"), find("false")) {
        (Some(yes), Some(no)) => (Some(yes), Some(no)),
        _ => (answers.first(), answers.get(1)),
    };
    let true_correct = yes.is_some_and(|a| (a.fraction - 100.0).abs() < 1e-6)
        || no.is_some_and(|a| (a.fraction - 100.0).abs() >= 1e-6);
    let (true_text, false_text) = b.true_false();
    let feedback = |a: Option<&Choice>| a.map(|a| a.feedback.clone()).unwrap_or_default();
    draft.option(true_text.to_string(), true_correct, feedback(yes));
    draft.option(false_text.to_string(), !true_correct, feedback(no));
}

/// Converts a `<text>` and its siblings, read with the node's `format` attribute.
fn rich(node: Node, default_format: &str, resolve: html::Resolve<'_>) -> Converted {
    let format = node.attribute("format").unwrap_or(default_format);
    convert(&child_text(node, &["text"]), format, resolve)
}

fn convert(source: &str, format: &str, resolve: html::Resolve<'_>) -> Converted {
    match format {
        "plain_text" | "wiki_like" => Converted {
            blocks: text::plain_blocks(source),
            lost: Vec::new(),
        },
        "markdown" => Converted {
            blocks: text::markdown_blocks(source),
            lost: Vec::new(),
        },
        "moodle_auto_format" => html::to_blocks(source, true, resolve),
        _ => html::to_blocks(source, false, resolve),
    }
}

/// One line of text; what could not be kept goes to `lost`.
fn inline_of(converted: Converted, lost: &mut Vec<Note>) -> String {
    lost.extend(converted.lost);
    text::blocks_inline(&converted.blocks)
}

fn paragraphs_of(converted: Converted, lost: &mut Vec<Note>) -> String {
    lost.extend(converted.lost);
    text::blocks_paragraphs(&converted.blocks)
}

/// Splits the review paragraph written by `export` from the rest of the general feedback.
fn take_review(html: &str) -> (String, String) {
    let Some(start) = html.find(REVIEW_PARAGRAPH) else {
        return (html.to_string(), String::new());
    };
    let inner_start = start + REVIEW_PARAGRAPH.len();
    let end = html[inner_start..]
        .find("</p>")
        .map_or(html.len(), |e| inner_start + e);
    let review = inline_of(
        html::to_blocks(&html[inner_start..end], false, &mut |_: &str| None),
        &mut Vec::new(),
    );
    let after = (end + 4).min(html.len());
    (format!("{}{}", &html[..start], &html[after..]), review)
}

/// The base64 files attached to a text, by file name.
fn files_of(node: Node) -> HashMap<String, Vec<u8>> {
    node.children()
        .filter(|n| n.has_tag_name("file"))
        .filter_map(|n| {
            let name = n.attribute("name")?.to_string();
            let data = crate::base64::decode(n.text().unwrap_or_default())?;
            Some((name, data))
        })
        .collect()
}

fn media_path(b: &mut Builder, files: &mut HashMap<String, Vec<u8>>, src: &str) -> Option<String> {
    let name = percent_decode(src.strip_prefix(PLUGINFILE)?.rsplit('/').next()?);
    let data = files.get(&name)?.clone();
    b.media(&name, data)
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut k = 0;
    while k < bytes.len() {
        let hex = bytes
            .get(k + 1..k + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[k], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                k += 3;
            }
            (byte, _) => {
                out.push(byte);
                k += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn child<'a, 'i>(node: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    node.children().find(|n| n.has_tag_name(name))
}

/// The text of the element at `path` under `node`, trimmed; empty when absent.
fn child_text(node: Node, path: &[&str]) -> String {
    let mut current = node;
    for name in path {
        match child(current, name) {
            Some(next) => current = next,
            None => return String::new(),
        }
    }
    let text: String = current
        .children()
        .filter(Node::is_text)
        .filter_map(|n| n.text())
        .collect();
    text.trim().to_string()
}

/// The `<text>` of every tag under `<tags>`.
fn descendants_text(node: Node, container: &str) -> Vec<String> {
    child(node, container)
        .map(|tags| {
            tags.children()
                .filter(|n| n.has_tag_name("tag"))
                .map(|tag| child_text(tag, &["text"]))
                .collect()
        })
        .unwrap_or_default()
}

/// A Moodle XML bank, with what it leaves behind.
#[derive(Debug, Clone, PartialEq)]
pub struct Exported {
    pub xml: String,
    pub issues: Vec<Issue>,
}

/// The paper as a Moodle XML bank: one category for the paper, one per theme, then the
/// questions, media embedded, code coloured with `code` (as `mcq_highlight` gives it; a block
/// without tokens leaves uncoloured).
pub fn export(project: &Project, code: &[CodeTokens]) -> Exported {
    let paper = &project.paper;
    let mut issues = Vec::new();
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<quiz>\n");
    let title = if paper.title.trim().is_empty() {
        "Paper"
    } else {
        paper.title.trim()
    };
    let top = format!("$course$/top/{}", category_name(title));
    write_category(&mut out, &top, &paper.description, "");
    for theme in &paper.themes {
        let path = format!("{top}/{}", category_name(&theme.title));
        write_category(&mut out, &path, &theme.course_ref, &theme.id);
        for question in paper.questions.iter().filter(|q| q.theme_id == theme.id) {
            write_question(&mut out, project, code, question, &mut issues);
        }
    }
    let orphans: Vec<&Question> = paper
        .questions
        .iter()
        .filter(|q| paper.theme(&q.theme_id).is_none())
        .collect();
    if !orphans.is_empty() {
        write_category(&mut out, &top, &paper.description, "");
        for question in orphans {
            write_question(&mut out, project, code, question, &mut issues);
        }
    }
    out.push_str("</quiz>\n");
    report_settings(paper, &mut issues);
    Exported { xml: out, issues }
}

fn category_name(name: &str) -> String {
    name.replace('/', "//")
}

fn write_category(out: &mut String, path: &str, info: &str, idnumber: &str) {
    out.push_str("  <question type=\"category\">\n");
    let _ = writeln!(
        out,
        "    <category>\n      {}\n    </category>",
        text_element(path)
    );
    let _ = writeln!(
        out,
        "    <info format=\"html\">\n      {}\n    </info>",
        text_element(&html::from_markdown_inline(info))
    );
    let _ = writeln!(out, "    <idnumber>{}</idnumber>", html::escape(idnumber));
    out.push_str("  </question>\n");
}

fn write_question(
    out: &mut String,
    project: &Project,
    code: &[CodeTokens],
    question: &Question,
    issues: &mut Vec<Issue>,
) {
    let paper = &project.paper;
    let at = Location::Question(question.id.clone());
    let (body, files) = body_html(project, code, question, issues);
    let mut feedback = String::new();
    if !question.rule.is_empty() {
        feedback.push_str(&html::from_markdown(&question.rule));
    }
    if !question.course_ref.is_empty() {
        let _ = write!(
            feedback,
            "{REVIEW_PARAGRAPH}{}</p>",
            html::from_markdown_inline(&question.course_ref)
        );
    }
    let correct = question.correct_count();
    let multiple = correct > 1;
    if correct > 10 {
        issues.push(Issue::new(
            Severity::Warning,
            "EXPORT_WEIGHTS",
            at.clone(),
            "Moodle offers answer weights for up to 10 correct answers: check this question \
             after the import.",
            "Moodle propose des pondérations pour 10 bonnes réponses au plus : vérifiez cette \
             question après l'import.",
        ));
    }

    out.push_str("  <question type=\"multichoice\">\n");
    let _ = writeln!(
        out,
        "    <name>\n      {}\n    </name>",
        text_element(&question.id)
    );
    let _ = writeln!(
        out,
        "    <questiontext format=\"html\">\n      {}\n{files}    </questiontext>",
        text_element(&body)
    );
    let _ = writeln!(
        out,
        "    <generalfeedback format=\"html\">\n      {}\n    </generalfeedback>",
        text_element(&feedback)
    );
    out.push_str("    <defaultgrade>1.0000000</defaultgrade>\n");
    out.push_str("    <penalty>0.3333333</penalty>\n");
    out.push_str("    <hidden>0</hidden>\n");
    let _ = writeln!(
        out,
        "    <idnumber>{}</idnumber>",
        html::escape(&question.id)
    );
    let _ = writeln!(out, "    <single>{}</single>", !multiple);
    let _ = writeln!(
        out,
        "    <shuffleanswers>{}</shuffleanswers>",
        paper.shuffle_options
    );
    out.push_str("    <answernumbering>abc</answernumbering>\n");
    out.push_str("    <showstandardinstruction>0</showstandardinstruction>\n");
    for name in [
        "correctfeedback",
        "partiallycorrectfeedback",
        "incorrectfeedback",
    ] {
        let _ = writeln!(
            out,
            "    <{name} format=\"html\">\n      <text></text>\n    </{name}>"
        );
    }
    for option in &question.options {
        let fraction = match (option.is_correct, multiple) {
            (true, false) => "100".to_string(),
            (false, false) => "0".to_string(),
            (true, true) => weight(100.0 / correct as f64),
            (false, true) => weight(-100.0 / correct as f64),
        };
        let _ = writeln!(out, "    <answer fraction=\"{fraction}\" format=\"html\">");
        let _ = writeln!(
            out,
            "      {}",
            text_element(&html::from_markdown_inline(&option.text))
        );
        let _ = writeln!(
            out,
            "      <feedback format=\"html\">\n        {}\n      </feedback>",
            text_element(&html::from_markdown(&option.explanation))
        );
        out.push_str("    </answer>\n");
    }
    let mut tags = Vec::new();
    if let Some(n) = level_number(question.level) {
        tags.push(format!("{LEVEL_TAG}{n}"));
    }
    if question.estimated_seconds > 0 {
        tags.push(format!("{TIME_TAG}{}", question.estimated_seconds));
    }
    if !tags.is_empty() {
        out.push_str("    <tags>\n");
        for tag in tags {
            let _ = writeln!(out, "      <tag>{}</tag>", text_element(&tag));
        }
        out.push_str("    </tags>\n");
    }
    out.push_str("  </question>\n");
}

/// The statement as HTML and its `<file>` elements.
fn body_html(
    project: &Project,
    code: &[CodeTokens],
    question: &Question,
    issues: &mut Vec<Issue>,
) -> (String, String) {
    let mut body = String::new();
    let mut files = String::new();
    let mut embedded: Vec<&str> = Vec::new();
    let mut code_index = 0_u32;
    for block in &question.body {
        match block {
            Block::Text(text) => body.push_str(&html::from_markdown(text)),
            Block::Code(block) => {
                let tokens = code
                    .iter()
                    .find(|t| t.question_id == question.id && t.block == code_index);
                code_index += 1;
                write_code(&mut body, block, tokens);
            }
            Block::Media(media) => {
                let name = media.path.rsplit('/').next().unwrap_or(&media.path);
                let src = format!("{PLUGINFILE}{name}");
                let alt = html::escape(&media.alt);
                match media.kind {
                    mcq_model::MediaKind::Image => {
                        let _ = write!(body, "<p><img src=\"{src}\" alt=\"{alt}\"></p>");
                    }
                    kind => {
                        let tag = if kind == mcq_model::MediaKind::Audio {
                            "audio"
                        } else {
                            "video"
                        };
                        let _ = write!(
                            body,
                            "<p><{tag} controls=\"controls\" title=\"{alt}\"><source src=\"{src}\">{alt}</{tag}></p>"
                        );
                    }
                }
                if embedded.contains(&name) {
                    continue;
                }
                embedded.push(name);
                match project.media.get(&media.path) {
                    Some(data) => {
                        let _ = writeln!(
                            files,
                            "      <file name=\"{}\" path=\"/\" encoding=\"base64\">{}</file>",
                            html::escape(name),
                            crate::base64::encode(data)
                        );
                    }
                    None => issues.push(Issue::new(
                        Severity::Warning,
                        "EXPORT_MEDIA_MISSING",
                        Location::Question(question.id.clone()),
                        format!(
                            "`{}` is not in the paper folder: it is not embedded.",
                            media.path
                        ),
                        format!(
                            "`{}` n'est pas dans le dossier du sujet : il n'est pas intégré.",
                            media.path
                        ),
                    )),
                }
            }
        }
    }
    (body, files)
}

/// Colours of the player's light theme, written on each token: Moodle has no stylesheet for them.
fn token_colour(kind: TokenKind) -> Option<&'static str> {
    match kind {
        TokenKind::Plain | TokenKind::Operator => None,
        TokenKind::Keyword => Some("#a626a4"),
        TokenKind::Type => Some("#8a5c00"),
        TokenKind::Function => Some("#2a62d9"),
        TokenKind::String => Some("#2f6e2c"),
        TokenKind::Number => Some("#7d5500"),
        TokenKind::Constant => Some("#016a96"),
        TokenKind::Comment => Some("#65666b"),
    }
}

/// Background of a highlighted line, as in the player's light theme.
const MARKED_LINE: &str = "#fff1c2";

/// A code block as `<pre><code>`: the language as a class and the highlighted lines as
/// `data-lines` (read back by the import), the colours and marks as inline styles (what Moodle
/// shows; the import keeps only the text).
fn write_code(body: &mut String, block: &CodeBlock, tokens: Option<&CodeTokens>) {
    body.push_str("<pre");
    if !block.highlighted_lines.is_empty() {
        let lines: Vec<String> = block.highlighted_lines.iter().map(u32::to_string).collect();
        let _ = write!(body, " data-lines=\"{}\"", lines.join(","));
    }
    body.push_str("><code");
    if !block.language.is_empty() {
        let _ = write!(
            body,
            " class=\"language-{}\"",
            html::escape(&block.language)
        );
    }
    body.push('>');
    for (k, source) in block.source.split('\n').enumerate() {
        if k > 0 {
            body.push('\n');
        }
        let number = u32::try_from(k + 1).unwrap_or(u32::MAX);
        let marked = block.highlighted_lines.contains(&number);
        if marked {
            let _ = write!(body, "<span style=\"background-color: {MARKED_LINE}\">");
        }
        // Tokens are used only when they give this very line back.
        let runs = tokens
            .and_then(|t| t.lines.get(k))
            .filter(|runs| runs.iter().map(|r| r.text.as_str()).collect::<String>() == source);
        match runs {
            Some(runs) => {
                for run in runs {
                    match token_colour(run.kind) {
                        Some(colour) => {
                            let _ = write!(
                                body,
                                "<span style=\"color: {colour}\">{}</span>",
                                html::escape(&run.text)
                            );
                        }
                        None => body.push_str(&html::escape(&run.text)),
                    }
                }
            }
            None => body.push_str(&html::escape(source)),
        }
        if marked {
            body.push_str("</span>");
        }
    }
    body.push_str("</code></pre>");
}

/// `33.33333`, `-50`, `100`: Moodle's own way of writing weights.
fn weight(value: f64) -> String {
    let text = format!("{value:.5}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn level_number(level: Level) -> Option<u32> {
    (1..=5).find(|&n| Level::from_number(n) == Some(level))
}

/// `<text>` holding the value, in CDATA when it has markup.
fn text_element(value: &str) -> String {
    if value.contains(['<', '>', '&']) {
        format!(
            "<text><![CDATA[{}]]></text>",
            value.replace("]]>", "]]]]><![CDATA[>")
        )
    } else {
        format!("<text>{value}</text>")
    }
}

/// Quiz settings are not part of a question bank: say which ones stay behind.
fn report_settings(paper: &Paper, issues: &mut Vec<Issue>) {
    let defaults = Paper::new();
    // (English, French) names of the settings left behind.
    let mut left: Vec<(&str, &str)> = Vec::new();
    if paper.duration_minutes != 0 {
        left.push(("duration", "la durée"));
    }
    if paper.penalty != 0.0 || paper.unanswered_penalty != 0.0 {
        left.push(("penalties", "les pénalités"));
    }
    if paper.shuffle_questions != defaults.shuffle_questions {
        left.push(("question order", "l'ordre des questions"));
    }
    if !paper.notes.is_empty() || paper.themes.iter().any(|t| !t.notes.is_empty()) {
        left.push(("author notes", "les notes d'auteur"));
    }
    if !left.is_empty() {
        let en: Vec<&str> = left.iter().map(|(en, _)| *en).collect();
        let fr: Vec<&str> = left.iter().map(|(_, fr)| *fr).collect();
        issues.push(Issue::new(
            Severity::Info,
            "EXPORT_SETTINGS",
            Location::Paper,
            format!(
                "A Moodle question bank holds questions, not quiz settings: {} stay behind. Set \
                 them in the Moodle quiz.",
                en.join(", ")
            ),
            format!(
                "Une banque de questions Moodle contient des questions, pas les réglages du test : \
                 {} restent en arrière. Réglez-les dans le test Moodle.",
                fr.join(", ")
            ),
        ));
    }
    let multiple = paper.questions.iter().any(|q| q.correct_count() > 1);
    // Moodle's `shownumcorrect` counts the right answers after the attempt, not before it.
    if multiple && paper.show_answer_count {
        issues.push(Issue::new(
            Severity::Info,
            "EXPORT_ANSWER_COUNT",
            Location::Paper,
            "Moodle cannot tell students how many answers are correct before they answer: this \
             paper's count stays behind.",
            "Moodle ne peut pas annoncer aux élèves le nombre de bonnes réponses avant qu'ils \
             répondent : celui de ce sujet reste en arrière.",
        ));
    }
    if multiple && paper.scoring == Scoring::AllOrNothing {
        issues.push(Issue::new(
            Severity::Warning,
            "EXPORT_SCORING",
            Location::Paper,
            "Moodle gives partial credit on multiple-answer questions; this paper grades them all \
             or nothing.",
            "Moodle donne un crédit partiel aux questions à choix multiples ; ce sujet les note en \
             tout ou rien.",
        ));
    }
}
