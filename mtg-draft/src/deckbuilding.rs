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
    pub fn total_cards(&self) -> usize {
        self.maindeck.len() + self.lands.values().sum::<u32>() as usize
    }
}

const BASIC_LANDS: &[&str] = &["Plains", "Island", "Swamp", "Mountain", "Forest"];

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
            let n = count.as_u64().unwrap_or(0) as u32;
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
                    lands.insert(name.clone(), n as u32);
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
        let name = card.split(" // ").next().unwrap_or(card);
        *pool_counts.entry(name).or_insert(0) += 1;
    }

    // Check maindeck against pool (strip DFC back face names)
    let mut used_counts: HashMap<&str, u32> = HashMap::new();
    for card in maindeck {
        let name = card.split(" // ").next().unwrap_or(card.as_str());
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

    // Compute sideboard (pool cards not in maindeck)
    let mut remaining_pool: Vec<String> = pool.to_vec();
    for card in maindeck {
        if let Some(pos) = remaining_pool.iter().position(|c| {
            let front = c.split(" // ").next().unwrap_or(c);
            front == card
        }) {
            remaining_pool.remove(pos);
        }
    }

    Ok(DraftDeck {
        maindeck: maindeck.to_vec(),
        lands: lands.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        sideboard: remaining_pool,
    })
}

/// Convert a DraftDeck to a Decklist for the game engine.
pub fn to_decklist(deck: &DraftDeck) -> Vec<(String, u32)> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    for name in &deck.maindeck {
        *counts.entry(name.clone()).or_insert(0) += 1;
    }
    let mut entries: Vec<(String, u32)> = counts.into_iter().collect();
    for (land_name, count) in &deck.lands {
        entries.push((land_name.clone(), *count));
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

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
