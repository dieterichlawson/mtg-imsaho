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

// ── The printed-face cache ──────────────────────────────────────────

/// `CardBehavior::card_data` CONSTRUCTS a whole `CardData` — the name, the
/// oracle text, the keyword list, the trigger list, each a fresh allocation
/// — and returns it by value. `GameState::walk_effects` used to call it
/// once per object in the game per lookup, and `has_keyword` is one such
/// lookup, so one view of a 1,000-permanent board built about sixteen
/// million of them (#565).
///
/// The registry now reads each face's printed effects and keywords once, at
/// registration. That is only sound while the cache says what the card
/// says, so this sweeps the whole pool and asks both.
#[test]
fn the_registrys_printed_cache_says_what_each_card_says() {
    let registry = CardRegistry::with_all_cards();

    let mut checked = 0;
    let mut with_effects = 0;
    let mut with_backs = 0;
    for name in registry.all_names() {
        let id = registry.get_id_by_name(name).expect("a registered name");
        let behavior = registry.get(id).expect("a registered card");
        let front = behavior.card_data();
        checked += 1;
        if !front.continuous_effects.is_empty() {
            with_effects += 1;
        }

        assert_eq!(registry.printed_continuous_effects(id, false), front.continuous_effects,
            "{name}: front-face effects");
        assert_eq!(registry.printed_keywords(id, false), Some(front.keywords.as_slice()),
            "{name}: front-face keywords");

        match behavior.back_face_data() {
            Some(back) => {
                with_backs += 1;
                assert_eq!(registry.printed_continuous_effects(id, true), back.continuous_effects,
                    "{name}: back-face effects");
                assert_eq!(registry.printed_keywords(id, true), Some(back.keywords.as_slice()),
                    "{name}: back-face keywords");
            }
            None => {
                // A card with no back face falls back to its front for
                // effects, and reports NOTHING for keywords — which is what
                // `has_keyword` has always done, to keep a stale front-face
                // keyword off a transformed DFC.
                assert_eq!(registry.printed_continuous_effects(id, true), front.continuous_effects,
                    "{name}: no back face, so the front's effects stand");
                assert_eq!(registry.printed_keywords(id, true), Some(&[][..]),
                    "{name}: no back face, so no printed keywords");
            }
        }
    }

    // The sweep has to have swept something of each kind, or it agrees
    // about an empty pool.
    assert!(checked > 100, "only {checked} cards registered");
    assert!(with_effects > 5, "only {with_effects} cards print a continuous effect");
    assert!(with_backs > 5, "only {with_backs} cards have a back face");
}

/// A card the registry has never heard of — a token with no face — has no
/// printed effects and no printed keyword list at all, which is what sends
/// `has_keyword` to the object's own vector.
#[test]
fn an_unregistered_card_prints_nothing_and_says_so() {
    let registry = CardRegistry::with_all_cards();
    let unknown = mtg_engine::ids::CardId(9999);

    assert!(registry.printed_continuous_effects(unknown, false).is_empty());
    assert!(registry.printed_continuous_effects(unknown, true).is_empty());
    // `None`, not `Some(&[])`: "this card prints no keywords" and "there is
    // no such card" send `has_keyword` to different places.
    assert_eq!(registry.printed_keywords(unknown, false), None);
    assert_eq!(registry.printed_keywords(unknown, true), None);
}

/// Reading the battlefield is linear in the battlefield.
///
/// The cost that made #565 was shaped, not constant: `walk_effects` asked
/// every object in the game for its continuous effects, `has_keyword` is
/// one such walk, and a view asked for fifteen keywords per permanent — so
/// building one view was `O(15 n²)` calls to `CardBehavior::card_data`,
/// each of which CONSTRUCTS a `CardData`. A 1,000-permanent board took
/// three seconds in release and a 12,928-permanent one never answered.
///
/// A wall-clock assertion would be flaky and would say nothing about why.
/// This counts the constructions instead: the card under test tallies its
/// own `card_data` calls, so the quadratic term is visible as a number.
/// Against master, n=40 gives tens of thousands.
#[test]
fn building_a_view_does_not_ask_each_card_about_every_other_one() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CALLS: AtomicUsize = AtomicUsize::new(0);

    struct Counted;
    impl CardBehavior for Counted {
        fn card_data(&self) -> CardData {
            CALLS.fetch_add(1, Ordering::Relaxed);
            CardData {
                name: "Counted".into(),
                card_types: vec![CardType::Creature],
                power: Some(1),
                toughness: Some(1),
                keywords: vec![Keyword::Flying],
                ..Default::default()
            }
        }
    }

    let mut registry = CardRegistry::new();
    let counted = registry.register(Box::new(Counted));

    const N: usize = 40;
    let mut state = game_at_step(Step::PrecombatMain, P0);
    for i in 0..N {
        let owner = if i % 2 == 0 { P0 } else { P1 };
        let id = state.create_object(counted, owner, Zone::Battlefield, Some(1), Some(1));
        state.get_object_mut(id).unwrap().summoning_sick = false;
    }

    CALLS.store(0, Ordering::Relaxed);
    let view = mtg_engine::view::GameView::for_player(&state, P0, &registry);
    let calls = CALLS.load(Ordering::Relaxed);

    assert_eq!(view.battlefield.len(), N, "the board is the one that was built");
    assert!(
        view.battlefield.iter().all(|p| p.keywords.contains(&Keyword::Flying)),
        "and the view still reads the keyword off the card"
    );
    assert!(
        calls < 20 * N,
        "building a view of {N} permanents constructed {calls} CardDatas — that is per-pair, \
         not per-permanent, and it is what made a 1,000-permanent board take three seconds \
         and a 12,928-permanent one never answer (#565)"
    );
}
