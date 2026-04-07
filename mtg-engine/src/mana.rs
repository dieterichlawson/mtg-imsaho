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
/// Lower = prefer tapping first.
fn source_sort_key(source: &ManaSource, hand_demand: &std::collections::HashMap<Color, u32>) -> (ManaSourceKind, usize, u32) {
    (source.source_kind, source_flexibility(source), hand_demand_score(source, hand_demand))
}

/// Build hand demand map: for each color, how many colored pips across all hand_costs.
fn build_hand_demand(hand_costs: &[ManaCost]) -> std::collections::HashMap<Color, u32> {
    let mut demand = std::collections::HashMap::new();
    for cost in hand_costs {
        for (color, count) in cost.colored_requirements() {
            *demand.entry(color).or_insert(0) += count;
        }
    }
    demand
}

/// Check if a source can produce a specific mana type, and return the ability_index if so.
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

/// Compute the optimal set of mana sources to tap in order to pay a cost.
///
/// Returns `Some(tap_plan)` with (object_id, ability_index) pairs, or `None` if
/// the cost cannot be paid with available sources + floating mana.
///
/// `hand_costs` are the mana costs of OTHER castable spells in the player's hand,
/// used for color preservation (prefer not tapping sources needed by other spells).
pub fn compute_autotap(
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

    // Deduct floating mana: generic (colorless first, then colors).
    if remaining.generic > 0 {
        let types = [
            ManaType::Colorless,
            ManaType::White, ManaType::Blue, ManaType::Black,
            ManaType::Red, ManaType::Green,
        ];
        for mt in types {
            if remaining.generic == 0 { break; }
            let available = sim_pool.get(mt);
            let deduct = available.min(remaining.generic);
            if deduct > 0 {
                sim_pool.mana.insert(mt, available - deduct);
                remaining.generic -= deduct;
            }
        }
    }

    // If everything is satisfied by floating mana, done.
    if remaining.colored.is_empty() && remaining.colorless == 0 && remaining.generic == 0 {
        return Some(vec![]);
    }

    // Build mutable list of available sources (indices into the original slice).
    let mut available: Vec<usize> = (0..sources.len()).collect();
    let mut tap_plan: Vec<(ObjectId, usize)> = Vec::new();
    let mut excess_mana: u32 = 0; // excess mana from already-tapped multi-mana sources

    // Phase 1: Satisfy colorless-specific requirements ({C}).
    while remaining.colorless > 0 {
        // Sort available sources that can produce colorless by priority.
        let best = available.iter()
            .enumerate()
            .filter(|&(_, &src_idx)| can_produce_colorless(&sources[src_idx]))
            .min_by_key(|&(_, &src_idx)| source_sort_key(&sources[src_idx], &hand_demand));

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
        let best = available.iter()
            .enumerate()
            .filter(|&(_, &src_idx)| ability_producing(&sources[src_idx], mt).is_some())
            .min_by_key(|&(_, &src_idx)| source_sort_key(&sources[src_idx], &hand_demand));

        if let Some((avail_pos, &src_idx)) = best {
            let source = &sources[src_idx];
            let ability_idx = ability_producing(source, mt).unwrap();
            let ability = source.abilities.iter().find(|a| a.ability_index == ability_idx).unwrap();
            let total_produced = ability_total_mana(ability);
            // 1 mana used for the colored pip, rest is excess.
            excess_mana += total_produced - 1;

            tap_plan.push((source.object_id, ability_idx));
            available.remove(avail_pos);
        } else {
            return None; // Can't satisfy this colored requirement.
        }
    }

    // Phase 3: Satisfy generic mana.
    // First use excess from already-tapped sources.
    if remaining.generic > 0 {
        let used = excess_mana.min(remaining.generic);
        remaining.generic -= used;
    }

    // Then tap more sources as needed, sorted by priority.
    while remaining.generic > 0 {
        // Sort available sources by priority.
        let best = available.iter()
            .enumerate()
            .min_by_key(|&(_, &src_idx)| source_sort_key(&sources[src_idx], &hand_demand));

        if let Some((avail_pos, &src_idx)) = best {
            let source = &sources[src_idx];
            // Pick the ability to activate. For sources with multiple abilities (dual lands),
            // pick the one whose color has lowest hand demand.
            let ability_idx = source.abilities.iter()
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

    Some(tap_plan)
}

/// Check if a mana pool can pay a given cost.
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
pub fn auto_pay(pool: &mut ManaPool, cost: &ManaCost) -> Result<(), ManaError> {
    try_auto_pay(pool, cost)
}

fn try_auto_pay(pool: &mut ManaPool, cost: &ManaCost) -> Result<(), ManaError> {
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
        // Pay from colorless first, then from each color.
        let types = [
            ManaType::Colorless,
            ManaType::White, ManaType::Blue, ManaType::Black,
            ManaType::Red, ManaType::Green,
        ];
        for mt in types {
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

    // ---- autotap tests ----

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
            description: format!("Add {:?}", mana_type),
            produced: vec![(mana_type, 1)],
            requires_tap: true,
            has_side_effects: false,
        }
    }

    fn dual_abilities(mt1: ManaType, mt2: ManaType) -> Vec<ManaAbilityDef> {
        vec![
            ManaAbilityDef {
                ability_index: 0,
                description: format!("Add {:?}", mt1),
                produced: vec![(mt1, 1)],
                requires_tap: true,
                has_side_effects: false,
            },
            ManaAbilityDef {
                ability_index: 1,
                description: format!("Add {:?}", mt2),
                produced: vec![(mt2, 1)],
                requires_tap: true,
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
    fn autotap_prefers_mono_over_dual() {
        // Cost {G}, sources: Forest + Hinterland Harbor (G/U). Should tap Forest.
        let cost = ManaCost::new(vec![ManaSymbol::Colored(Color::Green)]);
        let sources = vec![
            make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
            make_source(2, ManaSourceKind::NonBasicMana, dual_abilities(ManaType::Green, ManaType::Blue)),
        ];
        let plan = compute_autotap(&cost, &ManaPool::new(), &sources, &[]).unwrap();
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].0, ObjectId(1)); // Forest, not Harbor
    }

    #[test]
    fn autotap_opportunity_cost_tiers() {
        // Cost {G}, sources: Forest (BasicMana) + Wolf Run land (HasUtilityAbility).
        // Both produce green. Should tap Forest.
        let cost = ManaCost::new(vec![ManaSymbol::Colored(Color::Green)]);
        let sources = vec![
            make_source(1, ManaSourceKind::HasUtilityAbility, vec![mono_ability(ManaType::Green)]),
            make_source(2, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
        ];
        let plan = compute_autotap(&cost, &ManaPool::new(), &sources, &[]).unwrap();
        assert_eq!(plan[0].0, ObjectId(2)); // BasicMana Forest
    }

    #[test]
    fn autotap_creature_deprioritized() {
        // Cost {W}, sources: Plains (BasicMana) + Avacyn's Pilgrim (Creature).
        // Should tap Plains.
        let cost = ManaCost::new(vec![ManaSymbol::Colored(Color::White)]);
        let sources = vec![
            make_source(1, ManaSourceKind::Creature, vec![mono_ability(ManaType::White)]),
            make_source(2, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::White)]),
        ];
        let plan = compute_autotap(&cost, &ManaPool::new(), &sources, &[]).unwrap();
        assert_eq!(plan[0].0, ObjectId(2)); // Plains
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
    fn autotap_side_effects_last() {
        // Cost {1}, sources: Sol Ring (BasicMana, 2 colorless) + Deranged Assistant (HasSideEffects, 1 colorless).
        // Should tap Sol Ring.
        let cost = ManaCost::new(vec![ManaSymbol::Generic(1)]);
        let sol_ring = ManaAbilityDef {
            ability_index: 0,
            description: "Add {C}{C}".into(),
            produced: vec![(ManaType::Colorless, 2)],
            requires_tap: true,
            has_side_effects: false,
        };
        let deranged = ManaAbilityDef {
            ability_index: 0,
            description: "Add {C}".into(),
            produced: vec![(ManaType::Colorless, 1)],
            requires_tap: true,
            has_side_effects: true,
        };
        let sources = vec![
            make_source(1, ManaSourceKind::HasSideEffects, vec![deranged]),
            make_source(2, ManaSourceKind::BasicMana, vec![sol_ring]),
        ];
        let plan = compute_autotap(&cost, &ManaPool::new(), &sources, &[]).unwrap();
        assert_eq!(plan[0].0, ObjectId(2)); // Sol Ring
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
            has_side_effects: false,
        };
        let sources = vec![
            make_source(1, ManaSourceKind::BasicMana, vec![sol_ring]),
        ];
        let plan = compute_autotap(&cost, &ManaPool::new(), &sources, &[]).unwrap();
        assert_eq!(plan.len(), 1);
    }

    #[test]
    fn autotap_scarcity_ordering() {
        // Cost {G}{U}, sources: Hinterland Harbor (G/U), Forest, Island.
        // Should assign Forest→G, Island→U (not use Harbor for either).
        let cost = ManaCost::new(vec![
            ManaSymbol::Colored(Color::Green),
            ManaSymbol::Colored(Color::Blue),
        ]);
        let sources = vec![
            make_source(1, ManaSourceKind::NonBasicMana, dual_abilities(ManaType::Green, ManaType::Blue)),
            make_source(2, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
            make_source(3, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Blue)]),
        ];
        let plan = compute_autotap(&cost, &ManaPool::new(), &sources, &[]).unwrap();
        assert_eq!(plan.len(), 2);
        // Should use Forest and Island, not Harbor.
        let ids: Vec<ObjectId> = plan.iter().map(|p| p.0).collect();
        assert!(ids.contains(&ObjectId(2))); // Forest
        assert!(ids.contains(&ObjectId(3))); // Island
    }

    #[test]
    fn autotap_hand_preservation() {
        // Cost {G}, hand has a {U}{G} spell.
        // Sources: Forest (mono G) + Harbor (G/U).
        // Should tap Forest to preserve Harbor's U for the other spell.
        let cost = ManaCost::new(vec![ManaSymbol::Colored(Color::Green)]);
        let hand_costs = vec![ManaCost::new(vec![
            ManaSymbol::Colored(Color::Blue),
            ManaSymbol::Colored(Color::Green),
        ])];
        let sources = vec![
            make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
            make_source(2, ManaSourceKind::NonBasicMana, dual_abilities(ManaType::Green, ManaType::Blue)),
        ];
        let plan = compute_autotap(&cost, &ManaPool::new(), &sources, &hand_costs).unwrap();
        assert_eq!(plan[0].0, ObjectId(1)); // Forest, preserving Harbor
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

    #[test]
    fn autotap_dual_for_generic() {
        // Cost {1}{G}, sources: Forest + Hinterland Harbor (G/U), hand has {U} spell.
        // Should tap Forest for {G}, and for the generic {1} should tap Harbor
        // but use the U ability (lower hand demand) since hand needs U less... actually
        // hand needs U more. So it should use G ability to preserve U.
        // Wait: hand has {U} spell, so U demand is 1. G demand is 0 (the current spell
        // doesn't count). So for generic, Harbor should produce G (demand 0) not U (demand 1).
        let cost = ManaCost::new(vec![
            ManaSymbol::Generic(1),
            ManaSymbol::Colored(Color::Green),
        ]);
        let hand_costs = vec![ManaCost::new(vec![ManaSymbol::Colored(Color::Blue)])];
        let sources = vec![
            make_source(1, ManaSourceKind::BasicMana, vec![mono_ability(ManaType::Green)]),
            make_source(2, ManaSourceKind::NonBasicMana, dual_abilities(ManaType::Green, ManaType::Blue)),
        ];
        let plan = compute_autotap(&cost, &ManaPool::new(), &sources, &hand_costs).unwrap();
        assert_eq!(plan.len(), 2);
        // Forest for the green pip.
        assert_eq!(plan[0].0, ObjectId(1));
        // Harbor for generic, should pick ability 0 (Green, demand=0) over ability 1 (Blue, demand=1).
        assert_eq!(plan[1].0, ObjectId(2));
        assert_eq!(plan[1].1, 0); // Green ability
    }
}
