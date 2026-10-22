//! Deck building: the LLM round-trip and the prompt it is built on.

use std::fmt::Write as _;

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
    let prompt = DraftLlmClient::notes_section(client.notes()) + &build_deck_prompt(pool, cards);
    let rows = deckbuilding::deck_rows(pool);
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

        match deckbuilding::parse_deck_response_by_rows(&response, &rows) {
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
    // Numbered, because the answer is one count per row: a schema keyed by
    // these names was one nobody had sent before, every seat (issue #867).
    for (i, (name, copies)) in deckbuilding::deck_rows(pool).iter().enumerate() {
        let _ = writeln!(prompt, "{i}: {copies}x {}", cards.pool_line(name));
    }
    let basics: Vec<&str> = pool.iter().map(|c| mtg_draft::front_face(c))
        .filter(|n| deckbuilding::BASIC_LANDS.contains(n)).collect();
    if !basics.is_empty() {
        let _ = writeln!(prompt, "Drafted basic lands, not rows (they go in `lands`): {}", basics.join(", "));
    }
    let _ = writeln!(prompt, "\n{}", cards.pool_shape(pool));
    prompt.push_str(
        "\n## Building it\n\
         - A deck is at least 40 cards (CR 100.2b); a smaller one is rejected and you are asked again\n\
         - The usual limited build is 17 basic lands and 23 spells from the pool\n\
         - Two colors is the normal build, a third only as a splash you can reliably cast\n\
         - Everything you leave out is your sideboard: before each game of a match after the first you may swap cards between deck and sideboard\n\
         \n## Your answer\n\
         - `maindeck` is one count per numbered row of your pool, in order: how many copies of that card you are playing (0 cuts it)\n\
         - `lands` maps each basic land to how many to add. Basic lands are not drafted and are not limited: they go here, and only here, even if you drafted one\n",
    );
    prompt
}

/// Where a match stands, from the side of the seat deciding between games.
pub struct BetweenGames<'a> {
    /// 1-based number of the game about to be played.
    pub game: usize,
    pub best_of: usize,
    pub your_wins: usize,
    pub their_wins: usize,
    /// How the last game went, from this seat's side: "you lost game 1 on
    /// turn 9". `None` before game 1.
    pub last: Option<String>,
    /// This seat chooses whether to play or draw the next game.
    pub chooses: bool,
    /// The cards the opponent has shown in the match so far.
    pub opponent_showed: &'a [String],
}

/// What a seat decided between games.
pub struct SideboardDecision {
    /// The deck for the next game, when it is not the one being played.
    pub deck: Option<DraftDeck>,
    /// Whether to play first, when the answer said.
    pub play_first: Option<bool>,
    pub prompt: String,
    pub response: String,
    /// Why the answer's deck could not be played, when it could not: the
    /// deck stays as it is.
    pub error: Option<String>,
}

/// Ask a seat between games: the deck for the next game, from its pool,
/// and whether to play first when it chooses. One call, no retry: a deck
/// the answer gets wrong is the deck it was already playing.
pub fn sideboard_with_llm(
    client: &mut DraftLlmClient,
    pool: &[String],
    current: &DraftDeck,
    cards: &CardLines,
    situation: &BetweenGames<'_>,
) -> SideboardDecision {
    let prompt = DraftLlmClient::notes_section(client.notes())
        + &build_sideboard_prompt(pool, current, cards, situation);
    let response = client.send_sideboard_message(&prompt, pool);
    let rows = deckbuilding::deck_rows(pool);
    let play_first = serde_json::from_str::<serde_json::Value>(&response).ok()
        .and_then(|v| v["play_first"].as_bool());
    let parsed = deckbuilding::parse_deck_response_by_rows(&response, &rows)
        .and_then(|(main, lands)| deckbuilding::validate_deck(pool, &main, &lands));
    let (deck, error) = match parsed {
        Ok(deck) if same_deck(&deck, current) => (None, None),
        Ok(deck) => (Some(deck), None),
        Err(e) => (None, Some(e)),
    };
    SideboardDecision { deck, play_first, prompt, response, error }
}

/// "in: Bloodcrazed Neonate; out: Moon Heron; Swamp +1, Island -1": what
/// a sideboard changed, as the log and the table say it.
#[must_use]
pub fn deck_change(before: &DraftDeck, after: &DraftDeck) -> String {
    let count = |cards: &[String]| {
        let mut m: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
        for c in cards {
            *m.entry(mtg_draft::front_face(c).to_string()).or_default() += 1;
        }
        m
    };
    let (b, a) = (count(&before.maindeck), count(&after.maindeck));
    let names: std::collections::BTreeSet<&String> = b.keys().chain(a.keys()).collect();
    let (mut ins, mut outs) = (Vec::new(), Vec::new());
    for name in names {
        let d = a.get(name).copied().unwrap_or(0) - b.get(name).copied().unwrap_or(0);
        let label = |n: i64| if n.abs() == 1 { name.clone() } else { format!("{}x {name}", n.abs()) };
        if d > 0 { ins.push(label(d)); } else if d < 0 { outs.push(label(d)); }
    }
    let mut lands = Vec::new();
    for land in deckbuilding::BASIC_LANDS {
        let d = i64::from(after.lands.get(*land).copied().unwrap_or(0)) - i64::from(before.lands.get(*land).copied().unwrap_or(0));
        if d != 0 {
            lands.push(format!("{land} {d:+}"));
        }
    }
    let mut parts = Vec::new();
    if !ins.is_empty() { parts.push(format!("in: {}", ins.join(", "))); }
    if !outs.is_empty() { parts.push(format!("out: {}", outs.join(", "))); }
    if !lands.is_empty() { parts.push(lands.join(", ")); }
    if parts.is_empty() { "no change".to_string() } else { parts.join("; ") }
}

/// The log's record of a seat's between-games answer: its prompt, its
/// answer, and one line of what it did.
pub fn log_sideboard(seat: usize, game: usize, before: &DraftDeck, decision: &SideboardDecision, chooses: bool) {
    let w = |label: &str, body: &str| mtg_player::game_log::write(file!(), line!(), label, body);
    w(&format!("[Seat {seat}] SIDEBOARD_PROMPT before game {game}"), &decision.prompt);
    w(&format!("[Seat {seat}] SIDEBOARD_RESPONSE before game {game}"), &decision.response);
    let change = match (&decision.deck, &decision.error) {
        (Some(deck), _) => deck_change(before, deck),
        (None, Some(e)) => format!("kept the deck (the answer's could not be played: {e})"),
        (None, None) => "kept the deck".to_string(),
    };
    let choice = match (chooses, decision.play_first) {
        (false, _) => String::new(),
        (true, Some(false)) => "; draws first".to_string(),
        (true, _) => "; plays first".to_string(),
    };
    w(&format!("[Seat {seat}] SIDEBOARD before game {game}: {change}{choice}"), "");
}

/// Two decks with the same cards and lands, in any order.
#[must_use]
pub fn same_deck(a: &DraftDeck, b: &DraftDeck) -> bool {
    let sorted = |d: &DraftDeck| {
        let mut main = d.maindeck.clone();
        main.sort();
        let mut lands: Vec<(String, u32)> = d.lands.iter().filter(|(_, n)| **n > 0).map(|(k, v)| (k.clone(), *v)).collect();
        lands.sort();
        (main, lands)
    };
    sorted(a) == sorted(b)
}

/// The between-games message: where the match stands, what the opponent
/// has shown, and the pool with the counts being played now, answered in
/// the deck build's own shape so the counts can be repeated to change
/// nothing.
#[must_use]
pub fn build_sideboard_prompt(pool: &[String], current: &DraftDeck, cards: &CardLines, situation: &BetweenGames<'_>) -> String {
    let BetweenGames { game, best_of, your_wins, their_wins, .. } = *situation;
    let mut prompt = format!("Between games: game {game} of a best-of-{best_of} is next. You have won {your_wins}, \
your opponent {their_wins}.");
    if let Some(last) = &situation.last {
        let _ = write!(prompt, " Last game: {last}.");
    }
    prompt.push('\n');
    if situation.chooses {
        let why = if game == 1 { "you won the coin flip" } else { "you lost the last game" };
        let _ = writeln!(prompt, "You choose whether to play first or draw first in game {game} ({why}). \
Playing first is the usual choice in Limited; drawing first suits a slow deck that wins the long game.");
    }
    if situation.opponent_showed.is_empty() {
        prompt.push_str("\nYour opponent has shown no cards yet.\n");
    } else {
        let _ = writeln!(prompt, "\nYour opponent has shown ({}):", situation.opponent_showed.len());
        for card in situation.opponent_showed {
            let _ = writeln!(prompt, "- {}", cards.pool_line(card));
        }
    }
    let playing = |name: &str| current.maindeck.iter()
        .filter(|c| mtg_draft::front_face(c) == mtg_draft::front_face(name)).count();
    let _ = writeln!(prompt, "\nYour pool, each row with how many you are playing now ({} cards in the deck):",
        current.total_cards());
    for (i, (name, copies)) in deckbuilding::deck_rows(pool).iter().enumerate() {
        let _ = writeln!(prompt, "{i}: playing {} of {copies}x {}", playing(name), cards.pool_line(name));
    }
    let mut lands: Vec<(&String, &u32)> = current.lands.iter().filter(|(_, n)| **n > 0).collect();
    lands.sort();
    let lands: Vec<String> = lands.iter().map(|(l, n)| format!("{l} {n}")).collect();
    let _ = writeln!(prompt, "Basic lands now: {}", if lands.is_empty() { "none".to_string() } else { lands.join(", ") });
    prompt.push_str(
        "\n## Your answer\n\
         - `maindeck` is one count per numbered row: the deck for the next game. Repeat the counts above to keep the deck as it is; a change lasts to the end of the match\n\
         - `lands` is the basic lands for the next game\n\
         - `play_first`: true to play first, false to draw first; it is read only when you choose\n\
         - The deck stays at least 40 cards; one that is not is ignored and the deck stays as it is\n",
    );
    prompt
}

/// Write a built deck as `COUNT NAME` lines, the format `mtg-runner
/// --deck1 <path>` reads, creating the directory it goes in.
///
/// # Errors
/// The directory cannot be made or the file cannot be written.
pub fn write_deck_file(path: &std::path::Path, deck: &DraftDeck) -> std::io::Result<()> {
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

        // The pool, with what each card costs and is, numbered for the
        // answer's counts (#867); a drafted basic is no row.
        assert!(prompt.contains("0: 1x Chapel Geist"), "{prompt}");
        assert!(prompt.contains("1: 2x Moon Heron {3}{U} | Creature — Spirit Bird 3/2"), "{prompt}");
        assert!(prompt.contains("not rows (they go in `lands`): Plains"), "{prompt}");
        assert!(prompt.contains("Colors"), "{prompt}");
        assert!(prompt.contains("Curve"), "{prompt}");
        // The deck it is asking for.
        assert!(prompt.contains("40 cards"), "{prompt}");
        // A card left out can come in between games (#876): the prompt
        // said nothing was sideboarded, which stopped being true with 1de2aac.
        assert!(prompt.contains("swap cards between deck and sideboard") && !prompt.contains("nothing is sideboarded"), "{prompt}");
        assert!(prompt.contains("17 basic lands and 23 spells"), "{prompt}");
        // The answer's shape, and where a drafted basic land goes.
        assert!(prompt.contains("`maindeck`"), "{prompt}");
        assert!(prompt.contains("`lands`"), "{prompt}");
        assert!(prompt.contains("even if you drafted one"), "{prompt}");
        // The sideboard it is silently creating.
        assert!(prompt.contains("sideboard"), "{prompt}");
    }
}
