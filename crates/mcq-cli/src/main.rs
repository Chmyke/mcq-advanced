//! ROLE:    the `mcq` command line: checks, publishes and opens papers without any window.
//! OWNER:   owns file access, the system RNG and the terminal; the logic lives in the library crates.
//!
//! Exit code: 0 on success, 1 on failure, 2 on bad usage.

mod check;
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
      Prints the full paper, answers included.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest = args.get(1..).unwrap_or_default();
    match args.first().map(String::as_str) {
        Some("check") => check::check(rest),
        Some("identity") => secure::identity(rest),
        Some("publish") => secure::publish(rest),
        Some("inspect") => secure::inspect(rest),
        Some("answers") => secure::answers(rest),
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
