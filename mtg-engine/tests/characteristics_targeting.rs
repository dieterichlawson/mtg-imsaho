//! Regression tests for characteristics-layer targeting fixes.
//!
//! Non-token permanents have empty object-level `card_types`, so any filter
//! that read `obj.card_types` directly silently excluded them:
//! - `HasCardType([Land])` (Ghost Quarter) found zero non-token lands.
//! - `AnyTarget` (Lightning Bolt) could not target non-token planeswalkers.
//!
//! Both now resolve card types through `GameState::has_card_type`, which
//! falls back to the active face's registry data.

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::engine;
use mtg_engine::types::*;

/// Ghost Quarter's "Destroy target land" uses `PermanentWithFilter(HasCardType([Land]))`.
/// A non-token land (empty object-level `card_types`) must be a valid target.
#[test]
fn ghost_quarter_can_target_non_token_land() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gq = named_permanent(&mut state, &reg, "Ghost Quarter", P0);
    let forest = named_permanent(&mut state, &reg, "Forest", P1);
    assert!(state.get_object(forest).unwrap().card_types.is_empty(),
        "test precondition: non-token permanents have empty object-level card_types");

    let legal = engine::legal_actions(&state, &reg);
    let gq_targets: Vec<Target> = legal.actions.iter()
        .filter_map(|a| match a {
            Action::ActivateAbility { object_id, targets, .. } if *object_id == gq => {
                Some(targets.clone())
            }
            _ => None,
        })
        .flatten()
        .collect();

    assert!(gq_targets.contains(&Target::Object(forest)),
        "Ghost Quarter should be able to target a non-token land; got targets {gq_targets:?}");
}

/// Lightning Bolt's `AnyTarget` must include non-token planeswalkers.
#[test]
fn any_target_includes_non_token_planeswalker() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    set_loyalty(&mut state, liliana, 3);
    assert!(state.get_object(liliana).unwrap().card_types.is_empty(),
        "test precondition: non-token permanents have empty object-level card_types");

    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);

    let legal = engine::legal_actions(&state, &reg);
    let bolt_targets: Vec<Target> = legal.actions.iter()
        .filter_map(|a| match a {
            Action::CastSpell { object_id, targets, .. } if *object_id == bolt => {
                Some(targets.clone())
            }
            _ => None,
        })
        .flatten()
        .collect();

    assert!(bolt_targets.contains(&Target::Object(liliana)),
        "Lightning Bolt (any target) should be able to target a non-token planeswalker; got {bolt_targets:?}");
}

// ---------------------------------------------------------------------------
// A card's target filter has to match its own wording (CR 601.2c)
// ---------------------------------------------------------------------------

/// What each card's wording must reject, and — as importantly — what it must
/// still accept.
///
/// The rejection alone proves nothing: a filter that rejected everything, or an
/// id the state has never heard of, satisfies it. One of the tests this
/// replaces built its creature in `&mut state.clone()` and then asked about it
/// in the original state, so the answer was "no such object" rather than
/// anything about lands.
///
/// Asked through `legal_actions`, which is where a target is actually offered
/// (CR 601.2c). This used to call `behavior.is_valid_target` directly, which
/// tests one implementation layer rather than the rule: a card whose wording
/// lives entirely in its `TargetRequirement` — the normal case, and where these
/// two are heading — has no override at all, and the trait default returns
/// `true` for everything, so every row would have passed vacuously.
#[test]
fn a_cards_target_filter_matches_its_wording() {
    /// What to put on the battlefield and offer to the spell.
    enum Candidate {
        /// A token with these subtypes — its characteristics live on the
        /// object, which is where the registry-only filters used to miss them.
        Token(&'static str, &'static [&'static str]),
        VanillaCreature,
        Caster,
        Opponent,
    }

    // (card, a target it must accept, one it must reject, the wording at issue)
    let cases: &[(&str, Candidate, Candidate, &str)] = &[
        ("Victim of Night", Candidate::VanillaCreature,
         Candidate::Token("Vampire", &["Vampire"]),
         "'non-Vampire, non-Werewolf, non-Zombie creature' — a Vampire token is \
          a Vampire, and its subtypes are on the object, not in the registry"),
        ("Tribute to Hunger", Candidate::Opponent, Candidate::Caster,
         "'target opponent' is not 'target player'"),
        // Both "target opponent" cards get a row: what this catches is a card
        // wired to the wrong requirement, which is per-card even though the
        // restriction itself is now stated once, in `OpponentOnly`.
        ("Bump in the Night", Candidate::Opponent, Candidate::Caster,
         "'target opponent' is not 'target player'"),
    ];

    for (name, accept, reject, why) in cases {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let mut place = |c: &Candidate| match c {
            Candidate::Token(token_name, subtypes) => {
                let id = state.create_token_with_subtypes(
                    token_name, P1, 2, 2, vec![Color::Black], vec![CardType::Creature],
                    vec![], subtypes.iter().map(|s| (*s).to_string()).collect(), &reg)[0];
                state.get_object_mut(id).unwrap().summoning_sick = false;
                Target::Object(id)
            }
            Candidate::VanillaCreature => Target::Object(ready_creature(&mut state, P1, 3, 3)),
            Candidate::Caster => Target::Player(P0),
            Candidate::Opponent => Target::Player(P1),
        };
        let good = place(accept);
        let bad = place(reject);

        state.priority_player = Some(P0);
        let spell = castable_spell(&mut state, &reg, name, P0);
        let offered = offered_targets(&state, &reg, spell);

        assert!(offered.contains(&good),
            "{name} must accept {good:?}: {why}. offered: {offered:?}");
        assert!(!offered.contains(&bad),
            "{name} must reject {bad:?}: {why}. offered: {offered:?}");
    }
}

/// "Destroy target land. Into the Maw of Hell deals 13 damage to target
/// creature." Two slots wanting different things, and which candidate may go in
/// which is decided when the pairs are enumerated — `is_valid_target` has no
/// slot to key on and legitimately accepts either kind.
///
/// The test this replaces asked `is_valid_target` about a creature and expected
/// "no", which is not a question that function can answer. It also built the
/// creature in `&mut state.clone()` and asked about it in the original state,
/// so its "no" meant "no such object".
#[test]
fn into_the_maw_of_hell_pairs_a_land_with_a_creature_in_that_order() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let land = named_permanent(&mut state, &reg, "Forest", P1);
    let creature = ready_creature(&mut state, P1, 3, 3);
    let maw = castable_spell(&mut state, &reg, "Into the Maw of Hell", P0);

    let sets = offered_target_sets(&state, &reg, maw);
    assert!(!sets.is_empty(), "the spell is castable with a land and a creature out");
    for set in &sets {
        assert_eq!(set.len(), 2, "each offer names both targets; got {set:?}");
        assert_eq!(set[0], Target::Object(land),
            "the first slot is the land it destroys; got {set:?}");
        assert_eq!(set[1], Target::Object(creature),
            "and the second is the creature it burns; got {set:?}");
    }
}

/// "Return target creature card from your graveyard to the battlefield" needs a
/// creature card in a graveyard to point at, so with none anywhere the spell is
/// not castable at all (CR 601.2c) — rather than castable and choosing its
/// target on resolution.
#[test]
fn unburial_rites_is_not_castable_with_no_creature_card_to_return() {
    let reg = registry();

    let mut state = game_at_step(Step::PrecombatMain, P0);
    let rites = castable_spell(&mut state, &reg, "Unburial Rites", P0);
    assert!(!can_cast(&state, &reg, rites), "no graveyard has a creature card in it");

    named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    assert!(can_cast(&state, &reg, rites),
        "and one appearing makes it castable — the assertion above is about the \
         target, not about the mana");
}
