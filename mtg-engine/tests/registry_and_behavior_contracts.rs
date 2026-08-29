//! The contracts of `CardRegistry` lookup and `CardBehavior`'s optional
//! hooks — pinned directly, because a silently flipped default changes
//! every card that does not override it.
//!
//! The trigger filters (`should_trigger_on_*`) are opt-OUT: permissive by
//! default, so a card that declares a trigger kind and never overrides the
//! filter fires. Everything else here is opt-IN: transformation, loyalty,
//! dynamic token P/T, state triggers, and player protection all mean "no"
//! until a card says otherwise. The full mutation sweep (issues #26–#34)
//! showed nothing pinned either direction.

mod common;
use common::*;
use mtg_engine::cards::{CardBehavior, CardData, CardRegistry};
use mtg_engine::types::*;

/// A card that overrides nothing — the defaults themselves.
struct Vanilla;
impl CardBehavior for Vanilla {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Vanilla".into(),
            card_types: vec![CardType::Creature],
            power: Some(1),
            toughness: Some(1),
            ..Default::default()
        }
    }
}

#[test]
fn optional_hooks_default_permissive_for_filters_and_silent_for_grants() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let a = ready_creature(&mut state, P0, 1, 1);
    let b = ready_creature(&mut state, P1, 1, 1);
    let v = Vanilla;

    // Opt-out trigger filters: default is "fire".
    assert!(v.should_trigger_on_spell_cast(&state, a, P0, b, &reg));
    assert!(v.should_trigger_on_damage_to_player(&state, a, b, P1, &reg));
    assert!(v.should_trigger_on_creature_enters(&state, a, b, P1, &reg));

    // Opt-in capabilities: default is "none".
    assert!(!v.should_transform(&state, a, &reg));
    assert_eq!(v.starting_loyalty(), None);
    assert_eq!(v.token_dynamic_pt(&state, a, b, &reg), None);
    assert_eq!(v.state_trigger_description(), "");
    assert!(v.grants_player_protection_from().is_empty());
    assert!(!v.state_trigger_condition(&state, a, &reg));
}

/// The registry resolves a full double-faced name ("Front // Back") to the
/// front face, and only a name that actually carries a back half takes that
/// fallback.
#[test]
fn a_full_double_faced_name_resolves_to_its_front_face() {
    let reg = CardRegistry::with_all_cards();
    let full = reg.get_id_by_name("Mayor of Avabruck // Howlpack Alpha");
    assert!(full.is_some(), "the DFC fallback strips the back face");
    assert_eq!(full, reg.get_id_by_name("Mayor of Avabruck"));
    assert_eq!(reg.get_id_by_name("No Such Card"), None);
}
