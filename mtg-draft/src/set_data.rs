use mtg_engine::cards::CardRegistry;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// The rarity printed on a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Mythic,
}

impl Rarity {
    /// The word a card's rarity is shown as.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Common => "common",
            Self::Uncommon => "uncommon",
            Self::Rare => "rare",
            Self::Mythic => "mythic",
        }
    }
}

/// Top-level set data loaded from a JSON file (e.g., data/sets/isd.json).
#[derive(Debug, Deserialize)]
pub struct SetData {
    pub set_code: String,
    pub set_name: String,
    pub collation: Collation,
    pub runs: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Collation {
    pub common_pack_variants: CommonPackVariants,
    pub uncommon_variants: Vec<UncommonVariant>,
    pub rare_sheets: RareSheets,
    pub dfc_sheet_runs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommonPackVariants {
    pub c1_variants: Vec<CommonVariant>,
    pub c2_variants: Vec<CommonVariant>,
    pub c1_variant_weights: Vec<u32>,
    pub c2_variant_weights: Vec<u32>,
}

/// A common pack variant specifying how many cards to draw from each run.
/// Fields are optional because C1 variants have `c1` and C2 variants have `c2`.
#[derive(Debug, Deserialize)]
pub struct CommonVariant {
    pub a: usize,
    pub b: usize,
    #[serde(default)]
    pub c1: usize,
    #[serde(default)]
    pub c2: usize,
}

impl CommonVariant {
    #[must_use]
    pub fn c_count(&self) -> usize {
        self.c1 + self.c2
    }
}

#[derive(Debug, Deserialize)]
pub struct UncommonVariant {
    pub a: usize,
    pub b: usize,
    pub weight: u32,
}

#[derive(Debug, Deserialize)]
pub struct RareSheets {
    pub sheet_1_runs: Vec<String>,
    pub sheet_2_runs: Vec<String>,
}

impl SetData {
    /// Load set data from a JSON file.
    ///
    /// # Errors
    /// Returns an error string if the file at `path` cannot be read or if its
    /// contents cannot be parsed as valid `SetData` JSON.
    pub fn load(path: &Path) -> Result<Self, String> {
        let contents =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
    }

    /// Get a run by name, returning an error if it doesn't exist.
    ///
    /// # Errors
    /// Returns an error string if no run with `name` exists in the set data.
    pub fn run(&self, name: &str) -> Result<&[String], String> {
        self.runs
            .get(name)
            .map(std::vec::Vec::as_slice)
            .ok_or_else(|| format!("Run '{name}' not found in set data"))
    }

    /// Build the combined rare sheet 1 sequence (concatenation of its runs).
    ///
    /// # Errors
    /// Returns an error string if any run listed in
    /// `collation.rare_sheets.sheet_1_runs` is not present in the set data.
    pub fn rare_sheet_1(&self) -> Result<Vec<String>, String> {
        let mut sheet = Vec::new();
        for run_name in &self.collation.rare_sheets.sheet_1_runs {
            sheet.extend_from_slice(self.run(run_name)?);
        }
        Ok(sheet)
    }

    /// Build the combined rare sheet 2 sequence.
    ///
    /// # Errors
    /// Returns an error string if any run listed in
    /// `collation.rare_sheets.sheet_2_runs` is not present in the set data.
    pub fn rare_sheet_2(&self) -> Result<Vec<String>, String> {
        let mut sheet = Vec::new();
        for run_name in &self.collation.rare_sheets.sheet_2_runs {
            sheet.extend_from_slice(self.run(run_name)?);
        }
        Ok(sheet)
    }

    /// Build the combined DFC sheet sequence.
    ///
    /// # Errors
    /// Returns an error string if any run listed in
    /// `collation.dfc_sheet_runs` is not present in the set data.
    pub fn dfc_sheet(&self) -> Result<Vec<String>, String> {
        let mut sheet = Vec::new();
        for run_name in &self.collation.dfc_sheet_runs {
            sheet.extend_from_slice(self.run(run_name)?);
        }
        Ok(sheet)
    }

    /// Every card's rarity, recovered from the print sheets it appears on.
    ///
    /// The runs are named by rarity, which settles the common and uncommon
    /// sheets outright. A sheet that mixes rarities — the rare sheet, which
    /// carries mythics, and the double-faced sheet, which carries all four —
    /// separates them by how many slots a card holds: the rarer a card is,
    /// the fewer copies of it the sheet has. So within such a sheet the
    /// distinct copy-counts, most copies first, are common, uncommon, rare,
    /// mythic, and a sheet with fewer distinct counts is read from the
    /// common end (ISD's rare sheet has two, 4 copies and 2; its DFC sheet
    /// has four, 11 / 6 / 2 / 1).
    ///
    /// Cards whose run name names no rarity at all are left out.
    #[must_use]
    pub fn rarities(&self) -> HashMap<String, Rarity> {
        let mut rarities = HashMap::new();

        for (run_name, run) in &self.runs {
            let fixed = if run_name.starts_with("uncommon") {
                Some(Rarity::Uncommon)
            } else if run_name.starts_with("common") {
                Some(Rarity::Common)
            } else {
                None
            };
            if let Some(rarity) = fixed {
                for card in run {
                    rarities.insert(card.clone(), rarity);
                }
            }
        }

        for prefix in ["rare", "dfc"] {
            let mut copies: HashMap<&str, usize> = HashMap::new();
            for (run_name, run) in &self.runs {
                if !run_name.starts_with(prefix) {
                    continue;
                }
                for card in run {
                    *copies.entry(card.as_str()).or_default() += 1;
                }
            }
            if copies.is_empty() {
                continue;
            }

            // The sheet's distinct copy-counts, most copies first: common,
            // then uncommon, then rare, then mythic.
            let mut counts: Vec<usize> = copies.values().copied().collect();
            counts.sort_unstable_by(|a, b| b.cmp(a));
            counts.dedup();
            let ladder = [Rarity::Common, Rarity::Uncommon, Rarity::Rare, Rarity::Mythic];
            // A sheet carrying only the top rarities starts further down the
            // ladder: the rare sheet's two counts are rare and mythic, not
            // common and uncommon.
            let start = if prefix == "rare" { 2 } else { 0 };
            for (card, count) in copies {
                let step = counts.iter().position(|c| *c == count).unwrap_or(0);
                let rarity = ladder[(start + step).min(ladder.len() - 1)];
                rarities.insert(card.to_string(), rarity);
            }
        }

        rarities
    }

    /// Return all unique card names across all runs.
    #[must_use]
    pub fn all_card_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .runs
            .values()
            .flat_map(|run| run.iter())
            .cloned()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        names.sort();
        names
    }

    /// Filter all runs to only contain cards that are implemented in the registry.
    /// Returns a list of card names that were removed.
    pub fn filter_implemented(&mut self, registry: &CardRegistry) -> Vec<String> {
        let mut removed = Vec::new();
        for run in self.runs.values_mut() {
            let before_len = run.len();
            run.retain(|name| {
                // DFC names in the set data use "Front // Back" format.
                // The registry uses just the front face name.
                let lookup_name = crate::front_face(name);
                if registry.get_id_by_name(lookup_name).is_some() {
                    true
                } else {
                    removed.push(name.clone());
                    false
                }
            });
            if run.len() != before_len {
                // Note: removing cards from a run breaks the sequential collation
                // model slightly, but it's better than crashing on unknown cards.
            }
        }
        removed.sort();
        removed.dedup();
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn isd_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("data/sets/isd.json")
    }

    #[test]
    fn test_load_isd() {
        let data = SetData::load(&isd_path()).expect("Failed to load ISD set data");
        assert_eq!(data.set_code, "isd");
        assert_eq!(data.runs["common_a"].len(), 66);
        assert_eq!(data.runs["common_b"].len(), 66);
        assert_eq!(data.runs["common_c1"].len(), 55);
        assert_eq!(data.runs["common_c2"].len(), 55);
        assert_eq!(data.runs["uncommon_a"].len(), 66);
        assert_eq!(data.runs["uncommon_b"].len(), 54);
        assert_eq!(data.runs["dfc_a"].len(), 66);
        assert_eq!(data.runs["dfc_b"].len(), 55);
        assert_eq!(data.runs["rare_a"].len(), 55);
        assert_eq!(data.runs["rare_b"].len(), 55);
        assert_eq!(data.runs["rare_c"].len(), 66);
        assert_eq!(data.runs["rare_d"].len(), 66);
    }

    /// The rarity a pack line shows comes from the print sheets, so it has to
    /// agree with what is printed on the real cards — including on the
    /// double-faced sheet, which carries all four rarities and names none of
    /// them (issue #483).
    #[test]
    fn rarities_match_the_printed_cards() {
        let data = SetData::load(&isd_path()).expect("Failed to load ISD set data");
        let rarities = data.rarities();
        let rarity = |name: &str| {
            *rarities
                .get(name)
                .unwrap_or_else(|| panic!("{name} has no rarity"))
        };

        assert_eq!(rarity("Moon Heron"), Rarity::Common);
        assert_eq!(rarity("Abbey Griffin"), Rarity::Common);
        assert_eq!(rarity("Skaab Goliath"), Rarity::Uncommon);
        assert_eq!(rarity("Burning Vengeance"), Rarity::Uncommon);
        assert_eq!(rarity("Snapcaster Mage"), Rarity::Rare);
        assert_eq!(rarity("Geist-Honored Monk"), Rarity::Rare);
        // A mythic holds half a rare's slots on the sheet.
        assert_eq!(rarity("Liliana of the Veil"), Rarity::Mythic);
        assert_eq!(rarity("Geist of Saint Traft"), Rarity::Mythic);

        // The double-faced sheet: one card of each rarity it carries.
        assert_eq!(rarity("Delver of Secrets // Insectile Aberration"), Rarity::Common);
        assert_eq!(rarity("Village Ironsmith // Ironfang"), Rarity::Common);
        assert_eq!(rarity("Gatstaf Shepherd // Gatstaf Howler"), Rarity::Uncommon);
        assert_eq!(rarity("Bloodline Keeper // Lord of Lineage"), Rarity::Rare);
        assert_eq!(rarity("Garruk Relentless // Garruk, the Veil-Cursed"), Rarity::Mythic);

        // Every card in the set has one.
        for name in data.all_card_names() {
            assert!(rarities.contains_key(&name), "{name} has no rarity");
        }
    }

    #[test]
    fn test_combined_sheets() {
        let data = SetData::load(&isd_path()).expect("Failed to load ISD set data");
        assert_eq!(data.rare_sheet_1().unwrap().len(), 121);
        assert_eq!(data.rare_sheet_2().unwrap().len(), 121);
        assert_eq!(data.dfc_sheet().unwrap().len(), 121);
    }

    #[test]
    fn test_filter_implemented() {
        let mut data = SetData::load(&isd_path()).expect("Failed to load ISD set data");
        let registry = CardRegistry::with_all_cards();
        let removed = data.filter_implemented(&registry);
        // ISD has all cards implemented, so nothing should be removed
        // (DFC names use "Front // Back" format; front face should be in registry)
        assert!(removed.is_empty(),
            "Expected all ISD cards to be implemented, but {} were not found: {:?}",
            removed.len(),
            &removed[..removed.len().min(10)]
        );
    }
}
