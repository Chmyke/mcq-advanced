//! End-to-end runs of the `mcq` binary on fictional papers in a temporary folder.

use std::path::{Path, PathBuf};
use std::process::Command;

const SAMPLE: &str = include_str!("../../mcq-format/tests/fixtures/sample.qcm.md");

/// A fresh folder per test, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("mcq-cli-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("media")).expect("temp folder");
        Self(dir)
    }
    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, content).expect("write fixture");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(args: &[&Path]) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_mcq"))
        .arg("check")
        .args(args)
        .output()
        .expect("run mcq");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    (output.status.code().unwrap_or(-1), stdout)
}

#[test]
fn sample_with_its_media_passes() {
    let dir = TempDir::new("ok");
    dir.write("media/apple.png", "png");
    dir.write("media/crunch.ogg", "ogg");
    let paper = dir.write("sample.qcm.md", SAMPLE);
    let (code, out) = run(&[&paper]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("0 error(s)"), "{out}");
    assert!(out.contains("OK: 1 paper(s) checked"), "{out}");
}

#[test]
fn missing_media_fails() {
    let dir = TempDir::new("media");
    let paper = dir.write("sample.qcm.md", SAMPLE);
    let (code, out) = run(&[&paper]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("error[MEDIA_NOT_FOUND]"), "{out}");
}

#[test]
fn media_outside_the_folder_fails() {
    let dir = TempDir::new("escape");
    let source = SAMPLE.replace("media/apple.png", "../apple.png");
    dir.write("media/crunch.ogg", "ogg");
    let paper = dir.write("sample.qcm.md", &source);
    let (code, out) = run(&[&paper]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("must stay inside"), "{out}");
}

#[test]
fn sibling_duplicates_are_reported_and_copies_skipped() {
    let dir = TempDir::new("siblings");
    dir.write("media/apple.png", "png");
    dir.write("media/crunch.ogg", "ogg");
    let paper = dir.write("sample.qcm.md", SAMPLE);
    dir.write("copy.qcm.md", SAMPLE);
    dir.write("student copy.md", "# Sample\n\n### Question 1\n");
    let (code, out) = run(&[&paper]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("in copy.qcm.md"), "{out}");
    assert!(!out.contains("student copy.md"), "{out}");
}

#[test]
fn broken_paper_and_bad_usage() {
    let dir = TempDir::new("broken");
    let paper = dir.write("broken.qcm.md", "# THEME t\n");
    let (code, out) = run(&[&paper]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("error[NO_FRONTMATTER]"), "{out}");

    let (code, _) = run(&[]);
    assert_eq!(code, 2);
    let missing = dir.0.join("nope.qcm.md");
    let (code, out) = run(&[&missing]);
    assert_eq!(code, 1);
    assert!(out.contains("error[READ]"), "{out}");
}
