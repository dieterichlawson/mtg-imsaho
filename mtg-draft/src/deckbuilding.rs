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
/// Preferred format (JSON):
/// ```json
/// {
///   "thoughts": "...",
///   "maindeck": ["Card Name", "Card Name", ...],
///   "lands": {"Plains": 0, "Island": 9, "Swamp": 8, "Mountain": 0, "Forest": 0}
/// }
/// ```
///
/// Legacy text format (kept as a fallback for robustness):
/// ```text
/// MAINDECK:
/// Card Name
/// Card Name
/// ...
/// LANDS:
/// 9 Island
/// 8 Swamp
/// ```
pub fn parse_deck_response(response: &str) -> Result<(Vec<String>, HashMap<String, u32>), String> {
    // Primary path: JSON. Strip optional ```json code fences first.
    let stripped = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(stripped) {
        let maindeck: Vec<String> = v["maindeck"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
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
            return Err("JSON response missing or empty \"maindeck\" array.".to_string());
        }
        if lands.is_empty() {
            return Err("JSON response missing or empty \"lands\" object.".to_string());
        }
        return Ok((maindeck, lands));
    }

    // Legacy text format fallback.
    let mut maindeck = Vec::new();
    let mut lands = HashMap::new();

    #[derive(PartialEq)]
    enum Section {
        None,
        Maindeck,
        Lands,
    }

    let mut section = Section::None;

    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let upper = trimmed.to_uppercase();
        if upper.starts_with("MAINDECK") {
            section = Section::Maindeck;
            continue;
        }
        if upper.starts_with("LAND") {
            section = Section::Lands;
            continue;
        }

        match section {
            Section::Maindeck => {
                // Accept "Card Name" or "1 Card Name" or "1x Card Name"
                let card = parse_card_line(trimmed);
                if !card.is_empty() {
                    maindeck.push(card);
                }
            }
            Section::Lands => {
                // Expected: "9 Island" or "Island 9"
                if let Some((name, count)) = parse_land_line(trimmed) {
                    *lands.entry(name).or_insert(0) += count;
                }
            }
            Section::None => {
                // Skip lines before any section header
            }
        }
    }

    if maindeck.is_empty() {
        return Err("No maindeck cards found. Expected JSON `maindeck` array or a MAINDECK: section.".to_string());
    }
    if lands.is_empty() {
        return Err("No lands found. Expected JSON `lands` object or a LANDS: section.".to_string());
    }

    Ok((maindeck, lands))
}

/// Parse a card line, stripping leading count prefixes like "1 " or "1x ".
fn parse_card_line(line: &str) -> String {
    let trimmed = line.trim();

    // Try stripping "N " or "Nx " prefix
    if let Some(first_space) = trimmed.find(' ') {
        let prefix = &trimmed[..first_space];
        let rest = trimmed[first_space..].trim();
        // Check if prefix is a number or "Nx"
        let is_count = prefix.parse::<u32>().is_ok()
            || (prefix.ends_with('x')
                && prefix[..prefix.len() - 1].parse::<u32>().is_ok());
        if is_count && !rest.is_empty() {
            return rest.to_string();
        }
    }

    trimmed.to_string()
}

/// Parse a land line like "9 Island" or "Island 9".
fn parse_land_line(line: &str) -> Option<(String, u32)> {
    let trimmed = line.trim();
    let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return None;
    }

    // Try "9 Island" format
    if let Ok(count) = parts[0].parse::<u32>() {
        let name = parts[1].trim().to_string();
        return Some((name, count));
    }

    // Try "Island 9" format
    if let Ok(count) = parts[1].trim().parse::<u32>() {
        let name = parts[0].trim().to_string();
        return Some((name, count));
    }

    None
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
pub fn validate_deck(
    pool: &[String],
    maindeck: &[String],
    lands: &HashMap<String, u32>,
) -> Result<DraftDeck, String> {
    // Check lands are basic
    for land_name in lands.keys() {
        if !BASIC_LANDS.contains(&land_name.as_str()) {
            return Err(format!("'{}' is not a basic land. Only Plains, Island, Swamp, Mountain, Forest are allowed.", land_name));
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
            // Try matching against DFC front face
            let front_match = pool_counts.keys().find(|&&k| k == name);
            if front_match.is_none() {
                return Err(format!(
                    "'{}' is not in your drafted pool.",
                    name
                ));
            }
        }
    }

    // Check no single basic land count is absurd. Limited decks rarely
    // have more than ~17 basic lands of a single type and never more
    // than ~25. The model has been observed to hallucinate huge
    // integers (e.g. 76,543,210 Swamps) under high thinking, so reject
    // anything that's clearly not a real Magic deck.
    const MAX_PER_BASIC: u32 = 25;
    for (name, count) in lands {
        if *count > MAX_PER_BASIC {
            return Err(format!(
                "{} count is {} — basic lands must be 0..{}. Pick a sensible number for a 40-card limited deck (most decks have 16-18 total lands).",
                name, count, MAX_PER_BASIC
            ));
        }
    }

    // Check total deck size — must be at least 40, no more than 60.
    // The 60-card cap rejects absurd outputs (e.g. 66 Swamps in a
    // 27-card maindeck) without being so tight that it bites legitimate
    // splash builds.
    const MIN_TOTAL: usize = 40;
    const MAX_TOTAL: usize = 60;
    let land_count: u32 = lands.values().sum();
    let total = maindeck.len() + land_count as usize;
    if total < MIN_TOTAL {
        return Err(format!(
            "Deck has {} cards (need at least {}). Add more cards or basic lands.",
            total, MIN_TOTAL
        ));
    }
    if total > MAX_TOTAL {
        return Err(format!(
            "Deck has {} cards (must be {}..{}). Trim spells or basic lands — a typical limited deck is exactly 40 cards.",
            total, MIN_TOTAL, MAX_TOTAL
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
        lands: lands.clone(),
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
    fn test_parse_deck_response() {
        let response = r#"
I'll build a UB deck.

MAINDECK:
Snapcaster Mage
Delver of Secrets
Dead Weight
Victim of Night
Silent Departure
Stitched Drake
Makeshift Mauler
Forbidden Alchemy
Think Twice
Claustrophobia
Moon Heron
Deranged Assistant
Morkrut Banshee
Abattoir Ghoul
Diregraf Ghoul
Walking Corpse
Moan of the Unhallowed
Ghoulraiser
Typhoid Rats
Vampire Interloper
Screeching Bat
Altar's Reap
Sensory Deprivation

LANDS:
9 Island
8 Swamp
"#;

        let (maindeck, lands) = parse_deck_response(response).unwrap();
        assert_eq!(maindeck.len(), 23);
        assert_eq!(lands["Island"], 9);
        assert_eq!(lands["Swamp"], 8);
    }

    #[test]
    fn test_parse_with_counts() {
        let response = "MAINDECK:\n1 Snapcaster Mage\n1x Dead Weight\n\nLANDS:\n9 Island\n";
        let (maindeck, lands) = parse_deck_response(response).unwrap();
        assert_eq!(maindeck, vec!["Snapcaster Mage", "Dead Weight"]);
        assert_eq!(lands["Island"], 9);
    }

    #[test]
    fn test_validate_valid_deck() {
        let pool: Vec<String> = (0..45).map(|i| format!("Card {}", i)).collect();
        let maindeck: Vec<String> = (0..23).map(|i| format!("Card {}", i)).collect();
        let mut lands = HashMap::new();
        lands.insert("Island".to_string(), 9);
        lands.insert("Swamp".to_string(), 8);

        let deck = validate_deck(&pool, &maindeck, &lands).unwrap();
        assert_eq!(deck.total_cards(), 40);
        assert_eq!(deck.sideboard.len(), 22);
    }

    #[test]
    fn test_validate_too_few_cards() {
        let pool: Vec<String> = (0..45).map(|i| format!("Card {}", i)).collect();
        let maindeck: Vec<String> = (0..10).map(|i| format!("Card {}", i)).collect();
        let mut lands = HashMap::new();
        lands.insert("Island".to_string(), 5);

        let result = validate_deck(&pool, &maindeck, &lands);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("15 cards"));
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
