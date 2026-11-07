//! The copies left unfinished on this device, for the welcome screen: one entry per copy, kept
//! until that copy is handed in or erased, so opening the paper under another name never hides
//! the first one.
//!
//! Stored as one `localStorage` value: entries joined by `_`, each the copy's storage key and its
//! paper's title in base 64, joined by `.`. A single entry is what earlier versions wrote.

use crate::b64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Where the copy is saved (`mcq-session-<paper id>-<name>`, hex).
    pub key: String,
    pub title: String,
}

impl Entry {
    /// The paper's id in hex, from the copy's storage key; empty for a key of another shape.
    pub fn paper(&self) -> &str {
        self.key
            .strip_prefix("mcq-session-")
            .and_then(|rest| rest.split_once('-'))
            .map_or("", |(paper, _)| paper)
    }
}

/// The entries of a stored value; malformed ones are left out.
pub fn parse(value: &str) -> Vec<Entry> {
    value
        .split('_')
        .filter_map(|part| {
            let (key, title) = part.split_once('.')?;
            let title = String::from_utf8(b64::decode(title)?).ok()?;
            (!key.is_empty()).then(|| Entry {
                key: key.to_string(),
                title,
            })
        })
        .collect()
}

/// The value to store, from hex keys and base 64 titles only.
pub fn format(entries: &[Entry]) -> String {
    entries
        .iter()
        .map(|e| format!("{}.{}", e.key, b64::encode(e.title.as_bytes())))
        .collect::<Vec<_>>()
        .join("_")
}

/// The entries with the copy under `key` noted, once.
pub fn with(mut entries: Vec<Entry>, key: &str, title: &str) -> Vec<Entry> {
    entries.retain(|e| e.key != key);
    entries.push(Entry {
        key: key.to_string(),
        title: title.to_string(),
    });
    entries
}

/// The entries without the copy under `key`.
pub fn without(mut entries: Vec<Entry>, key: &str) -> Vec<Entry> {
    entries.retain(|e| e.key != key);
    entries
}

/// One line per paper with an unfinished copy, in the order noted: its id (hex) and title. Two
/// students' copies of the same paper make one line, which names nobody.
pub fn papers(entries: &[Entry]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for entry in entries {
        if !out.iter().any(|(paper, _)| paper == entry.paper()) {
            out.push((entry.paper().to_string(), entry.title.clone()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, title: &str) -> Entry {
        Entry {
            key: key.into(),
            title: title.into(),
        }
    }

    #[test]
    fn entries_round_trip_and_read_the_single_entry_of_earlier_versions() {
        let list = vec![
            entry("mcq-session-ab-0102", "Cells: 1 «revision»"),
            entry("mcq-session-cd-03", "Été"),
        ];
        let value = format(&list);
        assert!(
            value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"+/=-_.".contains(&b))
        );
        assert_eq!(parse(&value), list);
        let earlier = format!("mcq-session-ab-0102.{}", b64::encode(b"Cells"));
        assert_eq!(parse(&earlier), vec![entry("mcq-session-ab-0102", "Cells")]);
        assert!(parse("").is_empty());
        assert!(parse("garbage").is_empty());
    }

    #[test]
    fn a_copy_is_noted_once_and_forgotten_alone() {
        let list = with(Vec::new(), "mcq-session-ab-01", "A");
        let list = with(list, "mcq-session-ab-02", "A");
        let list = with(list, "mcq-session-ab-01", "A");
        assert_eq!(list.len(), 2);
        let list = without(list, "mcq-session-ab-01");
        assert_eq!(list, vec![entry("mcq-session-ab-02", "A")]);
    }

    #[test]
    fn two_copies_of_one_paper_make_one_line() {
        let list = vec![
            entry("mcq-session-ab-01", "A"),
            entry("mcq-session-cd-01", "B"),
            entry("mcq-session-ab-02", "A"),
        ];
        assert_eq!(
            papers(&list),
            vec![("ab".into(), "A".into()), ("cd".into(), "B".into())]
        );
        assert_eq!(entry("other", "x").paper(), "");
    }
}
