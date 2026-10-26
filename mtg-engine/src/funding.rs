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
    /// Groups of eligible tap sources, sorted by (category, name), then
    /// around what else the player could pay for (`order_around_demands`).
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
    /// The order floating mana is spent on X in: what the rest of what the
    /// player could pay for needs least first (`order_around_demands`).
    /// Empty — a board built without one — spends the largest bucket first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pool_order: Vec<ManaType>,
    /// What the rest of what the player could pay for needs of each type of
    /// floating mana in particular (`order_around_demands`): the allocator
    /// spends what is spare beyond this first, and this only when nothing
    /// spare is left. Empty: nothing is kept back.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pool_keep: BTreeMap<ManaType, u32>,
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

/// The groups as the proved kernel takes them (`mtg_kernel::funding`):
/// mana per tap, and how many sources tap for it.
fn quanta_and_counts(groups: &[FundingGroup]) -> (Vec<u32>, Vec<u32>) {
    groups
        .iter()
        .map(|g| (g.mana_per_tap, u32::try_from(g.source_ids.len()).unwrap_or(u32::MAX)))
        .unzip()
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
    let (quanta, counts) = quanta_and_counts(&options.groups);
    // Proved: `ok[m]` iff some tap sum the board has, topped up from the
    // pool, comes to exactly `m`.
    let ok = mtg_kernel::funding::fundable_amounts(pool_total, &quanta, &counts, options.max_x);
    (0..=options.max_announceable_x())
        .filter(|&x| ok.get(options.mana_for_x(x) as usize).copied().unwrap_or(false))
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

    // Proved (`mtg_kernel::funding::fund`): the taps are ones the board
    // has, they fund `target` exactly whenever any allocation does, an
    // exact one taps as little as any exact one (the pool is drained before
    // anything is tapped), and nothing funds more of `target` than this.
    let (quanta, counts) = quanta_and_counts(&options.groups);
    let funding = mtg_kernel::funding::fund(pool_total, &quanta, &counts, target);
    let tap_sum = funding.tap_sum;

    // Pool: around `pool_keep` when the engine gave it, through the proved
    // `pay_around`: what is spare first, in `pool_order`, and what the rest
    // of the hand needs only once nothing spare is left. Taking all of one
    // type before the next spent a floating {W} a spell in hand needed while
    // a spare {U} sat beside it, as payment once did (#252). Otherwise in
    // `pool_order` when there is one, or largest buckets first.
    let mut remaining = target - tap_sum;
    if !options.pool_keep.is_empty() {
        if let Some(drained) = crate::mana::spend_floating_around(&options.pool, remaining, &options.pool_keep) {
            response.pool = drained;
            remaining = 0;
        }
    }
    let mut pool_sorted: Vec<(ManaType, u32)> =
        options.pool.iter().map(|(k, v)| (*k, *v)).collect();
    pool_sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    if !options.pool_order.is_empty() {
        let rank = |mt: &ManaType| options.pool_order.iter().position(|o| o == mt).unwrap_or(usize::MAX);
        pool_sorted.sort_by_key(|(mt, _)| rank(mt));
    }
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

    // Taps, group by group in their given order (`order_around_demands`:
    // what nothing else needs first, creatures last): the kernel has each
    // group give the most it can among the allocations summing to
    // `tap_sum`.
    for (g, &taps) in options.groups.iter().zip(&funding.taps) {
        if taps > 0 {
            response.taps.insert(g.name.clone(), taps * g.mana_per_tap);
        }
    }

    (response, remaining)
}

/// What announcing `x` spends, in the words a cast's row uses for its tap
/// plan: `"X = 3: tap Nephalia Drownyard, 2x Island"`, `"X = 4: 1 Red from
/// the pool, tap 3x Island"`, `"X = 0: nothing is spent"`. None when `x` is
/// not payable, which the prompt refuses on its own terms.
///
/// The X prompt listed the sources and took a number, and
/// [`allocate_for_x`] chose among them unseen: the person learned which
/// lands it tapped from the log, after the cast (#845). Every surface that
/// asks for X shows this for the value being typed, before it is committed;
/// the page carries a copy, held to this one by `x-funding-cases.jsonl`.
#[must_use]
pub fn describe_x_plan(options: &FundingOptions, x: u32) -> Option<String> {
    if x > options.max_announceable_x() {
        return None;
    }
    let (response, shortfall) = allocate_for_x(options, x);
    if shortfall > 0 {
        return None;
    }
    let mut parts = Vec::new();
    let pool: Vec<String> = response
        .pool
        .iter()
        .filter(|(_, n)| **n > 0)
        .map(|(mt, n)| format!("{n} {mt:?}"))
        .collect();
    if !pool.is_empty() {
        parts.push(format!("{} from the pool", pool.join(", ")));
    }
    // In the order the groups are spent, not the response map's.
    let taps: Vec<String> = options
        .groups
        .iter()
        .filter_map(|g| {
            let amount = response.taps.get(&g.name).copied().unwrap_or(0);
            if amount == 0 || g.mana_per_tap == 0 {
                return None;
            }
            let count = amount / g.mana_per_tap;
            Some(if count > 1 { format!("{count}x {}", g.name) } else { g.name.clone() })
        })
        .collect();
    if !taps.is_empty() {
        parts.push(format!("tap {}", taps.join(", ")));
    }
    if parts.is_empty() {
        return Some(format!("X = {x}: nothing is spent"));
    }
    Some(format!("X = {x}: {}", parts.join(", ")))
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
        pool_order: Vec::new(),
        pool_keep: BTreeMap::new(),
    }
}

/// Reorder `options.groups` so that the groups the rest of what `player`
/// could pay for needs come last: first any group holding a permanent whose
/// own `{T}` ability is one of those demands, then by how many of their
/// coloured pips its colours could pay, then a source of `{C}` alone before
/// one that makes a colour, as a cast's plan spends it (#114). Only a
/// demand the board can pay now has a say: Stensia Bloodhall's `{3}{B}{R}`
/// with the only red source already spent on the spell kept the Swamps and
/// itself back and sent the Islands a flashback needed into X (#843).
/// Creatures still come last whatever they make: tapping one costs an
/// attacker or a blocker.
/// The allocator — this crate's [`allocate_for_x`] and the page's copy —
/// walks the groups in order, so X is funded from what nothing else needs
/// first, as a cast's plan is (issue #837, #840). Stable, so ties keep the
/// category and name order. It also sets `pool_order`, the order the
/// allocator spends floating mana in. `paying_for` (the spell being cast,
/// or the permanent whose ability it is) is not "the rest".
pub fn order_around_demands(
    options: &mut FundingOptions,
    state: &GameState,
    registry: &CardRegistry,
    player: PlayerId,
    paying_for: Option<ObjectId>,
) {
    let demands: Vec<crate::mana::Demand> = crate::engine::mana_demands(state, registry, player)
        .into_iter()
        .filter(|(id, _)| Some(*id) != paying_for)
        .map(|(_, d)| d)
        .collect();
    let pool = &state.get_player(player).mana_pool;
    let sources = crate::engine::gather_mana_sources(state, player, registry);
    let payable: Vec<&crate::mana::Demand> = demands.iter()
        .filter(|d| d.payable(pool, &sources, &[]))
        .collect();
    let rank = |g: &FundingGroup| -> (bool, u32, bool) {
        let keeps = payable.iter().any(|d| d.keep.is_some_and(|k| g.source_ids.contains(&k)));
        let pips = payable.iter()
            .flat_map(|d| d.cost.colored_requirements())
            .filter(|(c, _)| g.colors_produced.contains(c))
            .map(|(_, n)| n)
            .sum();
        (keeps, pips, !g.colors_produced.is_empty())
    };
    options.groups.sort_by_key(|g| (g.category == FundingCategory::Dorks, rank(g)));
    // Floating mana is spent on X around the same demands, as a payment
    // out of the pool is (`auto_pay_reserving`, #678).
    // Around the payable ones only: Army of the Damned's three {B} with
    // four mana floating made the {U} Think Twice needs look spare (#843).
    let reserve = crate::mana::hand_reserve(&payable.iter().map(|d| d.cost.clone()).collect::<Vec<_>>());
    options.pool_order = crate::mana::generic_payment_order(&state.get_player(player).mana_pool, &reserve);
    options.pool_keep = crate::mana::specific_by_type(&reserve);
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

    /// X out of floating mana spends what the hand does not need first.
    /// {W}{W}{U} floating and a {W} spell in hand: X = 2 is one White and the
    /// Blue. Taking every White first, because White and Blue were one spare
    /// each and White sorts first, left the hand a Blue it could not cast
    /// with: the defect `auto_pay_reserving` had (#252), in the allocator
    /// every seat's X answer goes through. Now the pool step is that proved
    /// payment.
    #[test]
    fn x_from_the_pool_spends_what_the_hand_does_not_need_first() {
        let pool = BTreeMap::from([(ManaType::White, 2), (ManaType::Blue, 1)]);
        let in_hand = crate::types::ManaCost::new(vec![
            crate::types::ManaSymbol::Colored(crate::types::Color::White),
        ]);
        let options = FundingOptions {
            pool: pool.clone(),
            groups: vec![],
            max_x: 3,
            x_discount: 0,
            pool_order: crate::mana::generic_payment_order(
                &crate::types::ManaPool { mana: pool }, &in_hand),
            pool_keep: crate::mana::specific_by_type(&in_hand),
        };
        let (response, shortfall) = allocate_for_x(&options, 2);
        assert_eq!(shortfall, 0);
        assert_eq!(response.pool, BTreeMap::from([(ManaType::White, 1), (ManaType::Blue, 1)]),
            "one White stays floating for the spell in hand");

        // Every last point of it, when X asks for all of it.
        let (response, shortfall) = allocate_for_x(&options, 3);
        assert_eq!(shortfall, 0);
        assert_eq!(response.pool, BTreeMap::from([(ManaType::White, 2), (ManaType::Blue, 1)]));
    }

    #[test]
    fn validate_accepts_empty_response() {
        let options = FundingOptions {
            pool: BTreeMap::new(),
            groups: vec![group("Swamp", FundingCategory::Lands, 1, 3)],
            max_x: 3,
            x_discount: 0,
            pool_order: Vec::new(),
            pool_keep: BTreeMap::new(),
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
            pool_order: Vec::new(),
            pool_keep: BTreeMap::new(),
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
            pool_order: Vec::new(),
            pool_keep: BTreeMap::new(),
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
            pool_order: Vec::new(),
            pool_keep: BTreeMap::new(),
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
            pool_order: Vec::new(),
            pool_keep: BTreeMap::new(),
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
            pool_order: Vec::new(),
            pool_keep: BTreeMap::new(),
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
            pool_order: Vec::new(),
            pool_keep: BTreeMap::new(),
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
            pool_order: Vec::new(),
            pool_keep: BTreeMap::new(),
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
            pool_order: Vec::new(),
            pool_keep: BTreeMap::new(),
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
            pool_order: Vec::new(),
            pool_keep: BTreeMap::new(),
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
            pool_order: Vec::new(),
            pool_keep: BTreeMap::new(),
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
                        pool_order: Vec::new(),
                        pool_keep: BTreeMap::new(),
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

    /// Every tap sum `groups` can produce exactly, by brute force over how
    /// many sources of each group are tapped.
    fn every_tap_sum(groups: &[FundingGroup]) -> std::collections::BTreeSet<u32> {
        let mut sums: std::collections::BTreeSet<u32> = [0].into_iter().collect();
        for g in groups {
            let count = u32::try_from(g.source_ids.len()).unwrap();
            let mut next = std::collections::BTreeSet::new();
            for &s in &sums {
                for t in 0..=count {
                    next.insert(s + t * g.mana_per_tap);
                }
            }
            sums = next;
        }
        sums
    }

    #[test]
    fn an_allocation_funds_the_most_of_x_the_board_can_reach_and_says_the_rest_is_short() {
        // The contract every seat leans on, stated over the response rather
        // than over the arithmetic that builds it:
        //
        //   what it funds + what it reports short == the mana X needs
        //   what it funds == the most any allocation of this board reaches
        //
        // Break the first and a player announces an X the payment does not
        // cover, or taps mana the announcement never bought. Break the
        // second and a payable mana is left on the board while the
        // shortfall claims X was out of reach (#593). Brute force says what
        // the board reaches; `validate` says the response is one the
        // payment can actually execute.
        let boards = vec![
            vec![],
            vec![group("Sol Ring", FundingCategory::Rocks, 2, 1)],
            vec![group("Sol Ring", FundingCategory::Rocks, 2, 3)],
            vec![group("Mountain", FundingCategory::Lands, 1, 1),
                 group("Sol Ring", FundingCategory::Rocks, 2, 1)],
            vec![group("Mountain", FundingCategory::Lands, 1, 3),
                 group("Sol Ring", FundingCategory::Rocks, 2, 2)],
            vec![group("Gilded Lotus", FundingCategory::Rocks, 3, 2),
                 group("Sol Ring", FundingCategory::Rocks, 2, 1)],
            vec![group("Worn Powerstone", FundingCategory::Rocks, 4, 2),
                 group("Gilded Lotus", FundingCategory::Rocks, 3, 1)],
            vec![group("Mountain", FundingCategory::Lands, 1, 2),
                 group("Gilded Lotus", FundingCategory::Rocks, 3, 1),
                 group("Llanowar Elves", FundingCategory::Dorks, 1, 2)],
            vec![group("Cabal Coffers", FundingCategory::Lands, 5, 1),
                 group("Sol Ring", FundingCategory::Rocks, 2, 2),
                 group("Llanowar Elves", FundingCategory::Dorks, 1, 1)],
            // A group with no output at all must not shift the answer.
            vec![group("Tapped Out", FundingCategory::Rocks, 0, 2),
                 group("Sol Ring", FundingCategory::Rocks, 2, 2)],
        ];
        for groups in &boards {
            let tap_sums = every_tap_sum(groups);
            let tappable: u32 = groups.iter().map(FundingGroup::max_contribution).sum();
            for (red, green) in [(0, 0), (1, 0), (2, 0), (3, 0), (1, 2), (2, 2)] {
                let mut pool = BTreeMap::new();
                if red > 0 {
                    pool.insert(ManaType::Red, red);
                }
                if green > 0 {
                    pool.insert(ManaType::Green, green);
                }
                let pool_total: u32 = red + green;
                for discount in 0..3u32 {
                    let options = FundingOptions {
                        pool: pool.clone(),
                        groups: groups.clone(),
                        max_x: pool_total + tappable,
                        x_discount: discount,
                        pool_order: Vec::new(),
                        pool_keep: BTreeMap::new(),
                    };
                    for x in 0..=options.max_announceable_x() {
                        let target = options.mana_for_x(x);
                        let reachable = tap_sums
                            .iter()
                            .flat_map(|&s| (0..=pool_total).map(move |p| s + p))
                            .filter(|&funded| funded <= target)
                            .max()
                            .unwrap_or(0);
                        let (response, shortfall) = allocate_for_x(&options, x);
                        assert_eq!(
                            validate(&response, &options), Ok(()),
                            "X = {x} on {options:?} produced {response:?}");
                        assert_eq!(
                            response.x_value() + shortfall, target,
                            "X = {x} on {options:?}: {response:?} funds {} and calls \
                             {shortfall} short of {target}", response.x_value());
                        assert_eq!(
                            response.x_value(), reachable,
                            "X = {x} on {options:?}: {response:?} funds {} where some \
                             allocation reaches {reachable}", response.x_value());
                    }
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

    /// The X prompt says what the typed X will tap before it is committed,
    /// in the spend order (#845): the Drownyard before the Islands.
    #[test]
    fn the_plan_names_what_x_spends() {
        let mut pool = BTreeMap::new();
        pool.insert(ManaType::Red, 1);
        let options = FundingOptions {
            pool,
            groups: vec![
                group("Nephalia Drownyard", FundingCategory::Lands, 1, 1),
                group("Island", FundingCategory::Lands, 1, 3),
                group("Sol Ring", FundingCategory::Rocks, 2, 1),
            ],
            max_x: 7,
            x_discount: 0,
            pool_order: Vec::new(),
            pool_keep: BTreeMap::new(),
        };
        assert_eq!(describe_x_plan(&options, 0).as_deref(), Some("X = 0: nothing is spent"));
        assert_eq!(describe_x_plan(&options, 1).as_deref(), Some("X = 1: 1 Red from the pool"));
        assert_eq!(describe_x_plan(&options, 4).as_deref(),
            Some("X = 4: 1 Red from the pool, tap Nephalia Drownyard, 2x Island"));
        assert_eq!(describe_x_plan(&options, 7).as_deref(),
            Some("X = 7: 1 Red from the pool, tap Nephalia Drownyard, 3x Island, Sol Ring"));
        assert_eq!(describe_x_plan(&options, 8), None);
        // A value the sources cannot sum to has no plan.
        let rock = FundingOptions {
            pool: BTreeMap::new(),
            groups: vec![group("Sol Ring", FundingCategory::Rocks, 2, 1)],
            max_x: 2,
            x_discount: 0,
            pool_order: Vec::new(),
            pool_keep: BTreeMap::new(),
        };
        assert_eq!(describe_x_plan(&rock, 1), None);
        assert_eq!(describe_x_plan(&rock, 2).as_deref(), Some("X = 2: tap Sol Ring"));
    }
}
