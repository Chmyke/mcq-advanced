//! Which interface language a paper gets, and whether the student may switch.

use mcq_player::texts::{Lang, interface_for};

fn tags(list: &[&str]) -> Vec<String> {
    list.iter().map(|t| t.to_string()).collect()
}

#[test]
fn the_interface_languages_match_the_format() {
    let tags: Vec<&str> = Lang::ALL.iter().map(|l| l.tag()).collect();
    assert_eq!(tags, mcq_model::INTERFACE_LANGUAGES);
    assert_eq!(Lang::parse("FR-fr"), Some(Lang::Fr));
    assert_eq!(Lang::parse("de"), None);
    assert_eq!(Lang::parse(""), None);
}

#[test]
fn without_languages_there_is_no_switch_and_the_paper_decides() {
    assert_eq!(interface_for("fr", &[], Lang::En), (Lang::Fr, vec![]));
    assert_eq!(interface_for("en-GB", &[], Lang::Fr), (Lang::En, vec![]));
    // No language stated, or one without texts: the link or browser stands in.
    assert_eq!(interface_for("", &[], Lang::Fr), (Lang::Fr, vec![]));
    assert_eq!(interface_for("de", &[], Lang::En), (Lang::En, vec![]));
}

#[test]
fn offered_languages_are_the_only_choices() {
    let offered = tags(&["fr", "en"]);
    assert_eq!(
        interface_for("fr", &offered, Lang::En),
        (Lang::En, vec![Lang::Fr, Lang::En])
    );
    // A single language offered forces it, with no switch.
    assert_eq!(
        interface_for("en", &tags(&["fr"]), Lang::En),
        (Lang::Fr, vec![Lang::Fr])
    );
    // Languages without texts are left out; none left means none offered.
    assert_eq!(
        interface_for("fr", &tags(&["de", "en", "EN"]), Lang::Fr),
        (Lang::En, vec![Lang::En])
    );
    assert_eq!(
        interface_for("fr", &tags(&["de"]), Lang::En),
        (Lang::Fr, vec![])
    );
}
