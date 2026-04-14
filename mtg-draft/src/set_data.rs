use mtg_engine::cards::CardRegistry;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

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
    pub fn load(path: &Path) -> Result<Self, String> {
        let contents =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
    }

    /// Get a run by name, returning an error if it doesn't exist.
    pub fn run(&self, name: &str) -> Result<&[String], String> {
        self.runs
            .get(name)
            .map(std::vec::Vec::as_slice)
            .ok_or_else(|| format!("Run '{name}' not found in set data"))
    }

    /// Build the combined rare sheet 1 sequence (concatenation of its runs).
    pub fn rare_sheet_1(&self) -> Result<Vec<String>, String> {
        let mut sheet = Vec::new();
        for run_name in &self.collation.rare_sheets.sheet_1_runs {
            sheet.extend_from_slice(self.run(run_name)?);
        }
        Ok(sheet)
    }

    /// Build the combined rare sheet 2 sequence.
    pub fn rare_sheet_2(&self) -> Result<Vec<String>, String> {
        let mut sheet = Vec::new();
        for run_name in &self.collation.rare_sheets.sheet_2_runs {
            sheet.extend_from_slice(self.run(run_name)?);
        }
        Ok(sheet)
    }

    /// Build the combined DFC sheet sequence.
    pub fn dfc_sheet(&self) -> Result<Vec<String>, String> {
        let mut sheet = Vec::new();
        for run_name in &self.collation.dfc_sheet_runs {
            sheet.extend_from_slice(self.run(run_name)?);
        }
        Ok(sheet)
    }

    /// Return all unique card names across all runs.
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
                let lookup_name = name.split(" // ").next().unwrap_or(name);
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
