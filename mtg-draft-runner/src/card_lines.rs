//! What a drafting seat is shown about a card.
//!
//! A pick is made on colour, cost, size and rarity, and the pack listing
//! used to carry none of them — fourteen bare names, with everything the
//! decision needs scattered through a 44 KB card reference the seat had to
//! cross-reference fourteen times a pick, and rarity nowhere at all in a
//! prompt that tells the seat to value rares (issue #483). These are the
//! lines that carry it, on the pack listing, in the pool and in the
//! deck-building prompt, all rendered through `mtg_player::llm::card_headline`
//! so a card reads the same wherever this workspace shows it.

use std::collections::HashMap;
use std::fmt::Write;

use mtg_draft::set_data::Rarity;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::{CardType, Color, ManaSymbol};

/// One card as a drafter sees it.
struct CardLine {
    /// Name, cost, type line and size — both faces of a double-faced card.
    headline: String,
    rarity: Option<Rarity>,
    colors: Vec<Color>,
    mana_value: u32,
    is_land: bool,
}

/// Every card in the set, as a drafting seat sees it.
pub struct CardLines {
    lines: HashMap<String, CardLine>,
}

impl CardLines {
    /// Describe every card in `names`, which are set-data names and so may
    /// be `"Front // Back"`. Each card is reachable under that name and
    /// under its front face, because a pack, a pool and a decklist do not
    /// agree on which they use.
    #[must_use]
    pub fn new(
        names: &[String],
        rarities: &HashMap<String, Rarity>,
        registry: &CardRegistry,
    ) -> Self {
        let mut lines = HashMap::new();
        // An Innistrad pack has a basic land slot, so a basic land can be
        // drafted and has to describe itself like any other card (#487).
        // The set data's runs do not list them.
        let basics = ["Plains", "Island", "Swamp", "Mountain", "Forest"].map(String::from);
        for name in names.iter().chain(basics.iter()) {
            let faces = mtg_player::llm::card_faces(name, registry);
            let Some((front_name, front)) = faces.first() else { continue };

            // The back of a double-faced card is what the front is drafted
            // for, so its name and size ride along on the same line; the
            // card reference carries the rules text for both (issue #205).
            let mut headline = mtg_player::llm::card_headline(front_name, front);
            if let Some((back_name, back)) = faces.get(1) {
                let size = match (back.power, back.toughness) {
                    (Some(p), Some(t)) => format!(" {p}/{t}"),
                    _ => String::new(),
                };
                write!(headline, " // {back_name}{size}").unwrap();
            }

            let line = CardLine {
                headline,
                rarity: rarities.get(name).copied(),
                colors: cost_colors(front),
                mana_value: front.cost.as_ref().map_or(0, mtg_engine::types::ManaCost::mana_value),
                is_land: front.card_types.contains(&CardType::Land),
            };
            lines.insert(mtg_draft::front_face(name).to_string(), CardLine { ..clone_line(&line) });
            lines.insert(name.clone(), line);
        }
        Self { lines }
    }

    /// A card's line in a pack: everything the pick is made on, rarity
    /// included — in a real pack the rarity is the one thing you can see
    /// without reading the card.
    #[must_use]
    pub fn pack_line(&self, name: &str) -> String {
        match self.lines.get(name) {
            Some(line) => match line.rarity {
                Some(rarity) => format!("{} [{}]", line.headline, rarity.label()),
                None => line.headline.clone(),
            },
            // A card the set data does not describe still has to list.
            None => mtg_draft::front_face(name).to_string(),
        }
    }

    /// A card's line in the seat's own pool, where its rarity no longer
    /// matters but its colour and cost still do.
    #[must_use]
    pub fn pool_line(&self, name: &str) -> String {
        self.lines.get(name).map_or_else(
            || mtg_draft::front_face(name).to_string(),
            |line| line.headline.clone(),
        )
    }

    /// The pool as a drafter reads it: counted lines, plus the colour count
    /// and curve that decide which two colours the deck is and whether it
    /// can cast what it drafts.
    #[must_use]
    pub fn pool_listing(&self, pool: &[String]) -> String {
        let mut counts: Vec<(String, usize)> = Vec::new();
        for card in pool {
            let line = self.pool_line(card);
            match counts.iter_mut().find(|(l, _)| *l == line) {
                Some((_, n)) => *n += 1,
                None => counts.push((line, 1)),
            }
        }
        counts.sort();

        let mut listing = String::new();
        for (line, count) in &counts {
            writeln!(listing, "{count}x {line}").unwrap();
        }
        writeln!(listing, "\n{}", self.pool_shape(pool)).unwrap();
        listing
    }

    /// One line of arithmetic over a pool: how many cards of each colour,
    /// and the mana curve of its non-lands.
    #[must_use]
    pub fn pool_shape(&self, pool: &[String]) -> String {
        let mut by_color: Vec<(Color, usize)> = Vec::new();
        let mut colorless = 0;
        let mut curve: Vec<(u32, usize)> = Vec::new();
        let mut lands = 0;

        for card in pool {
            let Some(line) = self.lines.get(card) else { continue };
            if line.is_land {
                lands += 1;
                continue;
            }
            if line.colors.is_empty() {
                colorless += 1;
            }
            for color in &line.colors {
                match by_color.iter_mut().find(|(c, _)| c == color) {
                    Some((_, n)) => *n += 1,
                    None => by_color.push((*color, 1)),
                }
            }
            match curve.iter_mut().find(|(mv, _)| *mv == line.mana_value) {
                Some((_, n)) => *n += 1,
                None => curve.push((line.mana_value, 1)),
            }
        }

        by_color.sort_by_key(|(color, count)| (std::cmp::Reverse(*count), format!("{color:?}")));
        curve.sort_unstable();

        let mut colors: Vec<String> = by_color
            .iter()
            .map(|(color, count)| format!("{count} {}", color.label().to_lowercase()))
            .collect();
        if colorless > 0 {
            colors.push(format!("{colorless} colorless"));
        }
        if lands > 0 {
            colors.push(format!("{lands} land"));
        }
        let curve: Vec<String> = curve
            .iter()
            .map(|(mana_value, count)| format!("{mana_value}:{count}"))
            .collect();

        format!(
            "Colors (a gold card counts once per color): {}\nCurve (mana value:cards): {}",
            if colors.is_empty() { "none".to_string() } else { colors.join(", ") },
            if curve.is_empty() { "none".to_string() } else { curve.join(" ") },
        )
    }
}

/// A card's colors, as its mana cost states them.
fn cost_colors(data: &mtg_engine::cards::CardData) -> Vec<Color> {
    // A face with no mana cost says its colors with the indicator beside
    // its type line instead (CR 204.2), which the registry carries.
    if data.cost.is_none() {
        return data.color_indicator.clone();
    }
    let mut colors = Vec::new();
    for symbol in data.cost.iter().flat_map(|cost| cost.symbols.iter()) {
        if let ManaSymbol::Colored(color) = symbol {
            if !colors.contains(color) {
                colors.push(*color);
            }
        }
    }
    colors
}

fn clone_line(line: &CardLine) -> CardLine {
    CardLine {
        headline: line.headline.clone(),
        rarity: line.rarity,
        colors: line.colors.clone(),
        mana_value: line.mana_value,
        is_land: line.is_land,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines() -> CardLines {
        let set_data = mtg_draft::set_data::SetData::load(std::path::Path::new(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../data/sets/isd.json"),
        ))
        .expect("ISD set data");
        let registry = CardRegistry::with_all_cards();
        CardLines::new(&set_data.all_card_names(), &set_data.rarities(), &registry)
    }

    /// #483: a pack line carried the name and nothing else — no cost, no
    /// colour, no type, no size, and no rarity anywhere in the prompt.
    #[test]
    fn a_pack_line_carries_what_the_pick_is_made_on() {
        let lines = lines();
        assert_eq!(
            lines.pack_line("Moon Heron"),
            "Moon Heron {3}{U} | Creature — Spirit Bird 3/2 [common]"
        );
        assert_eq!(
            lines.pack_line("Snapcaster Mage"),
            "Snapcaster Mage {1}{U} | Creature — Human Wizard 2/1 [rare]"
        );
    }

    /// A double-faced card is drafted for its back, so the back's name and
    /// size are on the line too, and the card is reachable under either the
    /// set-data name or the front face.
    #[test]
    fn a_double_faced_card_shows_both_faces_under_either_name() {
        let lines = lines();
        let full = lines.pack_line("Delver of Secrets // Insectile Aberration");
        assert_eq!(
            full,
            "Delver of Secrets {U} | Creature — Human Wizard 1/1 // Insectile Aberration 3/2 [common]"
        );
        assert_eq!(lines.pack_line("Delver of Secrets"), full);
    }

    /// The colour count and curve a drafter reads off their pool, which no
    /// prompt stated (#481, #487).
    #[test]
    fn a_pool_is_counted_by_color_and_curve() {
        let lines = lines();
        let pool = vec![
            "Moon Heron".to_string(),
            "Moon Heron".to_string(),
            "Chapel Geist".to_string(),
            "Blazing Torch".to_string(),
            "Plains".to_string(),
        ];

        let listing = lines.pool_listing(&pool);
        assert!(listing.contains("2x Moon Heron {3}{U} | Creature — Spirit Bird 3/2"), "{listing}");
        assert!(listing.contains("1x Chapel Geist {1}{W}{W}"), "{listing}");
        // The rarity is a pack-line thing; a card in hand is already yours.
        assert!(!listing.contains("[common]"), "{listing}");

        let shape = lines.pool_shape(&pool);
        assert!(shape.contains("2 blue"), "{shape}");
        assert!(shape.contains("1 white"), "{shape}");
        assert!(shape.contains("1 colorless"), "{shape}");
        assert!(shape.contains("1 land"), "{shape}");
        // Moon Heron twice at 4, Chapel Geist at 3, Blazing Torch at 1.
        assert!(shape.contains("1:1 3:1 4:2"), "{shape}");
    }
}
