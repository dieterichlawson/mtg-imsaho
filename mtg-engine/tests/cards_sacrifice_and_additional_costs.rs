//! Spells whose cost is more than mana (CR 601.2b) — sacrifice a creature, exile
//! from a graveyard, discard — and the cards that make a player do the same.
//!
//! Cards covered (12), so this is greppable by name as well as by rule:
//!
//! - Altar's Reap
//! - Brain Weevil
//! - Corpse Lunge
//! - Disciple of Griselbrand
//! - Divine Reckoning
//! - Harvest Pyre
//! - Infernal Plunge
//! - Selfless Cathar
//! - Silverchase Fox
//! - Skirsdag Cultist
//! - Stitcher's Apprentice
//! - Tribute to Hunger

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;
/// "{1}{W}, Sacrifice this creature: Creatures **you control** get +1/+1 until
/// end of turn."
#[test]
fn selfless_cathar_pumps_only_the_creatures_you_control() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cathar = named_permanent(&mut state, &reg, "Selfless Cathar", P0);
    let bear = ready_creature(&mut state, P0, 2, 2);
    let theirs = ready_creature(&mut state, P1, 2, 2);

    // Add mana for the ability: {1}{W}
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let new_state = activate(&state, &reg, cathar, 0, vec![]);

    // Cathar should be in graveyard (sacrificed).
    assert_eq!(
        new_state.get_object(cathar).unwrap().zone,
        Zone::Graveyard,
        "Selfless Cathar should be sacrificed"
    );

    // Bear should have +1/+1 from the effect.
    assert_eq!(new_state.effective_power(bear, &reg).unwrap(), 3);
    assert_eq!(new_state.effective_toughness(bear, &reg).unwrap(), 3);

    assert_eq!(new_state.effective_power(theirs, &reg).unwrap(), 2,
        "the opponent's creature is not one you control");
    assert_eq!(new_state.effective_toughness(theirs, &reg).unwrap(), 2);
}

/// Ruling: "You can activate Selfless Cathar's ability even if you control no
/// other creatures." The ability names no minimum, and the Cathar sacrifices
/// itself to pay for it — so the empty board is the ordinary case, not an
/// exception.
///
/// And the cost really is {1}{W}: with one white mana the ability is not on
/// offer, which is the only way to tell {1}{W} from {W} here.
#[test]
fn selfless_cathar_can_be_activated_with_no_other_creatures_but_not_for_one_mana() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let cathar = named_permanent(&mut state, &reg, "Selfless Cathar", P0);

    let offered = |s: &mtg_engine::state::GameState| {
        engine::legal_actions(s, &reg).actions.iter().any(|a| matches!(a,
            Action::ActivateAbility { object_id, .. } if *object_id == cathar))
    };

    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);
    assert!(!offered(&state), "{{W}} alone does not pay {{1}}{{W}}");

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    assert!(offered(&state),
        "with {{1}}{{W}} it is activatable, alone on the battlefield");

    let after = activate(&state, &reg, cathar, 0, vec![]);
    assert_eq!(after.get_object(cathar).unwrap().zone, Zone::Graveyard,
        "and it sacrifices itself, pumping nothing");
}
#[test]
fn silverchase_fox_exiles_enchantment() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let fox = named_permanent(&mut state, &reg, "Silverchase Fox", P0);

    // Create an enchantment for P1 (use Pacifism as a representative enchantment).
    let enchantment = named_permanent(&mut state, &reg, "Glorious Anthem", P1);

    // Add mana for the ability: {1}{W}
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let new_state = activate(&state, &reg, fox, 0, vec![Target::Object(enchantment)]);

    // Fox should be in graveyard (sacrificed).
    assert_eq!(
        new_state.get_object(fox).unwrap().zone,
        Zone::Graveyard,
        "Silverchase Fox should be sacrificed"
    );

    // Enchantment should be in exile.
    assert_eq!(
        new_state.get_object(enchantment).unwrap().zone,
        Zone::Exile,
        "Target enchantment should be exiled"
    );
}
#[test]
fn brain_weevil_forces_discard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let weevil = named_permanent(&mut state, &reg, "Brain Weevil", P0);

    // Give P1 exactly 2 cards in hand (auto-discards all when <= 2).
    let _c1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    let _c2 = spell_in_hand(&mut state, &reg, "Lightning Bolt", P1);

    let hand_before = state.objects_in_zone(Zone::Hand, P1).len();
    assert_eq!(hand_before, 2);

    let new_state = activate(&state, &reg, weevil, 0, vec![Target::Player(P1)]);

    // Weevil should be in graveyard (sacrificed).
    assert_eq!(
        new_state.get_object(weevil).unwrap().zone,
        Zone::Graveyard,
        "Brain Weevil should be sacrificed"
    );

    // P1 should have discarded 2 cards (2 - 2 = 0 remaining).
    let hand_after = new_state.objects_in_zone(Zone::Hand, P1).len();
    assert_eq!(hand_after, 0, "P1 should have 0 cards left after discarding 2");
}
/// Ruling 2013-04-15: "If you cast this as normal during your main phase, it
/// will enter the battlefield and you'll receive priority. If no abilities
/// trigger because of this, you can activate its ability immediately, before
/// any other player has a chance to remove it from the battlefield."
///
/// The cost is "Sacrifice this creature" with no {T} in it, and summoning
/// sickness only restricts a {T} (or {Q}) symbol in a creature's own cost
/// (CR 302.6). So a Brain Weevil that arrived this turn can still eat itself.
#[test]
fn brain_weevil_can_be_sacrificed_the_turn_it_arrives() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let weevil = named_permanent(&mut state, &reg, "Brain Weevil", P0);
    state.get_object_mut(weevil).unwrap().summoning_sick = true;
    spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);

    assert!(offers_ability_of(&state, &reg, weevil),
        "no {{T}} in the cost, so summoning sickness has nothing to say");

    let after = activate(&state, &reg, weevil, 0, vec![Target::Player(P1)]);
    assert_eq!(after.objects_in_zone(Zone::Hand, P1).len(), 0);
}

/// "Target **player**" — that includes you. Nothing about the ability says
/// opponent.
#[test]
fn brain_weevil_can_target_its_own_controller() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let weevil = named_permanent(&mut state, &reg, "Brain Weevil", P0);
    spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    spell_in_hand(&mut state, &reg, "Lightning Bolt", P0);

    let after = activate(&state, &reg, weevil, 0, vec![Target::Player(P0)]);
    assert_eq!(after.objects_in_zone(Zone::Hand, P0).len(), 0,
        "you may point it at yourself");
}

/// "Discards two cards" with one card in hand discards the one — a player does
/// as much as they can and does not lose for the shortfall.
#[test]
fn brain_weevil_takes_the_only_card_in_a_one_card_hand() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let weevil = named_permanent(&mut state, &reg, "Brain Weevil", P0);
    let only = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);

    let after = activate(&state, &reg, weevil, 0, vec![Target::Player(P1)]);
    assert_eq!(after.get_object(only).unwrap().zone, Zone::Graveyard);
    assert!(after.awaiting_action.is_none(),
        "one card is not a choice, so no prompt is left open");
}

/// An empty hand is not an error, and not a prompt either.
#[test]
fn brain_weevil_against_an_empty_hand_does_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let weevil = named_permanent(&mut state, &reg, "Brain Weevil", P0);
    let after = activate(&state, &reg, weevil, 0, vec![Target::Player(P1)]);

    assert!(after.awaiting_action.is_none());
    assert_eq!(after.get_object(weevil).unwrap().zone, Zone::Graveyard,
        "the sacrifice is a cost, paid whether or not the effect finds anything");
}

/// Both discards are the *targeted player's* choice, not the activator's, and
/// the second is asked after the first has left the hand.
///
/// The card used to chain the second discard itself, carrying the target
/// player between the two in `card_state` as an `ObjectId`.
#[test]
fn brain_weevils_two_discards_are_both_the_targeted_players_choice() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let weevil = named_permanent(&mut state, &reg, "Brain Weevil", P0);
    let a = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    let b = spell_in_hand(&mut state, &reg, "Lightning Bolt", P1);
    let c = spell_in_hand(&mut state, &reg, "Giant Growth", P1);

    let mut state = activate(&state, &reg, weevil, 0, vec![Target::Player(P1)]);

    for expected_left in [3usize, 2] {
        let Some(mtg_engine::state::AwaitingAction::ResolutionChoice { player, choice, .. }) =
            &state.awaiting_action else {
            panic!("expected a discard prompt with {expected_left} cards in hand; \
                    got {:?}", state.awaiting_action);
        };
        assert_eq!(*player, P1, "the discarding player chooses, not the activator");
        let mtg_engine::state::ResolutionChoiceKind::ChooseCardFromHand { cards, .. } = choice else {
            panic!("expected a hand choice, got {choice:?}");
        };
        assert_eq!(cards.len(), expected_left,
            "the second prompt is against the hand as it stands after the first");
        let pick = cards[0];
        state = engine::submit_action(&state, &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenCard(pick),
        }, &reg);
    }

    let left: Vec<_> = state.objects_in_zone(Zone::Hand, P1).iter().map(|o| o.id).collect();
    assert_eq!(left.len(), 1, "two of the three went");
    assert!([a, b, c].contains(&left[0]));
    assert!(state.awaiting_action.is_none(), "and nothing is still being asked");
}

#[test]
fn disciple_of_griselbrand_gains_life() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let disciple = named_permanent(&mut state, &reg, "Disciple of Griselbrand", P0);
    // Create a 2/5 creature to sacrifice for max life.
    let fatty = ready_creature(&mut state, P0, 2, 5);

    let life_before = state.get_player(P0).life;

    // Add mana for the ability: {1}
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Player explicitly chooses to sacrifice the fatty (5 toughness → 5 life).
    let new_state = activate_sacrificing(&state, &reg, disciple, 0, vec![], fatty);

    // Should gain exactly 5 life (the fatty's toughness).
    let life_after = new_state.get_player(P0).life;
    assert_eq!(life_after - life_before, 5,
        "Should have gained 5 life from sacrificing the 2/5 fatty");
    assert_eq!(new_state.get_object(fatty).unwrap().zone, Zone::Graveyard,
        "fatty should have been sacrificed");
    assert_eq!(new_state.get_object(disciple).unwrap().zone, Zone::Battlefield,
        "disciple should still be on the battlefield");
}
#[test]
fn altars_reap_sacrifices_and_draws_two() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P0 a creature to sacrifice and cards to draw.
    let creature = ready_creature(&mut state, P0, 2, 2);
    for _ in 0..3 {
        let c = state.create_object(mtg_engine::ids::CardId(9999), P0, Zone::Library, None, None);
        state.get_player_mut(P0).library_order.push(c);
    }

    let spell = castable_spell(&mut state, &reg, "Altar's Reap", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    // Creature should be in graveyard (sacrificed).
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "Sacrificed creature should be in graveyard");

    // Should have drawn 2 cards (hand was empty before spell was added).
    let hand_after = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();
    assert_eq!(hand_after, 2,
        "Should have drawn 2 cards");
}

/// Ruling: "You must sacrifice exactly one creature to cast this spell; you
/// cannot cast it without sacrificing a creature." With no creature the cost
/// is unpayable, so the cast is not offered at all (CR 601.2h) — and a
/// submitted cast is refused with everything intact.
#[test]
fn altars_reap_cannot_be_cast_without_a_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let spell = castable_spell(&mut state, &reg, "Altar's Reap", P0);
    let mana_before = state.get_player(P0).mana_pool.clone();

    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    assert!(!legal.actions.iter().any(|a|
        matches!(a, Action::CastSpell { object_id, .. } if *object_id == spell)),
        "no creature to sacrifice, so the cast is not offered");

    // Submitted anyway, nothing happens and nothing is paid.
    let state = cast_onto_stack(&state, &reg, spell, vec![]);
    assert_eq!(state.get_object(spell).unwrap().zone, Zone::Hand,
        "the spell never left the hand");
    assert_eq!(state.get_player(P0).mana_pool, mana_before, "and no mana was paid");
}

/// Ruling: "No one can try to destroy the creature you sacrificed to prevent
/// you from casting this spell" — the sacrifice is part of paying the cost
/// (CR 601.2h), so by the first moment anyone could respond, the creature is
/// already in the graveyard and the spell already on the stack.
#[test]
fn altars_reap_sacrifice_happens_with_the_cast_not_the_resolution() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let spell = castable_spell(&mut state, &reg, "Altar's Reap", P0);
    let state = cast_onto_stack(&state, &reg, spell, vec![]);

    assert_eq!(state.get_object(spell).unwrap().zone, Zone::Stack,
        "the spell is on the stack, unresolved");
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "and the creature is already gone — the cost was paid at announcement");
}
#[test]
fn infernal_plunge_sacrifices_and_adds_rrr() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P0 a creature to sacrifice.
    let creature = ready_creature(&mut state, P0, 1, 1);

    let spell = castable_spell(&mut state, &reg, "Infernal Plunge", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    // Creature should be sacrificed.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "Sacrificed creature should be in graveyard");

    // Should have 3 red mana in pool (the {R} used to cast was consumed, then {R}{R}{R} added).
    assert_eq!(state.get_player(P0).mana_pool.get(ManaType::Red), 3,
        "Should have 3 red mana in pool after Infernal Plunge");
}
#[test]
fn tribute_to_hunger_opponent_sacs_and_gain_life() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give the opponent a 3/4 creature.
    let opp_creature = ready_creature(&mut state, P1, 3, 4);
    // Set the creature's name for logging.
    state.get_object_mut(opp_creature).unwrap().name = "Big Beast".into();

    let initial_life = state.get_player(P0).life;

    let spell = castable_spell(&mut state, &reg, "Tribute to Hunger", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![mtg_engine::actions::Target::Player(P1)]);

    // Opponent's creature should be sacrificed.
    assert_eq!(state.get_object(opp_creature).unwrap().zone, Zone::Graveyard,
        "Opponent's creature should be sacrificed");

    // Caster should have gained life equal to toughness (4).
    assert_eq!(state.get_player(P0).life, initial_life + 4,
        "Should have gained life equal to sacrificed creature's toughness");
}
/// "Target opponent sacrifices a creature **of their choice**." The choice is
/// the opponent's, not the caster's, and with more than one creature it has to
/// actually be presented to them. The existing test gives the opponent one
/// creature, where there is nothing to choose and the prompt never appears.
#[test]
fn tribute_to_hunger_lets_the_opponent_pick_which_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let small = ready_creature(&mut state, P1, 1, 1);
    let big = ready_creature(&mut state, P1, 5, 5);
    let life = state.get_player(P0).life;

    let spell = castable_spell(&mut state, &reg, "Tribute to Hunger", P0);
    let mut state = cast_onto_stack(&state, &reg, spell, vec![Target::Player(P1)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    let Some(mtg_engine::state::AwaitingAction::ResolutionChoice { player, .. }) = &state.awaiting_action else {
        panic!("the opponent has two creatures, so they must be asked which to \
                sacrifice; got {:?}", state.awaiting_action);
    };
    assert_eq!(*player, P1, "the choice belongs to the opponent, not the caster");

    let options = pending_choice_options(&state);
    assert!(options.contains(&Target::Object(small)) && options.contains(&Target::Object(big)),
        "both of the opponent's creatures are choosable: {options:?}");

    // They keep the big one.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(small))),
    }, &reg);

    assert_eq!(state.get_object(small).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(big).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_player(P0).life, life + 1,
        "life equal to the toughness of the creature they actually chose");
}

/// Ruling 2024-11-08: "Use the sacrificed creature's toughness as it last
/// existed on the battlefield to determine how much life to gain."
///
/// So the number is its toughness *including* whatever was modifying it, read
/// before it leaves — not its printed toughness, and not zero because it is
/// already in the graveyard by the time the life is added.
#[test]
fn tribute_to_hunger_gains_life_for_the_toughness_it_last_had() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 2, 2);
    state.add_counters(creature, CounterType::PlusOnePlusOne, 3);
    assert_eq!(state.effective_toughness(creature, &reg), Some(5), "test setup");
    let life = state.get_player(P0).life;

    let spell = castable_spell(&mut state, &reg, "Tribute to Hunger", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Player(P1)]);

    assert_eq!(state.get_player(P0).life, life + 5,
        "5, the toughness it last had on the battlefield — not its printed 2");
}

/// A sacrifice is not a destruction, so indestructible does not stop it
/// (CR 701.17a), and the creature does not get to regenerate.
#[test]
fn tribute_to_hunger_takes_an_indestructible_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 4, 4);
    state.get_object_mut(creature).unwrap().keywords.push(Keyword::Indestructible);

    let spell = castable_spell(&mut state, &reg, "Tribute to Hunger", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Player(P1)]);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "sacrifice is not destruction; indestructible does not apply");
}

/// The creature is chosen, not targeted — only the opponent is a target — so
/// hexproof has nothing to say about it (CR 702.11b is about targeting).
#[test]
fn tribute_to_hunger_can_take_a_hexproof_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(creature).unwrap().keywords.push(Keyword::Hexproof);

    let spell = castable_spell(&mut state, &reg, "Tribute to Hunger", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Player(P1)]);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "the creature is chosen by its controller, never targeted");
}

#[test]
fn tribute_to_hunger_no_creatures_does_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let initial_life = state.get_player(P0).life;

    let spell = castable_spell(&mut state, &reg, "Tribute to Hunger", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![mtg_engine::actions::Target::Player(P1)]);

    // No life change since no creature was sacrificed.
    assert_eq!(state.get_player(P0).life, initial_life,
        "No life gain when opponent has no creatures");
}
#[test]
fn divine_reckoning_keeps_one_per_player() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has 3 creatures: 2/1, 2/3, 1/5.
    let c1 = ready_creature(&mut state, P0, 2, 1);
    let _c2 = ready_creature(&mut state, P0, 2, 3);
    let _c3 = ready_creature(&mut state, P0, 1, 5);

    // P1 has 2 creatures: 4/2, 3/4.
    let c4 = ready_creature(&mut state, P1, 4, 2);
    let _c5 = ready_creature(&mut state, P1, 3, 4);

    let spell = castable_spell(&mut state, &reg, "Divine Reckoning", P0);
    let mut state = cast_and_resolve(&state, &reg, spell, vec![]);

    // P0 (active player) should be asked to choose first.
    assert!(state.awaiting_action.is_some(), "Should be awaiting P0's creature choice");

    // P0 chooses to keep c1 (the 2/1).
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(c1))),
        },
        &reg,
    );

    // P1 should now be asked to choose.
    assert!(state.awaiting_action.is_some(), "Should be awaiting P1's creature choice");

    // P1 chooses to keep c4 (the 4/2).
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(c4))),
        },
        &reg,
    );

    check_state_based_actions(&mut state, &reg);

    // P0 should have exactly 1 creature left on battlefield (c1).
    let p0_creatures: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == P0 && o.power.is_some())
        .collect();
    assert_eq!(p0_creatures.len(), 1, "P0 should have exactly 1 creature");
    assert_eq!(p0_creatures[0].id, c1, "P0 should keep the chosen creature");

    // P1 should have exactly 1 creature left on battlefield (c4).
    let p1_creatures: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == P1 && o.power.is_some())
        .collect();
    assert_eq!(p1_creatures.len(), 1, "P1 should have exactly 1 creature");
    assert_eq!(p1_creatures[0].id, c4, "P1 should keep the chosen creature");
}
#[test]
fn divine_reckoning_with_one_creature_keeps_it() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has 1 creature.
    let c1 = ready_creature(&mut state, P0, 3, 3);
    // P1 has 0 creatures.

    let spell = castable_spell(&mut state, &reg, "Divine Reckoning", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    // P0's single creature should still be on the battlefield.
    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Battlefield,
        "Single creature should survive Divine Reckoning");

    // P1 should have no creatures (had none to begin with).
    let p1_creatures = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == P1 && o.power.is_some())
        .count();
    assert_eq!(p1_creatures, 0, "P1 should have no creatures");
}
#[test]
fn skirsdag_cultist_deals_2_damage_to_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cultist = named_permanent(&mut state, &reg, "Skirsdag Cultist", P0);
    // Need a creature to sacrifice (player picks the fodder, not the cultist).
    let fodder = ready_creature(&mut state, P0, 1, 1);
    let target = ready_creature(&mut state, P1, 3, 3);

    // Add red mana for the activation cost.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let state = activate_sacrificing(&state, &reg, cultist, 0, vec![Target::Object(target)], fodder);

    // Target creature should have taken 2 damage.
    let obj = state.get_object(target).unwrap();
    assert_eq!(obj.damage_marked, 2, "Target should have 2 damage marked");
    // Cultist should still be alive (we sacrificed the fodder).
    assert_eq!(state.get_object(cultist).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(fodder).unwrap().zone, Zone::Graveyard);
}
#[test]
fn skirsdag_cultist_deals_2_damage_to_player() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cultist = named_permanent(&mut state, &reg, "Skirsdag Cultist", P0);
    let fodder = ready_creature(&mut state, P0, 1, 1);

    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let state = activate_sacrificing(&state, &reg, cultist, 0, vec![Target::Player(P1)], fodder);

    assert_eq!(state.get_player(P1).life, 18, "Opponent should be at 18 life");
    assert_eq!(state.get_object(fodder).unwrap().zone, Zone::Graveyard);

    // CR 510.1: an ability's damage is not combat damage, whatever the source
    // is. Which event is emitted decides whether every "whenever ~ deals combat
    // damage" trigger in the set fires, and nothing checked it.
    assert!(state.events.iter().any(|e| matches!(e,
        mtg_engine::events::GameEvent::NonCombatDamageDealt { .. })),
        "the ability's damage is non-combat damage");
    assert!(!state.events.iter().any(|e| matches!(e,
        mtg_engine::events::GameEvent::CombatDamageDealt { .. })),
        "and emphatically not combat damage — nobody was in combat");
}
#[test]
fn skirsdag_cultist_cannot_activate_without_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Cultist is the only creature. It will be sacrificed as part of the cost,
    // but we need at least one creature to sacrifice. Since the cultist itself
    // counts, the ability should still be available.
    let _cultist = named_permanent(&mut state, &reg, "Skirsdag Cultist", P0);

    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let has_activate = actions.actions.iter().any(|a| matches!(a, Action::ActivateAbility { .. }));
    assert!(has_activate, "Should be able to activate (cultist counts as sacrifice fodder)");
}
/// "{1}{U}, {T}: Create a 2/2 blue Homunculus creature token, then sacrifice a
/// creature."
///
/// Ruling (2018-12-07): "The creature you sacrifice for the ability of
/// Stitcher's Apprentice could be the Homunculus you've just created. It could
/// also be Stitcher's Apprentice itself."
///
/// So the offered list is the whole claim: both of those, and — CR 701.16b, you
/// can only sacrifice what you control — nothing of the opponent's. Offering
/// every creature on the battlefield instead passed the whole suite.
#[test]
fn stitchers_apprentice_offers_every_creature_you_control_and_only_those() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let apprentice = named_permanent(&mut state, &reg, "Stitcher's Apprentice", P0);
    let theirs = ready_creature(&mut state, P1, 3, 3);

    add_mana(&mut state, P0, &[(ManaType::Blue, 1), (ManaType::Colorless, 1)]);
    let state = activate(&state, &reg, apprentice, 0, vec![]);

    let token = state.objects.values()
        .find(|o| o.zone == Zone::Battlefield && o.is_token)
        .map(|o| o.id)
        .expect("the token is created before the sacrifice is chosen");

    let options = match &state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            player, choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. }, ..
        }) => {
            assert_eq!(*player, P0, "the ability's controller chooses");
            options.clone()
        }
        other => panic!("a creature has to be chosen to sacrifice, got {other:?}"),
    };

    assert!(options.contains(&Target::Object(token)),
        "the Homunculus it just created is one of them");
    assert!(options.contains(&Target::Object(apprentice)),
        "and so is Stitcher's Apprentice itself");
    assert!(!options.contains(&Target::Object(theirs)),
        "CR 701.16b: you sacrifice only what you control");

    // "then sacrifice a creature" — not "you may". Declining is not on offer.
    let actions = mtg_engine::engine::legal_actions(&state, &reg).actions;
    assert!(!actions.iter().any(|a| matches!(a,
        Action::ResolveChoice { choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(None) })),
        "the sacrifice is mandatory, so there is nothing to decline; got {actions:?}");
}

/// The other half: choosing one actually sacrifices it.
#[test]
fn stitchers_apprentice_creates_token_then_sacrifices() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let apprentice = named_permanent(&mut state, &reg, "Stitcher's Apprentice", P0);

    add_mana(&mut state, P0, &[(ManaType::Blue, 1), (ManaType::Colorless, 1)]);

    let creatures_before = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
        .count();
    assert_eq!(creatures_before, 1, "only the apprentice on the battlefield");

    let state = activate(&state, &reg, apprentice, 0, vec![]);
    assert!(state.awaiting_action.is_some(), "a creature has to be chosen");

    let token_id = state.objects.values()
        .find(|o| o.zone == Zone::Battlefield && o.is_token && o.power.is_some())
        .map(|o| o.id)
        .expect("Token should exist");

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(token_id))),
        },
        &reg,
    );

    assert_eq!(state.get_object(token_id).unwrap().zone, Zone::Graveyard,
        "the chosen creature is the one sacrificed");
    let creatures_after = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
        .count();
    assert_eq!(creatures_after, 1, "create + sacrifice leaves the count where it started");
}

/// "a 2/2 **blue** Homunculus creature token" — read through the accessors, so
/// this is the token's characteristics and not the raw fields it happened to be
/// built with. Its colour had no assertion at all: making it black passed the
/// whole suite.
#[test]
fn stitchers_apprentice_token_is_a_two_two_blue_homunculus() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let apprentice = named_permanent(&mut state, &reg, "Stitcher's Apprentice", P0);
    // Something else to sacrifice, so the token survives to be inspected.
    let fodder = ready_creature(&mut state, P0, 1, 1);

    add_mana(&mut state, P0, &[(ManaType::Blue, 1), (ManaType::Colorless, 1)]);
    let state = activate(&state, &reg, apprentice, 0, vec![]);

    let token = state.objects.values()
        .find(|o| o.zone == Zone::Battlefield && o.is_token && o.power.is_some())
        .map(|o| o.id)
        .expect("a token should exist on the battlefield");
    assert_ne!(token, fodder, "test setup: the fodder is not a token");

    assert_eq!(state.effective_power(token, &reg), Some(2));
    assert_eq!(state.effective_toughness(token, &reg), Some(2));
    assert!(state.colors_of(token, &reg).contains(&Color::Blue), "blue");
    assert!(state.has_subtype(token, "Homunculus", &reg), "a Homunculus");
    assert!(state.is_creature(token, &reg), "a creature token");
    assert_eq!(state.get_object(token).unwrap().name, "Homunculus Token",
        "CR 111.4: an unnamed token is its subtypes plus \"Token\"");
}

#[test]
fn corpse_lunge_deals_damage_equal_to_exiled_power() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a 4/4 creature in P0's graveyard.
    let gy_creature = ready_creature(&mut state, P0, 4, 4);
    state.get_object_mut(gy_creature).unwrap().name = "Big Creature".into();
    state.move_object(gy_creature, Zone::Graveyard, &reg);

    // Target creature on P1's battlefield.
    let target = ready_creature(&mut state, P1, 5, 5);

    // Cast Corpse Lunge.
    let spell = castable_spell(&mut state, &reg, "Corpse Lunge", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);

    // The graveyard creature should be in exile.
    let exiled = state.get_object(gy_creature).unwrap();
    assert_eq!(exiled.zone, Zone::Exile, "Graveyard creature should be exiled");

    // Target creature should have 4 damage.
    let target_obj = state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 4, "Target should have 4 damage from Corpse Lunge");
}
/// "damage equal to **the exiled card's** power" — the card's power where it
/// is, which is exile, and read when the spell resolves.
///
/// Boneyard Wurm is the card that can tell the difference: "power and toughness
/// are each equal to the number of creature cards in your graveyard", and a
/// characteristic-defining ability functions in every zone (CR 604.3). Exiling
/// the Wurm to pay the cost takes it *out* of the graveyard, so it stops
/// counting itself: with two other creature cards down there it is a 2/2 in
/// exile, not the 3/3 it was in the graveyard a moment earlier.
#[test]
fn corpse_lunge_reads_the_exiled_cards_power_where_it_now_is() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wurm = named_card_in_graveyard(&mut state, &reg, "Boneyard Wurm", P0);
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    assert_eq!(state.effective_power(wurm, &reg), Some(3),
        "test setup: in the graveyard the Wurm counts itself and the other two");

    let target = ready_creature(&mut state, P1, 5, 9);
    let spell = castable_spell(&mut state, &reg, "Corpse Lunge", P0);
    let state = cast_and_resolve(&state, &reg, spell,
        vec![Target::Object(target)]);

    assert_eq!(state.get_object(wurm).unwrap().zone, Zone::Exile,
        "test setup: the Wurm is the card that paid the cost");
    assert_eq!(state.effective_power(wurm, &reg), Some(2),
        "out of the graveyard, the Wurm counts only the two cards still in it");
    assert_eq!(state.get_object(target).unwrap().damage_marked, 2,
        "and that is the power the spell deals");
}

/// The limit case of the same rule: a Boneyard Wurm that is the *only* creature
/// card in the graveyard pays the cost by leaving it, and is a 0/0 in exile
/// with nothing left to count. Corpse Lunge deals no damage at all — and 0
/// damage is not damage (CR 120.8), so nothing is marked.
#[test]
fn corpse_lunge_exiling_a_lone_boneyard_wurm_deals_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wurm = named_card_in_graveyard(&mut state, &reg, "Boneyard Wurm", P0);
    assert_eq!(state.effective_power(wurm, &reg), Some(1),
        "test setup: in the graveyard the Wurm counts itself");

    let target = ready_creature(&mut state, P1, 3, 3);
    let spell = castable_spell(&mut state, &reg, "Corpse Lunge", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);

    assert_eq!(state.get_object(wurm).unwrap().zone, Zone::Exile,
        "the Wurm paid the additional cost");
    assert_eq!(state.effective_power(wurm, &reg), Some(0),
        "and out of the graveyard there is nothing left for it to count");
    assert_eq!(state.get_object(target).unwrap().damage_marked, 0);
}

/// The other half of the same sentence: "the exiled card's power" is a value
/// the spell reads as it resolves, not one it carried up from the cast. Corpse
/// Lunge is an instant, so there is a priority window between the two — a
/// creature card reaching the graveyard in it raises the exiled Wurm's power,
/// and the damage with it.
#[test]
fn corpse_lunge_reads_the_exiled_cards_power_when_it_resolves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Three creature cards down, so the Wurm is a 3/3 and the strongest thing
    // in the graveyard — which is what the cost picks.
    let wurm = named_card_in_graveyard(&mut state, &reg, "Boneyard Wurm", P0);
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    // Two latecomers, not one: with one, "read in the graveyard at cast time"
    // and "read in exile at resolution" both come to 3, and the test would not
    // be able to tell them apart.
    let latecomers = [
        named_permanent(&mut state, &reg, "Walking Corpse", P0),
        named_permanent(&mut state, &reg, "Grizzly Bears", P0),
    ];

    let target = ready_creature(&mut state, P1, 5, 9);
    let spell = castable_spell(&mut state, &reg, "Corpse Lunge", P0);
    let state = cast_onto_stack(&state, &reg, spell, vec![Target::Object(target)]);
    let mut state = resolve_exile_choice_max_power(&state, &reg);
    assert_eq!(state.get_object(wurm).unwrap().zone, Zone::Exile,
        "test setup: the cost is paid as the spell is cast (CR 601.2f)");
    assert_eq!(state.effective_power(wurm, &reg), Some(2),
        "test setup: two creature cards left in the graveyard, and the spell \
         is still on the stack");

    // In response, two more creature cards reach the graveyard.
    for id in latecomers {
        state.move_object(id, Zone::Graveyard, &reg);
    }
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(target).unwrap().damage_marked, 4,
        "the Wurm was a 2/2 in exile when the spell went on the stack and a \
         4/4 when it resolved, and the spell deals what it reads on resolution \
         (3 would be its power back in the graveyard as the cost was paid)");
}

/// Ruling: "You must exile exactly one creature card from your graveyard to
/// cast this spell; you cannot cast it without exiling a creature card."
///
/// So the answer to an empty graveyard is that the spell is not castable, not
/// that it is castable and deals nothing. This used to force a `CastSpell`
/// action past `legal_actions` — one the engine never offers — and pin the
/// damage such an illegal cast happens to do at 0. A test standing on a state
/// the engine cannot produce can pass while the rule it claims to cover is
/// broken; here the rule is the castability, which nothing checked.
#[test]
fn corpse_lunge_cannot_be_cast_without_a_creature_card_to_exile() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    ready_creature(&mut state, P1, 3, 3);
    let spell = castable_spell(&mut state, &reg, "Corpse Lunge", P0);

    // A noncreature card in the graveyard is not fuel.
    named_card_in_graveyard(&mut state, &reg, "Forest", P0);
    assert!(!can_cast(&state, &reg, spell),
        "the additional cost cannot be paid, so the spell cannot be cast");

    named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    assert!(can_cast(&state, &reg, spell),
        "and with a creature card down there it can");
}
#[test]
fn corpse_lunge_picks_highest_power_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put two creatures in graveyard: a 2/2 and a 5/5.
    let small = ready_creature(&mut state, P0, 2, 2);
    state.move_object(small, Zone::Graveyard, &reg);
    let big = ready_creature(&mut state, P0, 5, 5);
    state.move_object(big, Zone::Graveyard, &reg);

    let target = ready_creature(&mut state, P1, 6, 6);

    let spell = castable_spell(&mut state, &reg, "Corpse Lunge", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);

    // Should exile the 5/5 and deal 5 damage.
    let big_obj = state.get_object(big).unwrap();
    assert_eq!(big_obj.zone, Zone::Exile, "Highest-power creature should be exiled");

    let target_obj = state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, 5, "Should deal 5 damage (power of exiled 5/5)");
}
/// Harvest Pyre: "Exile X cards from your graveyard: this deals X damage to
/// target creature." Four tests walked one X each; X is the whole card, so it
/// is a table.
#[test]
fn harvest_pyre_exiles_x_of_your_own_cards_and_deals_x() {
    // (cards in your graveyard, cards in the opponent's, X chosen)
    const CASES: &[(usize, usize, u32)] = &[
        (4, 0, 4),  // exile everything
        (4, 0, 2),  // exile some — the rest stay
        (3, 0, 0),  // X=0 is legal and does nothing
        (3, 2, 3),  // "your graveyard": the opponent's cards are not touched
    ];
    let reg = registry();
    for &(mine, theirs, x) in CASES {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        for _ in 0..mine {
            let c = state.create_object(CardId(9999), P0, Zone::Battlefield, Some(1), Some(1));
            state.move_object(c, Zone::Graveyard, &reg);
        }
        for _ in 0..theirs {
            let c = state.create_object(CardId(9999), P1, Zone::Battlefield, Some(1), Some(1));
            state.move_object(c, Zone::Graveyard, &reg);
        }
        let target = ready_creature(&mut state, P1, 6, 6);

        let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);
        let mut state = engine::submit_action(
            &state,
            &Action::CastSpell {
                object_id: spell,
                targets: vec![Target::Object(target)],
                sacrifice: None,
                exile_count: Some(x),
                exile_ids: vec![],
                alternative_cost: None,
                tap_plan: vec![],
            },
            &reg,
        );
        mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

        let exiled = state.objects.values()
            .filter(|o| o.zone == Zone::Exile && o.owner == P0)
            .count();
        assert_eq!(exiled, x as usize, "X={x} should exile {x} of your own cards");

        // Your remaining cards, plus Harvest Pyre itself once it has resolved.
        let left = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == P0)
            .count();
        assert_eq!(left, mine - x as usize + 1,
            "X={x} out of {mine} should leave {} of yours, plus Harvest Pyre",
            mine - x as usize);

        let theirs_left = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == P1)
            .count();
        assert_eq!(theirs_left, theirs, "the opponent's graveyard is never touched");

        assert_eq!(state.get_object(target).unwrap().damage_marked, x,
            "X={x} should deal {x} damage");
    }
}

#[test]
fn harvest_pyre_legal_actions_emits_single_cast_per_target() {
    // With the ChooseExileFromGraveyard refactor, the engine emits ONE
    // CastSpell per target for Harvest Pyre (not 2^gy_size entries per
    // target — the old subset enumeration). The player picks which
    // cards to exile via the resolution prompt.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    for _ in 0..3 {
        let c = state.create_object(CardId(9999), P0, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard, &reg);
    }

    let _target = ready_creature(&mut state, P1, 3, 3);

    let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let actions = engine::legal_actions(&state, &reg);
    let harvest_actions: Vec<_> = actions.actions.iter()
        .filter(|a| {
            if let Action::CastSpell { object_id, .. } = a { *object_id == spell } else { false }
        })
        .collect();

    // One target × one cast action = 1 (no subset enumeration).
    assert_eq!(harvest_actions.len(), 1,
        "Should have exactly 1 cast action (one per target; exile choice \
         happens via the ChooseExileFromGraveyard prompt), but got {}",
        harvest_actions.len());
}
// ── Bug H: CastableSpell.exile_x_from_gy_max ─────────────────────────
//
// Regression tests for BUG_REPORT_8SEAT.md Bug H. The LLM player was
// always casting Harvest Pyre with exile_count: None (→ X=0) because
// `choose_cast_targets` constructed a fresh CastSpell action instead
// of looking up one of the engine's pre-enumerated expanded variants,
// and the action label gave no hint at the effective X. The fix plumbs
// `exile_x_from_gy_max` through CastableSpell so the LLM UI can both
// display the damage in the label and find the matching expanded
// action. These tests pin the engine side of that contract.

#[test]
fn castable_spell_exposes_exile_x_from_gy_max_for_harvest_pyre() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    // Put 5 cards in P0's graveyard. exile_x_from_gy_max should report 5.
    for _ in 0..5 {
        let c = state.create_object(CardId(9999), P0, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard, &reg);
    }
    // P1 graveyard cards must NOT count — exile_count is "your graveyard".
    for _ in 0..3 {
        let c = state.create_object(CardId(9999), P1, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard, &reg);
    }

    let _target = ready_creature(&mut state, P1, 6, 6);
    let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);

    let legal = engine::legal_actions(&state, &reg);
    let cs = legal.castable_spells.iter()
        .find(|cs| cs.object_id == spell)
        .expect("Harvest Pyre should be castable");

    assert_eq!(cs.exile_x_from_gy_max, Some(5),
        "exile_x_from_gy_max should equal the caster's own graveyard size");
}

#[test]
fn castable_spell_exile_x_from_gy_max_is_none_for_non_exile_x_spell() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    let _target = ready_creature(&mut state, P1, 2, 2);
    let spell = castable_spell(&mut state, &reg, "Lightning Bolt", P0);

    let legal = engine::legal_actions(&state, &reg);
    let cs = legal.castable_spells.iter()
        .find(|cs| cs.object_id == spell)
        .expect("Lightning Bolt should be castable");

    assert!(cs.exile_x_from_gy_max.is_none(),
        "Spells without ExileXFromGraveyard must report None, got {:?}",
        cs.exile_x_from_gy_max);
}

#[test]
fn castable_spell_exile_x_from_gy_max_reports_zero_when_graveyard_empty() {
    // Empty graveyard → max X is zero, NOT None. Distinguishes "no exile
    // cost" from "exile cost with nothing to pay". The LLM label then
    // renders "X=0 (0 damage)" so the model doesn't waste the card.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    let _target = ready_creature(&mut state, P1, 2, 2);
    let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);

    let legal = engine::legal_actions(&state, &reg);
    let cs = legal.castable_spells.iter()
        .find(|cs| cs.object_id == spell)
        .expect("Harvest Pyre is castable even with an empty graveyard");

    assert_eq!(cs.exile_x_from_gy_max, Some(0),
        "empty graveyard must surface as Some(0), not None");
}

#[test]
fn harvest_pyre_max_x_cast_deals_full_damage_via_exile_prompt() {
    // Integration check for the new flow: cast Harvest Pyre, then
    // resolve the ChooseExileFromGraveyard prompt by exiling every
    // graveyard card (max X). Damage should equal graveyard size.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    for _ in 0..3 {
        let c = state.create_object(CardId(9999), P0, Zone::Battlefield, Some(1), Some(1));
        state.move_object(c, Zone::Graveyard, &reg);
    }

    let target = ready_creature(&mut state, P1, 4, 4);
    let spell = castable_spell(&mut state, &reg, "Harvest Pyre", P0);

    let legal = engine::legal_actions(&state, &reg);
    let cs = legal.castable_spells.iter()
        .find(|cs| cs.object_id == spell)
        .expect("Harvest Pyre castable");
    let max_x = cs.exile_x_from_gy_max.expect("max X should be Some(3)");
    assert_eq!(max_x, 3);

    // cast_and_resolve picks the max-power (= max count for Harvest Pyre)
    // exile subset, then resolves the top of the stack.
    let new_state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);

    let target_obj = new_state.get_object(target).unwrap();
    assert_eq!(target_obj.damage_marked, max_x,
        "exiling max_x cards should deal max_x damage (={max_x})");
}

// -------------------------------------------------------------------------
// Infernal Plunge
// -------------------------------------------------------------------------

/// Cannot cast Infernal Plunge without a creature to sacrifice.
#[test]
fn cannot_cast_without_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _plunge = castable_spell(&mut state, &reg, "Infernal Plunge", P0);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let can_cast = actions.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if {
            state.get_object(*object_id)
                .is_some_and(|o| o.name == "Infernal Plunge")
        })
    });

    assert!(!can_cast,
        "Should not be able to cast Infernal Plunge without a creature to sacrifice");
}

/// Can cast Infernal Plunge when controlling a creature.
#[test]
fn can_cast_with_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _plunge = castable_spell(&mut state, &reg, "Infernal Plunge", P0);
    let _creature = ready_creature(&mut state, P0, 1, 1);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let can_cast = actions.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if {
            state.get_object(*object_id)
                .is_some_and(|o| o.name == "Infernal Plunge")
        })
    });

    assert!(can_cast,
        "Should be able to cast Infernal Plunge when controlling a creature");
}

/// Sacrifice happens at cast time (creature is gone before resolution).
#[test]
fn sacrifice_at_cast_time() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let plunge = castable_spell(&mut state, &reg, "Infernal Plunge", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    // Cast with explicit sacrifice target.
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell {
            object_id: plunge,
            targets: vec![],
            sacrifice: Some(creature),
            exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );

    // Creature should already be in graveyard (sacrificed at cast time).
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard,
        "Creature should be sacrificed at cast time, not resolution");

    // Spell should be on the stack.
    assert_eq!(state.get_object(plunge).unwrap().zone, Zone::Stack,
        "Infernal Plunge should be on the stack");
}

/// On resolution, adds {R}{R}{R} to mana pool.
#[test]
fn adds_three_red_mana() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let plunge = castable_spell(&mut state, &reg, "Infernal Plunge", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);

    // Cast with explicit sacrifice target.
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell {
            object_id: plunge,
            targets: vec![],
            sacrifice: Some(creature),
            exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );

    // Record mana before resolution (should be 0 since we spent {R} to cast).
    let red_before = state.get_player(P0).mana_pool.get(ManaType::Red);

    // Resolve the spell.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    let red_after = state.get_player(P0).mana_pool.get(ManaType::Red);
    assert_eq!(red_after - red_before, 3,
        "Infernal Plunge should add RRR on resolution");

    // The addition is announced like any other mana source's (CR 106.4) —
    // a spell adding mana must not bypass the ManaAdded event.
    assert!(state.events.iter().any(|e| matches!(e,
        mtg_engine::events::GameEvent::ManaAdded { player: p, mana_type: ManaType::Red, amount: 3 }
            if *p == P0)),
        "Infernal Plunge's mana should be announced with a ManaAdded event");
}

/// Legal actions show one `CastSpell` per eligible creature to sacrifice.
#[test]
fn one_action_per_sacrifice_target() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _plunge = castable_spell(&mut state, &reg, "Infernal Plunge", P0);
    let creature_a = ready_creature(&mut state, P0, 1, 1);
    let creature_b = ready_creature(&mut state, P0, 3, 3);

    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let plunge_actions: Vec<_> = actions.actions.iter().filter(|a| {
        if let Action::CastSpell { object_id, sacrifice, .. } = a {
            state.get_object(*object_id)
                .is_some_and(|o| o.name == "Infernal Plunge")
            && sacrifice.is_some()
        } else {
            false
        }
    }).collect();

    assert_eq!(plunge_actions.len(), 2,
        "Should have one CastSpell action per eligible creature (got {})", plunge_actions.len());

    // Both creatures should be represented as sacrifice options.
    let sac_ids: Vec<_> = plunge_actions.iter().filter_map(|a| {
        if let Action::CastSpell { sacrifice, .. } = a {
            *sacrifice
        } else {
            None
        }
    }).collect();
    assert!(sac_ids.contains(&creature_a), "Should include creature A as sacrifice option");
    assert!(sac_ids.contains(&creature_b), "Should include creature B as sacrifice option");
}

/// The creature list a player picks from is offered in a stable order.
///
/// It used to be built straight off `state.objects.values()`, a HashMap
/// iterator, so the order varied between runs of the same game. The player
/// picks from this list by position, so an unstable order means the same
/// decisions replay differently.
#[test]
fn divine_reckonings_choice_list_is_in_a_stable_order() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Several creatures for P0, so there is a real list to order.
    let mut mine: Vec<ObjectId> = (0..5).map(|_| ready_creature(&mut state, P0, 2, 2)).collect();
    mine.sort_by_key(|id| id.0);
    // And one for the opponent, which must not appear in P0's list.
    let theirs = ready_creature(&mut state, P1, 2, 2);

    let spell = castable_spell(&mut state, &reg, "Divine Reckoning", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    let options = match &state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. }, .. })
            => options.clone(),
        _ => panic!("expected P0 to be choosing a creature to keep"),
    };

    let offered: Vec<ObjectId> = options.iter()
        .filter_map(|t| match t { Target::Object(id) => Some(*id), _ => None })
        .collect();

    assert_eq!(offered, mine, "offered in ascending object-id order");
    assert!(!offered.contains(&theirs), "and only creatures this player controls");
}
