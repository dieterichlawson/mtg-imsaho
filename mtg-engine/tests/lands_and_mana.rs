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
fn ghost_quarter_search(reg: &mtg_engine::cards::CardRegistry, seed: u64) -> Vec<usize> {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.rng_state = seed;

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

    // The two halves of CR 602.2a, in order: the sacrifice is a cost, paid on
    // activation, and `on_activate_ability` only puts the ability on the stack.
    // The effect belongs to `resolve_top_of_stack`, so calling the activation
    // hook alone destroys nothing.
    state.move_object(gq, Zone::Graveyard, reg);
    activate_via_hooks(&mut state, reg, gq, 1, &[Target::Object(victim)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, reg);

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

/// Ruling 2013-07-01: "The target land's controller gets to search for a basic
/// land card **even if that land wasn't destroyed** by Ghost Quarter's ability.
/// This may happen because the land has indestructible or because it was
/// regenerated."
///
/// Two sentences in one ability, and only the first one is conditional on
/// anything.
#[test]
fn ghost_quarter_offers_the_search_even_when_the_land_survives() {
    let reg = registry();
    for (label, make_survive) in [
        ("indestructible", 0u32),
        ("regeneration", 1u32),
    ] {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let gq = named_permanent(&mut state, &reg, "Ghost Quarter", P0);
        let victim = named_permanent(&mut state, &reg, "Forest", P1);
        if make_survive == 0 {
            grant_keyword(&mut state, victim, Keyword::Indestructible);
        } else {
            state.add_regeneration_shield(victim);
        }
        let basic = state.create_object(
            reg.get_id_by_name("Plains").unwrap(), P1, Zone::Library, None, None);
        state.get_player_mut(P1).library_order.push(basic);

        // The sacrifice is a cost, paid on activation.
        state.move_object(gq, Zone::Graveyard, &reg);
        activate_via_hooks(&mut state, &reg, gq, 1, &[Target::Object(victim)]);
        mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

        assert_eq!(state.get_object(victim).unwrap().zone, Zone::Battlefield,
            "{label}: the land survived, which is the point of the ruling");
        assert!(state.awaiting_action.is_some(),
            "{label}: its controller is still offered the search; got {:?}",
            state.awaiting_action);

        let options = pending_choice_options(&state);
        assert!(options.contains(&Target::Object(basic)),
            "{label}: the basic land is among the options: {options:?}");

        // And the log says what actually happened rather than announcing a
        // destruction that did not occur.
        let claimed_destroyed = state.game_log.iter()
            .any(|e| e.message == "Ghost Quarter destroyed Forest");
        assert!(!claimed_destroyed,
            "{label}: the log must not claim the land was destroyed; log was {:?}",
            state.game_log.iter().map(|e| &e.message).collect::<Vec<_>>());
        assert!(state.game_log.iter().any(|e| e.message.contains("Ghost Quarter could not destroy")),
            "{label}: and it should say why not");
    }
}

/// Ruling 2006-05-01: "If you target Ghost Quarter with its own ability, the
/// ability won't resolve because its target is no longer on the battlefield.
/// You won't get to search for a land card." The sacrifice is a cost, so the
/// Quarter is already gone when the ability would resolve (CR 608.2b).
#[test]
fn ghost_quarter_targeting_itself_does_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gq = named_permanent(&mut state, &reg, "Ghost Quarter", P0);
    let basic = state.create_object(
        reg.get_id_by_name("Plains").unwrap(), P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(basic);

    state.move_object(gq, Zone::Graveyard, &reg);
    activate_via_hooks(&mut state, &reg, gq, 1, &[Target::Object(gq)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(state.awaiting_action.is_none(),
        "no search: the only target was the Quarter itself, already sacrificed");
    assert_eq!(state.get_object(basic).unwrap().zone, Zone::Library,
        "and the basic land stays where it is");
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
    for seed in 0..20u64 {
        let order = ghost_quarter_search(&reg, seed);
        if !orders.contains(&order) {
            orders.push(order);
        }
    }
    assert!(orders.len() > 1,
        "twenty seeds all left the library in the same order, so it is not \
         being shuffled: {:?}", orders.first());
}

// ── The auto-tap planner's contract ─────────────────────────────────────
//
// The full mutation sweep (issues #26–#34) left arithmetic inside
// `compute_autotap`/`try_auto_pay` unpinned. The planner's contract, not
// its heuristics: a plan pays the cost with the right colors, taps no
// more sources than the cost has pips, spends floating mana before
// tapping, and an unpayable cost is simply not offered.

/// {G} with three Forests taps exactly one — a plan never burns extra
/// sources.
#[test]
fn the_autotap_plan_taps_no_more_than_the_cost_needs() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    for _ in 0..3 {
        named_permanent(&mut state, &reg, "Forest", P0);
    }
    spell_in_hand(&mut state, &reg, "Avacyn's Pilgrim", P0);

    let legal = engine::legal_actions(&state, &reg);
    let cs = legal.castable_spells.iter()
        .find(|c| c.name == "Avacyn's Pilgrim")
        .expect("a {G} creature with three Forests up is castable");
    assert_eq!(cs.tap_plan.len(), 1, "one pip, one tap: {:?}", cs.tap_plan);
}

/// {2}{U} with exactly Island+Swamp+Plains routes the Island to the {U}
/// pip and fills generic from the rest.
#[test]
fn the_autotap_plan_routes_colored_pips_to_matching_sources() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let island = named_permanent(&mut state, &reg, "Island", P0);
    named_permanent(&mut state, &reg, "Swamp", P0);
    named_permanent(&mut state, &reg, "Plains", P0);
    spell_in_hand(&mut state, &reg, "Forbidden Alchemy", P0);

    let legal = engine::legal_actions(&state, &reg);
    let cs = legal.castable_spells.iter()
        .find(|c| c.name == "Forbidden Alchemy")
        .expect("{2}{U} is payable from Island+Swamp+Plains");
    assert_eq!(cs.tap_plan.len(), 3, "three pips, three taps");
    assert!(cs.tap_plan.iter().any(|(id, _)| *id == island),
        "the only blue source must be in the plan");
}

/// Issue #84: a generic pip is paid from a REDUNDANT source — one whose
/// colors the other untapped sources still produce — so no color access
/// is lost. Nephalia Drownyard's {1}{U}{B} with 2 Islands + 3 Swamps up
/// used to pay the {1} with the second Island, leaving {B}{B} where a
/// Swamp would have left {U}{B} and silently removing the {1}{U} spell
/// still in hand from the menu.
#[test]
fn the_autotap_plan_spends_redundant_colors_on_generic() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let islands = [
        named_permanent(&mut state, &reg, "Island", P0),
        named_permanent(&mut state, &reg, "Island", P0),
    ];
    for _ in 0..3 {
        named_permanent(&mut state, &reg, "Swamp", P0);
    }
    let yard = named_permanent(&mut state, &reg, "Nephalia Drownyard", P0);

    let legal = engine::legal_actions(&state, &reg);
    let plan = legal.actions.iter()
        .find_map(|a| match a {
            Action::ActivateAbility { object_id, tap_plan, .. }
                if *object_id == yard && !tap_plan.is_empty() => Some(tap_plan.clone()),
            _ => None,
        })
        .expect("the mill ability is affordable and on offer");
    let islands_tapped = plan.iter().filter(|(id, _)| islands.contains(id)).count();
    assert_eq!(islands_tapped, 1,
        "one Island for the {{U}} pip; the generic {{1}} comes from a \
         redundant Swamp, not the last blue source: {plan:?}");
}

/// A cost the battlefield cannot pay is not offered at all.
#[test]
fn an_unpayable_cost_is_not_offered() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Plains", P0);
    spell_in_hand(&mut state, &reg, "Midnight Haunting", P0); // {1}{W}

    let legal = engine::legal_actions(&state, &reg);
    assert!(!legal.castable_spells.iter().any(|c| c.name == "Midnight Haunting"),
        "{{1}}{{W}} with a single Plains is short one mana");
}

/// Floating mana is spent before anything is tapped.
#[test]
fn floating_mana_is_spent_before_tapping() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Forest", P0);
    add_mana(&mut state, P0, &[(ManaType::Green, 1)]);
    spell_in_hand(&mut state, &reg, "Avacyn's Pilgrim", P0);

    let legal = engine::legal_actions(&state, &reg);
    let cs = legal.castable_spells.iter()
        .find(|c| c.name == "Avacyn's Pilgrim")
        .expect("castable from the floating {G} alone");
    assert!(cs.tap_plan.is_empty(),
        "the floating {{G}} pays the whole cost; the Forest stays untapped: {:?}",
        cs.tap_plan);
}

/// One floating {W} is one {W}: it cannot pay both pips of {1}{W}{W}.
/// (Pins the pool-deduction bookkeeping inside the planner — an inflated
/// simulated pool would offer the cast anyway.)
#[test]
fn floating_mana_is_not_double_counted_across_pips() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    add_mana(&mut state, P0, &[(ManaType::White, 1)]);
    spell_in_hand(&mut state, &reg, "Chapel Geist", P0); // {1}{W}{W}

    let legal = engine::legal_actions(&state, &reg);
    assert!(!legal.castable_spells.iter().any(|c| c.name == "Chapel Geist"),
        "one floating {{W}} and no lands cannot pay {{1}}{{W}}{{W}}");
}
