//! ROLE:    the `mcq` command line: checks, publishes, opens, imports and exports papers without
//!          any window.
//! OWNER:   owns file access, the system RNG and the terminal; the logic lives in the library crates.
//!
//! Exit code: 0 on success, 1 on failure, 2 on bad usage.

mod check;
mod exchange;
mod secure;

use std::path::Path;
use std::process::ExitCode;

pub(crate) const USAGE: &str = "\
usage:
  mcq check [--quiet] <paper.qcm.md>...
      Checks each paper: format, pedagogical rules, media files and near-duplicate questions
      (inside the paper and against the other papers of its folder).
      --quiet  errors and warnings only, without infos
  mcq identity new <teacher.mcqid>
      Creates a teacher identity locked by a passphrase read from the first line of input.
  mcq identity show <teacher.mcqid>
      Prints the fingerprint students compare with the one on the room sheet.
  mcq publish <paper.qcm.md> --identity <teacher.mcqid> --out <paper.mcqpack> [--end-code] [--practice]
      Builds the signed, encrypted student package and prints its opening code.
      --end-code  also print an end code that lets students read the correction later
      --practice  no opening code: students see the answers right away
      The identity passphrase is read from the first line of input.
  mcq inspect <paper.mcqpack>
      Checks the signature and prints the teacher fingerprint and the opening mode.
  mcq answers <paper.mcqpack> (--identity <teacher.mcqid> | --end-code <code> | --practice)
      Prints the full paper, answers included.
  mcq import <bank> --out <paper.qcm.md | paper.qcmz> [--from gift|moodle|aiken|csv] [--lang <code>]
      Imports a GIFT, Moodle XML, Aiken or CSV question bank. The format follows the extension
      (.gift, .xml, .csv; a .txt file is recognised from its content). Media go to media/ next
      to a .qcm.md output. --lang names the true and false options (en, fr...).
  mcq export <paper.qcm.md> --to moodle --out <bank.xml>
      Writes a Moodle XML question bank, media embedded, for Moodle's question bank import.
  mcq handout <paper.qcm.md> --out <handout.md> [--check] [--force]
      Writes the paper for students in Markdown, without answers, to print or read on screen.
      --check  only tells whether the handout matches the paper (exit 1 when it does not)
      --force  replaces a file that mcq handout did not write";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest = args.get(1..).unwrap_or_default();
    match args.first().map(String::as_str) {
        Some("check") => check::check(rest),
        Some("identity") => secure::identity(rest),
        Some("publish") => secure::publish(rest),
        Some("inspect") => secure::inspect(rest),
        Some("answers") => secure::answers(rest),
        Some("import") => exchange::import(rest),
        Some("export") => exchange::export(rest),
        Some("handout") => exchange::handout_command(rest),
        Some("--help" | "-h" | "help") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        _ => usage(),
    }
}

pub(crate) fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

pub(crate) fn read_text(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read the file: {e}"))?;
    String::from_utf8(bytes).map_err(|_| "the file is not UTF-8 text".to_string())
}

/// Flags and options of one command, the rest being positional arguments.
pub(crate) struct Args<'a> {
    pub(crate) positional: Vec<&'a str>,
    flags: Vec<&'a str>,
    options: Vec<(&'a str, &'a str)>,
}

impl<'a> Args<'a> {
    /// `valued` lists the options that take a value.
    pub(crate) fn parse(args: &'a [String], valued: &[&str]) -> Option<Self> {
        let mut parsed = Args {
            positional: Vec::new(),
            flags: Vec::new(),
            options: Vec::new(),
        };
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if valued.contains(&arg.as_str()) {
                parsed.options.push((arg, iter.next()?));
            } else if arg.starts_with("--") {
                parsed.flags.push(arg);
            } else {
                parsed.positional.push(arg);
            }
        }
        Some(parsed)
    }

    pub(crate) fn flag(&self, name: &str) -> bool {
        self.flags.contains(&name)
    }

    pub(crate) fn option(&self, name: &str) -> Option<&'a str> {
        self.options
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| *v)
    }

    /// Fails on flags the command does not know, so a typo never goes unnoticed.
    pub(crate) fn only_flags(&self, known: &[&str]) -> bool {
        self.flags.iter().all(|f| known.contains(f))
    }
}

pub(crate) fn fail(message: impl std::fmt::Display) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(1)
}

pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.exists() {
        return Err(format!(
            "{} already exists: choose another name",
            path.display()
        ));
    }
    std::fs::write(path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))
}
