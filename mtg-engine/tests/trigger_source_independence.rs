//! CR 113.7a: a triggered ability on the stack exists independently of its
//! source. Destroying the source in response does not counter the ability.
//!
//! The engine's trigger dispatch used to gate half its arms on the source
//! still being on the battlefield and leave the other half ungated — the
//! split ran straight through matched pairs, so a creature's own
//! combat-damage trigger resolved after the creature died while a watcher's
//! did not. The gate is gone from the engine; these are the cards that had
//! re-implemented it themselves.

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::state::StackEntry;
use mtg_engine::triggers::{DeadCreature, PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::*;

/// Push a trigger for `source` directly onto the stack, the way the collector
/// would have, then remove the source and resolve.
fn resolve_after_source_dies(
    state: &mut mtg_engine::state::GameState,
    reg: &CardRegistry,
    source: mtg_engine::ids::ObjectId,
    event: TriggerEvent,
) {
    resolve_after_source_dies_targeting(state, reg, source, event, vec![]);
}

/// The same, for an ability that targets: CR 603.3d locks targets in when the
/// trigger goes on the stack, so they are chosen before the source dies.
fn resolve_after_source_dies_targeting(
    state: &mut mtg_engine::state::GameState,
    reg: &CardRegistry,
    source: mtg_engine::ids::ObjectId,
    event: TriggerEvent,
    targets: Vec<Target>,
) {
    let card_id = state.get_object(source).unwrap().card_id;
    let controller = state.get_object(source).unwrap().controller;
    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource { chosen_targets: targets, ..TriggerSource::new(source, card_id, controller, "") },
        event,
    }));
    state.move_object(source, Zone::Graveyard, reg);
    mtg_engine::triggers::resolve_next_trigger(state, reg);
}

// Rakish Heir: "Whenever a Vampire you control deals combat damage to a
// player, put a +1/+1 counter on that Vampire." Trading with a blocker in the
// same combat damage step must not cost the other Vampire its counter.
#[test]
fn rakish_heir_gives_its_counter_after_trading_in_combat() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let heir = named_creature(&mut state, &reg, "Rakish Heir", P0);
    let other = named_creature(&mut state, &reg, "Stromkirk Noble", P0);

    resolve_after_source_dies(&mut state, &reg, heir,
        TriggerEvent::AnyCombatDamageToPlayer { dealer: other, damaged_player: P1, amount: 1 });

    assert_eq!(counters_of(&state, other, CounterType::PlusOnePlusOne), 1,
        "CR 113.7a: the Heir dying in the same combat damage step does not \
         counter its trigger, so the other Vampire still gets its counter");
}

// Balefire Dragon: "Whenever Balefire Dragon deals combat damage to a player,
// it deals that much damage to each creature that player controls."
#[test]
fn balefire_dragon_wipes_the_board_after_being_killed_in_response() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let dragon = named_creature(&mut state, &reg, "Balefire Dragon", P0);
    let victim = ready_creature(&mut state, P1, 2, 2);

    resolve_after_source_dies(&mut state, &reg, dragon,
        TriggerEvent::CombatDamageToPlayer { damaged_player: P1, amount: 6 });

    assert!(state.get_object(victim).is_none_or(|o| o.damage_marked >= 6),
        "CR 113.7a: killing the Dragon with its trigger on the stack must not \
         save the defending player's creatures");
}

// Curiosity: "Whenever enchanted creature deals damage to an opponent, you may
// draw a card." Destroying the Aura in response still offers the draw.
#[test]
fn curiosity_offers_its_draw_after_the_aura_is_destroyed() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let bearer = ready_creature(&mut state, P0, 2, 2);
    let aura = named_creature(&mut state, &reg, "Curiosity", P0);
    state.get_object_mut(aura).unwrap().attached_to = Some(bearer);

    resolve_after_source_dies(&mut state, &reg, aura,
        TriggerEvent::AnyDamageToPlayer { dealer: bearer, damaged_player: P1, amount: 2 });

    assert!(state.awaiting_action.is_some(),
        "CR 113.7a: destroying Curiosity in response to its own trigger must \
         still present the 'you may draw a card' choice");
}

// Burning Vengeance: "Whenever you cast an instant or sorcery spell from your
// graveyard, Burning Vengeance deals 2 damage to any target."
#[test]
fn burning_vengeance_deals_its_damage_after_being_destroyed() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let vengeance = named_creature(&mut state, &reg, "Burning Vengeance", P0);
    let spell = named_card_in_graveyard(&mut state, &reg, "Think Twice", P0);
    state.get_object_mut(spell).unwrap().cast_with_flashback = true;

    resolve_after_source_dies_targeting(&mut state, &reg, vengeance,
        TriggerEvent::SpellCast { caster: P0, spell_id: spell }, vec![Target::Player(P1)]);

    assert_eq!(state.get_player(P1).life, 18,
        "CR 113.7a: destroying Burning Vengeance in response still deals the 2 damage");
}

// Curse of the Bloody Tome: "At the beginning of enchanted player's upkeep,
// that player mills two cards." Destroying the Curse in response still mills —
// and the trigger still knows whom it cursed (CR 608.2, last known information).
#[test]
fn curse_of_the_bloody_tome_mills_after_the_curse_is_destroyed() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P1);

    let curse = attach_curse_to_player(&mut state, &reg, "Curse of the Bloody Tome", P0, P1);
    for _ in 0..5 {
        let id = state.create_object(mtg_engine::ids::CardId(9999), P1, Zone::Library, None, None);
        state.get_player_mut(P1).library_order.push(id);
    }
    let before = state.objects_in_zone(Zone::Graveyard, P1).len();

    resolve_after_source_dies(&mut state, &reg, curse, TriggerEvent::Upkeep);

    assert_eq!(state.objects_in_zone(Zone::Graveyard, P1).len(), before + 2,
        "CR 113.7a/608.2: the Curse's mill still happens, and the trigger still \
         knows which player was cursed");
}

// Curse of Stalked Prey: "Whenever a creature deals combat damage to enchanted
// player, put a +1/+1 counter on that creature."
#[test]
fn curse_of_stalked_prey_gives_its_counter_after_the_curse_is_destroyed() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let curse = attach_curse_to_player(&mut state, &reg, "Curse of Stalked Prey", P0, P1);
    let attacker = ready_creature(&mut state, P0, 2, 2);

    resolve_after_source_dies(&mut state, &reg, curse,
        TriggerEvent::AnyCombatDamageToPlayer { dealer: attacker, damaged_player: P1, amount: 2 });

    assert_eq!(counters_of(&state, attacker, CounterType::PlusOnePlusOne), 1,
        "CR 113.7a/608.2: the counter is placed even though the Curse is gone");
}

// ---------------------------------------------------------------------------
// Structural guards
// ---------------------------------------------------------------------------

fn engine_sources() -> Vec<(std::path::PathBuf, String)> {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).expect("readable").flatten() {
            let p = e.path();
            if p.is_dir() { walk(&p, out); }
            else if p.extension().is_some_and(|x| x == "rs") { out.push(p); }
        }
    }
    walk(&src, &mut files);
    files.into_iter()
        .map(|p| { let t = std::fs::read_to_string(&p).expect("readable"); (p, t) })
        .collect()
}

/// A trigger is built in exactly one place per event, and only where events
/// are turned into triggers.
///
/// `TriggerSource` is the one way to name a trigger's source, so counting its
/// constructions counts the ways a trigger can come into existence. Everything
/// that reads a `GameEvent` goes through `triggers/collect/`; the two
/// exceptions are triggers the engine raises itself rather than off an event —
/// a state-triggered ability (CR 603.8) during SBA processing, and the ETB
/// ability a copy effect gives a new copy (CR 614.12).
#[test]
fn triggers_are_built_in_one_place() {
    const ALLOWED: &[&str] = &[
        "triggers/collect/mod.rs", // Collector::emit — every event-driven trigger
        "sba.rs",                  // CR 603.8 state-triggered abilities
        "engine/effects.rs",       // CR 614.12 ETB for a permanent that entered as a copy
    ];
    let mut offenders = Vec::new();
    for (path, text) in engine_sources() {
        let rel = path.to_string_lossy().replace('\\', "/");
        if ALLOWED.iter().any(|a| rel.ends_with(a)) {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            let l = line.trim();
            if l.starts_with("pub struct TriggerSource") || l.starts_with("impl TriggerSource") {
                continue; // the definition, not a construction
            }
            if l.contains("TriggerSource::new(") || l.contains("TriggerSource {") {
                offenders.push(format!("{rel}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(offenders.is_empty(),
        "triggers must be built through Collector::emit in triggers/collect/, \
         not spread across the engine:\n{}", offenders.join("\n"));
}

/// `resolve_next_trigger` does not consult the source's zone.
///
/// CR 113.7a: a triggered ability on the stack is independent of its source.
/// Ten of the old twenty dispatch arms gated on the source still being on the
/// battlefield and ten did not; the rule is now stated once, by not being
/// there at all. A handler that needs its permanent present checks for itself.
#[test]
fn trigger_dispatch_does_not_gate_on_the_source_zone() {
    let (_, text) = engine_sources().into_iter()
        .find(|(p, _)| p.file_name().is_some_and(|f| f == "triggers.rs"))
        .expect("triggers.rs");
    let start = text.find("pub fn resolve_next_trigger").expect("resolve_next_trigger");
    let body = &text[start..];
    let end = body.find("\npub fn process_triggers").unwrap_or(body.len());
    let offenders: Vec<&str> = body[..end].lines()
        .filter(|l| l.contains("Zone::Battlefield") && !l.trim_start().starts_with("//"))
        .collect();
    assert!(offenders.is_empty(),
        "CR 113.7a: trigger dispatch must not check whether the source is still \
         on the battlefield:\n{}", offenders.join("\n"));
}

// -------------------------------------------------------------------------
// Per-card cases
//
// Each of these is the same rule as the six above, reached through a different
// card's trigger. They go through `resolve_after_source_dies` for exactly the
// reason the helper exists: the shape (stack the trigger, remove the source,
// resolve) is the rule, and a test that hand-rolls it can quietly stop testing
// it.

// Angel of Flight Alabaster: "At the beginning of your upkeep, return target
// Spirit card from your graveyard to your hand."
#[test]
fn angel_of_flight_alabaster_returns_its_spirit_after_dying() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let angel = named_creature(&mut state, &reg, "Angel of Flight Alabaster", P0);
    // An actual Spirit card: the ability targets "target Spirit card in your
    // graveyard" and CR 608.2b re-checks that on resolution, so a synthetic
    // creature with no subtypes would fizzle for an unrelated reason.
    let spirit = named_card_in_graveyard(&mut state, &reg, "Chapel Geist", P0);

    resolve_after_source_dies_targeting(&mut state, &reg, angel, TriggerEvent::Upkeep,
        vec![Target::Object(spirit)]);

    assert_eq!(state.get_object(spirit).unwrap().zone, Zone::Hand,
        "CR 113.7a: the Angel dying does not counter its own upkeep trigger");
}

// Charmbreaker Devils: "At the beginning of your upkeep, return an instant or
// sorcery card at random from your graveyard to your hand."
#[test]
fn charmbreaker_devils_returns_a_spell_after_dying() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let devils = named_creature(&mut state, &reg, "Charmbreaker Devils", P0);
    let instant = named_card_in_graveyard(&mut state, &reg, "Think Twice", P0);

    resolve_after_source_dies(&mut state, &reg, devils, TriggerEvent::Upkeep);

    assert_eq!(state.get_object(instant).unwrap().zone, Zone::Hand,
        "CR 113.7a: the Devils dying does not counter their own upkeep trigger");
}

// Geist of Saint Traft: "Whenever Geist of Saint Traft attacks, create a 4/4
// white Angel creature token with flying that's tapped and attacking."
#[test]
fn geist_of_saint_traft_makes_its_angel_after_dying() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let geist = named_creature(&mut state, &reg, "Geist of Saint Traft", P0);

    resolve_after_source_dies(&mut state, &reg, geist,
        TriggerEvent::Attacks { attacker: geist, defending_player: P1 });

    assert_eq!(count_tokens_named(&state, "Angel"), 1,
        "CR 113.7a: killing the Geist with its attack trigger on the stack \
         still leaves the Angel");
}

// Kessig Cagebreakers: "Whenever Kessig Cagebreakers attacks, create a 2/2
// green Wolf creature token that's tapped and attacking for each creature card
// in your graveyard." Counted on resolution, so the Cagebreakers themselves
// count — three Wolves, not two.
#[test]
fn kessig_cagebreakers_counts_itself_among_the_dead() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let cb = named_creature(&mut state, &reg, "Kessig Cagebreakers", P0);
    for pt in [2, 3] {
        let c = ready_creature(&mut state, P0, pt, pt);
        state.move_object(c, Zone::Graveyard, &reg);
    }

    resolve_after_source_dies(&mut state, &reg, cb,
        TriggerEvent::Attacks { attacker: cb, defending_player: P1 });

    assert_eq!(count_tokens_named(&state, "Wolf"), 3,
        "CR 113.7a/608.2: the count happens on resolution, by which time the \
         Cagebreakers are themselves a creature card in the graveyard");
}

// Endless Ranks of the Dead: "At the beginning of your upkeep, create X 2/2
// black Zombie creature tokens, where X is half the number of Zombies you
// control, rounded down."
#[test]
fn endless_ranks_of_the_dead_makes_its_zombies_after_being_destroyed() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let ranks = named_creature(&mut state, &reg, "Endless Ranks of the Dead", P0);
    for _ in 0..4 {
        let z = ready_creature(&mut state, P0, 2, 2);
        let obj = state.get_object_mut(z).unwrap();
        obj.is_token = true;
        obj.subtypes = vec!["Zombie".into()];
        obj.name = "Zombie".into();
    }
    assert_eq!(count_tokens_named(&state, "Zombie"), 4, "test setup");

    resolve_after_source_dies(&mut state, &reg, ranks, TriggerEvent::Upkeep);

    assert_eq!(count_tokens_named(&state, "Zombie"), 6,
        "CR 113.7a: four Zombies makes two more, even though the enchantment \
         that counted them is gone");
}

// Splinterfright: "At the beginning of your upkeep, mill two cards."
#[test]
fn splinterfright_mills_after_dying() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let splinter = named_creature(&mut state, &reg, "Splinterfright", P0);
    let filler = reg.get_id_by_name("Forest").unwrap();
    for _ in 0..5 {
        let id = state.create_object(filler, P0, Zone::Library, None, None);
        state.get_player_mut(P0).library_order.push(id);
    }
    let before = state.get_player(P0).library_order.len();

    resolve_after_source_dies(&mut state, &reg, splinter, TriggerEvent::Upkeep);

    assert_eq!(before - state.get_player(P0).library_order.len(), 2,
        "CR 113.7a: the mill still happens after Splinterfright is destroyed");
}

// Undead Alchemist: "Whenever a creature card is put into an opponent's
// graveyard from their library, exile that card and create a 2/2 black Zombie
// creature token."
#[test]
fn undead_alchemist_exiles_and_makes_its_zombie_after_dying() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let alchemist = named_creature(&mut state, &reg, "Undead Alchemist", P0);
    let milled = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P1);

    resolve_after_source_dies(&mut state, &reg, alchemist,
        TriggerEvent::CreatureCardMilled { milled_object: milled, milled_player: P1 });

    assert_eq!(state.get_object(milled).unwrap().zone, Zone::Exile,
        "CR 113.7a: the exile still happens after the Alchemist is destroyed");
    assert_eq!(count_tokens_named(&state, "Zombie"), 1, "and so does the token");
}

// Mentor of the Meek: "Whenever a creature with power 2 or less enters the
// battlefield under your control, you may pay {1}. If you do, draw a card."
#[test]
fn mentor_of_the_meek_offers_its_payment_after_dying() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mentor = named_creature(&mut state, &reg, "Mentor of the Meek", P0);
    let small = ready_creature(&mut state, P0, 1, 1);

    resolve_after_source_dies(&mut state, &reg, mentor,
        TriggerEvent::CreatureEntered { entered: small, entered_controller: P0 });

    assert!(state.awaiting_action.is_some(),
        "CR 113.7a: destroying the Mentor still presents the 'you may pay {{1}}' choice");
}

// Trepanation Blade: "Whenever equipped creature attacks, defending player
// reveals cards from the top of their library until they reveal a land card."
// An Equipment's trigger is as independent of the Equipment as a creature's is
// of the creature.
#[test]
fn trepanation_blade_mills_after_the_equipment_is_destroyed() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);
    let blade = named_equipment(&mut state, &reg, "Trepanation Blade", P0);
    state.get_object_mut(blade).unwrap().attached_to = Some(creature);

    let filler = reg.get_id_by_name("Walking Corpse").unwrap();
    for _ in 0..2 {
        let id = state.create_object(filler, P1, Zone::Library, Some(2), Some(2));
        state.get_player_mut(P1).library_order.push(id);
    }
    let land_card = reg.get_id_by_name("Forest").unwrap();
    let land = state.create_object(land_card, P1, Zone::Library, None, None);
    state.get_player_mut(P1).library_order.push(land);
    let before = state.get_player(P1).library_order.len();

    resolve_after_source_dies(&mut state, &reg, blade,
        TriggerEvent::Attacks { attacker: blade, defending_player: P1 });

    assert!(state.get_player(P1).library_order.len() < before,
        "CR 113.7a: the reveal still happens after the Blade is destroyed");
}

// Sturmgeist: "Whenever Sturmgeist deals combat damage to a player, draw a
// card." Reached through the dispatcher, not by calling the hook — a card
// hook called directly cannot tell you whether trigger dispatch honours
// CR 113.7a, which is the whole subject of this file.
#[test]
fn sturmgeist_draws_after_dying() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let sturmgeist = named_creature(&mut state, &reg, "Sturmgeist", P0);
    let card_id = reg.get_id_by_name("Grizzly Bears").unwrap();
    let lib_card = state.create_object(card_id, P0, Zone::Library, Some(2), Some(2));
    state.get_player_mut(P0).library_order.push(lib_card);
    let before = state.objects_in_zone(Zone::Hand, P0).len();

    resolve_after_source_dies(&mut state, &reg, sturmgeist,
        TriggerEvent::CombatDamageToPlayer { damaged_player: P1, amount: 3 });

    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), before + 1,
        "CR 113.7a: the draw still happens after Sturmgeist is destroyed");
}

// ---------------------------------------------------------------------------
// Simultaneous death (CR 603.10)
// ---------------------------------------------------------------------------
//
// A death-watch trigger whose watcher died in the same event still fires: the
// game looks back in time, so the watcher was there when the creature died.

// Gutter Grime: "Whenever a nontoken creature you control dies, put a slime
// counter on Gutter Grime, then create a green Ooze creature token."
#[test]
fn gutter_grime_makes_its_ooze_after_dying_alongside_the_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_creature(&mut state, &reg, "Gutter Grime", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.move_object(creature, Zone::Graveyard, &reg);

    resolve_after_source_dies(&mut state, &reg, grime, TriggerEvent::CreatureDied {
        dead: DeadCreature { id: creature, controller: P0, damaged_by: vec![], toughness: 2, is_token: false },
    });

    assert_eq!(count_tokens_named(&state, "Ooze"), 1,
        "CR 603.10: a death-watch fires for a creature that died simultaneously \
         with the watcher");
}

// Murder of Crows: "Whenever another creature dies, you may draw a card. If you
// do, discard a card."
#[test]
fn murder_of_crows_offers_its_draw_after_dying_alongside_the_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let murder = named_creature(&mut state, &reg, "Murder of Crows", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    state.move_object(creature, Zone::Graveyard, &reg);

    resolve_after_source_dies(&mut state, &reg, murder, TriggerEvent::CreatureDied {
        dead: DeadCreature { id: creature, controller: P0, damaged_by: vec![], toughness: 2, is_token: false },
    });

    assert!(state.awaiting_action.is_some(),
        "CR 603.10: the draw choice is offered even though the Crows died too");
}

// ---------------------------------------------------------------------------
// The whole path, not just dispatch
// ---------------------------------------------------------------------------

/// The tests above stack the trigger by hand, which proves dispatch is right
/// but assumes the collector put one there. This one casts Armored Skaab for
/// real, lets the ETB trigger be collected, kills it, and only then processes
/// triggers — so a regression anywhere along cast → resolve → collect → process
/// shows up here.
#[test]
fn an_etb_trigger_collected_for_real_survives_its_source_dying() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    for _ in 0..10 {
        let card = state.create_object(
            registry.get_id_by_name("Grizzly Bears").unwrap(),
            P0, Zone::Library, Some(2), Some(2),
        );
        state.get_player_mut(P0).library_order.push(card);
    }
    let lib_before = state.get_player(P0).library_order.len();

    let skaab = castable_spell(&mut state, &registry, "Armored Skaab", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: skaab, targets: vec![], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &registry,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);
    assert_eq!(state.get_object(skaab).unwrap().zone, Zone::Battlefield,
        "test setup: the Skaab resolved onto the battlefield");

    state.move_object(skaab, Zone::Graveyard, &registry);
    mtg_engine::triggers::process_triggers(&mut state, &registry);

    assert_eq!(lib_before - state.get_player(P0).library_order.len(), 4,
        "CR 113.7a: the Skaab's ETB mill happens even though it is already dead");
}
