//! Tests for Innistrad Tier 2 cards: targeted removal, bounce, fight,
//! permanent destruction, and counter variants.
//!
//! Cards covered (12), so this is greppable by name as well as by rule:
//!
//! - Bramblecrush
//! - Dissipate
//! - Frightful Delusion
//! - Geistflame
//! - Lost in the Mist
//! - Naturalize
//! - Prey Upon
//! - Rebuke
//! - Silent Departure
//! - Smite the Monstrous
//! - Urgent Exorcism
//! - Victim of Night

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;
// ── Simple damage spells ────────────────────────────────────────────

// Bump in the Night's 3-life-drain to a player is covered by the
// parametric `direct_damage_spells_drain_player_life` in spells.rs.
// Flashback behavior is covered in flashback.rs.

#[test]
fn geistflame_deals_1_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 2, 2);
    let card = castable_spell(&mut state, &reg, "Geistflame", P0);

    state = cast_and_resolve(&state, &reg, card, vec![Target::Object(creature)]);

    assert_eq!(state.get_object(creature).unwrap().damage_marked, 1);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "2/2 with 1 damage should survive");
}

// Brimstone Volley's 3-damage-to-player case is covered by the
// parametric `direct_damage_spells_drain_player_life` in spells.rs.

// ── Counter variants ────────────────────────────────────────────────

/// Dissipate counters and exiles the spell (not graveyard).
#[test]
fn dissipate_counters_and_exiles() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 casts a creature spell.
    let tusker = castable_spell(&mut state, &reg, "Kalonian Tusker", P0);

    state = cast_onto_stack(&state, &reg, tusker, vec![]);

    // P1 casts Dissipate targeting the Tusker on the stack.
    let diss = castable_spell(&mut state, &reg, "Dissipate", P1);
    state.priority_player = Some(P1);

    state = cast_and_resolve(&state, &reg, diss, vec![Target::Object(tusker)]);

    assert_eq!(state.get_object(tusker).unwrap().zone, Zone::Exile,
        "Dissipate should exile the countered spell, not put it in graveyard");
    assert_eq!(state.get_object(diss).unwrap().zone, Zone::Graveyard);
}

/// Frightful Delusion counters and forces a discard.
#[test]
fn frightful_delusion_counters_and_discards() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Give P0 a card in hand (to be discarded).
    let hand_card = spell_in_hand(&mut state, &reg, "Mountain", P0);

    // P0 casts a creature.
    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P0);

    state = cast_onto_stack(&state, &reg, bears, vec![]);

    // P1 casts Frightful Delusion.
    let fd = castable_spell(&mut state, &reg, "Frightful Delusion", P1);
    state.priority_player = Some(P1);

    state = cast_and_resolve(&state, &reg, fd, vec![Target::Object(bears)]);

    // CR 608.2g: P0 is asked whether to pay {1}. Their only Mountain is in
    // hand, so declining is the only legal answer.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: mtg_engine::actions::ResolvedChoice::PayDecision(false),
    }, &reg);

    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "Spell should be countered");
    // P0's hand card should have been discarded.
    assert_eq!(state.get_object(hand_card).unwrap().zone, Zone::Graveyard,
        "Controller of countered spell should discard a card");
}

// ── What a removal spell is allowed to point at ─────────────────────

/// A candidate for a removal spell to consider, built fresh per row.
enum Candidate {
    /// A vanilla creature of this size.
    Creature(i32, i32),
    /// A named card put onto the battlefield (for its subtypes).
    Named(&'static str),
    /// A basic land.
    Land,
    /// An Aura, which needs a creature to enchant — so this also supplies the
    /// creature the row's "illegal" side uses.
    Enchantment,
}

fn place(state: &mut mtg_engine::state::GameState, reg: &mtg_engine::cards::CardRegistry, c: &Candidate) -> ObjectId {
    match *c {
        Candidate::Creature(p, t) => ready_creature(state, P1, p, t),
        Candidate::Named(name) => named_permanent(state, reg, name, P1),
        Candidate::Land => {
            let id = reg.get_id_by_name("Forest").unwrap();
            let land = state.create_object(id, P1, Zone::Battlefield, None, None);
            state.get_object_mut(land).unwrap().summoning_sick = false;
            land
        }
        Candidate::Enchantment => {
            let creature = ready_creature(state, P1, 2, 2);
            let pac = castable_spell(state, reg, "Pacifism", P1);
            // The Aura's controller has to hold priority to pay for it.
            state.priority_player = Some(P1);
            *state = cast_and_resolve(state, reg, pac, vec![Target::Object(creature)]);
            pac
        }
    }
}

/// Targeted removal, and what each spell's text does and does not let it point
/// at. CR 601.2c: the engine only offers legal targets, so both halves are
/// observable from `legal_actions`.
///
/// Every row carries a legal candidate as well as an illegal one. Without it, a
/// row asserts only "this target is not offered" — which an engine that offered
/// nothing at all would satisfy. Three of the tests this replaces were exactly
/// that shape.
#[test]
fn targeted_removal_offers_the_targets_its_text_allows() {
    // (spell, something it may target, something it may not, what the rule is)
    const CASES: &[(&str, Candidate, Candidate, &str)] = &[
        ("Victim of Night", Candidate::Creature(2, 2), Candidate::Named("Markov Patrician"),
         "'creature that isn't a Vampire, Werewolf, or Zombie' — the Patrician is a Vampire"),
        ("Smite the Monstrous", Candidate::Creature(5, 5), Candidate::Creature(2, 2),
         "'creature with power 4 or greater'"),
        ("Naturalize", Candidate::Enchantment, Candidate::Creature(3, 3),
         "'target artifact or enchantment'"),
        ("Bramblecrush", Candidate::Land, Candidate::Creature(3, 3),
         "'target noncreature permanent'"),
        ("Urgent Exorcism", Candidate::Named("Chapel Geist"), Candidate::Creature(3, 3),
         "'target Spirit or enchantment' — the Geist is a Spirit"),
    ];

    for (spell_name, legal, illegal, rule) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let good = place(&mut state, &reg, legal);
        let bad = place(&mut state, &reg, illegal);
        state.priority_player = Some(P0);
        let spell = castable_spell(&mut state, &reg, spell_name, P0);

        let offered = offered_targets(&state, &reg, spell);
        assert!(offered.contains(&Target::Object(good)),
            "{spell_name} should be able to target it: {rule}. offered: {offered:?}");
        assert!(!offered.contains(&Target::Object(bad)),
            "{spell_name} should not be able to target it: {rule}");

        // And the spell does what it says to the target it was allowed.
        let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(good)]);
        assert_eq!(state.get_object(good).unwrap().zone, Zone::Graveyard,
            "{spell_name} destroys what it targeted");
    }
}

/// Rebuke ("Destroy target attacking creature") needs a combat to have a legal
/// target at all, so it gets its own setup — same rule as the table above.
#[test]
fn rebuke_only_targets_a_creature_that_is_attacking() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 3, 3);
    let bystander = ready_creature(&mut state, P0, 2, 2);
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    state.priority_player = Some(P1);

    let rebuke = castable_spell(&mut state, &reg, "Rebuke", P1);
    let offered = offered_targets(&state, &reg, rebuke);
    assert!(offered.contains(&Target::Object(attacker)), "the attacking creature is a legal target");
    assert!(!offered.contains(&Target::Object(bystander)), "the one that stayed home is not");

    let state = cast_and_resolve(&state, &reg, rebuke, vec![Target::Object(attacker)]);
    assert_eq!(state.get_object(attacker).unwrap().zone, Zone::Graveyard);
}

// ── Bounce ──────────────────────────────────────────────────────────

/// Silent Departure returns a creature to its owner's hand.
#[test]
fn silent_departure_bounces_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);

    let card = castable_spell(&mut state, &reg, "Silent Departure", P0);

    state = cast_and_resolve(&state, &reg, card, vec![Target::Object(creature)]);

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Hand,
        "Creature should be returned to hand");
}

// ── Fight ───────────────────────────────────────────────────────────

/// Prey Upon: your creature fights their creature. Both deal damage.
#[test]
fn prey_upon_fight() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mine = ready_creature(&mut state, P0, 3, 3);
    let theirs = ready_creature(&mut state, P1, 2, 2);

    let pu = castable_spell(&mut state, &reg, "Prey Upon", P0);

    state = cast_and_resolve(&state, &reg, pu, vec![Target::Object(mine), Target::Object(theirs)]);

    // 3/3 deals 3 to 2/2, 2/2 deals 2 to 3/3.
    assert_eq!(state.get_object(mine).unwrap().damage_marked, 2);
    assert_eq!(state.get_object(theirs).unwrap().damage_marked, 3);

    // SBA kills the 2/2.
    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(theirs).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(mine).unwrap().zone, Zone::Battlefield);
}

// ── Two-target spells ───────────────────────────────────────────────

/// Lost in the Mist counters a spell and bounces a permanent.
#[test]
fn lost_in_the_mist_counters_and_bounces() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 has a creature on the battlefield.
    let creature = ready_creature(&mut state, P0, 3, 3);

    // P0 casts a spell.
    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P0);

    state = cast_onto_stack(&state, &reg, bears, vec![]);

    // P1 casts Lost in the Mist targeting the spell + the creature.
    let litm = castable_spell(&mut state, &reg, "Lost in the Mist", P1);
    state.priority_player = Some(P1);

    state = cast_and_resolve(&state, &reg, litm, vec![Target::Object(bears), Target::Object(creature)]);

    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "Spell should be countered");
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Hand,
        "Permanent should be bounced to hand");
}

// -------------------------------------------------------------------------
// Bramblecrush
// -------------------------------------------------------------------------

/// Bramblecrush should use the destruction pipeline for non-creature permanents.
/// An indestructible enchantment should survive Bramblecrush.
#[test]
fn bramblecrush_respects_indestructible() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a non-creature permanent (enchantment) with indestructible.
    let enchantment = state.create_object(CardId(9999), P1, Zone::Battlefield, None, None);
    state.get_object_mut(enchantment).unwrap().name = "Indestructible Enchantment".into();
    state.get_object_mut(enchantment).unwrap().card_types = vec![CardType::Enchantment];
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::GrantKeyword {
            target: enchantment,
            keyword: Keyword::Indestructible,
        },
    );

    let crush = castable_spell(&mut state, &reg, "Bramblecrush", P0);
    state = cast_and_resolve(&state, &reg, crush, vec![Target::Object(enchantment)]);

    // Indestructible enchantment should survive.
    assert_eq!(state.get_object(enchantment).unwrap().zone, Zone::Battlefield,
        "Bramblecrush should respect indestructible on non-creature permanents");
}
