//! CR 400.7: an object that changes zones is a new object with no memory of
//! its previous existence. `move_object` resets the battlefield-only state
//! on the way out; these tests pin the fields the rulebook sweep found were
//! NOT being reset, and the copy paths whose identity leaked through a
//! zone change.

mod common;
use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind};
use mtg_engine::types::*;

/// CR 602.5b/606.3 with 400.7: "this turn" activation memory belongs to the
/// permanent that used the ability, not to the card it was printed on.
#[test]
fn a_permanents_activation_memory_does_not_follow_it_to_the_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let wolf = named_permanent(&mut state, &reg, "Darkthicket Wolf", P0);
    state.get_object_mut(wolf).unwrap().abilities_activated_this_turn.insert(0);

    state.move_object(wolf, Zone::Graveyard, &reg);

    assert!(state.get_object(wolf).unwrap().abilities_activated_this_turn.is_empty(),
        "a dead permanent has activated nothing (CR 400.7); a same-turn \
         reanimation must not find it locked out");
}

/// The two copy paths write `card_types`, `keywords` and `is_legendary` on a
/// non-token permanent; they are runtime characteristics and stop at the
/// zone change like subtypes and colors already did.
#[test]
fn copied_types_keywords_and_legendary_flag_do_not_survive_leaving_the_battlefield() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = ready_creature(&mut state, P0, 2, 2);
    {
        let o = state.get_object_mut(bear).unwrap();
        o.card_types.push(CardType::Artifact);
        o.keywords.push(Keyword::Flying);
        o.is_legendary = true;
    }
    assert!(state.has_card_type(bear, CardType::Artifact, &reg), "test precondition");

    state.move_object(bear, Zone::Graveyard, &reg);

    let o = state.get_object(bear).unwrap();
    assert!(o.card_types.is_empty() && o.keywords.is_empty() && !o.is_legendary,
        "runtime characteristics are gone in the graveyard: {:?} {:?} legendary={}",
        o.card_types, o.keywords, o.is_legendary);
    assert!(!state.has_card_type(bear, CardType::Artifact, &reg),
        "a graveyard card answers with its printed types only (CR 400.7)");
    assert!(!state.is_legendary(bear, &reg));
}

/// Evil Twin models "enters as a copy" as a choice raised by its ETB. Killed
/// in response, the card in the graveyard is a new object the choice does
/// not concern: the copy must not be written onto it.
#[test]
fn a_copy_choice_answered_after_the_source_died_copies_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);
    let twin_card = state.get_object(twin).unwrap().card_id;
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    // The choice is up, and the Twin is killed before it is answered.
    state.get_object_mut(twin).unwrap().entering_copy_source = true;
    state.move_object(twin, Zone::Graveyard, &reg);
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: P0,
        source: twin,
        choice: ResolutionChoiceKind::ChooseTarget {
            description: "copy".into(),
            options: vec![Target::Object(bear)],
            optional: false,
            effect: PendingEffect::CopyCreature { source_id: twin },
        },
    });

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(Target::Object(bear))) },
        &reg,
    );

    let o = state.get_object(twin).unwrap();
    assert_eq!(o.card_id, twin_card, "the graveyard card is still an Evil Twin");
    assert!(o.copy_grantor.is_none());
    assert_eq!(state.name_of(twin, &reg), "Evil Twin");
    assert!(!o.entering_copy_source, "the SBA copy-guard is disarmed either way");
}

/// CR 707.8/707.8a: a copy of a transformed permanent copies the face that is
/// up, and shows it. The token used to take the back face's name, P/T and
/// subtypes while its `card_id` said "front face up": a Human-Werewolf
/// chimera whose accessors disagreed with each other.
#[test]
fn a_token_copy_of_a_transformed_permanent_shows_the_back_face() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let smith = named_permanent(&mut state, &reg, "Village Ironsmith", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, smith, &reg);
    assert_eq!(state.name_of(smith, &reg), "Ironfang", "test precondition");

    let token = state.create_token_copy(smith, P0, &reg);

    assert_eq!(state.name_of(token, &reg), "Ironfang");
    let subtypes = state.subtypes_of(token, &reg);
    assert!(subtypes.iter().any(|s| s == "Werewolf") && !subtypes.iter().any(|s| s == "Human"),
        "the copy has the back face's subtypes only, got {subtypes:?}");
    assert!(state.get_object(token).unwrap().is_transformed);

    // CR 701.28c: the token is not a double-faced card, so it never
    // transforms back.
    mtg_engine::cards::helpers::apply_transform(&mut state, token, &reg);
    assert_eq!(state.name_of(token, &reg), "Ironfang");
}

/// The same for Evil Twin copying a transformed permanent — and since the
/// card under the copy is single-faced, "transform" does nothing to it
/// (CR 701.28c), and leaving the battlefield restores the printed Twin.
#[test]
fn an_evil_twin_copying_a_transformed_permanent_shows_the_back_face_and_cannot_transform() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let smith = named_permanent(&mut state, &reg, "Village Ironsmith", P1);
    mtg_engine::cards::helpers::apply_transform(&mut state, smith, &reg);
    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: P0,
        source: twin,
        choice: ResolutionChoiceKind::ChooseTarget {
            description: "copy".into(),
            options: vec![Target::Object(smith)],
            optional: false,
            effect: PendingEffect::CopyCreature { source_id: twin },
        },
    });
    let mut state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(Target::Object(smith))) },
        &reg,
    );

    assert_eq!(state.name_of(twin, &reg), "Ironfang");
    assert!(state.has_subtype(twin, "Werewolf", &reg) && !state.has_subtype(twin, "Human", &reg));

    mtg_engine::cards::helpers::apply_transform(&mut state, twin, &reg);
    assert_eq!(state.name_of(twin, &reg), "Ironfang",
        "a single-faced card showing a copied back face cannot transform (CR 701.28c)");

    state.move_object(twin, Zone::Graveyard, &reg);
    let o = state.get_object(twin).unwrap();
    assert!(!o.is_transformed && o.copy_grantor.is_none());
    assert_eq!(state.name_of(twin, &reg), "Evil Twin", "CR 400.7: the printed card again");
}

/// A "for as long as" control effect is about the permanent it was created
/// for; the id it carries must not follow that permanent through the
/// graveyard onto whatever comes back under the same id.
#[test]
fn a_control_effect_ends_when_its_object_leaves_the_battlefield() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let vampire = named_permanent(&mut state, &reg, "Markov Patrician", P1);
    state.gain_control_while_source_controlled(vampire, olivia, &reg);
    assert_eq!(state.control_effects.len(), 1, "test precondition");

    mtg_engine::destruction::try_destroy(&mut state, vampire, &reg);
    assert!(state.control_effects.is_empty(),
        "the effect ended with the object it was over (CR 400.7)");

    // Reanimated under the thief (Grimoire of the Dead's shape) — a new object.
    state.move_object(vampire, Zone::Battlefield, &reg);
    state.change_control(vampire, P0);
    mtg_engine::destruction::try_destroy(&mut state, olivia, &reg);
    mtg_engine::sba::check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(vampire).unwrap().controller, P0,
        "Olivia leaving ends nothing about the new object");
}
