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
    /// Ceiling on the mana this response may fund = pool total + sum of
    /// group contributions.
    pub max_x: u32,
    /// Generic cost reduction that comes off the announced X rather than off
    /// the printed cost (CR 601.2f). The first `x_discount` of X costs no
    /// mana, so the largest X the player may announce is `max_x + x_discount`
    /// and a response funds `X - x_discount`.
    #[serde(default)]
    pub x_discount: u32,
}

impl FundingOptions {
    /// The largest X the player may announce: the mana they can produce plus
    /// the part of X a cost reduction pays for (CR 601.2b, 601.2f).
    #[must_use]
    pub fn max_announceable_x(&self) -> u32 {
        self.max_x + self.x_discount
    }

    /// The mana a response must fund to announce `x`.
    #[must_use]
    pub fn mana_for_x(&self, x: u32) -> u32 {
        x.saturating_sub(self.x_discount)
    }
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

/// Tap sums groups `i..` can produce exactly, as one reachability row per
/// suffix: `rows[i][s]` is true iff some choice of taps among groups `i..`
/// sums to exactly `s`. `rows[n]` is `{0}` — no groups, no mana.
///
/// Bounded at `limit` because an allocation may never overshoot the X the
/// player announced, and computed per residue class so a group of 4,000
/// one-mana lands costs `O(limit)` rather than `O(limit * 4000)`.
fn tap_reachability(groups: &[FundingGroup], limit: u32) -> Vec<Vec<bool>> {
    let width = limit as usize + 1;
    let mut rows: Vec<Vec<bool>> = vec![Vec::new(); groups.len() + 1];
    let mut row = vec![false; width];
    row[0] = true;
    rows[groups.len()] = row;
    for i in (0..groups.len()).rev() {
        let prev = &rows[i + 1];
        let g = &groups[i];
        // A group that produces nothing contributes nothing (`build_options`
        // filters these out; a hand-built `FundingOptions` may not).
        if g.mana_per_tap == 0 {
            rows[i] = prev.clone();
            continue;
        }
        let q = g.mana_per_tap as usize;
        let max_taps = usize::try_from(g.source_ids.len()).unwrap_or(usize::MAX);
        let mut next = vec![false; width];
        for r in 0..q.min(width) {
            // Steps of `q` back to the nearest reachable sum. The nearest is
            // the criterion: if it is more taps than the group has sources,
            // so is every one behind it.
            let mut taps_back: Option<usize> = None;
            let mut s = r;
            while s < width {
                taps_back = if prev[s] { Some(0) } else { taps_back.map(|t| t + 1) };
                if taps_back.is_some_and(|t| t <= max_taps) {
                    next[s] = true;
                }
                s += q;
            }
        }
        rows[i] = next;
    }
    rows
}

/// Every X the player may announce that some allocation of `options` funds
/// exactly, in ascending order.
///
/// The range the prompt states — `0..=max_announceable_x()` — is not this
/// set. A board whose only source taps for two mana can fund 0 and 2 and
/// nothing between them, and a surface that offers the player 1 is offering
/// a value it cannot honour (#595). Always non-empty: X = 0 funds itself.
#[must_use]
pub fn fundable_x_values(options: &FundingOptions) -> Vec<u32> {
    let pool_total: u32 = options.pool.values().sum();
    let rows = tap_reachability(&options.groups, options.max_x);
    let reachable = &rows[0];
    // The largest tap sum at or below each mana amount, so "can the pool
    // top some tap sum up to exactly `m`?" is one comparison.
    let mut largest_at_or_below: Vec<Option<u32>> = vec![None; reachable.len()];
    let mut best: Option<u32> = None;
    for (s, &ok) in reachable.iter().enumerate() {
        if ok {
            best = Some(u32::try_from(s).unwrap_or(u32::MAX));
        }
        largest_at_or_below[s] = best;
    }
    (0..=options.max_announceable_x())
        .filter(|&x| {
            let mana = options.mana_for_x(x);
            largest_at_or_below
                .get(mana as usize)
                .copied()
                .flatten()
                .is_some_and(|s| mana - s <= pool_total)
        })
        .collect()
}

/// Build the response that funds `x`, and say how much of it could not be
/// funded.
///
/// Funds `x` **exactly whenever the board can**, which is not the same as
/// taking whole activations greedily in the order the prompt lists them:
/// one Mountain and one Sol Ring can fund X = 2, but only by leaving the
/// Mountain untapped, and a greedy pass spent the Mountain first and then
/// reported the leftover 1 as unfundable — announcing X = 1 on a board
/// where X = 2 and X = 3 were both exactly payable (#593).
///
/// Among the allocations that fund `x` exactly, the preference order the
/// prompt implies is kept as a tie-break: drain the pool before tapping
/// anything (largest colour bucket first), then spend lands before rocks
/// before dorks — a player would rather keep a mana dork untapped.
///
/// When *no* allocation funds `x` — the genuinely unbuyable values, which
/// [`fundable_x_values`] enumerates — this funds as much as it can and
/// returns the shortfall. Callers that need an exact X check the second
/// element, and an interactive surface should not offer a value whose
/// shortfall is non-zero in the first place.
///
/// One implementation, every seat. The terminal had it inline and the
/// fuzzer had none — it answered every X prompt by allocating the whole
/// board, which is by construction `max_announceable_x` and nothing else,
/// so no seeded game has ever announced an intermediate X and the
/// under-tapping branch above was reachable from the keyboard only (#564).
#[must_use]
pub fn allocate_for_x(options: &FundingOptions, x: u32) -> (FundingResponse, u32) {
    let mut response = FundingResponse::default();
    // A cost reduction with no generic pips to come off pays for the first
    // `x_discount` of X, so only the rest is funded with mana (CR 601.2f).
    let target = options.mana_for_x(x);
    let pool_total: u32 = options.pool.values().sum();

    let rows = tap_reachability(&options.groups, target);
    let reachable = &rows[0];

    // The pool can top any tap sum up by 0..=pool_total, so `target` is
    // exactly fundable iff some reachable tap sum sits in this window.
    let floor = target.saturating_sub(pool_total);
    let exact = (floor..=target).find(|&s| reachable[s as usize]);
    // Exactness outranks the preference for pool over taps: the smallest
    // tap sum in the window leaves the most of the pool intact, but if the
    // window holds none, spend the whole pool behind the largest tap sum
    // there is and report what is left over.
    let tap_sum = exact.unwrap_or_else(|| {
        (0..floor)
            .rev()
            .find(|&s| reachable[s as usize])
            .unwrap_or(0)
    });

    // Pool: drain largest buckets first.
    let mut remaining = target - tap_sum;
    let mut pool_sorted: Vec<(ManaType, u32)> =
        options.pool.iter().map(|(k, v)| (*k, *v)).collect();
    pool_sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (mt, avail) in pool_sorted {
        if remaining == 0 {
            break;
        }
        let take = avail.min(remaining);
        if take > 0 {
            response.pool.insert(mt, take);
            remaining -= take;
        }
    }

    // Taps: walk the groups in their given order (category-sorted), taking
    // the most each can contribute that still leaves `left` reachable by
    // the groups behind it. That spends lands before rocks before dorks
    // among the allocations summing to `tap_sum`.
    let mut left = tap_sum;
    for (i, g) in options.groups.iter().enumerate() {
        if left == 0 {
            break;
        }
        if g.mana_per_tap == 0 {
            continue;
        }
        let max_taps = u32::try_from(g.source_ids.len()).unwrap_or(u32::MAX);
        let take_taps = (0..=(left / g.mana_per_tap).min(max_taps))
            .rev()
            .find(|&t| rows[i + 1][(left - t * g.mana_per_tap) as usize])
            .unwrap_or(0);
        if take_taps > 0 {
            let amount = take_taps * g.mana_per_tap;
            response.taps.insert(g.name.clone(), amount);
            left -= amount;
        }
    }

    (response, remaining)
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
        // Set by the caster, which is where the spell's cost is known.
        x_discount: 0,
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
            x_discount: 0,
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
            x_discount: 0,
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
            x_discount: 0,
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
            x_discount: 0,
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
            x_discount: 0,
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
            x_discount: 0,
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
    fn an_x_the_board_can_pay_exactly_is_funded_exactly() {
        // One Mountain (1/tap) and one Sol Ring (2/tap): X = 2 is payable
        // only by leaving the Mountain up. Taking whole activations in
        // category order spent the Mountain, could not spend the Sol Ring
        // on the leftover 1, and announced X = 1 on a board where X = 3
        // succeeded — the larger X buyable and the smaller one not (#593).
        let options = FundingOptions {
            pool: BTreeMap::new(),
            groups: vec![
                group("Mountain", FundingCategory::Lands, 1, 1),
                group("Sol Ring", FundingCategory::Rocks, 2, 1),
            ],
            max_x: 3,
            x_discount: 0,
        };
        for x in 0..=3 {
            let (response, shortfall) = allocate_for_x(&options, x);
            assert_eq!(shortfall, 0, "X = {x} is exactly payable on this board");
            assert_eq!(response.x_value(), x, "X = {x} funded {response:?}");
            assert_eq!(validate(&response, &options), Ok(()));
        }
        // And the tie-break survives: the Sol Ring alone pays 2, while 1
        // and 3 still spend the land first.
        assert_eq!(allocate_for_x(&options, 2).0.taps,
            [("Sol Ring".to_string(), 2)].into_iter().collect());
        assert_eq!(allocate_for_x(&options, 1).0.taps,
            [("Mountain".to_string(), 1)].into_iter().collect());
        assert_eq!(allocate_for_x(&options, 3).0.taps,
            [("Mountain".to_string(), 1), ("Sol Ring".to_string(), 2)].into_iter().collect());
    }

    #[test]
    fn the_preference_order_is_only_a_tie_break_among_exact_allocations() {
        // Two dorks (1/tap) and one Sol Ring (2/tap), X = 2. Both the
        // dork pair and the Sol Ring pay it exactly; a player would rather
        // keep the attackers, so the rock wins. X = 3 needs both, and X = 1
        // can only be a dork.
        let options = FundingOptions {
            pool: BTreeMap::new(),
            groups: vec![
                group("Sol Ring", FundingCategory::Rocks, 2, 1),
                group("Llanowar Elves", FundingCategory::Dorks, 1, 2),
            ],
            max_x: 4,
            x_discount: 0,
        };
        for x in 0..=4 {
            let (response, shortfall) = allocate_for_x(&options, x);
            assert_eq!((x, shortfall), (x, 0));
            assert_eq!(response.x_value(), x);
        }
        assert_eq!(allocate_for_x(&options, 2).0.taps,
            [("Sol Ring".to_string(), 2)].into_iter().collect());
        assert_eq!(allocate_for_x(&options, 1).0.taps,
            [("Llanowar Elves".to_string(), 1)].into_iter().collect());
    }

    #[test]
    fn an_x_no_allocation_can_reach_reports_its_shortfall() {
        // The branch that must survive the fix: one 2/tap source funds 0
        // and 2 and nothing between, so X = 1 is unbuyable by any
        // allocation, not merely missed by this one.
        let options = FundingOptions {
            pool: BTreeMap::new(),
            groups: vec![group("Sol Ring", FundingCategory::Rocks, 2, 1)],
            max_x: 2,
            x_discount: 0,
        };
        let (response, shortfall) = allocate_for_x(&options, 1);
        assert_eq!(shortfall, 1);
        assert_eq!(response.x_value(), 0);
        assert_eq!(fundable_x_values(&options), vec![0, 2]);
    }

    #[test]
    fn the_pool_is_spent_first_unless_that_costs_the_announced_x() {
        // Floating mana goes before taps (it is lost at end of step either
        // way), so a pool that covers X taps nothing.
        let mut pool = BTreeMap::new();
        pool.insert(ManaType::Red, 2);
        let options = FundingOptions {
            pool: pool.clone(),
            groups: vec![group("Mountain", FundingCategory::Lands, 1, 2)],
            max_x: 4,
            x_discount: 0,
        };
        let (response, shortfall) = allocate_for_x(&options, 2);
        assert_eq!(shortfall, 0);
        assert_eq!(response.pool, pool);
        assert!(response.taps.is_empty());

        // But exactness outranks that preference: with one Red floating and
        // a single 2/tap rock, X = 2 is payable only by tapping the rock and
        // keeping the Red. Spending the Red first left 1 to find and a
        // source that cannot make 1.
        let mut one_red = BTreeMap::new();
        one_red.insert(ManaType::Red, 1);
        let options = FundingOptions {
            pool: one_red,
            groups: vec![group("Sol Ring", FundingCategory::Rocks, 2, 1)],
            max_x: 3,
            x_discount: 0,
        };
        let (response, shortfall) = allocate_for_x(&options, 2);
        assert_eq!(shortfall, 0);
        assert_eq!(response.x_value(), 2);
        assert!(response.pool.is_empty(), "the Red is kept: {response:?}");
        assert_eq!(fundable_x_values(&options), vec![0, 1, 2, 3]);
    }

    #[test]
    fn fundable_x_values_names_exactly_the_x_values_that_fund_without_shortfall() {
        // The property the interactive surfaces rely on: the set they may
        // offer is the set that funds exactly, over every shape of board.
        let boards = vec![
            vec![],
            vec![group("Sol Ring", FundingCategory::Rocks, 2, 1)],
            vec![group("Sol Ring", FundingCategory::Rocks, 2, 3)],
            vec![group("Mountain", FundingCategory::Lands, 1, 1),
                 group("Sol Ring", FundingCategory::Rocks, 2, 1)],
            vec![group("Gilded Lotus", FundingCategory::Rocks, 3, 2),
                 group("Sol Ring", FundingCategory::Rocks, 2, 1)],
            vec![group("Worn Powerstone", FundingCategory::Rocks, 4, 2),
                 group("Gilded Lotus", FundingCategory::Rocks, 3, 1)],
            // A group with no output at all must not make X unfundable.
            vec![group("Tapped Out", FundingCategory::Rocks, 0, 2),
                 group("Sol Ring", FundingCategory::Rocks, 2, 2)],
        ];
        for pool_red in 0..3u32 {
            for discount in 0..3u32 {
                for groups in &boards {
                    let mut pool = BTreeMap::new();
                    if pool_red > 0 {
                        pool.insert(ManaType::Red, pool_red);
                    }
                    let max_x = pool_red
                        + groups.iter().map(FundingGroup::max_contribution).sum::<u32>();
                    let options = FundingOptions {
                        pool,
                        groups: groups.clone(),
                        max_x,
                        x_discount: discount,
                    };
                    let fundable = fundable_x_values(&options);
                    for x in 0..=options.max_announceable_x() {
                        let (response, shortfall) = allocate_for_x(&options, x);
                        assert_eq!(
                            validate(&response, &options), Ok(()),
                            "X = {x} on {options:?} produced {response:?}");
                        assert_eq!(
                            fundable.contains(&x), shortfall == 0,
                            "X = {x} on {options:?}: fundable says {}, shortfall is {shortfall}",
                            fundable.contains(&x));
                        if shortfall == 0 {
                            assert_eq!(response.x_value(), options.mana_for_x(x),
                                "X = {x} on {options:?} funded {response:?}");
                        }
                    }
                    assert!(fundable.contains(&0), "X = 0 always funds itself");
                }
            }
        }
    }

    #[test]
    fn category_keys_match_schema_expectations() {
        assert_eq!(FundingCategory::Lands.key(), "lands");
        assert_eq!(FundingCategory::Rocks.key(), "rocks");
        assert_eq!(FundingCategory::Dorks.key(), "dorks");
    }
}
