use crate::cards::CardRegistry;
use crate::ids::{ObjectId, PlayerId};
use crate::mana;
use crate::state::{GameState, LogLevel};
use crate::types::{Zone, CardType, Supertype, ManaCost, ContinuousEffect};
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
///
/// The ability is looked up through [`available_mana_abilities`], not through
/// `CardBehavior::mana_abilities`, so the cost-legality gate holds where the
/// tap actually happens and not only where the action was offered. A tap plan
/// is worked out in full before any of it is executed, and the state moves
/// underneath it: the first Deranged Assistant in a plan mills the last card
/// of the library, and the second can no longer pay its cost (CR 701.17b).
/// Reading the card's own list here executed every such plan as written.
pub fn activate_mana_source(
    state: &mut GameState,
    source_id: ObjectId,
    ability_index: usize,
    registry: &CardRegistry,
) {
    activate_mana_source_reserving(state, source_id, ability_index, None, registry);
}

/// As [`activate_mana_source`], but told what the mana being gathered is for.
///
/// A filter's own mana cost is paid out of the pool the plan has been filling,
/// and paying it greedily took the very mana the spell needed: a `{W}{W}` plan
/// of Plains + Forest + Shimmering Grotto paid the Grotto's `{1}` with the
/// White and left the Green, so the plan the engine had offered could not be
/// executed (issue #252). `reserve` is the cost still to be paid, so the
/// filter spends what that cost does not need.
pub fn activate_mana_source_reserving(
    state: &mut GameState,
    source_id: ObjectId,
    ability_index: usize,
    reserve: Option<&ManaCost>,
    registry: &CardRegistry,
) {
    let obj = state.get_object(source_id).expect("mana source must exist");
    let card_id = obj.card_id;
    let controller = obj.controller;

    // Stony Silence: "activated abilities of artifacts can't be activated",
    // mana abilities included. The auto-tap planner already skips artifact
    // sources, but a SUBMITTED action — a standalone ActivateManaAbility or a
    // tap plan naming one — funnels through here too, so the gate holds where
    // the tap actually happens (same reasoning as the cost-legality gate
    // above). The refusal produces no mana, which the cast path's funding
    // rehearsal then turns into a refused cast.
    if prevents_artifact_abilities(state, registry)
        && state.has_card_type(source_id, CardType::Artifact, registry) {
        return;
    }

    let abilities = available_mana_abilities(state, source_id, registry);
    let Some(ability) = abilities.iter().find(|a| a.ability_index == ability_index) else {
        return;
    };
    let Some(behavior) = registry.get(card_id) else { return };

    // A filter's mana cost is paid before it produces (CR 605.1a — it
    // is still a mana ability, it just isn't free). The tap plan puts
    // cost-bearing abilities last so the mana is already floating.
    if !ability.cost.symbols.is_empty() {
        let ability_cost = ability.cost.clone();
        let pool = &mut state.get_player_mut(controller).mana_pool;
        let paid = match reserve {
            Some(reserve) => mana::auto_pay_reserving(pool, &ability_cost, reserve),
            None => mana::auto_pay(pool, &ability_cost),
        };
        if paid.is_err() {
            return;
        }
    }
    if ability.requires_tap {
        state.tap(source_id);
    }
    for &(mana_type, amount) in &ability.produced {
        state.add_mana(controller, mana_type, amount);
    }
    behavior.on_activate_mana_ability(state, source_id, ability_index, registry);

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
        activate_mana_source_reserving(state, src_id, ability_index, Some(cost), registry);
    }
    mana::auto_pay(&mut state.get_player_mut(player).mana_pool, cost).is_ok()
}
