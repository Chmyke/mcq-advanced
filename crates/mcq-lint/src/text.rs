//! Text heuristics of the linter, in French and English.

/// Negations that turn a question into "find the exception". English "false" is left out:
/// "true or false?" is a common, harmless stem.
const NEGATIONS: [&str; 12] = [
    "pas",
    "jamais",
    "sauf",
    "aucun",
    "aucune",
    "fausse",
    "incorrecte",
    "not",
    "never",
    "except",
    "none",
    "incorrect",
];

/// Options that refer to the other options.
const META_OPTIONS: [&str; 9] = [
    "toutes les réponses",
    "toutes ces réponses",
    "aucune des réponses",
    "aucune de ces réponses",
    "all of the above",
    "all the above",
    "none of the above",
    "all of these",
    "none of these",
];

/// Stems that do not say whether compiling or running is asked, compared without spaces.
const VAGUE_ENDINGS: [&str; 6] = [
    "quesepasse-t-il?",
    "qu'obtient-on?",
    "quelestlerésultat?",
    "whathappens?",
    "whatdoweget?",
    "whatistheresult?",
];

/// The sentences that end with `?`, each followed by a space. Empty when nothing is asked.
pub fn interrogative_part(text: &str) -> String {
    let mut out = String::new();
    let mut sentence = String::new();
    for c in text.chars() {
        sentence.push(c);
        if matches!(c, '.' | '!' | '?' | '\n') {
            if c == '?' {
                out.push_str(sentence.trim());
                out.push(' ');
            }
            sentence.clear();
        }
    }
    out
}

/// The first negation written in lower case only, when the part has no bold at all.
pub fn unemphasised_negation(part: &str) -> Option<&'static str> {
    if part.contains("**") {
        return None;
    }
    let lower = part.to_lowercase();
    let word = NEGATIONS.into_iter().find(|w| contains_word(&lower, w))?;
    let upper = word.to_uppercase();
    (!contains_word(part, &upper)).then_some(word)
}

pub fn is_meta_option(text: &str) -> bool {
    let lower = text.trim().to_lowercase().replace('\u{2019}', "'");
    META_OPTIONS.iter().any(|m| lower.starts_with(m))
}

pub fn is_vague_stem(part: &str) -> bool {
    let compact: String = part
        .to_lowercase()
        .replace('\u{2019}', "'")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    VAGUE_ENDINGS.iter().any(|e| compact.ends_with(e))
}

/// Case-sensitive search where the characters around the match are not letters.
pub fn contains_word(haystack: &str, word: &str) -> bool {
    haystack.match_indices(word).any(|(start, _)| {
        let before = haystack[..start].chars().next_back();
        let after = haystack[start + word.len()..].chars().next();
        !before.is_some_and(char::is_alphabetic) && !after.is_some_and(char::is_alphabetic)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrogative_part_keeps_questions_only() {
        assert_eq!(
            interrogative_part("Here is code. What does it print?\nNote."),
            "What does it print? "
        );
        assert_eq!(interrogative_part("Pick one:"), "");
    }

    #[test]
    fn words_need_non_letter_neighbours() {
        assert!(contains_word("ce n'est pas vrai", "pas"));
        assert!(!contains_word("il faut passer", "pas"));
        assert!(!contains_word("un compas", "pas"));
        assert!(contains_word("(pas)", "pas"));
        assert!(contains_word("Which is not true?", "not"));
        assert!(!contains_word("nothing", "not"));
    }

    #[test]
    fn negation_needs_emphasis() {
        assert_eq!(unemphasised_negation("Which is not a fruit? "), Some("not"));
        assert_eq!(unemphasised_negation("Which is NOT a fruit? "), None);
        assert_eq!(unemphasised_negation("Which is **not** a fruit? "), None);
        assert_eq!(
            unemphasised_negation("Quelle réponse est fausse ? "),
            Some("fausse")
        );
        assert_eq!(unemphasised_negation("Which is a fruit? "), None);
    }

    #[test]
    fn meta_options_in_both_languages() {
        assert!(is_meta_option("Toutes les réponses sont justes"));
        assert!(is_meta_option("Aucune de ces réponses"));
        assert!(is_meta_option("None of the above"));
        assert!(!is_meta_option("All fruit are sweet"));
    }

    #[test]
    fn vague_stems_ignore_spacing() {
        assert!(is_vague_stem("Que se passe-t-il ? "));
        assert!(is_vague_stem("Que se passe-t-il\u{202f}? "));
        assert!(is_vague_stem("What happens? "));
        assert!(!is_vague_stem("What does it print? "));
    }
}
