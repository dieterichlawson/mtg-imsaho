//! Tests for land plays and mana payment rules.

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::engine;
use mtg_engine::mana;
use mtg_engine::types::*;

/// Rule 305.1: You can play a land during your second main phase.
#[test]
fn can_play_land_in_postcombat_main() {
    let registry = registry();
    let mut state = game_at_step(Step::PostcombatMain, P0);

    spell_in_hand(&mut state, &registry, "Forest", P0);

    let actions = engine::legal_actions(&state, &registry);
    assert!(actions.actions.iter().any(|a| matches!(a, Action::PlayLand { .. })),
        "Should be able to play land in postcombat main (rule 305.1)");
}

/// One land per turn: after playing one, you can't play another.
#[test]
fn only_one_land_per_turn() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let land1 = spell_in_hand(&mut state, &registry, "Forest", P0);
    spell_in_hand(&mut state, &registry, "Forest", P0);

    state = engine::submit_action(&state, &Action::PlayLand { object_id: land1 }, &registry);

    assert_eq!(state.get_player(P0).land_plays_remaining, 0);
    let actions = engine::legal_actions(&state, &registry);
    assert!(!actions.actions.iter().any(|a| matches!(a, Action::PlayLand { .. })),
        "Should not be able to play a second land");
}

/// Land plays reset at the start of your turn (during untap).
#[test]
fn land_plays_reset_at_untap() {
    let registry = registry();
    let mut state = game_at_step(Step::Cleanup, P0);
    state.priority_player = None;
    state.get_player_mut(P0).land_plays_remaining = 0;

    // Round the table back to P0's own untap step.
    advance_to_next_turn(&mut state, &registry);
    advance_to_next_turn(&mut state, &registry);
    assert_eq!((state.active_player, state.step), (P0, Step::Untap), "test setup");

    assert_eq!(state.get_player(P0).land_plays_remaining, 1);
}

/// Rule 116.2a: Playing a land doesn't use the stack.
#[test]
fn playing_land_doesnt_use_stack() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let land = spell_in_hand(&mut state, &registry, "Forest", P0);

    state = engine::submit_action(&state, &Action::PlayLand { object_id: land }, &registry);

    assert!(state.stack.is_empty());
    assert_eq!(state.get_object(land).unwrap().zone, Zone::Battlefield);
}

/// A just-played land can be tapped for mana immediately.
#[test]
fn can_tap_just_played_land() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let land = spell_in_hand(&mut state, &registry, "Forest", P0);

    state = engine::submit_action(&state, &Action::PlayLand { object_id: land }, &registry);

    let actions = engine::legal_actions(&state, &registry);
    let has_mana_ability = actions.actions.iter().any(|a| match a {
        Action::ActivateManaAbility { object_id, .. } => *object_id == land,
        _ => false,
    });
    assert!(has_mana_ability, "Should be able to tap a just-played land for mana");
}

/// Can't play a land during opponent's turn.
#[test]
fn cannot_play_land_during_opponent_turn() {
    let registry = registry();
    let mut state = game_at_step(Step::PrecombatMain, P1);
    state.priority_player = Some(P0); // P0 has priority but it's P1's turn

    spell_in_hand(&mut state, &registry, "Forest", P0);

    let actions = engine::legal_actions(&state, &registry);
    assert!(!actions.actions.iter().any(|a| matches!(a, Action::PlayLand { .. })),
        "Should not be able to play land during opponent's turn");
}

/// Can't play a land during combat.
#[test]
fn cannot_play_land_during_combat() {
    let registry = registry();
    let mut state = game_at_step(Step::BeginCombat, P0);

    spell_in_hand(&mut state, &registry, "Forest", P0);

    let actions = engine::legal_actions(&state, &registry);
    assert!(!actions.actions.iter().any(|a| matches!(a, Action::PlayLand { .. })),
        "Should not be able to play land during combat");
}

/// Generic mana can be paid with any color.
#[test]
fn generic_mana_payable_with_any_color() {
    let mut pool = ManaPool::new();
    pool.add(ManaType::Blue, 3);

    // {2}{R} — can't pay the {R}
    let cost_needs_red = ManaCost::new(vec![
        ManaSymbol::Generic(2),
        ManaSymbol::Colored(Color::Red),
    ]);
    assert!(!mana::can_pay(&pool, &cost_needs_red));

    // {3} — all generic, payable with blue
    let cost_all_generic = ManaCost::new(vec![ManaSymbol::Generic(3)]);
    assert!(mana::can_pay(&pool, &cost_all_generic));
}

/// Mana payment deducts correctly and leaves leftover.
#[test]
fn mana_payment_leaves_correct_remainder() {
    let mut pool = ManaPool::new();
    pool.add(ManaType::Green, 3);
    pool.add(ManaType::Red, 1);

    let cost = ManaCost::new(vec![
        ManaSymbol::Generic(1),
        ManaSymbol::Colored(Color::Green),
    ]);

    mana::auto_pay(&mut pool, &cost).unwrap();

    assert_eq!(pool.total(), 2);
    // Auto-pay uses red for generic (it comes before green in the pay order),
    // so we should have 2 green left and 0 red.
    assert_eq!(pool.get(ManaType::Green), 2);
    assert_eq!(pool.get(ManaType::Red), 0);
}

/// Free cost (no mana required) can always be paid.
#[test]
fn free_cost_always_payable() {
    let pool = ManaPool::new();
    let cost = ManaCost::free();
    assert!(mana::can_pay(&pool, &cost));
}

/// Can't pay cost with empty pool.
#[test]
fn empty_pool_cannot_pay() {
    let pool = ManaPool::new();
    let cost = ManaCost::new(vec![ManaSymbol::Generic(1)]);
    assert!(!mana::can_pay(&pool, &cost));
}

// ---------------------------------------------------------------------------
// Ghost Quarter: "{T}, Sacrifice Ghost Quarter: Destroy target land. Its
// controller may search their library for a basic land card, put it onto the
// battlefield, then shuffle."
// ---------------------------------------------------------------------------

/// Search the library and take the first basic offered, reporting the order the
/// remaining cards were left in.
///
/// Keyed on object ids rather than names: the ten cards are two each of five
/// basics, so a name-keyed order cannot tell several genuinely different
/// shuffles apart.
fn ghost_quarter_search(reg: &mtg_engine::cards::CardRegistry) -> Vec<usize> {
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gq = named_permanent(&mut state, reg, "Ghost Quarter", P0);
    let victim = named_permanent(&mut state, reg, "Forest", P1);
    let library: Vec<ObjectId> = ["Plains", "Island", "Swamp", "Mountain", "Forest",
                                  "Plains", "Island", "Swamp", "Mountain", "Forest"]
        .iter()
        .map(|name| {
            let id = state.create_object(reg.get_id_by_name(name).unwrap(), P1, Zone::Library, None, None);
            state.get_player_mut(P1).library_order.push(id);
            id
        })
        .collect();

    let behavior = reg.get(state.get_object(gq).unwrap().card_id).unwrap();
    state.move_object(gq, Zone::Graveyard, reg);
    behavior.on_activate_ability(&mut state, gq, 1, &[Target::Object(victim)], reg);

    let options = pending_choice_options(&state);
    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(options.first().cloned()),
        },
        reg,
    );

    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Graveyard, "the land is destroyed");
    // Positions within the library as it was built, so orders from separate
    // runs are comparable.
    let left: Vec<usize> = state.get_player(P1).library_order.iter()
        .map(|id| library.iter().position(|l| l == id).expect("a card that was stocked"))
        .collect();
    assert_eq!(left.len(), 9, "exactly one basic was found and put onto the battlefield");
    left
}

/// "…then shuffle." Checked across repeated searches rather than against one
/// forbidden order: a shuffle of these nine cards can legitimately land back on
/// any particular arrangement, and `assert_ne!` against one of them is
/// satisfied by almost any bug as well — an emptied library passes it.
///
/// Without a shuffle every run leaves the same order; with one, twenty runs
/// landing on a single order is not something that happens.
#[test]
fn ghost_quarter_shuffles_the_library_after_the_search() {
    let reg = registry();
    let mut orders: Vec<Vec<usize>> = Vec::new();
    for _ in 0..20 {
        let order = ghost_quarter_search(&reg);
        if !orders.contains(&order) {
            orders.push(order);
        }
    }
    assert!(orders.len() > 1,
        "twenty searches all left the library in the same order, so it is not \
         being shuffled: {:?}", orders.first());
}
