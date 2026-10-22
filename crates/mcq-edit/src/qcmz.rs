//! The `.qcmz` file: a paper folder (`<name>.qcm.md` and `media/`) as one zip, so the web Studio
//! can import and export it. Any zip of such a folder is accepted, with or without the folder
//! itself at the top.

use std::io::{Cursor, Read, Write};

use mcq_format::binary::is_safe_path;
use mcq_model::{Issue, MediaKind};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::{MEDIA_FOLDER, Project, slug};

/// Larger files are refused on import: no paper needs them, and a forged zip could claim more.
pub const MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;
/// Larger contents are refused on import.
pub const MAX_TOTAL_BYTES: u64 = 200 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QcmzError {
    /// Not a readable zip.
    NotAZip,
    /// No `.qcm.md` (or single `.md`) file inside.
    NoPaper,
    /// Several papers: which one to open is ambiguous.
    SeveralPapers,
    /// A file or the whole content is above the import limits.
    TooLarge,
    /// The paper is not UTF-8 text.
    NotText,
    /// The zip could not be written (never expected in memory).
    Write,
}

impl std::fmt::Display for QcmzError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            QcmzError::NotAZip => "not a .qcmz or zip file",
            QcmzError::NoPaper => "no .qcm.md paper inside",
            QcmzError::SeveralPapers => "several papers inside: keep one per file",
            QcmzError::TooLarge => "too large to import",
            QcmzError::NotText => "the paper is not UTF-8 text",
            QcmzError::Write => "cannot write the zip",
        })
    }
}

/// Name of the paper file inside an exported `.qcmz`, also a good name for the `.qcmz` itself.
pub fn file_stem(project: &Project) -> String {
    match slug(&project.paper.title) {
        s if s.is_empty() => "paper".to_string(),
        s => s,
    }
}

/// The paper and all its media, the paper file at the top of the zip.
pub fn export(project: &Project) -> Result<Vec<u8>, QcmzError> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let text = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    // Media formats are already compressed: storing them is as small and much faster.
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let mut add = |name: &str, data: &[u8], options| -> Result<(), QcmzError> {
        zip.start_file(name, options)
            .map_err(|_| QcmzError::Write)?;
        zip.write_all(data).map_err(|_| QcmzError::Write)
    };
    let paper_name = format!("{}.qcm.md", file_stem(project));
    add(&paper_name, project.source().as_bytes(), text)?;
    for (path, data) in &project.media {
        let options = if path.to_ascii_lowercase().ends_with(".svg") {
            text
        } else {
            stored
        };
        add(path, data, options)?;
    }
    let cursor = zip.finish().map_err(|_| QcmzError::Write)?;
    Ok(cursor.into_inner())
}

/// Reads a `.qcmz`: the paper, the media files under its `media/` folder, and what the reader
/// reported about the paper. Other files are ignored.
pub fn import(bytes: &[u8]) -> Result<(Project, Vec<Issue>), QcmzError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| QcmzError::NotAZip)?;
    let mut names: Vec<(usize, String)> = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|_| QcmzError::NotAZip)?;
        if entry.is_dir() {
            continue;
        }
        // `enclosed_name` refuses absolute paths and `..`: nothing can point outside the folder.
        if let Some(path) = entry.enclosed_name() {
            let name = path.to_string_lossy().replace('\\', "/");
            names.push((index, name));
        }
    }

    let papers: Vec<&(usize, String)> = names
        .iter()
        .filter(|(_, n)| n.to_ascii_lowercase().ends_with(".qcm.md"))
        .collect();
    let papers = if papers.is_empty() {
        names
            .iter()
            .filter(|(_, n)| n.to_ascii_lowercase().ends_with(".md"))
            .collect()
    } else {
        papers
    };
    let (paper_index, paper_name) = match papers.as_slice() {
        [] => return Err(QcmzError::NoPaper),
        [one] => (*one).clone(),
        _ => return Err(QcmzError::SeveralPapers),
    };
    let base = paper_name
        .rsplit_once('/')
        .map_or(String::new(), |(folder, _)| format!("{folder}/"));

    let mut total = 0_u64;
    let source = read_entry(&mut archive, paper_index, &mut total)?;
    let source = String::from_utf8(source).map_err(|_| QcmzError::NotText)?;
    let mut project = Project::default();
    let issues = project.set_source(&source);

    for (index, name) in &names {
        let Some(relative) = name.strip_prefix(&base) else {
            continue;
        };
        if !relative.starts_with(MEDIA_FOLDER)
            || !is_safe_path(relative)
            || MediaKind::from_path(relative).is_none()
        {
            continue;
        }
        let data = read_entry(&mut archive, *index, &mut total)?;
        project.media.insert(relative.to_string(), data);
    }
    Ok((project, issues))
}

/// Reads one entry within the size limits, whatever size the zip claims.
fn read_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    index: usize,
    total: &mut u64,
) -> Result<Vec<u8>, QcmzError> {
    let entry = archive.by_index(index).map_err(|_| QcmzError::NotAZip)?;
    let mut data = Vec::new();
    entry
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut data)
        .map_err(|_| QcmzError::NotAZip)?;
    *total += data.len() as u64;
    if data.len() as u64 > MAX_FILE_BYTES || *total > MAX_TOTAL_BYTES {
        return Err(QcmzError::TooLarge);
    }
    Ok(data)
}
