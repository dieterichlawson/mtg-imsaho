//! Protection from a color, on a player.
//!
//! Hexproof and protection are different restrictions and the engine only had
//! the first. Hexproof stops a player being targeted; protection additionally
//! stops them being *enchanted* by an Aura of that color (CR 702.16b). So a
//! player can be a legal target for "attached to target player" and still not
//! be somewhere the Curse can go — and CR 303.4h says an Aura that would
//! enter attached to something it can't legally enchant doesn't enter at all.
//!
//! Innistrad has no card that grants a player protection, so the test
//! registers one alongside the real set. `CardBehavior` is the extension
//! point; the rule lives in `GameState`, not in the card.

mod common;

use common::*;
use mtg_engine::actions::{ResolvedChoice, Target};
use mtg_engine::cards::{CardBehavior, CardData, CardRegistry};
use mtg_engine::ids::CardId;
use mtg_engine::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use mtg_engine::types::*;

/// "You have protection from red." Stands in for a Leyline of Sanctity-style
/// effect; the set has none.
struct WardOfRed;

impl CardBehavior for WardOfRed {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ward of Red".into(),
            cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2)])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "You have protection from red.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn grants_player_protection_from(&self) -> Vec<Color> {
        vec![Color::Red]
    }
}

/// The full set plus the ward. Named apart from `common::registry` rather than
/// shadowing it, so a reader can see at a glance which one a test is using.
fn registry_with_ward() -> (CardRegistry, CardId) {
    let mut reg = CardRegistry::with_all_cards();
    let ward = reg.register(Box::new(WardOfRed));
    (reg, ward)
}

fn give_ward(state: &mut GameState, ward: CardId, player: PlayerId) -> ObjectId {
    let id = state.create_object(ward, player, Zone::Battlefield, None, None);
    state.get_object_mut(id).unwrap().name = "Ward of Red".into();
    id
}

/// Put a named Curse card into a player's library.
fn curse_in_library(state: &mut GameState, reg: &CardRegistry, name: &str, player: PlayerId) -> ObjectId {
    let id = state.create_object(reg.get_id_by_name(name).unwrap(), player, Zone::Library, None, None);
    state.get_object_mut(id).unwrap().name = name.into();
    state.get_player_mut(player).library_order.push(id);
    id
}

#[test]
fn a_player_with_protection_cannot_be_enchanted_by_that_color() {
    let (reg, ward) = registry_with_ward();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    give_ward(&mut state, ward, P1);

    let red_curse = curse_in_library(&mut state, &reg, "Curse of the Pierced Heart", P0);
    let black_curse = curse_in_library(&mut state, &reg, "Curse of Death's Hold", P0);

    assert!(state.player_has_protection_from(P1, Color::Red, &reg));
    assert!(!state.player_has_protection_from(P1, Color::Black, &reg));
    assert!(!state.player_has_protection_from(P0, Color::Red, &reg),
        "the ward protects only its controller");

    assert!(!state.player_can_be_enchanted_by(red_curse, P1, &reg),
        "a red Curse can't enchant a player with protection from red (CR 702.16b)");
    assert!(state.player_can_be_enchanted_by(black_curse, P1, &reg),
        "a black Curse is unaffected");
    assert!(state.player_can_be_enchanted_by(red_curse, P0, &reg),
        "and the unprotected player is fine either way");
}

/// Bitterheart Witch offers only players the chosen Curse could legally
/// enchant.
#[test]
fn bitterheart_witch_does_not_offer_a_protected_player() {
    let (reg, ward) = registry_with_ward();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    give_ward(&mut state, ward, P1);
    curse_in_library(&mut state, &reg, "Curse of the Pierced Heart", P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);
    mtg_engine::destruction::try_destroy(&mut state, witch, &reg);

    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, witch, &[], &reg);
    // "You may search" — yes.
    behavior.on_yes_no_choice(&mut state, witch, true, &reg);

    let options = match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTarget { options, .. }, .. }) => options.clone(),
        other => panic!("expected a player choice, got {other:?}"),
    };
    assert!(options.contains(&Target::Player(P0)),
        "the Witch's controller is still a legal choice; got {options:?}");
    assert!(!options.contains(&Target::Player(P1)),
        "a player with protection from red can't be enchanted by a red Curse, \
         so they are not a legal choice; got {options:?}");
}

/// A black Curse is unaffected by protection from red — the filter is per
/// Curse, not per player.
#[test]
fn bitterheart_witch_still_offers_the_player_for_a_curse_of_another_color() {
    let (reg, ward) = registry_with_ward();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    give_ward(&mut state, ward, P1);
    curse_in_library(&mut state, &reg, "Curse of Death's Hold", P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);
    mtg_engine::destruction::try_destroy(&mut state, witch, &reg);

    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, witch, &[], &reg);
    behavior.on_yes_no_choice(&mut state, witch, true, &reg);

    let options = match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTarget { options, .. }, .. }) => options.clone(),
        other => panic!("expected a player choice, got {other:?}"),
    };
    assert!(options.contains(&Target::Player(P1)),
        "protection from red says nothing about a black Curse; got {options:?}");
}

/// CR 303.4h: even if the choice were somehow made, the Curse does not enter
/// the battlefield attached to a player it can't enchant.
#[test]
fn a_curse_does_not_enter_attached_to_a_player_it_cannot_enchant() {
    let (reg, ward) = registry_with_ward();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let ward_id = give_ward(&mut state, ward, P1);
    let curse = curse_in_library(&mut state, &reg, "Curse of the Pierced Heart", P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);
    mtg_engine::destruction::try_destroy(&mut state, witch, &reg);
    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();

    // Choose the Curse, then answer the player choice with the protected
    // player — the state the re-check exists for (the ward could have arrived
    // after the choice was offered).
    state.move_object(ward_id, Zone::Graveyard, &reg);
    behavior.on_dies(&mut state, witch, &[], &reg);
    behavior.on_yes_no_choice(&mut state, witch, true, &reg);
    let options = match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTarget { options, .. }, .. }) => options.clone(),
        other => panic!("expected a player choice, got {other:?}"),
    };
    assert!(options.contains(&Target::Player(P1)),
        "with the ward gone P1 is offered; got {options:?}");

    // The ward comes back before the choice is answered.
    state.move_object(ward_id, Zone::Battlefield, &reg);
    state = mtg_engine::engine::submit_action(&state, &mtg_engine::actions::Action::ResolveChoice {
        choice: ResolvedChoice::ChosenTarget(Some(Target::Player(P1))),
    }, &reg);

    assert_eq!(state.get_object(curse).unwrap().zone, Zone::Library,
        "the Curse can't legally enchant P1, so it never enters the battlefield");
    assert_eq!(state.get_object(curse).unwrap().attached_to_player, None);
}
