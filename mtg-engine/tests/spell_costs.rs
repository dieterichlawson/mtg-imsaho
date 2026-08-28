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

    named_permanent(&mut state, &reg, "Heartless Summoning", P0);
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

    named_permanent(&mut state, &reg, "Heartless Summoning", P0);
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
    named_permanent(&mut state, &reg, "Heartless Summoning", P0);

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
    named_permanent(&mut state, &reg, "Heartless Summoning", P0);

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

    let summoning = named_permanent(&mut state, &reg, "Heartless Summoning", P0);
    assert_eq!(mana_value(&state, &reg, "Skaab Ruinator", P0, CastMethod::Normal), 2);

    state.move_object(summoning, Zone::Graveyard, &reg);
    assert_eq!(mana_value(&state, &reg, "Skaab Ruinator", P0, CastMethod::Normal), 3);
}

/// An opponent's discount is not yours.
#[test]
fn a_reduction_only_helps_the_player_who_controls_it() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Heartless Summoning", P1);

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
    named_permanent(&mut state, &reg, "Heartless Summoning", P0);
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
        state.get_object(lunge).unwrap().card_state
            .get(&mtg_engine::cards::exiled_to_cost_key(0)).copied(),
        Some(strong),
        "and the spell records the card it exiled, so it can ask that card its \
         power when it resolves");
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

// ---------------------------------------------------------------------------
// CR 601.2f — an alternative cost, wherever the spell is cast from
// ---------------------------------------------------------------------------

/// "You may pay {0} rather than pay the mana cost for Zombie creature spells
/// you cast" (Rooftop Storm). Skaab Ruinator is a Zombie that may be cast from
/// the graveyard, so it is the case where the two meet: the alternative cost
/// has to reach the graveyard cast, not only the cast from hand.
///
/// This replaces a test named `..._not_offered_from_graveyard` whose body
/// talked itself out of the graveyard case ("Actually there are no Zombie
/// creatures with flashback in ISD... The simplest test: verify Rooftop Storm
/// works from hand first") and then tested casting from hand, which
/// `cards_rule_modifiers.rs` already covers.
#[test]
fn rooftop_storms_free_cast_reaches_a_zombie_cast_from_the_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ruinator = named_card_in_graveyard(&mut state, &reg, "Skaab Ruinator", P0);
    // Its additional cost: exile three creature cards from your graveyard.
    for _ in 0..3 {
        named_card_in_graveyard(&mut state, &reg, "Doomed Traveler", P0);
    }
    assert!(state.has_subtype(ruinator, "Zombie", &reg),
        "test precondition: the Ruinator is a Zombie, which is what Rooftop \
         Storm's filter names");

    // Not a drop of mana anywhere.
    assert_eq!(state.get_player(P0).mana_pool.total(), 0);
    assert!(!can_cast(&state, &reg, ruinator),
        "without Rooftop Storm, {{1}}{{U}}{{U}} is unaffordable");

    named_permanent(&mut state, &reg, "Rooftop Storm", P0);
    assert!(can_cast(&state, &reg, ruinator),
        "CR 601.2f: the alternative cost applies to the graveyard cast too, so \
         the Ruinator is castable for {{0}}");
}

/// Ruling: "You must still pay any mandatory additional costs, such as exiling
/// a creature card from your graveyard for Makeshift Mauler." Paying {0}
/// replaces the MANA cost only (CR 601.2b) — with no creature card in the
/// graveyard the Mauler stays uncastable however free the mana is, and a
/// {0} cast that does happen still exiles.
#[test]
fn rooftop_storms_zero_does_not_waive_a_mandatory_additional_cost() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    named_permanent(&mut state, &reg, "Rooftop Storm", P0);
    let mauler = spell_in_hand(&mut state, &reg, "Makeshift Mauler", P0);
    assert_eq!(state.get_player(P0).mana_pool.total(), 0);

    assert!(!can_cast(&state, &reg, mauler),
        "no creature card in the graveyard: {{0}} pays the mana, not the exile");

    let fodder = named_card_in_graveyard(&mut state, &reg, "Doomed Traveler", P0);
    assert!(can_cast(&state, &reg, mauler), "with fodder the free cast is legal");

    // Submit the OFFERED action (it carries the {0} alternative cost), then
    // answer the exile prompt and resolve.
    let offered = mtg_engine::engine::legal_actions(&state, &reg).actions.into_iter()
        .find(|a| matches!(a, mtg_engine::actions::Action::CastSpell { object_id, .. } if *object_id == mauler))
        .expect("the free cast is offered");
    let state = mtg_engine::engine::submit_action(&state, &offered, &reg);
    let mut state = resolve_exile_choice_max_power(&state, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(fodder).unwrap().zone, Zone::Exile,
        "the additional cost was really paid on the {{0}} cast");
    assert_eq!(state.get_object(mauler).unwrap().zone, Zone::Battlefield,
        "and the Mauler resolved");
}

// ---------------------------------------------------------------------------
// Mana value is not the total cost
// ---------------------------------------------------------------------------

/// Blasphemous Act ruling: "The mana value of the spell is determined only by
/// its mana cost, no matter what the total cost to cast the spell was."
///
/// Its {8}{R} can be reduced all the way to {R}, so it is the card where the
/// two numbers come apart most. Everything that reads a mana value reads
/// `card_data().cost`, and nothing may start routing that through
/// `cost_to_cast` — Mindshrieker's "+X/+X where X is the mana value of that
/// card" is the reader that would show it.
#[test]
fn a_cost_reduction_does_not_change_a_cards_mana_value() {
    use mtg_engine::actions::Target;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let shrieker = named_permanent(&mut state, &reg, "Mindshrieker", P0);
    // Nine more creatures: with ten on the battlefield the Act costs {R}.
    for _ in 0..9 {
        ready_creature(&mut state, P0, 1, 1);
    }
    assert_eq!(mana_value(&state, &reg, "Blasphemous Act", P0, CastMethod::Normal), 1,
        "test precondition: ten creatures reduce the total cost to {{R}}");

    let act = reg.get_id_by_name("Blasphemous Act").unwrap();
    let lib_card = state.create_object(act, P1, Zone::Library, None, None);
    state.get_player_mut(P1).library_order = vec![lib_card];

    add_mana(&mut state, P0, &[(ManaType::Colorless, 2)]);
    let state = activate_offered(&state, &reg, shrieker, Some(Target::Player(P1)));

    assert_eq!(state.effective_power(shrieker, &reg), Some(1 + 9),
        "the milled Act's mana value is its printed {{8}}{{R}} = 9, not the \
         {{R}} it would have cost to cast with ten creatures out");
}
