//! Creatures and lands whose behaviour is an activated ability (CR 602).
//!
//! Cards covered (6), so this is greppable by name as well as by rule:
//!
//! - Elder of Laurels
//! - Gavony Township
//! - Mindshrieker
//! - Nephalia Drownyard
//! - Skirsdag High Priest
//! - Stensia Bloodhall

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::ObjectId;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::state::GameState;
use mtg_engine::types::*;
// ══════════════════════════════════════════════════════════════════
// Elder of Laurels
// ══════════════════════════════════════════════════════════════════

/// Elder of Laurels gives +X/+X where X = number of creatures you control.
#[test]
fn elder_of_laurels_pumps_by_creature_count() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let elder = named_permanent(&mut state, &reg, "Elder of Laurels", P0);
    let target = ready_creature(&mut state, P0, 2, 2);
    let _extra = ready_creature(&mut state, P0, 1, 1);
    // P0 controls 3 creatures: elder, target, extra. So X = 3.

    add_mana(&mut state, P0, &[(ManaType::Colorless, 3), (ManaType::Green, 1)]);
    state = activate_offered(&state, &reg, elder, Some(Target::Object(target)));

    // Target should get +3/+3 (3 creatures controlled).
    assert_eq!(state.effective_power(target, &reg), Some(5));
    assert_eq!(state.effective_toughness(target, &reg), Some(5));
}

/// Put the Elder's ability on the stack without resolving it, so the board can
/// change underneath it.
fn activate_without_resolving(
    state: &GameState,
    reg: &CardRegistry,
    elder: ObjectId,
    target: ObjectId,
) -> GameState {
    let legal = mtg_engine::engine::legal_actions(state, reg);
    let action = legal.actions.iter()
        .find(|a| matches!(a, Action::ActivateAbility { object_id: o, targets, .. }
            if *o == elder && targets.contains(&Target::Object(target))))
        .expect("the Elder's ability is offered at that target")
        .clone();
    mtg_engine::engine::submit_action(state, &action, reg)
}

/// Ruling: "The number of creatures you control is counted as the ability
/// resolves."
///
/// The count moves between activation and resolution, so a test where the
/// board holds still cannot tell the two apart.
#[test]
fn elder_of_laurels_counts_creatures_when_the_ability_resolves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let elder = named_permanent(&mut state, &reg, "Elder of Laurels", P0);
    let target = ready_creature(&mut state, P0, 2, 2);
    let doomed = ready_creature(&mut state, P0, 1, 1);
    add_mana(&mut state, P0, &[(ManaType::Colorless, 3), (ManaType::Green, 1)]);

    // Three creatures when the ability is announced.
    let mut state = activate_without_resolving(&state, &reg, elder, target);

    // In response, one of them dies. Two are left when it resolves.
    mtg_engine::destruction::try_destroy(&mut state, doomed, &reg);
    check_state_based_actions(&mut state, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.effective_power(target, &reg), Some(4),
        "X is two — the count when the ability resolved, not the three there \
         were when it was announced");
}

/// Ruling: "Once the ability has resolved, the bonus won't change if the number
/// of creatures you control changes later in the turn."
///
/// The bonus is a fixed number recorded at resolution, not a live count.
#[test]
fn elder_of_laurels_bonus_does_not_follow_the_creature_count() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let elder = named_permanent(&mut state, &reg, "Elder of Laurels", P0);
    let target = ready_creature(&mut state, P0, 2, 2);
    add_mana(&mut state, P0, &[(ManaType::Colorless, 3), (ManaType::Green, 1)]);

    let mut state = activate_offered(&state, &reg, elder, Some(Target::Object(target)));
    assert_eq!(state.effective_power(target, &reg), Some(4),
        "test precondition: two creatures, so +2/+2");

    // Two more creatures arrive, and the Elder itself leaves.
    ready_creature(&mut state, P0, 1, 1);
    ready_creature(&mut state, P0, 1, 1);
    mtg_engine::destruction::try_destroy(&mut state, elder, &reg);
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.effective_power(target, &reg), Some(4),
        "the bonus was fixed when the ability resolved and does not move with \
         the board");
}

/// The Elder destroyed in response to its own ability: the ability still
/// resolves (CR 113.7a), "you" is the player who last controlled it
/// (CR 608.2g), and it is no longer one of the creatures that player controls.
#[test]
fn elder_of_laurels_killed_in_response_no_longer_counts_itself() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let elder = named_permanent(&mut state, &reg, "Elder of Laurels", P0);
    let target = ready_creature(&mut state, P0, 2, 2);
    add_mana(&mut state, P0, &[(ManaType::Colorless, 3), (ManaType::Green, 1)]);

    let mut state = activate_without_resolving(&state, &reg, elder, target);

    mtg_engine::destruction::try_destroy(&mut state, elder, &reg);
    check_state_based_actions(&mut state, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.effective_power(target, &reg), Some(3),
        "the ability still resolves, and X is one: the Elder is no longer a \
         creature its controller controls");
}

// ══════════════════════════════════════════════════════════════════
// Mindshrieker
// ══════════════════════════════════════════════════════════════════

/// "{2}: Target player mills a card. Mindshrieker gets +X/+X until end of turn,
/// where X is the mana value of that card." A land has no mana cost, so X is 0
/// — the row that shows the pump is read off the milled card rather than being
/// a constant.
#[test]
fn mindshrieker_pumps_by_the_milled_cards_mana_value() {
    // (card on top of the library, its mana value)
    const CARDS: &[(&str, i32)] = &[("Kindercatch", 6), ("Forest", 0)];

    for &(card_name, mana_value) in CARDS {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let shrieker = named_permanent(&mut state, &reg, "Mindshrieker", P0);
        let card_id = reg.get_id_by_name(card_name).unwrap();
        let lib_card = state.create_object(card_id, P1, Zone::Library, None, None);
        state.get_player_mut(P1).library_order = vec![lib_card];

        add_mana(&mut state, P0, &[(ManaType::Colorless, 2)]);
        let state = activate_offered(&state, &reg, shrieker, Some(Target::Player(P1)));

        assert_eq!(state.get_object(lib_card).unwrap().zone, Zone::Graveyard,
            "{card_name} is milled");
        assert_eq!(state.effective_power(shrieker, &reg), Some(1 + mana_value),
            "{card_name} has mana value {mana_value}, so Mindshrieker is 1+{mana_value}");
        assert_eq!(state.effective_toughness(shrieker, &reg), Some(1 + mana_value));
    }
}

// ══════════════════════════════════════════════════════════════════
// Skirsdag High Priest
// ══════════════════════════════════════════════════════════════════

/// "{T}, Tap two untapped creatures you control: Create a 5/5 black Demon
/// creature token with flying. Activate only if a creature died this turn."
///
/// Two conditions gate the ability, and the engine has to check both when
/// deciding what to offer (CR 602.2a). All three rows are needed: with only the
/// first, an ability that ignored both conditions would pass.
#[test]
fn skirsdag_high_priest_is_offered_only_with_morbid_and_two_helpers() {
    // (a creature died this turn, other untapped creatures, offered?)
    const CASES: &[(bool, usize, bool)] = &[
        (true, 2, true),
        (false, 2, false),
        (true, 1, false),
    ];

    for &(morbid, helpers, offered) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);
        state.creature_died_this_turn = morbid;

        let priest = named_permanent(&mut state, &reg, "Skirsdag High Priest", P0);
        for _ in 0..helpers {
            ready_creature(&mut state, P0, 1, 1);
        }

        assert_eq!(offers_ability_of(&state, &reg, priest), offered,
            "morbid={morbid}, {helpers} other untapped creature(s)");

        if offered {
            let state = activate_offered(&state, &reg, priest, None);
            let demons: Vec<_> = state.objects.values()
                .filter(|o| o.zone == Zone::Battlefield && o.name == "Demon Token")
                .collect();
            assert_eq!(demons.len(), 1, "one Demon token");
            assert_eq!((demons[0].power, demons[0].toughness), (Some(5), Some(5)), "a 5/5");
            assert_eq!(demons[0].colors, vec![Color::Black], "a *black* Demon");
            assert!(demons[0].subtypes.iter().any(|t| t == "Demon"), "with the Demon type");
            assert!(demons[0].keywords.contains(&Keyword::Flying), "with flying");
        }
    }
}

/// Ruling 2020-08-07: "Unlike Skirsdag High Priest itself, the two other
/// creatures you tap to activate its ability aren't required to have been
/// under your control continuously since the beginning of your most recent
/// turn."
///
/// Summoning sickness (CR 302.6) restricts the {T} *symbol* in a creature's
/// own cost. The two helpers are tapped by the Priest's cost, which is not
/// that, so a creature that arrived this turn can pay it. The Priest itself
/// still cannot — its own {T} is a {T} symbol — and that half is in
/// `tap_cost_legality.rs`.
#[test]
fn skirsdag_high_priests_helpers_may_be_summoning_sick() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    let priest = named_permanent(&mut state, &reg, "Skirsdag High Priest", P0);
    let sick_a = sick_creature(&mut state, P0, 1, 1);
    let sick_b = sick_creature(&mut state, P0, 1, 1);

    assert!(offers_ability_of(&state, &reg, priest),
        "two creatures that arrived this turn can still be tapped for the cost");

    let after = activate_offered(&state, &reg, priest, None);
    assert!(after.get_object(sick_a).unwrap().tapped);
    assert!(after.get_object(sick_b).unwrap().tapped);
    assert_eq!(count_tokens_named_by(&after, "Demon Token", P0), 1);
}

/// CR 602.2a: the ability's controller is the player who activated it. An
/// opponent who takes the Priest with the ability already on the stack does
/// not get the Demon — the activator does.
///
/// The card used to read `o.controller` off the Priest at resolution, so the
/// token followed the permanent.
#[test]
fn skirsdag_high_priests_demon_goes_to_whoever_activated_it() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    let priest = named_permanent(&mut state, &reg, "Skirsdag High Priest", P0);
    ready_creature(&mut state, P0, 1, 1);
    ready_creature(&mut state, P0, 1, 1);

    // P0 activates; the ability is on the stack.
    let mut state = activate_onto_stack(&state, &reg, priest, None);
    // P1 takes the Priest in response.
    state.get_object_mut(priest).unwrap().controller = P1;
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(count_tokens_named_by(&state, "Demon Token", P0), 1,
        "the Demon belongs to the player who activated the ability (CR 602.2a)");
    assert_eq!(count_tokens_named_by(&state, "Demon Token", P1), 0,
        "and not to whoever controls the Priest when it resolves");
}

// ══════════════════════════════════════════════════════════════════
// Gavony Township
// ══════════════════════════════════════════════════════════════════

/// Gavony Township puts a +1/+1 counter on each creature you control.
#[test]
fn gavony_township_counters_all_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let township = named_permanent(&mut state, &reg, "Gavony Township", P0);

    let c1 = ready_creature(&mut state, P0, 2, 2);
    let c2 = ready_creature(&mut state, P0, 3, 3);
    let enemy = ready_creature(&mut state, P1, 4, 4); // Opponent's creature: should not get counter.

    add_mana(&mut state, P0, &[(ManaType::Colorless, 2), (ManaType::Green, 1), (ManaType::White, 1)]);

    state = activate_offered(&state, &reg, township, None);

    // Both of P0's creatures should have +1/+1 counters.
    assert_eq!(counters_of(&state, c1, CounterType::PlusOnePlusOne), 1);
    assert_eq!(counters_of(&state, c2, CounterType::PlusOnePlusOne), 1);
    // Opponent's creature should NOT have a counter.
    assert_eq!(counters_of(&state, enemy, CounterType::PlusOnePlusOne), 0);
}

// ══════════════════════════════════════════════════════════════════
// Nephalia Drownyard
// ══════════════════════════════════════════════════════════════════

/// Nephalia Drownyard mills 3 cards from target player.
#[test]
fn nephalia_drownyard_mills_three() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let drownyard = named_permanent(&mut state, &reg, "Nephalia Drownyard", P0);

    // Put 5 cards in P1's library.
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let mut lib = Vec::new();
    for _ in 0..5 {
        let id = state.create_object(forest_id, P1, Zone::Library, None, None);
        lib.push(id);
    }
    state.players[1].library_order = lib.clone();

    add_mana(&mut state, P0, &[(ManaType::Colorless, 1), (ManaType::Blue, 1), (ManaType::Black, 1)]);

    state = activate_offered(&state, &reg, drownyard, Some(Target::Player(P1)));

    // P1 should have 2 cards left in library (5 - 3 = 2).
    assert_eq!(state.players[1].library_order.len(), 2);
    // 3 cards should be in the graveyard.
    let graveyard_count = state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P1)
        .count();
    assert_eq!(graveyard_count, 3);
}

// ══════════════════════════════════════════════════════════════════
// Stensia Bloodhall
// ══════════════════════════════════════════════════════════════════

/// Stensia Bloodhall deals 2 damage to target player.
#[test]
fn stensia_bloodhall_deals_2_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bloodhall = named_permanent(&mut state, &reg, "Stensia Bloodhall", P0);

    add_mana(&mut state, P0, &[(ManaType::Colorless, 3), (ManaType::Black, 1), (ManaType::Red, 1)]);

    state = activate_offered(&state, &reg, bloodhall, Some(Target::Player(P1)));

    assert_eq!(state.get_player(P1).life, 18, "P1 should take 2 damage (20 - 2 = 18)");
}

/// Every target the Bloodhall's ability is offered, with mana already floating.
fn bloodhall_targets(state: &GameState, reg: &CardRegistry, bloodhall: ObjectId) -> Vec<Target> {
    mtg_engine::engine::legal_actions(state, reg).actions.iter()
        .filter_map(|a| match a {
            Action::ActivateAbility { object_id, targets, .. } if *object_id == bloodhall =>
                Some(targets.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// A Bloodhall with its activation cost already floating.
fn ready_bloodhall(state: &mut GameState, reg: &CardRegistry) -> ObjectId {
    let bloodhall = named_permanent(state, reg, "Stensia Bloodhall", P0);
    add_mana(state, P0, &[(ManaType::Colorless, 3), (ManaType::Black, 1), (ManaType::Red, 1)]);
    bloodhall
}

/// "target **player or planeswalker**" — not a creature. The set's other
/// direction is covered (`damage_helper.rs`: an ability that says "another
/// target creature" cannot resolve against a planeswalker); this is the half
/// where the creature is the illegal one.
#[test]
fn stensia_bloodhall_cannot_point_at_a_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 2, 2);
    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P1);
    let bloodhall = ready_bloodhall(&mut state, &reg);

    let offered = bloodhall_targets(&state, &reg, bloodhall);
    assert!(!offered.contains(&Target::Object(creature)),
        "a creature is neither a player nor a planeswalker; offered {offered:?}");
    // Both legal kinds are offered, so the assertion above is about the
    // creature and not about the ability being unavailable.
    assert!(offered.contains(&Target::Player(P1)), "a player is; offered {offered:?}");
    assert!(offered.contains(&Target::Object(garruk)),
        "and so is a planeswalker; offered {offered:?}");
}

/// CR 702.11b: a player with hexproof can't be the target of spells or
/// abilities their opponents control. Witchbane Orb grants it, and this is the
/// set's one activated ability that targets a player.
#[test]
fn stensia_bloodhall_cannot_target_a_player_with_hexproof() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    named_permanent(&mut state, &reg, "Witchbane Orb", P1);
    let bloodhall = ready_bloodhall(&mut state, &reg);

    let offered = bloodhall_targets(&state, &reg, bloodhall);
    assert!(!offered.contains(&Target::Player(P1)),
        "P1 has hexproof from the Orb, so an opponent's ability cannot target \
         them; offered {offered:?}");
    // Hexproof stops opponents, not its own controller (CR 702.11b), and the
    // Bloodhall's controller is still a legal target for their own ability.
    assert!(offered.contains(&Target::Player(P0)),
        "the Orb's protection is against opponents; offered {offered:?}");
}

/// Scryfall ruling (2011-09-22): "Like other lands, Stensia Bloodhall is
/// colorless. The damage it deals is from a colorless source, even though
/// activating its ability requires colored mana."
///
/// Nothing in this set gives a player protection from a color, so the ruling
/// has no reachable consequence here — what it guards against is deriving a
/// permanent's colour from its activation cost, which is what would make the
/// {B}{R} in the ability turn the land black and red. So this asserts the
/// ruling directly: the source is colourless, and the damage is attributed to
/// the land itself.
#[test]
fn stensia_bloodhall_is_a_colorless_source() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bloodhall = ready_bloodhall(&mut state, &reg);

    assert!(state.colors_of(bloodhall, &reg).is_empty(),
        "the land is colourless despite {{B}}{{R}} in its ability's cost; got {:?}",
        state.colors_of(bloodhall, &reg));

    // And the damage comes from the land, not from some anonymous source.
    let creature = ready_creature(&mut state, P1, 5, 5);
    let garruk = named_permanent(&mut state, &reg, "Garruk Relentless", P1);
    let _ = (creature, garruk);
    let state = activate_offered(&state, &reg, bloodhall, Some(Target::Player(P1)));
    assert_eq!(state.get_player(P1).life, 18, "and it does deal the 2 damage");
}
