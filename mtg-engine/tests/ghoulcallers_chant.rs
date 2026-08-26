//! Ghoulcaller's Chant — {B} Sorcery.
//!
//! "Choose one —
//!  • Return target creature card from your graveyard to your hand.
//!  • Return two target Zombie creature cards from your graveyard to your hand."
//!
//! A modal spell whose two modes have different target requirements: one
//! target of one kind, or two of a narrower kind. What the engine offers is
//! therefore a *set* of target sets, and that shape is what these check —
//! `legal_actions` has to produce every legal combination of every mode and
//! nothing else (CR 601.2b/601.2c).

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::types::*;

/// The offered target sets, split by arity: (mode-1 singles, mode-2 pairs).
fn modes(
    state: &mtg_engine::state::GameState,
    reg: &mtg_engine::cards::CardRegistry,
    chant: ObjectId,
) -> (Vec<Target>, Vec<Vec<Target>>) {
    let sets = offered_target_sets(state, reg, chant);
    let singles = sets.iter().filter(|t| t.len() == 1).map(|t| t[0].clone()).collect();
    let pairs = sets.into_iter().filter(|t| t.len() == 2).collect();
    (singles, pairs)
}

/// Which cards each mode may name, for every shape of graveyard that matters.
///
/// The negative half of each row is as important as the positive: mode 2 needs
/// *two* Zombies, so one Zombie beside a Bear offers no pair, and two Bears
/// offer no pair either — an engine that ignored the Zombie restriction would
/// pass a test that only looked at the all-Zombie case.
#[test]
fn each_mode_offers_exactly_the_cards_it_may_name() {
    // (cards in your graveyard, cards in the opponent's, mode-1 count, mode-2 count)
    const CASES: &[(&[&str], &[&str], usize, usize, &str)] = &[
        (&["Grizzly Bears"], &[], 1, 0,
         "one non-Zombie: mode 1 only"),
        (&["Walking Corpse", "Diregraf Ghoul"], &[], 2, 1,
         "two Zombies: either one alone, or both together"),
        (&["Grizzly Bears", "Savannah Lions"], &[], 2, 0,
         "two non-Zombies: no pair, because mode 2 names Zombies"),
        (&["Walking Corpse", "Grizzly Bears"], &[], 2, 0,
         "one Zombie and one not: still no pair"),
        (&["Grizzly Bears", "Walking Corpse", "Diregraf Ghoul"], &[], 3, 1,
         "three cards, two of them Zombies: three singles and the one pair"),
        (&[], &["Grizzly Bears"], 0, 0,
         "'your graveyard' — an opponent's creature card is not a legal target"),
    ];

    for &(mine, theirs, singles_expected, pairs_expected, why) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let ids: Vec<ObjectId> = mine.iter()
            .map(|n| named_card_in_graveyard(&mut state, &reg, n, P0))
            .collect();
        for n in theirs {
            named_card_in_graveyard(&mut state, &reg, n, P1);
        }

        let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);
        let (singles, pairs) = modes(&state, &reg, chant);

        assert_eq!(singles.len(), singles_expected, "{why}: mode 1 count");
        assert_eq!(pairs.len(), pairs_expected, "{why}: mode 2 count");

        // Every card offered is one of yours, and every one of yours is offered.
        for id in &ids {
            assert!(singles.contains(&Target::Object(*id)),
                "{why}: every creature card in your graveyard is a mode-1 target");
        }
        for pair in &pairs {
            for t in pair {
                let Target::Object(id) = t else { panic!("{why}: mode 2 names cards") };
                assert!(state.has_subtype(*id, "Zombie", &reg),
                    "{why}: mode 2 names Zombies only");
            }
        }
    }
}

/// Mode 1 resolving: the named card comes back.
#[test]
fn mode_one_returns_the_creature_card_it_named() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bear = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let bystander = named_card_in_graveyard(&mut state, &reg, "Savannah Lions", P0);

    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);
    let state = cast_and_resolve(&state, &reg, chant, vec![Target::Object(bear)]);

    assert_eq!(state.get_object(bear).unwrap().zone, Zone::Hand);
    assert_eq!(state.get_object(bystander).unwrap().zone, Zone::Graveyard,
        "only the card it named");
}

/// Mode 2 resolving: both named Zombies come back.
#[test]
fn mode_two_returns_both_zombies_it_named() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let a = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let b = named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);

    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);
    let state = cast_and_resolve(&state, &reg, chant,
        vec![Target::Object(a), Target::Object(b)]);

    assert_eq!(state.get_object(a).unwrap().zone, Zone::Hand);
    assert_eq!(state.get_object(b).unwrap().zone, Zone::Hand);
}
