//! `docs/isd-booster-collation.md` is the stated contract for the pack
//! simulator: what a reader checks the generator against, and what anyone
//! reimplementing it would build from. So where the document names cards,
//! the names have to be the ones in `data/sets/isd.json`.
//!
//! They were not. Three rares were listed as mythics and three mythics as
//! rares, inverting the stated frequency of six cards — Rooftop Storm read
//! as a 0.83% mythic and Angelic Overseer as a 1.65% rare, both backwards
//! (issue #216). The generator derives rarity from the copy counts in the
//! data file and never from these lists, so the packs were right the whole
//! time; only the document was wrong, which is the kind of error that costs
//! a reader an evening rather than a run.

use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn doc() -> String {
    std::fs::read_to_string(repo_root().join("docs/isd-booster-collation.md"))
        .expect("the collation doc is part of the repo")
}

/// The bulleted card names under a heading, up to the first blank line
/// after the list starts.
fn listed_after(heading: &str) -> BTreeSet<String> {
    let text = doc();
    let start = text
        .find(heading)
        .unwrap_or_else(|| panic!("the doc still has a {heading:?} list"));
    text[start..]
        .lines()
        .skip(1)
        .skip_while(|l| l.trim().is_empty())
        .take_while(|l| l.starts_with("- "))
        .map(|l| l[2..].trim().to_string())
        .collect()
}

/// Names in the A run with a given number of copies. Copy count is what the
/// generator reads, so it is the authority on which cards are which rarity:
/// mythics are printed twice on the sheet and rares four times.
fn a_run_names_with_copies(copies: usize) -> BTreeSet<String> {
    let path = repo_root().join("data/sets/isd.json");
    let raw = std::fs::read_to_string(path).expect("the shipped set data");
    let json: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
    let run = json["runs"]["rare_a"].as_array().expect("rare_a is a run");

    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for card in run {
        *counts.entry(card.as_str().expect("a card name").to_string()).or_default() += 1;
    }
    counts
        .into_iter()
        .filter(|(_, n)| *n == copies)
        .map(|(name, _)| name)
        .collect()
}

#[test]
fn the_doc_lists_the_a_run_mythics_the_data_file_has() {
    let documented = listed_after("Mythics in A run");
    let actual = a_run_names_with_copies(2);
    assert_eq!(
        documented, actual,
        "the doc's mythic list must be the cards printed twice in rare_a"
    );
    assert_eq!(actual.len(), 15, "Innistrad has 15 mythic rares");
}

#[test]
fn the_doc_lists_the_a_run_rares_the_data_file_has() {
    let documented = listed_after("Rares in A run");
    let actual = a_run_names_with_copies(4);
    assert_eq!(
        documented, actual,
        "the doc's rare list must be the cards printed four times in rare_a"
    );
    assert_eq!(actual.len(), 6, "six rares share the A run with the mythics");
}

/// The arithmetic the two lists are supposed to satisfy: 15x2 + 6x4 + 1 = 55.
#[test]
fn the_a_run_adds_up_to_its_55_slots() {
    let mythics = a_run_names_with_copies(2).len();
    let rares = a_run_names_with_copies(4).len();
    let singles = a_run_names_with_copies(1).len();
    assert_eq!(mythics * 2 + rares * 4 + singles, 55);
}
