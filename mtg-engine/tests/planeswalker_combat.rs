//! Attacking planeswalkers (CR 508.1a, 510.1c, 702.19d/i).
//!
//! Garruk Relentless and Liliana of the Veil are in the pool, so a
//! planeswalker on the defending side is a legal attack target: the attacker
//! still defends against the walker's controller, its combat damage removes
//! loyalty instead of life, trample overflow follows what is being attacked,
//! and a walker that leaves before damage takes the damage with it (no
//! redirect to the player — that rule left the game in 2018).

mod common;

use common::*;
use mtg_engine::actions::{Action, CombatPrompt};
use mtg_engine::combat;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;

/// Declare `attacker` attacking `walker` through the real submit path.
fn submit_attack_on_walker(
    state: &mut mtg_engine::state::GameState,
    attacker: mtg_engine::ids::ObjectId,
    walker: mtg_engine::ids::ObjectId,
    reg: &mtg_engine::cards::CardRegistry,
) {
    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::DeclareAttackers);
    *state = mtg_engine::engine::submit_action(
        state,
        &Action::DeclareAttackers {
            attackers: vec![],
            planeswalker_attacks: vec![(attacker, walker)],
        },
        reg,
    );
}

/// The attack prompt lists the defender's planeswalkers as attackable.
#[test]
fn the_attack_prompt_offers_the_defenders_planeswalkers() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::DeclareAttackers);

    let _attacker = ready_creature(&mut state, P0, 2, 2);
    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    set_loyalty(&mut state, liliana, 3);
    // Your OWN walker is not an attack target.
    let own = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    set_loyalty(&mut state, own, 3);

    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let Some(CombatPrompt::ChooseAttackers { defending_planeswalkers, .. }) = legal.combat_prompt
    else { panic!("expected an attack prompt") };

    assert_eq!(defending_planeswalkers, vec![liliana],
        "the opponent's walker is attackable; your own is not");
}

/// Unblocked combat damage to a planeswalker removes that much loyalty and
/// touches nobody's life total (CR 120.3c).
#[test]
fn an_unblocked_attacker_knocks_loyalty_off_the_walker() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 2, 2);
    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    set_loyalty(&mut state, liliana, 3);

    submit_attack_on_walker(&mut state, attacker, liliana, &reg);
    assert!(state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&attacker)),
        "the walker's controller is the defending player, so the attack is real");
    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(loyalty_of(&state, liliana), 1, "3 loyalty - 2 damage");
    assert_eq!(state.get_player(P1).life, 20, "the player took nothing");
    assert_eq!(state.get_object(liliana).unwrap().zone, Zone::Battlefield);
}

/// Lethal combat damage: loyalty hits 0 and the walker dies to the SBA
/// (CR 704.5i).
#[test]
fn lethal_combat_damage_kills_the_walker() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    // Liliana, not Garruk: Garruk Relentless has a STATE trigger at <=2
    // loyalty (transform) that CR 603.8 fires before the zero-loyalty SBA,
    // which is its own card's test. Liliana dies plainly.
    let attacker = ready_creature(&mut state, P0, 4, 4);
    let garruk = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    set_loyalty(&mut state, garruk, 3);

    submit_attack_on_walker(&mut state, attacker, garruk, &reg);
    combat::deal_combat_damage(&mut state, &reg);
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(garruk).unwrap().zone, Zone::Graveyard,
        "0 loyalty is a dead planeswalker");
    assert_eq!(state.get_player(P1).life, 20,
        "without trample the excess over its loyalty is NOT dealt to the player");
}

/// The defending player may block for their walker; a blocked attacker
/// without trample deals the walker nothing.
#[test]
fn a_blocker_keeps_the_damage_off_the_walker() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 3, 3);
    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    set_loyalty(&mut state, liliana, 3);
    let blocker = ready_creature(&mut state, P1, 2, 4);

    submit_attack_on_walker(&mut state, attacker, liliana, &reg);
    state.step = Step::DeclareBlockers;
    combat::declare_blockers_with_registry(&mut state, &[(blocker, attacker)], &reg);
    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 3, "the blocker ate it");
    assert_eq!(loyalty_of(&state, liliana), 3, "the walker is untouched");
}

/// Trample through a blocker: lethal to the blocker, the rest to the WALKER
/// (CR 702.19d — excess goes to the player or planeswalker being attacked).
#[test]
fn trample_overflow_lands_on_the_attacked_walker() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 5, 5);
    state.get_object_mut(attacker).unwrap().keywords.push(Keyword::Trample);
    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    set_loyalty(&mut state, liliana, 3);
    let blocker = ready_creature(&mut state, P1, 2, 2);

    submit_attack_on_walker(&mut state, attacker, liliana, &reg);
    state.step = Step::DeclareBlockers;
    combat::declare_blockers_with_registry(&mut state, &[(blocker, attacker)], &reg);
    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(loyalty_of(&state, liliana), 0, "5 power - 2 lethal to the blocker = 3 to the walker");
    assert_eq!(state.get_player(P1).life, 20,
        "exactly lethal to the walker leaves nothing to spill to the player");
}

/// Trample past the walker itself: damage beyond its remaining loyalty may be
/// assigned to its controller (CR 702.19i).
#[test]
fn trample_past_the_walker_spills_to_the_player() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 6, 6);
    state.get_object_mut(attacker).unwrap().keywords.push(Keyword::Trample);
    let garruk = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    set_loyalty(&mut state, garruk, 2);

    submit_attack_on_walker(&mut state, attacker, garruk, &reg);
    combat::deal_combat_damage(&mut state, &reg);
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(garruk).unwrap().zone, Zone::Graveyard, "2 of the 6 finish the walker");
    assert_eq!(state.get_player(P1).life, 16, "and the other 4 trample through");
}

/// The walker is destroyed before combat damage: the attack has nothing left
/// to hit. No damage is dealt to anything — in particular not to the player
/// (the pre-2018 redirect rule is gone).
#[test]
fn a_walker_that_dies_first_takes_the_attack_with_it() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 4, 4);
    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    set_loyalty(&mut state, liliana, 3);

    submit_attack_on_walker(&mut state, attacker, liliana, &reg);
    // Removed in response to the attack (Victim of Night, a minus ability...).
    state.move_object(liliana, Zone::Graveyard, &reg);
    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 20,
        "an attacker whose walker left combat deals no combat damage (CR 510.1c)");
}

/// The submit path is the authority: attacking your own walker, or a
/// non-planeswalker, is dropped rather than trusted.
#[test]
fn illegal_walker_attacks_are_dropped_on_submit() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 2, 2);
    let own_walker = named_permanent(&mut state, &reg, "Garruk Relentless", P0);
    set_loyalty(&mut state, own_walker, 3);
    let their_creature = ready_creature(&mut state, P1, 1, 1);

    for bogus in [own_walker, their_creature] {
        let mut probe = state.clone();
        submit_attack_on_walker(&mut probe, attacker, bogus, &reg);
        let attacking = probe.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&attacker));
        assert!(!attacking, "an attack on {} must be dropped",
            probe.get_object(bogus).unwrap().name);
    }
}

fn loyalty_of(state: &mtg_engine::state::GameState, id: mtg_engine::ids::ObjectId) -> u32 {
    state.get_object(id).unwrap().counters.get(&CounterType::Loyalty).copied().unwrap_or(0)
}

// ── Tokens created "tapped and attacking" choose their defender ──────
//
// CR 508.4b, and the matching rulings on both cards (Geist of Saint Traft
// 2020-08-07, Kessig Cagebreakers 2011-09-22): "You choose which player or
// planeswalker the token is attacking. It doesn't have to be attacking the
// same player or planeswalker [the creature] is attacking." With a single
// opponent and no planeswalkers there is only one legal choice and nothing is
// asked — which is what every pre-planeswalker test of these cards exercises.

fn answer_attack_target(
    state: &mtg_engine::state::GameState,
    chosen: mtg_engine::actions::Target,
    reg: &mtg_engine::cards::CardRegistry,
) -> mtg_engine::state::GameState {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTarget { options, .. }, ..
        }) => assert!(options.contains(&chosen), "{chosen:?} not among {options:?}"),
        other => panic!("expected an attack-target choice, got {other:?}"),
    }
    mtg_engine::engine::submit_action(state, &Action::ResolveChoice {
        choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(chosen)),
    }, reg)
}

/// Geist's Angel may be sent at a planeswalker the defending player controls,
/// and its combat damage then removes loyalty rather than life.
#[test]
fn geists_angel_can_be_sent_at_a_planeswalker() {
    use mtg_engine::actions::Target;
    use mtg_engine::cards::AttackInfo;

    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let geist = named_permanent(&mut state, &reg, "Geist of Saint Traft", P0);
    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    set_loyalty(&mut state, liliana, 3);
    attacks_unblocked(&mut state, geist, P1);

    let card_id = state.get_object(geist).unwrap().card_id;
    reg.get(card_id).unwrap()
        .on_attacks(&mut state, geist, AttackInfo::new(geist, P1), &[], &reg);

    // The walker makes it a real choice: the player or their planeswalker.
    let state = answer_attack_target(&state, Target::Object(liliana), &reg);
    let mut state = state;

    let angel = find_token_named(&state, "Angel Token").expect("the Angel exists");
    assert_eq!(state.combat.as_ref().and_then(|c| c.attackers.get(&angel).copied()), Some(P1),
        "the Angel still defends against the walker's controller (CR 508.1a)");
    assert_eq!(
        state.combat.as_ref().and_then(|c| c.planeswalker_defenders.get(&angel).copied()),
        Some(liliana),
        "and the walker is what it is attacking");

    combat::deal_combat_damage(&mut state, &reg);
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 18,
        "only Geist's 2 reached the player; the Angel's 4 went to the walker");
    assert_ne!(state.get_object(liliana).map(|o| o.zone), Some(Zone::Battlefield),
        "3 loyalty - 4 damage: Liliana died to the loyalty SBA");
}

/// Kessig Cagebreakers' ruling says "each token": with two Wolves and a
/// walker on the defending side, the controller is asked once per Wolf and
/// the answers are independent.
#[test]
fn each_kessig_wolf_chooses_its_own_defender() {
    use mtg_engine::actions::Target;
    use mtg_engine::cards::AttackInfo;

    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let cage = named_permanent(&mut state, &reg, "Kessig Cagebreakers", P0);
    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    set_loyalty(&mut state, liliana, 3);
    for _ in 0..2 {
        let c = ready_creature(&mut state, P0, 2, 2);
        state.move_object(c, Zone::Graveyard, &reg);
    }
    attacks_unblocked(&mut state, cage, P1);

    let card_id = state.get_object(cage).unwrap().card_id;
    reg.get(card_id).unwrap()
        .on_attacks(&mut state, cage, AttackInfo::new(cage, P1), &[], &reg);

    // First Wolf at the walker, second at the player.
    let state = answer_attack_target(&state, Target::Object(liliana), &reg);
    let state = answer_attack_target(&state, Target::Player(P1), &reg);

    assert!(state.awaiting_action.is_none(), "two tokens, two answers, done");
    let combat = state.combat.as_ref().unwrap();
    let wolves: Vec<_> = state.objects.values()
        .filter(|o| o.is_token && o.name == "Wolf Token")
        .map(|o| o.id)
        .collect();
    assert_eq!(wolves.len(), 2);
    assert!(wolves.iter().all(|w| combat.attackers.get(w) == Some(&P1)),
        "both Wolves defend against the walker's controller");
    let at_walker: Vec<_> = wolves.iter()
        .filter(|w| combat.planeswalker_defenders.get(w) == Some(&liliana))
        .collect();
    assert_eq!(at_walker.len(), 1,
        "exactly one Wolf is attacking Liliana; the other attacks the player");
}
