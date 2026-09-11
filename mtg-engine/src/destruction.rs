//! Unified destruction pipeline.
//!
//! All "destroy" effects and state-based destruction flow through `try_destroy`,
//! which checks indestructible and regeneration before actually killing a permanent.
//! Sacrifice uses `sacrifice`, which bypasses both.

use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::ObjectId;
use crate::state::{GameState, LogLevel};
use crate::types::{Keyword, Zone};

/// Result of attempting to destroy a permanent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestroyResult {
    /// Permanent was destroyed (moved to graveyard).
    Died,
    /// Destruction prevented by indestructible (no state change).
    Indestructible,
    /// Destruction replaced by regeneration (state changed: tapped, damage cleared).
    Regenerated,
    /// Nothing to destroy: the object is not on the battlefield (CR 701.7a
    /// destroys permanents; a card already in the graveyard is a new object
    /// that an earlier "destroy that creature" no longer concerns, CR 400.7).
    /// No event, no zone change — a second death used to be announced for a
    /// creature that had already died (found by fuzzing).
    NotAPermanent,
}

fn on_battlefield(state: &GameState, id: ObjectId) -> bool {
    state.get_object(id).is_some_and(|o| o.zone == Zone::Battlefield)
}

/// Attempt to destroy a permanent.
///
/// Pipeline:
/// 1. Indestructible — prevents destruction entirely (no state change).
/// 2. Regeneration shields — replaces destruction (tap, remove damage, consume shield).
/// 3. Falls through — permanent is destroyed (moved to graveyard).
///
/// Called by destroy spells (Doom Blade, etc.) and by SBAs for lethal damage / deathtouch.
/// NOT called for 0-toughness deaths (rule 704.5f) — those are not destruction.
pub fn try_destroy(state: &mut GameState, id: ObjectId, registry: &CardRegistry) -> DestroyResult {
    let result = decide_destroy(state, id, registry);
    apply_destroy(state, id, result, registry);
    result
}

/// What destroying this permanent would do, decided against the state as it
/// stands and before anything moves.
///
/// Deciding is separate from applying for two reasons, and both have callers:
/// a simultaneous destruction has to decide for every permanent against the
/// same battlefield (CR 700.2c, see `try_destroy_all`), and a caller that
/// names itself in the log has to know the outcome *before* the outcome is
/// written, so the line that names the cause comes before the line that
/// records the consequence (`try_destroy_by`, and `sacrifice_by` for the same
/// reason).
fn decide_destroy(state: &GameState, id: ObjectId, registry: &CardRegistry) -> DestroyResult {
    if !on_battlefield(state, id) {
        // CR 701.7a destroys permanents.
        DestroyResult::NotAPermanent
    } else if state.has_keyword(id, Keyword::Indestructible, registry) {
        // Indestructible prevents destruction (CR 701.7b).
        DestroyResult::Indestructible
    } else if state.get_object(id).is_some_and(|o| o.regeneration_shields > 0) {
        // Regeneration replaces destruction (CR 701.15a).
        DestroyResult::Regenerated
    } else {
        DestroyResult::Died
    }
}

/// Carry out a decision from [`decide_destroy`]. Indestructible and
/// `NotAPermanent` are no-ops by definition: nothing about the game changed.
fn apply_destroy(state: &mut GameState, id: ObjectId, result: DestroyResult, registry: &CardRegistry) {
    match result {
        DestroyResult::Died => destroy(state, id, Some(registry)),
        DestroyResult::Regenerated => regenerate(state, id),
        DestroyResult::Indestructible | DestroyResult::NotAPermanent => {}
    }
}

/// The one line that says what a named source did to a named permanent.
///
/// Every caller that announces its own destruction writes this line, so there
/// is one wording and one place the four outcomes are spelled out. Three
/// copies of this `match` had drifted apart before it existed.
pub fn destroy_line(source: &str, name: &str, result: DestroyResult) -> String {
    match result {
        DestroyResult::Died => format!("{source} destroyed {name}"),
        DestroyResult::Regenerated => format!("{source} could not destroy {name} — it regenerated"),
        DestroyResult::Indestructible => format!("{source} could not destroy {name} — it is indestructible"),
        DestroyResult::NotAPermanent => format!("{source} found nothing to destroy — {name} is no longer on the battlefield"),
    }
}

/// `try_destroy`, with one accurate line in the log naming what tried.
///
/// The pipeline already announces what *happened* — `move_object` writes the
/// death, `regenerate` writes the regeneration — but neither names the source,
/// and five cards wrote their own "X destroyed Y" line beside it without
/// looking at the result. Ghost Quarter's ruling is explicit that the land can
/// survive ("even if that land wasn't destroyed... because the land has
/// indestructible or because it was regenerated"), and the log said it was
/// destroyed anyway. This is the same shape as `mill_cards` taking a source:
/// the line that names the card is the one a reader trusts, so it has to be
/// the true one.
pub fn try_destroy_by(
    state: &mut GameState,
    id: ObjectId,
    source: &str,
    registry: &CardRegistry,
) -> DestroyResult {
    let name = state.obj_name(id);
    let result = decide_destroy(state, id, registry);
    state.log(LogLevel::Event, destroy_line(source, &name, result));
    apply_destroy(state, id, result, registry);
    result
}

/// Destroy several permanents simultaneously (CR 700.2c, CR 701.7b).
///
/// "Destroy all creatures" is one event, not a sequence of them, and the
/// difference is observable. Angelic Overseer is "indestructible as long as
/// you control a Human"; when a Wrath catches the Overseer and its last Human
/// together, the Human is still on the battlefield at the moment destruction
/// happens, so the Overseer survives. A loop over `try_destroy` gets that
/// wrong whenever it reaches the Human first — the Overseer's condition is
/// gone by the time its own check runs, and the Overseer dies too.
///
/// So this decides for every permanent against the same game state, the one
/// before any of them has died, and captures each death's last known
/// information there as well (CR 608.2g) before moving anything. Results come
/// back in the order given.
pub fn try_destroy_all(
    state: &mut GameState,
    ids: &[ObjectId],
    registry: &CardRegistry,
) -> Vec<(ObjectId, DestroyResult)> {
    // Phase 1 — decide. Nothing has moved yet, so every check sees the same
    // battlefield.
    let decisions: Vec<(ObjectId, DestroyResult)> = ids.iter()
        .map(|&id| (id, decide_destroy(state, id, registry)))
        .collect();

    apply_destroy_all(state, &decisions, registry)
}

/// Phases 2 and 3 of [`try_destroy_all`]: capture every death event against
/// the undisturbed state, then move everything. Split out so a caller that
/// announces itself can write its lines between the decision and the
/// application without the decision going stale.
fn apply_destroy_all(
    state: &mut GameState,
    decisions: &[(ObjectId, DestroyResult)],
    registry: &CardRegistry,
) -> Vec<(ObjectId, DestroyResult)> {
    // Phase 2 — capture the death events, still against that same state, so a
    // creature whose toughness depends on the others (Splinterfright counting
    // creature cards in the graveyard) is remembered as it was.
    let deaths: Vec<(ObjectId, Option<GameEvent>)> = decisions.iter()
        .filter(|(_, r)| *r == DestroyResult::Died)
        .map(|&(id, _)| (id, death_event(state, id, Some(registry))))
        .collect();

    // Phase 3 — apply.
    for &(id, result) in decisions {
        if result == DestroyResult::Regenerated {
            regenerate(state, id);
        }
    }
    for (id, event) in deaths {
        if let Some(event) = event {
            state.events.push(event);
            state.creature_died_this_turn = true;
        }
        state.move_object(id, Zone::Graveyard, registry);
    }

    decisions.to_vec()
}

/// Destroy a permanent, bypassing regeneration ("can't be regenerated").
/// Still respects indestructible.
pub fn try_destroy_no_regen(state: &mut GameState, id: ObjectId, registry: &CardRegistry) -> DestroyResult {
    if !on_battlefield(state, id) {
        return DestroyResult::NotAPermanent;
    }
    if state.has_keyword(id, Keyword::Indestructible, registry) {
        return DestroyResult::Indestructible;
    }
    destroy(state, id, Some(registry));
    DestroyResult::Died
}

/// Sacrifice a permanent. Bypasses indestructible and regeneration.
/// Returns true if the permanent existed and was sacrificed.
///
/// Prefer [`sacrifice_by`], which also writes the log line.
pub fn sacrifice(state: &mut GameState, id: ObjectId, registry: &CardRegistry) -> bool {
    let exists = state.get_object(id)
        .is_some_and(|o| o.zone == Zone::Battlefield);
    if !exists {
        return false;
    }
    destroy(state, id, Some(registry));
    true
}

/// `sacrifice`, with the one line the log was missing: **who** sacrificed
/// **what**, and **why**.
///
/// CR 701.17a is "a player sacrifices a permanent they control", and the
/// sacrificing player was exactly the fact the log dropped. The only trace of
/// an ability's sacrifice cost used to be `move_object`'s generic
/// `<name> died`, indistinguishable from a combat death or a Doom Blade; with
/// two eligible creatures the log could not say which one paid, even though
/// the action menu had (issue #263).
///
/// `reason` is the trailing clause — `"to pay for Skirsdag Cultist's ability"`,
/// `"as an additional cost of Infernal Plunge"` — so the line reads
/// `p0 sacrificed Spirit Token (#121) to pay for Skirsdag Cultist's ability`.
/// It is written BEFORE the permanent leaves, both so the name is still there
/// to read and so the log stops saying "it died, and then it was sacrificed"
/// (the spell path printed the two lines in that order).
pub fn sacrifice_by(
    state: &mut GameState,
    id: ObjectId,
    reason: &str,
    registry: &CardRegistry,
) -> bool {
    let Some(who) = state.get_object(id)
        .filter(|o| o.zone == Zone::Battlefield)
        .map(|o| o.controller)
    else {
        return false;
    };
    let name = state.obj_name(id);
    let line = if reason.is_empty() {
        format!("p{} sacrificed {name}", who.0)
    } else {
        format!("p{} sacrificed {name} {reason}", who.0)
    };
    state.log(LogLevel::Event, line);
    sacrifice(state, id, registry)
}

/// Apply regeneration: tap, remove damage, consume one shield, remove from combat.
fn regenerate(state: &mut GameState, id: ObjectId) {
    state.tap(id);
    if let Some(obj) = state.get_object_mut(id) {
        // CR 701.15a: regenerating "removes all damage marked on it". It does
        // not un-deal that damage. `damaged_by` is the record of who dealt
        // damage to this creature *this turn* — a fact about the turn, which
        // cleanup clears (CR 514.2) and regeneration does not. Clearing it
        // here meant a creature Abattoir Ghoul damaged, that regenerated and
        // died later the same turn, fed the Ghoul nothing.
        //
        // `dealt_deathtouch_damage` does go, because it is a property of the
        // marked damage: SBA 704.5h must not destroy the creature again for
        // damage that is no longer there.
        obj.damage_marked = 0;
        obj.dealt_deathtouch_damage = false;
        obj.regeneration_shields -= 1;
    }
    remove_from_combat(state, id);
    state.log(LogLevel::Event, format!("{} regenerated",
        state.obj_name(id)));
}

/// The `CreatureDied` event for a permanent about to be destroyed, built from
/// last known information (CR 608.2g) — so it has to be called BEFORE the zone
/// change, which clears the object's battlefield state. `None` for a
/// non-creature, which announces no death.
pub(crate) fn death_event(state: &GameState, id: ObjectId, registry: Option<&CardRegistry>) -> Option<GameEvent> {
    let is_creature = registry.is_some_and(|r| state.is_creature(id, r))
        || state.get_object(id).is_some_and(|o| o.power.is_some());
    if !is_creature {
        return None;
    }
    let (cid, ctrl, damaged_by, is_token) = state.get_object(id)
        .map_or((crate::ids::CardId(0), crate::ids::PlayerId(0), Vec::new(), false), |o| (o.card_id, o.controller, o.damaged_by.clone(), o.is_token));
    let name = state.get_object(id).map(|o| o.name.clone()).unwrap_or_default();

    let last_known_toughness = registry
        .and_then(|r| state.effective_toughness(id, r))
        .or_else(|| state.get_object(id).and_then(|o| o.toughness))
        .unwrap_or(0);
    // The active face's subtypes, read while the permanent still has an active
    // face: `move_object` clears `is_transformed` on the way out (CR 400.7), so
    // a Werewolf that died would read back as the Human on its front.
    let subtypes = registry.map(|r| state.subtypes_of(id, r)).unwrap_or_default();
    Some(GameEvent::CreatureDied { object: id, name, card_id: cid, controller: ctrl, damaged_by, last_known_toughness, is_token, subtypes })
}

/// Actually destroy a permanent: emit events, move to graveyard, set morbid flag.
fn destroy(state: &mut GameState, id: ObjectId, registry: Option<&CardRegistry>) {
    // Only a permanent can be destroyed; every caller checks, and this is
    // the last line of defence for the ones that reach here directly.
    if !on_battlefield(state, id) {
        return;
    }
    if let Some(event) = death_event(state, id, registry) {
        state.events.push(event);
        state.creature_died_this_turn = true;
    }
    // move_object handles the death/graveyard log message.
    state.move_object(id, Zone::Graveyard, registry.expect("registry required for move_object"));
}

/// Regenerate during SBA processing (public for sba.rs).
/// Skips the indestructible check since SBAs snapshot that separately.
pub fn regenerate_sba(state: &mut GameState, id: ObjectId) {
    regenerate(state, id);
}

/// Destroy during SBA processing (public for sba.rs).
/// Skips the indestructible check since SBAs snapshot that separately.
pub fn destroy_sba(state: &mut GameState, id: ObjectId, registry: &CardRegistry) {
    destroy(state, id, Some(registry));
}

/// Remove a creature from the current combat (if any).
/// Used by regeneration and other effects that pull a creature out of combat.
/// The implementation lives on `GameState` so `change_control` (CR 506.4d)
/// can reach it too.
pub fn remove_from_combat(state: &mut GameState, id: ObjectId) {
    state.remove_from_combat(id);
}
