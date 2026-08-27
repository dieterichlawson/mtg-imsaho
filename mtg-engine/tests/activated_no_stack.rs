//! CR 602.2a: activating an ability puts it on the stack. It does not happen
//! yet — every player gets priority first, and the ability can be responded to,
//! countered, or made to fizzle before any of its effect lands.
//!
//! Costs are the exception: they are paid during activation (CR 601.2h), so a
//! creature tapped to pay stays tapped even if the ability never resolves.
//!
//! Each test checks the effect has *not* happened, then resolves the stack
//! entry and checks it does. On its own, "not yet" passes for an ability that
//! is simply broken; it means something only next to "and now".
//!
//! Most of these drive the two hooks by hand. The last test drives the real
//! action instead — `submit_action(ActivateAbility)` — because the engine used
//! to resolve the ability on the spot, immediately after pushing it, so every
//! test in this file was checking a seam the game never went through.

mod common;
use common::*;

use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::state::GameState;
use mtg_engine::types::*;

fn setup_skirsdag(state: &mut GameState, reg: &CardRegistry) -> (ObjectId, ObjectId, ObjectId) {
    let priest = named_permanent(state, reg, "Skirsdag High Priest", P0);
    let creature1 = ready_creature(state, P0, 2, 2);
    let creature2 = ready_creature(state, P0, 2, 2);
    state.creature_died_this_turn = true;
    (priest, creature1, creature2)
}

/// Resolve the ability sitting on top of the stack.
///
/// Every test below activates an ability and checks its effect has *not*
/// happened yet. On its own that passes for an ability that is simply broken,
/// so each one now resolves the stack entry afterwards and checks the effect
/// does land — "not yet" only means something next to "and now".
fn resolve_the_ability(state: &mut GameState, reg: &CardRegistry) {
    assert!(!state.stack.is_empty(),
        "CR 602.2a: activating should have put the ability on the stack");
    mtg_engine::stack::resolve_top_of_stack(state, reg);
}

// CR 602.2a: activating Back from the Brink's ability should place it on the
// stack. The token copy is created only when the ability resolves, giving
// opponents a window to respond.
#[test]
fn back_from_the_brink_makes_its_token_on_resolution() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let brink = named_permanent(&mut state, &reg, "Back from the Brink", P0);
    let gy_creature = named_card_in_graveyard(&mut state, &reg, "Unbreathing Horde", P0);

    // Back from the Brink's ability names a card in the graveyard to exile, so
    // it encodes the chosen card's id in the ability index rather than using a
    // target.
    activate_via_hooks(&mut state, &reg, brink, gy_creature.0 as usize, &[]);

    assert_eq!(
        count_tokens_named(&state, "Unbreathing Horde"), 0,
        "CR 602.2a: activated ability should be on the stack; token created only on resolution"
    );

    resolve_the_ability(&mut state, &reg);
    assert_eq!(
        count_tokens_named(&state, "Unbreathing Horde"), 1,
        "and the token does arrive once the ability resolves"
    );
}

// CR 602.2a: Kessig Wolf Run's pump ability should go on the stack before
// the +X/+0 and trample are granted. An opponent can respond by removing
// the target creature.
#[test]
fn kessig_wolf_run_pumps_on_resolution() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf_run = named_permanent(&mut state, &reg, "Kessig Wolf Run", P0);
    let target = ready_creature(&mut state, P0, 2, 2);

    state.last_activated_x_value = Some(3);
    let base_power = state.effective_power(target, &reg).unwrap();

    activate_via_hooks(&mut state, &reg, wolf_run, 1, &[Target::Object(target)]);

    assert_eq!(
        state.effective_power(target, &reg).unwrap(), base_power,
        "CR 602.2a: Kessig Wolf Run pump should not apply until ability resolves from stack"
    );

    resolve_the_ability(&mut state, &reg);
    assert_eq!(
        state.effective_power(target, &reg).unwrap(), base_power + 3,
        "and the +3/+0 does apply once the ability resolves"
    );
    assert!(state.has_keyword(target, Keyword::Trample, &reg),
        "along with the trample it grants");
}

// CR 602.2a: Nephalia Drownyard's mill ability should go on the stack.
// The target player gets priority to respond before any cards are milled.
#[test]
fn nephalia_drownyard_mills_on_resolution() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let drownyard = named_permanent(&mut state, &reg, "Nephalia Drownyard", P0);
    stock_library(&mut state, &reg, P1, 5);

    let lib_before = state.players[1].library_order.len();

    activate_via_hooks(&mut state, &reg, drownyard, 1, &[Target::Player(P1)]);

    assert_eq!(
        state.players[1].library_order.len(), lib_before,
        "CR 602.2a: Nephalia Drownyard mill should not occur until ability resolves from stack"
    );

    resolve_the_ability(&mut state, &reg);
    assert_eq!(
        state.players[1].library_order.len(), lib_before - 3,
        "and the mill does happen once the ability resolves — Drownyard mills three"
    );
}

// CR 602.2a: Tree of Redemption's exchange ability should go on the stack.
// Opponents can destroy the Tree in response, causing the exchange to fail
// because the Tree's toughness no longer exists as a value to exchange.
#[test]
fn tree_of_redemption_exchanges_on_resolution() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let tree = named_permanent(&mut state, &reg, "Tree of Redemption", P0);
    let life_before = state.players[0].life;

    activate_via_hooks(&mut state, &reg, tree, 0, &[]);

    assert_eq!(
        state.players[0].life, life_before,
        "CR 602.2a: Tree of Redemption exchange should not occur until ability resolves from stack"
    );

    resolve_the_ability(&mut state, &reg);
    assert_eq!(
        state.players[0].life, 13,
        "and the exchange does happen once the ability resolves — life becomes the Tree's toughness"
    );
}

// CR 602.2a: Full Moon's Rise sacrifice-and-regenerate ability should go on
// the stack. Opponents receive priority to respond (e.g., exile a Werewolf)
// before regeneration shields are applied.
#[test]
fn full_moons_rise_shields_on_resolution() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let full_moon = named_permanent(&mut state, &reg, "Full Moon's Rise", P0);
    let werewolf = named_permanent(&mut state, &reg, "Daybreak Ranger", P0);

    activate_via_hooks(&mut state, &reg, full_moon, 0, &[]);

    assert_eq!(
        state.get_object(werewolf).unwrap().regeneration_shields, 0,
        "CR 602.2a: regeneration shields should not apply until ability resolves from stack"
    );

    resolve_the_ability(&mut state, &reg);
    assert!(
        state.get_object(werewolf).unwrap().regeneration_shields > 0,
        "and the shield does go up once the ability resolves"
    );
}

// "{1}{U}: This creature's owner shuffles it into their library. If that player
// does, they reveal cards from the top of that library until a card named
// Mirror-Mad Phantasm is revealed. The player puts that card onto the
// battlefield and all other cards revealed this way into their graveyard."
#[test]
fn mirror_mad_phantasm_shuffles_and_digs_itself_back_out_on_resolution() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let phantasm = named_permanent(&mut state, &reg, "Mirror-Mad Phantasm", P0);
    stock_library(&mut state, &reg, P0, 10);

    activate_via_hooks(&mut state, &reg, phantasm, 0, &[]);

    assert_eq!(state.players[0].library_order.len(), 10,
        "CR 602.2a: with the ability on the stack, nothing has been shuffled or \
         milled yet — this is the window to respond in");

    resolve_the_ability(&mut state, &reg);

    // How many cards get milled depends on where the shuffle put the Phantasm,
    // so what is asserted is what does not depend on that.
    assert_eq!(state.get_object(phantasm).unwrap().zone, Zone::Battlefield,
        "it shuffles in, is revealed, and comes back onto the battlefield");
    let library = state.players[0].library_order.len();
    let graveyard = state.objects_in_zone(Zone::Graveyard, P0).len();
    assert_eq!(library + graveyard, 10,
        "every stocked card is either still in the library or was revealed and \
         milled; library={library}, graveyard={graveyard}");
}

// Per Scryfall ruling on Mirror-Mad Phantasm: if the creature is no longer on
// the battlefield when the ability resolves, it cannot be "shuffled into its
// owner's library." The conditional "if that player does" is false, so no
// reveal or mill occurs. The library should be unchanged.
#[test]
fn mirror_mad_phantasm_source_removed_before_resolution() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let phantasm = named_permanent(&mut state, &reg, "Mirror-Mad Phantasm", P0);
    stock_library(&mut state, &reg, P0, 10);

    let lib_before = state.players[0].library_order.len();

    // Opponent exiles Mirror-Mad Phantasm before the ability resolves.
    state.move_object(phantasm, Zone::Exile, &reg);

    activate_via_hooks(&mut state, &reg, phantasm, 0, &[]);

    assert_eq!(
        state.players[0].library_order.len(), lib_before,
        "creature in exile cannot be shuffled into library; no reveal/mill should occur"
    );
}

// CR 602.2a: Skirsdag High Priest's ability should go on the stack after
// activation. The Demon token is created only when the ability resolves,
// giving the opponent a window to respond.
#[test]
fn skirsdag_high_priest_makes_its_demon_on_resolution() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let (priest, _c1, _c2) = setup_skirsdag(&mut state, &reg);

    activate_via_hooks(&mut state, &reg, priest, 0, &[]);

    assert_eq!(
        count_tokens_named(&state, "Demon"), 0,
        "CR 602.2a: Skirsdag High Priest should not create Demon token until ability resolves"
    );

    resolve_the_ability(&mut state, &reg);
    assert_eq!(
        count_tokens_named(&state, "Demon"), 1,
        "and the Demon does arrive once the ability resolves"
    );
}

// CR 602.2a: the two creatures tapped as part of Skirsdag High Priest's
// activation cost should be tapped before the ability goes on the stack (cost
// payment is immediate), but the Demon token should not be created until the
// ability resolves. If the ability were countered, the creatures would remain
// tapped and no token would exist.
#[test]
fn skirsdag_high_priests_tap_cost_is_paid_at_activation() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let (priest, creature1, creature2) = setup_skirsdag(&mut state, &reg);

    activate_via_hooks(&mut state, &reg, priest, 0, &[]);

    assert!(
        state.get_object(creature1).unwrap().tapped,
        "tap cost should be paid during activation"
    );
    assert!(
        state.get_object(creature2).unwrap().tapped,
        "tap cost should be paid during activation"
    );
    assert_eq!(
        count_tokens_named(&state, "Demon"), 0,
        "CR 602.2a: Demon token should not be created until ability resolves from stack"
    );

    resolve_the_ability(&mut state, &reg);
    assert_eq!(count_tokens_named(&state, "Demon"), 1,
        "and the Demon does arrive once the ability resolves");
    assert!(state.get_object(creature1).unwrap().tapped,
        "the creatures tapped as a cost stay tapped");
}

// CR 302.6: the {T} restriction (summoning sickness) applies only to the
// High Priest's own tap symbol, not to the other creatures tapped as an
// additional cost. A summoning-sick creature is a valid tap candidate.
// CR 602.2a: even with a summoning-sick creature tapped as cost, the ability
// should go on the stack — no Demon token until resolution.
#[test]
fn skirsdag_summoning_sick_creature_can_be_tapped() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let priest = named_permanent(&mut state, &reg, "Skirsdag High Priest", P0);
    let _creature_nonsick = ready_creature(&mut state, P0, 2, 2);
    let creature_sick = sick_creature(&mut state, P0, 2, 2);
    state.creature_died_this_turn = true;

    activate_via_hooks(&mut state, &reg, priest, 0, &[]);

    assert!(
        state.get_object(creature_sick).unwrap().tapped,
        "CR 302.6: summoning-sick creature should be valid tap candidate for additional cost"
    );
    assert_eq!(
        count_tokens_named(&state, "Demon"), 0,
        "CR 602.2a: Demon token should not exist until ability resolves from stack"
    );

    resolve_the_ability(&mut state, &reg);
    assert_eq!(count_tokens_named(&state, "Demon"), 1,
        "and the Demon does arrive once the ability resolves");
}

// ══════════════════════════════════════════════════════════════════
// The real path
// ══════════════════════════════════════════════════════════════════

/// Every test above calls the two hooks itself. This one takes the action a
/// player takes.
///
/// The engine's `ActivateAbility` handler used to call `on_activate_ability`
/// and then `resolve_top_of_stack` in the same breath, so an ability was
/// pushed and resolved without priority ever passing: CR 602.2a's stack entry
/// existed for the length of one function call, and no opponent could respond
/// to anything. The whole file was testing a seam the game did not use.
#[test]
fn activating_through_the_engine_leaves_the_ability_on_the_stack() {
    use mtg_engine::actions::Action;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf = named_permanent(&mut state, &reg, "Darkthicket Wolf", P0);
    add_mana(&mut state, P0, &[(ManaType::Colorless, 2), (ManaType::Green, 1)]);
    assert_eq!(state.effective_power(wolf, &reg), Some(2), "test premise: a 2/2");

    let action = mtg_engine::engine::legal_actions(&state, &reg).actions.into_iter()
        .find(|a| matches!(a, Action::ActivateAbility { object_id, .. } if *object_id == wolf))
        .expect("the pump ability is offered");
    let state = mtg_engine::engine::submit_action(&state, &action, &reg);

    assert!(matches!(state.stack.last(), Some(mtg_engine::state::StackEntry::Ability { .. })),
        "CR 602.2a: activating puts the ability on the stack; got {:?}", state.stack);
    assert_eq!(state.effective_power(wolf, &reg), Some(2),
        "the +2/+2 has not happened yet — an opponent still has priority");
    assert_eq!(state.consecutive_passes, 0,
        "CR 117.3b: taking an action resets the pass count, or the ability \
         resolves without the opponent ever seeing it");

    let mut state = state;
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert_eq!(state.effective_power(wolf, &reg), Some(4),
        "and it lands when the ability resolves");
}
