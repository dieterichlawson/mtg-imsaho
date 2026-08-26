//! Terror of Kruin Pass (Kruin Outlaw's back face): "Double strike. Werewolves
//! you control can't be blocked except by two or more creatures."
//!
//! Menace granted by a permanent to a class of other permanents, which means
//! three separate claims — it reaches Werewolves, it reaches its own controller's
//! only, and it reaches itself ("Werewolves you control" includes it).

mod common;
use common::*;
use mtg_engine::types::*;

/// Put Terror of Kruin Pass — the back face — onto `owner`'s battlefield.
fn terror_of_kruin_pass(
    state: &mut mtg_engine::state::GameState,
    reg: &mtg_engine::cards::CardRegistry,
    owner: PlayerId,
) -> ObjectId {
    let id = named_permanent(state, reg, "Kruin Outlaw", owner);
    // Through the engine's own transform, so the object ends up in the state a
    // real transform leaves it in rather than one the test invented.
    mtg_engine::cards::helpers::apply_transform(state, id, reg);
    assert_eq!(state.get_object(id).unwrap().name, "Terror of Kruin Pass", "test setup");
    id
}

/// Declare `blockers` against `attacker` and report how many the engine kept.
fn blockers_accepted(
    state: &mut mtg_engine::state::GameState,
    reg: &mtg_engine::cards::CardRegistry,
    attacker: ObjectId,
    defender: PlayerId,
    blockers: &[ObjectId],
) -> usize {
    let mut combat = mtg_engine::state::CombatState::new();
    combat.attackers.insert(attacker, defender);
    combat.blocker_assignments.insert(attacker, vec![]);
    state.combat = Some(combat);

    let pairs: Vec<_> = blockers.iter().map(|b| (*b, attacker)).collect();
    submit_declare_blockers(state, defender, &pairs, reg);
    state.combat.as_ref().unwrap().blocker_assignments[&attacker].len()
}

/// Who the menace reaches, and who it does not.
///
/// The negative rows are what separate this from a Terror that gave menace to
/// every creature on the battlefield — including the opponent's, which would
/// be helping them.
#[test]
fn terror_of_kruin_pass_gives_menace_to_your_werewolves_only() {
    // (whose creature, its subtype, does one blocker suffice)
    const CASES: &[(bool, &str, bool, &str)] = &[
        (true, "Werewolf", false, "your Werewolf needs two blockers"),
        (true, "Human", true, "a non-Werewolf of yours does not"),
        (false, "Werewolf", true, "and neither does an opponent's Werewolf"),
    ];

    for &(yours, subtype, one_blocker_is_enough, why) in CASES {
        let reg = registry();
        let (attacker_owner, defender) = if yours { (P0, P1) } else { (P1, P0) };
        let mut state = game_at_step(Step::DeclareBlockers, attacker_owner);
        state.active_player = attacker_owner;

        terror_of_kruin_pass(&mut state, &reg, P0);

        let attacker = ready_creature(&mut state, attacker_owner, 3, 3);
        state.get_object_mut(attacker).unwrap().subtypes = vec![subtype.into()];
        let blocker = ready_creature(&mut state, defender, 2, 2);

        let accepted = blockers_accepted(&mut state, &reg, attacker, defender, &[blocker]);
        assert_eq!(accepted == 1, one_blocker_is_enough, "{why}");
    }
}

/// "Werewolves you control" includes the Terror itself, so it needs two
/// blockers too — and two is enough.
#[test]
fn terror_of_kruin_pass_needs_two_blockers_itself() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let terror = terror_of_kruin_pass(&mut state, &reg, P0);
    let first = ready_creature(&mut state, P1, 2, 2);
    let second = ready_creature(&mut state, P1, 2, 2);

    assert_eq!(blockers_accepted(&mut state, &reg, terror, P1, &[first]), 0,
        "one blocker is turned away");
    assert_eq!(blockers_accepted(&mut state, &reg, terror, P1, &[first, second]), 2,
        "two are accepted — the restriction is 'except by two or more', not \
         'can't be blocked'");
}

/// The restriction is the Menace keyword, so it shows up in `has_keyword` and
/// not only in the blocker validation.
#[test]
fn terror_of_kruin_pass_grants_menace_as_a_keyword() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let terror = terror_of_kruin_pass(&mut state, &reg, P0);
    let wolf = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(wolf).unwrap().subtypes = vec!["Werewolf".into()];
    let human = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(human).unwrap().subtypes = vec!["Human".into()];

    assert!(state.has_keyword(terror, Keyword::Menace, &reg), "the Terror itself");
    assert!(state.has_keyword(wolf, Keyword::Menace, &reg), "another of your Werewolves");
    assert!(!state.has_keyword(human, Keyword::Menace, &reg), "but not a non-Werewolf");
}
