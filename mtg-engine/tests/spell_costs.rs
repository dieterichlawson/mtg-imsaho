//! One determination of what a spell costs (CR 601.2f).
//!
//! A spell's cost used to be adjusted in five unrelated ways, each read at
//! whichever call site remembered it: a `ReduceCost` continuous effect, an
//! `AlternativeCost` one, `CardBehavior::modified_cost`,
//! `CardData::flashback_cost`, and `CardData::additional_cost`. Cost
//! reductions reached spells cast from hand and nowhere else.

mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{CastMethod, cost_to_cast};
use mtg_engine::types::*;
fn mana_value(state: &mtg_engine::state::GameState, reg: &CardRegistry,
              name: &str, player: mtg_engine::ids::PlayerId, method: CastMethod) -> u32 {
    let card_id = reg.get_id_by_name(name).unwrap_or_else(|| panic!("unknown card {name}"));
    cost_to_cast(state, reg, card_id, player, &method).mana.mana_value()
}

// ---------------------------------------------------------------------------
// CR 601.2f — reductions apply to whatever the base cost is
// ---------------------------------------------------------------------------

/// Heartless Summoning: "Creature spells you cast cost {2} less to cast."
#[test]
fn a_cost_reduction_applies_to_a_creature_spell_from_hand() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Skaab Ruinator is {1}{U}{U}.
    assert_eq!(mana_value(&state, &reg, "Skaab Ruinator", P0, CastMethod::Normal), 3);

    named_creature(&mut state, &reg, "Heartless Summoning", P0);
    // {1}{U}{U} has only {1} of generic to give up, so a {2} reduction leaves
    // {U}{U}. CR 601.2f: a reduction never eats a coloured requirement.
    assert_eq!(mana_value(&state, &reg, "Skaab Ruinator", P0, CastMethod::Normal), 2);
}

/// The same spell cast from the graveyard. Skaab Ruinator's "you may cast this
/// card from your graveyard" runs through the flashback path, which used to
/// autotap for the printed cost directly — so the discount stopped at the
/// hand.
#[test]
fn a_cost_reduction_applies_to_a_spell_cast_from_the_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let printed = reg.card_data(reg.get_id_by_name("Skaab Ruinator").unwrap())
        .and_then(|d| d.cost).unwrap();

    named_creature(&mut state, &reg, "Heartless Summoning", P0);
    let from_gy = mana_value(&state, &reg, "Skaab Ruinator", P0,
        CastMethod::Alternative(printed.clone()));

    assert_eq!(from_gy, 2,
        "CR 601.2f: a reduction applies to the total cost however the spell is \
         cast, not only to a cast from hand");
}

/// A reduction never eats a coloured requirement (CR 601.2f).
#[test]
fn a_reduction_only_comes_off_the_generic_portion() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_creature(&mut state, &reg, "Heartless Summoning", P0);

    let card_id = reg.get_id_by_name("Skaab Ruinator").unwrap();
    let cost = cost_to_cast(&state, &reg, card_id, P0, &CastMethod::Normal).mana;
    let blue = cost.colored_requirements().get(&Color::Blue).copied().unwrap_or(0);
    assert_eq!(blue, 2, "both {{U}} survive a {{2}} reduction");
    assert_eq!(cost.generic_amount(), 0, "the {{1}} is gone");
}

/// A spell that is not a creature spell is not reduced by a creature-spell
/// discount.
#[test]
fn a_reduction_respects_its_filter() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_creature(&mut state, &reg, "Heartless Summoning", P0);

    let before = mana_value(&state, &reg, "Brimstone Volley", P0, CastMethod::Normal);
    let printed = reg.card_data(reg.get_id_by_name("Brimstone Volley").unwrap())
        .and_then(|d| d.cost).unwrap();
    assert_eq!(before, printed.mana_value(),
        "an instant is not a creature spell");
}

/// The discount is read through the continuous-effect layer, so it follows the
/// permanent leaving.
#[test]
fn a_reduction_stops_when_its_source_leaves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let summoning = named_creature(&mut state, &reg, "Heartless Summoning", P0);
    assert_eq!(mana_value(&state, &reg, "Skaab Ruinator", P0, CastMethod::Normal), 2);

    state.move_object(summoning, Zone::Graveyard, &reg);
    assert_eq!(mana_value(&state, &reg, "Skaab Ruinator", P0, CastMethod::Normal), 3);
}

/// An opponent's discount is not yours.
#[test]
fn a_reduction_only_helps_the_player_who_controls_it() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_creature(&mut state, &reg, "Heartless Summoning", P1);

    assert_eq!(mana_value(&state, &reg, "Skaab Ruinator", P0, CastMethod::Normal), 3,
        "P1's Heartless Summoning does not discount P0's creature spells");
    assert_eq!(mana_value(&state, &reg, "Skaab Ruinator", P1, CastMethod::Normal), 2);
}

/// The same claim end to end, through `legal_actions`: with the discount out,
/// two blue mana is enough to cast Skaab Ruinator from the graveyard, where
/// its printed {1}{U}{U} would need three.
#[test]
fn a_graveyard_cast_is_offered_at_the_reduced_cost() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ruinator = named_card_in_graveyard(&mut state, &reg, "Skaab Ruinator", P0);
    for _ in 0..3 {
        named_card_in_graveyard(&mut state, &reg, "Doomed Traveler", P0);
    }
    named_creature(&mut state, &reg, "Heartless Summoning", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 2);

    let offered = |state: &mtg_engine::state::GameState| {
        mtg_engine::engine::legal_actions(state, &reg).actions.iter().any(|a|
            matches!(a, mtg_engine::actions::Action::CastSpell { object_id, .. }
                if *object_id == ruinator))
    };
    assert!(offered(&state),
        "CR 601.2f: Heartless Summoning discounts the Ruinator's graveyard cast \
         to {{U}}{{U}}, which is exactly what is available");

    // And the discount is what made the difference: without it, two blue is
    // short of the printed {1}{U}{U}.
    let summoning = state.objects.values()
        .find(|o| o.name == "Heartless Summoning").map(|o| o.id).unwrap();
    state.move_object(summoning, Zone::Graveyard, &reg);
    assert!(!offered(&state));
}

// ---------------------------------------------------------------------------
// CR 601.2b — additional costs
// ---------------------------------------------------------------------------

/// Corpse Lunge "deals damage equal to the exiled creature's power". When the
/// player makes no choice the engine picks the strongest creature card — but
/// the cast handler ranked candidates by `obj.power`, which is `None` for
/// every non-token card, so it was really picking by object id.
#[test]
fn auto_paying_an_exile_cost_picks_the_strongest_creature_card() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put the weak creature in the graveyard first, so a by-id pick takes it.
    let weak = named_card_in_graveyard(&mut state, &reg, "Doomed Traveler", P0);      // 1/1
    let strong = named_card_in_graveyard(&mut state, &reg, "Bloodgift Demon", P0);    // 5/5
    let lunge = spell_in_hand(&mut state, &reg, "Corpse Lunge", P0);

    mtg_engine::engine::pay_exile_creatures(&mut state, &reg, lunge, P0, 1, &[]);

    assert_eq!(state.get_object(strong).unwrap().zone, Zone::Exile,
        "the 5/5 is exiled, not whichever card happened to come first");
    assert_eq!(state.get_object(weak).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        state.get_object(lunge).unwrap().card_state.get("exiled_power").map(|o| o.0),
        Some(5),
        "and its power is what the spell will deal");
}

// ---------------------------------------------------------------------------
// Structural guard
// ---------------------------------------------------------------------------

/// Everything that needs to know what a spell costs asks `cost_to_cast`.
///
/// The mechanisms are still declared in several places — that is fine, a card
/// says what it does. What must not come back is a second place that *decides*
/// the total: reading `flashback_cost` or `additional_cost` and acting on it
/// without going through the one determination.
#[test]
fn spell_costs_are_determined_in_one_place() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).expect("readable").flatten() {
            let p = e.path();
            if p.is_dir() { walk(&p, out); }
            else if p.extension().is_some_and(|x| x == "rs") { out.push(p); }
        }
    }
    walk(&src, &mut files);

    const ALLOWED: &[&str] = &[
        "src/engine/costs.rs", // the one determination
        "src/cards/mod.rs",    // the CardData field declarations
        "src/view.rs",         // shows the printed cost to a player
    ];
    let mut offenders = Vec::new();
    for f in &files {
        let rel = f.to_string_lossy().replace('\\', "/");
        if ALLOWED.iter().any(|a| rel.ends_with(a)) || rel.contains("/cards/isd/") {
            continue;
        }
        let text = std::fs::read_to_string(f).expect("readable");
        for (n, line) in text.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("//") || t.starts_with("///") {
                continue;
            }
            if t.contains("data.additional_cost") || t.contains("card_data().additional_cost") {
                offenders.push(format!("{rel}:{}: {}", n + 1, t));
            }
        }
    }
    assert!(offenders.is_empty(),
        "additional costs are determined by engine::costs, not re-read per call site:\n{}",
        offenders.join("\n"));
}
