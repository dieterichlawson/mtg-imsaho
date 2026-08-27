//! CR 109.1: a token is not a card.
//!
//! Oracle text is precise about this. "Each Zombie card in your graveyard"
//! counts cards; "each other Zombie you control" counts permanents, tokens
//! included. Several cards read the graveyard with a filter that had no
//! `is_token` check, so a token sitting in the graveyard was counted as a
//! card.
//!
//! That window is real. CR 704.5e removes a token from a non-battlefield zone
//! as a state-based action, and SBAs are a discrete pass that runs when a
//! player would receive priority — not the instant the token arrives. Anything
//! that reads the graveyard mid-resolution (an ETB replacement effect
//! computing entering counters, a characteristic-defining ability recomputed
//! during combat damage) sees tokens that are on their way out.
//!
//! Also in this file: two cards that were reading state at the wrong moment —
//! Tree of Redemption resolving after it left the battlefield, and
//! Mindshrieker milling without announcing it.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

/// Put a creature token straight into the graveyard, modelling the window
/// between the token arriving there and the next SBA pass sweeping it up.
fn creature_token_in_graveyard(
    state: &mut mtg_engine::state::GameState,
    reg: &CardRegistry,
    name: &str,
    subtypes: Vec<String>,
    owner: PlayerId,
) -> ObjectId {
    let token = *state.create_token_with_subtypes(
        name, owner, 2, 2, vec![Color::Black], vec![CardType::Creature],
        vec![], subtypes, reg)
        .first().expect("token should be created");
    state.move_object(token, Zone::Graveyard, reg);
    assert!(!state.is_card(token), "test precondition: a token is not a card");
    token
}

// ---------------------------------------------------------------------------
// Unbreathing Horde: "enters with a +1/+1 counter on it for each other Zombie
// you control and each Zombie CARD in your graveyard."
// ---------------------------------------------------------------------------

#[test]
fn zombie_token_in_graveyard_not_counted() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    creature_token_in_graveyard(&mut state, &reg, "Zombie", vec!["Zombie".into()], P0);

    // The Horde is still in hand — `entering_with_counters` is a replacement
    // effect, consulted before the zone change.
    let horde = spell_in_hand(&mut state, &reg, "Unbreathing Horde", P0);
    let counters = plan_entering(&mut state, &reg, horde, Some(Zone::Hand)).counters;

    assert!(counters.is_empty(),
        "a Zombie TOKEN in the graveyard is not a Zombie card (CR 109.1); got {counters:?}");
}

#[test]
fn zombie_card_in_graveyard_still_counted() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Walking Corpse is a real Zombie card.
    let corpse = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    assert!(state.is_card(corpse) && state.has_subtype(corpse, "Zombie", &reg),
        "test precondition: Walking Corpse is a Zombie card");

    let horde = spell_in_hand(&mut state, &reg, "Unbreathing Horde", P0);
    let counters = plan_entering(&mut state, &reg, horde, Some(Zone::Hand)).counters;

    assert_eq!(counters, vec![(CounterType::PlusOnePlusOne, 1)],
        "the !is_token guard must not exclude a real Zombie card");
}

/// The battlefield half of the same ability says "each other Zombie you
/// control", with no "card" — so it counts tokens, and must keep doing so.
#[test]
fn zombie_token_on_the_battlefield_is_still_counted() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    state.create_token_with_subtypes(
        "Zombie", P0, 2, 2, vec![Color::Black], vec![CardType::Creature],
        vec![], vec!["Zombie".into()], &reg);

    let horde = spell_in_hand(&mut state, &reg, "Unbreathing Horde", P0);
    let counters = plan_entering(&mut state, &reg, horde, Some(Zone::Hand)).counters;

    assert_eq!(counters, vec![(CounterType::PlusOnePlusOne, 1)],
        "\"each other Zombie you control\" says Zombie, not Zombie card — tokens count");
}

// ---------------------------------------------------------------------------
// Splinterfright: "power and toughness are each equal to the number of
// creature CARDS in your graveyard."
// ---------------------------------------------------------------------------

#[test]
fn cda_does_not_count_tokens_in_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let fright = named_permanent(&mut state, &reg, "Splinterfright", P0);
    creature_token_in_graveyard(&mut state, &reg, "Spirit", vec!["Spirit".into()], P0);

    assert_eq!(state.effective_power(fright, &reg), Some(0),
        "a creature token in the graveyard is not a creature card (CR 109.1)");

    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    assert_eq!(state.effective_power(fright, &reg), Some(1),
        "a real creature card in the graveyard does count");
}

// ---------------------------------------------------------------------------
// Tree of Redemption: "{T}: Exchange your life total with this creature's
// toughness." An activated ability stays on the stack when its source leaves
// the battlefield; there is then no "this creature" to exchange with.
// ---------------------------------------------------------------------------

/// Destroyed in response: nothing is exchanged.
#[test]
fn tree_destroyed_in_response_no_exchange() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let tree = named_permanent(&mut state, &reg, "Tree of Redemption", P0);
    state.get_player_mut(P0).life = 4;

    state.move_object(tree, Zone::Graveyard, &reg);

    reg.get(state.get_object(tree).unwrap().card_id).unwrap()
        .resolve_activated_ability(&mut state, tree, 0, &[], &reg);

    assert_eq!(state.get_player(P0).life, 4,
        "the Tree left the battlefield before the ability resolved — no exchange");
}

/// Bounced in response: same, and from a different zone.
#[test]
fn tree_bounced_in_response_no_exchange() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let tree = named_permanent(&mut state, &reg, "Tree of Redemption", P0);
    state.get_player_mut(P0).life = 4;

    state.move_object(tree, Zone::Hand, &reg);

    reg.get(state.get_object(tree).unwrap().card_id).unwrap()
        .resolve_activated_ability(&mut state, tree, 0, &[], &reg);

    assert_eq!(state.get_player(P0).life, 4,
        "the Tree was bounced before the ability resolved — no exchange");
    assert_eq!(state.get_object(tree).unwrap().toughness, Some(13),
        "and the card in hand keeps its printed toughness");
}

/// The ability still works when the Tree is where it should be.
#[test]
fn tree_on_the_battlefield_exchanges_normally() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let tree = named_permanent(&mut state, &reg, "Tree of Redemption", P0);
    state.get_player_mut(P0).life = 4;

    reg.get(state.get_object(tree).unwrap().card_id).unwrap()
        .resolve_activated_ability(&mut state, tree, 0, &[], &reg);

    assert_eq!(state.get_player(P0).life, 13, "life becomes the Tree's toughness");
    assert_eq!(state.get_object(tree).unwrap().toughness, Some(4),
        "and the Tree's toughness becomes the old life total");
}

// ---------------------------------------------------------------------------
// Mindshrieker: milling has to be announced, not just performed.
// ---------------------------------------------------------------------------

/// Undead Alchemist watches for a creature card put into an opponent's
/// graveyard from their library. Mindshrieker used to move the card by hand,
/// so no `CreatureCardMilled` event was emitted and the Alchemist never fired.
#[test]
fn mindshrieker_milled_creature_triggers_undead_alchemist() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    named_permanent(&mut state, &reg, "Undead Alchemist", P0);
    let shrieker = named_permanent(&mut state, &reg, "Mindshrieker", P0);

    // Opponent's library: one creature card on top.
    let top = state.create_object(
        reg.get_id_by_name("Walking Corpse").unwrap(), P1, Zone::Library, Some(2), Some(2));
    state.get_player_mut(P1).library_order.push(top);

    let zombies_before = state.objects_in_zone(Zone::Battlefield, P0).iter()
        .filter(|o| o.is_token).count();

    activate_via_hooks(&mut state, &reg, shrieker, 0,
        &[mtg_engine::actions::Target::Player(P1)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(top).unwrap().zone, Zone::Exile,
        "Undead Alchemist exiles the milled creature card");
    let zombies_after = state.objects_in_zone(Zone::Battlefield, P0).iter()
        .filter(|o| o.is_token).count();
    assert_eq!(zombies_after, zombies_before + 1,
        "and creates a 2/2 Zombie token");
}

/// CR 109.1: a token is not a card. Every ISD card whose text counts or picks
/// "creature **cards** in your graveyard" has to skip a token sitting there.
///
/// A token in a graveyard is not a hypothetical state — CR 111.7 removes it as
/// a state-based action, so between the moment it dies and the next SBA check
/// it really is in the graveyard, which is exactly the window a dies-trigger
/// sees. Boneyard Wurm read 2/2 in that window with one creature card and one
/// dead token in the yard, and 1/1 the instant SBAs ran.
///
/// Unbreathing Horde was the one card that already got this right, and its
/// comment says why: its own text says "each other **Zombie** you control"
/// (tokens count) and "each Zombie **card** in your graveyard" (they do not).
#[test]
fn a_token_in_a_graveyard_is_not_a_creature_card() {
    let reg = registry();
    // (card, how to read its "creature cards in your graveyard" count)
    let mut checked = 0;
    for name in ["Boneyard Wurm", "Splinterfright"] {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let pt_source = named_permanent(&mut state, &reg, name, P0);
        named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
        let before = state.effective_power(pt_source, &reg);

        // A creature token dies and lands in the graveyard. It is still there
        // until the next state-based action check.
        let token = state.create_token(
            "Zombie", P0, 2, 2, vec![], vec![CardType::Creature], vec![], &reg)[0];
        state.move_object(token, Zone::Graveyard, &reg);
        assert!(state.get_object(token).is_some(),
            "{name}: test premise — the token is in the graveyard until SBAs run");

        assert_eq!(state.effective_power(pt_source, &reg), before,
            "{name}: a token in the graveyard is not a creature card (CR 109.1), \
             so the count must not move");
        checked += 1;
    }
    assert!(checked >= 2, "only {checked} cards checked — the sweep stopped covering anything");
}

/// The other four cards that read a graveyard for "cards" and did not ask.
///
/// Each one presents the graveyard to a player as a list to pick from, or picks
/// from it at random. A creature token waiting on the next SBA pass was in the
/// list.
#[test]
fn a_token_in_a_graveyard_is_not_offered_as_a_card_to_choose() {
    use mtg_engine::actions::Target;
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Graveyard Shovel, with one real card and one token in P1's graveyard.
    let shovel = named_permanent(&mut state, &reg, "Graveyard Shovel", P0);
    let real = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P1);
    let token = creature_token_in_graveyard(&mut state, &reg, "Zombie", vec!["Zombie".into()], P1);

    activate_via_hooks(&mut state, &reg, shovel, 0, &[Target::Player(P1)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // One card and one token: only the card is a choice, so there is nothing to
    // choose between and it is exiled outright.
    assert!(state.awaiting_action.is_none()
        || !matches!(&state.awaiting_action,
            Some(AwaitingAction::ResolutionChoice {
                choice: ResolutionChoiceKind::ChooseTarget { options, .. }, .. })
            if options.contains(&Target::Object(token))),
        "\"exiles a card from their graveyard\" must not offer a token (CR 109.1); \
         got {:?}", state.awaiting_action);
    assert_eq!(state.get_object(real).unwrap().zone, Zone::Exile,
        "the one actual card is the one exiled");
}

/// Moldgraf Monstrosity returns "two creature **cards** at random"; a token in
/// the graveyard must not be one of the two it can draw.
#[test]
fn moldgraf_does_not_return_a_token_from_the_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let monstrosity = named_permanent(&mut state, &reg, "Moldgraf Monstrosity", P0);
    let token = creature_token_in_graveyard(&mut state, &reg, "Zombie", vec!["Zombie".into()], P0);
    mtg_engine::destruction::try_destroy(&mut state, monstrosity, &reg);

    reg.get(state.get_object(monstrosity).unwrap().card_id).unwrap()
        .on_dies(&mut state, monstrosity, &[], &reg);

    assert_ne!(state.get_object(token).unwrap().zone, Zone::Battlefield,
        "a token in the graveyard is not a creature card, so it cannot be \
         returned to the battlefield (CR 109.1)");
}

/// Woodland Sleuth returns "a creature **card** at random". Its filter fell
/// back to `is_creature` when `face_data` was None — which is exactly the token
/// case — so the fallback admitted what the filter was written to exclude.
#[test]
fn woodland_sleuth_does_not_return_a_token_from_the_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let token = creature_token_in_graveyard(&mut state, &reg, "Zombie", vec!["Zombie".into()], P0);
    state.creature_died_this_turn = true;
    let sleuth = named_permanent(&mut state, &reg, "Woodland Sleuth", P0);

    reg.get(state.get_object(sleuth).unwrap().card_id).unwrap()
        .on_enter_battlefield(&mut state, sleuth, &[], &reg);

    assert_ne!(state.get_object(token).unwrap().zone, Zone::Hand,
        "a token in the graveyard is not a creature card (CR 109.1)");
}

/// "Target **card** in a graveyard" does not offer a token, and does not
/// resolve against one that slipped in.
///
/// This is the engine's own target enumeration rather than any card's: six
/// `TargetRequirement` variants name a *card* in a graveyard or in exile, and
/// none of them asked. Purify the Grave ("Exile target card from a graveyard")
/// is the one to point at it, but the fix is shared by everything that uses
/// those requirements.
#[test]
fn a_token_in_a_graveyard_is_not_a_targetable_card() {
    use mtg_engine::actions::{Action, Target};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let real = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P1);
    let token = creature_token_in_graveyard(&mut state, &reg, "Zombie", vec!["Zombie".into()], P1);
    let spell = castable_spell(&mut state, &reg, "Purify the Grave", P0);

    let offered: Vec<Target> = mtg_engine::engine::legal_actions(&state, &reg).actions.iter()
        .filter_map(|a| match a {
            Action::CastSpell { object_id, targets, .. } if *object_id == spell => targets.first().cloned(),
            _ => None,
        })
        .collect();

    assert!(offered.contains(&Target::Object(real)),
        "control: the actual card in the graveyard is offered; got {offered:?}");
    assert!(!offered.contains(&Target::Object(token)),
        "a token in a graveyard is not a card (CR 109.1), so it is not a legal \
         target for \"exile target card from a graveyard\"; got {offered:?}");
}
