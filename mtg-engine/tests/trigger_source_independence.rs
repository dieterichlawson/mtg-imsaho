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
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
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
    let card_id = state.get_object(source).unwrap().card_id;
    let controller = state.get_object(source).unwrap().controller;
    state.stack.push(StackEntry::Trigger(PendingTrigger::new(
        TriggerSource::new(source, card_id, controller, ""),
        event,
    )));
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

    let card_id = state.get_object(vengeance).unwrap().card_id;
    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource {
            chosen_targets: vec![mtg_engine::actions::Target::Player(P1)],
            ..TriggerSource::new(vengeance, card_id, P0, "")
        },
        event: TriggerEvent::SpellCast { caster: P0, spell_id: spell },
    }));
    state.move_object(vengeance, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

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
// -------------------------------------------------------------------------

// CR 113.7a: A triggered ability on the stack is independent of its source.
// Removing the source after the trigger is on the stack does not counter it.
// The engine gates trigger resolution on `source.zone == Battlefield`,
// which incorrectly prevents resolution when the source has left.

// Angel of Flight Alabaster: "At the beginning of your upkeep, return target
// Spirit card from your graveyard to your hand."
// Destroying the Angel after the trigger stacks should not prevent the return.
#[test]
fn test_angel_of_flight_alabaster_trigger_resolves_after_death() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let angel = named_creature(&mut state, &reg, "Angel of Flight Alabaster", P0);
    let angel_card = reg.get_id_by_name("Angel of Flight Alabaster").unwrap();

    // An actual Spirit card: the Angel's ability targets "target Spirit card
    // in your graveyard", and CR 608.2b re-checks that on resolution. A
    // synthetic creature with no subtypes is not a legal target, so the
    // trigger would rightly fizzle for a reason unrelated to this test.
    let spirit = named_card_in_graveyard(&mut state, &reg, "Chapel Geist", P0);

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource { chosen_targets: vec![Target::Object(spirit)], ..TriggerSource::new(angel, angel_card, P0, "Angel of Flight Alabaster") },
        event: TriggerEvent::Upkeep,
    }));

    state.move_object(angel, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        state.get_object(spirit).unwrap().zone,
        Zone::Hand,
        "CR 113.7a: Angel upkeep trigger should return Spirit to hand even after Angel is destroyed"
    );
}

// Charmbreaker Devils: "At the beginning of your upkeep, return an instant or
// sorcery card at random from your graveyard to your hand."
// Destroying the Devils after the trigger stacks should not prevent the return.
#[test]
fn test_charmbreaker_devils_trigger_resolves_after_death() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let devils = named_creature(&mut state, &reg, "Charmbreaker Devils", P0);
    let devils_card = reg.get_id_by_name("Charmbreaker Devils").unwrap();

    let instant = named_card_in_graveyard(&mut state, &reg, "Think Twice", P0);
    state.get_object_mut(instant).unwrap().card_types = vec![CardType::Instant];

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource::new(devils, devils_card, P0, "Charmbreaker Devils"),
        event: TriggerEvent::Upkeep,
    }));

    state.move_object(devils, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        state.get_object(instant).unwrap().zone,
        Zone::Hand,
        "CR 113.7a: Devils upkeep trigger should return instant to hand even after Devils are destroyed"
    );
}

// Geist of Saint Traft: "Whenever Geist of Saint Traft attacks, create a 4/4
// white Angel creature token with flying that's tapped and attacking."
// Destroying the Geist after the attack trigger stacks should still create the token.
#[test]
fn test_geist_of_saint_traft_angel_token_created_after_death() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let geist = named_creature(&mut state, &reg, "Geist of Saint Traft", P0);
    let geist_card = reg.get_id_by_name("Geist of Saint Traft").unwrap();

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource::new(geist, geist_card, P0, "Geist of Saint Traft"),
        event: TriggerEvent::Attacks { attacker: geist, defending_player: P1 },
    }));

    state.move_object(geist, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        count_tokens_named(&state, "Angel"),
        1,
        "CR 113.7a: Geist attack trigger should create Angel token even after Geist is destroyed"
    );
}

// Kessig Cagebreakers: "Whenever Kessig Cagebreakers attacks, create a 2/2
// green Wolf creature token that's tapped and attacking for each creature
// card in your graveyard."
// After destroying Cagebreakers in response, the count includes Cagebreakers itself.
#[test]
fn test_kessig_cagebreakers_tokens_created_after_death() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let cb = named_creature(&mut state, &reg, "Kessig Cagebreakers", P0);
    let cb_card = reg.get_id_by_name("Kessig Cagebreakers").unwrap();

    let c1 = ready_creature(&mut state, P0, 2, 2);
    state.move_object(c1, Zone::Graveyard, &reg);
    let c2 = ready_creature(&mut state, P0, 3, 3);
    state.move_object(c2, Zone::Graveyard, &reg);

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource::new(cb, cb_card, P0, "Kessig Cagebreakers"),
        event: TriggerEvent::Attacks { attacker: cb, defending_player: P1 },
    }));

    state.move_object(cb, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        count_tokens_named(&state, "Wolf"),
        3,
        "CR 113.7a: Cagebreakers should create 3 Wolf tokens even after death (2 original + itself)"
    );
}

// Splinterfright: "At the beginning of your upkeep, mill two cards."
// Destroying Splinterfright after the trigger stacks should not prevent the mill.
#[test]
fn test_splinterfright_mill_resolves_after_death() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let splinter = named_creature(&mut state, &reg, "Splinterfright", P0);
    let splinter_card = reg.get_id_by_name("Splinterfright").unwrap();

    let filler = reg.get_id_by_name("Forest").unwrap();
    for _ in 0..5 {
        let id = state.create_object(filler, P0, Zone::Library, None, None);
        state.players[0].library_order.push(id);
    }
    let lib_before = state.players[0].library_order.len();

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource::new(splinter, splinter_card, P0, "Splinterfright"),
        event: TriggerEvent::Upkeep,
    }));

    state.move_object(splinter, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    let lib_after = state.players[0].library_order.len();
    assert_eq!(
        lib_before - lib_after,
        2,
        "CR 113.7a: Splinterfright upkeep trigger should mill 2 even after destruction"
    );
}

// Undead Alchemist: "Whenever a creature card is put into an opponent's graveyard
// from their library, exile that card and create a 2/2 black Zombie creature token."
// Destroying the Alchemist after the trigger stacks should not prevent exile + token.
#[test]
fn test_undead_alchemist_watch_resolves_after_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let alchemist = named_creature(&mut state, &reg, "Undead Alchemist", P0);
    let alchemist_card = reg.get_id_by_name("Undead Alchemist").unwrap();

    let milled = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P1);

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource::new(alchemist, alchemist_card, P0, "Undead Alchemist"),
        event: TriggerEvent::CreatureCardMilled { milled_object: milled, milled_player: P1 },
    }));

    state.move_object(alchemist, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        state.get_object(milled).unwrap().zone,
        Zone::Exile,
        "CR 113.7a: Alchemist trigger should exile milled creature even after Alchemist is destroyed"
    );
    assert_eq!(
        count_tokens_named(&state, "Zombie"),
        1,
        "CR 113.7a: Alchemist trigger should create Zombie token even after Alchemist is destroyed"
    );
}

// Gutter Grime: "Whenever a nontoken creature you control dies, put a slime
// counter on Gutter Grime, then create a green Ooze creature token."
// When Gutter Grime and a creature are destroyed simultaneously, the death
// trigger should still fire per CR 603.10.
#[test]
fn test_gutter_grime_creates_token_when_ltb() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_creature(&mut state, &reg, "Gutter Grime", P0);
    let grime_card = reg.get_id_by_name("Gutter Grime").unwrap();
    let creature = ready_creature(&mut state, P0, 2, 2);

    state.move_object(grime, Zone::Graveyard, &reg);
    state.move_object(creature, Zone::Graveyard, &reg);

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource::new(grime, grime_card, P0, "Gutter Grime"),
        event: TriggerEvent::CreatureDied { dead: DeadCreature { id: creature, controller: P0, damaged_by: vec![], toughness: 2, is_token: false } },
    }));

    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        count_tokens_named(&state, "Ooze"),
        1,
        "CR 603.10: Gutter Grime death-watch should create Ooze token after simultaneous destruction"
    );
}

// Murder of Crows: "Whenever another creature dies, you may draw a card.
// If you do, discard a card."
// When Murder of Crows and another creature die simultaneously, the death
// trigger should still present the draw choice per CR 603.10.
#[test]
fn test_murder_of_crows_trigger_resolves_after_simultaneous_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let murder = named_creature(&mut state, &reg, "Murder of Crows", P0);
    let murder_card = reg.get_id_by_name("Murder of Crows").unwrap();
    let creature = ready_creature(&mut state, P0, 2, 2);

    state.move_object(murder, Zone::Graveyard, &reg);
    state.move_object(creature, Zone::Graveyard, &reg);

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource::new(murder, murder_card, P0, "Murder of Crows"),
        event: TriggerEvent::CreatureDied { dead: DeadCreature { id: creature, controller: P0, damaged_by: vec![], toughness: 2, is_token: false } },
    }));

    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        state.awaiting_action.is_some(),
        true,
        "CR 603.10: Murder of Crows trigger should present draw choice even after simultaneous death"
    );
}

// Mentor of the Meek: "Whenever a creature with power 2 or less enters the
// battlefield under your control, you may pay {1}. If you do, draw a card."
// Destroying Mentor after the trigger stacks should not prevent the pay choice.
#[test]
fn test_mentor_of_the_meek_trigger_resolves_after_removal() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mentor = named_creature(&mut state, &reg, "Mentor of the Meek", P0);
    let mentor_card = reg.get_id_by_name("Mentor of the Meek").unwrap();
    let small_creature = ready_creature(&mut state, P0, 1, 1);

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource::new(mentor, mentor_card, P0, "Mentor of the Meek"),
        event: TriggerEvent::CreatureEntered { entered: small_creature, entered_controller: P0 },
    }));

    state.move_object(mentor, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(
        state.awaiting_action.is_some(),
        true,
        "CR 113.7a: Mentor trigger should present pay choice even after Mentor is destroyed"
    );
}

// Trepanation Blade: "Whenever equipped creature attacks, defending player
// reveals cards from the top of their library until they reveal a land card.
// That creature gets +1/+0 until end of turn for each card revealed this way."
// Destroying the Blade after the trigger stacks should not prevent the mill + pump.
#[test]
fn test_trepanation_blade_trigger_resolves_after_equipment_destroyed() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);
    let blade = named_equipment(&mut state, &reg, "Trepanation Blade", P0);
    let blade_card = reg.get_id_by_name("Trepanation Blade").unwrap();
    state.get_object_mut(blade).unwrap().attached_to = Some(creature);

    let filler = reg.get_id_by_name("Walking Corpse").unwrap();
    for _ in 0..2 {
        let id = state.create_object(filler, P1, Zone::Library, Some(2), Some(2));
        state.get_object_mut(id).unwrap().card_types = vec![CardType::Creature];
        state.players[1].library_order.push(id);
    }
    let land_card = reg.get_id_by_name("Forest").unwrap();
    let land = state.create_object(land_card, P1, Zone::Library, None, None);
    state.get_object_mut(land).unwrap().card_types = vec![CardType::Land];
    state.players[1].library_order.push(land);

    let lib_before = state.players[1].library_order.len();

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource::new(blade, blade_card, P0, "Trepanation Blade"),
        event: TriggerEvent::Attacks { attacker: blade, defending_player: P1 },
    }));

    state.move_object(blade, Zone::Graveyard, &reg);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    let lib_after = state.players[1].library_order.len();
    assert_ne!(
        lib_before, lib_after,
        "CR 113.7a: Trepanation Blade trigger should mill even after equipment is destroyed"
    );
}
