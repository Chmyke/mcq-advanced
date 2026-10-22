//! Identity, publishing and opening commands.
//!
//! Secrets are read from the first line of standard input, so scripts and tests can drive them;
//! they are never taken from arguments, which other users of the machine can see.

use std::path::Path;
use std::process::ExitCode;

use mcq_crypto::rand_core::UnwrapErr;
use mcq_crypto::{Identity, KdfParams, KeyUnlock, Opening, OpeningKind, PublishOptions};
use mcq_format::binary::Entry;
use mcq_model::Severity;

use crate::{read_text, usage};

/// Flags and options of one command, the rest being positional arguments.
struct Args<'a> {
    positional: Vec<&'a str>,
    flags: Vec<&'a str>,
    options: Vec<(&'a str, &'a str)>,
}

impl<'a> Args<'a> {
    /// `valued` lists the options that take a value.
    fn parse(args: &'a [String], valued: &[&str]) -> Option<Self> {
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

    fn flag(&self, name: &str) -> bool {
        self.flags.contains(&name)
    }

    fn option(&self, name: &str) -> Option<&'a str> {
        self.options
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| *v)
    }

    /// Fails on flags the command does not know, so a typo never goes unnoticed.
    fn only_flags(&self, known: &[&str]) -> bool {
        self.flags.iter().all(|f| known.contains(f))
    }
}

fn fail(message: impl std::fmt::Display) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(1)
}

fn system_rng() -> UnwrapErr<getrandom::SysRng> {
    UnwrapErr(getrandom::SysRng)
}

fn read_secret() -> Result<String, String> {
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| format!("cannot read the passphrase: {e}"))?;
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.exists() {
        return Err(format!(
            "{} already exists: choose another name",
            path.display()
        ));
    }
    std::fs::write(path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn unlock(identity_path: &str) -> Result<Identity, String> {
    let file =
        std::fs::read(identity_path).map_err(|e| format!("cannot read the identity: {e}"))?;
    let passphrase = read_secret()?;
    Identity::unlock(&file, &passphrase).map_err(|e| e.to_string())
}

pub fn identity(args: &[String]) -> ExitCode {
    let Some(parsed) = Args::parse(args, &[]) else {
        return usage();
    };
    match (parsed.positional.as_slice(), parsed.flags.is_empty()) {
        (["new", path], true) => {
            let passphrase = match read_secret() {
                Ok(p) => p,
                Err(e) => return fail(e),
            };
            let mut rng = system_rng();
            let identity = Identity::generate(&mut rng);
            let file = match identity.lock(&passphrase, KdfParams::PASSPHRASE, &mut rng) {
                Ok(file) => file,
                Err(e) => return fail(e),
            };
            if let Err(e) = write_new(Path::new(path), &file) {
                return fail(e);
            }
            println!("Identity created: {path}");
            println!("Fingerprint: {}", identity.public().fingerprint());
            println!(
                "Keep a backup copy: without this file and its passphrase, copies cannot be read."
            );
            ExitCode::SUCCESS
        }
        (["show", path], true) => {
            let file = match std::fs::read(path) {
                Ok(file) => file,
                Err(e) => return fail(format!("cannot read the identity: {e}")),
            };
            match Identity::read_public(&file) {
                Ok(public) => {
                    println!("Fingerprint: {}", public.fingerprint());
                    ExitCode::SUCCESS
                }
                Err(e) => fail(e),
            }
        }
        _ => usage(),
    }
}

pub fn publish(args: &[String]) -> ExitCode {
    let Some(parsed) = Args::parse(args, &["--identity", "--out"]) else {
        return usage();
    };
    let (Some(identity_path), Some(out), [paper_path]) = (
        parsed.option("--identity"),
        parsed.option("--out"),
        parsed.positional.as_slice(),
    ) else {
        return usage();
    };
    if !parsed.only_flags(&["--end-code", "--practice"]) {
        return usage();
    }
    let practice = parsed.flag("--practice");
    if practice && parsed.flag("--end-code") {
        return fail("a practice package has no end code: its answers are already readable");
    }

    let paper_path = Path::new(paper_path);
    let issues = match crate::check::check_file(paper_path) {
        Ok(issues) => issues,
        Err(e) => return fail(e),
    };
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    if !errors.is_empty() {
        for issue in errors {
            println!("{}", crate::check::format_issue(paper_path, issue));
        }
        return fail("the paper has errors: fix them before publishing");
    }
    let source = match read_text(paper_path) {
        Ok(source) => source,
        Err(e) => return fail(e),
    };
    let (paper, _) = mcq_format::read(&source);
    let folder = paper_path.parent().unwrap_or(Path::new("."));
    let mut media: Vec<Entry> = Vec::new();
    for item in paper.questions.iter().flat_map(|q| q.media()) {
        if media.iter().any(|m| m.path == item.path) {
            continue;
        }
        match std::fs::read(folder.join(&item.path)) {
            Ok(data) => media.push(Entry {
                path: item.path.clone(),
                data,
            }),
            Err(e) => return fail(format!("cannot read {}: {e}", item.path)),
        }
    }

    // Code is coloured once, here, so players only map token kinds to theme colours.
    let tokens = mcq_highlight::Highlighter::new().paper(&paper);
    if !tokens.is_empty() {
        media.push(Entry {
            path: mcq_format::binary::CODE_TOKENS_FILE.to_string(),
            data: mcq_format::binary::encode_code_tokens(&tokens),
        });
    }

    let identity = match unlock(identity_path) {
        Ok(identity) => identity,
        Err(e) => return fail(e),
    };
    let mut rng = system_rng();
    let opening = if practice {
        Opening::Open
    } else {
        Opening::Code(mcq_crypto::generate_code(&mut rng))
    };
    let end_code = parsed
        .flag("--end-code")
        .then(|| mcq_crypto::generate_code(&mut rng));
    let options = PublishOptions {
        opening: opening.clone(),
        end_code: end_code.clone(),
        kdf: KdfParams::CODE,
    };
    let package = match mcq_crypto::publish(&identity, &paper, &media, &options, &mut rng) {
        Ok(package) => package,
        Err(e) => return fail(e),
    };
    if let Err(e) = write_new(Path::new(out), &package) {
        return fail(e);
    }
    println!("Package written: {out}");
    println!("Teacher fingerprint: {}", identity.public().fingerprint());
    match &opening {
        Opening::Code(code) => println!("Opening code: {code}"),
        _ => println!("Practice package: no opening code, answers readable by students."),
    }
    if let Some(code) = end_code {
        println!("End code: {code}");
    }
    ExitCode::SUCCESS
}

pub fn inspect(args: &[String]) -> ExitCode {
    let Some(parsed) = Args::parse(args, &[]) else {
        return usage();
    };
    let ([path], true) = (parsed.positional.as_slice(), parsed.flags.is_empty()) else {
        return usage();
    };
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => return fail(format!("cannot read the package: {e}")),
    };
    match mcq_crypto::inspect(&bytes) {
        Ok(header) => {
            println!("Signature: valid");
            println!("Teacher fingerprint: {}", header.teacher.fingerprint());
            let opening = match header.opening {
                OpeningKind::Code => "code",
                OpeningKind::Passphrase => "passphrase",
                OpeningKind::Open => "none (practice)",
            };
            println!("Opening: {opening}");
            println!(
                "End code: {}",
                if header.has_end_code { "yes" } else { "no" }
            );
            ExitCode::SUCCESS
        }
        Err(e) => fail(e),
    }
}

pub fn answers(args: &[String]) -> ExitCode {
    let Some(parsed) = Args::parse(args, &["--identity", "--end-code"]) else {
        return usage();
    };
    let ([path], true) = (
        parsed.positional.as_slice(),
        parsed.only_flags(&["--practice"]),
    ) else {
        return usage();
    };
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => return fail(format!("cannot read the package: {e}")),
    };
    let identity;
    let unlock_with = match (
        parsed.option("--identity"),
        parsed.option("--end-code"),
        parsed.flag("--practice"),
    ) {
        (Some(identity_path), None, false) => {
            identity = match unlock(identity_path) {
                Ok(identity) => identity,
                Err(e) => return fail(e),
            };
            KeyUnlock::Teacher(&identity)
        }
        (None, Some(code), false) => KeyUnlock::EndCode(code),
        (None, None, true) => KeyUnlock::Practice,
        _ => return usage(),
    };
    match mcq_crypto::open_answer_key(&bytes, unlock_with) {
        Ok(paper) => {
            print!("{}", mcq_format::write(&paper));
            ExitCode::SUCCESS
        }
        Err(e) => fail(e),
    }
}
