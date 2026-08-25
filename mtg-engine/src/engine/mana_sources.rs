use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::mana;
use crate::state::{GameState, LogLevel};
use crate::types::{Zone, CardType, Supertype, ManaCost, ManaSymbol, ContinuousEffect};
use super::*;

/// The mana abilities of `object_id` that could legally be activated right now.
///
/// Every caller that needs a permanent's mana abilities goes through here
/// rather than calling `CardBehavior::mana_abilities` directly, so the
/// cost-legality gate is applied in exactly one place: the permanent has to be
/// on the battlefield, and a `{T}` ability additionally needs
/// `can_pay_tap_cost` (untapped, and past summoning sickness unless hasty —
/// CR 302.6). A card's own `mana_abilities` describes only what is particular
/// to the ability, e.g. Deranged Assistant needing a card left to mill.
pub fn available_mana_abilities(
    state: &GameState,
    object_id: ObjectId,
    registry: &CardRegistry,
) -> Vec<crate::cards::ManaAbilityDef> {
    let Some(obj) = state.get_object(object_id) else { return Vec::new(); };
    if obj.zone != Zone::Battlefield {
        return Vec::new();
    }
    let Some(behavior) = registry.get(obj.card_id) else { return Vec::new(); };
    let can_tap = state.can_pay_tap_cost(object_id, registry);
    behavior.mana_abilities(state, object_id)
        .into_iter()
        .filter(|ma| !ma.requires_tap || can_tap)
        .collect()
}
/// `available_mana_abilities`, further narrowed to the ones whose mana cost the
/// player can pay from the pool right now.
///
/// This is the standalone "activate a mana ability" action, where nothing else
/// is going to produce mana first. The auto-tap planner uses the unfiltered
/// list instead, because there the funding comes from earlier entries in the
/// same plan.
pub(crate) fn activatable_mana_abilities(
    state: &GameState,
    object_id: ObjectId,
    registry: &CardRegistry,
) -> Vec<crate::cards::ManaAbilityDef> {
    let Some(controller) = state.get_object(object_id).map(|o| o.controller) else { return Vec::new() };
    let pool = &state.get_player(controller).mana_pool;
    available_mana_abilities(state, object_id, registry)
        .into_iter()
        .filter(|ma| mana::can_pay(pool, &ma.cost))
        .collect()
}
/// Whether `player` could pay `cost` right now — from floating mana, or by
/// tapping what they control.
///
/// CR 608.2g: a resolving effect that gives a player the option to pay a cost
/// lets that player activate mana abilities before deciding. The engine has no
/// priority window mid-resolution, so it does what it already does for spell
/// and ability costs: work out a tap plan and run it when the player says yes.
/// Without this, "you may pay {1}" was offered only to a player who happened
/// to have {1} already floating — nearly nobody — and everyone else was
/// treated as having declined.
pub fn can_pay_with_sources(
    state: &GameState,
    player: PlayerId,
    cost: &ManaCost,
    registry: &CardRegistry,
) -> bool {
    let pool = &state.get_player(player).mana_pool;
    if mana::can_pay(pool, cost) {
        return true;
    }
    let sources = gather_mana_sources(state, player, registry, prevents_artifact_abilities(state, registry));
    mana::compute_autotap(cost, pool, &sources, &[]).is_some()
}
/// Pay `cost`, tapping sources if the pool alone can't cover it. Returns false
/// and leaves the game state untouched when it cannot be paid — the tap plan
/// is worked out in full before anything is tapped.
pub fn pay_cost_with_sources(
    state: &mut GameState,
    player: PlayerId,
    cost: &ManaCost,
    registry: &CardRegistry,
) -> bool {
    if !mana::can_pay(&state.get_player(player).mana_pool, cost) {
        let sources = gather_mana_sources(state, player, registry, prevents_artifact_abilities(state, registry));
        let plan = {
            let pool = &state.get_player(player).mana_pool;
            mana::compute_autotap(cost, pool, &sources, &[])
        };
        let Some(plan) = plan else { return false };
        for (source_id, ability_index) in plan {
            activate_mana_source(state, source_id, ability_index, registry);
        }
    }
    mana::auto_pay(&mut state.get_player_mut(player).mana_pool, cost).is_ok()
}
/// Stony Silence and friends: no artifact ability may be activated, mana
/// abilities included.
pub(crate) fn prevents_artifact_abilities(state: &GameState, registry: &CardRegistry) -> bool {
    state.global_effects(registry).iter()
        .any(|e| matches!(e, ContinuousEffect::PreventArtifactAbilities))
}
/// Gather all available mana sources for a player, classified by opportunity cost.
pub(crate) fn gather_mana_sources(
    state: &GameState,
    player: PlayerId,
    registry: &CardRegistry,
    prevent_artifact_abilities: bool,
) -> Vec<mana::ManaSource> {
    use mana::{ManaSource, ManaSourceKind};

    let mut sources = Vec::new();
    for obj in state.objects_in_zone(Zone::Battlefield, player) {
        // Stony Silence: skip mana abilities from artifacts.
        if prevent_artifact_abilities
            && state.has_card_type(obj.id, CardType::Artifact, registry) {
            continue;
        }
        if let Some(behavior) = registry.get(obj.card_id) {
            let abilities = available_mana_abilities(state, obj.id, registry);
            if abilities.is_empty() { continue; }

            // Classify the source kind.
            let has_side_effects = abilities.iter().any(|a| a.has_side_effects);
            let is_creature = state.is_creature(obj.id, registry);
            let has_utility = !behavior.activated_abilities(state, obj.id, registry).is_empty();
            let is_basic = state.face_data(obj.id, registry)
                .is_some_and(|d| d.supertypes.contains(&Supertype::Basic));

            let source_kind = if has_side_effects {
                ManaSourceKind::HasSideEffects
            } else if is_creature {
                ManaSourceKind::Creature
            } else if has_utility {
                ManaSourceKind::HasUtilityAbility
            } else if is_basic {
                ManaSourceKind::BasicMana
            } else {
                ManaSourceKind::NonBasicMana
            };

            sources.push(ManaSource {
                object_id: obj.id,
                abilities,
                source_kind,
            });
        }
    }
    sources
}
/// Activate a single mana source (tap + add mana + side effects).
/// Shared by both `ActivateManaAbility` and `CastSpell` `tap_plan` execution.
pub fn activate_mana_source(
    state: &mut GameState,
    source_id: ObjectId,
    ability_index: usize,
    registry: &CardRegistry,
) {
    let obj = state.get_object(source_id).expect("mana source must exist");
    let card_id = obj.card_id;
    let controller = obj.controller;

    if let Some(behavior) = registry.get(card_id) {
        let abilities = behavior.mana_abilities(state, source_id);
        if let Some(ability) = abilities.iter().find(|a| a.ability_index == ability_index) {
            // A filter's mana cost is paid before it produces (CR 605.1a — it
            // is still a mana ability, it just isn't free). The tap plan puts
            // cost-bearing abilities last so the mana is already floating.
            if !ability.cost.symbols.is_empty()
                && mana::auto_pay(&mut state.get_player_mut(controller).mana_pool, &ability.cost).is_err() {
                return;
            }
            if ability.requires_tap {
                state.get_object_mut(source_id).expect("object must exist for tapping").tapped = true;
                state.events.push(GameEvent::Tapped { object: source_id });
            }
            for &(mana_type, amount) in &ability.produced {
                state.get_player_mut(controller).mana_pool.add(mana_type, amount);
                state.events.push(GameEvent::ManaAdded {
                    player: controller,
                    mana_type,
                    amount,
                });
            }
            behavior.on_activate_mana_ability(state, source_id, ability_index, registry);
        }
    }

    let name = card_name(state, registry, source_id);
    let pool = &state.get_player(controller).mana_pool;
    let pool_str: Vec<String> = pool.mana.iter()
        .filter(|(_, &v)| v > 0)
        .map(|(t, v)| format!("{t:?}:{v}"))
        .collect();
    state.log(LogLevel::Info, format!("p{} tapped {} for mana (pool: {})",
        controller.0, name, if pool_str.is_empty() { "empty".into() } else { pool_str.join(" ") }));
}
/// Compute an autotap plan for paying `cost` from `player`'s pool + untapped
/// mana sources, without modifying state. Returns `None` if the cost is not
/// autotap-reachable. Callers pair this with [`execute_tap_plan_and_pay`] to
/// actually commit. Used by card handlers (e.g. Screeching Bat's may-pay
/// upkeep) that need to know whether a mid-resolution cost is affordable
/// *before* asking the player — pool-only checks miss the common case where
/// the pool is empty but enough untapped sources are available (CR 106.4:
/// mana empties between steps).
#[must_use]
pub(crate) fn plan_autotap_for_cost(
    state: &GameState,
    player: PlayerId,
    cost: &ManaCost,
    registry: &CardRegistry,
) -> Option<Vec<(ObjectId, usize)>> {
    let sources = gather_mana_sources(state, player, registry, false);
    let pool = &state.get_player(player).mana_pool;
    mana::compute_autotap(cost, pool, &sources, &[])
}
/// Execute a tap plan and deduct `cost` from the resulting pool.
/// Returns `true` on success, `false` if `auto_pay` fails (which shouldn't
/// happen when `tap_plan` came from `compute_autotap` on the same `cost`).
pub(crate) fn execute_tap_plan_and_pay(
    state: &mut GameState,
    player: PlayerId,
    cost: &ManaCost,
    tap_plan: &[(ObjectId, usize)],
    registry: &CardRegistry,
) -> bool {
    for &(src_id, ability_index) in tap_plan {
        activate_mana_source(state, src_id, ability_index, registry);
    }
    mana::auto_pay(&mut state.get_player_mut(player).mana_pool, cost).is_ok()
}
/// Find alternative costs provided by continuous effects on permanents the caster controls.
/// Returns a list of alternative `ManaCosts` that the caster may use for the given spell.
pub(crate) fn alternative_costs_from_effects(state: &GameState, registry: &CardRegistry, card_id: CardId, caster: PlayerId) -> Vec<ManaCost> {
    use crate::types::{ContinuousEffect, SpellFilter};

    let card_data = registry.card_data(card_id);
    let is_creature = card_data.as_ref()
        .is_some_and(|d| d.card_types.contains(&CardType::Creature));
    let subtypes: Vec<String> = card_data.as_ref()
        .map(|d| d.subtypes.clone())
        .unwrap_or_default();

    let mut alt_costs = Vec::new();
    for obj in state.objects.values() {
        if obj.zone != Zone::Battlefield || obj.controller != caster {
            continue;
        }
        if let Some(behavior) = registry.get(obj.card_id) {
            for effect in &behavior.card_data().continuous_effects {
                if let ContinuousEffect::AlternativeCost { cost, filter } = effect {
                    let applies = match filter {
                        SpellFilter::CreatureSpells => is_creature,
                        SpellFilter::CreatureWithSubtype(sub) => {
                            is_creature && subtypes.iter().any(|s| s == sub)
                        }
                    };
                    if applies {
                        alt_costs.push(cost.clone());
                    }
                }
            }
        }
    }
    alt_costs
}
/// Compute the effective mana cost of a spell after applying cost reduction effects.
/// Returns a reduced `ManaCost` (generic portion lowered, colored requirements unchanged).
#[must_use]
pub fn effective_spell_cost(state: &GameState, registry: &CardRegistry, card_id: CardId, base_cost: &ManaCost, caster: PlayerId) -> ManaCost {
    use crate::types::{ContinuousEffect, SpellFilter};

    // Check if the card has a custom cost modification (e.g., Blasphemous Act).
    if let Some(behavior) = registry.get(card_id) {
        if let Some(modified) = behavior.modified_cost(state, registry) {
            return modified;
        }
    }

    let card_data = registry.card_data(card_id);
    let is_creature = card_data.as_ref()
        .is_some_and(|d| d.card_types.contains(&CardType::Creature));
    let subtypes: Vec<String> = card_data.as_ref()
        .map(|d| d.subtypes.clone())
        .unwrap_or_default();

    // Gather all ReduceCost effects from permanents the caster controls.
    let mut total_reduction: u32 = 0;
    for obj in state.objects.values() {
        if obj.zone != Zone::Battlefield || obj.controller != caster {
            continue;
        }
        if let Some(behavior) = registry.get(obj.card_id) {
            for effect in &behavior.card_data().continuous_effects {
                if let ContinuousEffect::ReduceCost { reduction, filter } = effect {
                    let applies = match filter {
                        SpellFilter::CreatureSpells => is_creature,
                        SpellFilter::CreatureWithSubtype(sub) => {
                            is_creature && subtypes.iter().any(|s| s == sub)
                        }
                    };
                    if applies {
                        total_reduction += reduction;
                    }
                }
            }
        }
    }

    if total_reduction == 0 {
        return base_cost.clone();
    }

    // Apply reduction to generic mana first, keeping colored requirements.
    let mut remaining_reduction = total_reduction;
    let mut new_symbols = Vec::new();
    for sym in &base_cost.symbols {
        match sym {
            ManaSymbol::Generic(n) => {
                if remaining_reduction >= *n {
                    remaining_reduction -= *n;
                    // Reduced to zero, omit this symbol.
                } else {
                    new_symbols.push(ManaSymbol::Generic(*n - remaining_reduction));
                    remaining_reduction = 0;
                }
            }
            other => new_symbols.push(other.clone()),
        }
    }
    ManaCost::new(new_symbols)
}
