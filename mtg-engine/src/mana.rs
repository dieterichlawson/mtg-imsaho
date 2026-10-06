use crate::types::{ManaCost, ManaPool, ManaType, ManaSymbol, Color};
use crate::ids::ObjectId;
use crate::cards::ManaAbilityDef;

#[derive(Debug)]
pub enum ManaError {
    InsufficientMana,
}

/// The kind of mana source, ordered by opportunity cost of tapping.
/// Lower ordinal = lower opportunity cost = prefer tapping first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// NOTE: If you change these priorities, also update the "Auto-tap" bullet in
/// `GAME_RULES` in mtg-player/src/llm.rs so the agent's system prompt stays accurate.
pub enum ManaSourceKind {
    /// Basic land or mana-only artifact (zero opportunity cost).
    BasicMana = 0,
    /// Non-basic land with only mana abilities (flexibility cost only).
    NonBasicMana = 1,
    /// Permanent with non-mana activated abilities (tapping locks out utility).
    HasUtilityAbility = 2,
    /// Creature with mana ability (tapping prevents attack + block).
    Creature = 3,
    /// Source with side effects (e.g. Deranged Assistant mills a card).
    HasSideEffects = 4,
}

/// A mana source available for autotapping.
#[derive(Debug, Clone)]
pub struct ManaSource {
    pub object_id: ObjectId,
    pub abilities: Vec<ManaAbilityDef>,
    pub source_kind: ManaSourceKind,
}

/// Remaining cost to satisfy after deducting floating mana.
struct RemainingCost {
    colored: Vec<Color>,
    colorless: u32,
    generic: u32,
}

/// Compute the "flexibility" of a mana source: how many distinct colors it can produce.
/// Colorless-only = 0, mono = 1, dual = 2, etc.
fn source_flexibility(source: &ManaSource) -> usize {
    let mut colors = std::collections::HashSet::new();
    for ability in &source.abilities {
        for &(mana_type, _) in &ability.produced {
            match mana_type {
                ManaType::White | ManaType::Blue | ManaType::Black
                | ManaType::Red | ManaType::Green => { colors.insert(mana_type); }
                ManaType::Colorless => {}
            }
        }
    }
    colors.len()
}

/// Compute the "hand demand" score for a source: how much other spells in hand
/// need the colors this source produces. Higher = more demanded = prefer NOT tapping.
fn hand_demand_score(source: &ManaSource, hand_demand: &std::collections::HashMap<Color, u32>) -> u32 {
    let mut score = 0u32;
    for ability in &source.abilities {
        for &(mana_type, _) in &ability.produced {
            let color = match mana_type {
                ManaType::White => Some(Color::White),
                ManaType::Blue => Some(Color::Blue),
                ManaType::Black => Some(Color::Black),
                ManaType::Red => Some(Color::Red),
                ManaType::Green => Some(Color::Green),
                ManaType::Colorless => None,
            };
            if let Some(c) = color {
                score += hand_demand.get(&c).copied().unwrap_or(0);
            }
        }
    }
    score
}

/// Composite sort key for source priority: (opportunity cost tier, flexibility, hand demand).
/// Lower = prefer tapping first. Within a tier, mono-color sources are preferred over
/// dual/multi-color (to preserve flexibility), and sources whose colors are less demanded
/// by other spells in hand are preferred.
/// NOTE: If you change this logic, also update the "Auto-tap" bullet in `GAME_RULES`
/// in mtg-player/src/llm.rs so the agent's system prompt stays accurate.
fn source_sort_key(source: &ManaSource, hand_demand: &std::collections::HashMap<Color, u32>) -> (ManaSourceKind, usize, u32) {
    (source.source_kind, source_flexibility(source), hand_demand_score(source, hand_demand))
}

/// Build hand demand map: for each color, how many colored pips across all `hand_costs`.
fn build_hand_demand(hand_costs: &[ManaCost]) -> std::collections::HashMap<Color, u32> {
    let mut demand = std::collections::HashMap::new();
    for cost in hand_costs {
        for (color, count) in cost.colored_requirements() {
            *demand.entry(color).or_insert(0) += count;
        }
    }
    demand
}

/// Check if a source can produce a specific mana type, and return the `ability_index` if so.
fn ability_producing(source: &ManaSource, mana_type: ManaType) -> Option<usize> {
    for ability in &source.abilities {
        for &(mt, amount) in &ability.produced {
            if mt == mana_type && amount > 0 {
                return Some(ability.ability_index);
            }
        }
    }
    None
}

/// The cheapest ability of `source` that produces `mana_type`: a free one
/// over a filter, when a source has both.
fn cheapest_ability_producing(source: &ManaSource, mana_type: ManaType) -> Option<&ManaAbilityDef> {
    source.abilities.iter()
        .filter(|a| a.produced.iter().any(|&(mt, amount)| mt == mana_type && amount > 0))
        .min_by_key(|a| ability_cost(a))
}

/// Check if a source can produce a specific color.
fn can_produce_color(source: &ManaSource, color: Color) -> bool {
    ability_producing(source, ManaType::from(color)).is_some()
}

/// Check if a source can produce colorless mana.
fn can_produce_colorless(source: &ManaSource) -> bool {
    for ability in &source.abilities {
        for &(mt, amount) in &ability.produced {
            if mt == ManaType::Colorless && amount > 0 {
                return true;
            }
        }
    }
    false
}

/// Get total mana produced by a specific ability.
fn ability_total_mana(ability: &ManaAbilityDef) -> u32 {
    ability.produced.iter().map(|&(_, amount)| amount).sum()
}

/// What activating this ability costs, in generic mana.
///
/// A filter ("{1}, {T}: Add one mana of any color") is net zero: it turns one
/// generic into one colored. The planner treats the cost as extra generic
/// demand, which is exactly right — the filter fixes color, it does not ramp.
fn ability_cost(ability: &ManaAbilityDef) -> u32 {
    ability.cost.mana_value()
}

/// Order a finished tap plan so every free ability is activated before any
/// cost-bearing one.
///
/// A filter's cost is paid from the pool at activation time, and the mana that
/// pays it comes from other sources in this same plan. Since a filter never
/// funds another filter — it produces exactly what it consumes — one stable
/// partition is enough.
fn free_abilities_first(plan: &mut [(ObjectId, usize)], sources: &[ManaSource]) {
    let costs_mana = |&(object_id, ability_index): &(ObjectId, usize)| -> bool {
        sources.iter()
            .find(|s| s.object_id == object_id)
            .and_then(|s| s.abilities.iter().find(|a| a.ability_index == ability_index))
            .is_some_and(|a| ability_cost(a) > 0)
    };
    // Stable partition: free first, cost-bearing after, order preserved within
    // each group.
    let free: Vec<_> = plan.iter().filter(|e| !costs_mana(e)).copied().collect();
    let paid: Vec<_> = plan.iter().filter(|e| costs_mana(e)).copied().collect();
    for (slot, entry) in plan.iter_mut().zip(free.into_iter().chain(paid)) {
        *slot = entry;
    }
}

/// Compute the optimal set of mana sources to tap in order to pay a cost.
///
/// Returns `Some(tap_plan)` with (`object_id`, `ability_index`) pairs, or `None` if
/// the cost cannot be paid with available sources + floating mana.
///
/// `hand_costs` are the mana costs of OTHER castable spells in the player's hand,
/// used for color preservation (prefer not tapping sources needed by other spells).
///
/// # Panics
/// Panics if a source selected to pay a colorless requirement does not actually
/// have an ability producing colorless mana (an internal inconsistency between
/// source filtering and ability lookup).
#[must_use]
pub fn compute_autotap(
    cost: &ManaCost,
    pool: &ManaPool,
    sources: &[ManaSource],
    hand_costs: &[ManaCost],
) -> Option<Vec<(ObjectId, usize)>> {
    let plan = greedy_autotap(cost, pool, sources, hand_costs)?;
    Some(keep_the_hand_castable(plan, cost, pool, sources, hand_costs))
}

/// The pips of `hand_costs` that only one kind of mana pays — coloured and
/// `{C}` — as one cost: what a payment out of the pool should spend last, so
/// the rest of the hand keeps the mana it needs. Generic plays no part:
/// anything pays it.
#[must_use]
pub fn hand_reserve(hand_costs: &[ManaCost]) -> ManaCost {
    ManaCost::new(hand_costs.iter()
        .flat_map(|c| c.symbols.iter())
        .filter(|s| matches!(s, ManaSymbol::Colored(_) | ManaSymbol::Colorless(_)))
        .cloned()
        .collect())
}

/// Run `plan` and pay `cost` the way the engine executes a cast or an
/// activation — each filter's own cost paid around what `cost` needs, then
/// `cost` paid around what `reserve` (the rest of the hand) needs — and
/// return what is left in the pool, or `None` if the plan does not pay.
fn pool_after(
    plan: &[(ObjectId, usize)],
    pool: &ManaPool,
    sources: &[ManaSource],
    cost: &ManaCost,
    reserve: &ManaCost,
) -> Option<ManaPool> {
    let mut pool = pool.clone();
    for &(object_id, ability_index) in plan {
        let ability = sources.iter()
            .find(|s| s.object_id == object_id)
            .and_then(|s| s.abilities.iter().find(|a| a.ability_index == ability_index))?;
        auto_pay_reserving(&mut pool, &ability.cost, cost).ok()?;
        for &(mana_type, amount) in &ability.produced {
            pool.add(mana_type, amount);
        }
    }
    auto_pay_reserving(&mut pool, cost, reserve).ok()?;
    Some(pool)
}

/// Repair a plan that leaves a spell in hand uncastable when another plan
/// for the same cost would not.
///
/// The greedy planner ranks each source by a fixed key — what an ability
/// costs, opportunity-cost tier, colours lost, hand demand last — and each
/// fix to that key has been met by a board the next key down decides
/// wrongly: an unfunded Shimmering Grotto filter counted as still covering
/// White, so a {1}{G} plan spent the only Plains (issue #674); the creature
/// tier kept Avacyn's Pilgrim untapped by tapping the only red source
/// (#679); floating {R} paid a generic pip while an untapped Forest could
/// have (#678). Each time the plan paid what it was asked and a castable
/// spell silently left the menu.
///
/// So the promise is checked on the plan itself rather than on the key: a
/// hand spell the board could pay before is still payable from what the
/// plan leaves. Only when the greedy plan strands one is anything else
/// tried — every plan with one source swapped (or one source's ability
/// changed), and failing a full repair, every plan with one more source —
/// and the plan stranding fewest wins, then the one tapping lower tiers.
/// Never at the price of a side effect, though: a plan that taps more
/// sources with one (Deranged Assistant milling a card) is never preferred,
/// as Phase 3 already holds — milling a card to save a colour is not a win.
/// A greedy plan that strands nothing is returned untouched, so the tiers
/// keep deciding everything they decided before.
fn keep_the_hand_castable(
    plan: Vec<(ObjectId, usize)>,
    cost: &ManaCost,
    pool: &ManaPool,
    sources: &[ManaSource],
    hand_costs: &[ManaCost],
) -> Vec<(ObjectId, usize)> {
    let castable_before: Vec<&ManaCost> = hand_costs.iter()
        .filter(|h| greedy_autotap(h, pool, sources, &[]).is_some())
        .collect();
    if castable_before.is_empty() {
        return plan;
    }
    let reserve = hand_reserve(hand_costs);
    let tapped = |p: &[(ObjectId, usize)]| -> Vec<&ManaSource> {
        p.iter()
            .filter_map(|(id, _)| sources.iter().find(|s| s.object_id == *id))
            .collect()
    };
    let side_effects = |p: &[(ObjectId, usize)]| -> usize {
        tapped(p).iter().filter(|s| s.source_kind == ManaSourceKind::HasSideEffects).count()
    };
    let tier_sum = |p: &[(ObjectId, usize)]| -> u32 {
        tapped(p).iter().map(|s| s.source_kind as u32).sum()
    };
    let stranded = |p: &[(ObjectId, usize)]| -> Option<usize> {
        let left = pool_after(p, pool, sources, cost, &reserve)?;
        let untapped: Vec<ManaSource> = sources.iter()
            .filter(|s| !p.iter().any(|(id, _)| *id == s.object_id))
            .cloned()
            .collect();
        Some(castable_before.iter()
            .filter(|h| greedy_autotap(h, &left, &untapped, &[]).is_none())
            .count())
    };
    let Some(base) = stranded(&plan) else { return plan };
    if base == 0 {
        return plan;
    }
    let mut best = ((side_effects(&plan), base, tier_sum(&plan)), plan.clone());
    let consider = |mut candidate: Vec<(ObjectId, usize)>,
                        best: &mut ((usize, usize, u32), Vec<(ObjectId, usize)>)| {
        free_abilities_first(&mut candidate, sources);
        if let Some(n) = stranded(&candidate) {
            let key = (side_effects(&candidate), n, tier_sum(&candidate));
            if key < best.0 {
                *best = (key, candidate);
            }
        }
    };
    let untapped: Vec<&ManaSource> = sources.iter()
        .filter(|s| !plan.iter().any(|(id, _)| *id == s.object_id))
        .collect();
    for i in 0..plan.len() {
        let own = sources.iter().find(|s| s.object_id == plan[i].0);
        for source in untapped.iter().copied().chain(own) {
            for ability in &source.abilities {
                let mut candidate = plan.clone();
                candidate[i] = (source.object_id, ability.ability_index);
                consider(candidate, &mut best);
            }
        }
    }
    if best.0 .1 > 0 {
        for source in &untapped {
            for ability in &source.abilities {
                let mut candidate = plan.clone();
                candidate.push((source.object_id, ability.ability_index));
                consider(candidate, &mut best);
            }
        }
    }
    best.1
}

/// The planner's first answer: one pass over the cost, choosing each
/// source by a fixed priority key. [`compute_autotap`] checks it against
/// the hand.
fn greedy_autotap(
    cost: &ManaCost,
    pool: &ManaPool,
    sources: &[ManaSource],
    hand_costs: &[ManaCost],
) -> Option<Vec<(ObjectId, usize)>> {
    // Skip X-cost spells -- caller should not pass these.
    if cost.symbols.iter().any(|s| matches!(s, ManaSymbol::X)) {
        return None;
    }

    // Free spells need no tapping.
    if cost.symbols.is_empty() {
        return Some(vec![]);
    }

    let hand_demand = build_hand_demand(hand_costs);

    // Phase 0: Deduct floating mana from the cost.
    let mut sim_pool = pool.clone();
    let mut remaining = RemainingCost {
        colored: Vec::new(),
        colorless: cost.colorless_amount(),
        generic: cost.generic_amount(),
    };

    // Collect colored requirements.
    for sym in &cost.symbols {
        if let ManaSymbol::Colored(color) = sym {
            remaining.colored.push(*color);
        }
    }

    // Deduct floating mana: colored first.
    remaining.colored.retain(|&color| {
        let mt = ManaType::from(color);
        let available = sim_pool.get(mt);
        if available > 0 {
            sim_pool.mana.insert(mt, available - 1);
            false // satisfied, remove from remaining
        } else {
            true // still needed
        }
    });

    // Deduct floating mana: colorless-specific.
    {
        let available = sim_pool.get(ManaType::Colorless);
        let deduct = available.min(remaining.colorless);
        if deduct > 0 {
            sim_pool.mana.insert(ManaType::Colorless, available - deduct);
            remaining.colorless -= deduct;
        }
    }

    // Generic is paid from what floats after the sources are chosen, in
    // Phase 3: any mana pays it, so nothing is gained by spending the pool
    // on it first. (This phase used to deduct it here as well, and the
    // copy no test could tell apart from Phase 3's was a surviving mutant,
    // issue #661.)
    if remaining.colored.is_empty() && remaining.colorless == 0
        && remaining.generic <= sim_pool.total()
    {
        return Some(vec![]);
    }

    // Build mutable list of available sources (indices into the original slice).
    let mut available: Vec<usize> = (0..sources.len()).collect();
    let mut tap_plan: Vec<(ObjectId, usize)> = Vec::new();
    let mut excess_mana: u32 = 0; // excess mana from already-tapped multi-mana sources

    // Phase 1: Satisfy colorless-specific requirements ({C}).
    while remaining.colorless > 0 {
        // Sort available sources that can produce colorless by priority —
        // after first sparing any source that is the only one left able to
        // make a colour this cost still needs. Shimmering Grotto's free {C}
        // outranks Deranged Assistant's (tier 1 against 4), and spending it
        // on the pip left nothing to make the {R} of {C}{R}, when the
        // Assistant's {C} and the Grotto filtering a floating {G} would have
        // paid it (issue #684).
        let sole_route = |src_idx: usize, available: &[usize]| -> bool {
            remaining.colored.iter().any(|&color|
                can_produce_color(&sources[src_idx], color)
                    && !available.iter().any(|&other| other != src_idx
                        && can_produce_color(&sources[other], color)))
        };
        let best = available.iter()
            .enumerate()
            .filter(|&(_, &src_idx)| can_produce_colorless(&sources[src_idx]))
            .min_by_key(|&(_, &src_idx)| (sole_route(src_idx, &available),
                source_sort_key(&sources[src_idx], &hand_demand)));

        if let Some((avail_pos, &src_idx)) = best {
            let source = &sources[src_idx];
            // Find the ability that produces colorless.
            let ability_idx = ability_producing(source, ManaType::Colorless).unwrap();
            let ability = source.abilities.iter().find(|a| a.ability_index == ability_idx).unwrap();
            let colorless_produced: u32 = ability.produced.iter()
                .filter(|&&(mt, _)| mt == ManaType::Colorless)
                .map(|&(_, amount)| amount)
                .sum();
            let total_produced = ability_total_mana(ability);

            let used_for_colorless = colorless_produced.min(remaining.colorless);
            remaining.colorless -= used_for_colorless;
            // Any extra mana (colorless or otherwise) from this tap goes to excess.
            excess_mana += total_produced - used_for_colorless;
            // A cost-bearing ability has to be paid for out of the same plan.
            remaining.generic += ability_cost(ability);

            tap_plan.push((source.object_id, ability_idx));
            available.remove(avail_pos);
        } else {
            return None; // Can't satisfy colorless requirement.
        }
    }

    // Phase 2: Satisfy colored requirements (most-constrained color first).
    // Sort colored needs by scarcity: colors with fewest available sources first.
    let mut colored_needs = remaining.colored.clone();
    colored_needs.sort_by_key(|&color| {
        available.iter()
            .filter(|&&src_idx| can_produce_color(&sources[src_idx], color))
            .count()
    });

    for color in colored_needs {
        let mt = ManaType::from(color);
        // Find the best available source that can produce this color.
        //
        // What the ability costs ranks ahead of the opportunity-cost tier. A
        // filter ("{1}, {T}: Add one mana of any color") paying a pip takes a
        // second source to fund it, so it is two taps where a source that
        // makes the colour for free is one. Ranking by tier alone let
        // Shimmering Grotto (tier 1) beat Avacyn's Pilgrim (tier 3) for {W}:
        // a {1}{W} spell off Grotto + Pilgrim was then unpayable and missing
        // from the menu, and off Forest + Pilgrim + Grotto it tapped all
        // three (issue #615).
        let best = available.iter()
            .enumerate()
            .filter_map(|(pos, &src_idx)| cheapest_ability_producing(&sources[src_idx], mt)
                .map(|ability| (pos, src_idx, ability_cost(ability))))
            .min_by_key(|&(_, src_idx, cost)| (cost, source_sort_key(&sources[src_idx], &hand_demand)));

        if let Some((avail_pos, src_idx, _)) = best {
            let source = &sources[src_idx];
            let ability = cheapest_ability_producing(source, mt).unwrap();
            let ability_idx = ability.ability_index;
            let total_produced = ability_total_mana(ability);
            // 1 mana used for the colored pip, rest is excess.
            excess_mana += total_produced - 1;
            remaining.generic += ability_cost(ability);

            tap_plan.push((source.object_id, ability_idx));
            available.remove(avail_pos);
        } else {
            return None; // Can't satisfy this colored requirement.
        }
    }

    // Phase 3: Satisfy generic mana.
    // First use excess from already-tapped sources, then whatever floating
    // mana Phase 0 left over — the spell's own generic, and any `{1}` a
    // filter added above: a floating {W} funds Shimmering Grotto's `{1}`
    // for {R} without tapping a Forest to do it (issue #615).
    if remaining.generic > 0 {
        let used = excess_mana.min(remaining.generic);
        remaining.generic -= used;
    }
    if remaining.generic > 0 {
        let used = sim_pool.total().min(remaining.generic);
        remaining.generic -= used;
    }

    // Then tap more sources as needed, sorted by priority. A source whose only
    // abilities cost mana is no help here: a filter produces exactly what it
    // consumes, so it can never reduce a generic requirement.
    while remaining.generic > 0 {
        // Prefer a REDUNDANT source — one whose every producible mana type
        // another still-available source also produces — over the plain
        // priority key: spending it on generic loses no color from the
        // untapped pool. Paying {1} with the second Island while two
        // Swamps sat untapped left {B}{B} where a Swamp would have left
        // {U}{B}, silently removing the {1}{U} spell still in hand from
        // the menu (issue #84). The check needs no lookahead into the
        // hand: covered-by-what-remains dominates regardless of what the
        // leftover mana is later asked to pay.
        let is_redundant = |src_idx: usize, available: &[usize]| -> bool {
            sources[src_idx].abilities.iter()
                .flat_map(|a| a.produced.iter())
                .filter(|&&(_, amount)| amount > 0)
                .all(|&(mt, _)| available.iter().any(|&other| other != src_idx
                    && ability_producing(&sources[other], mt).is_some()))
        };
        // Generic pips are the pips ANY mana can pay, so what tapping a
        // source costs here is measured in colors lost, and that loss
        // dominates the opportunity-cost tier (issue #114: two Mountains —
        // the deck's only red — paid {1}{1} while two colorless-only
        // Stensia Bloodhalls sat untapped, because the Bloodhall's utility
        // ability out-ranked preserving red; and a Plains beat Sol Ring
        // for the same reason, over-tapping by a full land):
        //   0 = colorless-only: spending it can never lose a color;
        //   1 = every color it makes is still covered by what remains;
        //   2 = tapping it loses access to a color;
        //   3 = side effects (milling a card to save a color is not a win).
        let color_loss = |src_idx: usize, available: &[usize]| -> u8 {
            let s = &sources[src_idx];
            if s.source_kind == ManaSourceKind::HasSideEffects {
                3
            } else if source_flexibility(s) == 0 {
                0
            } else if is_redundant(src_idx, available) {
                1
            } else {
                2
            }
        };
        // Sort available sources by priority.
        let best = available.iter()
            .enumerate()
            .filter(|&(_, &src_idx)| sources[src_idx].abilities.iter()
                .any(|a| ability_total_mana(a) > ability_cost(a)))
            .min_by_key(|&(_, &src_idx)| {
                let key = source_sort_key(&sources[src_idx], &hand_demand);
                (color_loss(src_idx, &available), key.0, key.1, key.2)
            });

        if let Some((avail_pos, &src_idx)) = best {
            let source = &sources[src_idx];
            // Pick the ability to activate. For sources with multiple abilities (dual lands),
            // pick the one whose color has lowest hand demand.
            let ability_idx = source.abilities.iter()
                .filter(|a| ability_total_mana(a) > ability_cost(a))
                .min_by_key(|ability| {
                    // Score: sum of hand demand for colors produced.
                    ability.produced.iter().map(|&(mt, _)| {
                        let color = match mt {
                            ManaType::White => Some(Color::White),
                            ManaType::Blue => Some(Color::Blue),
                            ManaType::Black => Some(Color::Black),
                            ManaType::Red => Some(Color::Red),
                            ManaType::Green => Some(Color::Green),
                            ManaType::Colorless => None,
                        };
                        color.and_then(|c| hand_demand.get(&c).copied()).unwrap_or(0)
                    }).sum::<u32>()
                })
                .map(|a| a.ability_index)
                .unwrap();
            let ability = source.abilities.iter().find(|a| a.ability_index == ability_idx).unwrap();
            let total_produced = ability_total_mana(ability);

            let used = total_produced.min(remaining.generic);
            remaining.generic -= used;

            tap_plan.push((source.object_id, ability_idx));
            available.remove(avail_pos);
        } else {
            return None; // Not enough sources.
        }
    }

    free_abilities_first(&mut tap_plan, sources);
    Some(tap_plan)
}

/// Whether `cost` (its non-X part) is within reach of `pool` plus `sources`
/// by counting alone — a necessary condition for any tap plan to pay it, not
/// a plan.
///
/// Each source is tapped once, for one of its abilities, so it counts once:
/// for the total, what its best ability nets after its own cost (a filter's
/// `{1}, {T}: Add one mana of any color` nets nothing, so Shimmering Grotto is
/// one mana — its free `{C}` — and not six); for each colour, what the best
/// ability making that colour makes. Summing every ability of every source
/// counted a lone Grotto as six mana and a dual land as two, and stopped the
/// seat at every main phase for a spell it could not cast (issue #617).
///
/// Being necessary rather than exact is the point for its one caller, the
/// auto-pass gate: it can stop for a spell the planner missed, and it never
/// passes a seat past a spell the sources really could pay for.
#[must_use]
pub fn within_reach(cost: &ManaCost, pool: &ManaPool, sources: &[ManaSource]) -> bool {
    let cost = cost.without_x();
    let net = |a: &ManaAbilityDef| ability_total_mana(a).saturating_sub(ability_cost(a));
    let total: u32 = pool.total()
        + sources.iter()
            .map(|s| s.abilities.iter().map(net).max().unwrap_or(0))
            .sum::<u32>();
    if total < cost.mana_value() {
        return false;
    }
    let reach = |mana_type: ManaType| -> u32 {
        pool.get(mana_type)
            + sources.iter()
                .map(|s| s.abilities.iter()
                    .flat_map(|a| a.produced.iter())
                    .filter(|&&(mt, _)| mt == mana_type)
                    .map(|&(_, amount)| amount)
                    .max()
                    .unwrap_or(0))
                .sum::<u32>()
    };
    let colored_ok = cost.colored_requirements().into_iter()
        .all(|(color, count)| reach(ManaType::from(color)) >= count);
    colored_ok && reach(ManaType::Colorless) >= cost.colorless_amount()
}

/// Check if a mana pool can pay a given cost.
#[must_use]
pub fn can_pay(pool: &ManaPool, cost: &ManaCost) -> bool {
    // Clone pool to simulate payment.
    let mut sim = pool.clone();
    try_auto_pay(&mut sim, cost).is_ok()
}

/// Automatically pay a mana cost from a pool.
/// Deducts the mana and returns Ok, or returns Err if insufficient.
///
/// Strategy: pay colored requirements first, then colorless requirements,
/// then generic from whatever remains.
///
/// # Errors
/// Returns [`ManaError`] if the pool doesn't have enough mana to pay `cost`.
pub fn auto_pay(pool: &mut ManaPool, cost: &ManaCost) -> Result<(), ManaError> {
    try_auto_pay(pool, cost)
}

/// Pay `cost` from `pool`, spending mana that `reserve` still needs only when
/// there is nothing else to spend.
///
/// The case this exists for: a tap plan for `{W}{W}` taps Plains and Forest
/// and then activates Shimmering Grotto's `{1}, {T}: Add {W}`. Paying that
/// `{1}` "colorless first, then W, U, B, R, G" takes the White the spell
/// needs and leaves the Green spare, so the plan the engine offered could not
/// be executed and the cast was silently refused (issue #252). Ordering by
/// what the rest of the cost does not need pays it from the Green.
///
/// # Errors
/// Returns [`ManaError`] if the pool doesn't have enough mana to pay `cost`.
pub fn auto_pay_reserving(
    pool: &mut ManaPool,
    cost: &ManaCost,
    reserve: &ManaCost,
) -> Result<(), ManaError> {
    try_auto_pay_with_order(pool, cost, &generic_payment_order(pool, reserve))
}

/// The fixed order generic costs are paid in when nothing is being reserved.
const GENERIC_ORDER: [ManaType; 6] = [
    ManaType::Colorless,
    ManaType::White, ManaType::Blue, ManaType::Black,
    ManaType::Red, ManaType::Green,
];

/// Which mana to spend on a generic cost first, given what `reserve` still
/// needs from the same pool: the biggest surplus first, so mana another cost
/// depends on is spent last. Ties keep [`GENERIC_ORDER`], so a plan stays
/// deterministic.
fn generic_payment_order(pool: &ManaPool, reserve: &ManaCost) -> Vec<ManaType> {
    let mut needed: std::collections::BTreeMap<ManaType, u32> = std::collections::BTreeMap::new();
    for sym in &reserve.symbols {
        match sym {
            ManaSymbol::Colored(color) => *needed.entry(ManaType::from(*color)).or_default() += 1,
            ManaSymbol::Colorless(n) => *needed.entry(ManaType::Colorless).or_default() += n,
            // Generic can be paid with anything, so it plays no part in
            // deciding which mana is precious; X is not a cost yet.
            ManaSymbol::Generic(_) | ManaSymbol::X => {}
        }
    }
    let mut order = GENERIC_ORDER.to_vec();
    order.sort_by_key(|mt| {
        let surplus = i64::from(pool.get(*mt)) - i64::from(needed.get(mt).copied().unwrap_or(0));
        std::cmp::Reverse(surplus)
    });
    order
}

fn try_auto_pay(pool: &mut ManaPool, cost: &ManaCost) -> Result<(), ManaError> {
    try_auto_pay_with_order(pool, cost, &GENERIC_ORDER)
}

fn try_auto_pay_with_order(
    pool: &mut ManaPool,
    cost: &ManaCost,
    generic_order: &[ManaType],
) -> Result<(), ManaError> {
    // 1. Pay colored requirements.
    for sym in &cost.symbols {
        if let ManaSymbol::Colored(color) = sym {
            let mana_type = ManaType::from(*color);
            let available = pool.get(mana_type);
            if available == 0 {
                return Err(ManaError::InsufficientMana);
            }
            pool.mana.insert(mana_type, available - 1);
        }
    }

    // 2. Pay specifically colorless requirements.
    let colorless_needed = cost.colorless_amount();
    if colorless_needed > 0 {
        let available = pool.get(ManaType::Colorless);
        if available < colorless_needed {
            return Err(ManaError::InsufficientMana);
        }
        pool.mana.insert(ManaType::Colorless, available - colorless_needed);
    }

    // 3. Pay generic costs from whatever is left.
    let generic_needed = cost.generic_amount();
    if generic_needed > 0 {
        let total_remaining = pool.total();
        if total_remaining < generic_needed {
            return Err(ManaError::InsufficientMana);
        }
        let mut remaining = generic_needed;
        for &mt in generic_order {
            if remaining == 0 { break; }
            let available = pool.get(mt);
            let to_use = available.min(remaining);
            if to_use > 0 {
                pool.mana.insert(mt, available - to_use);
                remaining -= to_use;
            }
        }
        if remaining > 0 {
            return Err(ManaError::InsufficientMana);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::ManaAbilityDef;

    // ---- auto_pay tests ----

    /// Issue #252: `auto_pay_reserving` spends the mana the REST of the cost
    /// still needs last, so a plan the engine offered can actually be paid.
    ///
    /// A tap plan for `{W}{W}` that taps Plains and Forest and then activates
    /// Shimmering Grotto's `{1}, {T}: Add {W}`: the `{1}` is paid out of a
    /// pool holding the White the spell still needs and a spare Green. Paid
    /// in the fixed order it takes the White, and the cast is refused with
    /// the mana for it sitting in the pool.
    #[test]
    fn a_generic_cost_is_paid_from_what_the_rest_of_the_cost_does_not_need() {
        let mut pool = ManaPool::new();
        pool.add(ManaType::White, 1);
        pool.add(ManaType::Green, 1);
        let one = ManaCost::new(vec![ManaSymbol::Generic(1)]);
        let reserve = ManaCost::new(vec![
            ManaSymbol::Colored(Color::White),
            ManaSymbol::Colored(Color::White),
        ]);

        auto_pay_reserving(&mut pool, &one, &reserve).expect("the {1} is payable");

        assert_eq!(pool.get(ManaType::White), 1,
            "the White the rest of the cost needs is still there");
        assert_eq!(pool.get(ManaType::Green), 0, "the spare Green paid the generic");
    }

    #[test]
    fn pay_simple_colored() {
        let mut pool = ManaPool::new();
        pool.add(ManaType::Green, 2);

        let cost = ManaCost::new(vec![
            ManaSymbol::Colored(Color::Green),
            ManaSymbol::Colored(Color::Green),
        ]);

        assert!(can_pay(&pool, &cost));
        assert!(auto_pay(&mut pool, &cost).is_ok());
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn pay_generic_plus_colored() {
        let mut pool = ManaPool::new();
        pool.add(ManaType::Red, 2);

        // {1}{R}
        let cost = ManaCost::new(vec![
            ManaSymbol::Generic(1),
            ManaSymbol::Colored(Color::Red),
        ]);

        assert!(can_pay(&pool, &cost));
        assert!(auto_pay(&mut pool, &cost).is_ok());
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn insufficient_mana() {
        let mut pool = ManaPool::new();
        pool.add(ManaType::Green, 1);

        let cost = ManaCost::new(vec![
            ManaSymbol::Colored(Color::Green),
            ManaSymbol::Colored(Color::Green),
        ]);

        assert!(!can_pay(&pool, &cost));
    }

    #[test]
    fn pay_generic_with_mixed_pool() {
        let mut pool = ManaPool::new();
        pool.add(ManaType::Red, 1);
        pool.add(ManaType::Green, 2);

        // {2}{G} — should pay G from green, then 2 generic from remaining green + red
        let cost = ManaCost::new(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::Colored(Color::Green),
        ]);

        assert!(can_pay(&pool, &cost));
        assert!(auto_pay(&mut pool, &cost).is_ok());
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn wrong_color() {
        let pool_with_red = {
            let mut p = ManaPool::new();
            p.add(ManaType::Red, 2);
            p
        };

        let cost_gg = ManaCost::new(vec![
            ManaSymbol::Colored(Color::Green),
            ManaSymbol::Colored(Color::Green),
        ]);

        assert!(!can_pay(&pool_with_red, &cost_gg));
    }

    // ---- what a payment actually spends ----
    //
    // The tests above read `pool.total()`, which says a payment took the right
    // NUMBER of mana and nothing about which. These read the pool one type at
    // a time, because every interesting way to get this wrong — paying a
    // colored pip out of the wrong colour, a colorless pip out of a coloured
    // source (CR 107.4c: {C} is its own symbol, not "one generic"), a generic
    // cost out of the mana the rest of the cost still needs — leaves the total
    // exactly right.

    /// The pool as a sorted list of what is left in it, so a case can say what
    /// a payment spent rather than only how much.
    fn residue(pool: &ManaPool) -> Vec<(ManaType, u32)> {
        GENERIC_ORDER.iter()
            .map(|&mt| (mt, pool.get(mt)))
            .filter(|&(_, n)| n > 0)
            .collect()
    }

    fn pool_of(entries: &[(ManaType, u32)]) -> ManaPool {
        let mut p = ManaPool::new();
        for &(mt, n) in entries {
            p.add(mt, n);
        }
        p
    }

    #[test]
    fn a_colored_pip_spends_exactly_one_of_exactly_that_color() {
        let mut pool = pool_of(&[(ManaType::White, 2), (ManaType::Green, 1)]);
        assert!(auto_pay(&mut pool, &ManaCost::new(vec![ManaSymbol::Colored(Color::White)])).is_ok());
        assert_eq!(residue(&pool), vec![(ManaType::White, 1), (ManaType::Green, 1)]);
    }

    #[test]
    fn a_colored_pip_is_not_payable_out_of_another_color() {
        let pool = pool_of(&[(ManaType::Green, 5)]);
        assert!(!can_pay(&pool, &ManaCost::new(vec![ManaSymbol::Colored(Color::White)])));
    }

    /// CR 107.4c: {C} is a symbol in its own right — "one colorless mana", not
    /// "one generic". Coloured mana cannot pay it however much of it there is.
    #[test]
    fn a_colorless_pip_spends_colorless_and_only_colorless() {
        let mut pool = pool_of(&[(ManaType::Colorless, 3), (ManaType::Green, 3)]);
        assert!(auto_pay(&mut pool, &ManaCost::new(vec![ManaSymbol::Colorless(2)])).is_ok());
        assert_eq!(residue(&pool), vec![(ManaType::Colorless, 1), (ManaType::Green, 3)]);

        // One {C} out of three leaves two, not three: the amount is taken off
        // the pool, not divided into it.
        let mut pool = pool_of(&[(ManaType::Colorless, 3)]);
        assert!(auto_pay(&mut pool, &ManaCost::new(vec![ManaSymbol::Colorless(1)])).is_ok());
        assert_eq!(residue(&pool), vec![(ManaType::Colorless, 2)]);

        let coloured_only = pool_of(&[(ManaType::Green, 5)]);
        assert!(!can_pay(&coloured_only, &ManaCost::new(vec![ManaSymbol::Colorless(1)])));
    }

    #[test]
    fn a_colorless_requirement_one_short_is_not_payable() {
        let pool = pool_of(&[(ManaType::Colorless, 1), (ManaType::Green, 5)]);
        assert!(!can_pay(&pool, &ManaCost::new(vec![ManaSymbol::Colorless(2)])));
        assert!(can_pay(&pool, &ManaCost::new(vec![ManaSymbol::Colorless(1)])));
    }

    /// Generic is paid colorless first and then in a fixed colour order, so a
    /// tap plan the engine offers is the one the payment executes.
    #[test]
    fn generic_is_paid_colorless_first_and_then_in_a_fixed_order() {
        let mut pool = pool_of(&[
            (ManaType::Colorless, 1), (ManaType::White, 1),
            (ManaType::Blue, 1), (ManaType::Green, 1),
        ]);
        assert!(auto_pay(&mut pool, &ManaCost::new(vec![ManaSymbol::Generic(3)])).is_ok());
        assert_eq!(residue(&pool), vec![(ManaType::Green, 1)],
            "colorless, then white, then blue — green is last in GENERIC_ORDER");
    }

    #[test]
    fn generic_may_take_the_whole_pool_but_not_more_than_it() {
        let mut pool = pool_of(&[(ManaType::White, 1), (ManaType::Green, 1)]);
        assert!(!can_pay(&pool, &ManaCost::new(vec![ManaSymbol::Generic(3)])));
        assert!(auto_pay(&mut pool, &ManaCost::new(vec![ManaSymbol::Generic(2)])).is_ok());
        assert_eq!(residue(&pool), vec![]);
    }

    /// Issue #252: the mana another cost still needs is spent last, so a plan
    /// that taps Plains and Forest and then filters through Shimmering Grotto
    /// pays the filter's {1} out of the Green rather than the White the spell
    /// is for.
    #[test]
    fn a_reserved_color_is_the_last_thing_a_generic_cost_spends() {
        let mut pool = pool_of(&[(ManaType::White, 1), (ManaType::Green, 1)]);
        let one = ManaCost::new(vec![ManaSymbol::Generic(1)]);
        let reserve_white = ManaCost::new(vec![ManaSymbol::Colored(Color::White)]);
        assert!(auto_pay_reserving(&mut pool, &one, &reserve_white).is_ok());
        assert_eq!(residue(&pool), vec![(ManaType::White, 1)]);

        // And with nothing reserved the fixed order applies, so the same pool
        // and the same cost spend the White instead.
        let mut pool = pool_of(&[(ManaType::White, 1), (ManaType::Green, 1)]);
        assert!(auto_pay(&mut pool, &one).is_ok());
        assert_eq!(residue(&pool), vec![(ManaType::Green, 1)]);
    }

    /// The same for a reserved `{C}`: true colorless is as precious as a
    /// colour, and for the same reason.
    ///
    /// `auto_pay_reserving`'s promise to its callers is that a cost paid
    /// through it leaves the reserve payable — issue #252 is what happens
    /// when it does not. The reserve scan reads two kinds of symbol and only
    /// the coloured one was covered, so the colorless arm could stop counting
    /// entirely and the suite stayed green (mutants issue #549). Nothing in
    /// the pool costs `{C}` today, but this is the function's contract rather
    /// than any card's, and it is the arm that decides it.
    #[test]
    fn a_reserved_colorless_cost_is_the_last_thing_a_generic_cost_spends() {
        let mut pool = pool_of(&[(ManaType::Colorless, 1), (ManaType::Green, 1)]);
        let one = ManaCost::new(vec![ManaSymbol::Generic(1)]);
        let reserve_c = ManaCost::new(vec![ManaSymbol::Colorless(1)]);

        assert!(auto_pay_reserving(&mut pool, &one, &reserve_c).is_ok());
        assert_eq!(residue(&pool), vec![(ManaType::Colorless, 1)],
            "the {{C}} the rest of the cost still needs is what is left");
        assert!(auto_pay(&mut pool, &reserve_c).is_ok(),
            "and so the reserved cost can still be paid, which is the promise");
    }

    /// A generic symbol in the reserved cost reserves nothing — it can be paid
    /// with anything, so no colour is precious on its account.
    #[test]
    fn a_reserved_generic_cost_makes_no_color_precious() {
        let mut pool = pool_of(&[(ManaType::White, 1), (ManaType::Green, 1)]);
        let one = ManaCost::new(vec![ManaSymbol::Generic(1)]);
        assert!(auto_pay_reserving(&mut pool, &one, &ManaCost::new(vec![ManaSymbol::Generic(1)])).is_ok());
        assert_eq!(residue(&pool), vec![(ManaType::Green, 1)],
            "the fixed order, unchanged: white is spent before green");
    }

    // ---- how a source is scored ----

    #[test]
    fn a_source_produces_colorless_only_if_an_ability_says_colorless() {
        let colorless = make_source(1, ManaSourceKind::BasicMana,
            vec![mono_ability(ManaType::Colorless)]);
        let green = make_source(2, ManaSourceKind::BasicMana,
            vec![mono_ability(ManaType::Green)]);
        assert!(can_produce_colorless(&colorless));
        assert!(!can_produce_colorless(&green), "a Forest is not a colorless source");
        assert!(can_produce_color(&green, Color::Green));
        assert!(!can_produce_color(&colorless, Color::Green));
    }

    // ---- the tap planner's one contract ----
    //
    // `compute_autotap` picks WHICH sources to tap, and that choice is a
    // heuristic — colour preservation, utility lands last, filters after the
    // sources that fund them. It has been retuned twice (issues #114, #252)
    // and will be again, so the cases below pin the contract rather than the
    // preference: a plan the planner offers is one the payment can actually
    // execute, and a `None` means no plan existed.
    //
    // That second half is the one issue #252 broke: the engine offered a plan,
    // the payment could not run it, and the cast was silently refused. Pinning
    // each arithmetic step of the internal simulation would have caught it and
    // frozen the heuristic; this catches it and leaves the heuristic free.

    /// Run a tap plan and say whether what it produces pays `cost`.
    ///
    /// The plan's order is part of it: a filter's own cost is paid out of the
    /// pool the earlier entries filled, which is what `free_abilities_first`
    /// is for.
    fn plan_pays(
        plan: &[(ObjectId, usize)],
        pool: &ManaPool,
        sources: &[ManaSource],
        cost: &ManaCost,
    ) -> bool {
        let mut pool = pool.clone();
        for &(object_id, ability_index) in plan {
            let ability = sources.iter()
                .find(|s| s.object_id == object_id)
                .and_then(|s| s.abilities.iter().find(|a| a.ability_index == ability_index))
                .expect("a plan names an ability its source has");
            if auto_pay_reserving(&mut pool, &ability.cost, cost).is_err() {
                return false;
            }
            for &(mana_type, amount) in &ability.produced {
                pool.add(mana_type, amount);
            }
        }
        can_pay(&pool, cost)
    }

    /// Every way to tap some subset of `sources`, one ability each.
    fn every_plan(sources: &[ManaSource]) -> Vec<Vec<(ObjectId, usize)>> {
        let mut plans = vec![vec![]];
        for source in sources {
            let mut next = Vec::new();
            for plan in &plans {
                next.push(plan.clone());
                for ability in &source.abilities {
                    let mut p = plan.clone();
                    p.push((source.object_id, ability.ability_index));
                    next.push(p);
                }
            }
            plans = next;
        }
        plans
    }

    fn filter_ability(index: usize, produces: ManaType) -> ManaAbilityDef {
        ManaAbilityDef {
            ability_index: index,
            description: format!("{{1}}, {{T}}: Add {produces:?}"),
            produced: vec![(produces, 1)],
            requires_tap: true,
            cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
            has_side_effects: false,
        }
    }

    /// The boards worth asking about: enough colours to make the choice real,
    /// a filter whose cost has to come from another source in the same plan,
    /// and a source that makes two mana at once.
    fn planner_cases() -> Vec<(&'static str, ManaPool, Vec<ManaSource>)> {
        let sol_ring = ManaAbilityDef {
            ability_index: 0,
            description: "Add {C}{C}".into(),
            produced: vec![(ManaType::Colorless, 2)],
            requires_tap: true,
            cost: ManaCost::free(),
            has_side_effects: false,
        };
        vec![
            ("two Forests", ManaPool::new(), vec![
                make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
                make_source(2, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
            ]),
            ("Plains, Forest, Mountain", ManaPool::new(), vec![
                make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::White)]),
                make_source(2, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
                make_source(3, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Red)]),
            ]),
            ("a dual and a Forest", ManaPool::new(), vec![
                make_source(1, ManaSourceKind::BasicMana, dual_abilities(ManaType::Red, ManaType::Green)),
                make_source(2, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
            ]),
            ("Plains, Forest and a filter", ManaPool::new(), vec![
                make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::White)]),
                make_source(2, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
                make_source(3, ManaSourceKind::HasUtilityAbility, vec![filter_ability(0, ManaType::Blue)]),
            ]),
            ("a two-mana rock and a Swamp", ManaPool::new(), vec![
                make_source(1, ManaSourceKind::BasicMana, vec![sol_ring]),
                make_source(2, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Black)]),
            ]),
            ("a Forest, with a floating W", {
                let mut p = ManaPool::new();
                p.add(ManaType::White, 1);
                p
            }, vec![
                make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
            ]),
            // Issue #615: the filter out-ranked the creature for the pip and
            // then had nothing left to fund its own {1}.
            ("a Shimmering Grotto and a mana creature", ManaPool::new(), vec![
                make_source(1, ManaSourceKind::NonBasicMana, grotto_abilities()),
                make_source(2, ManaSourceKind::Creature, vec![mono_ability(ManaType::Green)]),
            ]),
            ("a Shimmering Grotto, a Plains and a mana creature", ManaPool::new(), vec![
                make_source(1, ManaSourceKind::NonBasicMana, grotto_abilities()),
                make_source(2, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::White)]),
                make_source(3, ManaSourceKind::Creature, vec![mono_ability(ManaType::Green)]),
            ]),
            // Issue #615: mana left floating can fund the filter's {1}.
            ("a Shimmering Grotto, with a floating W", {
                let mut p = ManaPool::new();
                p.add(ManaType::White, 1);
                p
            }, vec![
                make_source(1, ManaSourceKind::NonBasicMana, grotto_abilities()),
            ]),
        ]
    }

    /// Shimmering Grotto: `{T}: Add {C}` and `{1}, {T}: Add one mana of any
    /// color`, one entry per colour, as the card declares them.
    fn grotto_abilities() -> Vec<ManaAbilityDef> {
        let mut abilities = vec![mono_ability(ManaType::Colorless)];
        for (i, mt) in [ManaType::White, ManaType::Blue, ManaType::Black,
                        ManaType::Red, ManaType::Green].into_iter().enumerate() {
            abilities.push(filter_ability(i + 1, mt));
        }
        abilities
    }

    fn planner_costs() -> Vec<ManaCost> {
        use ManaSymbol::{Colored, Colorless, Generic};
        vec![
            ManaCost::new(vec![Colored(Color::Green)]),
            ManaCost::new(vec![Colored(Color::Green), Colored(Color::Green)]),
            ManaCost::new(vec![Generic(1), Colored(Color::Green)]),
            ManaCost::new(vec![Generic(2)]),
            ManaCost::new(vec![Generic(3)]),
            ManaCost::new(vec![Colorless(1)]),
            ManaCost::new(vec![Colorless(2)]),
            ManaCost::new(vec![Colored(Color::White), Colored(Color::Green)]),
            ManaCost::new(vec![Generic(1), Colored(Color::Blue)]),
            ManaCost::new(vec![Colored(Color::Blue), Colored(Color::Blue)]),
            ManaCost::new(vec![Generic(4), Colored(Color::Black)]),
        ]
    }

    /// A plan the planner offers is a plan the payment can run.
    #[test]
    fn every_tap_plan_the_planner_offers_pays_the_cost_it_was_asked_for() {
        for (label, pool, sources) in planner_cases() {
            for cost in planner_costs() {
                let Some(plan) = compute_autotap(&cost, &pool, &sources, &[]) else { continue };
                assert!(plan_pays(&plan, &pool, &sources, &cost),
                    "{label}: the plan {plan:?} offered for {cost} does not pay it");
                let mut seen = std::collections::HashSet::new();
                assert!(plan.iter().all(|&(id, _)| seen.insert(id)),
                    "{label}: {plan:?} taps one source twice for {cost}");
            }
        }
    }

    /// And a `None` means no plan existed — checked against every plan there
    /// is, so a planner that gives up early is caught rather than trusted.
    #[test]
    fn the_planner_declines_only_when_no_tap_plan_would_have_worked() {
        for (label, pool, sources) in planner_cases() {
            for cost in planner_costs() {
                if compute_autotap(&cost, &pool, &sources, &[]).is_some() { continue }
                let worked: Vec<_> = every_plan(&sources).into_iter()
                    .filter(|p| plan_pays(p, &pool, &sources, &cost))
                    .collect();
                assert!(worked.is_empty(),
                    "{label}: no plan offered for {cost}, but {:?} pays it", worked.first());
            }
        }
    }

    /// `within_reach` is a necessary condition: whenever some tap plan pays a
    /// cost, the count says it is within reach. The auto-pass gate relies on
    /// that — a `false` there passes the seat.
    #[test]
    fn within_reach_never_rules_out_a_cost_some_plan_pays() {
        for (label, pool, sources) in planner_cases() {
            for cost in planner_costs() {
                let paid = every_plan(&sources).into_iter()
                    .map(|mut p| { free_abilities_first(&mut p, &sources); p })
                    .any(|p| plan_pays(&p, &pool, &sources, &cost));
                if paid {
                    assert!(within_reach(&cost, &pool, &sources),
                        "{label}: a plan pays {cost}, but within_reach says no");
                }
            }
        }
    }

    /// Issue #617: a source counts once, for one ability, net of its cost. A
    /// lone Shimmering Grotto is one mana, not six; a dual land is one, not
    /// two.
    #[test]
    fn within_reach_counts_each_source_once_net_of_its_cost() {
        let one_w = ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::White)]);
        let grotto = vec![make_source(1, ManaSourceKind::NonBasicMana, grotto_abilities())];
        assert!(!within_reach(&one_w, &ManaPool::new(), &grotto),
            "one Grotto cannot pay {{1}}{{W}}");
        let dual = vec![make_source(1, ManaSourceKind::NonBasicMana,
            dual_abilities(ManaType::Red, ManaType::White))];
        assert!(!within_reach(&one_w, &ManaPool::new(), &dual),
            "one dual land cannot pay {{1}}{{W}}");
        let w = ManaCost::new(vec![ManaSymbol::Colored(Color::White)]);
        assert!(within_reach(&w, &ManaPool::new(), &dual), "but it pays {{W}}");
    }

    /// Reach is per colour as well as in total: two Forests are two mana
    /// and still cannot pay {R}, nor {C}. A count that let enough mana of
    /// the wrong kind stand in stopped the auto-pass gate at every main
    /// phase for a spell the seat could not cast — #617's symptom by
    /// another route (issue #661).
    #[test]
    fn a_colour_no_source_makes_is_out_of_reach_however_much_mana_there_is() {
        let forests = vec![
            make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
            make_source(2, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
        ];
        for (cost, what) in [
            (ManaCost::new(vec![ManaSymbol::Colored(Color::Red)]), "{R}"),
            (ManaCost::new(vec![ManaSymbol::Colorless(1)]), "{C}"),
        ] {
            assert!(!within_reach(&cost, &ManaPool::new(), &forests), "two Forests cannot pay {what}");
        }
        let gg = ManaCost::new(vec![ManaSymbol::Colored(Color::Green), ManaSymbol::Colored(Color::Green)]);
        assert!(within_reach(&gg, &ManaPool::new(), &forests), "but they pay {{G}}{{G}}");
    }

    /// A plan taps no more sources than the fewest that would have paid.
    ///
    /// Issue #615: a `{1}{W}` spell off Forest, Avacyn's Pilgrim and
    /// Shimmering Grotto tapped all three — the filter took the pip, which
    /// the Pilgrim could have paid for free, and then its own `{1}` needed a
    /// third source. Which sources are tapped is the heuristic's business;
    /// how many is not.
    #[test]
    fn a_plan_taps_no_more_sources_than_the_fewest_that_would_pay() {
        for (label, pool, sources) in planner_cases() {
            for cost in planner_costs() {
                let Some(plan) = compute_autotap(&cost, &pool, &sources, &[]) else { continue };
                // Funded abilities after the free ones that fund them, as a
                // real plan runs.
                let fewest = every_plan(&sources).into_iter()
                    .map(|mut p| { free_abilities_first(&mut p, &sources); p })
                    .filter(|p| plan_pays(p, &pool, &sources, &cost))
                    .map(|p| p.len())
                    .min()
                    .expect("the planner's own plan pays");
                assert_eq!(plan.len(), fewest,
                    "{label}: {plan:?} taps {} sources for {cost}, but {fewest} would do",
                    plan.len());
            }
        }
    }

    /// Floating mana counts toward the cost, so a Forest and a floating {W}
    /// cast a {W}{G} spell by tapping once.
    #[test]
    fn the_planner_spends_the_pool_before_it_taps_anything() {
        let mut pool = ManaPool::new();
        pool.add(ManaType::White, 1);
        let sources = vec![
            make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
            make_source(2, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::White)]),
        ];
        let cost = ManaCost::new(vec![
            ManaSymbol::Colored(Color::White), ManaSymbol::Colored(Color::Green)]);
        let plan = compute_autotap(&cost, &pool, &sources, &[]).expect("castable");
        // The count and the untapped Plains, not the exact entry: which
        // ability index a source is tapped for is the planner's business.
        assert_eq!(plan.len(), 1, "one tap, because the pool already pays the {{W}}");
        assert!(!plan.iter().any(|&(id, _)| id == ObjectId(2)),
            "and it is not the Plains: {plan:?}");
    }

    // ---- autotap tests ----

    /// Issue #684: a `{C}` pip paid with the free `{C}` of the only source
    /// that could also make the coloured pip left the colour unpayable.
    /// Each board pays by hand: the Assistant's `{C}` for the pip, and the
    /// Grotto filtering other mana into the colour.
    #[test]
    fn a_colorless_pip_spares_the_only_route_to_a_colour() {
        let assistant = || {
            let mut a = mono_ability(ManaType::Colorless);
            a.has_side_effects = true;
            make_source(2, ManaSourceKind::HasSideEffects, vec![a])
        };
        let cost = |color: Color| ManaCost::new(vec![ManaSymbol::Colorless(1), ManaSymbol::Colored(color)]);
        let floating = |mt: ManaType| { let mut p = ManaPool::new(); p.add(mt, 1); p };
        let boards: Vec<(&str, ManaPool, Vec<ManaSource>, ManaCost)> = vec![
            ("Grotto, Assistant, {G} floating, {C}{R}", floating(ManaType::Green), vec![
                make_source(1, ManaSourceKind::NonBasicMana, grotto_abilities()), assistant()], cost(Color::Red)),
            ("Grotto, Assistant, {U} floating, {C}{W}", floating(ManaType::Blue), vec![
                make_source(1, ManaSourceKind::NonBasicMana, grotto_abilities()), assistant()], cost(Color::White)),
            ("Grotto, Pilgrim(R), Assistant, {C}{B}", ManaPool::new(), vec![
                make_source(1, ManaSourceKind::NonBasicMana, grotto_abilities()),
                make_source(3, ManaSourceKind::Creature, vec![mono_ability(ManaType::Red)]),
                assistant()], cost(Color::Black)),
        ];
        for (what, pool, sources, cost) in boards {
            let plan = compute_autotap(&cost, &pool, &sources, &[])
                .unwrap_or_else(|| panic!("{what}: no plan offered, but tapping by hand pays"));
            assert!(pool_after(&plan, &pool, &sources, &cost, &ManaCost::free()).is_some(),
                "{what}: the plan {plan:?} does not pay");
        }

        // And where another source can make the colour, the Grotto's free
        // `{C}` is still preferred to milling a card for it.
        let sources = vec![
            make_source(1, ManaSourceKind::NonBasicMana, grotto_abilities()),
            assistant(),
            make_source(4, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Red)]),
        ];
        let plan = compute_autotap(&cost(Color::Red), &ManaPool::new(), &sources, &[]).expect("payable");
        assert!(!plan.iter().any(|(id, _)| *id == ObjectId(2)), "no card milled when a Mountain makes the {{R}}: {plan:?}");
    }

    fn make_source(id: u64, kind: ManaSourceKind, abilities: Vec<ManaAbilityDef>) -> ManaSource {
        ManaSource {
            object_id: ObjectId(id),
            abilities,
            source_kind: kind,
        }
    }

    fn mono_ability(mana_type: ManaType) -> ManaAbilityDef {
        ManaAbilityDef {
            ability_index: 0,
            description: format!("Add {mana_type:?}"),
            produced: vec![(mana_type, 1)],
            requires_tap: true,
            cost: ManaCost::free(),
            has_side_effects: false,
        }
    }

    fn dual_abilities(mt1: ManaType, mt2: ManaType) -> Vec<ManaAbilityDef> {
        vec![
            ManaAbilityDef {
                ability_index: 0,
                description: format!("Add {mt1:?}"),
                produced: vec![(mt1, 1)],
                requires_tap: true,
                cost: ManaCost::free(),
                has_side_effects: false,
            },
            ManaAbilityDef {
                ability_index: 1,
                description: format!("Add {mt2:?}"),
                produced: vec![(mt2, 1)],
                requires_tap: true,
                cost: ManaCost::free(),
                has_side_effects: false,
            },
        ]
    }

    #[test]
    fn autotap_basic_three_forests() {
        // Cost {1}{G}{G}, 3 Forests available.
        let cost = ManaCost::new(vec![
            ManaSymbol::Generic(1),
            ManaSymbol::Colored(Color::Green),
            ManaSymbol::Colored(Color::Green),
        ]);
        let sources = vec![
            make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
            make_source(2, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
            make_source(3, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
        ];
        let result = compute_autotap(&cost, &ManaPool::new(), &sources, &[]);
        assert!(result.is_some());
        let plan = result.unwrap();
        assert_eq!(plan.len(), 3);
    }

    #[test]
    fn autotap_creature_used_when_only_source() {
        // Cost {W}, only source is Avacyn's Pilgrim. Must tap it.
        let cost = ManaCost::new(vec![ManaSymbol::Colored(Color::White)]);
        let sources = vec![
            make_source(1, ManaSourceKind::Creature, vec![mono_ability(ManaType::White)]),
        ];
        let plan = compute_autotap(&cost, &ManaPool::new(), &sources, &[]).unwrap();
        assert_eq!(plan[0].0, ObjectId(1));
    }

    #[test]
    fn autotap_colorless_specific() {
        // Cost {C}, sources: Sol Ring + Forest. Should use Sol Ring (produces colorless).
        let cost = ManaCost::new(vec![ManaSymbol::Colorless(1)]);
        let sol_ring = ManaAbilityDef {
            ability_index: 0,
            description: "Add {C}{C}".into(),
            produced: vec![(ManaType::Colorless, 2)],
            requires_tap: true,
            cost: ManaCost::free(),
            has_side_effects: false,
        };
        let sources = vec![
            make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
            make_source(2, ManaSourceKind::BasicMana, vec![sol_ring]),
        ];
        let plan = compute_autotap(&cost, &ManaPool::new(), &sources, &[]).unwrap();
        assert_eq!(plan[0].0, ObjectId(2)); // Sol Ring
    }

    #[test]
    fn autotap_floating_mana() {
        // Pool has {G}, cost {G}{G}, 1 Forest. Should tap 1 Forest.
        let cost = ManaCost::new(vec![
            ManaSymbol::Colored(Color::Green),
            ManaSymbol::Colored(Color::Green),
        ]);
        let mut pool = ManaPool::new();
        pool.add(ManaType::Green, 1);
        let sources = vec![
            make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
        ];
        let plan = compute_autotap(&cost, &pool, &sources, &[]).unwrap();
        assert_eq!(plan.len(), 1);
    }

    #[test]
    fn autotap_multi_mana_source() {
        // Cost {2}, Sol Ring produces 2 colorless. One tap should suffice.
        let cost = ManaCost::new(vec![ManaSymbol::Generic(2)]);
        let sol_ring = ManaAbilityDef {
            ability_index: 0,
            description: "Add {C}{C}".into(),
            produced: vec![(ManaType::Colorless, 2)],
            requires_tap: true,
            cost: ManaCost::free(),
            has_side_effects: false,
        };
        let sources = vec![
            make_source(1, ManaSourceKind::BasicMana, vec![sol_ring]),
        ];
        let plan = compute_autotap(&cost, &ManaPool::new(), &sources, &[]).unwrap();
        assert_eq!(plan.len(), 1);
    }

    /// Issue #114's symptom, which is what the hand-preservation heuristic is
    /// FOR: a plan for the spell being cast must not strand a spell still in
    /// hand that the remaining sources could have paid for.
    ///
    /// Stated as the symptom rather than as the choice. The heuristic behind
    /// it — colour demand, opportunity-cost tiers, mono before dual — is a
    /// knob that has been turned twice already, and a test naming which land
    /// gets tapped freezes the knob instead of the promise
    /// (docs/mutation-testing-guide.md).
    #[test]
    fn a_plan_does_not_strand_a_spell_the_rest_of_the_board_could_pay_for() {
        // (what the board is, cast this, with this still in hand)
        let cases: [(&str, Vec<ManaSource>, ManaCost, ManaCost); 3] = [
            ("Forest and a Green/Blue dual",
             vec![make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
                  make_source(2, ManaSourceKind::NonBasicMana,
                              dual_abilities(ManaType::Green, ManaType::Blue))],
             ManaCost::new(vec![ManaSymbol::Colored(Color::Green)]),
             ManaCost::new(vec![ManaSymbol::Colored(Color::Blue)])),
            ("two Mountains, a Swamp and two colorless utility lands",
             vec![make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Red)]),
                  make_source(2, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Red)]),
                  make_source(3, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Black)]),
                  make_source(4, ManaSourceKind::HasUtilityAbility, vec![mono_ability(ManaType::Colorless)]),
                  make_source(5, ManaSourceKind::HasUtilityAbility, vec![mono_ability(ManaType::Colorless)])],
             ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Black)]),
             ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Red)])),
            ("a Plains and a colorless rock",
             vec![make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::White)]),
                  make_source(2, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::White)]),
                  make_source(3, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Colorless)])],
             ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::White)]),
             ManaCost::new(vec![ManaSymbol::Colored(Color::White)])),
        ];

        for (board, sources, casting, in_hand) in cases {
            let plan = compute_autotap(&casting, &ManaPool::new(), &sources, &[in_hand.clone()])
                .unwrap_or_else(|| panic!("{board}: the spell being cast is payable"));
            assert!(plan_pays(&plan, &ManaPool::new(), &sources, &casting),
                "{board}: the plan pays what it was asked for");

            let tapped: Vec<ObjectId> = plan.iter().map(|&(id, _)| id).collect();
            let left: Vec<ManaSource> = sources.iter()
                .filter(|s| !tapped.contains(&s.object_id))
                .cloned()
                .collect();
            assert!(compute_autotap(&in_hand, &ManaPool::new(), &left, &[]).is_some(),
                "{board}: {in_hand} was still castable off the untapped sources before \
                 the plan for {casting} took them — it is not now (issue #114)");
        }
    }

    /// The same promise on the boards where the tier key decided wrongly
    /// and a plan paid its cost while silently taking a castable spell off
    /// the menu — checked after the payment, out of what the plan leaves:
    /// - an unfunded Shimmering Grotto filter counted as still making White,
    ///   so `{1}{G}` spent the Plains (or the Pilgrim) the `{W}` needed and
    ///   not the Grotto's free `{C}` (issue #674, and its comment's Gavony
    ///   Township board);
    /// - the creature tier kept Avacyn's Pilgrim untapped by tapping the
    ///   only red source for Chapel Geist's second `{W}` (issue #679);
    /// - floating `{R}` paid a generic pip that a floating or untapped
    ///   `{G}` could have (issue #678).
    #[test]
    fn a_plan_keeps_the_hand_castable_when_some_plan_would() {
        use ManaSymbol::{Colored, Generic};
        let w = || mono_ability(ManaType::White);
        let floating = |e: &[(ManaType, u32)]| pool_of(e);
        let forest = |id| make_source(id, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]);
        let plains = |id| make_source(id, ManaSourceKind::BasicMana, vec![w()]);
        let mountain = |id| make_source(id, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Red)]);
        let grotto = |id| make_source(id, ManaSourceKind::NonBasicMana, grotto_abilities());
        let pilgrim = |id| make_source(id, ManaSourceKind::Creature, vec![w()]);
        let clifftop = |id| make_source(id, ManaSourceKind::NonBasicMana,
            dual_abilities(ManaType::Red, ManaType::White));
        let bears = ManaCost::new(vec![Generic(1), Colored(Color::Green)]);
        let traveler = ManaCost::new(vec![Colored(Color::White)]);
        let geistflame = ManaCost::new(vec![Colored(Color::Red)]);
        let chapel_geist = ManaCost::new(vec![Generic(1), Colored(Color::White), Colored(Color::White)]);
        let township = ManaCost::new(vec![Generic(2), Colored(Color::Green), Colored(Color::White)]);
        // (board, pool, cast this, with this in hand)
        let cases: Vec<(&str, ManaPool, Vec<ManaSource>, ManaCost, ManaCost)> = vec![
            ("#674 Forest, Plains, Grotto", ManaPool::new(),
             vec![forest(1), plains(2), grotto(3)], bears.clone(), traveler.clone()),
            ("#674 Forest, Pilgrim, Grotto", ManaPool::new(),
             vec![forest(1), pilgrim(2), grotto(3)], bears.clone(), traveler.clone()),
            ("#674 Township's cost off Grotto, Forest, Plains, Pilgrim, Mountain", ManaPool::new(),
             vec![grotto(1), forest(2), plains(3), pilgrim(4), mountain(5)], township, traveler),
            ("#679 Plains, Pilgrim, Clifftop Retreat, Forest", ManaPool::new(),
             vec![plains(1), pilgrim(2), clifftop(3), forest(4)], chapel_geist.clone(), geistflame.clone()),
            ("#678 R G G floating", floating(&[(ManaType::Red, 1), (ManaType::Green, 2)]),
             vec![], bears.clone(), geistflame.clone()),
            ("#678 W R floating, Plains, Forest", floating(&[(ManaType::White, 1), (ManaType::Red, 1)]),
             vec![plains(1), forest(2)], chapel_geist, geistflame.clone()),
            ("#678 R floating, two Forests", floating(&[(ManaType::Red, 1)]),
             vec![forest(1), forest(2)], bears, geistflame),
        ];
        for (board, pool, sources, casting, in_hand) in cases {
            assert!(compute_autotap(&in_hand, &pool, &sources, &[]).is_some(),
                "{board}: {in_hand} is castable to begin with");
            let plan = compute_autotap(&casting, &pool, &sources, &[in_hand.clone()])
                .unwrap_or_else(|| panic!("{board}: {casting} is payable"));
            let left = pool_after(&plan, &pool, &sources, &casting, &hand_reserve(&[in_hand.clone()]))
                .unwrap_or_else(|| panic!("{board}: the plan {plan:?} pays {casting}"));
            let untapped: Vec<ManaSource> = sources.iter()
                .filter(|s| !plan.iter().any(|(id, _)| *id == s.object_id))
                .cloned()
                .collect();
            assert!(compute_autotap(&in_hand, &left, &untapped, &[]).is_some(),
                "{board}: after {plan:?} pays {casting}, {in_hand} is no longer castable \
                 (pool left {left:?})");
        }
    }

    #[test]
    fn autotap_insufficient() {
        // Cost {G}{G}, only 1 Forest. Should return None.
        let cost = ManaCost::new(vec![
            ManaSymbol::Colored(Color::Green),
            ManaSymbol::Colored(Color::Green),
        ]);
        let sources = vec![
            make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
        ];
        assert!(compute_autotap(&cost, &ManaPool::new(), &sources, &[]).is_none());
    }

    #[test]
    fn autotap_free_spell() {
        // Cost {0} (empty). Should return empty plan.
        let cost = ManaCost::free();
        let plan = compute_autotap(&cost, &ManaPool::new(), &[], &[]).unwrap();
        assert!(plan.is_empty());
    }

    #[test]
    fn autotap_x_spell_returns_none() {
        // X-cost spells should not be autotapped.
        let cost = ManaCost::new(vec![ManaSymbol::X, ManaSymbol::Colored(Color::Red)]);
        let sources = vec![
            make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Red)]),
        ];
        assert!(compute_autotap(&cost, &ManaPool::new(), &sources, &[]).is_none());
    }

}
