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
use mtg_engine::actions::Target;
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
            assert!(demons[0].keywords.contains(&Keyword::Flying), "with flying");
        }
    }
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
