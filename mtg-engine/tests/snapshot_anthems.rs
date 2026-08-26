//! "Creatures you control get +N/+N until end of turn" is continuous.
//!
//! A creature that arrives after the anthem resolved is still a creature you
//! control, so it gets the bonus too. The tempting implementation — walk the
//! battlefield at resolution and push one per-creature effect — silently makes
//! the anthem a snapshot instead, and nothing about the board at the moment it
//! resolved is what the card says.
//!
//! Also here: the other direction, a *static* anthem, which turns off the
//! moment its source leaves rather than lasting the turn.

mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::state::GameState;
use mtg_engine::types::*;

/// Resolve `name` as a spell with nothing on the battlefield to snapshot.
fn resolve_anthem(state: &mut GameState, reg: &CardRegistry, name: &str) {
    let card_id = reg.get_id_by_name(name).unwrap_or_else(|| panic!("unknown card {name}"));
    let spell = state.create_object(card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(spell).unwrap().name = name.into();
    reg.get(card_id).unwrap().on_resolve(state, spell, &[], reg);
}

/// Every "until end of turn" anthem in the set, each resolved onto an empty
/// battlefield — so anything it does to a creature that arrives afterwards is
/// the continuous behaviour and not a snapshot.
#[test]
fn an_until_end_of_turn_anthem_reaches_creatures_that_arrive_later() {
    // (anthem, a creature it should reach and that creature's printed power,
    //  the power it should have under the anthem)
    const CASES: &[(&str, &str, i32, i32)] = &[
        ("Rally the Peasants", "Grizzly Bears", 2, 4),
        ("Vampiric Fury", "Stromkirk Noble", 1, 3),
    ];

    for &(anthem, creature, printed, buffed) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        resolve_anthem(&mut state, &reg, anthem);

        let id = named_permanent(&mut state, &reg, creature, P0);
        assert_eq!(state.effective_power(id, &reg), Some(buffed),
            "{anthem} reaches {creature}, which entered after it resolved \
             (printed {printed})");
    }
}

/// The filtered anthem reaches only what it names.
#[test]
fn vampiric_fury_reaches_vampires_and_nothing_else() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    resolve_anthem(&mut state, &reg, "Vampiric Fury");

    let vampire = named_permanent(&mut state, &reg, "Stromkirk Noble", P0);
    let not_a_vampire = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let their_vampire = named_permanent(&mut state, &reg, "Stromkirk Noble", P1);

    assert_eq!(state.effective_power(vampire, &reg), Some(3), "your Vampire");
    assert!(state.has_keyword(vampire, Keyword::FirstStrike, &reg),
        "and it gains first strike too");
    assert_eq!(state.effective_power(not_a_vampire, &reg), Some(2), "a non-Vampire");
    assert_eq!(state.effective_power(their_vampire, &reg), Some(1),
        "an opponent's Vampire");
}

/// "Creatures you control gain protection from non-Human creatures until end of
/// turn" — the same rule for a granted keyword rather than a P/T change.
#[test]
fn spare_from_evil_protects_creatures_that_arrive_later() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    resolve_anthem(&mut state, &reg, "Spare from Evil");

    let newcomer = ready_creature(&mut state, P0, 2, 2);
    let non_human = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let human = named_permanent(&mut state, &reg, "Avacyn's Pilgrim", P1);

    assert!(state.has_protection_from(newcomer, non_human, &reg),
        "a creature entering after Spare from Evil resolved still gains the \
         protection — the wording is continuous");
    assert!(!state.has_protection_from(newcomer, human, &reg),
        "protection from *non-Human* creatures, so a Human is not covered");
}

/// Selfless Cathar's anthem comes from an activated ability that sacrifices it
/// ("{1}{W}, Sacrifice this creature: Creatures you control get +1/+1 until end
/// of turn"), so it is applied from the graveyard — but it is the same
/// continuous wording, and reaches creatures that arrive afterwards.
#[test]
fn selfless_cathars_anthem_reaches_creatures_that_arrive_later() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cathar = named_permanent(&mut state, &reg, "Selfless Cathar", P0);
    let already_there = ready_creature(&mut state, P0, 2, 2);

    // The engine sacrifices the Cathar before the ability's handler runs.
    state.move_object(cathar, Zone::Graveyard, &reg);
    reg.get(state.get_object(cathar).unwrap().card_id).unwrap()
        .on_activate_ability(&mut state, cathar, 0, &[], &reg);

    assert_eq!(state.effective_power(already_there, &reg), Some(3),
        "the creature that was already out gets +1/+1");

    let newcomer = ready_creature(&mut state, P0, 2, 2);
    assert_eq!(state.effective_power(newcomer, &reg), Some(3),
        "and so does one that arrives afterwards");
}

/// The other direction. Instigator Gang's "Attacking creatures you control get
/// +1/+0" is a *static* ability, so it stops the moment the Gang leaves — it is
/// not an until-end-of-turn effect that outlives its source.
#[test]
fn a_static_anthem_stops_when_its_source_leaves() {
    use mtg_engine::state::TemporaryEffect;

    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let gang = named_permanent(&mut state, &reg, "Instigator Gang", P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);

    state.until_end_of_turn.push(TemporaryEffect::ModifyPTWhileSourceInPlay {
        target: attacker,
        source: gang,
        power_mod: 1,
        toughness_mod: 0,
    });
    assert_eq!(state.effective_power(attacker, &reg), Some(3),
        "test precondition: with the Gang out, the attacker is 3/2");

    state.move_object(gang, Zone::Graveyard, &reg);

    assert_eq!(state.effective_power(attacker, &reg), Some(2),
        "the Gang is gone, so its static ability is gone — the +1/+0 must not \
         linger to end of turn the way a spell's anthem would");
}
