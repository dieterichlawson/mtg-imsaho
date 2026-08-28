//! Tests for Geist of Saint Traft.
//!
//! Oracle: {1}{W}{U} 2/2 Legendary Creature — Spirit Cleric
//! Hexproof
//! Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token
//! with flying that's tapped and attacking. Exile that token at end of combat.

mod common;
use common::*;
use mtg_engine::cards::{AttackInfo, CardRegistry};
use mtg_engine::combat;
use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::*;

#[test]
fn geist_creates_angel_on_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let geist = named_permanent(&mut state, &reg, "Geist of Saint Traft", P0);

    // Simulate attack trigger.
    let behavior = reg.get(state.get_object(geist).unwrap().card_id).unwrap();
    attacks_unblocked(&mut state, geist, P1);
    behavior.on_attacks(&mut state, geist, AttackInfo::new(geist, P1), &[], &reg);

    // "a 4/4 white Angel creature token with flying that's tapped and
    // attacking" — every adjective, read through the accessors. The colour and
    // the flying had no assertion at all: a black Angel, or one without
    // flying, passed the whole workspace.
    let angel = state.objects.values()
        .find(|o| o.name == "Angel Token" && o.zone == Zone::Battlefield)
        .map(|o| o.id)
        .expect("Angel token should be on the battlefield");
    assert_eq!(state.effective_power(angel, &reg), Some(4));
    assert_eq!(state.effective_toughness(angel, &reg), Some(4));
    assert!(state.colors_of(angel, &reg).contains(&Color::White), "white");
    assert!(state.has_subtype(angel, "Angel", &reg), "an Angel");
    assert!(state.is_creature(angel, &reg), "a creature token");
    assert!(state.has_keyword(angel, Keyword::Flying, &reg), "with flying");
    assert!(state.get_object(angel).unwrap().tapped, "Angel should be tapped");
    assert_eq!(state.combat.as_ref().and_then(|c| c.attackers.get(&angel).copied()), Some(P1),
        "and attacking");
}

/// Ruling (2020-08-07): "Although the Angel is an attacking creature, it was
/// never declared as an attacking creature. This means that abilities that
/// trigger whenever a creature attacks won't trigger when it enters the
/// battlefield attacking."
///
/// No card in this pool triggers on *another* creature attacking, so the
/// observable half is the stamp the engine keeps for "was declared as an
/// attacker" (CR 508.1) — `attacked_on_turn`, which `state.attacked_this_turn`
/// reads and Civilized Scholar's end-step trigger depends on. Geist gets it
/// from `declare_attackers`; the Angel, inserted into `combat.attackers` by the
/// card, must not.
#[test]
fn the_angel_is_attacking_but_was_never_declared_an_attacker() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let geist = named_permanent(&mut state, &reg, "Geist of Saint Traft", P0);
    state.get_object_mut(geist).unwrap().summoning_sick = false;
    // Through the engine's declaration path, not `attacks_unblocked`: the stamp
    // this test is about is made by `combat::declare_attackers`, and a
    // hand-built `CombatState` never has it — for either creature.
    submit_declare_attackers(&mut state, &[(geist, P1)], &reg);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    let angel = find_token_named(&state, "Angel Token").expect("Angel should exist");

    assert!(state.attacked_this_turn(geist),
        "Geist was declared as an attacker (CR 508.1)");
    assert!(state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&angel)),
        "the Angel is an attacking creature");
    assert!(!state.attacked_this_turn(angel),
        "but it was never DECLARED one — it was put onto the battlefield attacking");
}

#[test]
fn angel_exiled_at_end_of_combat() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let geist = named_permanent(&mut state, &reg, "Geist of Saint Traft", P0);
    attacks_unblocked(&mut state, geist, P1);

    let behavior = reg.get(state.get_object(geist).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, geist, AttackInfo::new(geist, P1), &[], &reg);

    let angel_id = find_token_named(&state, "Angel Token").expect("Angel should exist");

    // End combat fires the delayed trigger; auto-resolve exiles the Angel.
    state.step = Step::EndCombat;
    fire_step_trigger(&mut state, Step::EndCombat, &reg);

    assert_eq!(state.get_object(angel_id).unwrap().zone, Zone::Exile,
        "Angel token should be exiled at end of combat");
}

#[test]
fn angel_exiled_even_if_geist_dies() {
    // "Exile that token at end of combat" is a delayed triggered ability.
    // It fires even if Geist has left the battlefield.
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let geist = named_permanent(&mut state, &reg, "Geist of Saint Traft", P0);
    attacks_unblocked(&mut state, geist, P1);

    let behavior = reg.get(state.get_object(geist).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, geist, AttackInfo::new(geist, P1), &[], &reg);

    let angel_id = find_token_named(&state, "Angel Token").expect("Angel should exist");

    // Kill the Geist before end of combat.
    state.move_object(geist, Zone::Graveyard, &reg);
    assert_eq!(state.get_object(geist).unwrap().zone, Zone::Graveyard,
        "Geist should be dead");

    // End combat fires the delayed trigger; the exile entry was recorded
    // at attack time, so the Angel is still exiled even after Geist dies.
    state.step = Step::EndCombat;
    fire_step_trigger(&mut state, Step::EndCombat, &reg);

    assert_eq!(state.get_object(angel_id).unwrap().zone, Zone::Exile,
        "Angel should be exiled even when Geist has left the battlefield");
}

// -------------------------------------------------------------------------
// Delayed end-of-combat exile (CR 603.7d)
// -------------------------------------------------------------------------

fn setup_geist_attacking(state: &mut mtg_engine::state::GameState, reg: &CardRegistry) -> (mtg_engine::ids::ObjectId, mtg_engine::ids::ObjectId) {
    let geist = named_permanent(state, reg, "Geist of Saint Traft", P0);
    attacks_unblocked(state, geist, P1);
    let behavior = reg.get(state.get_object(geist).unwrap().card_id).unwrap();
    behavior.on_attacks(state, geist, AttackInfo::new(geist, P1), &[], reg);

    let angel_id = state.objects.values()
        .find(|o| o.name == "Angel Token" && o.zone == Zone::Battlefield)
        .map(|o| o.id)
        .expect("Angel token should exist on battlefield after on_attacks");

    (geist, angel_id)
}

/// Oracle: "Exile that token at end of combat" creates a delayed triggered ability
/// that goes on the stack at the beginning of the end of combat step, giving players
/// priority to respond before the Angel is actually exiled.
///
/// Bug: the exile is implemented as a turn-based action inside combat::end_combat(),
/// which runs before any StepStarted{EndCombat} triggers fire. The Angel is unconditionally
/// exiled with no stack entry and no priority window.
#[test]
fn geist_angel_exile_is_triggered_not_turn_based() {
    let reg = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let (_geist, angel_id) = setup_geist_attacking(&mut state, &reg);

    // Simulate the engine's turn-based action that runs when entering the EndCombat step.
    // In the correct implementation this should not exile the Angel — the exile is a
    // triggered ability and must go on the stack first.
    combat::end_combat(&mut state, &reg);

    // Angel should still be on the battlefield: the exile triggered ability has not yet
    // gone on the stack, let alone resolved. Players have not had priority to respond.
    assert_eq!(
        state.get_object(angel_id).unwrap().zone,
        Zone::Battlefield,
        "Angel token should still be on the battlefield after turn-based actions; \
         oracle says exile is a triggered ability that goes on the stack, \
         not an immediate turn-based action that bypasses priority",
    );
}

/// Bug: Geist of Saint Traft registers a TriggerKind::EndCombat triggered ability
/// ("exile the Angel token"). Because on_end_combat is not implemented, this fires
/// as a no-op EndCombatTrigger every end of combat step while Geist is on the
/// battlefield — including turns where Geist did not attack and no Angel token exists.
///
/// Oracle says: the exile trigger only arises because Geist attacked and created a token.
/// On turns where Geist did not attack, no end-of-combat trigger should appear.
#[test]
fn geist_no_spurious_end_combat_trigger_when_did_not_attack() {
    let reg = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::EndCombat, P0);

    // Geist is on the battlefield but did not attack this turn.
    let _geist = named_permanent(&mut state, &reg, "Geist of Saint Traft", P0);

    // Push the StepStarted{EndCombat} event and collect triggers without resolving them.
    // This lets us inspect what was queued before auto-resolution clears the stack.
    state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::EndCombat });
    mtg_engine::triggers::collect_triggers(&mut state, &reg);

    // No EndCombatTrigger should have been created: Geist didn't attack this turn,
    // so there is no Angel token to exile and no reason for a triggered ability.
    let has_end_combat_trigger = state.stack.iter()
        .filter_map(|e| e.as_trigger())
        .any(|t| matches!(t, PendingTrigger { source: TriggerSource { .. }, event: TriggerEvent::EndCombat }))
        || state.pending_trigger_pushes_ap.iter()
            .any(|t| matches!(t, PendingTrigger {
                source: TriggerSource { .. },
                event: TriggerEvent::EndCombat,
            }))
        || state.pending_trigger_pushes_nap.iter()
            .any(|t| matches!(t, PendingTrigger {
                source: TriggerSource { .. },
                event: TriggerEvent::EndCombat,
            }));

    assert_eq!(
        has_end_combat_trigger,
        false,
        "No EndCombatTrigger should be created for Geist of Saint Traft when it did \
         not attack this turn; the exile trigger only exists when an Angel token was created",
    );
}

/// Ruling: "The Angel token will be attacking the same player or planeswalker
/// that Geist of Saint Traft is attacking." The defender is read from Geist's
/// own entry in `combat.attackers`, not recomputed — recomputing it as "the
/// controller's opponent" happens to agree in a two-player game and is the
/// wrong rule.
#[test]
fn the_angel_token_attacks_whoever_geist_is_attacking() {
    use mtg_engine::cards::AttackInfo;
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let geist = named_permanent(&mut state, &reg, "Geist of Saint Traft", P0);
    attacks_unblocked(&mut state, geist, P1);

    let card_id = state.get_object(geist).unwrap().card_id;
    reg.get(card_id).unwrap()
        .on_attacks(&mut state, geist, AttackInfo::new(geist, P1), &[], &reg);

    let angel = state.objects.values()
        .find(|o| o.is_token && o.name.contains("Angel") && o.controller == P0)
        .map(|o| o.id)
        .expect("Geist should have spawned an Angel token");

    assert_eq!(state.combat.as_ref().and_then(|c| c.attackers.get(&angel).copied()), Some(P1),
        "the Angel is inserted into combat attacking the same player as Geist");
}

/// The same rule, on a board where "the player Geist is attacking" and "the
/// opponent" are different players.
///
/// The card used to write `state.opponent(controller)` and ignore the
/// `AttackInfo` its own trigger carried. With two players those agree, so the
/// test above passed either way; with three they do not, and re-deriving the
/// defender sends the Angel at the wrong player.
#[test]
fn the_angel_attacks_geists_defender_and_not_just_the_next_player() {
    use mtg_engine::actions::{Action, Target};
    use mtg_engine::cards::AttackInfo;
    use mtg_engine::ids::PlayerId;
    const P2: PlayerId = PlayerId(2);

    let reg = registry();
    let mut state = mtg_engine::state::GameState::new(3);
    state.step = Step::DeclareAttackers;
    state.active_player = P0;
    state.priority_player = Some(P0);
    state.is_first_turn = false;
    for p in 0..3 {
        state.players[p].life = 20;
    }
    assert_eq!(state.opponent(P0), P1, "test setup: the *next* player is P1");

    let geist = named_permanent(&mut state, &reg, "Geist of Saint Traft", P0);
    // ...but Geist is attacking P2.
    attacks_unblocked(&mut state, geist, P2);

    let card_id = state.get_object(geist).unwrap().card_id;
    reg.get(card_id).unwrap()
        .on_attacks(&mut state, geist, AttackInfo::new(geist, P2), &[], &reg);

    // With two live opponents there is a real choice, so the ruling's first
    // sentence applies: "You choose which player or planeswalker the Angel
    // token is attacking." The controller is asked rather than the engine
    // assuming Geist's own defender.
    let options = match &state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            player, choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. }, ..
        }) => {
            assert_eq!(*player, P0, "the token's controller chooses");
            options.clone()
        }
        other => panic!("expected an attack-target choice, got {other:?}"),
    };
    assert!(options.contains(&Target::Player(P1)) && options.contains(&Target::Player(P2)),
        "both opponents are legal; got {options:?}");

    let state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Player(P2))),
    }, &reg);

    let angel = state.objects.values()
        .find(|o| o.is_token && o.name.contains("Angel") && o.controller == P0)
        .map(|o| o.id)
        .expect("Geist should have spawned an Angel token");

    assert_eq!(state.combat.as_ref().and_then(|c| c.attackers.get(&angel).copied()), Some(P2),
        "the Angel attacks the player its controller chose");
}
