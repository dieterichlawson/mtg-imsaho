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
use mtg_engine::actions::Target;
use mtg_engine::cards::{CardBehavior, CardData, CardRegistry};
use mtg_engine::ids::CardId;
use mtg_engine::state::GameState;
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
            ..Default::default()
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

/// A red Curse can't be put onto the battlefield attached to a player with
/// protection from red (CR 303.4h), so it stays in the library.
///
/// This used to be tested as a *choice filter*: the player was picked after the
/// search, so the list could be narrowed to players the found Curse could
/// enchant. That is not the order the card plays in. "Put it onto the
/// battlefield attached to **target player**" is targeted, so the player is
/// chosen when the trigger goes on the stack (CR 603.3d) — before anyone knows
/// which Curse the search will find. The legality check is the one the ruling
/// describes, applied on resolution: "The Curse must be legally able to enchant
/// the player. For example, if the player has protection from red, you couldn't
/// put a red Curse onto the battlefield this way."
#[test]
fn bitterheart_witch_cannot_attach_a_red_curse_to_a_player_protected_from_red() {
    let (reg, ward) = registry_with_ward();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    give_ward(&mut state, ward, P1);
    let curse = curse_in_library(&mut state, &reg, "Curse of the Pierced Heart", P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);
    mtg_engine::destruction::try_destroy(&mut state, witch, &reg);

    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, witch, &[Target::Player(P1)], &reg);
    behavior.on_yes_no_choice(&mut state, witch, true, &reg);

    assert_eq!(state.get_object(curse).unwrap().zone, Zone::Library,
        "a red Curse can't enchant a player with protection from red, so it \
         never enters the battlefield (CR 303.4h)");
    assert_eq!(state.get_object(curse).unwrap().attached_to_player, None);
}

/// A black Curse is unaffected by protection from red — the check is per Curse,
/// not per player.
#[test]
fn bitterheart_witch_attaches_a_curse_of_another_color_to_the_same_player() {
    let (reg, ward) = registry_with_ward();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    give_ward(&mut state, ward, P1);
    let curse = curse_in_library(&mut state, &reg, "Curse of Death's Hold", P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);
    mtg_engine::destruction::try_destroy(&mut state, witch, &reg);

    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, witch, &[Target::Player(P1)], &reg);
    behavior.on_yes_no_choice(&mut state, witch, true, &reg);
    // CR 701.19b: the search offers the Curse rather than forcing it, so the
    // find is a separate step even with one Curse in the library.
    behavior.resolve_card_effect(&mut state, witch, "", &Target::Object(curse), &reg);

    assert_eq!(state.get_object(curse).unwrap().zone, Zone::Battlefield,
        "protection from red says nothing about a black Curse");
    assert_eq!(state.get_object(curse).unwrap().attached_to_player, Some(P1));
}

/// CR 303.4h is checked when the Curse would enter, not when the player was
/// targeted: the ward can arrive in between.
#[test]
fn a_curse_does_not_enter_attached_to_a_player_it_cannot_enchant() {
    let (reg, ward) = registry_with_ward();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let curse = curse_in_library(&mut state, &reg, "Curse of the Pierced Heart", P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);
    mtg_engine::destruction::try_destroy(&mut state, witch, &reg);
    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();

    // P1 is targeted while unprotected...
    behavior.on_dies(&mut state, witch, &[Target::Player(P1)], &reg);
    // ...and the ward arrives before the trigger resolves.
    give_ward(&mut state, ward, P1);
    behavior.on_yes_no_choice(&mut state, witch, true, &reg);
    behavior.resolve_card_effect(&mut state, witch, "", &Target::Object(curse), &reg);

    assert_eq!(state.get_object(curse).unwrap().zone, Zone::Library,
        "the Curse can't legally enchant P1 by the time it would enter, so it \
         never enters the battlefield");
    assert_eq!(state.get_object(curse).unwrap().attached_to_player, None);
}
