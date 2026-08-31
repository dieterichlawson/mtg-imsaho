//! Zone rules, game-state immutability, and object tracking — including the
//! distinction between looking at a library and drawing from it.

mod common;

use common::*;
use mtg_engine::actions::Action;
use mtg_engine::engine;
use mtg_engine::ids::CardId;
use mtg_engine::types::*;
use mtg_engine::cards::CardRegistry;

/// Rule 400.3: Objects always go to their OWNER's graveyard/hand/library,
/// even if controlled by another player.
#[test]
fn objects_go_to_owners_graveyard() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 owns a creature, but P1 controls it.
    let creature = state.create_object(CardId(99), P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(creature).unwrap().controller = P1;

    // Battlefield filters by controller.
    assert_eq!(state.objects_in_zone(Zone::Battlefield, P0).len(), 0);
    assert_eq!(state.objects_in_zone(Zone::Battlefield, P1).len(), 1);

    // When it dies, it goes to the OWNER's graveyard.
    state.move_object(creature, Zone::Graveyard, &registry);
    assert_eq!(state.objects_in_zone(Zone::Graveyard, P0).len(), 1,
        "Card should go to owner's graveyard (rule 400.3)");
    assert_eq!(state.objects_in_zone(Zone::Graveyard, P1).len(), 0);
}

/// Hand zone filters by owner (rule 400.1).
#[test]
fn hand_filters_by_owner() {
    let mut state = game_at_step(Step::PrecombatMain, P0);

    state.create_object(CardId(1), P0, Zone::Hand, None, None);
    state.create_object(CardId(2), P0, Zone::Hand, None, None);
    state.create_object(CardId(1), P1, Zone::Hand, None, None);

    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 2);
    assert_eq!(state.objects_in_zone(Zone::Hand, P1).len(), 1);
}

/// Verify that `submit_action` returns a new state without modifying the original.
#[test]
fn submit_action_preserves_original_state() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let land = spell_in_hand(&mut state, &registry, "Forest", P0);

    let original_hand_size = state.objects_in_zone(Zone::Hand, P0).len();

    let new_state = engine::submit_action(
        &state, &Action::PlayLand { object_id: land }, &registry,
    );

    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), original_hand_size,
        "Original state should not be modified");
    assert_eq!(state.get_object(land).unwrap().zone, Zone::Hand);
    assert_eq!(new_state.get_object(land).unwrap().zone, Zone::Battlefield);
}

/// Zone change counter increments on each zone change.
#[test]
fn zone_change_counter_increments() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = state.create_object(CardId(99), P0, Zone::Hand, Some(2), Some(2));
    assert_eq!(state.get_object(creature).unwrap().zone_change_count, 0);

    state.move_object(creature, Zone::Battlefield, &registry);
    assert_eq!(state.get_object(creature).unwrap().zone_change_count, 1);

    state.move_object(creature, Zone::Graveyard, &registry);
    assert_eq!(state.get_object(creature).unwrap().zone_change_count, 2);

    state.move_object(creature, Zone::Exile, &registry);
    assert_eq!(state.get_object(creature).unwrap().zone_change_count, 3);
}

/// Leaving the battlefield resets tapped, damage, and summoning sickness.
#[test]
fn leaving_battlefield_resets_state() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 3, 3);

    state.get_object_mut(creature).unwrap().tapped = true;
    state.get_object_mut(creature).unwrap().damage_marked = 2;

    state.move_object(creature, Zone::Graveyard, &registry);

    let obj = state.get_object(creature).unwrap();
    assert!(!obj.tapped);
    assert_eq!(obj.damage_marked, 0);
    assert!(!obj.summoning_sick);
}

/// Creature spell goes on the stack, not directly to battlefield.
#[test]
fn creature_spell_goes_on_stack() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = castable_spell(&mut state, &registry, "Kalonian Tusker", P0);

    let new_state = cast_onto_stack(&state, &registry, creature, vec![]);

    assert_eq!(new_state.get_object(creature).unwrap().zone, Zone::Stack);
    assert_eq!(new_state.stack.len(), 1);
}

/// Creature resolves to battlefield with summoning sickness.
#[test]
fn creature_resolves_with_summoning_sickness() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = castable_spell(&mut state, &registry, "Kalonian Tusker", P0);

    state = cast_and_resolve(&state, &registry, creature, vec![]);

    let obj = state.get_object(creature).unwrap();
    assert_eq!(obj.zone, Zone::Battlefield);
    assert!(obj.summoning_sick);
}

// ============================================================================
// Full integration: tap lands, cast, resolve
// ============================================================================

/// End-to-end: tap two Forests, cast Kalonian Tusker, resolve it.
#[test]
fn full_cast_and_resolve_sequence() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let forest1 = named_permanent(&mut state, &registry, "Forest", P0);
    let forest2 = named_permanent(&mut state, &registry, "Forest", P0);
    let tusker = spell_in_hand(&mut state, &registry, "Kalonian Tusker", P0);

    // Tap Forest 1.
    state = engine::submit_action(
        &state,
        &Action::ActivateManaAbility { object_id: forest1, ability_index: 0 },
        &registry,
    );
    assert_eq!(state.get_player(P0).mana_pool.get(ManaType::Green), 1);

    // Tap Forest 2.
    state = engine::submit_action(
        &state,
        &Action::ActivateManaAbility { object_id: forest2, ability_index: 0 },
        &registry,
    );
    assert_eq!(state.get_player(P0).mana_pool.get(ManaType::Green), 2);

    // Cast Kalonian Tusker.
    state = cast_onto_stack(&state, &registry, tusker, vec![]);
    assert_eq!(state.get_object(tusker).unwrap().zone, Zone::Stack);
    assert_eq!(state.get_player(P0).mana_pool.total(), 0);

    // Resolve.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    let obj = state.get_object(tusker).unwrap();
    assert_eq!(obj.zone, Zone::Battlefield);
    assert!(obj.summoning_sick);
    assert_eq!(obj.power, Some(3));
    assert_eq!(obj.toughness, Some(3));
    assert!(state.stack.is_empty());
}

// -------------------------------------------------------------------------
// Looking at a library is not drawing from it
// -------------------------------------------------------------------------

/// Bug: Mirror-Mad Phantasm's ability uses `draw_top_card` for the reveal loop,
/// which sets `has_drawn_from_empty=true` if library runs out. This causes the
/// player to lose via SBA even though they didn't actually draw from empty.
/// CR 701.15a: revealing is not drawing. `reveal_top_card` must not set
/// `has_drawn_from_empty`, or a reveal loop that runs the library out — the
/// shape Mirror-Mad Phantasm and Trepanation Blade both use — would lose its
/// controller the game to SBA (CR 704.5b).
#[test]
fn revealing_past_the_end_of_a_library_is_not_drawing_from_it() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bears = registry.get_id_by_name("Grizzly Bears").unwrap();
    for _ in 0..3 {
        let id = state.create_object(bears, P0, Zone::Library, Some(2), Some(2));
        state.get_player_mut(P0).library_order.push(id);
    }

    let mut revealed = 0;
    while state.get_player_mut(P0).reveal_top_card().is_some() {
        revealed += 1;
        assert!(revealed <= 3, "reveal_top_card kept handing out cards past the end");
    }
    assert_eq!(revealed, 3, "every card in the library is revealed exactly once");
    assert!(!state.get_player(P0).has_drawn_from_empty,
        "running the library out by revealing must not flag a draw from an empty library");
}

// ── Mutation-motivated guards (reports/mutants-backlog.txt) ──────────────

/// CR 701.20a: only a tapped permanent untaps. `untap` on an untapped
/// permanent is a complete no-op — no `Untapped` event — mirroring what
/// `tap` already documents for the reverse.
#[test]
fn untapping_an_untapped_permanent_emits_nothing() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let c = ready_creature(&mut state, P0, 2, 2);

    state.events.clear();
    state.untap(c);
    assert!(!state.events.iter().any(|e| matches!(e,
        mtg_engine::events::GameEvent::Untapped { object } if *object == c)),
        "an untapped permanent does not untap again — no event");

    state.tap(c);
    state.events.clear();
    state.untap(c);
    assert!(state.events.iter().any(|e| matches!(e,
        mtg_engine::events::GameEvent::Untapped { object } if *object == c)),
        "a tapped permanent untapping emits the event");
    assert!(!state.get_object(c).unwrap().tapped);
}

/// The leaves-the-battlefield log line names the destination: "died" for the
/// graveyard, "was exiled" for exile — and only for a creature leaving the
/// BATTLEFIELD. A card moving hand → graveyard did not die.
#[test]
fn leaving_the_battlefield_is_logged_by_destination() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bear = named_permanent(&mut state, &registry, "Grizzly Bears", P0);
    state.move_object(bear, Zone::Exile, &registry);
    assert!(state.game_log.iter().any(|e| e.message.contains("Grizzly Bears was exiled")),
        "battlefield -> exile logs 'was exiled'");

    let traveler = named_permanent(&mut state, &registry, "Doomed Traveler", P0);
    state.move_object(traveler, Zone::Graveyard, &registry);
    assert!(state.game_log.iter().any(|e| e.message.contains("Doomed Traveler died")),
        "battlefield -> graveyard logs 'died'");

    let in_hand = spell_in_hand(&mut state, &registry, "Elder Cathar", P0);
    let log_len = state.game_log.len();
    state.move_object(in_hand, Zone::Graveyard, &registry);
    assert!(!state.game_log[log_len..].iter().any(|e|
            e.message.contains("Elder Cathar died")
            || e.message.contains("Elder Cathar was exiled")
            || e.message.contains("Elder Cathar left the battlefield")),
        "a card that was never on the battlefield gets no leave-the-battlefield line");
}

/// CR 400.7: leaving the battlefield makes a new object. Runtime-granted
/// subtypes/colors are wiped for a real card (its printed ones live in the
/// registry) but kept for a token (its object fields ARE its printed
/// characteristics), and the cast's X value dies with the permanent.
#[test]
fn leaving_the_battlefield_resets_runtime_grants_but_not_a_tokens_print() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bear = named_permanent(&mut state, &registry, "Grizzly Bears", P0);
    {
        let obj = state.get_object_mut(bear).unwrap();
        obj.subtypes.push("Vampire".into());
        obj.colors.push(Color::Black);
        obj.x_value = Some(5);
    }
    state.move_object(bear, Zone::Graveyard, &registry);
    let obj = state.get_object(bear).unwrap();
    assert!(obj.subtypes.is_empty(), "a granted subtype does not follow a card to the graveyard");
    assert!(obj.colors.is_empty(), "nor a granted color");
    assert_eq!(obj.x_value, None, "X was chosen for one cast (CR 107.3b)");

    let token = *state.create_token_with_subtypes(
        "Wolf", P0, 2, 2, vec![Color::Green], vec![CardType::Creature],
        vec![], vec!["Wolf".into()], &registry)
        .first().expect("token created");
    state.move_object(token, Zone::Graveyard, &registry);
    let tok = state.get_object(token).unwrap();
    assert_eq!(tok.subtypes, vec!["Wolf".to_string()],
        "a token's object-level subtypes are its printed ones and stay");
    assert_eq!(tok.colors, vec![Color::Green]);
}

/// CR 400.7: an until-end-of-turn effect targeting a permanent ends when the
/// permanent leaves the battlefield, so a same-turn return reusing the id is
/// a clean object.
#[test]
fn until_eot_effects_end_when_their_target_leaves_the_battlefield() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let c = ready_creature(&mut state, P0, 2, 2);
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantKeyword {
        target: c, keyword: Keyword::Haste,
    });

    state.move_object(c, Zone::Graveyard, &registry);
    assert!(state.until_end_of_turn.is_empty(),
        "the pump ended with the permanent (CR 400.7); a same-turn return \
         must not inherit it");
}

/// "As [this] enters, choose ..." (CR 614.12) runs exactly when the object
/// enters the battlefield — not when the same card moves between other zones.
#[test]
fn chooses_as_it_enters_fires_on_entering_the_battlefield_and_only_then() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let entering = spell_in_hand(&mut state, &registry, "Nevermore", P0);
    state.move_object(entering, Zone::Battlefield, &registry);
    assert!(matches!(state.awaiting_action,
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            choice: mtg_engine::state::ResolutionChoiceKind::ChooseCardName { .. }, ..
        })),
        "Nevermore chooses its name as it enters, got {:?}", state.awaiting_action);
    state.awaiting_action = None;

    let discarded = spell_in_hand(&mut state, &registry, "Nevermore", P0);
    state.move_object(discarded, Zone::Graveyard, &registry);
    assert!(state.awaiting_action.is_none(),
        "a Nevermore going hand -> graveyard never entered the battlefield; \
         no as-it-enters choice may fire");
}

/// Enters-the-battlefield replacements (counters, tapped) apply to entering
/// the BATTLEFIELD. A card moved hand -> graveyard is not entering anything:
/// Unbreathing Horde ("enters with a +1/+1 counter for each Zombie card in
/// your graveyard") discarded with a full graveyard gains nothing.
#[test]
fn enter_replacements_do_not_run_on_non_battlefield_moves() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_card_in_graveyard(&mut state, &registry, "Walking Corpse", P0);
    named_card_in_graveyard(&mut state, &registry, "Walking Corpse", P0);

    let horde = spell_in_hand(&mut state, &registry, "Unbreathing Horde", P0);
    state.move_object(horde, Zone::Graveyard, &registry);
    assert!(state.get_object(horde).unwrap().counters.is_empty(),
        "no +1/+1 counters for a discard — the enters-with-counters \
         replacement is an entering-the-battlefield event only");

    // The control: actually entering the battlefield does add them.
    let horde2 = spell_in_hand(&mut state, &registry, "Unbreathing Horde", P0);
    state.move_object(horde2, Zone::Battlefield, &registry);
    let n = *state.get_object(horde2).unwrap()
        .counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0);
    assert_eq!(n, 3,
        "two Walking Corpses and the discarded Horde in the graveyard -> three counters");
}
