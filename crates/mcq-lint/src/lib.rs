//! ROLE:    the pedagogical linter: structural errors that block publishing, and writing advice.
//! OWNER:   pure functions over a `Paper`; no I/O.
//! INVARIANTS: issue codes are stable (documented in `docs/lint-rules.md`);
//!             word lists cover French and English, so no language setting is required.

mod duplicates;
mod text;

use mcq_model::{Block, Issue, Level, Location, Paper, Question, Severity};

pub use duplicates::{DUPLICATE_THRESHOLD, find_duplicates, find_duplicates_within, similarity};

/// Above this many options, reading costs more than choosing.
pub const MAX_RECOMMENDED_OPTIONS: usize = 4;
/// Longer papers measure fatigue more than knowledge.
pub const MAX_DURATION_MINUTES: u32 = 90;
/// Share of level-1 (recall) questions above which the paper mostly measures vocabulary.
pub const MAX_RECALL_SHARE: f32 = 0.15;
/// A theme with fewer questions gives a meaningless success rate.
pub const MIN_QUESTIONS_PER_THEME: usize = 3;
/// Correct answer length over the longest distractor's from which the length gives it away.
pub const LENGTH_BIAS_RATIO: f32 = 1.6;
/// Visible characters the correct answer must add over the longest distractor before the ratio
/// counts: "12 mol" against "1 mol" gives nothing away.
pub const LENGTH_BIAS_MIN_EXTRA_CHARS: usize = 8;
/// Paper-wide checks need this many questions to mean anything.
pub const MIN_QUESTIONS_FOR_PAPER_CHECKS: usize = 10;
/// Share of single-answer questions whose correct answer is strictly the longest option, above
/// which it is a pattern.
pub const MAX_LONGEST_SHARE: f32 = 0.35;
/// Share of correct answers at one position, above which proofreading reads a column.
pub const MAX_SAME_POSITION_SHARE: f32 = 0.6;

/// Lints a paper. Errors block publishing; warnings and infos never do.
pub fn lint(paper: &Paper) -> Vec<Issue> {
    let mut out = Out::default();
    if paper.title.trim().is_empty() {
        out.error(
            "NO_TITLE",
            Location::Paper,
            "The paper has no title: add `title:` to the header.",
            "Le sujet n'a pas de titre : ajoutez `title:` dans l'en-tête.",
        );
    }
    if paper.questions.is_empty() {
        out.error(
            "NO_QUESTION",
            Location::Paper,
            "The paper has no question.",
            "Le sujet ne contient aucune question.",
        );
        return out.issues;
    }

    let mut seen_ids: Vec<&str> = Vec::with_capacity(paper.questions.len());
    for question in &paper.questions {
        if !question.id.is_empty() {
            if seen_ids.contains(&question.id.as_str()) {
                out.error(
                    "DUPLICATE_QUESTION_ID",
                    at(question),
                    "This id is already used: ids are never reused, even after a deletion.",
                    "Cet identifiant est déjà utilisé : un identifiant ne se réutilise jamais, même \
                     après une suppression.",
                );
            }
            seen_ids.push(&question.id);
        }
        check_structure(paper, question, &mut out);
        check_options(question, &mut out);
        check_stem(question, &mut out);
        check_media(question, &mut out);
        let mut texts: Vec<&str> = question
            .body
            .iter()
            .filter_map(|b| match b {
                Block::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect();
        for option in &question.options {
            texts.push(&option.text);
            texts.push(&option.explanation);
        }
        check_formulas(&texts, at(question), &mut out);
    }
    check_formulas(&[paper.description.as_str()], Location::Paper, &mut out);

    check_duration(paper, &mut out);
    check_levels(paper, &mut out);
    check_themes(paper, &mut out);
    for question in &paper.questions {
        check_feedback(paper, question, &mut out);
        check_length_bias(question, &mut out);
    }
    check_length_rank(paper, &mut out);
    check_answer_position(paper, &mut out);
    out.issues
}

/// True when any issue blocks publishing.
pub fn has_errors(issues: &[Issue]) -> bool {
    issues.iter().any(|i| i.severity == Severity::Error)
}

#[derive(Default)]
struct Out {
    issues: Vec<Issue>,
}

impl Out {
    /// Every message comes in English (`en`) and French (`fr`).
    fn push(
        &mut self,
        severity: Severity,
        code: &'static str,
        at: Location,
        en: String,
        fr: String,
    ) {
        self.issues.push(Issue::new(severity, code, at, en, fr));
    }
    fn error(
        &mut self,
        code: &'static str,
        at: Location,
        en: impl Into<String>,
        fr: impl Into<String>,
    ) {
        self.push(Severity::Error, code, at, en.into(), fr.into());
    }
    fn warn(
        &mut self,
        code: &'static str,
        at: Location,
        en: impl Into<String>,
        fr: impl Into<String>,
    ) {
        self.push(Severity::Warning, code, at, en.into(), fr.into());
    }
    fn info(
        &mut self,
        code: &'static str,
        at: Location,
        en: impl Into<String>,
        fr: impl Into<String>,
    ) {
        self.push(Severity::Info, code, at, en.into(), fr.into());
    }
}

/// Where to point for a question: its id, else its source line.
fn at(question: &Question) -> Location {
    if !question.id.is_empty() {
        Location::Question(question.id.clone())
    } else if question.line > 0 {
        Location::Line(question.line)
    } else {
        Location::Paper
    }
}

fn check_structure(paper: &Paper, question: &Question, out: &mut Out) {
    if question.id.is_empty() {
        out.error(
            "QUESTION_WITHOUT_ID",
            at(question),
            "The question has no id: expected `## Q id | level 3 | 20s`.",
            "La question n'a pas d'identifiant : forme attendue `## Q id | level 3 | 20s`.",
        );
    }
    if question.statement_text().is_empty() {
        out.error(
            "EMPTY_STATEMENT",
            at(question),
            "The question has no statement.",
            "La question n'a pas d'énoncé.",
        );
    }
    if question.theme_id.is_empty() {
        out.error(
            "QUESTION_WITHOUT_THEME",
            at(question),
            "The question comes before any `# THEME`: declare a theme first.",
            "La question précède tout `# THEME` : déclarez d'abord un thème.",
        );
    } else if paper.theme(&question.theme_id).is_none() {
        out.error(
            "UNKNOWN_THEME",
            at(question),
            format!("Theme `{}` is not declared.", question.theme_id),
            format!("Le thème `{}` n'est pas déclaré.", question.theme_id),
        );
    }
}

fn check_options(question: &Question, out: &mut Out) {
    let count = question.options.len();
    for (index, option) in question.options.iter().enumerate() {
        if option.text.trim().is_empty() {
            out.error(
                "EMPTY_OPTION",
                at(question),
                format!("Option {} has no text.", letter(index)),
                format!("La réponse {} n'a pas de texte.", letter(index)),
            );
        }
    }
    if count < 2 {
        out.error(
            "TOO_FEW_OPTIONS",
            at(question),
            "A question needs at least 2 options.",
            "Une question demande au moins 2 réponses.",
        );
        return;
    }
    let correct = question.correct_count();
    if correct == 0 {
        out.error(
            "NO_CORRECT_OPTION",
            at(question),
            "No option is marked correct: mark it with `[x]`.",
            "Aucune réponse n'est marquée juste : marquez la bonne avec `[x]`.",
        );
    } else if correct == count {
        out.error(
            "ALL_OPTIONS_CORRECT",
            at(question),
            "Every option is correct: nothing is left to tell apart.",
            "Toutes les réponses sont justes : il ne reste rien à distinguer.",
        );
    } else if correct > 1 {
        out.info(
            "MULTIPLE_CORRECT_OPTIONS",
            at(question),
            format!("{correct} correct options: the student is told to tick several boxes."),
            format!("{correct} bonnes réponses : le candidat est invité à cocher plusieurs cases."),
        );
    }

    let declared = question.declared_answers as usize;
    if declared > 0 && correct > 0 && declared != correct {
        out.error(
            "ANSWER_COUNT_MISMATCH",
            at(question),
            format!(
                "`answers {declared}` is announced but {correct} {} marked correct.",
                en(correct, "option is", "options are")
            ),
            format!(
                "`answers {declared}` est annoncé mais {correct} {}.",
                fr(
                    correct,
                    "réponse est marquée juste",
                    "réponses sont marquées justes"
                )
            ),
        );
    }

    let mut seen: Vec<String> = Vec::with_capacity(count);
    for option in &question.options {
        let key = option.text.trim().to_lowercase();
        if key.is_empty() {
            continue;
        }
        if seen.contains(&key) {
            out.error(
                "DUPLICATE_OPTION",
                at(question),
                format!("Two options read `{}`.", option.text.trim()),
                format!("Deux réponses disent `{}`.", option.text.trim()),
            );
        } else {
            seen.push(key);
        }
    }

    if count > MAX_RECOMMENDED_OPTIONS {
        out.warn(
            "TOO_MANY_OPTIONS",
            at(question),
            format!(
                "{count} options: past {MAX_RECOMMENDED_OPTIONS}, reading costs more than choosing \
                 and the extra option is usually a distractor nobody picks."
            ),
            format!(
                "{count} réponses : au-delà de {MAX_RECOMMENDED_OPTIONS}, lire coûte plus que choisir \
                 et la réponse en trop est souvent un distracteur que personne ne choisit."
            ),
        );
    }
    for option in &question.options {
        if text::is_meta_option(&option.text) {
            out.warn(
                "META_OPTION",
                at(question),
                format!(
                    "`{}` can be guessed by partial elimination and hides which distractor misled.",
                    option.text.trim()
                ),
                format!(
                    "`{}` se devine par élimination partielle et cache quel distracteur a trompé.",
                    option.text.trim()
                ),
            );
        }
    }
}

fn check_stem(question: &Question, out: &mut Out) {
    let stem = question.statement_text();
    if stem.is_empty() {
        return;
    }
    let asked = text::interrogative_part(&stem);
    if let Some(word) = text::unemphasised_negation(&asked) {
        out.warn(
            "NEGATIVE_STEM",
            at(question),
            format!(
                "The question asks for an exception (`{word}`) without emphasis: write it in \
                 capitals or bold so nobody misreads it."
            ),
            format!(
                "La question demande une exception (`{word}`) sans la mettre en valeur : écrivez-la \
                 en capitales ou en gras pour que personne ne la lise de travers."
            ),
        );
    }
    // A true or false question judges a statement: it asks nothing by nature.
    if !question.is_true_false() && !stem.contains('?') && !stem.trim_end().ends_with(':') {
        out.warn(
            "MISSING_QUESTION",
            at(question),
            "The statement never asks anything: add the sentence that says what is asked.",
            "L'énoncé ne pose aucune question : ajoutez la phrase qui dit ce qui est demandé.",
        );
    }
    let compile_options = question
        .options
        .iter()
        .filter(|o| o.text.to_lowercase().contains("compil"))
        .count();
    if text::is_vague_stem(&asked)
        && question.options.len() >= 2
        && compile_options > 0
        && compile_options < question.options.len()
    {
        out.warn(
            "VAGUE_STEM",
            at(question),
            "Some options are about compiling, others about running: say which one is asked, \
             for example \"Does this compile, and what does it print?\".",
            "Certaines réponses portent sur la compilation, d'autres sur l'exécution : précisez ce \
             qui est demandé, par exemple « Ce code compile-t-il, et qu'affiche-t-il ? ».",
        );
    }
}

fn check_media(question: &Question, out: &mut Out) {
    for media in question.media() {
        if media.alt.trim().is_empty() {
            out.warn(
                "MEDIA_WITHOUT_ALT",
                at(question),
                format!(
                    "`{}` has no text alternative: describe it in `![...]` for students who \
                     cannot see or hear it.",
                    media.path
                ),
                format!(
                    "`{}` n'a pas de texte alternatif : décrivez-le dans `![...]` pour les candidats qui \
                     ne peuvent ni le voir ni l'entendre.",
                    media.path
                ),
            );
        }
    }
}

/// Formulas that would not show, and dollar signs that open none.
fn check_formulas(texts: &[&str], place: Location, out: &mut Out) {
    let stray: usize = texts.iter().map(|t| mcq_math::split(t).stray_dollars).sum();
    for (latex, display) in mcq_math::formulas_in(texts) {
        if let Err(why) = mcq_math::to_mathml(latex, display) {
            let shown = if display {
                format!("$${latex}$$")
            } else {
                format!("${latex}$")
            };
            out.error(
                "BAD_FORMULA",
                place.clone(),
                format!("`{shown}` cannot be shown: {}.", why.trim_end_matches('.')),
                format!(
                    "`{shown}` ne peut pas s'afficher : {}.",
                    why.trim_end_matches('.')
                ),
            );
        }
    }
    if stray > 0 {
        out.warn(
            "STRAY_DOLLAR",
            place,
            format!(
                "{stray} `$` {} no formula and will show as is: write `\\$` for a dollar \
                 sign, `$x$` for a formula.",
                en(stray, "opens", "open")
            ),
            format!(
                "`$` sans formule ({stray}) : il s'affiche tel quel. Écrivez `\\$` pour un signe \
                 dollar, `$x$` pour une formule."
            ),
        );
    }
}

fn check_duration(paper: &Paper, out: &mut Out) {
    // Rounded up, as the settings show it: 40 s is about 1 min, never 0.
    let minutes = paper.estimated_seconds_with_margin().div_ceil(60);
    out.info(
        "DURATION",
        Location::Paper,
        format!(
            "{} questions, about {minutes} min including a 20 % margin.",
            paper.questions.len()
        ),
        format!(
            "{} questions, environ {minutes} min, marge de 20 % comprise.",
            paper.questions.len()
        ),
    );
    if minutes > MAX_DURATION_MINUTES {
        out.warn(
            "TOO_LONG",
            Location::Paper,
            format!(
                "About {minutes} min: past {MAX_DURATION_MINUTES} min, remove questions rather \
                 than lengthen the session."
            ),
            format!(
                "Environ {minutes} min : au-delà de {MAX_DURATION_MINUTES} min, retirez des questions \
                 plutôt que d'allonger l'épreuve."
            ),
        );
    }
    if paper.duration_minutes > 0 && minutes > paper.duration_minutes {
        out.warn(
            "DURATION_MISMATCH",
            Location::Paper,
            format!(
                "The header announces {} min, the questions need about {minutes}.",
                paper.duration_minutes
            ),
            format!(
                "L'en-tête annonce {} min, les questions en demandent environ {minutes}.",
                paper.duration_minutes
            ),
        );
    }
}

fn check_levels(paper: &Paper, out: &mut Out) {
    let total = paper.questions.len();
    let count = |level: Level| paper.questions.iter().filter(|q| q.level == level).count();
    let unknown = count(Level::Unknown);
    if unknown > 0 {
        out.warn(
            "LEVEL_MISSING",
            Location::Paper,
            format!(
                "{unknown} {} no level: add `level 1` to `level 5`.",
                en(unknown, "question has", "questions have")
            ),
            format!(
                "{unknown} {} sans niveau : ajoutez de `level 1` à `level 5`.",
                fr(unknown, "question", "questions")
            ),
        );
    }
    let recall = count(Level::Remember);
    if recall as f32 / total as f32 > MAX_RECALL_SHARE {
        out.warn(
            "TOO_MUCH_RECALL",
            Location::Paper,
            format!(
                "{} % of the questions are recall (level 1): the paper mostly measures vocabulary.",
                recall * 100 / total
            ),
            format!(
                "{} % des questions sont de la restitution (niveau 1) : le sujet mesure surtout du \
                 vocabulaire.",
                recall * 100 / total
            ),
        );
    }
    let applied = count(Level::Apply) + count(Level::Analyse) + count(Level::Evaluate);
    if applied * 2 < total {
        out.warn(
            "NOT_ENOUGH_APPLY",
            Location::Paper,
            "Fewer than half the questions ask to apply, analyse or evaluate (levels 3 to 5).",
            "Moins de la moitié des questions demandent d'appliquer, d'analyser ou d'évaluer \
             (niveaux 3 à 5).",
        );
    }
}

fn check_themes(paper: &Paper, out: &mut Out) {
    for theme in &paper.themes {
        let count = paper
            .questions
            .iter()
            .filter(|q| q.theme_id == theme.id)
            .count();
        let location = Location::Theme(theme.id.clone());
        if count == 0 {
            out.warn(
                "EMPTY_THEME",
                location,
                "The theme is declared but has no question.",
                "Le thème est déclaré mais ne contient aucune question.",
            );
        } else if count < MIN_QUESTIONS_PER_THEME {
            out.warn(
                "THIN_THEME",
                location,
                format!(
                    "{count} {} only: below {MIN_QUESTIONS_PER_THEME}, the theme's success rate \
                     means little.",
                    en(count, "question", "questions")
                ),
                format!(
                    "{count} {} seulement : en dessous de {MIN_QUESTIONS_PER_THEME}, le taux de \
                     réussite du thème ne veut pas dire grand-chose.",
                    fr(count, "question", "questions")
                ),
            );
        }
    }
}

fn check_feedback(paper: &Paper, question: &Question, out: &mut Out) {
    let missing = question
        .options
        .iter()
        .filter(|o| o.explanation.trim().is_empty())
        .count();
    if !question.options.is_empty() && missing == question.options.len() {
        out.warn(
            "NO_EXPLANATION",
            at(question),
            "No option is explained: the student sees right or wrong without knowing why.",
            "Aucune réponse n'est expliquée : le candidat voit juste ou faux sans savoir pourquoi.",
        );
    } else if missing > 0 {
        out.warn(
            "PARTIAL_EXPLANATION",
            at(question),
            format!(
                "{missing} {}: a student who ticks {} learns nothing.",
                en(
                    missing,
                    "option has no explanation",
                    "options have no explanation"
                ),
                en(missing, "it", "them")
            ),
            format!(
                "{missing} {} sans explication : le candidat qui {} coche n'apprend rien.",
                fr(missing, "réponse", "réponses"),
                fr(missing, "la", "les")
            ),
        );
    }
    if question.rule.trim().is_empty() {
        out.warn(
            "NO_RULE",
            at(question),
            "No rule to remember: add `RULE:`.",
            "Aucune règle à retenir : ajoutez `RULE:`.",
        );
    }
    if question.course_ref.trim().is_empty()
        && let Some(theme) = paper.theme(&question.theme_id)
        && theme.course_ref.trim().is_empty()
    {
        out.warn(
            "NO_COURSE_REF",
            at(question),
            "Nowhere to revise: add `COURSE:` to the theme or `REVIEW:` to the question.",
            "Rien pour réviser : ajoutez `COURSE:` au thème ou `REVIEW:` à la question.",
        );
    }
}

fn visible_len(text: &str) -> usize {
    text.trim().chars().count()
}

/// The correct answer of a single-answer question; `None` for multiple-answer ones, where
/// "the correct answer" is not one option and length comparisons mean nothing.
fn single_correct(question: &Question) -> Option<&mcq_model::AnswerOption> {
    if question.options.len() < 2 || question.correct_count() != 1 {
        return None;
    }
    question.first_correct()
}

fn check_length_bias(question: &Question, out: &mut Out) {
    let Some(correct) = single_correct(question) else {
        return;
    };
    let longest_distractor = question
        .options
        .iter()
        .filter(|o| !o.is_correct)
        .map(|o| visible_len(&o.text))
        .max()
        .unwrap_or(0);
    if longest_distractor == 0 {
        return;
    }
    let len = visible_len(&correct.text);
    let ratio = len as f32 / longest_distractor as f32;
    if ratio >= LENGTH_BIAS_RATIO && len >= longest_distractor + LENGTH_BIAS_MIN_EXTRA_CHARS {
        out.warn(
            "LENGTH_BIAS",
            at(question),
            format!(
                "The correct answer is {ratio:.1} times longer than the longest of the others: \
                 students often spot the longest one. Lengthen the other answers or shorten the \
                 correct one; a justification belongs in the explanation."
            ),
            format!(
                "La bonne réponse est {} fois plus longue que la plus longue des autres : les \
                 candidats repèrent souvent la plus longue. Rallongez les autres réponses ou \
                 raccourcissez la bonne ; une justification a sa place dans l'explication.",
                format!("{ratio:.1}").replace('.', ",")
            ),
        );
    }
}

fn check_length_rank(paper: &Paper, out: &mut Out) {
    if paper.questions.len() < MIN_QUESTIONS_FOR_PAPER_CHECKS {
        return;
    }
    let mut counted = 0_usize;
    let mut longest = 0_usize;
    // Expected count if the correct answer sat on a random option: a question whose longest
    // length is tied has no strictly longest option, so it adds nothing.
    let mut by_chance = 0.0_f32;
    for question in &paper.questions {
        let Some(correct) = single_correct(question) else {
            continue;
        };
        let len = visible_len(&correct.text);
        if len == 0 {
            continue;
        }
        counted += 1;
        let lengths: Vec<usize> = question
            .options
            .iter()
            .map(|o| visible_len(&o.text))
            .collect();
        let max = lengths.iter().copied().max().unwrap_or(0);
        if lengths.iter().filter(|&&l| l == max).count() == 1 {
            by_chance += 1.0 / lengths.len() as f32;
            if len == max {
                longest += 1;
            }
        }
    }
    if counted < MIN_QUESTIONS_FOR_PAPER_CHECKS {
        return;
    }
    let share = longest as f32 / counted as f32;
    if share > MAX_LONGEST_SHARE {
        let percent = (share * 100.0 + 0.5) as u32;
        let chance = (by_chance * 100.0 / counted as f32 + 0.5) as u32;
        out.warn(
            "LENGTH_RANK_BIAS",
            Location::Paper,
            format!(
                "The correct answer is strictly the longest option in {percent} % of the \
                 single-answer questions, against about {chance} % by chance: students often \
                 spot the longest one. Lengthen some wrong answers or shorten some correct ones."
            ),
            format!(
                "La bonne réponse est strictement la plus longue dans {percent} % des questions à \
                 réponse unique, contre environ {chance} % par hasard : les candidats repèrent \
                 souvent la plus longue. Rallongez des mauvaises réponses ou raccourcissez des \
                 bonnes."
            ),
        );
    }
}

fn check_answer_position(paper: &Paper, out: &mut Out) {
    if paper.questions.len() < MIN_QUESTIONS_FOR_PAPER_CHECKS {
        return;
    }
    let mut counts: Vec<usize> = Vec::new();
    let mut counted = 0_usize;
    for question in &paper.questions {
        let Some(index) = question.options.iter().position(|o| o.is_correct) else {
            continue;
        };
        if counts.len() <= index {
            counts.resize(index + 1, 0);
        }
        counts[index] += 1;
        counted += 1;
    }
    if counted == 0 {
        return;
    }
    // First maximum wins, so the report is deterministic.
    let (index, &top) = counts
        .iter()
        .enumerate()
        .fold((0, &0), |best, cur| if cur.1 > best.1 { cur } else { best });
    let share = top as f32 / counted as f32;
    if share > MAX_SAME_POSITION_SHARE {
        let percent = (share * 100.0) as u32;
        let place = letter(index);
        let (english, french) = if paper.shuffle_options {
            (
                format!(
                    "The correct answer is option {place} in {percent} % of the questions as you wrote \
                     them. Options are shuffled for students, but varying where the correct \
                     answer sits keeps it from being guessed on a printed or unshuffled paper."
                ),
                format!(
                    "La bonne réponse est en {place} dans {percent} % des questions telles que \
                     vous les avez écrites. Les réponses sont mélangées pour les candidats, mais \
                     varier la place de la bonne réponse évite qu'elle se devine sur un sujet \
                     imprimé ou non mélangé."
                ),
            )
        } else {
            (
                format!(
                    "The correct answer is option {place} in {percent} % of the questions, and options \
                     are not shuffled: students can guess it from where it sits. Vary its place, \
                     or turn on option shuffling in the settings."
                ),
                format!(
                    "La bonne réponse est en {place} dans {percent} % des questions, et les \
                     réponses ne sont pas mélangées : les candidats peuvent la deviner à sa place. \
                     Variez la place de la bonne réponse, ou activez le mélange des réponses dans \
                     les réglages."
                ),
            )
        };
        out.warn("ANSWER_POSITION_BIAS", Location::Paper, english, french);
    }
}

/// A, B, C... for an option index.
fn letter(index: usize) -> char {
    u8::try_from(index)
        .ok()
        .filter(|i| *i < 26)
        .map_or('?', |i| char::from(b'A' + i))
}

/// English singular or plural: only 1 takes the singular.
fn en(count: usize, one: &'static str, many: &'static str) -> &'static str {
    if count == 1 { one } else { many }
}

/// French singular or plural: 0 and 1 take the singular.
fn fr(count: usize, one: &'static str, many: &'static str) -> &'static str {
    if count <= 1 { one } else { many }
}

#[cfg(test)]
mod tests {
    use super::{en, fr};

    #[test]
    fn plurals_follow_each_language() {
        assert_eq!(en(0, "question", "questions"), "questions");
        assert_eq!(en(1, "question", "questions"), "question");
        assert_eq!(en(2, "question", "questions"), "questions");
        assert_eq!(fr(0, "question", "questions"), "question");
        assert_eq!(fr(1, "question", "questions"), "question");
        assert_eq!(fr(2, "question", "questions"), "questions");
    }
}
