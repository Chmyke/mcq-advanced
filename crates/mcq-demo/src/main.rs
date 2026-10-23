//! Writes the website demo into a folder: `<lang>.mcqpack` and `<lang>.qcmz` per language, and the
//! gallery in `gallery/`: `<stem>-<lang>.mcqpack` and `.qcmz` per model, and `index.json`.
//! Run: cargo run -p mcq-demo -- <folder>

use std::path::Path;
use std::process::ExitCode;

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    println!("{}", path.display());
    Ok(())
}

fn run(folder: &Path) -> Result<(), String> {
    let built = mcq_demo::build_all()?;
    let gallery = folder.join("gallery");
    std::fs::create_dir_all(&gallery)
        .map_err(|e| format!("cannot create {}: {e}", gallery.display()))?;
    for demo in &built {
        write(
            &folder.join(format!("{}.mcqpack", demo.lang)),
            &demo.package,
        )?;
        write(&folder.join(format!("{}.qcmz", demo.lang)), &demo.qcmz)?;
    }
    for model in &mcq_demo::GALLERY {
        let built = mcq_demo::build_model(model)?;
        let name = format!("{}-{}", model.stem, model.source.lang);
        write(&gallery.join(format!("{name}.mcqpack")), &built.package)?;
        write(&gallery.join(format!("{name}.qcmz")), &built.qcmz)?;
    }
    write(
        &gallery.join("index.json"),
        mcq_demo::gallery_index().as_bytes(),
    )?;
    if let Some(first) = built.first() {
        println!("Demo fingerprint: {}", first.fingerprint);
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [folder] = args.as_slice() else {
        eprintln!("usage: mcq-demo <folder>");
        return ExitCode::from(2);
    };
    match run(Path::new(folder)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
