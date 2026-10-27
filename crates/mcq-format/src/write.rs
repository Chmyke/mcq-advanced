//! Writes a `Paper` back to the `.qcm.md` format, and derives the student copy of a paper.
//!
//! The output is canonical: `read(&write(&paper))` gives the paper back, source lines aside, for
//! any paper the reader produced whose themes hold contiguous questions.

use std::fmt::Write as _;

use mcq_model::{Block, Level, Paper, Question, Scoring};

/// The canonical `.qcm.md` text of a paper.
pub fn write(paper: &Paper) -> String {
    let mut out = String::new();
    write_header(&mut out, paper);
    if !paper.notes.is_empty() {
        let _ = writeln!(out, "{}", paper.notes);
    }
    // Questions without a theme can only come first, before any `# THEME`.
    for question in paper.questions.iter().filter(|q| q.theme_id.is_empty()) {
        write_question(&mut out, question);
    }
    for theme in &paper.themes {
        out.push('\n');
        if theme.title.is_empty() || theme.title == theme.id {
            let _ = writeln!(out, "# THEME {}", theme.id);
        } else {
            let _ = writeln!(out, "# THEME {} | {}", theme.id, theme.title);
        }
        if !theme.course_ref.is_empty() {
            let _ = writeln!(out, "COURSE: {}", theme.course_ref);
        }
        if !theme.notes.is_empty() {
            let _ = writeln!(out, "{}", theme.notes);
        }
        for question in paper.questions.iter().filter(|q| q.theme_id == theme.id) {
            write_question(&mut out, question);
        }
    }
    out
}

/// What students receive: no correct flag, no explanation, no rule, no author note, but the
/// number of options to tick on every question, so multiple-answer questions stay announced.
pub fn student_view(paper: &Paper) -> Paper {
    let mut copy = paper.clone();
    copy.notes.clear();
    for theme in &mut copy.themes {
        theme.notes.clear();
    }
    for question in &mut copy.questions {
        question.declared_answers = u32::try_from(question.answers_to_tick()).unwrap_or(u32::MAX);
        question.rule.clear();
        for option in &mut question.options {
            option.is_correct = false;
            option.explanation.clear();
        }
    }
    copy
}

fn write_header(out: &mut String, paper: &Paper) {
    out.push_str("---\n");
    let _ = writeln!(out, "title: {}", paper.title);
    if !paper.description.is_empty() {
        let _ = writeln!(out, "description: {}", paper.description);
    }
    if !paper.lang.is_empty() {
        let _ = writeln!(out, "lang: {}", paper.lang);
    }
    if !paper.languages.is_empty() {
        let _ = writeln!(out, "languages: {}", paper.languages.join(", "));
    }
    let scoring = match paper.scoring {
        Scoring::AllOrNothing => "all-or-nothing",
        Scoring::Partial => "partial",
    };
    let _ = writeln!(out, "duration: {}", paper.duration_minutes);
    let _ = writeln!(out, "penalty: {}", paper.penalty);
    let _ = writeln!(out, "unansweredPenalty: {}", paper.unanswered_penalty);
    let _ = writeln!(out, "scoring: {scoring}");
    let _ = writeln!(out, "shuffleQuestions: {}", paper.shuffle_questions);
    let _ = writeln!(out, "shuffleOptions: {}", paper.shuffle_options);
    let _ = writeln!(out, "showAnswerCount: {}", paper.show_answer_count);
    out.push_str("---\n");
}

fn write_question(out: &mut String, question: &Question) {
    let _ = write!(out, "\n## Q {}", question.id);
    if let Some(n) = level_number(question.level) {
        let _ = write!(out, " | level {n}");
    }
    if question.estimated_seconds > 0 {
        let _ = write!(out, " | {}s", question.estimated_seconds);
    }
    if question.declared_answers > 0 {
        let _ = write!(out, " | answers {}", question.declared_answers);
    }
    out.push('\n');

    for block in &question.body {
        match block {
            Block::Text(text) => {
                let _ = writeln!(out, "{text}");
            }
            Block::Code(code) => {
                out.push_str("```");
                out.push_str(&code.language);
                if !code.highlighted_lines.is_empty() {
                    let lines: Vec<String> =
                        code.highlighted_lines.iter().map(u32::to_string).collect();
                    let _ = write!(out, " {{{}}}", lines.join(","));
                }
                out.push('\n');
                if !code.source.is_empty() {
                    let _ = writeln!(out, "{}", code.source);
                }
                out.push_str("```\n");
            }
            Block::Media(media) => {
                let _ = writeln!(out, "![{}]({})", media.alt, media.path);
            }
        }
        out.push('\n');
    }

    for option in &question.options {
        let mark = if option.is_correct { 'x' } else { ' ' };
        let _ = writeln!(out, "- [{mark}] {}", option.text);
        for (index, paragraph) in option.explanation.split('\n').enumerate() {
            if paragraph.is_empty() && index == 0 {
                break;
            }
            if index > 0 {
                out.push_str("  >\n");
            }
            let _ = writeln!(out, "  > {paragraph}");
        }
    }

    if !question.rule.is_empty() || !question.course_ref.is_empty() {
        out.push('\n');
    }
    if !question.rule.is_empty() {
        let _ = writeln!(out, "RULE: {}", question.rule);
    }
    if !question.course_ref.is_empty() {
        let _ = writeln!(out, "REVIEW: {}", question.course_ref);
    }
}

fn level_number(level: Level) -> Option<u32> {
    match level {
        Level::Unknown => None,
        Level::Remember => Some(1),
        Level::Understand => Some(2),
        Level::Apply => Some(3),
        Level::Analyse => Some(4),
        Level::Evaluate => Some(5),
    }
}
