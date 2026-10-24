//! The Studio's state and every change the interface can make to it, without any screen: the
//! components only display it and call these methods, so tests drive the editor directly.

use std::collections::{BTreeMap, BTreeSet};

use mcq_edit::Project;
use mcq_edit::ops::{self, Kind};
use mcq_model::{Block, CodeBlock, Issue, Location, Media, MediaKind, Paper, Question, Severity};
use mcq_player::b64;
use mcq_player::render::mime;

/// What the Formula button inserts: a formula on its own line, to replace.
pub const FORMULA_EXAMPLE: &str = r"$$\frac{a}{b}$$";

/// The spaces of the Studio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    Write,
    Settings,
    Publish,
    Correct,
}

pub struct Studio {
    pub project: Project,
    pub space: Space,
    /// The question open in the editor.
    pub selected: Option<String>,
    /// What the reader reported on the file just opened; empty after an edit.
    read_issues: Vec<Issue>,
    /// Questions shown as multiple-answer although fewer than two options are ticked yet.
    multiple: BTreeSet<String>,
    /// Media as `data:` URLs for the previews, rebuilt only when files change.
    media_urls: BTreeMap<String, String>,
    /// Grows with every change: the autosave watches it.
    pub revision: u64,
    /// The question being dragged in the list, and the theme it is over.
    pub drag: Option<Drag>,
}

/// A question dragged from the list towards another place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drag {
    pub question: String,
    pub over: Option<String>,
}

impl Studio {
    pub fn new(project: Project) -> Self {
        Self::with_issues(project, Vec::new())
    }

    /// A project just read from a file, with what the reader reported.
    pub fn with_issues(project: Project, read_issues: Vec<Issue>) -> Self {
        let mut studio = Self {
            selected: project.paper.questions.first().map(|q| q.id.clone()),
            project,
            space: Space::Write,
            read_issues,
            multiple: BTreeSet::new(),
            media_urls: BTreeMap::new(),
            revision: 0,
            drag: None,
        };
        studio.refresh_media_urls();
        studio
    }

    fn touch(&mut self) {
        self.revision += 1;
    }

    /// Opens another paper in place; the revision keeps growing so the autosave sees it.
    pub fn replace(&mut self, project: Project, read_issues: Vec<Issue>) {
        let revision = self.revision + 1;
        let space = self.space;
        *self = Self::with_issues(project, read_issues);
        self.revision = revision;
        self.space = space;
    }

    /// Nothing written yet: replacing it loses nothing.
    pub fn is_blank(&self) -> bool {
        self.project.media.is_empty()
            && self
                .project
                .paper
                .questions
                .iter()
                .all(|q| q.body.is_empty() && q.options.iter().all(|o| o.text.is_empty()))
    }

    /// Reader findings first, then the linter's and the media checks.
    pub fn issues(&self) -> Vec<Issue> {
        let mut issues = self.read_issues.clone();
        issues.extend(self.project.issues());
        issues
    }

    pub fn select(&mut self, id: &str) {
        self.selected = Some(id.to_string());
    }

    pub fn selected_question(&self) -> Option<&Question> {
        self.project.paper.question(self.selected.as_deref()?)
    }

    /// Any visual change to the paper goes through here.
    pub fn edit_paper(&mut self, change: impl FnOnce(&mut Paper)) {
        change(&mut self.project.paper);
        self.read_issues.clear();
        self.touch();
    }

    pub fn edit_question(&mut self, id: &str, change: impl FnOnce(&mut Question)) {
        self.edit_paper(|paper| {
            if let Some(q) = paper.questions.iter_mut().find(|q| q.id == id) {
                change(q);
            }
        });
    }

    pub fn add_question(&mut self, theme_id: &str) {
        let mut added = None;
        self.edit_paper(|paper| added = Some(ops::add_question(paper, theme_id)));
        if added.is_some() {
            self.selected = added;
        }
    }

    pub fn start_drag(&mut self, id: &str) {
        self.drag = Some(Drag {
            question: id.to_string(),
            over: None,
        });
    }

    /// Only redraws when the theme under the pointer changes.
    pub fn drag_over(&mut self, theme_id: &str) {
        if let Some(drag) = &mut self.drag
            && drag.over.as_deref() != Some(theme_id)
        {
            drag.over = Some(theme_id.to_string());
        }
    }

    pub fn end_drag(&mut self) {
        self.drag = None;
    }

    /// Drops the dragged question into a theme, before `before` or at its end, and keeps it open.
    pub fn drop_question(&mut self, theme_id: &str, before: Option<&str>) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        self.edit_paper(|paper| {
            ops::move_question_to(paper, &drag.question, theme_id, before);
        });
        self.selected = Some(drag.question);
    }

    pub fn duplicate_question(&mut self, id: &str) {
        let mut copy = None;
        self.edit_paper(|paper| copy = ops::duplicate_question(paper, id));
        if copy.is_some() {
            self.selected = copy;
        }
    }

    /// Removes a question and selects its neighbour.
    pub fn remove_question(&mut self, id: &str) {
        let index = self.project.paper.questions.iter().position(|q| q.id == id);
        self.edit_paper(|paper| {
            ops::remove_question(paper, id);
        });
        let questions = &self.project.paper.questions;
        self.selected = index
            .and_then(|i| questions.get(i).or_else(|| questions.last()))
            .map(|q| q.id.clone());
        self.forget_unused_media();
    }

    pub fn rename_question(&mut self, id: &str, new_id: &str) -> Result<(), ops::IdError> {
        ops::rename_question(&mut self.project.paper, id, new_id)?;
        if self.multiple.remove(id) {
            self.multiple.insert(new_id.to_string());
        }
        if self.selected.as_deref() == Some(id) {
            self.selected = Some(new_id.to_string());
        }
        self.read_issues.clear();
        self.touch();
        Ok(())
    }

    /// The type shown in the editor: multiple once chosen, even before a second tick.
    pub fn kind(&self, question: &Question) -> Kind {
        match ops::kind(question) {
            Kind::Single if self.multiple.contains(&question.id) => Kind::Multiple,
            kind => kind,
        }
    }

    pub fn set_kind(&mut self, id: &str, kind: Kind) {
        let lang = self.project.paper.lang.clone();
        self.edit_question(id, |q| ops::set_kind(q, kind, &lang));
        if kind == Kind::Multiple {
            self.multiple.insert(id.to_string());
        } else {
            self.multiple.remove(id);
        }
    }

    pub fn toggle_correct(&mut self, id: &str, index: usize) {
        let single = self
            .project
            .paper
            .question(id)
            .is_some_and(|q| self.kind(q) != Kind::Multiple);
        self.edit_question(id, |q| ops::toggle_correct(q, index, single));
    }

    pub fn add_text_block(&mut self, id: &str) {
        self.edit_question(id, |q| q.body.push(Block::Text(String::new())));
    }

    /// A text block holding an example formula on its own line, ready to be replaced.
    pub fn add_formula_block(&mut self, id: &str) {
        self.edit_question(id, |q| {
            q.body.push(Block::Text(FORMULA_EXAMPLE.to_string()));
        });
    }

    pub fn add_code_block(&mut self, id: &str, language: &str) {
        let language = language.to_string();
        self.edit_question(id, |q| {
            q.body.push(Block::Code(CodeBlock {
                language,
                ..CodeBlock::default()
            }));
        });
    }

    pub fn remove_block(&mut self, id: &str, index: usize) {
        self.edit_question(id, |q| {
            if index < q.body.len() {
                q.body.remove(index);
            }
        });
        self.forget_unused_media();
    }

    /// Stores a dropped, pasted, chosen or recorded file and shows it in the question.
    /// `None` when the file is not an image, a sound or a video.
    pub fn add_media(&mut self, id: &str, file_name: &str, data: Vec<u8>) -> Option<String> {
        self.project.paper.question(id)?;
        let path = self.project.add_media(file_name, data)?;
        let kind = MediaKind::from_path(&path)?;
        let block_path = path.clone();
        self.edit_question(id, |q| {
            q.body.push(Block::Media(Media {
                kind,
                path: block_path,
                alt: String::new(),
            }));
        });
        self.refresh_media_urls();
        Some(path)
    }

    fn forget_unused_media(&mut self) {
        let before = self.project.media.len();
        self.project.prune_media();
        if self.project.media.len() != before {
            self.refresh_media_urls();
        }
    }

    fn refresh_media_urls(&mut self) {
        let media = &self.project.media;
        self.media_urls.retain(|path, _| media.contains_key(path));
        for (path, data) in media {
            self.media_urls
                .entry(path.clone())
                .or_insert_with(|| format!("data:{};base64,{}", mime(path), b64::encode(data)));
        }
    }

    pub fn media_urls(&self) -> &BTreeMap<String, String> {
        &self.media_urls
    }
}

/// The issues about one question: by id, or by a source line inside it.
pub fn issues_for<'a>(paper: &Paper, issues: &'a [Issue], question_id: &str) -> Vec<&'a Issue> {
    issues
        .iter()
        .filter(|issue| match &issue.location {
            Location::Question(id) => id == question_id,
            Location::Line(line) => question_at_line(paper, *line) == Some(question_id),
            _ => false,
        })
        .collect()
}

/// The issues about the paper as a whole, its themes, or lines outside any question.
pub fn paper_issues<'a>(paper: &Paper, issues: &'a [Issue]) -> Vec<&'a Issue> {
    issues
        .iter()
        .filter(|issue| match &issue.location {
            Location::Question(id) => paper.question(id).is_none(),
            Location::Line(line) => question_at_line(paper, *line).is_none(),
            _ => true,
        })
        .collect()
}

/// The most serious finding of a question, informations aside.
pub fn worst(issues: &[&Issue]) -> Option<Severity> {
    issues
        .iter()
        .map(|i| i.severity)
        .filter(|s| *s != Severity::Info)
        .min()
}

/// The question whose header is the last one at or above a source line.
fn question_at_line(paper: &Paper, line: usize) -> Option<&str> {
    paper
        .questions
        .iter()
        .filter(|q| q.line > 0 && q.line <= line)
        .max_by_key(|q| q.line)
        .map(|q| q.id.as_str())
}

/// The first words of a statement, for the question list.
pub fn summary(question: &Question, max_chars: usize) -> String {
    let text = question.statement_text();
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max_chars {
        flat
    } else {
        let cut: String = flat.chars().take(max_chars).collect();
        format!("{}…", cut.trim_end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("../../../crates/mcq-format/tests/fixtures/sample.qcm.md");

    fn studio() -> Studio {
        let mut project = Project::default();
        let issues = project.set_source(SAMPLE);
        Studio::with_issues(project, issues)
    }

    #[test]
    fn a_visual_edit_reaches_the_exported_source() {
        let mut s = studio();
        let id = s.selected.clone().expect("selection");
        s.edit_question(&id, |q| q.rule = "Read twice.".into());
        let source = s.project.source();
        assert!(source.contains("RULE: Read twice."), "{source}");
    }

    #[test]
    fn multiple_stays_chosen_before_the_second_tick() {
        let mut s = studio();
        let id = s.project.paper.questions[0].id.clone();
        s.set_kind(&id, Kind::Multiple);
        let q = s.project.paper.question(&id).expect("question").clone();
        assert_eq!(s.kind(&q), Kind::Multiple);
        s.toggle_correct(&id, 1);
        assert_eq!(
            s.project.paper.question(&id).map(Question::correct_count),
            Some(2)
        );
        s.set_kind(&id, Kind::Single);
        assert_eq!(
            s.project.paper.question(&id).map(Question::correct_count),
            Some(1)
        );
        s.toggle_correct(&id, 2);
        assert_eq!(
            s.project.paper.question(&id).map(Question::correct_count),
            Some(1)
        );
    }

    #[test]
    fn media_join_the_question_and_leave_with_it() {
        let mut s = studio();
        let id = s.project.paper.questions[0].id.clone();
        let blocks = s.project.paper.questions[0].body.len();
        let path = s
            .add_media(&id, "Pasted image.PNG", vec![1, 2, 3])
            .expect("media");
        assert_eq!(path, "media/pasted-image.png");
        assert_eq!(s.project.paper.questions[0].body.len(), blocks + 1);
        assert!(s.media_urls()[&path].starts_with("data:image/png;base64,"));
        assert!(s.issues().iter().any(|i| i.code == "MEDIA_WITHOUT_ALT"));
        assert_eq!(s.add_media(&id, "notes.txt", vec![1]), None);

        s.remove_block(&id, blocks);
        assert!(!s.project.media.contains_key(&path));
        assert!(!s.media_urls().contains_key(&path));
    }

    #[test]
    fn removing_selects_the_neighbour_and_renaming_follows() {
        let mut s = studio();
        let first = s.project.paper.questions[0].id.clone();
        let second = s.project.paper.questions[1].id.clone();
        s.select(&first);
        s.remove_question(&first);
        assert_eq!(s.selected.as_deref(), Some(second.as_str()));
        assert!(s.rename_question(&second, "renamed").is_ok());
        assert_eq!(s.selected.as_deref(), Some("renamed"));
        assert!(s.rename_question("renamed", "has space").is_err());
    }

    #[test]
    fn issues_land_on_their_question() {
        let mut s = studio();
        let id = s.project.paper.questions[0].id.clone();
        s.edit_question(&id, |q| {
            q.options.iter_mut().for_each(|o| o.is_correct = false)
        });
        let issues = s.issues();
        let mine = issues_for(&s.project.paper, &issues, &id);
        assert!(mine.iter().any(|i| i.code == "NO_CORRECT_OPTION"));
        assert_eq!(worst(&mine), Some(Severity::Error));
        assert!(
            paper_issues(&s.project.paper, &issues)
                .iter()
                .all(|i| i.code != "NO_CORRECT_OPTION")
        );
    }

    #[test]
    fn summaries_are_short() {
        let q = Question {
            body: vec![Block::Text("Which   of these\nis a fruit?".into())],
            ..Question::default()
        };
        assert_eq!(summary(&q, 100), "Which of these is a fruit?");
        assert_eq!(summary(&q, 8), "Which of…");
    }
}
