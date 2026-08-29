//! X-cost funding: when casting an X-cost spell or activating an X-cost
//! ability, the player explicitly chooses which mana sources to tap and how
//! much floating mana to spend. The sum determines X.
//!
//! This replaces the older flow in which X was picked as a single integer
//! and the engine auto-selected sources. The new prompt gives the player
//! full control over which sources go into X (preserving color for other
//! spells, avoiding mana dorks they want to attack with, etc.).
//!
//! The cast-flow ordering is also now rules-correct: X is announced (via
//! this funding choice) before the spell is moved to the stack (CR 601.2b
//! announce X → 601.2i spell becomes cast).

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::cards::CardRegistry;
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{CardType, Color, Keyword, ManaType, Zone};

/// Top-level bucket for grouping funding sources in the prompt schema.
///
/// The three categories communicate different tap costs to the agent:
/// tapping a land has no real cost; tapping a rock has no combat cost;
/// tapping a dork removes a potential attacker/blocker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FundingCategory {
    /// Land permanents.
    Lands,
    /// Non-land, non-creature permanents (mana rocks like Sol Ring).
    Rocks,
    /// Creature permanents with mana abilities (mana dorks).
    Dorks,
}

impl FundingCategory {
    /// JSON schema key under which this category's groups appear.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            FundingCategory::Lands => "lands",
            FundingCategory::Rocks => "rocks",
            FundingCategory::Dorks => "dorks",
        }
    }
}

/// A group of interchangeable mana sources.
///
/// Two sources are in the same group iff their [`funding_key`] is equal.
/// Today the key is the card's printed name; effects that make individual
/// copies produce different output (e.g. Utopia Sprawl on one specific
/// Forest) would extend the key to include modifier info so per-instance
/// differences bucket into distinct groups.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingGroup {
    /// JSON schema key and display name for this group (e.g. "Forest").
    pub name: String,
    /// Top-level category this group belongs to.
    pub category: FundingCategory,
    /// Mana produced by one activation of any source in this group.
    /// Sources with different outputs must be in different groups.
    pub mana_per_tap: u32,
    /// Concrete sources, sorted by `ObjectId` for determinism. When a
    /// response allocates `N * mana_per_tap` mana to this group the engine
    /// taps the first `N` of these.
    pub source_ids: Vec<ObjectId>,
    /// Colors this source can produce. Shown in the schema description so
    /// the agent can reason about color preservation (even though X is
    /// generic and color doesn't otherwise matter here).
    pub colors_produced: Vec<Color>,
}

impl FundingGroup {
    /// Maximum mana this group can contribute (all sources tapped).
    #[must_use]
    pub fn max_contribution(&self) -> u32 {
        let count = u32::try_from(self.source_ids.len()).unwrap_or(u32::MAX);
        count.saturating_mul(self.mana_per_tap)
    }
}

/// Options presented to the player for funding an X-cost payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingOptions {
    /// Floating mana per type at the moment of prompting.
    pub pool: BTreeMap<ManaType, u32>,
    /// Groups of eligible tap sources, sorted by (category, name).
    pub groups: Vec<FundingGroup>,
    /// Ceiling on X = pool total + sum of group contributions.
    pub max_x: u32,
}

/// A player's response to a [`ResolutionChoiceKind::ChooseXFunding`] prompt.
///
/// Values are mana amounts (not source counts). For groups with
/// `mana_per_tap > 1`, each value must be a multiple of `mana_per_tap`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FundingResponse {
    /// Mana to drain from the pool, by type. Each value bounded by the
    /// pool's current availability for that type.
    pub pool: BTreeMap<ManaType, u32>,
    /// Mana to contribute from each group by tapping sources. Keys are
    /// `FundingGroup.name`s from the prompt's options.
    pub taps: BTreeMap<String, u32>,
}

impl FundingResponse {
    /// The X value implied by this response (pool drain + tap output).
    #[must_use]
    pub fn x_value(&self) -> u32 {
        let pool_total: u32 = self.pool.values().sum();
        let tap_total: u32 = self.taps.values().sum();
        pool_total + tap_total
    }

    /// Total mana this response contributes from taps.
    #[must_use]
    pub fn tap_total(&self) -> u32 {
        self.taps.values().sum()
    }

    /// True iff the response contains no allocations (X = 0, nothing tapped).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.x_value() == 0
    }
}

/// Error returned when a [`FundingResponse`] is inconsistent with its
/// [`FundingOptions`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FundingError {
    /// Tap allocation refers to a group that wasn't offered.
    UnknownGroup(String),
    /// Tap allocation isn't a multiple of the group's `mana_per_tap`.
    InvalidTapAmount { group: String, amount: u32, per_tap: u32 },
    /// Tap allocation exceeds the group's total output.
    TapOverflow { group: String, amount: u32, max: u32 },
    /// Pool drain exceeds the floating mana of that type.
    PoolOverdraw {
        mana_type: ManaType,
        amount: u32,
        available: u32,
    },
    /// Resulting X exceeds `max_x`.
    ExceedsMaxX { x: u32, max_x: u32 },
}

impl std::fmt::Display for FundingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FundingError::UnknownGroup(g) => write!(f, "unknown funding group {g:?}"),
            FundingError::InvalidTapAmount { group, amount, per_tap } =>
                write!(f, "tap amount {amount} for {group:?} is not a multiple of {per_tap}"),
            FundingError::TapOverflow { group, amount, max } =>
                write!(f, "tap amount {amount} for {group:?} exceeds maximum {max}"),
            FundingError::PoolOverdraw { mana_type, amount, available } =>
                write!(f, "requested {amount} {mana_type:?} from pool but only {available} available"),
            FundingError::ExceedsMaxX { x, max_x } =>
                write!(f, "X = {x} exceeds max_x = {max_x}"),
        }
    }
}

impl std::error::Error for FundingError {}

/// Validate that `response` is consistent with `options`.
///
/// # Errors
/// Returns [`FundingError`] for any inconsistency (unknown group, wrong
/// tap increment, pool overdraw, or X exceeding max).
pub fn validate(response: &FundingResponse, options: &FundingOptions) -> Result<(), FundingError> {
    // Tap allocations must match offered groups and be valid increments.
    for (group_name, &amount) in &response.taps {
        let group = options
            .groups
            .iter()
            .find(|g| g.name == *group_name)
            .ok_or_else(|| FundingError::UnknownGroup(group_name.clone()))?;
        if group.mana_per_tap == 0 {
            // Degenerate (shouldn't happen — a group with zero output would
            // be filtered out); treat any nonzero amount as overflow.
            if amount > 0 {
                return Err(FundingError::TapOverflow {
                    group: group_name.clone(),
                    amount,
                    max: 0,
                });
            }
            continue;
        }
        if amount % group.mana_per_tap != 0 {
            return Err(FundingError::InvalidTapAmount {
                group: group_name.clone(),
                amount,
                per_tap: group.mana_per_tap,
            });
        }
        let max = group.max_contribution();
        if amount > max {
            return Err(FundingError::TapOverflow {
                group: group_name.clone(),
                amount,
                max,
            });
        }
    }

    // Pool drains bounded by floating mana.
    for (&mt, &amount) in &response.pool {
        let available = options.pool.get(&mt).copied().unwrap_or(0);
        if amount > available {
            return Err(FundingError::PoolOverdraw {
                mana_type: mt,
                amount,
                available,
            });
        }
    }

    // X must not exceed max.
    let x = response.x_value();
    if x > options.max_x {
        return Err(FundingError::ExceedsMaxX {
            x,
            max_x: options.max_x,
        });
    }
    Ok(())
}

/// Canonical funding-bucket key for a permanent.
///
/// Two sources are interchangeable for X-cost funding iff their keys match.
/// Today the key is the printed card name from the registry; effects that
/// modify a specific source's mana output (e.g. Utopia Sprawl on one
/// specific Forest) would extend this key to include modifier info so the
/// affected source falls into its own group.
#[must_use]
pub fn funding_key(state: &GameState, registry: &CardRegistry, id: ObjectId) -> String {
    let Some(obj) = state.get_object(id) else {
        return format!("#{}", id.0);
    };
    registry
        .card_data(obj.card_id)
        .map_or_else(|| obj.name.clone(), |d| d.name)
}

/// Build funding options for a player's current battlefield state.
///
/// Scans untapped permanents the player controls and collects fixed-output
/// mana sources, bucketed into groups. Sources are excluded when they have
/// variable output, side effects beyond producing mana, or cost-bearing
/// activation choices — the player must manually tap those before casting
/// so their mana floats in the pool.
#[must_use]
pub fn build_options(
    state: &GameState,
    player: PlayerId,
    registry: &CardRegistry,
) -> FundingOptions {
    let pool: BTreeMap<ManaType, u32> = state.get_player(player).mana_pool.mana.clone();
    let pool_total: u32 = pool.values().sum();

    // Bucket-building scratch: key -> (category, mana_per_tap, colors, source_ids).
    let mut buckets: HashMap<
        String,
        (FundingCategory, u32, Vec<Color>, Vec<ObjectId>),
    > = HashMap::new();

    for obj in state.objects_in_zone(Zone::Battlefield, player) {
        if obj.tapped {
            continue;
        }
        let abilities = crate::engine::available_mana_abilities(state, obj.id, registry);
        if abilities.is_empty() {
            continue;
        }

        // Filter out sources whose abilities don't all produce the same
        // fixed mana AMOUNT per activation. Dual lands (two abilities each
        // producing 1 mana of different colors) pass — per-activation
        // output is uniform. A hypothetical source with {C} and {C}{C}
        // abilities would be excluded and the player would pre-tap.
        let per_tap_candidates: Vec<u32> = abilities
            .iter()
            .map(|a| a.produced.iter().map(|&(_, n)| n).sum::<u32>())
            .collect();
        let Some(&first_per_tap) = per_tap_candidates.first() else {
            continue;
        };
        if first_per_tap == 0 {
            continue;
        }
        if per_tap_candidates.iter().any(|&n| n != first_per_tap) {
            continue;
        }
        if abilities.iter().any(|a| a.has_side_effects) {
            continue;
        }

        // Summoning-sick creatures can't activate {T} abilities unless
        // they have haste (CR 302.6). Lands are never summoning-sick.
        let is_creature = state.is_creature(obj.id, registry);
        let needs_tap = abilities.iter().any(|a| a.requires_tap);
        if is_creature && obj.summoning_sick && needs_tap
            && !state.has_keyword(obj.id, Keyword::Haste, registry)
        {
            continue;
        }

        let category = if state.has_card_type(obj.id, CardType::Land, registry) {
            FundingCategory::Lands
        } else if is_creature {
            FundingCategory::Dorks
        } else {
            FundingCategory::Rocks
        };

        let mut colors: Vec<Color> = Vec::new();
        for ab in &abilities {
            for &(mt, _) in &ab.produced {
                if let Some(c) = mana_type_to_color(mt) {
                    if !colors.contains(&c) {
                        colors.push(c);
                    }
                }
            }
        }

        let key = funding_key(state, registry, obj.id);
        let entry = buckets
            .entry(key)
            .or_insert_with(|| (category, first_per_tap, colors, Vec::new()));
        entry.3.push(obj.id);
    }

    let mut groups: Vec<FundingGroup> = buckets
        .into_iter()
        .map(|(name, (category, mana_per_tap, colors, mut ids))| {
            ids.sort();
            FundingGroup {
                name,
                category,
                mana_per_tap,
                source_ids: ids,
                colors_produced: colors,
            }
        })
        .collect();
    groups.sort_by(|a, b| {
        let cat_cmp = (a.category as u8).cmp(&(b.category as u8));
        if cat_cmp == std::cmp::Ordering::Equal {
            a.name.cmp(&b.name)
        } else {
            cat_cmp
        }
    });

    let tap_total: u32 = groups.iter().map(FundingGroup::max_contribution).sum();
    let max_x = pool_total.saturating_add(tap_total);

    FundingOptions {
        pool,
        groups,
        max_x,
    }
}

/// Apply a validated funding response to the game state: drain the player's
/// specified pool mana, tap the specified sources (which pushes their mana
/// into the pool), and then drain the tapped mana back out so it goes toward
/// paying X instead of floating.
///
/// Returns the final X value.
///
/// # Panics
/// Panics if the response references a group not in `options` (validation
/// should be run before this call).
pub fn apply(
    state: &mut GameState,
    player: PlayerId,
    options: &FundingOptions,
    response: &FundingResponse,
    registry: &CardRegistry,
) -> u32 {
    // Step 1: drain player-specified pool mana (colored preservation).
    for (&mt, &amount) in &response.pool {
        if amount > 0 {
            state.get_player_mut(player).mana_pool.sub(mt, amount);
        }
    }

    // Step 2: tap sources in each group. Track how much each mana type was
    // added by the taps so we can drain exactly that much afterwards
    // (without touching mana the player wanted to preserve).
    let mut tap_added: HashMap<ManaType, u32> = HashMap::new();
    for (group_name, &amount) in &response.taps {
        let group = options
            .groups
            .iter()
            .find(|g| g.name == *group_name)
            .expect("funding::validate should have caught unknown group");
        if group.mana_per_tap == 0 || amount == 0 {
            continue;
        }
        let count = (amount / group.mana_per_tap) as usize;
        for &src_id in group.source_ids.iter().take(count) {
            // Pick the first mana ability for determinism (dual lands: any
            // color works since X is generic).
            let ability_index = {
                let abs = crate::engine::available_mana_abilities(state, src_id, registry);
                let Some(first) = abs.first() else { continue; };
                first.ability_index
            };
            let before = state.get_player(player).mana_pool.mana.clone();
            crate::engine::activate_mana_source(state, src_id, ability_index, registry);
            let after = &state.get_player(player).mana_pool.mana;
            for (&mt, &after_amount) in after {
                let before_amount = before.get(&mt).copied().unwrap_or(0);
                if after_amount > before_amount {
                    *tap_added.entry(mt).or_insert(0) += after_amount - before_amount;
                }
            }
        }
    }

    // Step 3: drain the tap-added mana (this is what the taps "paid" for X).
    for (mt, amount) in tap_added {
        if amount > 0 {
            state.get_player_mut(player).mana_pool.sub(mt, amount);
        }
    }

    response.x_value()
}

fn mana_type_to_color(mt: ManaType) -> Option<Color> {
    match mt {
        ManaType::White => Some(Color::White),
        ManaType::Blue => Some(Color::Blue),
        ManaType::Black => Some(Color::Black),
        ManaType::Red => Some(Color::Red),
        ManaType::Green => Some(Color::Green),
        ManaType::Colorless => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ObjectId;

    fn group(name: &str, category: FundingCategory, mana_per_tap: u32, count: usize) -> FundingGroup {
        FundingGroup {
            name: name.into(),
            category,
            mana_per_tap,
            source_ids: (0..count).map(|i| ObjectId(100 + i as u64)).collect(),
            colors_produced: vec![],
        }
    }

    #[test]
    fn validate_accepts_empty_response() {
        let options = FundingOptions {
            pool: BTreeMap::new(),
            groups: vec![group("Swamp", FundingCategory::Lands, 1, 3)],
            max_x: 3,
        };
        let response = FundingResponse::default();
        assert_eq!(validate(&response, &options), Ok(()));
        assert_eq!(response.x_value(), 0);
    }

    #[test]
    fn validate_rejects_unknown_group() {
        let options = FundingOptions {
            pool: BTreeMap::new(),
            groups: vec![group("Swamp", FundingCategory::Lands, 1, 2)],
            max_x: 2,
        };
        let mut response = FundingResponse::default();
        response.taps.insert("Forest".into(), 1);
        assert_eq!(
            validate(&response, &options),
            Err(FundingError::UnknownGroup("Forest".into()))
        );
    }

    #[test]
    fn validate_rejects_non_multiple_of_per_tap() {
        let options = FundingOptions {
            pool: BTreeMap::new(),
            groups: vec![group("Sol Ring", FundingCategory::Rocks, 2, 1)],
            max_x: 2,
        };
        let mut response = FundingResponse::default();
        response.taps.insert("Sol Ring".into(), 1);
        assert_eq!(
            validate(&response, &options),
            Err(FundingError::InvalidTapAmount {
                group: "Sol Ring".into(),
                amount: 1,
                per_tap: 2,
            })
        );
    }

    #[test]
    fn validate_rejects_tap_overflow() {
        let options = FundingOptions {
            pool: BTreeMap::new(),
            groups: vec![group("Swamp", FundingCategory::Lands, 1, 2)],
            max_x: 2,
        };
        let mut response = FundingResponse::default();
        response.taps.insert("Swamp".into(), 3);
        assert_eq!(
            validate(&response, &options),
            Err(FundingError::TapOverflow {
                group: "Swamp".into(),
                amount: 3,
                max: 2,
            })
        );
    }

    #[test]
    fn validate_rejects_pool_overdraw() {
        let mut pool = BTreeMap::new();
        pool.insert(ManaType::Black, 1);
        let options = FundingOptions {
            pool,
            groups: vec![],
            max_x: 1,
        };
        let mut response = FundingResponse::default();
        response.pool.insert(ManaType::Black, 2);
        assert_eq!(
            validate(&response, &options),
            Err(FundingError::PoolOverdraw {
                mana_type: ManaType::Black,
                amount: 2,
                available: 1,
            })
        );
    }

    #[test]
    fn validate_rejects_exceeds_max_x() {
        // Constructed scenario: group max = 1 but response says 2 via pool.
        let mut pool = BTreeMap::new();
        pool.insert(ManaType::Black, 3);
        let options = FundingOptions {
            pool,
            groups: vec![],
            max_x: 1, // artificially low
        };
        let mut response = FundingResponse::default();
        response.pool.insert(ManaType::Black, 2);
        assert_eq!(
            validate(&response, &options),
            Err(FundingError::ExceedsMaxX { x: 2, max_x: 1 })
        );
    }

    #[test]
    fn x_value_is_pool_plus_taps() {
        let mut response = FundingResponse::default();
        response.pool.insert(ManaType::White, 2);
        response.pool.insert(ManaType::Blue, 1);
        response.taps.insert("Mountain".into(), 2);
        response.taps.insert("Sol Ring".into(), 2);
        assert_eq!(response.x_value(), 2 + 1 + 2 + 2);
    }

    #[test]
    fn multi_mana_source_enum_values() {
        // 2 Sol Rings: legal amounts should be 0, 2, 4.
        let g = group("Sol Ring", FundingCategory::Rocks, 2, 2);
        assert_eq!(g.max_contribution(), 4);
    }

    #[test]
    fn category_keys_match_schema_expectations() {
        assert_eq!(FundingCategory::Lands.key(), "lands");
        assert_eq!(FundingCategory::Rocks.key(), "rocks");
        assert_eq!(FundingCategory::Dorks.key(), "dorks");
    }
}
