//! Copy effects: what gets copied, what may be chosen, and which abilities
//! trigger when the copy arrives.
//!
//! CR 707.2 defines the copiable values; CR 614.12 says a permanent that
//! "enters as a copy" enters already bearing them, so the abilities that
//! trigger on it entering are the COPIED creature's; and CR 115.1 says an
//! effect targets only where the word "target" appears — so "a copy of any
//! creature on the battlefield" is a choice, not a target.

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::state::{GameState, PendingEffect};
use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::*;

fn copy_onto(state: &mut GameState, reg: &CardRegistry, copier: ObjectId, victim: ObjectId) {
    mtg_engine::engine::apply_pending_effect(
        state, &Target::Object(victim),
        &PendingEffect::CopyCreature { source_id: copier }, reg);
}

// ── Evil Twin: choosing is not targeting ─────────────────────────

/// CR 115.1: hexproof and protection restrict TARGETING. Evil Twin's copy is
/// a choice (CR 614.12b), so an opponent's hexproof creature is fair game.
#[test]
fn evil_twin_may_copy_a_hexproof_creature_it_could_not_target() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);
    let hexproof = named_permanent(&mut state, &reg, "Walking Corpse", P1);
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantKeyword {
        target: hexproof, keyword: Keyword::Hexproof,
    });
    assert!(state.has_keyword(hexproof, Keyword::Hexproof, &reg), "test precondition");

    // Drive Evil Twin's own enters-the-battlefield handler, so this covers the
    // card's candidate list rather than the helper in isolation.
    let behavior = reg.get(state.get_object(twin).unwrap().card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, twin, &[], &reg);

    let options = match &state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. }, ..
        }) => options.clone(),
        other => panic!("Evil Twin should offer a copy choice, got {other:?}"),
    };

    assert!(options.contains(&Target::Object(hexproof)),
        "an opponent's hexproof creature can't be TARGETED but can be chosen \
         to copy — 'any creature on the battlefield' has no 'target' in it; \
         offered {options:?}");
}

/// A generic token's printed keywords live on the object, not in the
/// registry, so copying one used to drop them.
#[test]
fn copying_a_token_preserves_its_keywords() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);
    let token = *state.create_token_with_subtypes(
        "Spirit", P1, 1, 1, vec![Color::White], vec![CardType::Creature],
        vec![Keyword::Flying], vec!["Spirit".into()], &reg)
        .first().expect("token created");
    assert!(state.has_keyword(token, Keyword::Flying, &reg), "test precondition");

    copy_onto(&mut state, &reg, twin, token);

    assert!(state.has_keyword(twin, Keyword::Flying, &reg),
        "the copy has the token's flying — a generic token has no registry \
         entry, so its printed keywords are the ones on the object");
}

/// CR 614.12: the permanent enters as the copy, so the copied creature's
/// enters-the-battlefield ability is the one that triggers.
#[test]
fn a_copy_fires_the_copied_creatures_etb_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);
    // Fiend Hunter's ETB exiles a creature — an unmistakable ability.
    let hunter = named_permanent(&mut state, &reg, "Fiend Hunter", P1);

    state.pending_triggers.clear();
    copy_onto(&mut state, &reg, twin, hunter);

    let queued = state.pending_triggers.iter().any(|t| matches!(t,
        PendingTrigger { source: TriggerSource { id: object_id, .. }, event: TriggerEvent::SelfEntered }
        if *object_id == twin));
    assert!(queued,
        "copying a creature with an enters-the-battlefield ability must raise \
         that ability for the copy (CR 614.12); it was silently lost");
}

/// Copying something with no ETB ability raises nothing.
#[test]
fn a_copy_of_a_vanilla_creature_raises_no_etb_trigger() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);
    let vanilla = named_permanent(&mut state, &reg, "Walking Corpse", P1);

    state.pending_triggers.clear();
    copy_onto(&mut state, &reg, twin, vanilla);

    assert!(state.pending_triggers.is_empty(),
        "Walking Corpse has no enters-the-battlefield ability, so nothing \
         should be queued; got {:?}", state.pending_triggers);
}

// ── Cackling Counterpart ─────────────────────────────────────────

/// The basic case, which had no coverage at all: a token copy of a creature
/// you control, with the copied creature's characteristics.
#[test]
fn cackling_counterpart_creates_a_token_copy() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let original = named_permanent(&mut state, &reg, "Avacyn's Pilgrim", P0);
    let spell = castable_spell(&mut state, &reg, "Cackling Counterpart", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(original)]);

    let copies: Vec<&mtg_engine::state::GameObject> = state.objects.values()
        .filter(|o| o.is_token && o.zone == Zone::Battlefield)
        .collect();
    assert_eq!(copies.len(), 1, "exactly one token copy should exist");

    let token = copies[0].id;
    assert_eq!(state.name_of(token, &reg), "Avacyn's Pilgrim",
        "the token copies the creature's name");
    assert!(state.has_subtype(token, "Human", &reg),
        "and its subtypes; subtypes_of = {:?}", state.subtypes_of(token, &reg));
    assert_eq!(state.effective_power(token, &reg), state.effective_power(original, &reg),
        "and its power");
}

/// The token is a copy of the card, so the copied creature's ETB ability
/// triggers when the token enters.
#[test]
fn a_token_copy_fires_the_copied_creatures_etb_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mentor = named_permanent(&mut state, &reg, "Mentor of the Meek", P0);
    let original = named_permanent(&mut state, &reg, "Avacyn's Pilgrim", P0);

    // A 1/1 token entering under P0's control satisfies Mentor's condition.
    state.stack.clear();
    let token = state.create_token_copy(original, P0, &reg);
    assert_eq!(state.name_of(token, &reg), "Avacyn's Pilgrim", "test precondition");
    mtg_engine::triggers::collect_triggers(&mut state, &reg);

    let mentor_triggered = state.stack.iter().any(|e| matches!(e,
        mtg_engine::state::StackEntry::Trigger(
            PendingTrigger {
                source: TriggerSource { id: watcher_id, .. },
                event: TriggerEvent::CreatureEntered { .. },
            })
        if *watcher_id == mentor));
    assert!(mentor_triggered,
        "a token copy entering the battlefield is a creature entering, and \
         watchers must see it");
}


/// A copy takes the source's colours. When the source is a card, they come
/// from its face; when the source is a token, they live on the object — two
/// different places for `create_token_copy` to read from, so both are checked.
#[test]
fn a_token_copy_takes_its_sources_colors_from_wherever_they_live() {
    let reg = registry();

    // From a card's face.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bears = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let cc = castable_spell(&mut state, &reg, "Cackling Counterpart", P0);
    let state = cast_and_resolve(&state, &reg, cc, vec![Target::Object(bears)]);
    let token = find_token_named(&state, "Grizzly Bears").expect("token copy exists");
    assert_eq!(state.get_object(token).unwrap().colors, vec![Color::Green],
        "the copy of a green Bear is green — an empty colour list is the bug \
         this catches, and so is the wrong colour");

    // From a token's own object.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let zombie = state.create_token_with_subtypes(
        "Zombie", P0, 2, 2, vec![Color::Black], vec![CardType::Creature],
        vec![], vec!["Zombie".to_string()], &reg)[0];
    let copy = state.create_token_copy(zombie, P0, &reg);
    let copy_obj = state.get_object(copy).expect("copy token exists");

    assert_eq!((copy_obj.power, copy_obj.toughness), (Some(2), Some(2)), "a copy of a 2/2 is a 2/2");
    assert!(copy_obj.card_types.contains(&CardType::Creature),
        "card types carry over, got {:?}", copy_obj.card_types);
    assert!(copy_obj.subtypes.contains(&"Zombie".to_string()),
        "subtypes carry over, got {:?}", copy_obj.subtypes);
    assert!(copy_obj.colors.contains(&Color::Black),
        "and colour, got {:?}", copy_obj.colors);
}

// ── Evil Twin's "except it has..." clause ────────────────────────


/// "You may have this creature enter as a copy of any creature on the
/// battlefield, except it has '{U}{B}, {T}: Destroy target creature with the
/// same name as this creature.'"
///
/// The granted ability comes from the "except it has" clause, which only
/// applies once a copy is actually made. Ruling: "You can choose not to copy
/// anything. In that case, Evil Twin enters as a 0/0 creature" — with no
/// destroy ability.
///
/// The previous version asserted `!(has_marker && has_choice)`, which is
/// satisfied by either half being false — including by an Evil Twin that never
/// offered a choice at all. Both halves are asserted separately now.
#[test]
fn evil_twin_is_not_marked_as_a_copy_until_the_choice_is_made() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    ready_creature(&mut state, P1, 3, 3);
    let twin = castable_spell(&mut state, &reg, "Evil Twin", P0);
    state = cast_and_resolve(&state, &reg, twin, vec![]);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert!(state.awaiting_action.is_some(),
        "with a creature on the battlefield, the copy choice must be offered");
    assert!(state.get_object(twin).is_some_and(|o| o.copy_grantor.is_none()),
        "and until it is answered nothing has been copied, so the 'except it \
         has' ability must not be granted yet");
}

/// After the copy, the granted ability is still reachable — the copy changes
/// which card the permanent's abilities are looked up from, and the "except it
/// has" clause has to survive that.
#[test]
fn evil_twin_keeps_its_granted_ability_after_copying() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let victim = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);

    reg.get(state.get_object(twin).unwrap().card_id).unwrap()
        .on_enter_battlefield(&mut state, twin, &[], &reg);

    assert!(state.awaiting_action.is_some(), "the copy choice is offered");
    state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(victim))),
    }, &reg);

    add_mana(&mut state, P0, &[(ManaType::Blue, 1), (ManaType::Black, 1)]);
    assert!(offers_ability_of(&state, &reg, twin),
        "{{U}}{{B}}, {{T}}: Destroy target creature with the same name — the \
         ability is granted by the copy effect, not by the copied card, so \
         looking abilities up from the new card_id must not lose it");
}

/// CR 614.1d: "enter as a copy" is a replacement effect, so the permanent is
/// already the copy when it arrives. Evil Twin's printed body is 0/0, and a
/// 0/0 on the battlefield dies to SBA 704.5f — so entry has to hold the
/// state-based check off until the copy has had its chance to apply.
#[test]
fn evil_twin_survives_state_based_actions_while_its_copy_choice_is_pending() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Something to copy, so the choice is not a silent no-op.
    ready_creature(&mut state, P1, 2, 2);

    // Enter through the real chokepoint, which arms the copy guard.
    let card_id = reg.get_id_by_name("Evil Twin").unwrap();
    let twin = state.create_object(card_id, P0, Zone::Hand, Some(0), Some(0));
    state.get_object_mut(twin).unwrap().name = "Evil Twin".into();
    state.move_object(twin, Zone::Battlefield, &reg);

    mtg_engine::sba::check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(twin).map(|o| o.zone), Some(Zone::Battlefield),
        "the printed 0/0 must not be swept away before the copy applies");
}

// ── CR 707.2: copiable values only ───────────────────────────────

/// Evil Twin's ruling: "Evil Twin copies exactly what was printed on the
/// original creature ... It doesn't copy ... any non-copy effects that have
/// changed its power, toughness, types, color, or so on."
///
/// Olivia Voldaren's "That creature becomes a Vampire in addition to its other
/// types" is exactly such an effect, and it is implemented by writing
/// `obj.subtypes` — the same object vector that stands in for *printed*
/// subtypes on a token. The copy used to read that vector directly, so it
/// picked up the granted Vampire type along with the real ones.
#[test]
fn a_copy_does_not_pick_up_a_subtype_granted_by_a_non_copy_effect() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // A real card with a registry face, so `printed_subtypes_of` has a face to
    // read. Ambush Viper is a Snake.
    let victim = named_permanent(&mut state, &reg, "Ambush Viper", P1);
    assert!(state.has_subtype(victim, "Snake", &reg), "printed subtype");

    // Olivia grants the Vampire type by writing the object vector.
    state.get_object_mut(victim).unwrap().subtypes.push("Vampire".into());
    assert!(state.has_subtype(victim, "Vampire", &reg), "granted subtype is live");

    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);
    copy_onto(&mut state, &reg, twin, victim);

    assert!(state.has_subtype(twin, "Snake", &reg),
        "the copy has the printed subtypes");
    assert!(!state.has_subtype(twin, "Vampire", &reg),
        "but not one a non-copy effect granted to the original");
}

/// The same rule for colors. Grimoire of the Dead's "becomes black" is a
/// non-copy effect written into `obj.colors`.
#[test]
fn a_copy_does_not_pick_up_a_color_granted_by_a_non_copy_effect() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Ambush Viper is green ({1}{G}).
    let victim = named_permanent(&mut state, &reg, "Ambush Viper", P1);
    state.get_object_mut(victim).unwrap().colors.push(Color::Black);
    assert!(state.colors_of(victim, &reg).contains(&Color::Black),
        "granted colour is live");

    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);
    copy_onto(&mut state, &reg, twin, victim);

    assert!(state.colors_of(twin, &reg).contains(&Color::Green),
        "the copy is the printed colour");
    assert!(!state.colors_of(twin, &reg).contains(&Color::Black),
        "but not one a non-copy effect granted to the original");
}

/// The token case the object vectors exist for must keep working: a token has
/// no registry face, so its object vectors ARE its printed characteristics.
#[test]
fn a_copy_of_a_token_still_takes_the_tokens_printed_characteristics() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ids = state.create_token_with_subtypes(
        "Spirit", P1, 1, 1, vec![Color::White], vec![CardType::Creature],
        vec![Keyword::Flying], vec!["Spirit".into()], &reg);
    let token = ids[0];

    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);
    copy_onto(&mut state, &reg, twin, token);

    assert!(state.has_subtype(twin, "Spirit", &reg), "token subtypes still copied");
    assert!(state.has_keyword(twin, Keyword::Flying, &reg), "and its keywords");
    assert_eq!(state.effective_power(twin, &reg), Some(1));
    assert!(!state.get_object(twin).unwrap().is_token,
        "copying a token does not make Evil Twin a token");
}

/// CR 111.7: a token that is a copy of a double-faced card is not itself
/// double-faced — it has only the copied face — so it cannot transform.
///
/// `apply_transform` enforces this for every DFC at once, but Bloodline
/// Keeper's `{B}: Transform` set `is_transformed` and the name by hand and so
/// went around it. Cackling Counterpart makes exactly such a token.
#[test]
fn a_token_copy_of_bloodline_keeper_cannot_transform() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let keeper = named_permanent(&mut state, &reg, "Bloodline Keeper", P0);

    // A token copy of it, as Cackling Counterpart would make.
    let ids = state.create_token_with_subtypes(
        "Bloodline Keeper", P0, 3, 3, vec![Color::Black], vec![CardType::Creature],
        vec![Keyword::Flying], vec!["Vampire".into()], &reg);
    let token = ids[0];
    state.get_object_mut(token).unwrap().card_id =
        state.get_object(keeper).unwrap().card_id;

    // Five Vampires so the ability's own condition is satisfied.
    for _ in 0..4 {
        let v = ready_creature(&mut state, P0, 1, 1);
        state.get_object_mut(v).unwrap().subtypes = vec!["Vampire".into()];
    }

    let behavior = reg.get(state.get_object(token).unwrap().card_id).unwrap();
    let abilities = behavior.activated_abilities(&state, token, &reg);
    assert!(abilities.iter().all(|a| !a.description.contains("Transform")),
        "a token copy is not offered a transform it could not perform");

    // And even driven directly, it does not flip.
    behavior.resolve_activated_ability(&mut state, token, 1, &[], &reg);
    assert!(!state.get_object(token).unwrap().is_transformed,
        "CR 111.7: a token copy of a DFC cannot transform");
}

/// A copy of a real card takes the card's colors, which for a card with no
/// color indicator derive from its mana cost (CR 202.2). Walking Corpse costs
/// {1}{B}: the copy is black — via the cost-derivation path, not a runtime
/// grant (those are covered elsewhere).
#[test]
fn a_copy_derives_the_copied_cards_colors_from_its_cost() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);
    let victim = named_permanent(&mut state, &reg, "Walking Corpse", P1);
    copy_onto(&mut state, &reg, twin, victim);

    let colors = &state.get_object(twin).unwrap().colors;
    assert_eq!(colors, &vec![Color::Black],
        "the copy's color comes from the copied card's {{1}}{{B}} cost");
}
