//! Cards whose behaviour is a death trigger, a token, or an anthem over them.
//!
//! Cards covered (15), so this is greppable by name as well as by rule:
//!
//! - Doomed Traveler
//! - Elder Cathar
//! - Falkenrath Noble
//! - Fiend Hunter
//! - Intangible Virtue
//! - Lumberknot
//! - Mausoleum Guard
//! - Midnight Haunting
//! - Moan of the Unhallowed
//! - Pitchburn Devils
//! - Rage Thrower
//! - Slayer of the Wicked
//! - Unruly Mob
//! - Village Bell-Ringer
//! - Village Cannibals

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::engine;
use mtg_engine::ids::PlayerId;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::triggers;
use mtg_engine::types::*;
// ══════════════════════════════════════════════════════════════════
// Token-generating spells
// ══════════════════════════════════════════════════════════════════

/// Spells that make two identical creature tokens. One shape, so the cards'
/// differences — what the token is and what it has — are the only thing the
/// table has to state.
#[test]
fn token_making_spells_make_the_tokens_they_print() {
    // (spell, token name, power, toughness, keywords)
    const SPELLS: &[(&str, &str, i32, i32, &[Keyword])] = &[
        ("Midnight Haunting", "Spirit", 1, 1, &[Keyword::Flying]),
        ("Moan of the Unhallowed", "Zombie", 2, 2, &[]),
    ];

    for &(spell_name, token_name, power, toughness, keywords) in SPELLS {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let card = castable_spell(&mut state, &reg, spell_name, P0);
        state = cast_and_resolve(&state, &reg, card, vec![]);

        assert_eq!(state.get_object(card).unwrap().zone, Zone::Graveyard,
            "{spell_name} is a sorcery; it goes to the graveyard");
        assert_eq!(count_tokens_named(&state, token_name), 2,
            "{spell_name} should make two {token_name} tokens");

        for o in state.objects.values().filter(|o| o.is_token && o.name == token_name) {
            assert_eq!((o.power, o.toughness), (Some(power), Some(toughness)),
                "{spell_name}'s tokens are {power}/{toughness}");
            for kw in keywords {
                assert!(o.keywords.contains(kw), "{spell_name}'s tokens have {kw:?}");
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════
// Dies triggers — token creators
// ══════════════════════════════════════════════════════════════════

/// "When this creature dies, create N 1/1 white Spirit creature tokens with
/// flying." Same rule, same token, different count.
#[test]
fn creatures_that_leave_spirits_behind_leave_the_right_number() {
    // (card, how many Spirits it leaves)
    const CARDS: &[(&str, usize)] = &[
        ("Doomed Traveler", 1),
        ("Mausoleum Guard", 2),
    ];

    for &(name, count) in CARDS {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let creature = named_permanent(&mut state, &reg, name, P0);
        kill_by_damage(&mut state, &reg, creature);
        triggers::process_triggers(&mut state, &reg);

        assert_eq!(count_tokens_named(&state, "Spirit"), count,
            "{name} should leave {count} Spirit token(s) behind");
        for o in state.objects.values().filter(|o| o.is_token && o.name == "Spirit") {
            assert_eq!((o.power, o.toughness), (Some(1), Some(1)), "{name}'s Spirits are 1/1");
            assert!(o.keywords.contains(&Keyword::Flying), "{name}'s Spirits fly");
        }
    }
}

// ══════════════════════════════════════════════════════════════════
// ETB triggers
// ══════════════════════════════════════════════════════════════════

/// Village Bell-Ringer has flash and untaps all creatures you control on ETB.
#[test]
fn village_bell_ringer_untaps_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Two tapped creatures on P0's side.
    let c1 = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(c1).unwrap().tapped = true;
    let c2 = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(c2).unwrap().tapped = true;

    // An opponent's tapped creature (should NOT be untapped).
    let opp = ready_creature(&mut state, P1, 1, 1);
    state.get_object_mut(opp).unwrap().tapped = true;

    // Cast Village Bell-Ringer (flash creature, castable anytime).
    let vbr = castable_spell(&mut state, &reg, "Village Bell-Ringer", P0);

    state = cast_and_resolve(&state, &reg, vbr, vec![]);

    // Process ETB triggers.
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(vbr).unwrap().zone, Zone::Battlefield);
    assert!(!state.get_object(c1).unwrap().tapped,
        "P0's creature should be untapped by Village Bell-Ringer");
    assert!(!state.get_object(c2).unwrap().tapped,
        "P0's second creature should also be untapped");
    assert!(state.get_object(opp).unwrap().tapped,
        "Opponent's creature should NOT be untapped");
}

/// Slayer of the Wicked destroys a Vampire, Werewolf, or Zombie on ETB.
#[test]
fn slayer_of_the_wicked_destroys_zombie() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 has a Walking Corpse (Zombie) on the battlefield.
    let wc = named_permanent(&mut state, &reg, "Walking Corpse", P1);

    // Cast Slayer of the Wicked.
    let slayer = castable_spell(&mut state, &reg, "Slayer of the Wicked", P0);

    state = cast_and_resolve(&state, &reg, slayer, vec![]);

    triggers::process_triggers(&mut state, &reg);

    // The prompt has to be there: with the choice inside an `if let`, a Slayer
    // that never asked would assert nothing and pass.
    assert!(state.awaiting_action.is_some(), "Slayer should ask what to destroy");
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(wc))),
        },
        &reg,
    );
    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(slayer).unwrap().zone, Zone::Battlefield,
        "Slayer should be on the battlefield");
    assert_eq!(state.get_object(wc).unwrap().zone, Zone::Graveyard,
        "Walking Corpse (Zombie) should be destroyed by Slayer of the Wicked");
}

/// Fiend Hunter exiles an opponent's creature on ETB.
#[test]
fn fiend_hunter_exiles_on_etb() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 has a creature on the battlefield.
    let victim = ready_creature(&mut state, P1, 4, 4);
    state.get_object_mut(victim).unwrap().name = "Big Creature".into();

    // Cast Fiend Hunter.
    let fh = castable_spell(&mut state, &reg, "Fiend Hunter", P0);

    state = cast_and_resolve(&state, &reg, fh, vec![]);

    triggers::process_triggers(&mut state, &reg);

    assert!(state.awaiting_action.is_some(), "Fiend Hunter should ask what to exile");
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(victim))),
        },
        &reg,
    );

    assert_eq!(state.get_object(fh).unwrap().zone, Zone::Battlefield,
        "Fiend Hunter should be on the battlefield");
    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Exile,
        "Opponent's creature should be exiled by Fiend Hunter");
}

// ══════════════════════════════════════════════════════════════════
// Dies triggers — damage and life drain
// ══════════════════════════════════════════════════════════════════

/// Pitchburn Devils deals 3 damage to any target when it dies.
#[test]
fn pitchburn_devils_deals_3_on_death() {
    use mtg_engine::actions::ResolvedChoice;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let pd = named_permanent(&mut state, &reg, "Pitchburn Devils", P0);

    kill_by_damage(&mut state, &reg, pd);

    triggers::process_triggers(&mut state, &reg);

    // Should be awaiting a target choice (both players are valid targets).
    assert!(state.awaiting_action.is_some(), "Should be awaiting damage target choice");

    // Choose opponent as target. This attaches the target to the pending
    // trigger and pushes it on the stack.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(Target::Player(P1))) },
        &reg,
    );

    // Resolve the trigger on the stack so the damage is actually applied.
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 17,
        "Opponent should lose 3 life from Pitchburn Devils dying");
}

/// Falkenrath Noble drains 1 life whenever any creature dies.
#[test]
fn falkenrath_noble_drains_on_any_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Falkenrath Noble on P0's side.
    let _noble = named_permanent(&mut state, &reg, "Falkenrath Noble", P0);

    // A creature on P0's side to kill (Noble triggers on any creature dying).
    let victim = ready_creature(&mut state, P0, 1, 1);

    kill_by_damage(&mut state, &reg, victim);

    process_triggers_auto_target_opponent(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 19,
        "P1 should lose 1 life from Falkenrath Noble's trigger");
    assert_eq!(state.get_player(P0).life, 21,
        "P0 should gain 1 life from Falkenrath Noble's trigger");
}

/// Rage Thrower deals 2 damage to the opponent when another creature dies.
#[test]
fn rage_thrower_deals_2_on_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Rage Thrower on P0's side.
    let _rt = named_permanent(&mut state, &reg, "Rage Thrower", P0);

    // A creature on P1's side to kill.
    let victim = ready_creature(&mut state, P1, 1, 1);

    kill_by_damage(&mut state, &reg, victim);

    triggers::process_triggers(&mut state, &reg);

    // Rage Thrower presents a "target player or planeswalker" choice.
    // Choose opponent (P1).
    assert!(state.awaiting_action.is_some(), "Rage Thrower should present a target choice");
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Player(P1))),
        },
        &reg,
    );
    // Resolve the trigger on the stack to actually apply the damage.
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 18,
        "P1 should lose 2 life from Rage Thrower's trigger");
}

// ══════════════════════════════════════════════════════════════════
// Dies triggers — +1/+1 counters
// ══════════════════════════════════════════════════════════════════

/// "Whenever <a creature> dies, put a +1/+1 counter on <this creature>." One
/// rule, three cards, and the only thing that separates them is which death
/// counts — so that is the only thing the table says.
///
/// The Village Cannibals rows are a matched pair: the same setup, one death
/// that qualifies and one that does not. Without the second row the test would
/// pass for a card that counted every death.
#[test]
fn a_death_watcher_counts_the_deaths_its_text_names() {
    // (watcher, victim — a named card or a vanilla 1/1, victim's controller,
    //  counters expected, what the row is testing)
    const CASES: &[(&str, Option<&str>, PlayerId, u32, &str)] = &[
        ("Unruly Mob", None, P0, 1, "another creature you control dying"),
        ("Lumberknot", None, P1, 1, "any creature dying, an opponent's included"),
        ("Village Cannibals", Some("Doomed Traveler"), P1, 1, "a Human dying"),
        ("Village Cannibals", Some("Walking Corpse"), P1, 0, "a Zombie dying is not a Human dying"),
    ];

    for &(watcher_name, victim_name, victim_controller, expected, why) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let watcher = named_permanent(&mut state, &reg, watcher_name, P0);
        let base = state.effective_power(watcher, &reg).expect("watcher is a creature");
        let victim = match victim_name {
            Some(n) => named_permanent(&mut state, &reg, n, victim_controller),
            None => ready_creature(&mut state, victim_controller, 1, 1),
        };

        kill_by_damage(&mut state, &reg, victim);
        triggers::process_triggers(&mut state, &reg);

        assert_eq!(state.get_counter_count(watcher, CounterType::PlusOnePlusOne), expected,
            "{watcher_name} on {why}");
        assert_eq!(state.effective_power(watcher, &reg), Some(base + expected as i32),
            "and the counter shows up in {watcher_name}'s power");
    }
}

/// Elder Cathar grants a +1/+1 counter to another creature you control when it dies.
#[test]
fn elder_cathar_grants_counter_on_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Elder Cathar on P0's side.
    let cathar = named_permanent(&mut state, &reg, "Elder Cathar", P0);

    // Another creature on P0's side to receive the counter.
    let buddy = ready_creature(&mut state, P0, 2, 2);

    kill_by_damage(&mut state, &reg, cathar);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_counter_count(buddy, CounterType::PlusOnePlusOne), 1,
        "Buddy creature should have received a +1/+1 counter from Elder Cathar");
    assert_eq!(state.effective_power(buddy, &reg), Some(3));
    assert_eq!(state.effective_toughness(buddy, &reg), Some(3));
}

// ══════════════════════════════════════════════════════════════════
// Anthem enchantment
// ══════════════════════════════════════════════════════════════════

/// Intangible Virtue gives creature tokens you control +1/+1 and vigilance.
#[test]
fn intangible_virtue_buffs_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a token creature (Intangible Virtue only buffs tokens).
    let token = state.create_token(
        "Spirit", P0, 2, 2,
        vec![Color::White],
        vec![CardType::Creature],
        vec![],
        &reg,
    )[0];
    state.get_object_mut(token).unwrap().summoning_sick = false;

    // Also create a non-token creature that should NOT be buffed.
    let non_token = ready_creature(&mut state, P0, 2, 2);

    // Cast Intangible Virtue.
    let iv = castable_spell(&mut state, &reg, "Intangible Virtue", P0);

    state = cast_and_resolve(&state, &reg, iv, vec![]);

    assert_eq!(state.get_object(iv).unwrap().zone, Zone::Battlefield);
    // Token should get +1/+1.
    assert_eq!(state.effective_power(token, &reg), Some(3),
        "Token should get +1 power from Intangible Virtue");
    assert_eq!(state.effective_toughness(token, &reg), Some(3),
        "Token should get +1 toughness from Intangible Virtue");
    // Token should have vigilance.
    assert!(state.has_keyword(token, Keyword::Vigilance, &reg),
        "Token should have vigilance from Intangible Virtue");
    // Non-token should NOT be buffed.
    assert_eq!(state.effective_power(non_token, &reg), Some(2),
        "Non-token should NOT get buffed by Intangible Virtue");
    assert_eq!(state.effective_toughness(non_token, &reg), Some(2),
        "Non-token should NOT get buffed by Intangible Virtue");
    // Non-token should NOT have vigilance.
    assert!(!state.has_keyword(non_token, Keyword::Vigilance, &reg),
        "Non-token should NOT have vigilance from Intangible Virtue");
}

// ══════════════════════════════════════════════════════════════════
// Token mechanics
// ══════════════════════════════════════════════════════════════════

/// Tokens have summoning sickness when created.
#[test]
fn token_has_summoning_sickness() {
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let reg = registry();
    let token = state.create_token(
        "Spirit", P0, 1, 1,
        vec![Color::White],
        vec![CardType::Creature],
        vec![Keyword::Flying],
        &reg,
    )[0];

    let obj = state.get_object(token).unwrap();
    assert!(obj.summoning_sick,
        "Token should have summoning sickness on the turn it was created");
}
