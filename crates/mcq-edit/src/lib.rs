//! ROLE:    what the Studio edits: a paper with its media files, the editing operations on it, the
//!          checks the editor shows live, and the `.qcmz` file that carries both.
//! OWNER:   `Project` owns the paper and its media bytes; the editor holds one project at a time.
//! INVARIANTS: no I/O, no clock; media paths are `media/<safe name>`; option ids stay positional
//!             after every operation; the source text is always `mcq_format::write(&paper)`.

pub mod ops;
pub mod qcmz;

use std::collections::BTreeMap;

use mcq_model::{Issue, Location, MediaKind, Paper, Severity, Theme};

/// One media file above this size gets a warning: students download every file of the paper.
pub const HEAVY_MEDIA_BYTES: usize = 5 * 1024 * 1024;
/// A paper whose media add up to more than this gets a warning.
pub const HEAVY_PAPER_BYTES: usize = 25 * 1024 * 1024;

/// Folder of the media files, relative to the paper.
pub const MEDIA_FOLDER: &str = "media/";

/// A paper and the media files its statements use.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Project {
    pub paper: Paper,
    /// Path relative to the paper (`media/x.png`) to file content.
    pub media: BTreeMap<String, Vec<u8>>,
}

impl Project {
    /// A paper to start from: one theme holding one question with two options.
    pub fn starter(title: &str, lang: &str) -> Self {
        let mut paper = Paper::new();
        paper.title = title.to_string();
        paper.lang = lang.to_string();
        paper.themes.push(Theme {
            id: "theme-1".to_string(),
            title: if lang.starts_with("fr") {
                "Thème 1"
            } else {
                "Theme 1"
            }
            .to_string(),
            ..Theme::default()
        });
        ops::add_question(&mut paper, "theme-1");
        Self {
            paper,
            media: BTreeMap::new(),
        }
    }

    /// Reads a source text, keeping the current media; returns what the reader reported.
    pub fn set_source(&mut self, source: &str) -> Vec<Issue> {
        let (paper, issues) = mcq_format::read(source);
        self.paper = paper;
        issues
    }

    /// The `.qcm.md` text of the paper.
    pub fn source(&self) -> String {
        mcq_format::write(&self.paper)
    }

    /// Everything the editor shows: the linter's findings, then missing and heavy media.
    pub fn issues(&self) -> Vec<Issue> {
        let mut issues = mcq_lint::lint(&self.paper);
        for question in &self.paper.questions {
            for media in question.media() {
                if !self.media.contains_key(&media.path) {
                    issues.push(Issue::new(
                        Severity::Error,
                        "MEDIA_NOT_FOUND",
                        Location::Question(question.id.clone()),
                        format!("`{}` is not in the paper's media.", media.path),
                        format!("`{}` ne fait pas partie des médias du sujet.", media.path),
                    ));
                }
            }
        }
        let mut total = 0;
        for (path, data) in &self.media {
            total += data.len();
            if data.len() > HEAVY_MEDIA_BYTES {
                issues.push(Issue::new(
                    Severity::Warning,
                    "HEAVY_MEDIA",
                    Location::Paper,
                    format!(
                        "`{path}` weighs {}: every student downloads it. Compress it or shorten it.",
                        size_label(data.len())
                    ),
                    format!(
                        "`{path}` pèse {} : chaque étudiant le télécharge. Compressez-le ou \
                         raccourcissez-le.",
                        size_label_fr(data.len())
                    ),
                ));
            }
        }
        if total > HEAVY_PAPER_BYTES {
            issues.push(Issue::new(
                Severity::Warning,
                "HEAVY_PAPER",
                Location::Paper,
                format!(
                    "The media weigh {} in all: slow to open on a phone or a busy network.",
                    size_label(total)
                ),
                format!(
                    "Les médias pèsent {} en tout : lents à ouvrir sur un téléphone ou un réseau \
                     chargé.",
                    size_label_fr(total)
                ),
            ));
        }
        issues
    }

    /// Paths of the media files a statement uses, each once, in order of first use.
    pub fn used_media(&self) -> Vec<String> {
        let mut used: Vec<String> = Vec::new();
        for media in self.paper.questions.iter().flat_map(|q| q.media()) {
            if !used.contains(&media.path) {
                used.push(media.path.clone());
            }
        }
        used
    }

    /// Stores a file under a safe, unused name derived from `file_name` and returns its path.
    /// `None` when the file is not an image, a sound or a video.
    pub fn add_media(&mut self, file_name: &str, data: Vec<u8>) -> Option<String> {
        let name = safe_file_name(file_name);
        MediaKind::from_path(&name)?;
        let (stem, ext) = name.rsplit_once('.').unwrap_or((name.as_str(), ""));
        let mut path = format!("{MEDIA_FOLDER}{stem}.{ext}");
        let mut n = 2;
        while self
            .media
            .get(&path)
            .is_some_and(|existing| *existing != data)
        {
            path = format!("{MEDIA_FOLDER}{stem}-{n}.{ext}");
            n += 1;
        }
        self.media.insert(path.clone(), data);
        Some(path)
    }

    /// Forgets the files no statement uses any more.
    pub fn prune_media(&mut self) {
        let used = self.used_media();
        self.media.retain(|path, _| used.contains(path));
    }
}

/// Lowercase ASCII letters, digits, `-`, `_` and one `.` before the extension; never empty.
pub fn safe_file_name(file_name: &str) -> String {
    let base = file_name.rsplit(['/', '\\']).next().unwrap_or_default();
    let (stem, ext) = match base.rsplit_once('.') {
        Some((stem, ext)) => (stem, ext),
        None => (base, ""),
    };
    let stem = match slug(stem) {
        s if s.is_empty() => "media".to_string(),
        s => s,
    };
    let ext = slug(ext);
    if ext.is_empty() {
        stem
    } else {
        format!("{stem}.{ext}")
    }
}

/// Lowercase ASCII letters, digits and `_`, accents dropped (`é` gives `e`), other runs of
/// characters as one `-`; may be empty.
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars().flat_map(char::to_lowercase) {
        let folded = fold_accent(c);
        for c in folded.chars() {
            let c = if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '-'
            };
            if !(c == '-' && (out.is_empty() || out.ends_with('-'))) {
                out.push(c);
            }
        }
    }
    out.trim_end_matches('-').to_string()
}

/// The plain letters of an accented Latin lowercase letter; other characters unchanged.
fn fold_accent(c: char) -> String {
    let plain = match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => "a",
        'ç' => "c",
        'è' | 'é' | 'ê' | 'ë' => "e",
        'ì' | 'í' | 'î' | 'ï' => "i",
        'ñ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => "o",
        'ù' | 'ú' | 'û' | 'ü' => "u",
        'ý' | 'ÿ' => "y",
        'æ' => "ae",
        'œ' => "oe",
        'ß' => "ss",
        _ => return c.to_string(),
    };
    plain.to_string()
}

/// `1.4 MB`, `820 KB`: for messages, in powers of 1024.
pub fn size_label(bytes: usize) -> String {
    let kib = bytes as f64 / 1024.0;
    if kib < 1024.0 {
        format!("{} KB", kib.ceil() as u64)
    } else {
        format!("{:.1} MB", kib / 1024.0)
    }
}

/// `size_label` as written in French: `Ko`, `Mo` and a decimal comma.
pub fn size_label_fr(bytes: usize) -> String {
    size_label(bytes)
        .replace('.', ",")
        .replace("KB", "Ko")
        .replace("MB", "Mo")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_become_safe() {
        assert_eq!(
            safe_file_name("C:\\Users\\Zoë\\Mon Schéma (2).PNG"),
            "mon-schema-2.png"
        );
        assert_eq!(safe_file_name("../../etc/passwd"), "passwd");
        assert_eq!(safe_file_name(".png"), "media.png");
        assert_eq!(safe_file_name("voice"), "voice");
    }

    #[test]
    fn slugs_keep_the_letters_of_accented_words() {
        assert_eq!(
            slug("Dictée n°1, Œuvre à l'écrit"),
            "dictee-n-1-oeuvre-a-l-ecrit"
        );
        assert_eq!(slug("Thème 2"), "theme-2");
        assert_eq!(slug("日本"), "");
    }

    #[test]
    fn media_get_unique_paths_and_unused_ones_go() {
        let mut project = Project::starter("T", "en");
        let a = project.add_media("pic.png", vec![1]);
        let same = project.add_media("pic.png", vec![1]);
        let other = project.add_media("pic.png", vec![2]);
        assert_eq!(a.as_deref(), Some("media/pic.png"));
        assert_eq!(same, a, "the same file is stored once");
        assert_eq!(other.as_deref(), Some("media/pic-2.png"));
        assert_eq!(project.add_media("notes.txt", vec![3]), None);
        project.prune_media();
        assert!(project.media.is_empty());
    }

    #[test]
    fn sizes_read_like_a_file_manager() {
        assert_eq!(size_label(1000), "1 KB");
        assert_eq!(size_label(3 * 1024 * 1024 + 200_000), "3.2 MB");
        assert_eq!(size_label_fr(3 * 1024 * 1024 + 200_000), "3,2 Mo");
    }
}
