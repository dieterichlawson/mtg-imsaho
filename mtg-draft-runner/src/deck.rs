//! Deck building: the LLM round-trip and the prompt it is built on.

use mtg_draft::deckbuilding::{self, DraftDeck};
use mtg_engine::cards::CardRegistry;

use crate::card_lines::CardLines;
use crate::llm_client::DraftLlmClient;

/// One round-trip with the model during deck building.
pub struct DeckAttempt {
    pub prompt: String,
    pub response: String,
    pub error: Option<String>,
}

/// Return type for deck building LLM interaction.
pub struct DeckBuildResult {
    pub deck: DraftDeck,
    pub attempts: Vec<DeckAttempt>,
    pub retries: usize,
    /// True when no attempt produced a valid deck and the runner
    /// substituted one. A substituted deck is not a drafted deck, and
    /// every record of the run has to say so (issue #200).
    pub fallback: bool,
}


pub fn build_deck_with_llm(
    client: &mut DraftLlmClient,
    pool: &[String],
    registry: &CardRegistry,
    cards: &CardLines,
) -> DeckBuildResult {
    let prompt = build_deck_prompt(pool, cards);
    let mut last_error = String::new();
    let mut attempts: Vec<DeckAttempt> = Vec::new();
    let max_retries = 10;

    for attempt in 0..max_retries {
        if attempt > 0 {
            // Brief delay before retry (helps with transient network errors)
            std::thread::sleep(std::time::Duration::from_secs(2));
        }

        let msg = if attempt == 0 {
            prompt.clone()
        } else {
            format!(
                "Your previous deck was invalid: {last_error}. Please try again.\n\n{prompt}"
            )
        };

        let response = client.send_deck_building_message(&msg, pool);

        match deckbuilding::parse_deck_response(&response) {
            Ok((maindeck, lands)) => match deckbuilding::validate_deck(pool, &maindeck, &lands) {
                Ok(deck) => {
                    attempts.push(DeckAttempt { prompt: msg, response, error: None });
                    let retries = attempts.len() - 1;
                    return DeckBuildResult { deck, attempts, retries, fallback: false };
                }
                Err(e) => {
                    attempts.push(DeckAttempt { prompt: msg, response, error: Some(e.clone()) });
                    last_error = e;
                }
            },
            Err(e) => {
                attempts.push(DeckAttempt { prompt: msg, response, error: Some(e.clone()) });
                last_error = e;
            }
        }
    }

    // No attempt produced a valid deck. The draft has already been played,
    // so the round still has to happen — but the deck it happens with is the
    // runner's, not the seat's, and everything downstream is told so.
    mtg_player::stderr_line!("Warning: deck building failed after {max_retries} attempts, using fallback");
    let retries = attempts.len();
    DeckBuildResult {
        deck: deckbuilding::fallback_deck(pool, registry),
        attempts,
        retries,
        fallback: true,
    }
}

/// The one message that decides a seat's whole deck.
///
/// It used to be a sentence and a list of names and counts: no colour, no
/// cost, no type — the two things a limited deck is built on — no land
/// target, no statement of the answer's shape, and no mention of the
/// sideboard it was silently creating. The only land guidance a seat ever
/// got was a `description` string inside the JSON schema (issue #487).
#[must_use]
pub fn build_deck_prompt(pool: &[String], cards: &CardLines) -> String {
    let mut prompt = format!(
        "Draft complete! Build your deck out of the {} cards you drafted.\n\n\
         Your pool ({} cards):\n",
        pool.len(),
        pool.len(),
    );
    prompt.push_str(&cards.pool_listing(pool));
    prompt.push_str(
        "\n## Building it\n\
         - A deck is at least 40 cards (CR 100.2b); a smaller one is rejected and you are asked again\n\
         - The usual limited build is 17 basic lands and 23 spells from the pool\n\
         - Two colors is the normal build, a third only as a splash you can reliably cast\n\
         - Everything you leave out is your sideboard. It is recorded with your deck, but nothing is sideboarded between games of a match, so a card you leave out is a card you will not play\n\
         \n## Your answer\n\
         - `maindeck` maps each drafted card you are playing to how many copies (0, or leave it out, to cut it)\n\
         - `lands` maps each basic land to how many to add. Basic lands are not drafted and are not limited: they go here, and only here, even if you drafted one\n",
    );
    prompt
}

/// Write a built deck as `COUNT NAME` lines, the format `mtg-runner
/// --deck1 <path>` reads, creating the directory it goes in.
///
/// # Errors
/// The directory cannot be made or the file cannot be written.
pub fn write_deck_file(path: &std::path::Path, deck: &DraftDeck) -> std::io::Result<()> {
    use std::fmt::Write as _;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut text = String::new();
    for (name, count) in deckbuilding::to_decklist(deck) {
        let _ = writeln!(text, "{count} {name}");
    }
    std::fs::write(path, text)
}

#[cfg(test)]
mod deck_prompt_tests {
    use super::{build_deck_prompt, CardLines, CardRegistry};

    /// #487: the whole prompt used to be one sentence and a `Nx Name` list —
    /// no colour or cost to build on, no land target, no statement of the
    /// answer's shape, and no mention of the sideboard it creates.
    #[test]
    fn the_deck_prompt_says_what_it_is_asking_for() {
        let set_data = mtg_draft::set_data::SetData::load(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../data/sets/isd.json"
        )))
        .expect("ISD set data");
        let registry = CardRegistry::with_all_cards();
        let cards = CardLines::new(&set_data.all_card_names(), &set_data.rarities(), &registry);

        let pool = vec![
            "Moon Heron".to_string(),
            "Moon Heron".to_string(),
            "Chapel Geist".to_string(),
            "Plains".to_string(),
        ];
        let prompt = build_deck_prompt(&pool, &cards);

        // The pool, with what each card costs and is.
        assert!(prompt.contains("2x Moon Heron {3}{U} | Creature — Spirit Bird 3/2"), "{prompt}");
        assert!(prompt.contains("Colors"), "{prompt}");
        assert!(prompt.contains("Curve"), "{prompt}");
        // The deck it is asking for.
        assert!(prompt.contains("40 cards"), "{prompt}");
        assert!(prompt.contains("17 basic lands and 23 spells"), "{prompt}");
        // The answer's shape, and where a drafted basic land goes.
        assert!(prompt.contains("`maindeck`"), "{prompt}");
        assert!(prompt.contains("`lands`"), "{prompt}");
        assert!(prompt.contains("even if you drafted one"), "{prompt}");
        // The sideboard it is silently creating.
        assert!(prompt.contains("sideboard"), "{prompt}");
    }
}
