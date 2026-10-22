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

/// Runs `mcq` with `input` on standard input.
fn mcq(args: &[&str], input: &str) -> (i32, String, String) {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_mcq"))
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("run mcq");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait mcq");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn line_value<'a>(out: &'a str, label: &str) -> &'a str {
    out.lines()
        .find_map(|l| l.strip_prefix(label))
        .unwrap_or_else(|| panic!("no `{label}` in:\n{out}"))
        .trim()
}

#[test]
fn identity_publish_inspect_and_answers() {
    let dir = TempDir::new("chain");
    dir.write("media/apple.png", "png");
    dir.write("media/crunch.ogg", "ogg");
    let paper = dir.write("sample.qcm.md", SAMPLE);
    let id = dir.0.join("teacher.mcqid");
    let pack = dir.0.join("sample.mcqpack");
    let (id, pack, paper) = (
        id.to_str().expect("utf8"),
        pack.to_str().expect("utf8"),
        paper.to_str().expect("utf8"),
    );
    let passphrase = "a long enough passphrase\n";

    let (code, out, err) = mcq(&["identity", "new", id], "short\n");
    assert_eq!(code, 1, "{out}{err}");
    let (code, out, err) = mcq(&["identity", "new", id], passphrase);
    assert_eq!(code, 0, "{out}{err}");
    let fingerprint = line_value(&out, "Fingerprint:").to_string();
    let (code, out, _) = mcq(&["identity", "show", id], "");
    assert_eq!(
        (code, line_value(&out, "Fingerprint:")),
        (0, fingerprint.as_str())
    );
    let (code, _, err) = mcq(&["identity", "new", id], passphrase);
    assert_eq!(code, 1, "never overwrite an identity: {err}");

    let (code, out, err) = mcq(
        &[
            "publish",
            paper,
            "--identity",
            id,
            "--out",
            pack,
            "--end-code",
        ],
        "wrong passphrase\n",
    );
    assert_eq!(code, 1, "{out}{err}");
    let (code, out, err) = mcq(
        &[
            "publish",
            paper,
            "--identity",
            id,
            "--out",
            pack,
            "--end-code",
        ],
        passphrase,
    );
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(line_value(&out, "Teacher fingerprint:"), fingerprint);
    assert_eq!(line_value(&out, "Opening code:").len(), 14);
    let end_code = line_value(&out, "End code:").to_string();

    let (code, out, _) = mcq(&["inspect", pack], "");
    assert_eq!(code, 0, "{out}");
    assert_eq!(line_value(&out, "Teacher fingerprint:"), fingerprint);
    assert_eq!(line_value(&out, "Opening:"), "code");

    let (code, out, err) = mcq(&["answers", pack, "--identity", id], passphrase);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("- [x] Yellow"), "{out}");
    let (code, by_end_code, _) = mcq(&["answers", pack, "--end-code", &end_code], "");
    assert_eq!((code, by_end_code), (0, out));
    let (code, _, _) = mcq(&["answers", pack, "--practice"], "");
    assert_eq!(code, 1);

    let mut bytes = std::fs::read(pack).expect("read package");
    let middle = bytes.len() / 2;
    bytes[middle] ^= 1;
    std::fs::write(pack, bytes).expect("write package");
    let (code, _, err) = mcq(&["inspect", pack], "");
    assert_eq!(code, 1);
    assert!(err.contains("signature"), "{err}");
}

/// Runs a command without input, standard output and error together.
fn exchange(args: &[&str]) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_mcq"))
        .args(args)
        .output()
        .expect("run mcq");
    let text = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);
    (output.status.code().unwrap_or(-1), text)
}

fn arg(path: &Path) -> &str {
    path.to_str().expect("UTF-8 temp path")
}

#[test]
fn moodle_export_and_import_close_the_loop() {
    let dir = TempDir::new("moodle");
    dir.write("media/apple.png", "png");
    dir.write("media/crunch.ogg", "ogg");
    let paper = dir.write("sample.qcm.md", SAMPLE);
    let bank = dir.0.join("bank.xml");
    let (code, out) = exchange(&["export", arg(&paper), "--to", "moodle", "--out", arg(&bank)]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("info[EXPORT_SETTINGS]"), "{out}");
    // Never overwritten.
    let (code, _) = exchange(&["export", arg(&paper), "--out", arg(&bank)]);
    assert_eq!(code, 1);

    let back_dir = TempDir::new("moodle-back");
    let back = back_dir.0.join("back.qcm.md");
    let (code, out) = exchange(&["import", arg(&bank), "--out", arg(&back)]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        std::fs::read(back_dir.0.join("media/apple.png")).ok(),
        Some(b"png".to_vec())
    );
    let (code, out) = run(&[&back]);
    assert_eq!(code, 0, "{out}");

    let qcmz = back_dir.0.join("back.qcmz");
    let (code, out) = exchange(&["import", arg(&bank), "--out", arg(&qcmz)]);
    assert_eq!(code, 0, "{out}");
    assert!(qcmz.is_file());
}

#[test]
fn import_reports_what_it_cannot_read() {
    let dir = TempDir::new("import");
    let bank = dir.write("bank.txt", "no answers here\n");
    let out_path = dir.0.join("out.qcm.md");
    let (code, out) = exchange(&["import", arg(&bank), "--out", arg(&out_path)]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("--from"), "{out}");
    let (code, out) = exchange(&[
        "import",
        arg(&bank),
        "--from",
        "aiken",
        "--out",
        arg(&out_path),
    ]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("IMPORT_"), "{out}");
    assert!(!out_path.exists());
}

#[test]
fn handout_is_written_checked_and_protected() {
    let dir = TempDir::new("handout");
    let paper = dir.write("sample.qcm.md", SAMPLE);
    let handout = dir.write("handout.md", "my own notes\r\n");
    let (code, out) = exchange(&["handout", arg(&paper), "--out", arg(&handout)]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("--force"), "{out}");
    let (code, _) = exchange(&["handout", arg(&paper), "--out", arg(&handout), "--check"]);
    assert_eq!(code, 1);
    let (code, out) = exchange(&["handout", arg(&paper), "--out", arg(&handout), "--force"]);
    assert_eq!(code, 0, "{out}");
    let text = std::fs::read_to_string(&handout).expect("handout");
    assert!(
        text.contains("- **A.** Yellow\r\n"),
        "Windows line ends are kept"
    );
    let (code, out) = exchange(&["handout", arg(&paper), "--out", arg(&handout), "--check"]);
    assert_eq!(code, 0, "{out}");
    // A handout it wrote itself is replaced without --force.
    let (code, out) = exchange(&["handout", arg(&paper), "--out", arg(&handout)]);
    assert_eq!(code, 0, "{out}");
}
