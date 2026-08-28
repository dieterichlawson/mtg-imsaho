//! Cards whose behaviour is a death trigger, a token, or an anthem over them.
//!
//! Cards covered (16), so this is greppable by name as well as by rule:
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
//! - Murder of Crows
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
        ("Midnight Haunting", "Spirit Token", 1, 1, &[Keyword::Flying]),
        ("Moan of the Unhallowed", "Zombie Token", 2, 2, &[]),
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

        assert_eq!(count_tokens_named(&state, "Spirit Token"), count,
            "{name} should leave {count} Spirit token(s) behind");
        for o in state.objects.values().filter(|o| o.is_token && o.name == "Spirit Token") {
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

// -------------------------------------------------------------------------
// Falkenrath Noble — a death-watch that sees every creature, its own included
// -------------------------------------------------------------------------

/// Falkenrath Noble SHOULD trigger when an opponent's creature dies.
/// Oracle: "Whenever this creature or another creature dies" — any creature.
#[test]
fn falkenrath_noble_triggers_on_opponent_creature_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _noble = named_permanent(&mut state, &reg, "Falkenrath Noble", P0);

    // P1's creature dies.
    let enemy = ready_creature(&mut state, P1, 1, 1);
    state.get_object_mut(enemy).unwrap().damage_marked = 2;

    let p0_life_before = state.get_player(P0).life;
    let p1_life_before = state.get_player(P1).life;

    check_state_based_actions(&mut state, &reg);
    process_triggers_auto_target_opponent(&mut state, &reg);

    // Noble SHOULD trigger — "another creature dies" includes opponent's creatures.
    assert_eq!(state.get_player(P0).life, p0_life_before + 1,
        "Falkenrath Noble should gain 1 life when any creature dies");
    assert_eq!(state.get_player(P1).life, p1_life_before - 1,
        "Falkenrath Noble should drain opponent when any creature dies");
}

/// Falkenrath Noble SHOULD trigger when your own creature dies.
#[test]
fn falkenrath_noble_triggers_on_own_creature_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _noble = named_permanent(&mut state, &reg, "Falkenrath Noble", P0);
    let ally = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(ally).unwrap().damage_marked = 2;

    let p0_life_before = state.get_player(P0).life;
    let p1_life_before = state.get_player(P1).life;

    check_state_based_actions(&mut state, &reg);
    process_triggers_auto_target_opponent(&mut state, &reg);

    assert_eq!(state.get_player(P0).life, p0_life_before + 1,
        "Falkenrath Noble should gain 1 life when your creature dies");
    assert_eq!(state.get_player(P1).life, p1_life_before - 1,
        "Falkenrath Noble should drain opponent when your creature dies");
}

/// Falkenrath Noble SHOULD trigger on itself dying.
/// Oracle: "Whenever THIS CREATURE or another creature dies" — includes self.
#[test]
fn falkenrath_noble_triggers_on_self_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let noble = named_permanent(&mut state, &reg, "Falkenrath Noble", P0);
    state.get_object_mut(noble).unwrap().damage_marked = 5;

    let p0_life_before = state.get_player(P0).life;
    let p1_life_before = state.get_player(P1).life;

    check_state_based_actions(&mut state, &reg);
    process_triggers_auto_target_opponent(&mut state, &reg);

    // Noble SHOULD trigger on its own death ("this creature ... dies").
    assert_eq!(state.get_player(P0).life, p0_life_before + 1,
        "Falkenrath Noble should trigger on its own death");
    assert_eq!(state.get_player(P1).life, p1_life_before - 1,
        "Falkenrath Noble should drain opponent on its own death");
}

/// Ruling: "If Falkenrath Noble and another creature die at the same time,
/// Falkenrath Noble's triggered ability will trigger for each of them."
///
/// Two triggers, so two drains. It works because the death-watch collector
/// treats permanents that left in the same event batch as still having been
/// there (CR 603.10a) — the Noble is a legal watcher of the other creature's
/// death even though it died alongside it — while excluding the dead creature
/// from watching its *own* death, which is what the separate self-dies arm is
/// for. Get either half wrong and this reads 1 or 3.
#[test]
fn falkenrath_noble_triggers_once_per_creature_when_it_dies_alongside_another() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let noble = named_permanent(&mut state, &reg, "Falkenrath Noble", P0);
    let ally = ready_creature(&mut state, P0, 1, 1);
    // Both take lethal damage, so both die in the same state-based action pass.
    state.get_object_mut(noble).unwrap().damage_marked = 5;
    state.get_object_mut(ally).unwrap().damage_marked = 2;

    let mine = state.get_player(P0).life;
    let theirs = state.get_player(P1).life;

    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(noble).unwrap().zone, Zone::Graveyard,
        "test precondition: both died");
    assert_eq!(state.get_object(ally).unwrap().zone, Zone::Graveyard,
        "test precondition: both died");

    process_triggers_auto_target_opponent(&mut state, &reg);

    assert_eq!(state.get_player(P0).life, mine + 2,
        "one trigger for each creature that died");
    assert_eq!(state.get_player(P1).life, theirs - 2,
        "and each drains the target for 1");
}

/// "...and **you** gain 1 life." When the Noble itself dies, "you" is its last
/// known controller (CR 608.2g) — leaving the battlefield resets `controller`
/// to `owner` (CR 400.7), so a Noble whose owner and controller agree cannot
/// tell a correct read from a read of the reset field.
#[test]
fn falkenrath_nobles_life_goes_to_its_last_controller() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let noble = named_permanent(&mut state, &reg, "Falkenrath Noble", P0);
    state.get_object_mut(noble).unwrap().owner = P1;
    state.get_object_mut(noble).unwrap().damage_marked = 5;

    let mine = state.get_player(P0).life;
    let theirs = state.get_player(P1).life;

    check_state_based_actions(&mut state, &reg);
    process_triggers_auto_target_opponent(&mut state, &reg);

    assert_eq!(state.get_player(P0).life, mine + 1,
        "the life goes to the player who controlled the Noble, not its owner");
    assert_eq!(state.get_player(P1).life, theirs - 1,
        "and the opponent is the one drained");
}

// -------------------------------------------------------------------------
// Selhoff Occultist — the same death-watch shape, milling instead of draining
// -------------------------------------------------------------------------

/// "Whenever this creature or another creature dies, target player mills a
/// card." The plain effect, which nothing tested: the card's coverage was the
/// simultaneous-death regression and the hexproof-target filter, neither of
/// which asserts that a mill of one card happens on an ordinary death.
///
/// Both arms of "this creature or another": an ally dying, and the Occultist
/// itself. Its own death must mill once, not twice — the card declares two
/// triggered abilities for one ability, and only the collector's exclusion of
/// the dead creature from watching its own death keeps that honest.
#[test]
fn selhoff_occultist_mills_one_card_per_creature_death() {
    let reg = registry();
    for kill_the_occultist in [false, true] {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let occultist = named_permanent(&mut state, &reg, "Selhoff Occultist", P0);
        let ally = ready_creature(&mut state, P0, 1, 1);
        stock_library(&mut state, &reg, P1, 5);
        let before = state.get_player(P1).library_order.len();

        let victim = if kill_the_occultist { occultist } else { ally };
        state.get_object_mut(victim).unwrap().damage_marked = 99;
        check_state_based_actions(&mut state, &reg);
        process_triggers_auto_target_opponent(&mut state, &reg);

        assert_eq!(before - state.get_player(P1).library_order.len(), 1,
            "kill_the_occultist = {kill_the_occultist}: exactly one card, and \
             its own death is one trigger rather than two");
    }
}

/// A creature card the Occultist mills is seen by Undead Alchemist —
/// "whenever a creature card is put into an opponent's graveyard from their
/// library, exile that card and create a Zombie token". Here P0 controls both,
/// and the mill lands on P1.
///
/// This does *not* show which code path did the milling, and an earlier
/// version of this comment claimed it did. `move_object` emits
/// `CreatureCardMilled` for any library-to-graveyard move of a creature card,
/// deliberately — being a mill is a property of the zone change, not of the
/// caller having remembered a helper — so a card that moved the top card by
/// hand would reach an Alchemist just as well. What this pins is the
/// cross-card interaction, and that the Occultist mills the *targeted* player.
#[test]
fn selhoff_occultists_mill_is_visible_to_undead_alchemist() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    named_permanent(&mut state, &reg, "Undead Alchemist", P0);
    named_permanent(&mut state, &reg, "Selhoff Occultist", P0);
    // Top of P1's library — `mill_cards` takes index 0.
    let top = state.create_object(
        reg.get_id_by_name("Walking Corpse").unwrap(), P1, Zone::Library, Some(2), Some(2));
    state.get_player_mut(P1).library_order.insert(0, top);
    // One in P0's library too, so "the right player milled" is a real claim
    // rather than the only card that could possibly have moved.
    let mine = state.create_object(
        reg.get_id_by_name("Walking Corpse").unwrap(), P0, Zone::Library, Some(2), Some(2));
    state.get_player_mut(P0).library_order.insert(0, mine);

    let ally = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(ally).unwrap().damage_marked = 99;
    check_state_based_actions(&mut state, &reg);
    process_triggers_auto_target_opponent(&mut state, &reg);

    assert_eq!(state.get_object(top).unwrap().zone, Zone::Exile,
        "P1 was the target, so P1 milled — and the Alchemist exiled the card");
    assert_eq!(state.get_object(mine).unwrap().zone, Zone::Library,
        "the Occultist's controller did not mill; the target did");
}

// -------------------------------------------------------------------------
// Murder of Crows
// -------------------------------------------------------------------------

/// Murder of Crows: when another creature dies, the controller should get
/// a choice to draw (optional). If they draw, they must choose a card to discard.
#[test]
fn murder_of_crows_presents_draw_choice() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has Murder of Crows.
    let _crows = named_permanent(&mut state, &reg, "Murder of Crows", P0);

    // Give P0 some cards in hand so the discard has options.
    let hand_card = state.create_object(CardId(9999), P0, Zone::Hand, None, None);
    state.get_object_mut(hand_card).unwrap().name = "Hand Card".into();

    // A creature dies (P1's).
    let victim = ready_creature(&mut state, P1, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;

    // Give P0 library cards to draw from.
    let lib_card = state.create_object(CardId(9999), P0, Zone::Library, None, None);
    state.get_object_mut(lib_card).unwrap().name = "Library Card".into();
    state.get_player_mut(P0).library_order.push(lib_card);

    state.events.clear();
    state.trigger_event_index = 0;
    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    // Murder of Crows should present a "you may draw" yes/no choice.
    assert!(state.awaiting_action.is_some(),
        "Murder of Crows should present a yes/no draw choice");
    // Hand should still have 1 card (draw hasn't happened yet — waiting for choice).
    let hand_count = state.objects_in_zone(Zone::Hand, P0).len();
    assert_eq!(hand_count, 1,
        "Draw should NOT have happened yet (waiting for 'you may' choice)");
}

/// A Murder of Crows on the battlefield, another creature freshly dead, and
/// the "you may draw" question on the table. `library` is how many cards P0
/// has left to draw from; `hand` is how many they are holding.
fn crows_death_trigger(
    reg: &mtg_engine::cards::CardRegistry,
    library: usize,
    hand: usize,
) -> (mtg_engine::state::GameState, Vec<ObjectId>) {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, reg, "Murder of Crows", P0);

    let hand_cards: Vec<ObjectId> = (0..hand).map(|i| {
        let id = state.create_object(CardId(9999), P0, Zone::Hand, None, None);
        state.get_object_mut(id).unwrap().name = format!("Hand Card {i}");
        id
    }).collect();
    for i in 0..library {
        let id = state.create_object(CardId(9999), P0, Zone::Library, None, None);
        state.get_object_mut(id).unwrap().name = format!("Library Card {i}");
        state.get_player_mut(P0).library_order.push(id);
    }

    let victim = ready_creature(&mut state, P1, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;

    state.events.clear();
    state.trigger_event_index = 0;
    check_state_based_actions(&mut state, reg);
    triggers::process_triggers(&mut state, reg);
    (state, hand_cards)
}

fn answer(state: &mtg_engine::state::GameState, reg: &mtg_engine::cards::CardRegistry,
          choice: mtg_engine::actions::ResolvedChoice) -> mtg_engine::state::GameState {
    engine::submit_action(state, &Action::ResolveChoice { choice }, reg)
}

/// "you may draw a card. **If you do, discard a card.**" Saying yes does both,
/// in one resolution. Only the yes/no prompt was tested; the half of the card
/// that actually does something was not.
#[test]
fn murder_of_crows_draws_and_then_discards_when_you_accept() {
    let reg = registry();
    // An empty hand, so after the draw there is exactly one card and the
    // discard needs no further choice — the whole ability in one answer.
    let (state, _) = crows_death_trigger(&reg, 1, 0);

    let state = answer(&state, &reg, mtg_engine::actions::ResolvedChoice::YesNoDecision(true));

    assert!(state.objects_in_zone(Zone::Hand, P0).is_empty(),
        "the drawn card was then discarded, so the hand is empty again");
    assert_eq!(state.objects_in_zone(Zone::Graveyard, P0).len(), 1,
        "and it is in the graveyard");
    assert!(state.get_player(P0).library_order.is_empty(),
        "and it did come off the library — the draw really happened");
    assert!(state.awaiting_action.is_none(), "nothing is left pending");
}

/// "**you may** draw" — declining does nothing at all. Without this, an
/// implementation that ignored the answer and always drew would pass the
/// accepting test above.
#[test]
fn murder_of_crows_does_nothing_when_you_decline() {
    let reg = registry();
    let (state, _) = crows_death_trigger(&reg, 1, 1);

    let state = answer(&state, &reg, mtg_engine::actions::ResolvedChoice::YesNoDecision(false));

    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 1,
        "declining draws nothing");
    assert_eq!(state.get_player(P0).library_order.len(), 1,
        "and the card stays in the library");
    assert!(state.objects_in_zone(Zone::Graveyard, P0).is_empty(),
        "and nothing is discarded");
}

/// "If you do, discard a card." The discard is conditional on the draw having
/// happened, not on the ability resolving. With an empty library nothing is
/// drawn, so nothing is discarded — even though the player is holding cards
/// that an implementation checking the hand instead would take one of.
///
/// The card's own comment records this having been the behaviour once. A fixed
/// bug with no test is one refactor from coming back.
#[test]
fn murder_of_crows_discards_nothing_when_the_draw_found_no_card() {
    let reg = registry();
    let (state, _) = crows_death_trigger(&reg, 0, 2);

    let state = answer(&state, &reg, mtg_engine::actions::ResolvedChoice::YesNoDecision(true));

    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 2,
        "the library was empty, so no card was drawn and none may be discarded");
    assert!(state.objects_in_zone(Zone::Graveyard, P0).is_empty(),
        "'if you do' was not satisfied");
    // Not merely "nothing discarded yet": with two cards in hand the discard
    // would be a *choice*, and a version that asked for one would leave the
    // hand and graveyard exactly as they are here. The ability has to be
    // finished, with nothing outstanding.
    assert!(state.awaiting_action.is_none(),
        "no discard is pending either — the ability is over, not waiting on a \
         card to throw away; got {:?}", state.awaiting_action);
}

/// Scryfall ruling (2018-03-16): "You can't do anything in between drawing a
/// card and discarding a card, including casting or cycling the card you
/// drew."
///
/// The draw and the discard are one resolution. With more than one card in
/// hand the discard is a choice, and while it is pending the only legal
/// actions are the ones that answer it — nobody receives priority.
#[test]
fn murder_of_crows_gives_nobody_priority_between_the_draw_and_the_discard() {
    let reg = registry();
    let (state, _) = crows_death_trigger(&reg, 1, 2);

    let state = answer(&state, &reg, mtg_engine::actions::ResolvedChoice::YesNoDecision(true));

    assert!(matches!(state.awaiting_action,
            Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
                choice: mtg_engine::state::ResolutionChoiceKind::ChooseCardFromHand { .. }, .. })),
        "the draw happened and the discard is pending, mid-resolution");
    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 3,
        "the drawn card is in hand and nothing has been discarded yet");

    let legal = engine::legal_actions(&state, &reg);
    assert!(!legal.actions.is_empty(), "test precondition: there is something to do");
    assert!(legal.actions.iter().all(|a| matches!(a, Action::ResolveChoice { .. })),
        "the only thing anyone may do is answer the discard; got {:?}", legal.actions);

    // And answering it finishes the ability: the chosen card is discarded and
    // the hand is back to what it was.
    let chosen = match &state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            choice: mtg_engine::state::ResolutionChoiceKind::ChooseCardFromHand { cards, .. }, .. })
            => cards[0],
        other => panic!("expected a discard choice, got {other:?}"),
    };
    let state = answer(&state, &reg, mtg_engine::actions::ResolvedChoice::ChosenCard(chosen));
    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 2,
        "drew one and discarded one");
    assert_eq!(state.get_object(chosen).unwrap().zone, Zone::Graveyard,
        "and it was the card the player picked");
}

/// "Whenever **another** creature dies" — the Crows dying on their own is not
/// another creature dying, and must not offer the draw.
#[test]
fn murder_of_crows_does_not_trigger_on_its_own_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let crows = named_permanent(&mut state, &reg, "Murder of Crows", P0);
    let lib = state.create_object(CardId(9999), P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(lib);

    state.get_object_mut(crows).unwrap().damage_marked = 4;
    state.events.clear();
    state.trigger_event_index = 0;
    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(crows).unwrap().zone, Zone::Graveyard,
        "test precondition: the Crows died");
    assert!(state.awaiting_action.is_none(),
        "the Crows' own death is not 'another creature', so no draw is offered");
}
