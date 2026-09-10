use mtg_engine::cards::CardRegistry;
use mtg_engine::types::{Color, ManaSymbol};
use serde::Serialize;
use std::collections::HashMap;

/// A validated draft deck.
#[derive(Debug, Clone, Serialize)]
pub struct DraftDeck {
    pub maindeck: Vec<String>,
    pub lands: HashMap<String, u32>,
    pub sideboard: Vec<String>,
}

impl DraftDeck {
    #[must_use]
    pub fn total_cards(&self) -> usize {
        self.maindeck.len() + self.lands.values().sum::<u32>() as usize
    }
}

const BASIC_LANDS: &[&str] = &["Plains", "Island", "Swamp", "Mountain", "Forest"];

/// The basic land that produces each color, in WUBRG order.
const BASIC_FOR_COLOR: [(Color, &str); 5] = [
    (Color::White, "Plains"),
    (Color::Blue, "Island"),
    (Color::Black, "Swamp"),
    (Color::Red, "Mountain"),
    (Color::Green, "Forest"),
];

/// How many cards the fallback deck aims to play, and how many lands it
/// plays alongside them: the standard 23/17 limited split.
const FALLBACK_SPELLS: usize = 23;
const FALLBACK_LANDS: u32 = 17;

/// The minimum legal deck size (CR 100.2b).
const MIN_DECK_SIZE: usize = 40;

/// The colored mana a card's cost demands, or `None` for a card the
/// registry does not know.
///
/// A card with no cost (a land) and a card whose cost is all generic both
/// come back as an empty set: castable in any deck.
fn color_requirement(name: &str, registry: &CardRegistry) -> Option<Vec<Color>> {
    let id = registry.get_id_by_name(name)?;
    let data = registry.card_data(id)?;
    let mut colors: Vec<Color> = data
        .cost
        .as_ref()
        .map(|cost| {
            cost.symbols
                .iter()
                .filter_map(|sym| match sym {
                    ManaSymbol::Colored(c) => Some(*c),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    // `Color` is not `Ord`, so dedupe in place, keeping cost order.
    let mut seen: Vec<Color> = Vec::new();
    colors.retain(|c| {
        if seen.contains(c) {
            false
        } else {
            seen.push(*c);
            true
        }
    });
    Some(colors)
}

/// Build the deck the runner has to substitute when a seat never produced a
/// valid one.
///
/// This is a last resort, not a deck-building strategy: it exists so a draft
/// that has already spent its picks can still play a round. It used to dump
/// the entire pool into the maindeck behind a hard-coded 9 Island / 8 Swamp,
/// which produced a 59-card five-color pile that could not cast half of
/// itself (issue #200). Instead, pick the two-color pair the pool actually
/// supports, play what that pair can cast, and split the lands by the pips
/// those cards ask for.
///
/// The result is always legal: at least `MIN_DECK_SIZE` cards, every
/// non-land drawn from `pool`, and basics only (CR 100.2b, 100.4).
#[must_use]
pub fn fallback_deck(pool: &[String], registry: &CardRegistry) -> DraftDeck {
    // What each pool card demands. A card the registry does not know is
    // treated as unplayable rather than guessed at.
    let requirements: Vec<Option<Vec<Color>>> = pool
        .iter()
        .map(|card| color_requirement(card, registry))
        .collect();

    // The pair that can cast the most of the pool. Pairs are enumerated in
    // WUBRG order and ties keep the first, so this is deterministic.
    let mut best_pair = (Color::White, Color::Blue);
    let mut best_count = 0usize;
    for (i, (a, _)) in BASIC_FOR_COLOR.iter().enumerate() {
        for (b, _) in BASIC_FOR_COLOR.iter().skip(i + 1) {
            let count = requirements
                .iter()
                .filter(|req| {
                    req.as_ref()
                        .is_some_and(|cs| cs.iter().all(|c| c == a || c == b))
                })
                .count();
            if count > best_count {
                best_count = count;
                best_pair = (*a, *b);
            }
        }
    }

    // Play what the pair can cast, in pick order, up to the usual 23.
    let maindeck: Vec<String> = pool
        .iter()
        .zip(requirements.iter())
        .filter(|(_, req)| {
            req.as_ref()
                .is_some_and(|cs| cs.iter().all(|c| *c == best_pair.0 || *c == best_pair.1))
        })
        .map(|(card, _)| card.clone())
        .take(FALLBACK_SPELLS)
        .collect();

    // Split the lands by the colored pips the chosen cards actually ask
    // for, rather than by a fixed guess. A deck of nothing but colorless
    // cards has no pips to weigh, so it gets the pair's first color.
    let pip_count = |color: Color| -> u32 {
        maindeck
            .iter()
            .filter_map(|card| {
                let id = registry.get_id_by_name(card)?;
                let data = registry.card_data(id)?;
                let cost = data.cost.as_ref()?;
                Some(
                    cost.symbols
                        .iter()
                        .filter(|sym| matches!(sym, ManaSymbol::Colored(c) if *c == color))
                        .count() as u32,
                )
            })
            .sum()
    };
    let (pips_a, pips_b) = (pip_count(best_pair.0), pip_count(best_pair.1));
    let count_a = if pips_a + pips_b == 0 {
        FALLBACK_LANDS
    } else {
        // Round to nearest, so a single off-color pip still buys a land.
        (FALLBACK_LANDS * pips_a).div_ceil(pips_a + pips_b)
    };
    let count_b = FALLBACK_LANDS - count_a;

    let basic_for = |color: Color| -> String {
        BASIC_FOR_COLOR
            .iter()
            .find(|(c, _)| *c == color)
            .map(|(_, name)| (*name).to_string())
            .expect("every color has a basic land")
    };
    let mut lands: HashMap<String, u32> = HashMap::new();
    for (color, count) in [(best_pair.0, count_a), (best_pair.1, count_b)] {
        if count > 0 {
            *lands.entry(basic_for(color)).or_insert(0) += count;
        }
    }

    // A pool too small or too scattered to fill 23 slots still has to make
    // a legal deck; basics are unlimited, so top up with the primary color.
    let short = MIN_DECK_SIZE.saturating_sub(maindeck.len() + FALLBACK_LANDS as usize);
    if short > 0 {
        *lands.entry(basic_for(best_pair.0)).or_insert(0) += short as u32;
    }

    // Everything the fallback did not play is the sideboard, as it would be
    // for a deck a seat built.
    let mut sideboard: Vec<String> = pool.to_vec();
    for card in &maindeck {
        if let Some(pos) = sideboard.iter().position(|c| c == card) {
            sideboard.remove(pos);
        }
    }
    // Front faces, like every deck that comes out of `validate_deck`: the
    // fallback emitted pool names verbatim, so any fallback deck holding a
    // DFC handed the engine, the game's system prompt and the event log a
    // raw `"Front // Back"` that read as a different card from the board
    // and hand lines beside it (#403). Its own sideboard subtraction was
    // consistent, so it never double-counted — but it is the other producer
    // of these strings, and one physical card has one name.
    let maindeck: Vec<String> =
        maindeck.iter().map(|c| crate::front_face(c).to_string()).collect();

    DraftDeck { maindeck, lands, sideboard }
}

/// Parse an LLM's deck building response.
///
/// Preferred format (JSON — card-name → count mapping):
/// ```json
/// {
///   "thoughts": "...",
///   "maindeck": {"Fiend Hunter": 1, "Walking Corpse": 2, "Rebuke": 0},
///   "lands": {"Plains": 9, "Island": 0, "Swamp": 8, "Mountain": 0, "Forest": 0}
/// }
/// ```
///
/// Also accepts the legacy array format for backwards compatibility:
/// ```json
/// {"maindeck": ["Card Name", "Card Name", ...], "lands": {...}}
/// ```
///
/// # Errors
/// Returns an error string if `response` is not valid JSON, if the
/// `maindeck` field is missing or not an object/array, if the resulting
/// maindeck is empty, or if the `lands` field is empty or missing.
pub fn parse_deck_response(response: &str) -> Result<(Vec<String>, HashMap<String, u32>), String> {
    // Strip optional ```json code fences.
    let stripped = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let v: serde_json::Value = serde_json::from_str(stripped)
        .map_err(|e| format!("Failed to parse JSON: {e}"))?;

    // Parse maindeck — either object {name: count} or legacy array [name, ...]
    let maindeck: Vec<String> = if let Some(obj) = v["maindeck"].as_object() {
        // New format: expand {name: count} into repeated names
        let mut cards = Vec::new();
        for (name, count) in obj {
            let n = u32::try_from(count.as_u64().unwrap_or(0)).unwrap_or(u32::MAX);
            for _ in 0..n {
                cards.push(name.clone());
            }
        }
        cards
    } else if let Some(arr) = v["maindeck"].as_array() {
        // Legacy array format
        arr.iter()
            .filter_map(|c| c.as_str().map(std::string::ToString::to_string))
            .collect()
    } else {
        return Err("JSON response missing \"maindeck\" (expected object or array).".to_string());
    };

    // Parse lands
    let mut lands: HashMap<String, u32> = HashMap::new();
    if let Some(lmap) = v["lands"].as_object() {
        for (name, count) in lmap {
            if let Some(n) = count.as_u64() {
                if n > 0 {
                    lands.insert(name.clone(), u32::try_from(n).unwrap_or(u32::MAX));
                }
            }
        }
    }

    if maindeck.is_empty() {
        return Err("Maindeck is empty — include at least some cards with count > 0.".to_string());
    }
    if lands.is_empty() {
        return Err("Lands are empty — include at least some basic lands.".to_string());
    }

    Ok((maindeck, lands))
}

/// Validate a deck built from a draft pool.
///
/// Rules:
/// - All maindeck cards must exist in the pool
/// - No card can appear in maindeck more times than it appears in pool
/// - Total cards (maindeck + lands) must be >= 40
/// - Lands must be basic lands only
///
/// Returns a `DraftDeck` with the sideboard computed, or an error message.
///
/// # Errors
/// Returns an error string if `lands` contains any non-basic land, if a
/// card in `maindeck` is not present in `pool` (or appears more times in
/// the maindeck than in the pool), if any single land count is implausibly
/// large (> 200), or if the total deck size is fewer than 40 cards.
pub fn validate_deck<S: std::hash::BuildHasher>(
    pool: &[String],
    maindeck: &[String],
    lands: &HashMap<String, u32, S>,
) -> Result<DraftDeck, String> {
    // Check lands are basic
    for land_name in lands.keys() {
        if !BASIC_LANDS.contains(&land_name.as_str()) {
            return Err(format!("'{land_name}' is not a basic land. Only Plains, Island, Swamp, Mountain, Forest are allowed."));
        }
    }

    // Count available copies in pool (DFC names use "Front // Back", match on front)
    let mut pool_counts: HashMap<&str, u32> = HashMap::new();
    for card in pool {
        let name = crate::front_face(card);
        *pool_counts.entry(name).or_insert(0) += 1;
    }

    // Check maindeck against pool (strip DFC back face names)
    let mut used_counts: HashMap<&str, u32> = HashMap::new();
    for card in maindeck {
        let name = crate::front_face(card);
        *used_counts.entry(name).or_insert(0) += 1;

        let available = pool_counts.get(name).copied().unwrap_or(0);
        if used_counts[name] > available {
            if available == 0 {
                return Err(format!(
                    "'{name}' is not in your drafted pool."
                ));
            }
            return Err(format!(
                "'{}' appears {} time(s) in your maindeck but you only drafted {} copy/copies.",
                name, used_counts[name], available
            ));
        }
    }

    // Lands must be non-negative (the schema should enforce this, but
    // guard against negative values from malformed responses).
    for (name, count) in lands {
        if *count == 0 {
            continue;
        }
        // Sanity check: reject obviously hallucinated huge numbers.
        // There's no MTG rule capping basic lands, but a 40-card deck
        // can't have more lands than total cards, and pool size is the
        // real upper bound. 200 is a generous hallucination guard.
        if *count > 200 {
            return Err(format!(
                "{name} count is {count} — that's clearly a hallucinated number. A typical limited deck has 16-18 total lands."
            ));
        }
    }

    // Check total deck size — must be at least 40 (MTG rule 100.2b).
    // There is no maximum deck size in limited.
    let land_count: u32 = lands.values().sum();
    let total = maindeck.len() + land_count as usize;
    if total < 40 {
        return Err(format!(
            "Deck has {total} cards (need at least 40). Add more cards or basic lands."
        ));
    }

    // The sideboard is the pool minus the maindeck, which is this function's
    // whole contract — so both sides of the comparison are the card's front
    // face, the way the two checks above already read them. Comparing the
    // pool's front face against the RAW maindeck string meant a deck answer
    // naming a DFC in full matched nothing here, so the card stayed in the
    // remaining pool: the seat maindecked it AND sideboarded it, and 23 + 20
    // came to 43 physical cards out of a 42-card pool (issue #403).
    let mut remaining_pool: Vec<String> = pool.to_vec();
    for card in maindeck {
        let wanted = crate::front_face(card);
        if let Some(pos) = remaining_pool.iter()
            .position(|c| crate::front_face(c) == wanted)
        {
            remaining_pool.remove(pos);
        }
    }

    // The maindeck is recorded by the name the pool knows the card by, so
    // one physical card has one name everywhere downstream. A raw
    // `"Front // Back"` reached the engine, the game's system prompt and the
    // event log, and read as a different card from the board and hand lines
    // beside it (#403).
    Ok(DraftDeck {
        maindeck: maindeck.iter().map(|c| crate::front_face(c).to_string()).collect(),
        lands: lands.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        sideboard: remaining_pool,
    })
}

/// Convert a `DraftDeck` to a Decklist for the game engine.
///
/// The ORDER matters, and is the deck's own: `setup_game` walks these
/// entries to create the objects, pushes them onto the library in this
/// order, and only then shuffles. Same seed applied to a differently
/// ordered list is a different library.
///
/// Both halves used to be handed over in `HashMap` iteration order, which
/// Rust randomises, so `--seed 41` dealt a different opening hand every run
/// while the draft phase in front of it was byte-identical — `play_match`'s
/// comment that "a seeded run replays its games and not only its packs"
/// (#212) was not true of the games. A tournament result could not be
/// re-examined and a crash in the tournament phase could not be re-run
/// (issue #402).
///
/// The maindeck keeps first-appearance order, which is the seat's own pick
/// order, and the lands follow in WUBRG — the order the rest of this module
/// enumerates colors in. Nothing here consults a hash map's iteration
/// order.
#[must_use]
pub fn to_decklist(deck: &DraftDeck) -> Vec<(String, u32)> {
    let mut entries: Vec<(String, u32)> = Vec::new();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for name in &deck.maindeck {
        match counts.get(name.as_str()) {
            Some(&at) => entries[at].1 += 1,
            None => {
                counts.insert(name.as_str(), entries.len());
                entries.push((name.clone(), 1));
            }
        }
    }
    // Basics in WUBRG, then anything else by name, so a land the pool
    // acquires later still lands somewhere fixed.
    let mut lands: Vec<(&String, &u32)> = deck.lands.iter().collect();
    lands.sort_by_key(|(name, _)| {
        let basic = BASIC_FOR_COLOR.iter().position(|(_, b)| b == name);
        (basic.unwrap_or(BASIC_FOR_COLOR.len()), (*name).clone())
    });
    for (land_name, count) in lands {
        entries.push((land_name.clone(), *count));
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The colors the deck's lands can actually produce.
    fn land_colors(deck: &DraftDeck) -> Vec<Color> {
        BASIC_FOR_COLOR
            .iter()
            .filter(|(_, basic)| deck.lands.get(*basic).copied().unwrap_or(0) > 0)
            .map(|(c, _)| *c)
            .collect()
    }

    /// A 42-card pool of the shape a draft leaves behind: weighted toward
    /// one color pair, with a scattering of the other three.
    fn wide_pool() -> Vec<String> {
        let red_green = [
            "Ambush Viper", "Ancient Grudge", "Ashmouth Hound", "Avacyn's Pilgrim",
            "Boneyard Wurm", "Bramblecrush", "Brimstone Volley", "Bloodcrazed Neonate",
            "Caravan Vigil", "Crossway Vampire", "Darkthicket Wolf", "Desperate Ravings",
            "Devil's Play", "Elder of Laurels", "Feral Ridgewolf", "Festerhide Boar",
            "Full Moon's Rise", "Furor of the Bitten", "Geistflame", "Giant Growth",
            "Gnaw to the Bone", "Goblin Piker", "Grave Bramble", "Grizzled Outcasts",
            "Grizzly Bears", "Gutter Grime",
        ];
        // The three colors the fallback used to jam into the maindeck behind
        // hard-coded Islands and Swamps.
        let off_color = [
            "Elder Cathar", "Gallows Warden", "Voiceless Spirit",
            "Curse of the Bloody Tome", "Frightful Delusion", "Claustrophobia",
            "Walking Corpse", "Dead Weight", "Corpse Lunge", "Bump in the Night",
            "Divine Reckoning", "Undead Alchemist", "Mirror-Mad Phantasm",
            "Angelic Overseer", "Sensory Deprivation", "Purify the Grave",
        ];
        red_green
            .iter()
            .chain(off_color.iter())
            .map(|s| (*s).to_string())
            .collect()
    }

    /// Issue #200: the fallback dumped the whole pool behind a hard-coded
    /// 9 Island / 8 Swamp, so most of its maindeck was uncastable.
    #[test]
    fn fallback_deck_only_plays_what_its_lands_can_cast() {
        let registry = CardRegistry::with_all_cards();
        let pool = wide_pool();
        let deck = fallback_deck(&pool, &registry);

        let available = land_colors(&deck);
        for card in &deck.maindeck {
            let needed = color_requirement(card, &registry)
                .unwrap_or_else(|| panic!("{card} is not a known card"));
            for color in needed {
                assert!(
                    available.contains(&color),
                    "{card} needs {color:?} but the deck's lands make {available:?}"
                );
            }
        }
    }

    #[test]
    fn fallback_deck_is_a_legal_limited_deck_drawn_from_the_pool() {
        let registry = CardRegistry::with_all_cards();
        let pool = wide_pool();
        let deck = fallback_deck(&pool, &registry);

        assert!(
            deck.total_cards() >= MIN_DECK_SIZE,
            "a {} card deck is not legal (CR 100.2b)",
            deck.total_cards()
        );
        assert_eq!(
            deck.lands.values().sum::<u32>(),
            FALLBACK_LANDS,
            "a full pool needs no extra basics to reach 40"
        );
        for card in &deck.maindeck {
            assert!(pool.contains(card), "{card} was never drafted");
        }
        for land in deck.lands.keys() {
            assert!(BASIC_LANDS.contains(&land.as_str()), "{land} is not a basic");
        }
    }

    /// A pool too small to fill 23 slots still has to make a legal deck.
    #[test]
    fn fallback_deck_tops_up_a_short_pool_with_basics() {
        let registry = CardRegistry::with_all_cards();
        let pool: Vec<String> = ["Darkthicket Wolf", "Ambush Viper", "Prey Upon"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let deck = fallback_deck(&pool, &registry);
        assert!(deck.total_cards() >= MIN_DECK_SIZE, "deck: {deck:?}");
        assert!(deck.maindeck.len() <= pool.len());
    }

    /// Whatever the fallback leaves out is the sideboard, as for any deck.
    #[test]
    fn fallback_deck_sideboards_the_rest_of_the_pool() {
        let registry = CardRegistry::with_all_cards();
        let pool = wide_pool();
        let deck = fallback_deck(&pool, &registry);
        assert_eq!(deck.maindeck.len() + deck.sideboard.len(), pool.len());
    }

    #[test]
    fn test_parse_deck_response_object_format() {
        let response = r#"{
            "thoughts": "Building UB",
            "maindeck": {"Snapcaster Mage": 1, "Walking Corpse": 2, "Dead Weight": 1, "Rebuke": 0},
            "lands": {"Island": 9, "Swamp": 8}
        }"#;

        let (maindeck, lands) = parse_deck_response(response).unwrap();
        assert_eq!(maindeck.len(), 4); // 1 + 2 + 1 + 0
        assert_eq!(lands["Island"], 9);
        assert_eq!(lands["Swamp"], 8);
    }

    #[test]
    fn test_parse_deck_response_legacy_array() {
        let response = r#"{
            "maindeck": ["Snapcaster Mage", "Dead Weight"],
            "lands": {"Island": 9}
        }"#;

        let (maindeck, lands) = parse_deck_response(response).unwrap();
        assert_eq!(maindeck, vec!["Snapcaster Mage", "Dead Weight"]);
        assert_eq!(lands["Island"], 9);
    }

    #[test]
    fn test_parse_deck_response_zero_counts_excluded() {
        let response = r#"{
            "maindeck": {"Card A": 1, "Card B": 0},
            "lands": {"Island": 9, "Swamp": 0}
        }"#;

        let (maindeck, lands) = parse_deck_response(response).unwrap();
        assert_eq!(maindeck.len(), 1);
        assert!(!lands.contains_key("Swamp"));
    }

    #[test]
    fn test_validate_valid_deck() {
        let pool: Vec<String> = (0..45).map(|i| format!("Card {i}")).collect();
        let maindeck: Vec<String> = (0..23).map(|i| format!("Card {i}")).collect();
        let mut lands = HashMap::new();
        lands.insert("Island".to_string(), 9);
        lands.insert("Swamp".to_string(), 8);

        let deck = validate_deck(&pool, &maindeck, &lands).unwrap();
        assert_eq!(deck.total_cards(), 40);
        assert_eq!(deck.sideboard.len(), 22);
    }

    #[test]
    fn test_validate_too_few_cards() {
        let pool: Vec<String> = (0..45).map(|i| format!("Card {i}")).collect();
        let maindeck: Vec<String> = (0..10).map(|i| format!("Card {i}")).collect();
        let mut lands = HashMap::new();
        lands.insert("Island".to_string(), 5);

        let result = validate_deck(&pool, &maindeck, &lands);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("15 cards"));
    }

    #[test]
    fn test_validate_no_max_deck_size() {
        // MTG rule 100.2b: no maximum deck size in limited
        let pool: Vec<String> = (0..80).map(|i| format!("Card {i}")).collect();
        let maindeck: Vec<String> = (0..80).map(|i| format!("Card {i}")).collect();
        let mut lands = HashMap::new();
        lands.insert("Island".to_string(), 9);

        let result = validate_deck(&pool, &maindeck, &lands);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().total_cards(), 89);
    }

    #[test]
    fn test_validate_card_not_in_pool() {
        let pool = vec!["Card A".to_string(), "Card B".to_string()];
        let maindeck = vec!["Card C".to_string()];
        let mut lands = HashMap::new();
        lands.insert("Island".to_string(), 39);

        let result = validate_deck(&pool, &maindeck, &lands);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Card C"));
    }

    #[test]
    fn test_validate_non_basic_land() {
        let pool = vec!["Card A".to_string()];
        let maindeck = vec!["Card A".to_string()];
        let mut lands = HashMap::new();
        lands.insert("Gavony Township".to_string(), 1);
        lands.insert("Island".to_string(), 38);

        let result = validate_deck(&pool, &maindeck, &lands);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Gavony Township"));
    }

    #[test]
    fn test_to_decklist() {
        let deck = DraftDeck {
            maindeck: vec!["Card A".to_string(), "Card A".to_string(), "Card B".to_string()],
            lands: {
                let mut m = HashMap::new();
                m.insert("Island".to_string(), 9);
                m
            },
            sideboard: vec![],
        };

        let entries = to_decklist(&deck);
        assert!(entries.iter().any(|(n, c)| n == "Card A" && *c == 2));
        assert!(entries.iter().any(|(n, c)| n == "Card B" && *c == 1));
        assert!(entries.iter().any(|(n, c)| n == "Island" && *c == 9));
    }
}

#[cfg(test)]
mod handoff_tests {
    use super::*;

    fn deck(maindeck: &[&str], lands: &[(&str, u32)]) -> DraftDeck {
        DraftDeck {
            maindeck: maindeck.iter().map(|s| (*s).to_string()).collect(),
            lands: lands.iter().map(|(n, c)| ((*n).to_string(), *c)).collect(),
            sideboard: vec![],
        }
    }

    /// Issue #402: `--seed 41` dealt a different opening hand every run.
    ///
    /// The seed reaches the shuffle, but the LIST being shuffled was built
    /// in `HashMap` iteration order — and Rust gives each map its own
    /// random ordering, so two equal decks in one process hand the engine
    /// two different libraries. `setup_game` creates the objects in this
    /// order, pushes them onto the library in this order and only then
    /// shuffles, so the same permutation over a different list is a
    /// different game. The draft phase in front of it was byte-identical.
    ///
    /// Building the same deck twice is the test: pre-fix the two maps
    /// disagree, and no seed or subprocess is needed to see it.
    #[test]
    fn the_decklist_handed_to_the_engine_does_not_depend_on_a_hash_map() {
        let cards = [
            "Abbey Griffin", "Altar's Reap", "Armored Skaab", "Avacynian Priest",
            "Bloodcrazed Neonate", "Curse of the Bloody Tome", "Darkthicket Wolf",
            "Forbidden Alchemy", "Grave Bramble", "Grizzled Outcasts",
            "Hysterical Blindness", "Lantern Spirit", "Moment of Heroism",
            "Moan of the Unhallowed", "Armored Skaab",
        ];
        let lands = [("Island", 9), ("Swamp", 8), ("Plains", 1), ("Forest", 2)];

        let first = to_decklist(&deck(&cards, &lands));
        for _ in 0..16 {
            assert_eq!(to_decklist(&deck(&cards, &lands)), first,
                "two equal decks hand the engine the same list, in the same order");
        }

        // And that order is the deck's own: the maindeck in pick order with
        // duplicates counted where the card first appears, then the lands in
        // WUBRG.
        let names: Vec<&str> = first.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(&names[..3], &["Abbey Griffin", "Altar's Reap", "Armored Skaab"]);
        assert_eq!(&names[names.len() - 4..],
            &["Plains", "Island", "Swamp", "Forest"],
            "lands read WUBRG, whatever order the map holds them in");
        assert_eq!(first.iter().find(|(n, _)| n == "Armored Skaab").unwrap().1, 2,
            "a duplicate is counted, not repeated");
        assert_eq!(first.iter().map(|(_, c)| *c).sum::<u32>(),
            cards.len() as u32 + lands.iter().map(|(_, c)| c).sum::<u32>(),
            "and nothing is lost or gained on the way");
    }

    /// Issue #403: a deck answer naming a DFC "Front // Back" put the same
    /// physical card in the maindeck AND the sideboard.
    ///
    /// The pool census and the maindeck-vs-pool check both read the front
    /// face; the sideboard computation compared the pool's front face
    /// against the raw maindeck string, so the card was never removed from
    /// the remaining pool. 23 maindeck + 20 sideboard = 43 cards out of a
    /// 42-card pool, with one of them in two places at once.
    #[test]
    fn a_card_named_by_both_faces_is_still_one_physical_card() {
        let pool: Vec<String> = [
            "Grizzled Outcasts // Krallenhorde Wantons",
            "Cloistered Youth // Unholy Fiend",
            "Abbey Griffin", "Altar's Reap", "Armored Skaab",
        ].iter().map(|s| (*s).to_string()).collect();

        // The maindeck names one DFC in full and one by its front face:
        // both are the same physical card as the pool entry.
        let maindeck: Vec<String> = [
            "Grizzled Outcasts // Krallenhorde Wantons",
            "Cloistered Youth",
            "Abbey Griffin",
        ].iter().map(|s| (*s).to_string()).collect();
        let lands: HashMap<String, u32> =
            [("Island".to_string(), 20), ("Swamp".to_string(), 17)].into_iter().collect();

        let built = validate_deck(&pool, &maindeck, &lands).expect("a legal deck");
        assert_eq!(built.maindeck.len() + built.sideboard.len(), pool.len(),
            "maindeck {:?} + sideboard {:?} is the pool, once over",
            built.maindeck, built.sideboard);
        assert!(!built.sideboard.iter().any(|c| crate::front_face(c) == "Grizzled Outcasts"),
            "a maindecked card is not also in the sideboard: {:?}", built.sideboard);
        assert!(!built.sideboard.iter().any(|c| crate::front_face(c) == "Cloistered Youth"),
            "{:?}", built.sideboard);

        // And the deck records one name per physical card, so the engine,
        // the game's system prompt and the event log agree with the board.
        assert!(built.maindeck.iter().all(|c| !c.contains(" // ")),
            "the maindeck is recorded by front face: {:?}", built.maindeck);
        assert!(built.maindeck.contains(&"Grizzled Outcasts".to_string()));
    }
}
