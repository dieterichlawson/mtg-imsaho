//! Creatures with evasion keywords, and creatures whose power and toughness
//! are a function of the graveyard (CR 208.2).
//!
//! Cards covered (6), so this is greppable by name as well as by rule:
//!
//! - Battleground Geist
//! - Gallows Warden
//! - Geist-Honored Monk
//! - Orchard Spirit
//! - Spider Spawning
//! - Wreath of Geists
//!
//! Festerhide Boar's morbid "enters with counters" is in `intervening_if.rs`,
//! with the rest of CR 603.4.

mod common;

use common::*;
use mtg_engine::actions::Target;
use mtg_engine::combat;
use mtg_engine::ids::CardId;
use mtg_engine::triggers;
use mtg_engine::types::*;
// ── Spirit lords ───────────────────────────────────────────────────

/// "Other Spirit creatures you control get +N/+M." Four claims in one line —
/// other, Spirit, you control, and the size — so each lord is checked against
/// a board holding one of each.
#[test]
fn a_spirit_lord_buffs_other_spirits_you_control_and_nothing_else() {
    // (lord, its printed size, the power/toughness it grants)
    const LORDS: &[(&str, (i32, i32), (i32, i32))] = &[
        ("Battleground Geist", (3, 3), (1, 0)),
        ("Gallows Warden", (3, 3), (0, 1)),
    ];

    for &(name, printed, (dp, dt)) in LORDS {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);

        let lord = named_permanent(&mut state, &reg, name, P0);
        let spirit = named_permanent(&mut state, &reg, "Chapel Geist", P0);   // 2/3 Spirit
        let non_spirit = ready_creature(&mut state, P0, 2, 2);
        let their_spirit = named_permanent(&mut state, &reg, "Chapel Geist", P1);

        let pt = |s: &mtg_engine::state::GameState, id| {
            (s.effective_power(id, &reg).unwrap(), s.effective_toughness(id, &reg).unwrap())
        };

        assert_eq!(pt(&state, lord), printed, "{name}: 'other' excludes itself");
        assert_eq!(pt(&state, spirit), (2 + dp, 3 + dt), "{name}: your own Spirit is buffed");
        assert_eq!(pt(&state, non_spirit), (2, 2), "{name}: a non-Spirit is not");
        assert_eq!(pt(&state, their_spirit), (2, 3), "{name}: an opponent's Spirit is not");
    }
}

// ── Dynamic P/T ────────────────────────────────────────────────────

/// Geist-Honored Monk P/T equals creatures you control, ETB creates 2 tokens.
#[test]
fn geist_honored_monk_dynamic_pt_and_tokens() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let monk = castable_spell(&mut state, &reg, "Geist-Honored Monk", P0);
    stock_library(&mut state, &reg, P0, 5);

    state = cast_and_resolve(&state, &reg, monk, vec![]);
    triggers::process_triggers(&mut state, &reg);

    let creatures = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == P0 && o.power.is_some())
        .count();
    assert_eq!(creatures, 3, "Monk + 2 Spirit tokens");
    assert_eq!(state.effective_power(monk, &reg), Some(3),
        "and its P/T counts them, itself included");
    assert_eq!(state.effective_toughness(monk, &reg), Some(3));
}

/// "create two 1/1 white **Spirit** creature tokens **with flying**." The test
/// above counts three creatures, which two colourless vanilla 1/1s would also
/// satisfy. This pins what the tokens actually are.
#[test]
fn geist_honored_monk_makes_two_flying_white_spirits() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    stock_library(&mut state, &reg, P0, 5);

    let monk = castable_spell(&mut state, &reg, "Geist-Honored Monk", P0);
    let mut state = cast_and_resolve(&state, &reg, monk, vec![]);
    triggers::process_triggers(&mut state, &reg);

    let tokens: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == P0 && o.id != monk)
        .map(|o| o.id)
        .collect();
    assert_eq!(tokens.len(), 2, "two tokens, not one and not three");

    for t in tokens {
        assert_eq!(state.effective_power(t, &reg), Some(1), "1/1");
        assert_eq!(state.effective_toughness(t, &reg), Some(1), "1/1");
        assert!(state.has_subtype(t, "Spirit", &reg), "Spirit");
        assert!(state.has_keyword(t, Keyword::Flying, &reg), "with flying");
        assert_eq!(state.get_object(t).unwrap().colors, vec![Color::White], "white");
    }
}

/// "equal to the number of creatures **you control**" — an opponent's board
/// does not feed it, and the count is a characteristic-defining ability, so it
/// tracks the battlefield rather than being fixed when the Monk resolved.
#[test]
fn geist_honored_monks_count_is_yours_alone_and_keeps_up() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let monk = named_permanent(&mut state, &reg, "Geist-Honored Monk", P0);
    assert_eq!(state.effective_power(monk, &reg), Some(1),
        "alone on the battlefield it still counts itself (ruling: \"its second \
         ability will count itself\")");

    for _ in 0..3 {
        ready_creature(&mut state, P1, 2, 2);
    }
    assert_eq!(state.effective_power(monk, &reg), Some(1),
        "three creatures the opponent controls change nothing");

    let mine = ready_creature(&mut state, P0, 2, 2);
    assert_eq!(state.effective_power(monk, &reg), Some(2), "one of mine does");
    assert_eq!(state.effective_toughness(monk, &reg), Some(2));

    state.move_object(mine, Zone::Graveyard, &reg);
    assert_eq!(state.effective_power(monk, &reg), Some(1),
        "and it drops again when that creature leaves — the count is recomputed, \
         not snapshotted");
}

/// Ruling: "The ability that defines Geist-Honored Monk's power and toughness
/// works in all zones, not just the battlefield." That is CR 604.3 — a
/// characteristic-defining ability functions everywhere, including in a
/// graveyard, where CR 109.5 makes "you" the card's owner because a card
/// outside the battlefield has no controller.
#[test]
fn geist_honored_monks_defining_ability_works_outside_the_battlefield() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let monk = named_card_in_graveyard(&mut state, &reg, "Geist-Honored Monk", P0);
    assert_eq!(state.get_object(monk).unwrap().zone, Zone::Graveyard, "test precondition");
    assert_eq!(state.effective_power(monk, &reg), Some(0),
        "no creatures on the battlefield, and it is not there to count itself");

    ready_creature(&mut state, P0, 2, 2);
    ready_creature(&mut state, P0, 2, 2);
    assert_eq!(state.effective_power(monk, &reg), Some(2),
        "the defining ability still runs while the card sits in a graveyard");
    assert_eq!(state.effective_toughness(monk, &reg), Some(2));
}

/// "Enchanted creature gets +X/+X, where X is the number of creature **cards**
/// in your graveyard."
///
/// Ruling (2011-09-22): "The value of X is constantly updated as creature cards
/// are put into or removed from your graveyard."
///
/// Every clause gets a step. The version this replaces added anonymous
/// `CardId(9999)` objects with a P/T, which CR 205.1b makes creatures — so it
/// showed the count going up and nothing else. Counting *every* card in the
/// graveyard rather than the creature cards passed the whole suite.
#[test]
fn wreath_of_geists_counts_the_creature_cards_in_its_controllers_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    let wreath = castable_spell(&mut state, &reg, "Wreath of Geists", P0);
    let mut state = cast_and_resolve(&state, &reg, wreath, vec![Target::Object(creature)]);

    assert_eq!(state.effective_power(creature, &reg), Some(2),
        "an empty graveyard is X = 0");

    let corpse = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    assert_eq!(state.effective_power(creature, &reg), Some(3),
        "a creature card in the graveyard raises X");
    assert_eq!(state.effective_toughness(creature, &reg), Some(3));

    // "creature cards" — a land card is not one.
    named_card_in_graveyard(&mut state, &reg, "Forest", P0);
    assert_eq!(state.effective_power(creature, &reg), Some(3),
        "a land card in the same graveyard is not a creature card");

    // CR 109.1: nor is a token, which sits in the graveyard until the next
    // state-based-action check (CR 704.5e).
    let token = state.create_token_with_subtypes(
        "Zombie", P0, 2, 2, vec![Color::Black], vec![CardType::Creature],
        vec![], vec!["Zombie".into()], &reg)[0];
    state.move_object(token, Zone::Graveyard, &reg);
    assert_eq!(state.effective_power(creature, &reg), Some(3),
        "a creature TOKEN in the graveyard is not a creature card");

    // "…or removed from your graveyard" — the count is live, not a snapshot.
    state.move_object(corpse, Zone::Exile, &reg);
    assert_eq!(state.effective_power(creature, &reg), Some(2),
        "the creature card left the graveyard, so X drops again");
}

/// "…in **your** graveyard" — the Aura's controller's, which is not the same
/// player as the enchanted creature's controller once the Aura is on something
/// an opponent controls.
///
/// Reading the enchanted creature's controller instead passed the whole suite.
#[test]
fn wreath_of_geists_counts_its_own_controllers_graveyard_not_the_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // The creature belongs to the opponent, and so does a well-stocked
    // graveyard that must not count.
    let theirs = ready_creature(&mut state, P1, 2, 2);
    for _ in 0..3 {
        named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P1);
    }
    // One creature card in the Aura controller's own graveyard.
    named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

    let wreath = castable_spell(&mut state, &reg, "Wreath of Geists", P0);
    let state = cast_and_resolve(&state, &reg, wreath, vec![Target::Object(theirs)]);

    assert_eq!(state.effective_power(theirs, &reg), Some(3),
        "X is the one creature card in the Aura controller's graveyard, not \
         the three in the enchanted creature's controller's");
    assert_eq!(state.effective_toughness(theirs, &reg), Some(3));
}

// ── Block restriction ──────────────────────────────────────────────

/// "Orchard Spirit can't be blocked except by creatures with flying or reach."
/// Three rows, because "the ground creature can't block it" alone is also true
/// of an Orchard Spirit that nothing at all could block.
#[test]
fn orchard_spirit_is_blocked_only_by_flying_or_reach() {
    let reg = registry();
    // (blocker, may it block)
    const BLOCKERS: &[(&str, bool, &str)] = &[
        ("Walking Corpse", false, "a ground creature"),
        ("Chapel Geist", true, "flying"),
        ("Somberwald Spider", true, "reach"),
    ];

    for &(name, may_block, why) in BLOCKERS {
        let mut state = game_at_step(Step::DeclareBlockers, P0);
        let spirit = named_permanent(&mut state, &reg, "Orchard Spirit", P0);
        let blocker = named_permanent(&mut state, &reg, name, P1);

        assert_eq!(combat::can_block_attacker(&state, blocker, spirit, &reg), may_block,
            "{name} ({why})");
    }
}

// ── Token creation from graveyard ──────────────────────────────────

/// Spider Spawning creates tokens equal to creatures in graveyard.
#[test]
fn spider_spawning_creates_tokens() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    for i in 0..4 {
        let c = state.create_object(CardId(9999), P0, Zone::Graveyard, Some(1), Some(1));
        state.get_object_mut(c).unwrap().name = format!("Dead {i}");
    }

    let ss = castable_spell(&mut state, &reg, "Spider Spawning", P0);
    state = cast_and_resolve(&state, &reg, ss, vec![]);

    assert_eq!(count_tokens_named(&state, "Spider Token"), 4, "one Spider per creature card");
    for spider in state.objects.values().filter(|o| o.is_token && o.name == "Spider") {
        assert_eq!((spider.power, spider.toughness), (Some(1), Some(2)));
    }
}
