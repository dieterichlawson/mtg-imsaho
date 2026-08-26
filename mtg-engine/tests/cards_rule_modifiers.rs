//! Cards that change the rules of the game rather than affecting objects:
//! alternative and reduced costs, a replacement for losing the game, granted
//! flashback, doubled token creation, a banned card name.
//!
//! Cards covered (8), so this is greppable by name as well as by rule:
//!
//! - Devil's Play
//! - Heartless Summoning
//! - Kessig Wolf Run
//! - Laboratory Maniac
//! - Nevermore
//! - Parallel Lives
//! - Rooftop Storm
//! - Snapcaster Mage
//!
//! Past in Flames' flashback grant is tested in `flashback.rs`, next to the
//! mechanic it grants; Olivia Voldaren has her own file.

mod common;

use common::*;
use mtg_engine::engine;
use mtg_engine::actions::{Action, Target};
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;
// ── Laboratory Maniac ──────────────────────────────────────────

/// "If you would draw a card while your library has no cards in it, you win the
/// game instead." A replacement effect (CR 614), so it applies at the draw
/// rather than at the state-based check the draw would otherwise fail: with a
/// Maniac out, the game is already over when `draw_cards` returns.
#[test]
fn laboratory_maniac_replaces_the_empty_draw_loss_for_its_controller() {
    // (who controls a Lab Maniac, who draws from empty, does the drawer lose,
    //  does the drawer's opponent lose)
    const CASES: &[(Option<PlayerId>, PlayerId, bool, bool, &str)] = &[
        (None, P0, true, false, "no Lab Maniac: the drawer loses"),
        (Some(P0), P0, false, true, "its controller wins instead, so the opponent loses"),
        (Some(P0), P1, true, false, "it does nothing for the opponent"),
    ];

    for &(maniac_controller, drawer, drawer_loses, opponent_loses, why) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);
        if let Some(p) = maniac_controller {
            named_creature(&mut state, &reg, "Laboratory Maniac", p);
        }
        let opponent = state.opponent(drawer);

        state.get_player_mut(drawer).library_order.clear();
        let _ = engine::draw_cards(&mut state, drawer, 1, &reg);
        check_state_based_actions(&mut state, &reg);

        assert_eq!(state.get_player(drawer).lost, drawer_loses, "{why}");
        assert_eq!(state.get_player(opponent).lost, opponent_loses,
            "{why} (checking the other player too, so a row cannot pass by \
             everyone losing)");
        assert!(state.result.is_some(), "{why}: the game is over either way");
    }
}

// ── Parallel Lives ──────────────────────────────────────────

/// "If one or more tokens would be created under your control, twice that many
/// of those tokens are created instead" (CR 614.1c) — a replacement effect, so
/// it applies to whoever would create them, not to whoever asks for them.
#[test]
fn parallel_lives_doubles_only_its_controllers_tokens() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let make = |state: &mut mtg_engine::state::GameState, name: &str, p: PlayerId| {
        state.create_token(name, p, 1, 1, vec![Color::White], vec![CardType::Creature], vec![], &reg);
    };

    // Baseline: one token is one token.
    make(&mut state, "Spirit", P0);
    assert_eq!(count_tokens_named_by(&state, "Spirit", P0), 1,
        "without Parallel Lives, creating one token creates one token");

    named_creature(&mut state, &reg, "Parallel Lives", P0);

    make(&mut state, "Angel", P0);
    assert_eq!(count_tokens_named_by(&state, "Angel", P0), 2,
        "its controller's tokens are doubled");

    make(&mut state, "Zombie", P1);
    assert_eq!(count_tokens_named_by(&state, "Zombie", P1), 1,
        "an opponent's are not");
}

// ── Heartless Summoning ──────────────────────────────────────

/// Heartless Summoning's other half: "Creature spells you cast cost {2} less to
/// cast. Creatures you control get -1/-1."
#[test]
fn heartless_summoning_shrinks_the_creatures_it_cheapens() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    named_creature(&mut state, &reg, "Heartless Summoning", P0);
    let creature = named_creature(&mut state, &reg, "Kindercatch", P0); // printed 6/6

    assert_eq!(state.effective_power(creature, &reg).unwrap(), 5);
    assert_eq!(state.effective_toughness(creature, &reg).unwrap(), 5);
}

/// Static cost modifiers, and the spells they do and do not reach. A cost
/// reduction is only visible as "with exactly this much mana, may I cast it?",
/// so each row states the mana and the answer.
///
/// Each modifier needs both rows: with only the positive one, a card that
/// cheapened *every* spell would pass.
#[test]
fn a_cost_modifier_reaches_the_spells_its_text_names() {
    // (modifier on the battlefield, spell in hand, mana in pool, castable?, why)
    const CASES: &[(&str, &str, &[(ManaType, u32)], bool, &str)] = &[
        ("Heartless Summoning", "Kindercatch", &[(ManaType::Colorless, 1), (ManaType::Green, 3)], true,
         "{3}{G}{G}{G} less {2} is {1}{G}{G}{G}"),
        ("Heartless Summoning", "Lightning Bolt", &[], false,
         "the reduction is for creature spells, and this is an instant"),
        ("Rooftop Storm", "Walking Corpse", &[], true,
         "'you may pay {0} rather than pay the mana cost for Zombie creature spells'"),
        ("Rooftop Storm", "Grizzly Bears", &[], false,
         "and the Bears are not a Zombie"),
    ];

    for &(modifier, spell_name, mana, castable, why) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        named_creature(&mut state, &reg, modifier, P0);
        let spell = spell_in_hand(&mut state, &reg, spell_name, P0);
        for &(kind, n) in mana {
            state.get_player_mut(P0).mana_pool.add(kind, n);
        }

        assert_eq!(can_cast(&state, &reg, spell), castable,
            "{modifier} + {spell_name}: {why}");
    }
}

// ── Nevermore ──────────────────────────────────────────

/// "As Nevermore enters the battlefield, choose a nonland card name. Spells with
/// the chosen name can't be cast." Named, not "a card like it" — so the ban has
/// to be checked against the name and nothing else.
#[test]
fn nevermore_bans_the_name_it_chose_and_nothing_else() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P1);

    let nevermore = named_creature(&mut state, &reg, "Nevermore", P0);
    state.get_object_mut(nevermore).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::PreventCastingNamed { name: "Lightning Bolt".into() },
    ]);

    // Both spells in the opponent's hand, both fully paid for.
    state.priority_player = Some(P1);
    let bolt = spell_in_hand(&mut state, &reg, "Lightning Bolt", P1);
    let growth = spell_in_hand(&mut state, &reg, "Giant Growth", P1);
    state.get_player_mut(P1).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P1).mana_pool.add(ManaType::Green, 1);
    ready_creature(&mut state, P1, 2, 2); // a target for Giant Growth

    assert!(!can_cast(&state, &reg, bolt),
        "the named spell can't be cast, by either player");
    assert!(can_cast(&state, &reg, growth),
        "and everything else still can — a ban on all spells would pass the \
         first assertion on its own");
}

// ── Devil's Play ──────────────────────────────────────────

/// "Devil's Play deals X damage to any target." X comes from what is left in
/// the pool after the {R}, including nothing at all (CR 107.3).
#[test]
fn devils_play_deals_as_much_damage_as_x_was_paid_for() {
    // (extra colorless mana beyond the {R}, resulting damage)
    for (extra, damage) in [(3, 3), (0, 0)] {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let spell = spell_in_hand(&mut state, &reg, "Devil's Play", P0);
        state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
        if extra > 0 {
            state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, extra);
        }

        let state = cast_and_resolve(&state, &reg, spell, vec![Target::Player(P1)]);
        assert_eq!(state.get_player(P1).life, 20 - damage,
            "{{R}} plus {extra} generic funds X={damage}");
    }
}

// ── Kessig Wolf Run ──────────────────────────────────────────

/// Kessig Wolf Run with {1}{R}{G} funds X = 1, granting +1/+0 and trample.
#[test]
fn kessig_wolf_run_grants_power_and_trample() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf_run = named_creature(&mut state, &reg, "Kessig Wolf Run", P0);
    let creature = ready_creature(&mut state, P0, 3, 3);

    // {1}{R}{G} in the pool: {R}{G} pays the non-X portion, leaving 1
    // colorless that we'll allocate to X via the funding prompt.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);

    let activated = activate(&state, &reg, wolf_run, 1, vec![Target::Object(creature)]);
    let new_state = resolve_funding_max(&activated, &reg);

    // Creature should have +1/+0 until end of turn (base 3/3 → 4/3).
    assert_eq!(new_state.effective_power(creature, &reg).unwrap(), 4);
    assert_eq!(new_state.effective_toughness(creature, &reg).unwrap(), 3);

    // Creature should have trample.
    assert!(new_state.has_keyword(creature, Keyword::Trample, &reg));
}

/// Kessig Wolf Run taps for colorless mana.
#[test]
fn kessig_wolf_run_taps_for_mana() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let wolf_run = named_creature(&mut state, &reg, "Kessig Wolf Run", P0);

    // Activate mana ability.
    let new_state = engine::submit_action(
        &state,
        &Action::ActivateManaAbility {
            object_id: wolf_run,
            ability_index: 0,
        },
        &reg,
    );

    assert_eq!(new_state.get_player(P0).mana_pool.get(ManaType::Colorless), 1);
}

// ── Snapcaster Mage ──────────────────────────────────────────

/// Snapcaster Mage grants flashback to an instant/sorcery in graveyard.
#[test]
fn snapcaster_mage_grants_flashback() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Lightning Bolt in P0's graveyard.
    let bolt = named_card_in_graveyard(&mut state, &reg, "Lightning Bolt", P0);

    // Cast Snapcaster Mage (resolve immediately for ETB trigger).
    let snap = castable_spell(&mut state, &reg, "Snapcaster Mage", P0);
    let mut new_state = cast_onto_stack(&state, &reg, snap, vec![]);
    mtg_engine::stack::resolve_top_of_stack(&mut new_state, &reg);

    mtg_engine::triggers::process_triggers(&mut new_state, &reg);

    // The grant is asserted through the engine offering the cast, not by
    // finding the `GrantFlashback` entry: the entry existing and the engine
    // honouring it are two different claims.
    new_state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    assert!(can_cast(&new_state, &reg, bolt),
        "Snapcaster Mage should let Lightning Bolt be cast from the graveyard for {{R}}");
}

