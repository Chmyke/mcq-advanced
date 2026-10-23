//! `mcq check`: format, pedagogical rules, media files and near-duplicate questions.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use mcq_model::{Issue, Location, Paper, Severity};

use crate::{USAGE, read_text};

pub fn check(args: &[String]) -> ExitCode {
    let quiet = args.iter().any(|a| a == "--quiet");
    let files: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    if files.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let mut failed = 0_usize;
    for file in &files {
        let path = Path::new(file.as_str());
        let issues = match check_file(path) {
            Ok(issues) => issues,
            Err(message) => {
                println!("{}: error[READ] {message}", path.display());
                failed += 1;
                continue;
            }
        };
        let (errors, warnings) = count(&issues);
        for issue in &issues {
            if quiet && issue.severity == Severity::Info {
                continue;
            }
            println!("{}", format_issue(path, issue));
        }
        println!(
            "{}: {errors} error(s), {warnings} warning(s)",
            path.display()
        );
        if errors > 0 {
            failed += 1;
        }
    }
    if failed > 0 {
        println!(
            "FAIL: {failed} of {} paper(s) cannot be published",
            files.len()
        );
        ExitCode::from(1)
    } else {
        println!("OK: {} paper(s) checked", files.len());
        ExitCode::SUCCESS
    }
}

/// Every issue of one paper: reader, linter, media files, duplicates.
pub(crate) fn check_file(path: &Path) -> Result<Vec<Issue>, String> {
    let source = read_text(path)?;
    let (paper, mut issues) = mcq_format::read(&source);
    issues.extend(mcq_lint::lint(&paper));
    issues.extend(missing_media(&paper, path));
    issues.extend(mcq_lint::find_duplicates_within(&paper));
    for sibling in siblings(path) {
        // An unreadable or empty sibling (such as a student copy) is simply skipped.
        let Ok(text) = read_text(&sibling) else {
            continue;
        };
        let (other, _) = mcq_format::read(&text);
        if other.questions.is_empty() {
            continue;
        }
        let name = sibling
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        issues.extend(mcq_lint::find_duplicates(&paper, &other, &name));
    }
    Ok(issues)
}

/// Media referenced by the paper but absent next to it.
pub(crate) fn missing_media(paper: &Paper, path: &Path) -> Vec<Issue> {
    let folder = path.parent().unwrap_or(Path::new("."));
    let mut issues = Vec::new();
    for question in &paper.questions {
        for media in question.media() {
            let relative = Path::new(&media.path);
            let escapes = relative.is_absolute()
                || relative
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir));
            let (en, fr) = if escapes {
                (
                    format!(
                        "`{}` must stay inside the paper folder (a relative path, no `..`).",
                        media.path
                    ),
                    format!(
                        "`{}` doit rester dans le dossier du sujet (un chemin relatif, sans `..`).",
                        media.path
                    ),
                )
            } else if !folder.join(relative).is_file() {
                (
                    format!("`{}` is not found next to the paper.", media.path),
                    format!("`{}` est introuvable à côté du sujet.", media.path),
                )
            } else {
                continue;
            };
            issues.push(Issue::new(
                Severity::Error,
                "MEDIA_NOT_FOUND",
                Location::Question(question.id.clone()),
                en,
                fr,
            ));
        }
    }
    issues
}

/// Other Markdown papers of the same folder, in a stable order.
fn siblings(path: &Path) -> Vec<PathBuf> {
    let folder = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let me = std::fs::canonicalize(path).ok();
    let mut out: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")))
        .filter(|p| std::fs::canonicalize(p).ok() != me)
        .collect();
    out.sort();
    out
}

fn count(issues: &[Issue]) -> (usize, usize) {
    let errors = issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    let warnings = issues
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .count();
    (errors, warnings)
}

pub(crate) fn format_issue(path: &Path, issue: &Issue) -> String {
    format!(
        "{}: {}[{}] {}: {}",
        path.display(),
        issue.severity.label(),
        issue.code,
        issue.location,
        issue.message
    )
}
