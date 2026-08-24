mod common;
use common::*;

use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::state::StackEntry;
use mtg_engine::triggers::PendingTrigger;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// CR 113.7a: A triggered ability on the stack is independent of its source.
// Removing the source after the trigger is on the stack does not counter it.
// The engine gates trigger resolution on `source.zone == Battlefield`,
// which incorrectly prevents resolution when the source has left.

// Angel of Flight Alabaster: "At the beginning of your upkeep, return target
// Spirit card from your graveyard to your hand."
// Destroying the Angel after the trigger stacks should not prevent the return.
#[test]
fn test_angel_of_flight_alabaster_trigger_resolves_after_death() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let angel = named_creature(&mut state, &reg, "Angel of Flight Alabaster", P0);
    let angel_card = reg.get_id_by_name("Angel of Flight Alabaster").unwrap();

    // An actual Spirit card: the Angel's ability targets "target Spirit card
    // in your graveyard", and CR 608.2b re-checks that on resolution. A
    // synthetic creature with no subtypes is not a legal target, so the
    // trigger would rightly fizzle for a reason unrelated to this test.
    let spirit = named_card_in_graveyard(&mut state, &reg, "Chapel Geist", P0);

    state.stack.push(StackEntry::Trigger(PendingTrigger::UpkeepTrigger {
        object_id: angel,
        card_id: angel_card,
        controller: P0,
        description: "Angel of Flight Alabaster".into(),
        chosen_targets: vec![Target::Object(spirit)],
    }));

    state.move_object(angel, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        state.get_object(spirit).unwrap().zone,
        Zone::Hand,
        "CR 113.7a: Angel upkeep trigger should return Spirit to hand even after Angel is destroyed"
    );
}

// Charmbreaker Devils: "At the beginning of your upkeep, return an instant or
// sorcery card at random from your graveyard to your hand."
// Destroying the Devils after the trigger stacks should not prevent the return.
#[test]
fn test_charmbreaker_devils_trigger_resolves_after_death() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let devils = named_creature(&mut state, &reg, "Charmbreaker Devils", P0);
    let devils_card = reg.get_id_by_name("Charmbreaker Devils").unwrap();

    let instant = named_card_in_graveyard(&mut state, &reg, "Think Twice", P0);
    state.get_object_mut(instant).unwrap().card_types = vec![CardType::Instant];

    state.stack.push(StackEntry::Trigger(PendingTrigger::UpkeepTrigger {
        object_id: devils,
        card_id: devils_card,
        controller: P0,
        description: "Charmbreaker Devils".into(),
        chosen_targets: vec![],
    }));

    state.move_object(devils, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        state.get_object(instant).unwrap().zone,
        Zone::Hand,
        "CR 113.7a: Devils upkeep trigger should return instant to hand even after Devils are destroyed"
    );
}

// Geist of Saint Traft: "Whenever Geist of Saint Traft attacks, create a 4/4
// white Angel creature token with flying that's tapped and attacking."
// Destroying the Geist after the attack trigger stacks should still create the token.
#[test]
fn test_geist_of_saint_traft_angel_token_created_after_death() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let geist = named_creature(&mut state, &reg, "Geist of Saint Traft", P0);
    let geist_card = reg.get_id_by_name("Geist of Saint Traft").unwrap();

    state.stack.push(StackEntry::Trigger(PendingTrigger::AttacksTrigger {
        object_id: geist,
        card_id: geist_card,
        controller: P0,
        description: "Geist of Saint Traft".into(),
        chosen_targets: vec![],
    }));

    state.move_object(geist, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        count_tokens_named(&state, "Angel"),
        1,
        "CR 113.7a: Geist attack trigger should create Angel token even after Geist is destroyed"
    );
}

// Kessig Cagebreakers: "Whenever Kessig Cagebreakers attacks, create a 2/2
// green Wolf creature token that's tapped and attacking for each creature
// card in your graveyard."
// After destroying Cagebreakers in response, the count includes Cagebreakers itself.
#[test]
fn test_kessig_cagebreakers_tokens_created_after_death() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let cb = named_creature(&mut state, &reg, "Kessig Cagebreakers", P0);
    let cb_card = reg.get_id_by_name("Kessig Cagebreakers").unwrap();

    let c1 = ready_creature(&mut state, P0, 2, 2);
    state.move_object(c1, Zone::Graveyard, &reg);
    let c2 = ready_creature(&mut state, P0, 3, 3);
    state.move_object(c2, Zone::Graveyard, &reg);

    state.stack.push(StackEntry::Trigger(PendingTrigger::AttacksTrigger {
        object_id: cb,
        card_id: cb_card,
        controller: P0,
        description: "Kessig Cagebreakers".into(),
        chosen_targets: vec![],
    }));

    state.move_object(cb, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        count_tokens_named(&state, "Wolf"),
        3,
        "CR 113.7a: Cagebreakers should create 3 Wolf tokens even after death (2 original + itself)"
    );
}

// Splinterfright: "At the beginning of your upkeep, mill two cards."
// Destroying Splinterfright after the trigger stacks should not prevent the mill.
#[test]
fn test_splinterfright_mill_resolves_after_death() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let splinter = named_creature(&mut state, &reg, "Splinterfright", P0);
    let splinter_card = reg.get_id_by_name("Splinterfright").unwrap();

    let filler = reg.get_id_by_name("Forest").unwrap();
    for _ in 0..5 {
        let id = state.create_object(filler, P0, Zone::Library, None, None);
        state.players[0].library_order.push(id);
    }
    let lib_before = state.players[0].library_order.len();

    state.stack.push(StackEntry::Trigger(PendingTrigger::UpkeepTrigger {
        object_id: splinter,
        card_id: splinter_card,
        controller: P0,
        description: "Splinterfright".into(),
        chosen_targets: vec![],
    }));

    state.move_object(splinter, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    let lib_after = state.players[0].library_order.len();
    assert_eq!(
        lib_before - lib_after,
        2,
        "CR 113.7a: Splinterfright upkeep trigger should mill 2 even after destruction"
    );
}

// Undead Alchemist: "Whenever a creature card is put into an opponent's graveyard
// from their library, exile that card and create a 2/2 black Zombie creature token."
// Destroying the Alchemist after the trigger stacks should not prevent exile + token.
#[test]
fn test_undead_alchemist_watch_resolves_after_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let alchemist = named_creature(&mut state, &reg, "Undead Alchemist", P0);
    let alchemist_card = reg.get_id_by_name("Undead Alchemist").unwrap();

    let milled = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P1);

    state.stack.push(StackEntry::Trigger(PendingTrigger::CreatureCardMilledWatch {
        watcher_id: alchemist,
        watcher_card_id: alchemist_card,
        controller: P0,
        milled_object: milled,
        milled_player: P1,
        description: "Undead Alchemist".into(),
    }));

    state.move_object(alchemist, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        state.get_object(milled).unwrap().zone,
        Zone::Exile,
        "CR 113.7a: Alchemist trigger should exile milled creature even after Alchemist is destroyed"
    );
    assert_eq!(
        count_tokens_named(&state, "Zombie"),
        1,
        "CR 113.7a: Alchemist trigger should create Zombie token even after Alchemist is destroyed"
    );
}

// Gutter Grime: "Whenever a nontoken creature you control dies, put a slime
// counter on Gutter Grime, then create a green Ooze creature token."
// When Gutter Grime and a creature are destroyed simultaneously, the death
// trigger should still fire per CR 603.10.
#[test]
fn test_gutter_grime_creates_token_when_ltb() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_creature(&mut state, &reg, "Gutter Grime", P0);
    let grime_card = reg.get_id_by_name("Gutter Grime").unwrap();
    let creature = ready_creature(&mut state, P0, 2, 2);

    state.move_object(grime, Zone::Graveyard, &reg);
    state.move_object(creature, Zone::Graveyard, &reg);

    state.stack.push(StackEntry::Trigger(PendingTrigger::DeathWatch {
        watcher_id: grime,
        watcher_card_id: grime_card,
        controller: P0,
        dead_id: creature,
        dead_controller: P0,
        dead_damaged_by: vec![],
        dead_toughness: 2,
        dead_is_token: false,
        description: "Gutter Grime".into(),
        chosen_targets: vec![],
    }));

    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        count_tokens_named(&state, "Ooze"),
        1,
        "CR 603.10: Gutter Grime death-watch should create Ooze token after simultaneous destruction"
    );
}

// Murder of Crows: "Whenever another creature dies, you may draw a card.
// If you do, discard a card."
// When Murder of Crows and another creature die simultaneously, the death
// trigger should still present the draw choice per CR 603.10.
#[test]
fn test_murder_of_crows_trigger_resolves_after_simultaneous_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let murder = named_creature(&mut state, &reg, "Murder of Crows", P0);
    let murder_card = reg.get_id_by_name("Murder of Crows").unwrap();
    let creature = ready_creature(&mut state, P0, 2, 2);

    state.move_object(murder, Zone::Graveyard, &reg);
    state.move_object(creature, Zone::Graveyard, &reg);

    state.stack.push(StackEntry::Trigger(PendingTrigger::DeathWatch {
        watcher_id: murder,
        watcher_card_id: murder_card,
        controller: P0,
        dead_id: creature,
        dead_controller: P0,
        dead_damaged_by: vec![],
        dead_toughness: 2,
        dead_is_token: false,
        description: "Murder of Crows".into(),
        chosen_targets: vec![],
    }));

    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        state.awaiting_action.is_some(),
        true,
        "CR 603.10: Murder of Crows trigger should present draw choice even after simultaneous death"
    );
}

// Mentor of the Meek: "Whenever a creature with power 2 or less enters the
// battlefield under your control, you may pay {1}. If you do, draw a card."
// Destroying Mentor after the trigger stacks should not prevent the pay choice.
#[test]
fn test_mentor_of_the_meek_trigger_resolves_after_removal() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mentor = named_creature(&mut state, &reg, "Mentor of the Meek", P0);
    let mentor_card = reg.get_id_by_name("Mentor of the Meek").unwrap();
    let small_creature = ready_creature(&mut state, P0, 1, 1);

    state.stack.push(StackEntry::Trigger(PendingTrigger::EnterWatch {
        watcher_id: mentor,
        watcher_card_id: mentor_card,
        controller: P0,
        entered_id: small_creature,
        entered_controller: P0,
        description: "Mentor of the Meek".into(),
    }));

    state.move_object(mentor, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        state.awaiting_action.is_some(),
        true,
        "CR 113.7a: Mentor trigger should present pay choice even after Mentor is destroyed"
    );
}

// Trepanation Blade: "Whenever equipped creature attacks, defending player
// reveals cards from the top of their library until they reveal a land card.
// That creature gets +1/+0 until end of turn for each card revealed this way."
// Destroying the Blade after the trigger stacks should not prevent the mill + pump.
#[test]
fn test_trepanation_blade_trigger_resolves_after_equipment_destroyed() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);
    let blade = named_equipment(&mut state, &reg, "Trepanation Blade", P0);
    let blade_card = reg.get_id_by_name("Trepanation Blade").unwrap();
    state.get_object_mut(blade).unwrap().attached_to = Some(creature);

    let filler = reg.get_id_by_name("Walking Corpse").unwrap();
    for _ in 0..2 {
        let id = state.create_object(filler, P1, Zone::Library, Some(2), Some(2));
        state.get_object_mut(id).unwrap().card_types = vec![CardType::Creature];
        state.players[1].library_order.push(id);
    }
    let land_card = reg.get_id_by_name("Forest").unwrap();
    let land = state.create_object(land_card, P1, Zone::Library, None, None);
    state.get_object_mut(land).unwrap().card_types = vec![CardType::Land];
    state.players[1].library_order.push(land);

    let lib_before = state.players[1].library_order.len();

    state.stack.push(StackEntry::Trigger(PendingTrigger::AttacksTrigger {
        object_id: blade,
        card_id: blade_card,
        controller: P0,
        description: "Trepanation Blade".into(),
        chosen_targets: vec![],
    }));

    state.move_object(blade, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    let lib_after = state.players[1].library_order.len();
    assert_ne!(
        lib_before, lib_after,
        "CR 113.7a: Trepanation Blade trigger should mill even after equipment is destroyed"
    );
}
