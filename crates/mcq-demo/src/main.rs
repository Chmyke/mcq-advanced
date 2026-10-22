//! Writes the website demo into a folder: `<lang>.mcqpack` and `<lang>.qcmz` per language.
//! Run: cargo run -p mcq-demo -- <folder>

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [folder] = args.as_slice() else {
        eprintln!("usage: mcq-demo <folder>");
        return ExitCode::from(2);
    };
    let built = match mcq_demo::build_all() {
        Ok(built) => built,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let folder = Path::new(folder);
    if let Err(e) = std::fs::create_dir_all(folder) {
        eprintln!("error: cannot create {}: {e}", folder.display());
        return ExitCode::FAILURE;
    }
    for demo in &built {
        for (extension, bytes) in [("mcqpack", &demo.package), ("qcmz", &demo.qcmz)] {
            let path = folder.join(format!("{}.{extension}", demo.lang));
            if let Err(e) = std::fs::write(&path, bytes) {
                eprintln!("error: cannot write {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
            println!("{}", path.display());
        }
    }
    if let Some(first) = built.first() {
        println!("Demo fingerprint: {}", first.fingerprint);
    }
    ExitCode::SUCCESS
}
