//! Creatures with several interacting abilities — transform, a trigger and an
//! activated ability at once. The largest of the per-card files.
//!
//! Cards covered (21), so this is greppable by name as well as by rule:
//!
//! - Back from the Brink
//! - Bitterheart Witch
//! - Cellar Door
//! - Creeping Renaissance
//! - Creepy Doll
//! - Curse of Stalked Prey
//! - Dearly Departed
//! - Evil Twin
//! - Galvanic Juggernaut
//! - Geistcatcher's Rig
//! - Gutter Grime
//! - Heretic's Punishment
//! - Kessig Cagebreakers
//! - Manor Gargoyle
//! - Mentor of the Meek
//! - Mirror-Mad Phantasm
//! - Moldgraf Monstrosity
//! - Skaab Ruinator
//! - Tree of Redemption
//! - Unbreathing Horde
//! - Undead Alchemist

mod common;

use common::*;
use mtg_engine::cards::AttackInfo;
use mtg_engine::ids::{CardId, ObjectId};
use mtg_engine::engine;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::triggers;

use mtg_engine::types::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::{DamageTarget, GameEvent};
use mtg_engine::state::StackEntry;
// ── Curse of Stalked Prey ────────────────────────────────────────

/// Count the triggers this Curse has put on the stack.
fn curse_triggers_on_stack(state: &mtg_engine::state::GameState, curse: ObjectId) -> usize {
    state.stack.iter().filter(|e| matches!(e,
        StackEntry::Trigger(t) if t.source.id == curse)).count()
}

/// Drive a creature's combat damage to a player through the real event path,
/// the way the combat damage step does.
fn deal_combat_damage_to(
    state: &mut mtg_engine::state::GameState,
    reg: &CardRegistry,
    source: ObjectId,
    player: PlayerId,
    amount: u32,
) {
    state.events.push(GameEvent::CombatDamageDealt {
        source,
        target: DamageTarget::Player(player),
        amount,
    });
    triggers::process_triggers(state, reg);
}

/// "Whenever a creature deals combat damage to **enchanted player**, put a
/// +1/+1 counter on that creature."
///
/// Both arms, and the stack as well as the counter. CR 603.2 makes "to
/// enchanted player" part of the trigger event, so damage to anyone else does
/// not make the ability trigger — it must not put a do-nothing entry on the
/// stack, which is a real game object with a priority window around it.
#[test]
fn curse_of_stalked_prey_only_triggers_for_damage_to_the_enchanted_player() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let curse = attach_curse_to_player(&mut state, &reg, "Curse of Stalked Prey", P0, P1);
    let attacker = ready_creature(&mut state, P0, 2, 2);

    // Damage to a player the Curse is not on.
    deal_combat_damage_to(&mut state, &reg, attacker, P0, 2);
    assert_eq!(curse_triggers_on_stack(&state, curse), 0,
        "the ability does not trigger at all on damage to another player");
    assert_eq!(counters_of(&state, attacker, CounterType::PlusOnePlusOne), 0,
        "and so puts no counter");

    // Damage to the enchanted player.
    deal_combat_damage_to(&mut state, &reg, attacker, P1, 2);
    assert_eq!(counters_of(&state, attacker, CounterType::PlusOnePlusOne), 1,
        "the creature that dealt the damage gets the counter");
}

/// Ruling: "The ability will trigger when **any** creature deals combat damage
/// to the enchanted player, including one controlled by another opponent or
/// even by the enchanted player (if combat damage gets redirected somehow)."
///
/// The text is "a creature", with no "you control" — this is the restriction
/// the card conspicuously does not have.
#[test]
fn curse_of_stalked_prey_triggers_for_any_creature_whoever_controls_it() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let _curse = attach_curse_to_player(&mut state, &reg, "Curse of Stalked Prey", P0, P1);
    let mine = ready_creature(&mut state, P0, 2, 2);
    // A creature the enchanted player controls, dealing combat damage to
    // themselves — the case the ruling calls out.
    let theirs = ready_creature(&mut state, P1, 2, 2);

    // Both in one damage event batch, the way a combat damage step deals it.
    for source in [mine, theirs] {
        state.events.push(GameEvent::CombatDamageDealt {
            source,
            target: DamageTarget::Player(P1),
            amount: 2,
        });
    }
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(counters_of(&state, mine, CounterType::PlusOnePlusOne), 1,
        "the Curse controller's creature");
    assert_eq!(counters_of(&state, theirs, CounterType::PlusOnePlusOne), 1,
        "and the enchanted player's own creature — 'a creature', not 'a \
         creature you control'");
}

/// CR 121.1: a counter goes only on a permanent still on the battlefield. A
/// creature that dealt its combat damage and died in the same step gets
/// nothing, even though the ability did trigger.
#[test]
fn curse_of_stalked_prey_puts_no_counter_on_a_creature_that_already_died() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let _curse = attach_curse_to_player(&mut state, &reg, "Curse of Stalked Prey", P0, P1);
    let attacker = ready_creature(&mut state, P0, 2, 2);

    state.events.push(GameEvent::CombatDamageDealt {
        source: attacker,
        target: DamageTarget::Player(P1),
        amount: 2,
    });
    // It traded with a blocker in the same damage step.
    state.move_object(attacker, Zone::Graveyard, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(counters_of(&state, attacker, CounterType::PlusOnePlusOne), 0,
        "nothing on the battlefield to put a counter on");
}

// ── Dearly Departed ──────────────────────────────────────────────

#[test]
fn dearly_departed_gives_counter_to_entering_humans() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Dearly Departed in the graveyard.
    let _dd = named_card_in_graveyard(&mut state, &reg, "Dearly Departed", P0);

    // Create a Human in hand, then move to battlefield so the
    // entering-with-counters replacement effect fires.
    let champ_id = reg.get_id_by_name("Champion of the Parish").unwrap();
    let human = state.create_object(champ_id, P0, Zone::Hand, Some(1), Some(1));
    state.get_object_mut(human).unwrap().name = "Champion of the Parish".into();
    state.move_object(human, Zone::Battlefield, &reg);

    assert_eq!(counters_of(&state, human, CounterType::PlusOnePlusOne), 1,
        "Human should enter with a +1/+1 counter from Dearly Departed");
}

// ── Mentor of the Meek ───────────────────────────────────────────

#[test]
fn mentor_of_the_meek_draws_when_small_creature_enters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mentor = named_permanent(&mut state, &reg, "Mentor of the Meek", P0);

    // Give P0 mana to pay {1}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Put a card in library to draw.
    let lib_card = state.create_object(CardId(9999), P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(lib_card);

    // A 1/1 creature enters under our control.
    let small_creature = ready_creature(&mut state, P0, 1, 1);

    let behavior = reg.get(state.get_object(mentor).unwrap().card_id).unwrap();
    behavior.on_any_creature_enters(&mut state, mentor, small_creature, P0, &reg);

    // Oracle: "you may pay {1}" — should present a choice.
    assert!(state.awaiting_action.is_some(), "Should present 'you may pay' choice");

    // Player chooses yes (pay {1} and draw).
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );

    // Should have drawn a card.
    let hand_count = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();
    assert_eq!(hand_count, 1, "Mentor of the Meek should have drawn a card");

    // Mana should be spent.
    assert_eq!(state.get_player(P0).mana_pool.total(), 0, "Should have spent 1 mana");
}

// ── Kessig Cagebreakers ─────────────────────────────────────────

#[test]
fn kessig_cagebreakers_creates_wolf_tokens_on_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let cage = named_permanent(&mut state, &reg, "Kessig Cagebreakers", P0);

    // Set up combat: Kessig Cagebreakers is attacking P1.
    attacks_unblocked(&mut state, cage, P1);

    // Put 3 creature cards in the graveyard...
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 2, 2);
        state.move_object(c, Zone::Graveyard, &reg);
    }
    // ...and a non-creature card, which "for each creature card in your
    // graveyard" does not count.
    named_card_in_graveyard(&mut state, &reg, "Rebuke", P0);
    // A creature card in the *opponent's* graveyard is not in "your" graveyard.
    let theirs = ready_creature(&mut state, P1, 2, 2);
    state.move_object(theirs, Zone::Graveyard, &reg);

    let behavior = reg.get(state.get_object(cage).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, cage, AttackInfo::new(cage, P1), &[], &reg);

    // Should have 3 Wolf tokens on the battlefield. CR 111.4 names a token
    // after its subtypes, so these are "Wolf Token", not "Wolf" — a filter on
    // the latter matches nothing and asserts nothing.
    assert_eq!(count_tokens_named(&state, "Wolf Token"), 3, "Should have created 3 Wolf tokens");

    let wolves: Vec<ObjectId> = state.objects.values()
        .filter(|o| o.is_token && o.zone == Zone::Battlefield && o.name == "Wolf Token")
        .map(|o| o.id)
        .collect();
    assert_eq!(wolves.len(), 3);
    for wolf in wolves {
        // "a 2/2 green Wolf creature token that's tapped and attacking".
        assert_eq!(state.effective_power(wolf, &reg), Some(2), "Wolf token should be 2 power");
        assert_eq!(state.effective_toughness(wolf, &reg), Some(2), "Wolf token should be 2 toughness");
        assert_eq!(state.colors_of(wolf, &reg), vec![Color::Green], "Wolf token should be green");
        assert!(state.is_creature(wolf, &reg), "Wolf token should be a creature");
        assert!(state.has_subtype(wolf, "Wolf", &reg), "Wolf token should be a Wolf");
        assert!(state.get_object(wolf).unwrap().tapped, "Wolf tokens should be tapped");
        assert_eq!(state.combat.as_ref().unwrap().attackers.get(&wolf).copied(), Some(P1),
            "Wolf tokens should be attacking the player the Cagebreakers is attacking");
    }
    let combat_attackers = state.combat.as_ref().unwrap().attackers.len();
    // Cage + 3 wolves = 4 attackers.
    assert_eq!(combat_attackers, 4, "Should have 4 attackers (cage + 3 wolves)");
}

/// Ruling: the tokens are attacking, but "they were never declared as
/// attacking creatures" — and the player they attack comes from the trigger,
/// not from whoever happens to be the next player in turn order. With three
/// players that is a different answer.
#[test]
fn kessig_wolves_attack_the_cagebreakers_defender_and_not_just_the_next_player() {
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

    let cage = named_permanent(&mut state, &reg, "Kessig Cagebreakers", P0);
    // ...but the Cagebreakers is attacking P2.
    attacks_unblocked(&mut state, cage, P2);

    let c = ready_creature(&mut state, P0, 2, 2);
    state.move_object(c, Zone::Graveyard, &reg);

    let card_id = state.get_object(cage).unwrap().card_id;
    reg.get(card_id).unwrap()
        .on_attacks(&mut state, cage, AttackInfo::new(cage, P2), &[], &reg);

    // With two live opponents there is a real choice, so the ruling applies:
    // "You declare which player or planeswalker each token is attacking as
    // you put it onto the battlefield." The controller is asked rather than
    // the engine assuming the Cagebreakers' own defender.
    match &state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            player, choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. }, ..
        }) => {
            assert_eq!(*player, P0, "the token's controller chooses");
            assert!(options.contains(&Target::Player(P1)) && options.contains(&Target::Player(P2)),
                "both opponents are legal; got {options:?}");
        }
        other => panic!("expected an attack-target choice, got {other:?}"),
    }
    let state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Player(P2))),
    }, &reg);

    let wolf = find_token_named(&state, "Wolf Token").expect("should have created a Wolf token");
    assert_eq!(state.combat.as_ref().and_then(|c| c.attackers.get(&wolf).copied()), Some(P2),
        "the Wolf attacks the player its controller chose");
}

// ── Galvanic Juggernaut ──────────────────────────────────────────

/// "Whenever another creature dies, untap this creature." Through the trigger
/// system, not by calling the hook: the hook is reached only if the card's
/// `TriggerKind::AnyCreatureDies` declaration is right and the death-watch
/// collector picks the Juggernaut up, and calling it directly tests neither.
///
/// This is also the card's own two lines meeting each other. "Doesn't untap
/// during your untap step" is about the untap step alone (CR 302.6), so the
/// Juggernaut's own trigger untaps it — which is the entire point of the card.
#[test]
fn galvanic_juggernaut_untaps_when_another_creature_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let jug = named_permanent(&mut state, &reg, "Galvanic Juggernaut", P0);
    state.tap(jug);
    assert!(!state.untaps_normally(jug, &reg),
        "test setup: it is under its own \"doesn't untap\" restriction");

    let dead = ready_creature(&mut state, P1, 1, 1);
    kill_by_damage(&mut state, &reg, dead);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert!(!state.get_object(jug).unwrap().tapped,
        "the death untapped it, restriction and all");
}

/// The other half, which is what makes the trigger worth having: left alone,
/// the Juggernaut stays tapped through its controller's untap step.
///
/// An ordinary tapped creature beside it untaps, so this shows the untap step
/// really ran rather than being skipped.
#[test]
fn galvanic_juggernaut_does_not_untap_during_the_untap_step() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let jug = named_permanent(&mut state, &reg, "Galvanic Juggernaut", P0);
    let other = ready_creature(&mut state, P0, 2, 2);
    state.tap(jug);
    state.tap(other);

    // Round the table back to P0's untap step.
    advance_to_next_turn(&mut state, &reg);
    advance_to_next_turn(&mut state, &reg);
    assert_eq!(state.active_player, P0, "back to the Juggernaut's controller's turn");

    assert!(!state.get_object(other).unwrap().tapped,
        "an ordinary creature untapped, so the untap step ran");
    assert!(state.get_object(jug).unwrap().tapped,
        "the Juggernaut does not untap during its controller's untap step");
}

// ── Bitterheart Witch ────────────────────────────────────────────

#[test]
fn bitterheart_witch_finds_curse_on_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);

    // Put a curse in the library.
    let curse_card_id = reg.get_id_by_name("Curse of the Pierced Heart").unwrap();
    let curse_obj = state.create_object(curse_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(curse_obj).unwrap().name = "Curse of the Pierced Heart".into();
    state.get_player_mut(P0).library_order.push(curse_obj);

    // CR 603.3d: "attached to target player" is targeted, so the player was
    // chosen when the trigger went on the stack — before the search.
    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, witch, &[Target::Player(P1)], &reg);
    assert!(state.awaiting_action.is_some(), "Should be awaiting yes/no choice");

    // Player chooses yes to search...
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );
    // ...then picks the Curse. CR 701.19b: searching never forces a find, so
    // the choice is offered even with a single Curse in the library.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(Target::Object(curse_obj))) },
        &reg,
    );

    // The curse should be on the battlefield attached to opponent.
    let curse = state.get_object(curse_obj).unwrap();
    assert_eq!(curse.zone, Zone::Battlefield, "Curse should be on battlefield");
    assert_eq!(curse.attached_to_player, Some(P1), "Curse should be attached to opponent");
}

#[test]
fn bitterheart_witch_can_attach_curse_to_self() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);

    // Put a curse in the library.
    let curse_card_id = reg.get_id_by_name("Curse of the Pierced Heart").unwrap();
    let curse_obj = state.create_object(curse_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(curse_obj).unwrap().name = "Curse of the Pierced Heart".into();
    state.get_player_mut(P0).library_order.push(curse_obj);

    // Targeting yourself is legal — the card says "target player", not
    // "target opponent".
    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, witch, &[Target::Player(P0)], &reg);
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(Target::Object(curse_obj))) },
        &reg,
    );

    // The curse should be on the battlefield attached to self.
    let curse = state.get_object(curse_obj).unwrap();
    assert_eq!(curse.zone, Zone::Battlefield, "Curse should be on battlefield");
    assert_eq!(curse.attached_to_player, Some(P0), "Curse should be attached to self");
}

#[test]
fn bitterheart_witch_decline_search() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);

    // Put a curse in the library.
    let curse_card_id = reg.get_id_by_name("Curse of the Pierced Heart").unwrap();
    let curse_obj = state.create_object(curse_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(curse_obj).unwrap().name = "Curse of the Pierced Heart".into();
    state.get_player_mut(P0).library_order.push(curse_obj);

    // Trigger death.
    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, witch, &[Target::Player(P1)], &reg);
    assert!(state.awaiting_action.is_some(), "Should be awaiting yes/no choice");

    // Player declines to search.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(false) },
        &reg,
    );

    // Curse should still be in the library.
    let curse = state.get_object(curse_obj).unwrap();
    assert_eq!(curse.zone, Zone::Library, "Curse should remain in library when search declined");
}

// ── Gutter Grime ─────────────────────────────────────────────────

#[test]
fn gutter_grime_creates_ooze_on_creature_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);

    // A nontoken creature we control dies.
    let dead = ready_creature(&mut state, P0, 2, 2);

    let behavior = reg.get(state.get_object(grime).unwrap().card_id).unwrap();
    behavior.on_any_creature_dies(&mut state, grime, dead, P0, &[], 2, false, &[], &reg);

    // Should have a slime counter on Gutter Grime.
    assert_eq!(counters_of(&state, grime, CounterType::Slime), 1,
        "Gutter Grime should have 1 slime counter");

    // Should have created an Ooze token.
    assert_eq!(count_tokens_named(&state, "Ooze Token"), 1, "Should have created 1 Ooze token");

    // "create a **green Ooze creature** token" — every word of it.
    let ooze = find_token_named(&state, "Ooze Token").unwrap();
    assert_eq!(state.colors_of(ooze, &reg), vec![Color::Green], "the Ooze is green");
    assert!(state.is_creature(ooze, &reg), "and a creature");
    assert!(state.has_subtype(ooze, "Ooze", &reg), "and an Ooze");
    assert_eq!(state.get_object(ooze).unwrap().controller, P0,
        "under the Gutter Grime's controller");
}

// ── Heretic's Punishment ─────────────────────────────────────────

#[test]
fn heretics_punishment_mills_then_deals_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let hp = named_permanent(&mut state, &reg, "Heretic's Punishment", P0);

    // Put cards in library. Use a known card so mana value is deterministic.
    // Kalonian Tusker costs {G}{G} = MV 2.
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    for _ in 0..3 {
        let card = state.create_object(tusker_id, P0, Zone::Library, Some(3), Some(3));
        state.get_object_mut(card).unwrap().name = "Kalonian Tusker".into();
        state.get_player_mut(P0).library_order.insert(0, card);
    }

    let initial_life = state.get_player(P1).life;
    activate_via_hooks(&mut state, &reg, hp, 0, &[mtg_engine::actions::Target::Player(P1)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Should have dealt 2 damage (MV of Kalonian Tusker).
    let new_life = state.get_player(P1).life;
    assert_eq!(initial_life - new_life, 2, "Should deal damage equal to greatest MV (2)");

    // Milled cards should be in graveyard, not library.
    let lib_count = state.get_player(P0).library_order.len();
    assert_eq!(lib_count, 0, "Library should be empty after milling 3 cards");

    let gy_count = state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P0 && o.name == "Kalonian Tusker")
        .count();
    assert_eq!(gy_count, 3, "All 3 milled cards should be in graveyard");
}

#[test]
fn heretics_punishment_tracks_damaged_by_on_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let hp = named_permanent(&mut state, &reg, "Heretic's Punishment", P0);

    // Put cards in library.
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    for _ in 0..3 {
        let card = state.create_object(tusker_id, P0, Zone::Library, Some(3), Some(3));
        state.get_object_mut(card).unwrap().name = "Kalonian Tusker".into();
        state.get_player_mut(P0).library_order.insert(0, card);
    }

    let target_creature = ready_creature(&mut state, P1, 5, 5);

    activate_via_hooks(&mut state, &reg, hp, 0, &[Target::Object(target_creature)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    let obj = state.get_object(target_creature).unwrap();
    assert_eq!(obj.damage_marked, 2, "Creature should have 2 damage marked");
    assert!(obj.damaged_by.contains(&hp), "damaged_by should track the source");
}

#[test]
fn heretics_punishment_fizzles_when_target_illegal() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let hp = named_permanent(&mut state, &reg, "Heretic's Punishment", P0);

    // Put cards in library.
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    for _ in 0..3 {
        let card = state.create_object(tusker_id, P0, Zone::Library, Some(3), Some(3));
        state.get_object_mut(card).unwrap().name = "Kalonian Tusker".into();
        state.get_player_mut(P0).library_order.insert(0, card);
    }

    // Create a creature target then move it off the battlefield (illegal target).
    let target_creature = ready_creature(&mut state, P1, 3, 3);
    state.move_object(target_creature, Zone::Graveyard, &reg);

    activate_via_hooks(&mut state, &reg, hp, 0, &[Target::Object(target_creature)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Entire ability should fizzle: no cards milled.
    let lib_count = state.get_player(P0).library_order.len();
    assert_eq!(lib_count, 3, "Library should be unchanged when ability fizzles");
}

/// Put `n` copies of `card` on top of P0's library.
fn library_top(state: &mut mtg_engine::state::GameState, reg: &CardRegistry, card: &str, n: usize) {
    let card_id = reg.get_id_by_name(card).unwrap_or_else(|| panic!("Unknown card: {card}"));
    for _ in 0..n {
        let obj = state.create_object(card_id, P0, Zone::Library, None, None);
        state.get_object_mut(obj).unwrap().name = card.into();
        state.get_player_mut(P0).library_order.insert(0, obj);
    }
}

/// Ruling: "If you have two or fewer cards in your library when the ability
/// resolves, all of them will be put into your graveyard. Heretic's Punishment
/// will still deal damage equal to the highest mana value among those cards."
#[test]
fn heretics_punishment_mills_a_short_library_and_still_deals_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let hp = named_permanent(&mut state, &reg, "Heretic's Punishment", P0);
    // Two cards only. Kalonian Tusker is {G}{G}, mana value 2.
    library_top(&mut state, &reg, "Kalonian Tusker", 2);

    let before = state.get_player(P1).life;
    activate_via_hooks(&mut state, &reg, hp, 0, &[Target::Player(P1)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_player(P0).library_order.len(), 0,
        "all of them go, and running out is not a failure to resolve");
    assert_eq!(before - state.get_player(P1).life, 2,
        "damage is the highest mana value among the cards that were milled");
}

/// Ruling: "If all three cards have a mana value of 0, no damage will be
/// dealt." Basic lands have no mana cost at all, which is the same zero.
#[test]
fn heretics_punishment_deals_no_damage_when_every_card_is_mana_value_zero() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let hp = named_permanent(&mut state, &reg, "Heretic's Punishment", P0);
    library_top(&mut state, &reg, "Forest", 3);

    let before = state.get_player(P1).life;
    activate_via_hooks(&mut state, &reg, hp, 0, &[Target::Player(P1)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_player(P0).library_order.len(), 0,
        "the mill still happens");
    assert_eq!(state.get_player(P1).life, before,
        "but nothing is dealt — not a zero-damage event, which damage watchers \
         would otherwise see");
    assert!(!state.events.iter().any(|e| matches!(e,
        mtg_engine::events::GameEvent::NonCombatDamageDealt { .. })),
        "and no damage event is emitted at all");
}

/// Ruling: "The mana value of a double-faced card in your graveyard is the mana
/// value of the front face."
///
/// Villagers of Estwald is {2}{G}, mana value 3; its back face, Howlpack of
/// Estwald, has no mana cost at all, so reading the wrong face gives 0.
#[test]
fn heretics_punishment_reads_a_double_faced_cards_front_face() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let hp = named_permanent(&mut state, &reg, "Heretic's Punishment", P0);
    library_top(&mut state, &reg, "Villagers of Estwald", 3);

    let before = state.get_player(P1).life;
    activate_via_hooks(&mut state, &reg, hp, 0, &[Target::Player(P1)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(before - state.get_player(P1).life, 3,
        "the front face mana value of Villagers of Estwald is 3");
}

// ── Undead Alchemist ─────────────────────────────────────────────

#[test]
fn undead_alchemist_mills_instead_of_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let _alchemist = named_permanent(&mut state, &reg, "Undead Alchemist", P0);

    // Create a Zombie that dealt damage.
    let zombie = state.create_token_with_subtypes(
        "Zombie", P0, 2, 2,
        vec![Color::Black], vec![CardType::Creature], vec![],
        vec!["Zombie".into()],
        &reg,
    )[0];
    state.get_object_mut(zombie).unwrap().summoning_sick = false;

    // Put creature cards in P1's library so milling creates tokens.
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    for _ in 0..2 {
        let card = state.create_object(tusker_id, P1, Zone::Library, Some(3), Some(3));
        state.get_object_mut(card).unwrap().name = "Kalonian Tusker".into();
        state.get_player_mut(P1).library_order.insert(0, card);
    }

    let initial_life = state.get_player(P1).life;

    // The replacement effect intercepts damage before it's applied.
    let replaced = mtg_engine::replacement::apply(
        &mut state,
        mtg_engine::replacement::ReplaceableEvent::DealsDamage {
            source: zombie,
            target: mtg_engine::events::DamageTarget::Player(P1),
            amount: 2,
            combat: true,
        },
        &reg,
    )
    .is_none();
    assert!(replaced, "Undead Alchemist should replace Zombie combat damage");

    // Process triggers so the CreatureCardMilled events fire the exile+token ability.
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    // Life should be unchanged (damage was replaced, never applied).
    assert_eq!(state.get_player(P1).life, initial_life, "Life should be unchanged — damage was replaced");

    // The creature cards should have been exiled (not in graveyard).
    let p1_exile = state.objects.values()
        .filter(|o| o.zone == Zone::Exile && o.owner == P1)
        .count();
    assert_eq!(p1_exile, 2, "Milled creature cards should be exiled");

    // Should have created Zombie tokens.
    // 2 creature cards milled = 2 Zombie tokens + the original zombie we created.
    assert!(count_tokens_named_by(&state, "Zombie Token", P0) >= 2,
        "Should create Zombie tokens for each milled creature");
}

// ── Creeping Renaissance ─────────────────────────────────────────

#[test]
fn creeping_renaissance_returns_creatures_from_graveyard() {
    use mtg_engine::actions::{Action, ResolvedChoice};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put creature cards in graveyard.
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 2, 2);
        state.get_object_mut(c).unwrap().card_types = vec![CardType::Creature];
        state.move_object(c, Zone::Graveyard, &reg);
    }

    let spell = castable_spell(&mut state, &reg, "Creeping Renaissance", P0);
    // Cast the spell and put it on the stack.
    state = cast_onto_stack(&state, &reg, spell, vec![]);
    // Resolve: this triggers a ChooseCardType choice.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert!(state.awaiting_action.is_some(), "Should be awaiting card type choice");

    // Choose "Creature" (index 0).
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(0, "Option 0".into()) },
        &reg,
    );

    // All 3 creatures should be in hand now.
    let hand_creatures = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0 && o.power.is_some())
        .count();
    assert_eq!(hand_creatures, 3, "Should return all creature cards from graveyard to hand");
}

#[test]
fn creeping_renaissance_only_returns_chosen_type() {
    use mtg_engine::actions::{Action, ResolvedChoice};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put creatures and enchantments in graveyard.
    for _ in 0..2 {
        let c = ready_creature(&mut state, P0, 2, 2);
        state.get_object_mut(c).unwrap().card_types = vec![CardType::Creature];
        state.move_object(c, Zone::Graveyard, &reg);
    }
    for _ in 0..2 {
        let e = state.create_object(CardId(9999), P0, Zone::Battlefield, None, None);
        state.get_object_mut(e).unwrap().card_types = vec![CardType::Enchantment];
        state.move_object(e, Zone::Graveyard, &reg);
    }

    let spell = castable_spell(&mut state, &reg, "Creeping Renaissance", P0);
    state = cast_onto_stack(&state, &reg, spell, vec![]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Choose "Enchantment" (index 2).
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(2, "Option 2".into()) },
        &reg,
    );

    // Only enchantments should be in hand.
    let hand_enchantments = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0 && o.card_types.contains(&CardType::Enchantment))
        .count();
    assert_eq!(hand_enchantments, 2, "Should return enchantments to hand");

    // Creatures should still be in graveyard.
    let gy_creatures = state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P0 && o.card_types.contains(&CardType::Creature))
        .count();
    assert_eq!(gy_creatures, 2, "Creatures should remain in graveyard");
}

/// Ruling: "The permanent types are artifact, creature, enchantment, land, and
/// planeswalker." All five are offered, each returns its own kind, and
/// "**your** graveyard" is the caster's — a card goes to its owner's
/// (CR 404.3), and an opponent's is not yours.
///
/// The two tests above cover Creature and Enchantment with only the caster's
/// cards in play, so neither the other three options nor the word "your" was
/// exercised.
#[test]
fn creeping_renaissance_returns_the_chosen_type_from_your_graveyard_only() {
    use mtg_engine::actions::{Action, ResolvedChoice};

    // The options in the order the card offers them.
    let types = [
        (0usize, CardType::Creature),
        (1, CardType::Artifact),
        (2, CardType::Enchantment),
        (3, CardType::Land),
        (4, CardType::Planeswalker),
    ];

    for &(index, wanted) in &types {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        // One card of every type in each player's graveyard.
        let mut mine = Vec::new();
        let mut theirs = Vec::new();
        for &(_, t) in &types {
            for (owner, into) in [(P0, &mut mine), (P1, &mut theirs)] {
                let c = state.create_object(CardId(9999), owner, Zone::Battlefield, None, None);
                state.get_object_mut(c).unwrap().card_types = vec![t];
                state.move_object(c, Zone::Graveyard, &reg);
                into.push((t, c));
            }
        }

        let spell = castable_spell(&mut state, &reg, "Creeping Renaissance", P0);
        let mut state = cast_onto_stack(&state, &reg, spell, vec![]);
        mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
        let state = mtg_engine::engine::submit_action(
            &state,
            &Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(index, String::new()) },
            &reg,
        );

        for (t, id) in &mine {
            let zone = state.get_object(*id).unwrap().zone;
            let expected = if *t == wanted { Zone::Hand } else { Zone::Graveyard };
            assert_eq!(zone, expected, "{wanted:?} chosen: your {t:?} card");
        }
        for (t, id) in &theirs {
            assert_eq!(state.get_object(*id).unwrap().zone, Zone::Graveyard,
                "{wanted:?} chosen: an opponent's {t:?} card is not in your graveyard");
        }
    }
}

#[test]
fn creeping_renaissance_flashback_exiles() {
    use mtg_engine::actions::{Action, ResolvedChoice};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a creature in graveyard.
    let c = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(c).unwrap().card_types = vec![CardType::Creature];
    state.move_object(c, Zone::Graveyard, &reg);

    // Put Creeping Renaissance itself in graveyard for flashback.
    let card_id = reg.get_id_by_name("Creeping Renaissance").unwrap();
    let spell = state.create_object(card_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(spell).unwrap().name = "Creeping Renaissance".into();

    // Add flashback mana (5GG = 7 total).
    for _ in 0..5 { state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1); }
    for _ in 0..2 { state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1); }

    // Cast via flashback.
    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let fb = actions.actions.iter().find(|a| match a {
        Action::CastSpell { object_id, .. } => object_id == &spell,
        _ => false,
    });
    assert!(fb.is_some(), "Should be able to flashback Creeping Renaissance");

    state = mtg_engine::engine::submit_action(&state, fb.unwrap(), &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Choose "Creature" (index 0).
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(0, "Option 0".into()) },
        &reg,
    );

    // Creature in hand.
    let hand = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0 && o.card_types.contains(&CardType::Creature))
        .count();
    assert_eq!(hand, 1, "Creature should be in hand");

    // Creeping Renaissance should be exiled (flashback). A card is never
    // removed from the game outright, so "gone" is not an acceptable outcome.
    assert_eq!(state.get_object(spell).unwrap().zone, Zone::Exile,
        "Creeping Renaissance should be exiled after flashback");
}

// ── Cellar Door ──────────────────────────────────────────────────

/// "{3}, {T}: Target player puts the **bottom** card of their library into
/// their graveyard. **If it's a creature card**, **you** create a 2/2 black
/// Zombie creature token."
///
/// Each emphasis is a clause that had no test. The version this replaces put a
/// single creature card in the library and asserted a Zombie appeared: with one
/// card, the bottom and the top are the same object, so milling from the top
/// passed — and so did creating the token unconditionally. A decoy on the other
/// end of the library separates them.
#[test]
fn cellar_door_mills_the_bottom_card_and_zombies_only_for_a_creature() {
    // (what sits on the bottom, what sits on top, does a Zombie appear)
    const CASES: &[(&str, &str, bool)] = &[
        ("Walking Corpse", "Forest", true),
        // Reversed: the creature card is on TOP, where this ability must not
        // reach. Milling the top would take the Corpse and make a Zombie.
        ("Forest", "Walking Corpse", false),
    ];

    for &(bottom_name, top_name, expect_zombie) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let door = named_permanent(&mut state, &reg, "Cellar Door", P0);

        let put_in_library = |state: &mut mtg_engine::state::GameState, name: &str| {
            let card_id = reg.get_id_by_name(name).unwrap();
            let data = reg.card_data(card_id).unwrap();
            let id = state.create_object(card_id, P1, Zone::Library, data.power, data.toughness);
            state.get_object_mut(id).unwrap().name = name.into();
            id
        };
        let top = put_in_library(&mut state, top_name);
        let bottom = put_in_library(&mut state, bottom_name);
        state.get_player_mut(P1).library_order = vec![top, bottom];

        state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);
        activate_via_hooks(&mut state, &reg, door, 0, &[mtg_engine::actions::Target::Player(P1)]);
        mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

        assert_eq!(state.get_object(bottom).unwrap().zone, Zone::Graveyard,
            "the bottom card ({bottom_name}) is the one that goes");
        assert_eq!(state.get_object(top).unwrap().zone, Zone::Library,
            "and the top card ({top_name}) stays where it is");
        assert_eq!(count_tokens_named(&state, "Zombie Token"), usize::from(expect_zombie),
            "milled {bottom_name}: 'if it's a creature card' is {expect_zombie}");
    }
}

/// "**you** create a 2/2 black Zombie creature token" — you being the ability's
/// controller, not the player whose library was milled. And the token is the
/// thing the text describes, not merely something named Zombie.
#[test]
fn cellar_doors_zombie_is_a_two_two_black_zombie_for_the_activating_player() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let door = named_permanent(&mut state, &reg, "Cellar Door", P0);
    let card_id = reg.get_id_by_name("Walking Corpse").unwrap();
    let corpse = state.create_object(card_id, P1, Zone::Library, Some(2), Some(2));
    state.get_object_mut(corpse).unwrap().name = "Walking Corpse".into();
    state.get_player_mut(P1).library_order = vec![corpse];

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);
    activate_via_hooks(&mut state, &reg, door, 0, &[mtg_engine::actions::Target::Player(P1)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(count_tokens_named_by(&state, "Zombie Token", P0), 1,
        "the token belongs to whoever activated the ability");
    assert_eq!(count_tokens_named_by(&state, "Zombie Token", P1), 0,
        "not to the player whose library was milled");

    let token = find_token_named(&state, "Zombie Token").unwrap();
    assert_eq!(state.effective_power(token, &reg), Some(2));
    assert_eq!(state.effective_toughness(token, &reg), Some(2));
    assert!(state.colors_of(token, &reg).contains(&Color::Black), "black");
    assert!(state.has_subtype(token, "Zombie", &reg), "a Zombie");
}

/// An empty library has no bottom card: nothing is milled, so there is no
/// creature card and no Zombie.
#[test]
fn cellar_door_does_nothing_to_an_empty_library() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let door = named_permanent(&mut state, &reg, "Cellar Door", P0);
    state.get_player_mut(P1).library_order.clear();

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);
    activate_via_hooks(&mut state, &reg, door, 0, &[mtg_engine::actions::Target::Player(P1)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(count_tokens_named(&state, "Zombie Token"), 0,
        "nothing was milled, so nothing was a creature card");
    assert!(state.awaiting_action.is_none(),
        "and the ability finished rather than stalling");
}

// ── Skaab Ruinator ───────────────────────────────────────────────

#[test]
fn skaab_ruinator_exiles_creatures_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 3 creature cards in graveyard for the additional cost.
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard, &reg);
    }

    let spell = castable_spell(&mut state, &reg, "Skaab Ruinator", P0);
    let new_state = cast_and_resolve(&state, &reg, spell, vec![]);

    // Skaab Ruinator should be on the battlefield.
    let on_bf = new_state.objects.values()
        .any(|o| o.zone == Zone::Battlefield && o.name == "Skaab Ruinator");
    assert!(on_bf, "Skaab Ruinator should be on the battlefield");

    // 3 creatures should be exiled.
    let exiled = new_state.objects.values()
        .filter(|o| o.zone == Zone::Exile && o.owner == P0)
        .count();
    assert_eq!(exiled, 3, "Should exile 3 creatures from graveyard");
}

/// "Skaab Ruinator is on the stack when you pay its costs. It can't be exiled
/// to pay for itself." Casting it from your graveyard with only two *other*
/// creature cards there is not a legal cast — the Ruinator does not count
/// towards the three it has to exile.
#[test]
fn skaab_ruinator_cannot_be_exiled_to_pay_for_itself() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    let ruinator = named_card_in_graveyard(&mut state, &reg, "Skaab Ruinator", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Two other creature cards in the graveyard: three creature cards in the
    // zone, but only two the cost can reach.
    for _ in 0..2 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard, &reg);
    }
    assert!(!can_cast(&state, &reg, ruinator),
        "two other creature cards is not three — it cannot exile itself");

    // A third makes it castable.
    let c = ready_creature(&mut state, P0, 1, 1);
    state.move_object(c, Zone::Graveyard, &reg);
    assert!(can_cast(&state, &reg, ruinator),
        "three other creature cards pays the cost");
}

#[test]
fn skaab_ruinator_cast_from_graveyard() {
    // "You may cast this card from your graveyard" — uses normal mana cost, not flashback.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    // Put Skaab Ruinator in graveyard.
    let ruinator = named_card_in_graveyard(&mut state, &reg, "Skaab Ruinator", P0);

    // Put 3 creature cards in graveyard for the additional cost.
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard, &reg);
    }

    // Give enough mana ({1}{U}{U}).
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Should be castable from graveyard.
    let can_cast = can_cast(&state, &reg, ruinator);
    assert!(can_cast, "Skaab Ruinator should be castable from graveyard");

    // Cast it — the engine sets up a ChooseExileFromGraveyard prompt
    // (exile 3 creatures) and leaves the spell in the graveyard until
    // the cost is paid. Use the test helper to pick the max-power
    // subset (all three 1/1s).
    let mut new_state = cast_onto_stack(&state, &reg, ruinator, vec![]);
    new_state = resolve_exile_choice_max_power(&new_state, &reg);

    // Should be on the stack (not panicked!).
    assert_eq!(new_state.get_object(ruinator).unwrap().zone, Zone::Stack,
        "Skaab Ruinator should be on the stack after casting from graveyard");

    // Should NOT have cast_with_flashback set (it's not flashback).
    assert!(!new_state.get_object(ruinator).unwrap().cast_with_flashback,
        "Should not be marked as flashback — it's cast-from-graveyard");
}

#[test]
fn skaab_ruinator_not_castable_without_enough_creatures() {
    // Can't cast from graveyard without 3 creature cards.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    let ruinator = named_card_in_graveyard(&mut state, &reg, "Skaab Ruinator", P0);

    // Only 2 creatures (need 3).
    for _ in 0..2 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard, &reg);
    }

    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let can_cast = can_cast(&state, &reg, ruinator);
    assert!(!can_cast, "Should NOT be castable with only 2 creature cards in graveyard");
}

// ── Manor Gargoyle ───────────────────────────────────────────────

#[test]
fn manor_gargoyle_loses_defender_and_gains_flying() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gargoyle = named_permanent(&mut state, &reg, "Manor Gargoyle", P0);

    // Should start with Defender.
    assert!(state.has_keyword(gargoyle, Keyword::Defender, &reg),
        "Manor Gargoyle should start with Defender");

    // Activate ability.
    activate_via_hooks(&mut state, &reg, gargoyle, 0, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Should have lost Defender. Asked through the accessor: `obj.keywords`
    // holds only runtime grants and is empty for every registry card, so
    // reading it directly would report "no Defender" whatever happened.
    assert!(!state.has_keyword(gargoyle, Keyword::Defender, &reg),
        "Manor Gargoyle should lose Defender after activation");

    // Should have gained Flying. Asked through the accessor for the same
    // reason as Defender above: the `until_end_of_turn` entry existing and the
    // engine honouring it are two different claims, and only the second is
    // what the card promises.
    assert!(state.has_keyword(gargoyle, Keyword::Flying, &reg),
        "Manor Gargoyle should gain Flying until end of turn");
}

/// "This creature has indestructible **as long as** it has defender." The two
/// halves are tested separately elsewhere — that it has indestructible while
/// it has defender, and that activating removes defender — but never chained,
/// and the chain is the card. Losing defender has to cost it indestructible in
/// the same breath.
#[test]
fn manor_gargoyle_loses_indestructible_with_its_defender() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gargoyle = named_permanent(&mut state, &reg, "Manor Gargoyle", P0);
    assert!(state.has_keyword(gargoyle, Keyword::Indestructible, &reg),
        "test precondition: indestructible while it has defender");

    activate_via_hooks(&mut state, &reg, gargoyle, 0, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(!state.has_keyword(gargoyle, Keyword::Indestructible, &reg),
        "the condition is 'as long as it has defender', and it no longer does");

    advance_to_next_turn(&mut state, &reg);
    assert!(state.has_keyword(gargoyle, Keyword::Defender, &reg),
        "the loss was until end of turn");
    assert!(state.has_keyword(gargoyle, Keyword::Indestructible, &reg),
        "so the indestructible comes back with it");
}

/// The card's one ruling: "Lethal damage dealt to Manor Gargoyle while it has
/// indestructible will stay marked on it that turn. If Manor Gargoyle loses
/// indestructible after having been dealt lethal damage earlier in the turn,
/// it will be destroyed."
///
/// CR 120.3 marks the damage whatever the creature's indestructibility; CR
/// 704.5g only declines to destroy it. So the {1} ability is a way to kill
/// your own Gargoyle, and that is the trap worth pinning.
#[test]
fn manor_gargoyle_dies_to_damage_marked_while_it_was_indestructible() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gargoyle = named_permanent(&mut state, &reg, "Manor Gargoyle", P0);
    state.get_object_mut(gargoyle).unwrap().damage_marked = 4;
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(gargoyle).unwrap().zone, Zone::Battlefield,
        "lethal damage does not destroy an indestructible creature (CR 704.5g)");
    assert_eq!(state.get_object(gargoyle).unwrap().damage_marked, 4,
        "but the damage stays marked on it (CR 120.3) — surviving is not \
         healing");

    activate_via_hooks(&mut state, &reg, gargoyle, 0, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(gargoyle).unwrap().zone, Zone::Graveyard,
        "with defender gone the indestructible went too, and the damage that \
         was already marked is now lethal");
}

// ── Tree of Redemption ───────────────────────────────────────────

#[test]
fn tree_of_redemption_swaps_life_and_toughness() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let tree = named_permanent(&mut state, &reg, "Tree of Redemption", P0);
    // P0 starts at 20 life, Tree base toughness is 13.

    activate_via_hooks(&mut state, &reg, tree, 0, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Life should now be 13 (Tree's toughness).
    assert_eq!(state.get_player(P0).life, 13, "Life should become Tree's toughness (13)");

    // Tree's base toughness should now be 20 (old life).
    assert_eq!(state.get_object(tree).unwrap().toughness, Some(20),
        "Tree's toughness should become old life total (20)");
}

/// Ruling (2018-03-16): "Any toughness-modifying effects, counters, Auras, or
/// Equipment will apply after its toughness is set to your former life total.
/// For example, say Tree of Redemption is enchanted with Lunarch Mantle (which
/// makes it 2/15) and your life total is 7. After the exchange, Tree of
/// Redemption would be a 2/9 creature (its toughness became 7, which was then
/// modified by Lunarch Mantle) and your life total would be 15."
///
/// So the exchange reads the **effective** toughness — the modifier counts
/// toward the life you gain — and writes the **base**, which the modifier then
/// applies on top of. Reading the base instead passed the whole workspace.
///
/// Lunarch Mantle is not in this pool, so the modifier here is two +1/+1
/// counters, which the ruling names in the same breath. The numbers are the
/// ruling's own: 2/15 with 7 life becomes 2/9 with 15 life.
#[test]
fn tree_of_redemption_exchanges_the_toughness_it_actually_has() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let tree = named_permanent(&mut state, &reg, "Tree of Redemption", P0);
    state.add_counters(tree, CounterType::PlusOnePlusOne, 2);
    state.get_player_mut(P0).life = 7;

    assert_eq!(state.effective_power(tree, &reg), Some(2), "test setup: a 2/15");
    assert_eq!(state.effective_toughness(tree, &reg), Some(15));

    activate_via_hooks(&mut state, &reg, tree, 0, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_player(P0).life, 15,
        "you gain up to the toughness it actually had, counters included");
    assert_eq!(state.get_object(tree).unwrap().toughness, Some(7),
        "its BASE toughness becomes your former life total");
    assert_eq!(state.effective_toughness(tree, &reg), Some(9),
        "and the counters apply on top of that, so it is a 2/9");
    assert_eq!(state.effective_power(tree, &reg), Some(2),
        "power is untouched by the exchange");
}

// ── Back from the Brink ──────────────────────────────────────────

#[test]
fn back_from_the_brink_creates_token_copy() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let enchant = named_permanent(&mut state, &reg, "Back from the Brink", P0);

    // Put a creature in graveyard.
    let dead = named_card_in_graveyard(&mut state, &reg, "Kalonian Tusker", P0);


    // The ability_index encodes the creature's ObjectId.
    let ability_index = usize::try_from(dead.0).unwrap();
    activate_via_hooks(&mut state, &reg, enchant, ability_index, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // The creature should be exiled.
    assert_eq!(state.get_object(dead).unwrap().zone, Zone::Exile,
        "Original creature should be exiled");

    // A token copy should be on the battlefield.
    assert_eq!(count_tokens_named(&state, "Kalonian Tusker"), 1,
        "Should have created a token copy");
}

#[test]
fn back_from_the_brink_ability_per_creature_in_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let enchant = named_permanent(&mut state, &reg, "Back from the Brink", P0);

    // Put two different creatures in the graveyard.
    let _tusker = named_card_in_graveyard(&mut state, &reg, "Kalonian Tusker", P0);
    let _piker = named_card_in_graveyard(&mut state, &reg, "Goblin Piker", P0);

    let behavior = reg.get(state.get_object(enchant).unwrap().card_id).unwrap();
    let abilities = behavior.activated_abilities(&state, enchant, &reg);

    // Should have one ability per creature in the graveyard.
    assert_eq!(abilities.len(), 2, "Should have one ability per creature in graveyard");

    // Each ability should reference a different creature and have its mana cost.
    let tusker_ability = abilities.iter().find(|a| a.description.contains("Kalonian Tusker"));
    let piker_ability = abilities.iter().find(|a| a.description.contains("Goblin Piker"));
    assert!(tusker_ability.is_some(), "Should have ability for Kalonian Tusker");
    assert!(piker_ability.is_some(), "Should have ability for Goblin Piker");

    // Kalonian Tusker costs {G}{G} — 2 colored symbols.
    let tusker_cost = &tusker_ability.unwrap().cost;
    assert_eq!(tusker_cost.symbols.len(), 2, "Kalonian Tusker costs {{G}}{{G}}");

    // Goblin Piker costs {1}{R} — 2 symbols.
    let piker_cost = &piker_ability.unwrap().cost;
    assert_eq!(piker_cost.symbols.len(), 2, "Goblin Piker costs {{1}}{{R}}");
}

/// Ruling 2011-09-22: "If you exile a double-faced creature card this way,
/// you'll pay the mana cost of the front face. The token will be a copy of the
/// front face and it won't be able to transform."
#[test]
fn back_from_the_brink_copies_a_dfcs_front_face_only() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let enchant = named_permanent(&mut state, &reg, "Back from the Brink", P0);
    let villagers = named_card_in_graveyard(&mut state, &reg, "Villagers of Estwald", P0);

    let behavior = reg.get(state.get_object(enchant).unwrap().card_id).unwrap();
    let abilities = behavior.activated_abilities(&state, enchant, &reg);
    let ability = abilities.iter()
        .find(|a| a.description.contains("Villagers of Estwald"))
        .expect("the DFC is a creature card in the graveyard");
    assert_eq!(ability.cost.symbols.len(), 2,
        "the front face's {{2}}{{G}}, not the back face, which has no mana cost");

    let ability_index = usize::try_from(villagers.0).unwrap();
    activate_via_hooks(&mut state, &reg, enchant, ability_index, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    let token = state.objects.values()
        .find(|o| o.is_token && o.zone == Zone::Battlefield)
        .map(|o| o.id)
        .expect("a token copy was created");
    assert_eq!(state.get_object(token).unwrap().name, "Villagers of Estwald",
        "a copy of the front face");

    // "it won't be able to transform" — CR 111.7, a token copy of a DFC has
    // only the copied face.
    mtg_engine::cards::helpers::apply_transform(&mut state, token, &reg);
    assert!(!state.get_object(token).unwrap().is_transformed,
        "the token has only the face it was copied from");
    assert_eq!(state.get_object(token).unwrap().name, "Villagers of Estwald");
}

/// Ruling 2011-09-22: "Any 'enters' abilities of the creature will trigger when
/// the token enters."
#[test]
fn back_from_the_brinks_token_brings_its_enters_trigger() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let enchant = named_permanent(&mut state, &reg, "Back from the Brink", P0);
    // Ghoulraiser: "When this creature enters, return a Zombie card at random
    // from your graveyard to your hand." Something for it to find, too.
    let raiser = named_card_in_graveyard(&mut state, &reg, "Ghoulraiser", P0);
    let corpse = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

    let ability_index = usize::try_from(raiser.0).unwrap();
    activate_via_hooks(&mut state, &reg, enchant, ability_index, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(corpse).unwrap().zone, Zone::Hand,
        "the token's enters trigger fired and returned the Zombie card");
}

/// Ruling 2011-09-22: "Although you're paying the card's mana cost, you aren't
/// casting that card. Abilities that reduce the cost to cast a creature spell
/// won't apply... Alternative costs that affect what it costs to cast a
/// creature spell... can't."
///
/// Rooftop Storm is "You may pay {0} rather than pay the mana cost for Zombie
/// creature spells you cast" — an alternative cost for *casting*, so it has
/// nothing to say about this ability's cost.
#[test]
fn back_from_the_brink_ignores_an_alternative_cost_for_casting() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let enchant = named_permanent(&mut state, &reg, "Back from the Brink", P0);
    named_permanent(&mut state, &reg, "Rooftop Storm", P0);
    // Walking Corpse is a {1}{B} Zombie.
    let _corpse = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

    let behavior = reg.get(state.get_object(enchant).unwrap().card_id).unwrap();
    let abilities = behavior.activated_abilities(&state, enchant, &reg);
    let ability = abilities.iter()
        .find(|a| a.description.contains("Walking Corpse"))
        .expect("the Zombie is a creature card in the graveyard");

    assert_eq!(ability.cost.mana_value(), 2,
        "still {{1}}{{B}}: you are not casting it, so Rooftop Storm's {{0}} \
         does not apply (cost was {:?})", ability.cost);
}

/// CR 109.1: a token is not a card, and it sits in a graveyard until the next
/// state-based-action check — so it must never be offered as something to
/// exile.
#[test]
fn back_from_the_brink_does_not_offer_a_token_in_the_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let enchant = named_permanent(&mut state, &reg, "Back from the Brink", P0);
    let token = state.create_token(
        "Zombie", P0, 2, 2, vec![], vec![CardType::Creature], vec![], &reg)[0];
    state.move_object(token, Zone::Graveyard, &reg);
    assert!(state.get_object(token).is_some(), "test premise: it is still there");

    let behavior = reg.get(state.get_object(enchant).unwrap().card_id).unwrap();
    assert!(behavior.activated_abilities(&state, enchant, &reg).is_empty(),
        "a token in the graveyard is not a creature card");
}

#[test]
fn back_from_the_brink_no_abilities_without_creatures_in_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let enchant = named_permanent(&mut state, &reg, "Back from the Brink", P0);

    let behavior = reg.get(state.get_object(enchant).unwrap().card_id).unwrap();
    let abilities = behavior.activated_abilities(&state, enchant, &reg);

    assert_eq!(abilities.len(), 0, "No abilities when graveyard has no creatures");
}

#[test]
fn back_from_the_brink_uses_creature_mana_cost() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let enchant = named_permanent(&mut state, &reg, "Back from the Brink", P0);

    // Savannah Lions costs {W}.
    let lions = named_card_in_graveyard(&mut state, &reg, "Savannah Lions", P0);

    let behavior = reg.get(state.get_object(enchant).unwrap().card_id).unwrap();
    let abilities = behavior.activated_abilities(&state, enchant, &reg);
    assert_eq!(abilities.len(), 1);

    let ability = &abilities[0];
    // Savannah Lions costs {W} — 1 white mana symbol.
    assert_eq!(ability.cost.symbols.len(), 1, "Savannah Lions costs {{W}}");
    assert!(ability.sorcery_speed_only, "Activate only as a sorcery");

    // Activate the ability — use the creature's ObjectId as the ability index.
    let ability_index = usize::try_from(lions.0).unwrap();
    activate_via_hooks(&mut state, &reg, enchant, ability_index, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(lions).unwrap().zone, Zone::Exile,
        "Lions should be exiled");
    assert_eq!(count_tokens_named(&state, "Savannah Lions"), 1,
        "Should have created a token copy of Savannah Lions");
}

// ── Grimgrin, Corpse-Born ──────────────────────────────────────────

/// "Grimgrin enters tapped" is a replacement effect (CR 614.1c), so it is
/// tapped *as* it arrives — not moved onto the battlefield untapped and tapped
/// afterwards, which is what an `on_resolve` override used to do. `move_object`
/// emits `EnteredBattlefield` as part of the move, so under the old shape every
/// ETB watcher saw an untapped Grimgrin.
#[test]
fn grimgrin_enters_tapped() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grimgrin, Corpse-Born").unwrap();
    let id = state.create_object(card_id, P0, Zone::Stack, Some(5), Some(5));
    state.get_object_mut(id).unwrap().name = "Grimgrin, Corpse-Born".into();

    assert!(plan_entering(&mut state, &reg, id, Some(Zone::Stack)).tapped,
        "the replacement effect taps it as part of the entry event");

    state.move_object(id, Zone::Battlefield, &reg);
    assert!(state.get_object(id).unwrap().tapped);
    assert_eq!(state.get_object(id).unwrap().zone, Zone::Battlefield);
}

/// "Grimgrin enters tapped **and doesn't untap during your untap step**." The
/// second half is the whole reason the sacrifice ability exists — without it
/// Grimgrin would simply untap for free every turn — and it had no test.
///
/// Another tapped creature untaps in the same step, so this shows the untap
/// step really ran rather than being skipped.
#[test]
fn grimgrin_does_not_untap_during_his_controllers_untap_step() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let other = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(grimgrin).unwrap().tapped = true;
    state.get_object_mut(other).unwrap().tapped = true;

    // Round the table back to P0's untap step.
    advance_to_next_turn(&mut state, &reg);
    advance_to_next_turn(&mut state, &reg);
    assert_eq!(state.active_player, P0, "back to Grimgrin's controller's turn");

    assert!(!state.get_object(other).unwrap().tapped,
        "an ordinary creature untapped, so the untap step ran");
    assert!(state.get_object(grimgrin).unwrap().tapped,
        "Grimgrin does not untap during his controller's untap step");
}

#[test]
fn grimgrin_sacrifice_untaps_and_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    state.get_object_mut(grimgrin).unwrap().tapped = true;

    let zombie = ready_creature(&mut state, P0, 2, 2);

    // Activate through the engine, sacrificing the zombie (not Grimgrin itself).
    let new_state = activate_sacrificing(&state, &reg, grimgrin, 0, vec![], zombie);

    // Grimgrin should be untapped.
    assert!(!new_state.get_object(grimgrin).unwrap().tapped);
    // Grimgrin should have a +1/+1 counter.
    assert_eq!(new_state.get_counter_count(grimgrin, CounterType::PlusOnePlusOne), 1);
    // Zombie should be dead (sacrificed as cost).
    assert_eq!(new_state.get_object(zombie).unwrap().zone, Zone::Graveyard);
}

#[test]
fn grimgrin_sacrifice_not_available_without_other_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    state.get_object_mut(grimgrin).unwrap().tapped = true;

    // No other creatures — sacrifice ability should NOT be available.
    let legal = engine::legal_actions(&state, &reg);
    let has_activate = legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, ability_index: 0, .. } if *object_id == grimgrin
    ));
    assert!(!has_activate, "Grimgrin sacrifice ability should not be available without another creature");
}

#[test]
fn grimgrin_attack_trigger_destroys_and_adds_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let defender_creature = ready_creature(&mut state, P1, 3, 3);

    // Set up combat state with Grimgrin attacking P1.
    attacks_unblocked(&mut state, grimgrin, P1);

    // Fire the AttackersDeclared event and run the trigger pipeline.
    state.events.push(mtg_engine::events::GameEvent::AttackersDeclared {
        attackers: vec![(grimgrin, P1)],
    });
    triggers::process_triggers(&mut state, &reg);

    // With only one defender creature, the target is auto-chosen and the
    // trigger goes directly on the stack and resolves.
    assert_eq!(state.get_object(defender_creature).unwrap().zone, Zone::Graveyard,
        "Defending creature should be destroyed");
    assert_eq!(state.get_counter_count(grimgrin, CounterType::PlusOnePlusOne), 1,
        "Grimgrin should have a +1/+1 counter from attack trigger");
}

#[test]
fn grimgrin_attack_trigger_presents_choice_with_multiple_targets() {
    use mtg_engine::actions::Target;

    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let creature_a = ready_creature(&mut state, P1, 2, 2);
    let creature_b = ready_creature(&mut state, P1, 3, 3);

    // Set up combat state with Grimgrin attacking P1.
    attacks_unblocked(&mut state, grimgrin, P1);

    // Fire the AttackersDeclared event and collect triggers (which enters
    // the stack-time target-choice prompt because there are two valid defenders).
    state.events.push(mtg_engine::events::GameEvent::AttackersDeclared {
        attackers: vec![(grimgrin, P1)],
    });
    triggers::collect_triggers(&mut state, &reg);

    // With multiple targets, the controller should be presented a choice.
    assert!(state.awaiting_action.is_some(),
        "Should present target choice when defender has multiple creatures");

    // Resolve the choice — pick creature_a.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(Target::Object(creature_a))),
        },
        &reg,
    );
    // Resolve the trigger on the stack.
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(creature_a).unwrap().zone, Zone::Graveyard,
        "Chosen creature should be destroyed");
    assert_eq!(state.get_object(creature_b).unwrap().zone, Zone::Battlefield,
        "Unchosen creature should remain");
    assert_eq!(state.get_counter_count(grimgrin, CounterType::PlusOnePlusOne), 1,
        "Grimgrin should have a +1/+1 counter");
}

// Ruling: "If the defending player controls no creatures when Grimgrin attacks,
// the last ability will be removed from the stack and have no effect."
#[test]
fn grimgrin_attack_no_targets_no_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    // Defender has NO creatures.

    attacks_unblocked(&mut state, grimgrin, P1);

    state.events.push(mtg_engine::events::GameEvent::AttackersDeclared {
        attackers: vec![(grimgrin, P1)],
    });
    triggers::process_triggers(&mut state, &reg);

    // No valid targets — trigger removed from stack (CR 603.3c), no +1/+1 counter.
    assert_eq!(state.get_counter_count(grimgrin, CounterType::PlusOnePlusOne), 0,
        "Grimgrin should NOT get a +1/+1 counter when defender has no creatures");
}

// Ruling: "If Grimgrin's last ability resolves, but the targeted creature isn't destroyed
// (perhaps because it regenerated or has indestructible), you'll still put a +1/+1 on Grimgrin."
#[test]
fn grimgrin_attack_indestructible_target_still_gets_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let indestructible = ready_creature(&mut state, P1, 4, 4);
    // Make the creature indestructible by adding the keyword.
    if let Some(obj) = state.get_object_mut(indestructible) {
        obj.keywords.push(Keyword::Indestructible);
    }

    attacks_unblocked(&mut state, grimgrin, P1);

    state.events.push(mtg_engine::events::GameEvent::AttackersDeclared {
        attackers: vec![(grimgrin, P1)],
    });
    triggers::process_triggers(&mut state, &reg);

    // The indestructible creature should still be on the battlefield.
    assert_eq!(state.get_object(indestructible).unwrap().zone, Zone::Battlefield,
        "Indestructible creature should survive destruction");
    // But Grimgrin still gets the +1/+1 counter.
    assert_eq!(state.get_counter_count(grimgrin, CounterType::PlusOnePlusOne), 1,
        "Grimgrin should still get +1/+1 counter even if target survives");
}

// Ruling: "If the targeted creature is an illegal target by the time Grimgrin's last ability
// resolves, the entire ability doesn't resolve and none of its effects will occur."
// This test verifies the attack trigger uses the defending player from combat state.
#[test]
fn grimgrin_attack_uses_defending_player_from_combat() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    // Defender has a creature.
    let defender_creature = ready_creature(&mut state, P1, 2, 2);
    // Controller also has another creature (should NOT be targetable).
    let own_creature = ready_creature(&mut state, P0, 3, 3);

    attacks_unblocked(&mut state, grimgrin, P1);

    state.events.push(mtg_engine::events::GameEvent::AttackersDeclared {
        attackers: vec![(grimgrin, P1)],
    });
    triggers::process_triggers(&mut state, &reg);

    // With only one defender creature (own_creature is filtered out by
    // is_valid_target), the target auto-resolves. The defender's creature
    // should be destroyed, not the controller's own creature.
    assert_eq!(state.get_object(defender_creature).unwrap().zone, Zone::Graveyard,
        "Defending player's creature should be targeted");
    assert_eq!(state.get_object(own_creature).unwrap().zone, Zone::Battlefield,
        "Controller's own creature should not be targeted");
}

// ── Evil Twin ──────────────────────────────────────────

#[test]
fn evil_twin_copies_creature_on_etb() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let opponent_creature = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);

    let behavior = reg.get(state.get_object(twin).unwrap().card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, twin, &[], &reg);

    // ETB now presents an optional choice instead of auto-copying.
    assert!(state.awaiting_action.is_some(), "Should present a copy choice");

    // Resolve the choice by selecting the opponent's creature.
    let target = mtg_engine::actions::Target::Object(opponent_creature);
    let effect = mtg_engine::state::PendingEffect::CopyCreature { source_id: twin };
    state.awaiting_action = None;
    mtg_engine::engine::apply_pending_effect(&mut state, &target, &effect, &reg);

    // Evil Twin should have copied Grizzly Bears stats.
    assert_eq!(state.get_object(twin).unwrap().name, "Grizzly Bears");
    assert_eq!(state.get_object(twin).unwrap().power, Some(2));
    assert_eq!(state.get_object(twin).unwrap().toughness, Some(2));
    // Should still have the Evil Twin marker.
    assert!(state.get_object(twin).unwrap().copy_grantor.is_some(),
        "a permanent that entered as a copy records the card whose copy \
         effect made it, so the granted ability can be found (CR 706.2)");
}

/// A copy of a legendary creature is itself legendary (CR 707.2), so an Evil
/// Twin copying your own legend triggers the legend rule.
#[test]
fn evil_twin_copying_legendary_triggers_legend_rule() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 controls a legendary creature with fixed P/T.
    let original = named_permanent(&mut state, &reg, "Geist of Saint Traft", P0);
    assert!(state.get_object(original).unwrap().is_legendary);

    // Evil Twin (also P0) enters and copies it.
    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);
    let behavior = reg.get(state.get_object(twin).unwrap().card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, twin, &[], &reg);
    state.awaiting_action = None;
    engine::apply_pending_effect(
        &mut state,
        &Target::Object(original),
        &mtg_engine::state::PendingEffect::CopyCreature { source_id: twin },
        &reg,
    );

    // The copy is legendary and shares the original's name.
    assert!(state.get_object(twin).unwrap().is_legendary,
        "a copy of a legendary creature is itself legendary (CR 707.2)");
    assert_eq!(state.get_object(twin).unwrap().name, "Geist of Saint Traft");

    // Legend rule: P0 now controls two same-named legendaries — SBA must
    // require choosing which to keep.
    check_state_based_actions(&mut state, &reg);
    assert!(state.awaiting_action.is_some(),
        "legend rule should force P0 to choose which legendary to keep");
}

// ── Moldgraf Monstrosity ──────────────────────────────────────────

#[test]
fn moldgraf_monstrosity_returns_creatures_on_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let monstrosity = named_permanent(&mut state, &reg, "Moldgraf Monstrosity", P0);

    // Put two creatures in P0's graveyard.
    let gy1 = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(gy1).unwrap().name = "Creature 1".into();
    state.move_object(gy1, Zone::Graveyard, &reg);
    let gy2 = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(gy2).unwrap().name = "Creature 2".into();
    state.move_object(gy2, Zone::Graveyard, &reg);
    // A non-creature card in the same graveyard. "two CREATURE cards" was never
    // a claim this test could fail while everything there was a creature:
    // returning any card at all passed the whole workspace.
    let gy_noncreature = named_card_in_graveyard(&mut state, &reg, "Doom Blade", P0);

    // Die for real — the trigger resolves with the card already in the
    // graveyard, which is the only place "exile it" can apply.
    mtg_engine::destruction::try_destroy(&mut state, monstrosity, &reg);
    assert_eq!(state.get_object(monstrosity).unwrap().zone, Zone::Graveyard,
        "test precondition: the Monstrosity died");

    let behavior = reg.get(state.get_object(monstrosity).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, monstrosity, &[], &reg);

    // Monstrosity should be exiled.
    assert_eq!(state.get_object(monstrosity).unwrap().zone, Zone::Exile);
    // Both graveyard creatures should be on the battlefield.
    assert_eq!(state.get_object(gy1).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(gy2).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(gy_noncreature).unwrap().zone, Zone::Graveyard,
        "'two creature cards' — the instant stays where it is, and it is not \
         one of the two that came back");
}

// ── Liliana of the Veil ──────────────────────────────────────────

#[test]
fn liliana_enters_with_loyalty() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let id = state.create_object(card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(id).unwrap().name = "Liliana of the Veil".into();

    let behavior = reg.get(card_id).unwrap();
    behavior.on_resolve(&mut state, id, &[], &reg);

    assert_eq!(state.get_object(id).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_counter_count(id, CounterType::Loyalty), 3);
}

#[test]
fn liliana_plus_one_each_player_discards_with_choice() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
    use mtg_engine::actions::ResolvedChoice;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 3);
    let p0_card_a = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    let p0_card_b = spell_in_hand(&mut state, &reg, "Bump in the Night", P0);
    let p1_card_a = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    let p1_card_b = spell_in_hand(&mut state, &reg, "Bump in the Night", P1);

    // Activate +1.
    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 0, &[], &reg);

    // Active player (P0) should be asked to choose which card to discard.
    assert!(state.awaiting_action.is_some(), "Expected awaiting_action for P0 discard choice");
    if let Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseCardFromHand { cards, .. }, .. }) = &state.awaiting_action {
        assert_eq!(*player, P0, "Active player should choose first");
        assert_eq!(cards.len(), 2, "P0 has 2 cards to choose from");
    } else {
        panic!("Expected ChooseCardFromHand for P0");
    }

    // P0 chooses to discard card A.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenCard(p0_card_a),
    }, &reg);

    // CR 101.4: the choice is recorded, but nothing has left a hand yet —
    // every player chooses first and the cards are discarded together.
    assert_eq!(state.get_object(p0_card_a).unwrap().zone, Zone::Hand,
        "P0's card must stay in hand until P1 has also chosen");
    assert_eq!(state.get_object(p0_card_b).unwrap().zone, Zone::Hand);

    // Now P1 should be asked to choose.
    assert!(state.awaiting_action.is_some(), "Expected awaiting_action for P1 discard choice");
    if let Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseCardFromHand { cards, .. }, .. }) = &state.awaiting_action {
        assert_eq!(*player, P1, "Other player should choose second");
        assert_eq!(cards.len(), 2, "P1 has 2 cards to choose from");
    } else {
        panic!("Expected ChooseCardFromHand for P1");
    }

    // P1 chooses to discard card B.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenCard(p1_card_b),
    }, &reg);

    // Now that the last player has chosen, both discards happen at once.
    assert_eq!(state.get_object(p0_card_a).unwrap().zone, Zone::Graveyard,
        "P0's chosen card is discarded when the last player has chosen");
    assert_eq!(state.get_object(p1_card_b).unwrap().zone, Zone::Graveyard);
    // Each player's other card should still be in hand.
    assert_eq!(state.get_object(p0_card_b).unwrap().zone, Zone::Hand);
    assert_eq!(state.get_object(p1_card_a).unwrap().zone, Zone::Hand);
}

#[test]
fn liliana_plus_one_single_card_auto_discards() {
    // When a player has exactly 1 card, it should auto-discard (no choice needed).
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 3);
    let p0_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    let p1_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 0, &[], &reg);

    // Both players had 1 card each, so both auto-discard. No awaiting_action.
    assert!(state.awaiting_action.is_none(), "Should auto-discard when only 1 card");
    assert_eq!(state.get_object(p0_card).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(p1_card).unwrap().zone, Zone::Graveyard);
}

#[test]
fn liliana_plus_one_empty_hand_skipped() {
    // Ruling: "You can activate Liliana's first ability even if some or all players
    // will be unable to discard a card."
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 3);
    let p1_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 0, &[], &reg);

    // P0 is skipped (no cards), P1 auto-discards (1 card).
    assert!(state.awaiting_action.is_none(), "P1 auto-discards");
    assert_eq!(state.get_object(p1_card).unwrap().zone, Zone::Graveyard);
}

#[test]
fn liliana_minus_two_target_player_sacrifices_creature() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
    use mtg_engine::actions::ResolvedChoice;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 3);

    let creature_a = ready_creature(&mut state, P1, 3, 3);
    let creature_b = ready_creature(&mut state, P1, 2, 2);

    // Activate -2 targeting P1.
    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 1, &[Target::Player(P1)], &reg);

    // P1 should be asked which creature to sacrifice.
    assert!(state.awaiting_action.is_some(), "Expected sacrifice choice for P1");
    if let Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseTarget { options, .. }, .. }) = &state.awaiting_action {
        assert_eq!(*player, P1, "Target player chooses which creature to sacrifice");
        assert_eq!(options.len(), 2, "P1 has 2 creatures to choose from");
    } else {
        panic!("Expected ChooseTarget for P1");
    }

    // P1 chooses creature_a to sacrifice.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenTarget(Some(Target::Object(creature_a))),
    }, &reg);

    assert_eq!(state.get_object(creature_a).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(creature_b).unwrap().zone, Zone::Battlefield);
}

#[test]
fn liliana_minus_two_single_creature_auto_sacrifices() {
    // With only one creature, it's auto-sacrificed (no choice needed).
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 3);

    let creature = ready_creature(&mut state, P1, 3, 3);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 1, &[Target::Player(P1)], &reg);

    // Only one creature, auto-sacrificed.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Each of Liliana's three abilities has a "nothing to work with" case, and
/// all three used to be their own test asserting only `awaiting_action.is_none()`.
/// A test whose one assertion is "no prompt appeared" passes with the ability's
/// implementation deleted, so each row here is run twice — once with nothing to
/// act on, once with something — and the second half is what makes the first
/// mean anything.
#[test]
fn each_liliana_ability_with_nothing_to_act_on_asks_nothing_and_does_nothing() {
    let reg = registry();
    // (ability index, targets, what the ability would need)
    let cases: [(usize, &[Target], &str); 3] = [
        (0, &[], "a card in someone's hand to discard"),
        (1, &[Target::Player(P1)], "a creature the target player controls"),
        (2, &[Target::Player(P1)], "a permanent the target player controls"),
    ];

    for (index, targets, needs) in cases {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
        set_loyalty(&mut state, liliana, 9);
        let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
        behavior.on_loyalty_ability(&mut state, liliana, index, targets, &reg);

        assert!(state.awaiting_action.is_none(),
            "ability {index} with no {needs} must not stop for a choice");

        // The control: give it something, and the same call does ask.
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
        set_loyalty(&mut state, liliana, 9);
        if index == 0 {
            // Two cards each, so neither player's discard is auto-picked.
            for p in [P0, P1] {
                spell_in_hand(&mut state, &reg, "Grizzly Bears", p);
                spell_in_hand(&mut state, &reg, "Lightning Bolt", p);
            }
        } else {
            ready_creature(&mut state, P1, 2, 2);
            ready_creature(&mut state, P1, 3, 3);
        }
        let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
        behavior.on_loyalty_ability(&mut state, liliana, index, targets, &reg);

        assert!(state.awaiting_action.is_some(),
            "control: ability {index} does ask once there is {needs}");
    }
}

#[test]
fn liliana_minus_two_can_target_self() {
    // "Target player" means you can target yourself, not just opponent.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 3);

    let own_creature = ready_creature(&mut state, P0, 2, 2);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 1, &[Target::Player(P0)], &reg);

    // P0 targeted themselves, their creature is auto-sacrificed.
    assert_eq!(state.get_object(own_creature).unwrap().zone, Zone::Graveyard);
}

#[test]
fn liliana_minus_six_pile_division_and_choice() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
    use mtg_engine::actions::ResolvedChoice;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 9);
    let c1 = ready_creature(&mut state, P1, 3, 3);
    let c2 = ready_creature(&mut state, P1, 2, 2);
    let c3 = ready_creature(&mut state, P1, 1, 1);

    // Activate -6 targeting P1.
    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 2, &[Target::Player(P1)], &reg);

    // Step 1: Liliana's controller (P0) should divide permanents into two piles.
    assert!(state.awaiting_action.is_some(), "Expected pile division choice");
    if let Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::DividePermanentsIntoPiles { permanents, target_player, .. }, .. }) = &state.awaiting_action {
        assert_eq!(*player, P0, "Controller divides the piles");
        assert_eq!(*target_player, P1, "Target player is P1");
        assert_eq!(permanents.len(), 3, "Three permanents to divide");
    } else {
        panic!("Expected DividePermanentsIntoPiles");
    }

    // P0 puts c1 in pile 1, c2 and c3 in pile 2.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenSubset(vec![c1]),
    }, &reg);

    // Step 2: P1 should choose which pile to sacrifice.
    assert!(state.awaiting_action.is_some(), "Expected pile choice for P1");
    if let Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChoosePile { pile_1, pile_2, .. }, .. }) = &state.awaiting_action {
        assert_eq!(*player, P1, "Target player chooses which pile to sacrifice");
        assert_eq!(pile_1.len(), 1, "Pile 1 has 1 permanent");
        assert_eq!(pile_2.len(), 2, "Pile 2 has 2 permanents");
    } else {
        panic!("Expected ChoosePile for P1");
    }

    // P1 chooses pile 1 (sacrifices c1 only).
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenIndex(0, "Option 0".into()),
    }, &reg);

    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Graveyard, "c1 should be sacrificed");
    assert_eq!(state.get_object(c2).unwrap().zone, Zone::Battlefield, "c2 should survive");
    assert_eq!(state.get_object(c3).unwrap().zone, Zone::Battlefield, "c3 should survive");
}

#[test]
fn liliana_minus_six_empty_pile_allowed() {
    // Ruling: "A pile can be empty. If the player chooses an empty pile, no permanents will be sacrificed."
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 9);

    let c1 = ready_creature(&mut state, P1, 3, 3);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 2, &[Target::Player(P1)], &reg);

    // P0 puts all permanents in pile 1 (pile 2 is empty).
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenSubset(vec![c1]),
    }, &reg);

    // P1 chooses the empty pile (pile 2, index 1) — nothing sacrificed.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenIndex(1, "Option 1".into()),
    }, &reg);

    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Battlefield, "Chose empty pile, nothing sacrificed");
}

#[test]
fn liliana_minus_six_all_in_one_pile() {
    // Controller can put all permanents in one pile. If target player chooses that pile,
    // all permanents are sacrificed.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 9);

    let c1 = ready_creature(&mut state, P1, 3, 3);
    let c2 = ready_creature(&mut state, P1, 2, 2);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 2, &[Target::Player(P1)], &reg);

    // P0 puts everything in pile 1 (empty pile 2).
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenSubset(vec![c1, c2]),
    }, &reg);

    // P1 chooses pile 1 — all sacrificed.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenIndex(0, "Option 0".into()),
    }, &reg);

    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(c2).unwrap().zone, Zone::Graveyard);
}

#[test]
fn liliana_minus_six_can_target_self() {
    // -6 says "target player", so controller can target themselves.
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 9);
    let _c1 = ready_creature(&mut state, P0, 2, 2);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 2, &[Target::Player(P0)], &reg);

    // P0 is both controller and target. Division choice should still go to P0.
    assert!(state.awaiting_action.is_some(), "Expected pile division");
    if let Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::DividePermanentsIntoPiles { permanents, target_player, .. }, .. }) = &state.awaiting_action {
        assert_eq!(*player, P0, "Controller is P0");
        assert_eq!(*target_player, P0, "Target is also P0");
        // P0 has Liliana and the creature on the battlefield.
        assert!(permanents.len() >= 2, "At least Liliana + creature");
    } else {
        panic!("Expected DividePermanentsIntoPiles");
    }
}

// ── Essence of the Wild ──────────────────────────────────────────

#[test]
fn essence_overrides_entering_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Essence of the Wild on the battlefield via cast_and_resolve,
    // which calls on_resolve and sets entering_copy_source.
    let essence = castable_spell(&mut state, &reg, "Essence of the Wild", P0);
    state = cast_and_resolve(&state, &reg, essence, vec![]);

    // Verify Essence itself is on the battlefield.
    assert_eq!(state.get_object(essence).unwrap().zone, Zone::Battlefield);

    // Now cast a creature — it should enter as a 6/6 copy of Essence
    // via the replacement effect (before ETB triggers fire).
    let bear = castable_spell(&mut state, &reg, "Grizzly Bears", P0);
    state = cast_and_resolve(&state, &reg, bear, vec![]);

    // Bear should now be a 6/6 Essence copy (replacement effect, not trigger).
    assert_eq!(state.get_object(bear).unwrap().power, Some(6));
    assert_eq!(state.get_object(bear).unwrap().toughness, Some(6));
    assert_eq!(state.get_object(bear).unwrap().name, "Essence of the Wild");
    assert_eq!(state.get_object(bear).unwrap().subtypes, vec!["Avatar".to_string()]);
}

/// Issue #93 / CR 706.2: a creature that entered as an Essence of the Wild
/// copy has Essence's abilities and ONLY those — its own printed activated
/// ability must not be offered. (Triggered abilities were already dropped
/// correctly; the activated-ability collector consulted `copy_grantor`
/// unconditionally and handed the printed card's abilities back.)
#[test]
fn an_essence_copy_does_not_keep_its_printed_activated_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let essence = castable_spell(&mut state, &reg, "Essence of the Wild", P0);
    let mut state = cast_and_resolve(&state, &reg, essence, vec![]);

    let ranger = castable_spell(&mut state, &reg, "Daybreak Ranger", P0);
    let mut state = cast_and_resolve(&state, &reg, ranger, vec![]);
    assert_eq!(state.get_object(ranger).unwrap().name, "Essence of the Wild",
        "test precondition: the Ranger entered as an Essence copy");

    // A flying target and no summoning sickness, so the printed "{T}: deal
    // 2 damage to target creature with flying" WOULD be offered if the copy
    // still had it.
    state.get_object_mut(ranger).unwrap().summoning_sick = false;
    let flyer = ready_creature(&mut state, P1, 2, 2);
    grant_keyword(&mut state, flyer, Keyword::Flying);

    let offered = engine::legal_actions(&state, &reg).actions.iter().any(|a| matches!(
        a, Action::ActivateAbility { object_id, .. } if *object_id == ranger));
    assert!(!offered,
        "a permanent that entered as an Essence copy has only Essence's \
         abilities (CR 706.2) — Daybreak Ranger's printed ping must be gone");
}

#[test]
fn essence_does_not_override_opponent_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Essence on the battlefield for P0.
    let essence = castable_spell(&mut state, &reg, "Essence of the Wild", P0);
    state = cast_and_resolve(&state, &reg, essence, vec![]);

    // Opponent's creature enters via move_object — should NOT be affected
    // because Essence only applies to its controller's creatures.
    let opp_bear = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    state.move_object(opp_bear, Zone::Battlefield, &reg);

    // Opponent's creature should be unchanged.
    assert_eq!(state.get_object(opp_bear).unwrap().power, Some(2));
    assert_eq!(state.get_object(opp_bear).unwrap().toughness, Some(2));
    assert_eq!(state.get_object(opp_bear).unwrap().name, "Grizzly Bears");
}

/// Ruling: "a creature that would normally enter tapped will enter as an
/// untapped Essence of the Wild" — because the copy effect is applied before
/// the other effects that modify how it enters, and once it is an Essence it
/// no longer has the ability that would have tapped it.
///
/// Grimgrin, Corpse-Born is the set's creature that enters tapped.
#[test]
fn a_creature_that_would_enter_tapped_enters_as_an_untapped_essence() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let essence = castable_spell(&mut state, &reg, "Essence of the Wild", P0);
    let mut state = cast_and_resolve(&state, &reg, essence, vec![]);

    let grimgrin = castable_spell(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let state = cast_and_resolve(&state, &reg, grimgrin, vec![]);

    assert_eq!(state.get_object(grimgrin).unwrap().name, "Essence of the Wild",
        "test premise: it entered as a copy");
    assert!(!state.get_object(grimgrin).unwrap().tapped,
        "\"enters tapped\" is Grimgrin's ability, and it is not Grimgrin as it enters");
}

/// Ruling: "Because creatures you control enter as copies of Essence of the
/// Wild, any 'enters' triggered abilities printed on such creatures won't
/// trigger."
///
/// Village Bell-Ringer is "When this creature enters, untap all creatures you
/// control", and the tapped creature beside it is what shows the trigger did
/// not happen — the Bell-Ringer arrives as an Essence, which has no such
/// ability.
#[test]
fn an_enters_trigger_does_not_fire_for_a_creature_that_arrived_as_an_essence() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let essence = castable_spell(&mut state, &reg, "Essence of the Wild", P0);
    let mut state = cast_and_resolve(&state, &reg, essence, vec![]);

    let tapped = ready_creature(&mut state, P0, 2, 2);
    state.tap(tapped);

    let ringer = castable_spell(&mut state, &reg, "Village Bell-Ringer", P0);
    let mut state = cast_and_resolve(&state, &reg, ringer, vec![]);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(ringer).unwrap().name, "Essence of the Wild",
        "test premise: it entered as a copy");
    assert!(state.get_object(tapped).unwrap().tapped,
        "the Bell-Ringer's enters trigger is not on the thing that entered");
}

// ── Mirror-Mad Phantasm ──────────────────────────────────────────

/// Mirror-Mad Phantasm: "{1}{U}: its owner shuffles it into their library, then
/// reveals cards until a card named Mirror-Mad Phantasm is revealed, puts that
/// card onto the battlefield and all other cards revealed this way into their
/// graveyard."
///
/// The shuffle makes the mill count random, so assert what is true of every
/// shuffle: the Phantasm comes back to the battlefield, everything revealed
/// above it is in the graveyard, everything below it is still in the library,
/// and no card ends up anywhere else.
#[test]
fn mirror_mad_phantasm_mills_to_find_itself() {
    let reg = registry();

    // Twenty shuffles, one per seed, because a single shuffle could put the
    // Phantasm on top and mill nothing at all. Naming the seeds is what makes
    // "at least one of these milled something" a fact about the card rather
    // than a coin toss the test happens to win.
    let mut saw_a_mill = false;
    for seed in 0..20u64 {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        state.rng_state = seed;
        let phantasm = named_permanent(&mut state, &reg, "Mirror-Mad Phantasm", P0);

        let library: Vec<_> = ["Grizzly Bears", "Lightning Bolt", "Doom Blade", "Divination"]
            .iter()
            .map(|n| {
                let c = spell_in_hand(&mut state, &reg, n, P0);
                state.move_object(c, Zone::Library, &reg);
                c
            })
            .collect();
        state.players[0].library_order = library.clone();

        let behavior = reg.get(state.get_object(phantasm).unwrap().card_id).unwrap();
        behavior.resolve_activated_ability(&mut state, phantasm, 0, &[], &reg);

        assert_eq!(state.get_object(phantasm).unwrap().zone, Zone::Battlefield,
            "the Phantasm is always found — it was shuffled into the library it is milling");

        let still_in_library = &state.players[0].library_order;
        for card in &library {
            let zone = state.get_object(*card).unwrap().zone;
            if still_in_library.contains(card) {
                assert_eq!(zone, Zone::Library,
                    "a card left below the Phantasm stays in the library");
            } else {
                assert_eq!(zone, Zone::Graveyard,
                    "a card revealed above the Phantasm goes to the graveyard, not {zone:?}");
                saw_a_mill = true;
            }
        }
        assert!(!still_in_library.contains(&phantasm),
            "the Phantasm left the library for the battlefield");
    }
    assert!(saw_a_mill, "none of the 20 seeded shuffles put a card above the Phantasm");
}

/// Scryfall ruling (2011-09-22): "You can only activate the ability if you
/// control Mirror-Mad Phantasm, even if you don't own it."
///
/// The ability belongs to whoever controls the Phantasm; everything it does
/// belongs to the owner. So when an opponent steals it and activates it, it is
/// the *owner's* library that gets shuffled and revealed, the owner's cards
/// that are milled, and the owner who gets the Phantasm back — the thief
/// spends the mana and hands it over.
#[test]
fn mirror_mad_phantasm_digs_through_its_owners_library_not_the_activators() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 owns the Phantasm; P1 takes control of it.
    let phantasm = named_permanent(&mut state, &reg, "Mirror-Mad Phantasm", P0);
    state.change_control(phantasm, P1);
    assert_eq!(state.get_object(phantasm).unwrap().owner, P0);
    assert_eq!(state.get_object(phantasm).unwrap().controller, P1);

    // Both players have a library, so "the owner's" is a real choice between
    // two and not the only one available.
    let mine: Vec<ObjectId> = ["Grizzly Bears", "Lightning Bolt", "Doom Blade"]
        .iter().map(|n| {
            let c = spell_in_hand(&mut state, &reg, n, P0);
            state.move_object(c, Zone::Library, &reg);
            c
        }).collect();
    state.players[0].library_order = mine.clone();
    let theirs: Vec<ObjectId> = ["Grizzly Bears", "Divination"]
        .iter().map(|n| {
            let c = spell_in_hand(&mut state, &reg, n, P1);
            state.move_object(c, Zone::Library, &reg);
            c
        }).collect();
    state.players[1].library_order = theirs.clone();

    let behavior = reg.get(state.get_object(phantasm).unwrap().card_id).unwrap();
    behavior.resolve_activated_ability(&mut state, phantasm, 0, &[], &reg);

    // The thief's library is untouched.
    assert_eq!(state.players[1].library_order, theirs,
        "P1 activated it, but it is not P1's library that gets dug through");
    for card in &theirs {
        assert_eq!(state.get_object(*card).unwrap().zone, Zone::Library);
    }

    // The owner's library is where the Phantasm went and came back from.
    assert_eq!(state.get_object(phantasm).unwrap().zone, Zone::Battlefield,
        "it finds itself in its owner's library");
    assert_eq!(state.get_object(phantasm).unwrap().controller, P0,
        "and the *owner* puts it onto the battlefield, so the thief does not \
         keep it");
    let left_behind = state.players[0].library_order.len();
    let milled = mine.iter()
        .filter(|c| state.get_object(**c).unwrap().zone == Zone::Graveyard)
        .count();
    assert_eq!(left_behind + milled, mine.len(),
        "every one of the owner's cards is either still in their library or in \
         their graveyard; {left_behind} + {milled} != {}", mine.len());
}

// ── Grimoire of the Dead ──────────────────────────────────────────

/// "{1}, {T}, Discard a card:" — the discard is *cost*, everything before the
/// colon, so it is paid on activation (CR 601.2h via 602.2b) and an opponent
/// responding to the ability already sees the card in the graveyard.
///
/// It used to happen in `resolve_activated_ability`, on the far side of the
/// priority window: responding to the ability found the card still in hand,
/// and countering the ability would have taken the discard back with it.
#[test]
fn grimoire_discards_when_the_ability_is_activated_not_when_it_resolves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grimoire = named_permanent(&mut state, &reg, "Grimoire of the Dead", P0);
    let card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Activate WITHOUT resolving: the ability is on the stack.
    let state = engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: grimoire, ability_index: 0, targets: vec![],
            tap_plan: vec![], sacrifice: None, x_value: None, source_card_id: None,
        },
        &reg,
    );

    assert!(matches!(state.stack.last(), Some(mtg_engine::state::StackEntry::Ability { .. })),
        "test precondition: the ability is on the stack, unresolved");
    assert_eq!(state.get_object(card).unwrap().zone, Zone::Graveyard,
        "the discard is a cost — it is already paid while the ability waits");
    assert_eq!(state.get_counter_count(grimoire, CounterType::Study), 0,
        "and the counter is the effect, so it is not there yet");

    let mut state = state;
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert_eq!(state.get_counter_count(grimoire, CounterType::Study), 1,
        "the counter arrives when the ability resolves");
}

#[test]
fn grimoire_discard_presents_choice_and_adds_study_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grimoire of the Dead").unwrap();
    let grimoire = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(grimoire).unwrap().name = "Grimoire of the Dead".into();

    // Give P0 multiple cards in hand so a choice is presented.
    let c1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    let c2 = spell_in_hand(&mut state, &reg, "Giant Growth", P0);

    // Add {1} mana for the ability cost.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Activate ability 0 via the engine.
    state = activate(&state, &reg, grimoire, 0, vec![]);

    // Should be awaiting a discard choice.
    assert!(state.awaiting_action.is_some(), "Should be awaiting discard choice");

    // Choose to discard c1.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenCard(c1) },
        &reg,
    );

    // c1 should be in graveyard.
    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Graveyard);
    // c2 should still be in hand.
    assert_eq!(state.get_object(c2).unwrap().zone, Zone::Hand);
    // Study counter should be added via the proper counter system.
    assert_eq!(state.get_counter_count(grimoire, CounterType::Study), 1);
}

#[test]
fn grimoire_single_card_in_hand_auto_discards() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grimoire of the Dead").unwrap();
    let grimoire = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(grimoire).unwrap().name = "Grimoire of the Dead".into();

    // Give P0 only one card in hand.
    let c1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);

    // Add {1} mana for the ability cost.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Activate ability 0 via the engine.
    state = activate(&state, &reg, grimoire, 0, vec![]);

    // With only one card, discard should be automatic (no choice needed).
    assert!(state.awaiting_action.is_none(), "Should not be awaiting a choice with one card");
    // c1 should be discarded.
    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Graveyard);
    // Study counter should be added.
    assert_eq!(state.get_counter_count(grimoire, CounterType::Study), 1);
}

#[test]
fn grimoire_accumulates_three_study_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grimoire of the Dead").unwrap();
    let grimoire = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(grimoire).unwrap().name = "Grimoire of the Dead".into();

    // Give P0 three cards to discard (one at a time).
    let c1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    let c2 = spell_in_hand(&mut state, &reg, "Giant Growth", P0);
    let c3 = spell_in_hand(&mut state, &reg, "Lightning Bolt", P0);

    // Activate 3 times, discarding one card each time.
    // Each time: add mana, submit activate, choose a card.
    for (i, card_to_discard) in [c1, c2, c3].iter().enumerate() {
        state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
        // Untap Grimoire for subsequent activations.
        if i > 0 {
            state.get_object_mut(grimoire).unwrap().tapped = false;
        }

        state = activate(&state, &reg, grimoire, 0, vec![]);

        // For the last card in hand, auto-discard happens.
        if state.awaiting_action.is_some() {
            state = engine::submit_action(
                &state,
                &Action::ResolveChoice { choice: ResolvedChoice::ChosenCard(*card_to_discard) },
                &reg,
            );
        }
    }

    // Should have 3 study counters.
    assert_eq!(state.get_counter_count(grimoire, CounterType::Study), 3);
}

#[test]
fn grimoire_reanimates_all_graveyard_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grimoire of the Dead").unwrap();
    let grimoire = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(grimoire).unwrap().name = "Grimoire of the Dead".into();
    // Add 3 study counters via the proper counter system.
    state.add_counters(grimoire, CounterType::Study, 3);

    // Put creatures in both graveyards.
    let gy1 = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(gy1).unwrap().name = "Creature A".into();
    state.move_object(gy1, Zone::Graveyard, &reg);

    let gy2 = ready_creature(&mut state, P1, 4, 4);
    state.get_object_mut(gy2).unwrap().name = "Creature B".into();
    state.move_object(gy2, Zone::Graveyard, &reg);

    // Activate ability 1 via the engine (tap + sacrifice + remove counters).
    state = activate(&state, &reg, grimoire, 1, vec![]);

    // Both creatures should be on the battlefield under P0's control.
    assert_eq!(state.get_object(gy1).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(gy1).unwrap().controller, P0);
    assert_eq!(state.get_object(gy2).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(gy2).unwrap().controller, P0);
    // They should have the Zombie subtype and black color.
    assert!(state.get_object(gy1).unwrap().subtypes.contains(&"Zombie".into()));
    assert!(state.get_object(gy2).unwrap().subtypes.contains(&"Zombie".into()));
    assert!(state.get_object(gy1).unwrap().colors.contains(&Color::Black));
    assert!(state.get_object(gy2).unwrap().colors.contains(&Color::Black));
    // Grimoire should be sacrificed (in graveyard).
    assert_eq!(state.get_object(grimoire).unwrap().zone, Zone::Graveyard);
}

#[test]
fn grimoire_ability_1_not_available_without_3_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grimoire of the Dead").unwrap();
    let grimoire = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(grimoire).unwrap().name = "Grimoire of the Dead".into();
    // Only 2 study counters -- not enough.
    state.add_counters(grimoire, CounterType::Study, 2);

    let legal = engine::legal_actions(&state, &reg);
    let has_sacrifice_ability = legal.actions.iter().any(|a| {
        matches!(a, Action::ActivateAbility { ability_index: 1, .. })
    });
    assert!(!has_sacrifice_ability, "Should not be able to activate ability 1 with only 2 study counters");
}

// ── Civilized Scholar ──────────────────────────────────────────

// -------------------------------------------------------------------------
// Creepy Doll
// -------------------------------------------------------------------------

/// The trigger should fire when `CombatDamageDealt` event targets a creature.
#[test]
fn trigger_fires_on_combat_damage_to_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let doll = named_permanent(&mut state, &reg, "Creepy Doll", P0);
    let target = ready_creature(&mut state, P1, 3, 3);
    attacks_blocked_by(&mut state, doll, P1, &[target]);

    // Emit combat damage event (doll deals 1 damage to target creature).
    state.events.push(GameEvent::CombatDamageDealt {
        source: doll,
        target: DamageTarget::Object(target),
        amount: 1,
    });

    // Collect triggers.
    mtg_engine::triggers::collect_triggers(&mut state, &reg);

    // Should have a trigger on the stack for Creepy Doll's ability.
    let has_trigger = state.stack.iter().any(|entry| matches!(entry, StackEntry::Trigger(_)));
    assert!(has_trigger,
        "Should have a trigger on the stack for Creepy Doll's combat damage to creature");
}

/// The trigger should NOT fire when `CombatDamageDealt` targets a player.
#[test]
fn trigger_does_not_fire_on_combat_damage_to_player() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let doll = named_permanent(&mut state, &reg, "Creepy Doll", P0);
    attacks_unblocked(&mut state, doll, P1);

    // Emit combat damage event (doll deals 1 damage to player).
    state.events.push(GameEvent::CombatDamageDealt {
        source: doll,
        target: DamageTarget::Player(P1),
        amount: 1,
    });

    // Collect triggers.
    mtg_engine::triggers::collect_triggers(&mut state, &reg);

    // Should NOT have a trigger on the stack (Creepy Doll doesn't have CombatDamageToPlayer).
    let has_trigger = state.stack.iter().any(|entry| matches!(entry, StackEntry::Trigger(_)));
    assert!(!has_trigger,
        "Should NOT trigger on combat damage to player");
}

/// "Flip a coin. If you win the flip, destroy that creature." Both outcomes,
/// named rather than sampled: the game's randomness lives on `GameState`, so
/// a test says which way the coin went.
///
/// This used to run the hook fifty times in a loop and assert that at least
/// one run destroyed something — a claim about the coin, not about the card.
#[test]
fn creepy_doll_destroys_the_creature_when_it_wins_the_flip() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let doll = named_permanent(&mut state, &reg, "Creepy Doll", P0);
    let target = ready_creature(&mut state, P1, 3, 3);
    attacks_blocked_by(&mut state, doll, P1, &[target]);

    state.events.push(GameEvent::CombatDamageDealt {
        source: doll,
        target: DamageTarget::Object(target),
        amount: 1,
    });
    rig_next_coin_flip(&mut state, true);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(target).unwrap().zone, Zone::Graveyard,
        "a 3/3 survives one damage; the coin is what killed it");
}

#[test]
fn creepy_doll_destroys_nothing_when_it_loses_the_flip() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let doll = named_permanent(&mut state, &reg, "Creepy Doll", P0);
    let target = ready_creature(&mut state, P1, 3, 3);
    attacks_blocked_by(&mut state, doll, P1, &[target]);

    state.events.push(GameEvent::CombatDamageDealt {
        source: doll,
        target: DamageTarget::Object(target),
        amount: 1,
    });
    rig_next_coin_flip(&mut state, false);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(target).unwrap().zone, Zone::Battlefield);
}

/// CR 113.7a: the ability is on the stack and no longer the Doll's problem.
/// Indestructible does not make the Doll unsacrificeable — Grimgrin eats one
/// at instant speed — and the ability resolves regardless.
#[test]
fn creepy_dolls_flip_happens_even_if_the_doll_is_gone() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let doll = named_permanent(&mut state, &reg, "Creepy Doll", P0);
    let target = ready_creature(&mut state, P1, 3, 3);
    attacks_blocked_by(&mut state, doll, P1, &[target]);

    state.events.push(GameEvent::CombatDamageDealt {
        source: doll,
        target: DamageTarget::Object(target),
        amount: 1,
    });
    mtg_engine::triggers::collect_triggers(&mut state, &reg);
    // Sacrificed in response, with its trigger already on the stack.
    mtg_engine::destruction::sacrifice(&mut state, doll, &reg);
    rig_next_coin_flip(&mut state, true);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(target).unwrap().zone, Zone::Graveyard,
        "the ability exists independently of its source");
}

/// Ruling: "If the combat damage Creepy Doll deals to a creature is lethal,
/// you'll still flip a coin. If the creature is still on the battlefield
/// (perhaps because it regenerated), it could be destroyed a second time."
///
/// So the flip is not skipped for a creature that is already dying, and the
/// destroy it produces is a *second* destruction — one a second regeneration
/// shield would have to answer separately.
#[test]
fn creepy_doll_can_destroy_a_creature_that_regenerated_from_its_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let doll = named_permanent(&mut state, &reg, "Creepy Doll", P0);
    let target = ready_creature(&mut state, P1, 2, 1);
    attacks_blocked_by(&mut state, doll, P1, &[target]);

    // It took the Doll's damage and regenerated from it: one shield spent,
    // damage cleared, still on the battlefield.
    state.get_object_mut(target).unwrap().regeneration_shields = 1;
    mtg_engine::destruction::try_destroy(&mut state, target, &reg);
    assert_eq!(state.get_object(target).unwrap().zone, Zone::Battlefield,
        "test setup: it regenerated");
    assert_eq!(state.get_object(target).unwrap().regeneration_shields, 0);

    state.events.push(GameEvent::CombatDamageDealt {
        source: doll,
        target: DamageTarget::Object(target),
        amount: 1,
    });
    rig_next_coin_flip(&mut state, true);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(target).unwrap().zone, Zone::Graveyard,
        "the trigger's destroy is a second one, and there is no shield left");
}

/// Indestructible answers the trigger's destroy like any other (CR 702.12b),
/// and the log has to say so rather than announcing a kill that did not happen.
#[test]
fn creepy_doll_cannot_destroy_an_indestructible_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let doll = named_permanent(&mut state, &reg, "Creepy Doll", P0);
    let other_doll = named_permanent(&mut state, &reg, "Creepy Doll", P1);
    attacks_blocked_by(&mut state, doll, P1, &[other_doll]);

    state.events.push(GameEvent::CombatDamageDealt {
        source: doll,
        target: DamageTarget::Object(other_doll),
        amount: 1,
    });
    rig_next_coin_flip(&mut state, true);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(other_doll).unwrap().zone, Zone::Battlefield);
    assert!(state.game_log.iter().any(|e| e.message.contains("could not destroy")),
        "the log says what happened, not what was attempted");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------


// -------------------------------------------------------------------------
// Gutter Grime — the rest
// -------------------------------------------------------------------------

/// A slime counter and an Ooze per nontoken creature death, and the Oozes are
/// sized by the *current* counter count — so an Ooze made when the count was 1
/// is a 2/2 once the count reaches 2. That is what "power and toughness are
/// each equal to the number of slime counters on Gutter Grime" means: a
/// characteristic-defining ability, recomputed, not a size fixed at creation.
#[test]
fn every_ooze_is_sized_by_the_current_slime_count() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);

    for expected in 1..=2 {
        let creature = ready_creature(&mut state, P0, 2, 2);
        kill_by_damage(&mut state, &reg, creature);
        triggers::process_triggers(&mut state, &reg);

        assert_eq!(counters_of(&state, grime, CounterType::Slime), expected,
            "one slime counter per nontoken creature death");
        assert_eq!(count_tokens_named(&state, "Ooze Token"), expected as usize,
            "and one Ooze per death");

        // Every Ooze, including the ones made earlier, is the current size.
        // CR 111.4 names a token after its subtypes plus "Token", so these are
        // "Ooze Token" — a filter on "Ooze" matches nothing and asserts
        // nothing.
        let oozes: Vec<_> = state.objects.values()
            .filter(|o| o.is_token && o.zone == Zone::Battlefield && o.name == "Ooze Token")
            .map(|o| o.id)
            .collect();
        assert_eq!(oozes.len(), expected as usize);
        for ooze in oozes {
            assert_eq!(state.effective_power(ooze, &reg), Some(expected as i32),
                "with {expected} slime counter(s), every Ooze is {expected}/{expected}");
            assert_eq!(state.effective_toughness(ooze, &reg), Some(expected as i32));
        }
    }
}

/// "Whenever a **nontoken** creature **you control** dies" — two conditions,
/// and both need a row, since a Gutter Grime that ignored the condition
/// entirely would satisfy either one alone.
#[test]
fn gutter_grime_counts_only_your_own_nontoken_creatures() {
    // (whose creature, is it a token, does the slime counter arrive?)
    const CASES: &[(PlayerId, bool, bool)] = &[
        (P0, false, true),
        (P0, true, false),
        (P1, false, false),
    ];

    for &(controller, is_token, counts) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);

        let creature = if is_token {
            let id = state.create_token("Spirit", controller, 1, 1,
                vec![Color::White], vec![CardType::Creature], vec![], &reg)[0];
            state.get_object_mut(id).unwrap().summoning_sick = false;
            id
        } else {
            ready_creature(&mut state, controller, 2, 2)
        };

        // Killed for real, so a dying token is removed from `state.objects` by
        // SBA 704.5d before the trigger resolves — the case where reading
        // `is_token` back off the dead object answers `false` and the "nontoken"
        // clause silently stops applying.
        kill_by_damage(&mut state, &reg, creature);
        triggers::process_triggers(&mut state, &reg);

        assert_eq!(counters_of(&state, grime, CounterType::Slime), u32::from(counts),
            "controller=p{}, is_token={is_token}", controller.0);
        assert_eq!(count_tokens_named(&state, "Ooze Token"), usize::from(counts),
            "controller=p{}, is_token={is_token}", controller.0);
    }
}

/// Ruling: "If you control more than one Gutter Grime, each Ooze token
/// remembers which one created it. The power and toughness of that Ooze will
/// be equal to the number of slime counters on that Gutter Grime only."
///
/// The two Grimes have to be at different counts for the claim to bite, so the
/// second one arrives a death late.
#[test]
fn each_ooze_counts_the_slime_on_the_gutter_grime_that_made_it() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let kill_one = |state: &mut mtg_engine::state::GameState| {
        let creature = ready_creature(state, P0, 2, 2);
        kill_by_damage(state, &reg, creature);
        // With both Grimes out, one death raises two distinguishable
        // triggers, so CR 603.3b asks their order before anything reaches the
        // stack. Immaterial here: each Grime acts only on itself.
        triggers::collect_triggers(state, &reg);
        order_triggers_front_first(state, &reg);
        triggers::process_triggers(state, &reg);
    };
    let oozes_of = |state: &mtg_engine::state::GameState, grime: ObjectId| -> Vec<ObjectId> {
        let mut ids: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.is_token && o.zone == Zone::Battlefield
                && o.card_state.get(mtg_engine::cards::PT_DEFINED_BY) == Some(&grime))
            .map(|o| o.id)
            .collect();
        ids.sort_unstable();
        ids
    };

    let first = named_permanent(&mut state, &reg, "Gutter Grime", P0);
    kill_one(&mut state);
    let second = named_permanent(&mut state, &reg, "Gutter Grime", P0);
    kill_one(&mut state);

    assert_eq!(counters_of(&state, first, CounterType::Slime), 2);
    assert_eq!(counters_of(&state, second, CounterType::Slime), 1,
        "the second Grime missed the first death");
    assert_eq!(oozes_of(&state, first).len(), 2);
    assert_eq!(oozes_of(&state, second).len(), 1);

    for ooze in oozes_of(&state, first) {
        assert_eq!(state.effective_power(ooze, &reg), Some(2),
            "an Ooze from the first Grime counts its two slime counters");
        assert_eq!(state.effective_toughness(ooze, &reg), Some(2));
    }
    for ooze in oozes_of(&state, second) {
        assert_eq!(state.effective_power(ooze, &reg), Some(1),
            "and one from the second counts only that Grime's one");
        assert_eq!(state.effective_toughness(ooze, &reg), Some(1));
    }

    // Losing one Grime is felt only by its own Oozes.
    let seconds_oozes = oozes_of(&state, second);
    let firsts_oozes = oozes_of(&state, first);
    state.move_object(first, Zone::Graveyard, &reg);
    for ooze in firsts_oozes {
        assert_eq!(state.effective_power(ooze, &reg), Some(0),
            "its own Oozes lose their counters with it");
    }
    for ooze in seconds_oozes {
        assert_eq!(state.effective_power(ooze, &reg), Some(1),
            "the other Grime's Ooze is untouched");
    }
}

/// The Oozes' size is read off Gutter Grime, so losing Gutter Grime makes them
/// 0/0 — and state-based actions then bury them (CR 704.5a).
#[test]
fn the_oozes_die_when_gutter_grime_leaves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    kill_by_damage(&mut state, &reg, creature);
    triggers::process_triggers(&mut state, &reg);

    let ooze = find_token_named(&state, "Ooze Token").expect("an Ooze was made");
    assert_eq!(state.effective_power(ooze, &reg), Some(1), "test precondition: a 1/1");

    state.move_object(grime, Zone::Graveyard, &reg);

    assert_eq!(state.effective_power(ooze, &reg), Some(0),
        "with no Gutter Grime there are no slime counters to count");
    assert_eq!(state.effective_toughness(ooze, &reg), Some(0));

    mtg_engine::sba::check_state_based_actions(&mut state, &reg);
    assert!(state.get_object(ooze).is_none(),
        "a 0-toughness token dies and, being a token, ceases to exist");
}

// -------------------------------------------------------------------------
// Unbreathing Horde
// -------------------------------------------------------------------------

/// Combat damage is prevented and a counter is removed.
#[test]
fn prevents_combat_damage_removes_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let horde = named_permanent(&mut state, &reg, "Unbreathing Horde", P0);
    // Give it 3 +1/+1 counters.
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);

    // Attacker attacks, Horde blocks.
    let attacker = ready_creature(&mut state, P1, 2, 2);
    attacks_blocked_by(&mut state, attacker, P0, &[horde]);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // The Horde should have taken no damage but lost a counter.
    assert_eq!(state.get_object(horde).unwrap().damage_marked, 0,
        "Damage should be prevented");
    let counters = state.get_object(horde).unwrap().counters
        .get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    assert_eq!(counters, 2, "Should have lost one +1/+1 counter");
}

/// When Unbreathing Horde deals damage as attacker, the other creature still takes damage.
#[test]
fn still_deals_damage_to_others() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let horde = named_permanent(&mut state, &reg, "Unbreathing Horde", P0);
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);

    let blocker = ready_creature(&mut state, P1, 2, 5);
    // Horde attacks, blocker blocks.
    attacks_blocked_by(&mut state, horde, P1, &[blocker]);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // The blocker should have taken damage from Horde (0 base + 3 counters = 3 power).
    assert!(state.get_object(blocker).unwrap().damage_marked > 0,
        "Blocker should take damage from Unbreathing Horde");
    // The Horde should have taken no damage (prevented).
    assert_eq!(state.get_object(horde).unwrap().damage_marked, 0,
        "Horde damage should be prevented");
}

/// "a +1/+1 counter for each other Zombie you control and each Zombie card in
/// your graveyard", counted the two ways a permanent can be a Zombie.
///
/// The battlefield half says just "Zombie", so it includes tokens; the
/// graveyard half says "Zombie card", so it must not (CR 109.1). This was two
/// tests fifteen hundred lines apart in this file with the same assertion and
/// nothing saying what made them different.
#[test]
fn enters_with_a_counter_per_zombie_however_the_zombie_is_a_zombie() {
    for (what, as_tokens) in [
        ("tokens, whose subtypes live on the object", true),
        ("cards, whose subtypes live on the registry face", false),
    ] {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        if as_tokens {
            for _ in 0..2 {
                state.create_token_with_subtypes(
                    "Zombie", P0, 2, 2,
                    vec![Color::Black], vec![CardType::Creature], vec![],
                    vec!["Zombie".into()],
                    &reg,
                );
            }
        } else {
            let _z1 = named_permanent(&mut state, &reg, "Walking Corpse", P0);
            let _z2 = named_permanent(&mut state, &reg, "Diregraf Ghoul", P0);
        }

        let _gy_zombie = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

        let horde = castable_spell(&mut state, &reg, "Unbreathing Horde", P0);
        let state = cast_and_resolve(&state, &reg, horde, vec![]);

        assert_eq!(counters_of(&state, horde, CounterType::PlusOnePlusOne), 3,
            "two battlefield Zombies ({what}) plus one Zombie card in the graveyard");
    }
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug AC (`audits/AUDIT_BUGS.md)`: Unbreathing Horde under-counts when
/// reanimated from a graveyard.
///
/// Oracle (Unbreathing Horde): "This creature enters with a +1/+1
/// counter on it for each other Zombie you control and each Zombie
/// card in your graveyard."
///
/// Ruling (2011-09-22): "If Unbreathing Horde enters from a graveyard, it will
/// count itself when determining how many +1/+1 counters it enters with."
///
/// "Enters with X counters" is a CR 614.1c replacement effect, and CR 616.1
/// works it out against the game state as it was BEFORE the event — at which
/// point the Horde is still in the graveyard, and its own count includes it.
///
/// Failure mode: the count ran from an `on_enter_battlefield` hook, i.e. AFTER
/// the move, by which time the Horde is on the battlefield and no longer one of
/// the "Zombie cards in your graveyard", so the reanimated Horde came in a
/// counter short of the cast path. The test called that hook by hand to
/// reproduce it; now that the count lives in `replace_event`, calling it proves
/// nothing — `move_object` applies the entering replacement itself, and that is
/// what this asks about.
#[test]
fn bug_ac_unbreathing_horde_counts_itself_when_reanimated() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Two other Zombie creature cards in P0's graveyard.
    let walking_corpse_id = registry.get_id_by_name("Walking Corpse").unwrap();
    let z1 = state.create_object(walking_corpse_id, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(z1).unwrap().name = "Walking Corpse (a)".into();
    let z2 = state.create_object(walking_corpse_id, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(z2).unwrap().name = "Walking Corpse (b)".into();

    // Unbreathing Horde sitting in P0's graveyard, ready to be reanimated.
    let horde_card_id = registry.get_id_by_name("Unbreathing Horde").unwrap();
    let horde = state.create_object(horde_card_id, P0, Zone::Graveyard, Some(0), Some(0));
    state.get_object_mut(horde).unwrap().name = "Unbreathing Horde".into();

    // Reanimate, the way Unburial Rites does.
    state.move_object(horde, Zone::Battlefield, &registry);

    assert_eq!(
        counters_of(&state, horde, CounterType::PlusOnePlusOne), 3,
        "a reanimated Unbreathing Horde enters with three +1/+1 counters: the \
         two other Zombies in the graveyard, and itself — it is still in that \
         graveyard when CR 616.1 works the entering event out",
    );
}

/// "…put it onto the battlefield attached to **target player**" is a targeted
/// triggered ability, so the player is chosen as the trigger goes on the stack
/// (CR 603.3d) — not part-way through its resolution.
///
/// It used to be asked for at resolution, after the search: the trigger was
/// declared `target_requirement: None` and the card built its own player list.
/// An opponent responding to the trigger could not know whom it would hit, and
/// CR 608.2b never re-checked the choice. It also meant hexproof had to be
/// filtered by hand in the card rather than once, in the engine, for everything
/// that targets a player.
#[test]
fn bitterheart_witch_targets_its_player_when_the_trigger_goes_on_the_stack() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);
    let data = reg.card_data(state.get_object(witch).unwrap().card_id).unwrap();
    let trigger = data.triggered_abilities.first().expect("the death trigger");
    assert!(trigger.target_requirement.is_some(),
        "the trigger must declare its target so the engine chooses it at \
         CR 603.3d time; got {:?}", trigger.target_requirement);

    // With no target handed to it, the trigger does nothing rather than
    // inventing one mid-resolution.
    let curse_card_id = reg.get_id_by_name("Curse of the Pierced Heart").unwrap();
    let curse = state.create_object(curse_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(curse).unwrap().name = "Curse of the Pierced Heart".into();
    state.get_player_mut(P0).library_order.push(curse);

    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, witch, &[], &reg);
    assert!(state.awaiting_action.is_none(),
        "no target, no ability — it must not fall back to asking at resolution");
}

/// CR 701.19b: "If a player is instructed to search a hidden zone for cards
/// with a stated quality ... that player isn't required to find some or all of
/// those cards even if they're present."
///
/// So Bitterheart Witch's controller may search, decline the only Curse in the
/// library, and still have shuffled. The code used to take the Curse for them
/// whenever exactly one was there.
#[test]
fn bitterheart_witch_may_search_and_decline_the_only_curse() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);
    let curse_card_id = reg.get_id_by_name("Curse of the Pierced Heart").unwrap();
    let curse_obj = state.create_object(curse_card_id, P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(curse_obj);

    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, witch, &[Target::Player(P1)], &reg);

    // Yes, search.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );
    assert!(state.awaiting_action.is_some(),
        "the Curse is offered, not taken — even though there is only one");

    // Decline the find.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(None) },
        &reg,
    );

    assert_eq!(state.get_object(curse_obj).unwrap().zone, Zone::Library,
        "declining leaves the Curse in the library");
    assert_eq!(state.get_object(curse_obj).unwrap().attached_to_player, None);
}

// ── Geistcatcher's Rig ───────────────────────────────────────────

/// "When this creature enters, **you may** have it deal **4** damage to target
/// creature **with flying**."
///
/// Ruling (2011-09-22): "The target creature with flying is chosen when the
/// ability triggers and goes on the stack. You choose whether or not
/// Geistcatcher's Rig will deal 4 damage to it when the ability resolves."
///
/// So there are two moments, and three claims across them, none of which had a
/// test: only a flyer may be targeted, the choice is a "may", and the amount is
/// 4. `hexproof_filter.rs` checks that an opponent's hexproof flyer is not
/// offered, which needs none of these to be right.
///
/// `accept` runs the same board both ways.
fn geistcatchers_rig_hits_the_flyer(accept: bool) -> (mtg_engine::state::GameState, ObjectId, ObjectId) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let flyer = named_permanent(&mut state, &reg, "Abbey Griffin", P1);
    assert!(state.has_keyword(flyer, Keyword::Flying, &reg), "test precondition");
    let grounded = ready_creature(&mut state, P1, 3, 3);

    let rig = named_permanent(&mut state, &reg, "Geistcatcher's Rig", P0);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: rig, controller: P0,
    });
    mtg_engine::triggers::collect_triggers(&mut state, &reg);

    // CR 603.3d: the target is locked as the trigger goes on the stack, and
    // the flyer is the only legal one — the 3/3 is not a "creature with flying".
    let locked: Vec<Target> = state.stack.iter().filter_map(|e| match e {
        mtg_engine::state::StackEntry::Trigger(t) => Some(t.source.chosen_targets.clone()),
        mtg_engine::state::StackEntry::Spell(_) | mtg_engine::state::StackEntry::Ability { .. } => None,
    }).flatten().collect();
    assert!(locked.contains(&Target::Object(flyer)),
        "the flyer is the trigger's target");
    assert!(!locked.contains(&Target::Object(grounded)),
        "'target creature with flying' does not reach a creature without it");

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // "you may" — a prompt, not a fait accompli.
    assert!(matches!(&state.awaiting_action,
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget { optional: true, .. }, ..
        })),
        "the controller is asked whether to deal the damage, got {:?}",
        state.awaiting_action);
    assert_eq!(state.get_object(flyer).unwrap().damage_marked, 0,
        "and nothing has happened yet");

    let choice = if accept {
        ResolvedChoice::ChosenTarget(Some(Target::Object(flyer)))
    } else {
        ResolvedChoice::ChosenTarget(None)
    };
    let state = engine::submit_action(&state, &Action::ResolveChoice { choice }, &reg);
    (state, flyer, grounded)
}

#[test]
fn geistcatchers_rig_deals_four_to_the_flyer_when_you_say_yes() {
    let (state, flyer, grounded) = geistcatchers_rig_hits_the_flyer(true);
    assert_eq!(state.get_object(flyer).unwrap().damage_marked, 4,
        "four damage, not some other number");
    assert_eq!(state.get_object(grounded).unwrap().damage_marked, 0,
        "and only to the creature it targeted");
}

#[test]
fn geistcatchers_rig_deals_nothing_when_you_decline() {
    let (state, flyer, _) = geistcatchers_rig_hits_the_flyer(false);
    assert_eq!(state.get_object(flyer).unwrap().damage_marked, 0,
        "'you may' — declining deals no damage at all");
    assert!(state.awaiting_action.is_none(),
        "and the trigger is finished, not still asking");
}
