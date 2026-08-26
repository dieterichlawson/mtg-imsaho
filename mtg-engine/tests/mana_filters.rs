//! A filter land is a mana source, and the planner has to know it.
//!
//! Shimmering Grotto is "{T}: Add {C}" and "{1}, {T}: Add one mana of any
//! color". The second is a mana ability under CR 605.1a — an activated ability
//! that could put mana into a pool, with no target and no loyalty cost — and
//! only mana abilities are visible to `gather_mana_sources`. Exposing it
//! through `activated_abilities` meant the auto-tap planner never knew the
//! Grotto could make colored mana: with three Plains and a Grotto, a {2}{G}
//! spell produced no CastSpell action at all, even though the mana is there.
//!
//! `ManaAbilityDef::cost` is what makes that expressible. A filter is net
//! zero — it turns one generic into one colored — so the planner counts the
//! cost as extra generic demand and puts cost-bearing abilities last in the
//! tap plan, where the mana that pays for them is already floating.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

fn castable(state: &mtg_engine::state::GameState, reg: &CardRegistry, spell: mtg_engine::ids::ObjectId) -> bool {
    mtg_engine::engine::legal_actions(state, reg).actions.iter().any(|a|
        matches!(a, Action::CastSpell { object_id, .. } if *object_id == spell))
}

/// Three Plains and a Grotto can cast {2}{G}: two Plains for the generic, one
/// Plains to pay the Grotto's {1}, and the Grotto for the {G}.
#[test]
fn grotto_color_ability_funds_spell_in_tap_plan() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    for _ in 0..3 {
        named_permanent(&mut state, &reg, "Plains", P0);
    }
    named_permanent(&mut state, &reg, "Shimmering Grotto", P0);

    // Orchard Spirit is {2}{G} — no green source but the Grotto, and it needs
    // all four lands: two Plains for the generic, one to pay the Grotto's {1}.
    let spirit = spell_in_hand(&mut state, &reg, "Orchard Spirit", P0);

    assert!(castable(&state, &reg, spirit),
        "the Grotto's colored ability is the only green source, and the planner \
         has to see it: actions were {:?}",
        mtg_engine::engine::legal_actions(&state, &reg).actions);
}

/// Without the Grotto, four Plains cannot cast {2}{G} — no green anywhere.
#[test]
fn without_the_grotto_there_is_no_green() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    for _ in 0..4 {
        named_permanent(&mut state, &reg, "Plains", P0);
    }
    let spirit = spell_in_hand(&mut state, &reg, "Orchard Spirit", P0);

    assert!(!castable(&state, &reg, spirit), "four Plains make no green mana");
}

/// A filter does not ramp. Two Plains and a Grotto is three mana, and the
/// Grotto can only convert one of it — so {2}{G} (four mana) is out of reach.
#[test]
fn a_filter_does_not_add_mana() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    for _ in 0..2 {
        named_permanent(&mut state, &reg, "Plains", P0);
    }
    named_permanent(&mut state, &reg, "Shimmering Grotto", P0);
    let spirit = spell_in_hand(&mut state, &reg, "Orchard Spirit", P0);

    assert!(!castable(&state, &reg, spirit),
        "two Plains plus a Grotto is three mana; {{2}}{{G}} needs four");
}

/// Every mana ability the Grotto offers, in the order the engine lists them.
fn mana_ability_indices(
    state: &mtg_engine::state::GameState,
    reg: &CardRegistry,
    grotto: mtg_engine::ids::ObjectId,
) -> Vec<usize> {
    mtg_engine::engine::legal_actions(state, reg).actions.iter()
        .filter_map(|a| match a {
            Action::ActivateManaAbility { object_id, ability_index } if *object_id == grotto =>
                Some(*ability_index),
            _ => None,
        })
        .collect()
}

/// The ability index that produces `mana`, read off the card rather than
/// written down here — the point is "the ability that makes green", not the
/// number 5.
fn ability_producing(reg: &CardRegistry, state: &mtg_engine::state::GameState,
                     grotto: mtg_engine::ids::ObjectId, mana: ManaType) -> usize {
    let card_id = state.get_object(grotto).unwrap().card_id;
    reg.get(card_id).unwrap().mana_abilities(state, grotto).into_iter()
        .find(|a| a.produced.iter().any(|&(t, n)| t == mana && n == 1))
        .unwrap_or_else(|| panic!("Shimmering Grotto has no ability producing {mana:?}"))
        .ability_index
}

/// The Grotto's own colorless ability still works and still costs nothing,
/// and the five colored ones are gated on being able to pay their {1}.
#[test]
fn the_grotto_still_taps_for_colorless_for_free() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let grotto = named_permanent(&mut state, &reg, "Shimmering Grotto", P0);

    let free = ability_producing(&reg, &state, grotto, ManaType::Colorless);
    assert_eq!(mana_ability_indices(&state, &reg, grotto), vec![free],
        "with an empty pool only the free {{T}}: Add {{C}} is activatable — the \
         colored abilities cost {{1}}");

    // With a mana floating, the colored abilities become available too.
    state.get_player_mut(P0).mana_pool.add(ManaType::White, 1);
    let mut offered = mana_ability_indices(&state, &reg, grotto);
    offered.sort_unstable();
    let mut expected = vec![free];
    for color in [ManaType::White, ManaType::Blue, ManaType::Black,
                  ManaType::Red, ManaType::Green] {
        expected.push(ability_producing(&reg, &state, grotto, color));
    }
    expected.sort_unstable();
    assert_eq!(offered, expected,
        "the {{C}} ability plus one per color, once the {{1}} is payable");
}

/// Activating the filter spends the {1} and produces the color — for every
/// colour it offers, since "one mana of any color" is five abilities and only
/// green used to be checked.
#[test]
fn activating_the_filter_spends_and_produces() {
    let reg = registry();
    for color in [ManaType::White, ManaType::Blue, ManaType::Black,
                  ManaType::Red, ManaType::Green] {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let grotto = named_permanent(&mut state, &reg, "Shimmering Grotto", P0);
        // Pay the {1} with a colour the ability does not produce, so "the pool
        // holds one {color}" cannot be the mana that went in.
        let paid_with = if color == ManaType::Colorless { ManaType::White } else { ManaType::Colorless };
        state.get_player_mut(P0).mana_pool.add(paid_with, 1);

        let index = ability_producing(&reg, &state, grotto, color);
        state = mtg_engine::engine::submit_action(&state,
            &Action::ActivateManaAbility { object_id: grotto, ability_index: index }, &reg);

        let pool = &state.get_player(P0).mana_pool;
        assert_eq!(pool.get(paid_with), 0, "{color:?}: the floating mana paid the {{1}}");
        assert_eq!(pool.get(color), 1, "{color:?}: and the Grotto produced it");
        assert_eq!(pool.total(), 1, "{color:?}: a filter is net zero, not ramp");
        assert!(state.get_object(grotto).unwrap().tapped);
    }
}

/// The general guard: whatever plan the solver returns must actually pay the
/// cost when run. Costs in the plan make ordering matter, so this is worth
/// checking directly rather than trusting the phase logic.
#[test]
fn every_tap_plan_the_solver_returns_actually_pays_the_cost() {
    let reg = registry();
    // A spread of boards and costs, including several that need the filter.
    let boards: &[&[&str]] = &[
        &["Plains", "Plains", "Plains", "Shimmering Grotto"],
        &["Forest", "Shimmering Grotto"],
        &["Island", "Island", "Shimmering Grotto", "Shimmering Grotto"],
        &["Plains", "Island", "Swamp", "Mountain", "Forest"],
        &["Shimmering Grotto", "Shimmering Grotto", "Plains", "Plains"],
    ];
    let costs: &[&str] = &["Orchard Spirit", "Chapel Geist", "Walking Corpse",
                           "Brimstone Volley", "Darkthicket Wolf"];

    let mut exercised = 0;
    for board in boards {
        for spell_name in costs {
            let mut state = game_at_step(Step::PrecombatMain, P0);
            for land in *board {
                named_permanent(&mut state, &reg, land, P0);
            }
            let spell = spell_in_hand(&mut state, &reg, spell_name, P0);
            let Some(cost) = reg.card_data(state.get_object(spell).unwrap().card_id)
                .and_then(|d| d.cost) else { continue };

            let action = mtg_engine::engine::legal_actions(&state, &reg).actions.into_iter()
                .find(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == spell));
            let Some(action) = action else { continue };
            let Action::CastSpell { ref tap_plan, .. } = action else { unreachable!() };

            // Run the plan, then check the cost is payable from what it made.
            let mut sim = state.clone();
            for &(source_id, ability_index) in tap_plan {
                mtg_engine::engine::activate_mana_source(&mut sim, source_id, ability_index, &reg);
            }
            assert!(mtg_engine::mana::can_pay(&sim.get_player(P0).mana_pool, &cost),
                "board {board:?}, spell {spell_name}: the plan {tap_plan:?} left \
                 {:?}, which does not pay {cost}",
                sim.get_player(P0).mana_pool.mana);
            exercised += 1;
        }
    }
    assert!(exercised >= 8, "only {exercised} plans exercised — the guard is too weak");
}
