//! Sequential collation pack generator for ISD booster packs.
//! See docs/isd-booster-collation.md for full details on the simulation model.

use crate::set_data::SetData;
use rand::Rng;
use serde::Serialize;

/// A generated booster pack.
#[derive(Debug, Clone, Serialize)]
pub struct BoosterPack {
    pub commons: Vec<String>,
    pub uncommons: Vec<String>,
    pub rare: String,
    pub dfc: String,
    pub foil: Option<Foil>,
}

impl BoosterPack {
    /// All draft-relevant cards in this pack.
    pub fn all_cards(&self) -> Vec<String> {
        let mut cards = self.commons.clone();
        cards.extend(self.uncommons.iter().cloned());
        cards.push(self.rare.clone());
        cards.push(self.dfc.clone());
        if let Some(foil) = &self.foil {
            match foil {
                Foil::NonDfc { card, .. } => cards.push(card.clone()),
                Foil::Dfc { card, .. } => {
                    // DFC foil replaces the normal DFC, which is already in `dfc`.
                    // The foil DFC replaces it, so swap.
                    let len = cards.len();
                    cards[len - 1] = card.clone();
                }
            }
        }
        cards
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum Foil {
    NonDfc {
        card: String,
        rarity: FoilRarity,
        displaced_run: String,
    },
    Dfc {
        card: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum FoilRarity {
    Common,
    Uncommon,
    Rare,
    Mythic,
    BasicLand,
}

/// Mutable state for sequential collation simulation.
/// Each cursor advances through its circular run independently.
pub struct CollationState {
    pub common_a_cursor: usize,
    pub common_b_cursor: usize,
    pub common_c1_cursor: usize,
    pub common_c2_cursor: usize,
    pub uncommon_a_cursor: usize,
    pub uncommon_b_cursor: usize,
    pub rare_sheet1_cursor: usize,
    pub rare_sheet2_cursor: usize,
    pub dfc_cursor: usize,
    pub pack_index: usize,
}

impl CollationState {
    /// Initialize with random cursor positions (simulates a fresh booster box).
    pub fn new_random(rng: &mut impl Rng) -> Self {
        Self {
            common_a_cursor: rng.gen_range(0..66),
            common_b_cursor: rng.gen_range(0..66),
            common_c1_cursor: rng.gen_range(0..55),
            common_c2_cursor: rng.gen_range(0..55),
            uncommon_a_cursor: rng.gen_range(0..66),
            uncommon_b_cursor: rng.gen_range(0..54),
            rare_sheet1_cursor: rng.gen_range(0..121),
            rare_sheet2_cursor: rng.gen_range(0..121),
            dfc_cursor: rng.gen_range(0..121),
            pack_index: 0,
        }
    }
}

/// Pre-computed sheet data ready for pack generation.
pub struct SheetData {
    pub common_a: Vec<String>,
    pub common_b: Vec<String>,
    pub common_c1: Vec<String>,
    pub common_c2: Vec<String>,
    pub uncommon_a: Vec<String>,
    pub uncommon_b: Vec<String>,
    pub rare_sheet1: Vec<String>,
    pub rare_sheet2: Vec<String>,
    pub dfc_sheet: Vec<String>,

    // All non-DFC cards grouped by rarity, for foil selection.
    // These are deduped card names (not sheet positions).
    pub foil_pool_commons: Vec<String>,
    pub foil_pool_uncommons: Vec<String>,
    pub foil_pool_rares: Vec<String>,
    pub foil_pool_mythics: Vec<String>,
    pub foil_pool_basics: Vec<String>,

    // C1 variant weights [2B_variant, 1B_variant]
    pub c1_variant_weights: [u32; 2],
    // C2 variant weights [3A_variant, 4A_variant]
    pub c2_variant_weights: [u32; 2],
    // Uncommon variant weights [2A1B, 1A2B]
    pub uncommon_weights: [u32; 2],

    // C1 variants: [(a_count, b_count, c_count), ...]
    pub c1_variants: [(usize, usize, usize); 2],
    // C2 variants: [(a_count, b_count, c_count), ...]
    pub c2_variants: [(usize, usize, usize); 2],
}

impl SheetData {
    /// Build sheet data from loaded set data.
    pub fn from_set_data(data: &SetData) -> Result<Self, String> {
        let c1v = &data.collation.common_pack_variants.c1_variants;
        let c2v = &data.collation.common_pack_variants.c2_variants;
        let c1w = &data.collation.common_pack_variants.c1_variant_weights;
        let c2w = &data.collation.common_pack_variants.c2_variant_weights;
        let uw = &data.collation.uncommon_variants;

        // Build foil pools by examining which cards appear on which runs.
        // Commons: common_a, common_b, common_c1, common_c2 (deduped)
        // Uncommons: uncommon_a, uncommon_b (deduped)
        // Rares/Mythics: from rare sheets. Mythics appear 2x on sheet, rares appear 4x.
        // We identify mythics by having exactly 2 copies in the rare_a run.
        let rare_sheet1 = data.rare_sheet_1()?;
        let rare_sheet2 = data.rare_sheet_2()?;

        let mut common_set = std::collections::HashSet::new();
        for run_name in ["common_a", "common_b", "common_c1", "common_c2"] {
            for card in data.run(run_name)? {
                common_set.insert(card.clone());
            }
        }

        let mut uncommon_set = std::collections::HashSet::new();
        for run_name in ["uncommon_a", "uncommon_b"] {
            for card in data.run(run_name)? {
                uncommon_set.insert(card.clone());
            }
        }

        // Identify mythics vs rares from the A rare run.
        // Mythics appear 2x in A run (55 slots), rares appear 4x.
        let rare_a = data.run("rare_a")?;
        let mut rare_a_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for card in rare_a {
            *rare_a_counts.entry(card.as_str()).or_default() += 1;
        }

        let mut mythic_set = std::collections::HashSet::new();
        let mut rare_set = std::collections::HashSet::new();

        // Cards in A run with 2 copies are mythics
        for (card, count) in &rare_a_counts {
            if *count == 2 {
                mythic_set.insert(card.to_string());
            } else {
                rare_set.insert(card.to_string());
            }
        }

        // All other cards on rare sheets are rares
        let all_rare_sheets = [&rare_sheet1, &rare_sheet2];
        for sheet in &all_rare_sheets {
            for card in sheet.iter() {
                if !mythic_set.contains(card) {
                    rare_set.insert(card.clone());
                }
            }
        }

        let mut foil_pool_commons: Vec<String> = common_set.into_iter().collect();
        let mut foil_pool_uncommons: Vec<String> = uncommon_set.into_iter().collect();
        let mut foil_pool_rares: Vec<String> = rare_set.into_iter().collect();
        let mut foil_pool_mythics: Vec<String> = mythic_set.into_iter().collect();
        foil_pool_commons.sort();
        foil_pool_uncommons.sort();
        foil_pool_rares.sort();
        foil_pool_mythics.sort();

        let foil_pool_basics = vec![
            "Plains".to_string(),
            "Island".to_string(),
            "Swamp".to_string(),
            "Mountain".to_string(),
            "Forest".to_string(),
        ];

        Ok(Self {
            common_a: data.run("common_a")?.to_vec(),
            common_b: data.run("common_b")?.to_vec(),
            common_c1: data.run("common_c1")?.to_vec(),
            common_c2: data.run("common_c2")?.to_vec(),
            uncommon_a: data.run("uncommon_a")?.to_vec(),
            uncommon_b: data.run("uncommon_b")?.to_vec(),
            rare_sheet1,
            rare_sheet2,
            dfc_sheet: data.dfc_sheet()?,
            foil_pool_commons,
            foil_pool_uncommons,
            foil_pool_rares,
            foil_pool_mythics,
            foil_pool_basics,
            c1_variant_weights: [c1w[0], c1w[1]],
            c2_variant_weights: [c2w[0], c2w[1]],
            uncommon_weights: [uw[0].weight, uw[1].weight],
            c1_variants: [
                (c1v[0].a, c1v[0].b, c1v[0].c_count()),
                (c1v[1].a, c1v[1].b, c1v[1].c_count()),
            ],
            c2_variants: [
                (c2v[0].a, c2v[0].b, c2v[0].c_count()),
                (c2v[1].a, c2v[1].b, c2v[1].c_count()),
            ],
        })
    }
}

/// Take `count` consecutive cards from a circular run, advancing the cursor.
fn take_from_run(run: &[String], cursor: &mut usize, count: usize) -> Vec<String> {
    let len = run.len();
    let mut cards = Vec::with_capacity(count);
    for _ in 0..count {
        cards.push(run[*cursor % len].clone());
        *cursor += 1;
    }
    cards
}

/// Pick a weighted random index given weights.
fn weighted_pick(rng: &mut impl Rng, weights: &[u32]) -> usize {
    let total: u32 = weights.iter().sum();
    let roll = rng.gen_range(0..total);
    let mut cumulative = 0;
    for (i, &w) in weights.iter().enumerate() {
        cumulative += w;
        if roll < cumulative {
            return i;
        }
    }
    weights.len() - 1
}

/// Generate one booster pack using sequential collation.
pub fn generate_pack(
    sheets: &SheetData,
    state: &mut CollationState,
    rng: &mut impl Rng,
) -> BoosterPack {
    let is_c1 = state.pack_index % 2 == 0;
    let use_rare_sheet1 = state.pack_index % 2 == 0;

    // 1. Draw commons
    let (n_a, n_b, n_c) = if is_c1 {
        let variant = weighted_pick(rng, &sheets.c1_variant_weights);
        sheets.c1_variants[variant]
    } else {
        let variant = weighted_pick(rng, &sheets.c2_variant_weights);
        sheets.c2_variants[variant]
    };

    let mut commons = Vec::with_capacity(9);
    commons.extend(take_from_run(&sheets.common_a, &mut state.common_a_cursor, n_a));
    commons.extend(take_from_run(&sheets.common_b, &mut state.common_b_cursor, n_b));
    if is_c1 {
        commons.extend(take_from_run(&sheets.common_c1, &mut state.common_c1_cursor, n_c));
    } else {
        commons.extend(take_from_run(&sheets.common_c2, &mut state.common_c2_cursor, n_c));
    }

    // 2. Draw uncommons
    let unc_variant = weighted_pick(rng, &sheets.uncommon_weights);
    let (n_ua, n_ub) = if unc_variant == 0 { (2, 1) } else { (1, 2) };
    let mut uncommons = Vec::with_capacity(3);
    uncommons.extend(take_from_run(&sheets.uncommon_a, &mut state.uncommon_a_cursor, n_ua));
    uncommons.extend(take_from_run(&sheets.uncommon_b, &mut state.uncommon_b_cursor, n_ub));

    // 3. Draw rare
    let rare = if use_rare_sheet1 {
        take_from_run(&sheets.rare_sheet1, &mut state.rare_sheet1_cursor, 1)
            .into_iter()
            .next()
            .unwrap()
    } else {
        take_from_run(&sheets.rare_sheet2, &mut state.rare_sheet2_cursor, 1)
            .into_iter()
            .next()
            .unwrap()
    };

    // 4. Draw DFC
    let dfc = take_from_run(&sheets.dfc_sheet, &mut state.dfc_cursor, 1)
        .into_iter()
        .next()
        .unwrap();

    // 5. Handle foils (9/40 chance = 22.5%)
    let foil = if rng.gen_ratio(9, 40) {
        Some(generate_foil(sheets, rng, is_c1, &mut commons))
    } else {
        None
    };

    state.pack_index += 1;

    BoosterPack {
        commons,
        uncommons,
        rare,
        dfc,
        foil,
    }
}

/// Generate a foil card and apply displacement to the commons.
fn generate_foil(
    sheets: &SheetData,
    rng: &mut impl Rng,
    is_c1: bool,
    commons: &mut Vec<String>,
) -> Foil {
    // ~14% chance of DFC foil (1 DFC sheet out of ~7 total print sheets)
    if rng.gen_ratio(1, 7) {
        // DFC foil: pick a random DFC weighted by sheet position.
        // Simplification: pick a random position on the 121-card DFC sheet.
        let pos = rng.gen_range(0..sheets.dfc_sheet.len());
        let card = sheets.dfc_sheet[pos].clone();
        return Foil::Dfc { card };
    }

    // Non-DFC foil: determine rarity.
    // 11/16 common, 3/16 uncommon, 7/128 rare, 1/128 mythic, 1/16 basic land
    //
    // Common/basic foils only appear in C1 packs. If this is a C2 pack and we
    // roll common/basic, skip the foil entirely (those foils don't occur in C2).
    let roll: u32 = rng.gen_range(0..128);
    let (rarity, pool) = if roll < 88 {
        (FoilRarity::Common, &sheets.foil_pool_commons)
    } else if roll < 112 {
        (FoilRarity::Uncommon, &sheets.foil_pool_uncommons)
    } else if roll < 119 {
        (FoilRarity::Rare, &sheets.foil_pool_rares)
    } else if roll < 120 {
        (FoilRarity::Mythic, &sheets.foil_pool_mythics)
    } else {
        (FoilRarity::BasicLand, &sheets.foil_pool_basics)
    };

    // Common/basic foils only appear in C1 packs
    if !is_c1 && matches!(rarity, FoilRarity::Common | FoilRarity::BasicLand) {
        // No foil in this pack — common/basic foils can't appear in C2 packs.
        // Treat as uncommon foil instead (displaces B common, works in any pack).
        let card = sheets.foil_pool_uncommons[rng.gen_range(0..sheets.foil_pool_uncommons.len())].clone();
        remove_from_run(commons, "b");
        return Foil::NonDfc {
            card,
            rarity: FoilRarity::Uncommon,
            displaced_run: "b".to_string(),
        };
    }

    let card = pool[rng.gen_range(0..pool.len())].clone();

    let displaced_run = match rarity {
        FoilRarity::Common | FoilRarity::BasicLand => {
            // Displaces C1 common (only in C1 packs, verified above).
            commons.pop();
            "c1".to_string()
        }
        FoilRarity::Uncommon => {
            remove_from_run(commons, "b");
            "b".to_string()
        }
        FoilRarity::Rare | FoilRarity::Mythic => {
            remove_from_run(commons, "a");
            "a".to_string()
        }
    };

    Foil::NonDfc {
        card,
        rarity,
        displaced_run,
    }
}

/// Remove one common from the beginning of the commons list (A-run cards are first,
/// then B-run, then C-run in the pack assembly order).
fn remove_from_run(commons: &mut Vec<String>, _run: &str) {
    // In pack assembly, commons are ordered: A cards first, then B, then C.
    // Displacing an A common means removing from the front.
    // Displacing a B common means removing from after the A cards.
    // For simplicity, we just remove the first card (A displacement) or
    // a middle card (B displacement). The exact position matters less than
    // the count being correct.
    if !commons.is_empty() {
        match _run {
            "a" => {
                commons.remove(0);
            }
            "b" => {
                // B cards follow A cards. Find the approximate boundary.
                // A cards are typically 2-4, so remove from that area.
                let pos = commons.len().min(3); // approximate B start
                if pos < commons.len() {
                    commons.remove(pos);
                } else {
                    commons.pop();
                }
            }
            _ => {
                commons.pop();
            }
        }
    }
}

/// Generate all packs for a draft pod.
/// Returns packs grouped by player: packs[player_seat][pack_number].
pub fn generate_draft_packs(
    sheets: &SheetData,
    pod_size: usize,
    rng: &mut impl Rng,
) -> Vec<Vec<BoosterPack>> {
    let total_packs = pod_size * 3;
    let mut state = CollationState::new_random(rng);

    // Generate all packs sequentially (simulating one booster box)
    let all_packs: Vec<BoosterPack> = (0..total_packs)
        .map(|_| generate_pack(sheets, &mut state, rng))
        .collect();

    // Distribute: packs 0..pod_size are pack 1 for each player,
    // pod_size..2*pod_size are pack 2, etc.
    let mut by_player: Vec<Vec<BoosterPack>> = (0..pod_size).map(|_| Vec::with_capacity(3)).collect();
    for (i, pack) in all_packs.into_iter().enumerate() {
        let player = i % pod_size;
        by_player[player].push(pack);
    }

    by_player
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn load_sheets() -> SheetData {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("data/sets/isd.json");
        let data = SetData::load(&path).expect("Failed to load ISD set data");
        SheetData::from_set_data(&data).expect("Failed to build sheet data")
    }

    #[test]
    fn test_sheet_data_construction() {
        let sheets = load_sheets();
        assert_eq!(sheets.common_a.len(), 66);
        assert_eq!(sheets.common_b.len(), 66);
        assert_eq!(sheets.common_c1.len(), 55);
        assert_eq!(sheets.common_c2.len(), 55);
        assert_eq!(sheets.uncommon_a.len(), 66);
        assert_eq!(sheets.uncommon_b.len(), 54);
        assert_eq!(sheets.rare_sheet1.len(), 121);
        assert_eq!(sheets.rare_sheet2.len(), 121);
        assert_eq!(sheets.dfc_sheet.len(), 121);
        assert_eq!(sheets.foil_pool_mythics.len(), 15);
        assert_eq!(sheets.foil_pool_basics.len(), 5);
    }

    #[test]
    fn test_single_pack_structure() {
        let sheets = load_sheets();
        let mut rng = rand::thread_rng();
        let mut state = CollationState::new_random(&mut rng);

        let pack = generate_pack(&sheets, &mut state, &mut rng);

        // Without foil: 9 commons, 3 uncommons, 1 rare, 1 DFC = 14 cards
        // With foil: 8 commons + 1 foil, 3 uncommons, 1 rare, 1 DFC = still 14
        let total = pack.all_cards().len();
        assert!(
            total == 14 || total == 15,
            "Pack should have 14-15 cards, got {}",
            total
        );

        // Should have 3 uncommons always
        assert_eq!(pack.uncommons.len(), 3);

        // Commons should be 8 or 9 depending on foil
        if pack.foil.is_some() {
            match &pack.foil {
                Some(Foil::NonDfc { .. }) => {
                    assert!(
                        pack.commons.len() == 8,
                        "Non-DFC foil pack should have 8 commons, got {}",
                        pack.commons.len()
                    );
                }
                Some(Foil::Dfc { .. }) => {
                    assert_eq!(pack.commons.len(), 9, "DFC foil pack should have 9 commons");
                }
                None => unreachable!(),
            }
        } else {
            assert_eq!(pack.commons.len(), 9);
        }
    }

    #[test]
    fn test_generate_many_packs_no_panic() {
        let sheets = load_sheets();
        let mut rng = rand::thread_rng();
        let mut state = CollationState::new_random(&mut rng);

        // Generate a full booster box worth of packs
        for _ in 0..36 {
            let pack = generate_pack(&sheets, &mut state, &mut rng);
            assert!(pack.commons.len() >= 8 && pack.commons.len() <= 9);
            assert_eq!(pack.uncommons.len(), 3);
        }
    }

    #[test]
    fn test_draft_packs_distribution() {
        let sheets = load_sheets();
        let mut rng = rand::thread_rng();

        let packs = generate_draft_packs(&sheets, 8, &mut rng);
        assert_eq!(packs.len(), 8, "Should have packs for 8 players");
        for player_packs in &packs {
            assert_eq!(player_packs.len(), 3, "Each player should get 3 packs");
        }
    }

    #[test]
    fn test_sequential_collation_produces_adjacent_cards() {
        let sheets = load_sheets();
        let mut rng = rand::thread_rng();
        let mut state = CollationState::new_random(&mut rng);

        // Force a specific cursor position for A run
        state.common_a_cursor = 0;

        // Generate a C1 pack (even index) that uses 2 or 3 A cards.
        state.pack_index = 0; // C1
        let pack = generate_pack(&sheets, &mut state, &mut rng);

        // The first 2-3 commons should be consecutive cards from common_a
        // (starting from position 0)
        let a_run = &sheets.common_a;
        let first_common = &pack.commons[0];
        assert_eq!(
            first_common, &a_run[0],
            "First A-run common should match sheet position 0"
        );
        if pack.commons.len() > 1 {
            assert_eq!(
                &pack.commons[1], &a_run[1],
                "Second common should match sheet position 1"
            );
        }
    }

    #[test]
    fn test_foil_pool_sizes() {
        let sheets = load_sheets();
        // ISD has 101 commons, 60 uncommons, 53 rares, 15 mythics
        assert_eq!(sheets.foil_pool_commons.len(), 101);
        assert_eq!(sheets.foil_pool_uncommons.len(), 60);
        assert_eq!(sheets.foil_pool_rares.len(), 53);
        assert_eq!(sheets.foil_pool_mythics.len(), 15);
    }
}
