//! What a link to the player may say besides `?paper=`: a name to pre-fill, the interface
//! language, and the paper's editable source, which adds "open this paper in the editor".
//!
//! The web builds sit side by side (`player/`, `studio/`), so the editor is one folder up. Only
//! same-site paths of plain characters are accepted: a link can point the visitor to the Studio
//! with a paper of this site, never anywhere else.

/// Where the Studio is, from the player's page.
pub const EDITOR_PATH: &str = "../studio/";

/// The options read from the page's link.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinkOptions {
    /// Pre-filled in the name field; the student can change it.
    pub name: String,
    /// `fr`, `en`... for the interface; empty to follow the browser.
    pub lang: String,
    /// Absolute path, on this site, of the paper's `.qcmz` source.
    pub source: String,
}

/// True for an absolute path on this site made of characters that need no escaping.
pub fn is_site_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.starts_with("//")
        && !path.contains("..")
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-~".contains(&b))
}

/// The Studio link that opens the source, or nothing when there is no usable source.
pub fn editor_href(source: &str) -> Option<String> {
    is_site_path(source).then(|| format!("{EDITOR_PATH}?open={source}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_link_opens_a_paper_of_this_site() {
        assert_eq!(
            editor_href("/mcq-advanced/demo/fr.qcmz").as_deref(),
            Some("../studio/?open=/mcq-advanced/demo/fr.qcmz")
        );
    }

    #[test]
    fn editor_link_refuses_anything_else() {
        for source in [
            "",
            "demo/fr.qcmz",
            "//evil.example/x.qcmz",
            "https://evil.example/x.qcmz",
            "javascript:alert(1)",
            "/a/../../x.qcmz",
            "/a b.qcmz",
            "/x.qcmz?y=\"",
            "/x.qcmz#top",
            "/x.qcmz&open=/y",
        ] {
            assert_eq!(editor_href(source), None, "{source}");
        }
    }
}
