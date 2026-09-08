//! Self-tests for the event-window invariants (`mtg_engine::invariants`'s
//! `events` family): what the events of the current action say the state
//! must look like at the decision point that follows.
//!
//! Same contract as `invariant_checker.rs` and `invariant_families.rs` —
//! the checker is the fuzzer's only pair of eyes, so every clause needs a
//! state that violates it and, where the clause is conditional, a
//! neighbouring state that does not.

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::{DamageTarget, GameEvent, LossReason};
use mtg_engine::ids::{ObjectId, PlayerId};
use mtg_engine::invariants::check_core;
use mtg_engine::state::StackEntry;
use mtg_engine::types::*;
use mtg_engine::types::{ContinuousEffect, EffectScope};

fn base() -> (GameState, CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.turn_number = 3;
    (state, reg)
}

/// A hand-built fixture never ran the trigger collector; the game loop
/// checks a state only after it has.
fn as_collected(state: &GameState) -> GameState {
    let mut s = state.clone();
    s.trigger_event_index = s.events.len();
    s
}

#[track_caller]
fn clean(state: &GameState, reg: &CardRegistry) {
    assert_eq!(check_core(&as_collected(state), reg), Vec::<String>::new());
}

#[track_caller]
fn flags(state: &GameState, reg: &CardRegistry, needle: &str) {
    let v = check_core(state, reg);
    assert!(v.iter().any(|m| m.contains(needle)),
        "expected a violation containing {needle:?}, got: {v:?}");
}

#[track_caller]
fn quiet_about(state: &GameState, reg: &CardRegistry, needle: &str) {
    let v = check_core(state, reg);
    assert!(!v.iter().any(|m| m.contains(needle)),
        "expected no violation containing {needle:?}, got: {v:?}");
}

/// Every event that names a player is range-checked before anything reads
/// through the id. The check is one match with an arm per event shape, and
/// an arm that goes missing takes its events out of the check silently.
#[test]
fn every_event_that_names_a_player_is_range_checked() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let card_id = state.get_object(bear).unwrap().card_id;
    let g = PlayerId(u8::try_from(state.players.len()).unwrap());

    let events = [
        GameEvent::TurnStarted { player: g, turn: 3 },
        GameEvent::CardDrawn { player: g, object: bear },
        GameEvent::LandPlayed { player: g, object: bear },
        GameEvent::SpellCast { player: g, object: bear },
        GameEvent::ManaAdded { player: g, mana_type: ManaType::Green, amount: 1 },
        GameEvent::ManaPoolEmptied { player: g },
        GameEvent::LifeChanged { player: g, old: 20, new_life: 19 },
        GameEvent::PlayerLost { player: g, reason: LossReason::Conceded },
        GameEvent::PriorityPassed { player: g },
        GameEvent::Discarded { player: g, object: bear },
        GameEvent::LibraryShuffled { player: g },
        GameEvent::EnteredBattlefield { object: bear, controller: g },
        GameEvent::CreatureDied { object: bear, name: "Grizzly Bears".into(), card_id, controller: g, damaged_by: vec![],
                                  last_known_toughness: 2, is_token: false, subtypes: vec![] },
        GameEvent::LeftBattlefield { object: bear, to: Zone::Graveyard, last_controller: g },
        GameEvent::CreatureCardMilled { object: bear, milled_player: g },
        GameEvent::CombatDamageDealt { source: bear, target: DamageTarget::Player(g), amount: 1 },
        GameEvent::NonCombatDamageDealt { source: bear, target: DamageTarget::Player(g), amount: 1 },
    ];

    for e in events {
        let mut s = state.clone();
        s.events = vec![e.clone()];
        let v = check_core(&s, &reg);
        assert!(v.iter().any(|m| m.contains("names p2 who is not a player")),
            "{e:?} names a seat that does not exist, got: {v:?}");
    }

    // The same events about a real seat say nothing about players.
    let mut s = state.clone();
    s.events = vec![GameEvent::PriorityPassed { player: P1 }];
    quiet_about(&s, &reg, "who is not a player");
}

/// CR 112.1/601.2i/305.9: what a `SpellCast` event says about the object it
/// names — one cast per object per action, on the stack, under the caster,
/// and not a land.
#[test]
fn a_cast_event_describes_a_spell_on_the_stack() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let pump = castable_spell(&mut state, &reg, "Moment of Heroism", P0);
    state.priority_player = Some(P0);
    let cast = cast_onto_stack(&state, &reg, pump, vec![Target::Object(bear)]);
    clean(&cast, &reg);

    // CR 601.2i: one announcement per cast.
    let mut s = cast.clone();
    s.events.push(GameEvent::SpellCast { player: P0, object: pump });
    flags(&s, &reg, "twice in one action (CR 601.2i)");

    // An event about nothing.
    let mut s = cast.clone();
    s.events = vec![GameEvent::SpellCast { player: P0, object: ObjectId(4242) }];
    flags(&s, &reg, "no such object");

    // CR 112.1/112.2: the spell is on the stack, under the player who cast it.
    let mut s = cast.clone();
    s.get_object_mut(pump).unwrap().controller = P1;
    flags(&s, &reg, "(CR 112.1/112.2)");

    // CR 305.9: a land is played, never cast.
    let mut s = cast.clone();
    s.get_object_mut(pump).unwrap().card_types = vec![CardType::Land];
    flags(&s, &reg, "but it is a land (CR 305.9)");

    // CR 608.2b: a target that stopped being legal is marked, never announced.
    let mut s = cast.clone();
    s.get_object_mut(pump).unwrap().targets = vec![Target::Illegal];
    flags(&s, &reg, "carries an Illegal target");

    // CR 702.16b: a spell cannot target what has protection from it.
    let mut s = cast.clone();
    if let Some(sub) = s.subtypes_of(pump, &reg).first().cloned() {
        s.get_object_mut(bear).unwrap().instance_continuous_effects = Some(vec![
            ContinuousEffect::ProtectionFromSubtype { subtype: sub, scope: EffectScope::OnSelf },
        ]);
        flags(&s, &reg, "which has protection from it (CR 702.16b)");
    }

    // CR 608.2n: a resolved spell is off the stack.
    let mut s = cast.clone();
    s.events = vec![GameEvent::SpellResolved { object: pump }];
    flags(&s, &reg, "which is still on the stack (CR 608.2n)");
    s.stack.clear();
    quiet_about(&s, &reg, "(CR 608.2n)");
}

/// CR 702.11b/702.11c: the other half of the cast clause — what the spell
/// was allowed to point at. This is the checker's only look at target
/// legality after the fact, so a clause that goes quiet here takes a whole
/// class of illegal cast out of ~110k fuzzed games a night while the run
/// stays green.
///
/// Both halves matter and only one of them is obvious: hexproof stops a
/// spell an *opponent* controls (CR 702.11b), so an ordinary removal spell
/// pointed at an opponent's ordinary creature — the commonest legal cast in
/// the game — has to pass in silence.
#[test]
fn a_cast_events_targets_are_ones_the_spell_could_have_chosen() {
    let (mut state, reg) = base();
    let theirs = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let pump = castable_spell(&mut state, &reg, "Moment of Heroism", P0);
    state.priority_player = Some(P0);
    let cast = cast_onto_stack(&state, &reg, pump, vec![Target::Object(theirs)]);

    // An opponent's ordinary creature is a legal target and says nothing.
    quiet_about(&cast, &reg, "hexproof");
    quiet_about(&cast, &reg, "protection from it");

    // CR 702.11b: hexproof stops a spell an opponent controls.
    let mut s = cast.clone();
    grant_keyword(&mut s, theirs, Keyword::Hexproof);
    flags(&s, &reg, "which has hexproof from p0 (CR 702.11b)");

    // And only an opponent's: your own hexproof creature is a legal target
    // for your own spell, which is what the controller half of the test is.
    let mut s = cast.clone();
    s.get_object_mut(pump).unwrap().targets = vec![Target::Object(mine)];
    grant_keyword(&mut s, mine, Keyword::Hexproof);
    quiet_about(&s, &reg, "hexproof");
}

/// CR 702.11c: the same rule for a player. Witchbane Orb is the one card in
/// this pool that grants it, and the clause is worth its own case because
/// it is the only place the checker asks whether a *player* could have been
/// targeted at all.
#[test]
fn a_cast_event_may_not_target_a_player_with_hexproof() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let pump = castable_spell(&mut state, &reg, "Moment of Heroism", P0);
    state.priority_player = Some(P0);
    let cast = cast_onto_stack(&state, &reg, pump, vec![Target::Object(bear)]);

    // Targeting an opponent an Orb protects.
    let mut s = cast.clone();
    named_permanent(&mut s, &reg, "Witchbane Orb", P1);
    s.get_object_mut(pump).unwrap().targets = vec![Target::Player(P1)];
    flags(&s, &reg, "targets p1 who has hexproof (CR 702.11c)");

    // Your own Orb does not stop you targeting yourself: hexproof is about
    // spells your OPPONENTS control.
    let mut s = cast.clone();
    named_permanent(&mut s, &reg, "Witchbane Orb", P0);
    s.get_object_mut(pump).unwrap().targets = vec![Target::Player(P0)];
    quiet_about(&s, &reg, "who has hexproof");

    // And an Orb on the table stops nothing when no player is targeted.
    let mut s = cast.clone();
    named_permanent(&mut s, &reg, "Witchbane Orb", P1);
    quiet_about(&s, &reg, "who has hexproof");
}

/// CR 305.1/305.2: what a `LandPlayed` event says — one land, on your own
/// main phase with an empty stack, arriving unattached, with its
/// `EnteredBattlefield`.
#[test]
fn a_land_played_event_describes_a_land_that_arrived() {
    let (mut state, reg) = base();
    state.priority_player = Some(P0);
    let land = named_permanent(&mut state, &reg, "Forest", P0);
    state.get_player_mut(P0).land_plays_remaining = 0;
    state.events = vec![
        GameEvent::EnteredBattlefield { object: land, controller: P0 },
        GameEvent::LandPlayed { player: P0, object: land },
    ];
    clean(&state, &reg);

    // Two land drops in one action.
    let mut s = state.clone();
    let second = named_permanent(&mut s, &reg, "Forest", P0);
    s.events.push(GameEvent::EnteredBattlefield { object: second, controller: P0 });
    s.events.push(GameEvent::LandPlayed { player: P0, object: second });
    flags(&s, &reg, "2 lands played in one action (CR 305.2)");

    // The land is not on the battlefield after all.
    let mut s = state.clone();
    s.get_object_mut(land).unwrap().zone = Zone::Graveyard;
    flags(&s, &reg, "but no such land of p0 on the battlefield");

    // Or it is there but is not that player's.
    let mut s = state.clone();
    s.get_object_mut(land).unwrap().owner = P1;
    s.get_object_mut(land).unwrap().controller = P1;
    flags(&s, &reg, "but no such land of p0 on the battlefield");

    // It arrived attached to something.
    let mut s = state.clone();
    let host = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    s.get_object_mut(land).unwrap().attached_to = Some(host);
    flags(&s, &reg, "arrived attached");

    // Playing a land is a zone change, and it is reported as one.
    let mut s = state.clone();
    s.events.retain(|e| !matches!(e, GameEvent::EnteredBattlefield { .. }));
    flags(&s, &reg, "without its EnteredBattlefield");

    // CR 305.1: only with an empty stack.
    let mut s = state.clone();
    let spell = castable_spell(&mut s, &reg, "Moment of Heroism", P0);
    s.get_object_mut(spell).unwrap().zone = Zone::Stack;
    s.stack.push(StackEntry::Spell(spell));
    flags(&s, &reg, "with a spell or ability on the stack (CR 305.1)");

    // CR 117.3c: the player who played it keeps priority.
    let mut s = state.clone();
    s.priority_player = Some(P1);
    flags(&s, &reg, "played a land but priority is");

    // Unless nobody holds priority because the action raised a prompt: the
    // rule is about the decision point after the action, and there is not
    // one yet.
    let mut s = state.clone();
    s.priority_player = None;
    s.awaiting_action = Some(mtg_engine::state::AwaitingAction::DiscardToHandSize {
        player: P0, discard_count: 1 });
    quiet_about(&s, &reg, "played a land but priority is");
}

/// CR 509.1a/509.1b/509.1h: what a `BlockersDeclared` event says — each
/// blocker declared once, against a real attacker, by an untapped creature
/// the defending player controls, and combat's own maps agreeing.
#[test]
fn a_blockers_declared_event_describes_a_legal_block() {
    let (mut state, reg) = base();
    let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let blocker = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.step = Step::DeclareAttackers;
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    state.step = Step::DeclareBlockers;
    submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);
    state.priority_player = Some(P0);
    clean(&state, &reg);

    // A declaration with no combat at all.
    let mut s = state.clone();
    s.combat = None;
    flags(&s, &reg, "BlockersDeclared but no combat state");

    // CR 509.1b: a creature blocks one attacker.
    let mut s = state.clone();
    let second = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    s.combat.as_mut().unwrap().attackers.insert(second, P1);
    s.events = vec![GameEvent::BlockersDeclared {
        assignments: vec![(blocker, attacker), (blocker, second)] }];
    flags(&s, &reg, "blocker declared twice (CR 509.1b)");

    // A block against something that is not attacking.
    let mut s = state.clone();
    let bystander = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    s.events = vec![GameEvent::BlockersDeclared { assignments: vec![(blocker, bystander)] }];
    flags(&s, &reg, "not an attacker");
    // Unless it left the battlefield in the same action: combat has
    // forgotten it (CR 506.4) and the declaration is the record of what
    // happened before it did.
    let mut s = state.clone();
    let bystander = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    s.get_object_mut(bystander).unwrap().zone = Zone::Graveyard;
    s.events = vec![
        GameEvent::BlockersDeclared { assignments: vec![(blocker, bystander)] },
        GameEvent::LeftBattlefield { object: bystander, to: Zone::Graveyard, last_controller: P0 },
    ];
    quiet_about(&s, &reg, "not an attacker");

    // CR 509.1a: the blocker is a creature the defending player controls.
    let mut s = state.clone();
    s.get_object_mut(blocker).unwrap().controller = P0;
    flags(&s, &reg, "blocker is not a creature the defending player controls (CR 509.1a)");

    // Combat's maps and the declaration are the same block, both ways.
    let mut s = state.clone();
    s.events = vec![GameEvent::BlockersDeclared { assignments: vec![] }];
    flags(&s, &reg, "is in combat but was not declared");
    flags(&s, &reg, "is marked blocked but no block was declared for it");
}

/// CR 702.111b: menace needs two blockers — and two is enough.
#[test]
fn menace_is_satisfied_by_exactly_two_blockers() {
    let reg = registry();

    let blocked_by = |n: usize| {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        state.turn_number = 3;
        let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
        let blockers: Vec<ObjectId> = (0..n)
            .map(|_| named_permanent(&mut state, &reg, "Grizzly Bears", P1))
            .collect();
        state.step = Step::DeclareAttackers;
        submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
        state.step = Step::DeclareBlockers;
        let pairs: Vec<(ObjectId, ObjectId)> = blockers.iter().map(|b| (*b, attacker)).collect();
        submit_declare_blockers(&mut state, P1, &pairs, &reg);
        state.priority_player = Some(P0);
        // Granted after the declaration: the engine would refuse to declare
        // an illegal block, and the clause exists to catch a block that
        // became illegal some other way.
        grant_keyword(&mut state, attacker, Keyword::Menace);
        state
    };

    quiet_about(&blocked_by(2), &reg, "has menace but was blocked");
    flags(&blocked_by(1), &reg, "has menace but was blocked by 1 creature (CR 702.111b)");
}

/// CR 120.1a/120.3a/506.2/510.1: what a damage event says about where the
/// damage could have come from and gone to.
#[test]
fn a_damage_event_describes_damage_that_could_have_happened() {
    let (mut state, reg) = base();
    let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let blocker = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let land = named_permanent(&mut state, &reg, "Forest", P1);
    state.step = Step::CombatDamage;
    let mut c = mtg_engine::state::CombatState::new();
    c.any_attackers_declared = true;
    c.attackers.insert(attacker, P1);
    c.blocker_assignments.insert(attacker, vec![blocker]);
    c.blocked_attackers.insert(attacker);
    state.combat = Some(c);
    state.get_object_mut(blocker).unwrap().damage_marked = 1;
    state.get_object_mut(blocker).unwrap().damaged_by.push(attacker);
    state.events = vec![GameEvent::CombatDamageDealt {
        source: attacker, target: DamageTarget::Object(blocker), amount: 1 }];
    clean(&state, &reg);

    // CR 120.1a: damage lands on creatures and planeswalkers.
    let mut s = state.clone();
    s.events = vec![GameEvent::CombatDamageDealt {
        source: attacker, target: DamageTarget::Object(land), amount: 1 }];
    flags(&s, &reg, "is neither creature nor planeswalker (CR 120.1a)");

    // CR 510.1c: a blocked attacker hits what is blocking it.
    let mut s = state.clone();
    let other = named_permanent(&mut s, &reg, "Grizzly Bears", P1);
    s.get_object_mut(other).unwrap().damage_marked = 1;
    s.get_object_mut(other).unwrap().damaged_by.push(attacker);
    s.events = vec![GameEvent::CombatDamageDealt {
        source: attacker, target: DamageTarget::Object(other), amount: 1 }];
    flags(&s, &reg, "which is not blocking it (CR 510.1c)");

    // CR 510.1b: an unblocked attacker hits the player it is attacking, and
    // nothing else.
    let mut unblocked = state.clone();
    unblocked.combat.as_mut().unwrap().blocker_assignments.clear();
    unblocked.combat.as_mut().unwrap().blocked_attackers.clear();
    let mut s = unblocked.clone();
    s.events = vec![GameEvent::CombatDamageDealt {
        source: attacker, target: DamageTarget::Object(blocker), amount: 1 }];
    flags(&s, &reg, "which it is not attacking (CR 510.1b)");

    let mut s = unblocked.clone();
    s.get_player_mut(P0).life = 18;
    s.events = vec![GameEvent::LifeChanged { player: P0, old: 20, new_life: 18 },
                    GameEvent::CombatDamageDealt {
                        source: attacker, target: DamageTarget::Player(P0), amount: 2 }];
    flags(&s, &reg, "unblocked attacker hit p0 (CR 510.1b)");
    flags(&s, &reg, "combat damage to p0 who is not the defending player (CR 506.2)");

    // The source of combat damage exists, or died dealing it.
    let mut s = state.clone();
    s.objects.remove(&attacker);
    flags(&s, &reg, "the source neither exists nor died");

    // CR 702.4b: only double strike deals damage in both steps.
    let mut s = state.clone();
    s.combat.as_mut().unwrap().dealt_first_strike.insert(attacker);
    flags(&s, &reg, "dealt regular damage after first-strike damage without double strike (CR 702.4b)");
    grant_keyword(&mut s, attacker, Keyword::DoubleStrike);
    quiet_about(&s, &reg, "(CR 702.4b)");

    // Noncombat damage is checked the same way, outside combat.
    let mut s = base().0;
    named_permanent(&mut s, &reg, "Grizzly Bears", P1);
    let source = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    s.get_player_mut(P1).life = 18;
    s.events = vec![GameEvent::NonCombatDamageDealt {
        source, target: DamageTarget::Player(P1), amount: 2 }];
    flags(&s, &reg, "no matching life loss for p1 (CR 120.3a)");
    s.events.insert(0, GameEvent::LifeChanged { player: P1, old: 20, new_life: 18 });
    quiet_about(&s, &reg, "no matching life loss");
}

/// CR 510.1b-d/510.4/702.15b: the rest of what a combat damage event has to
/// agree with — who may hit whom once blocks are in, which step a striker
/// deals in, and the life a lifelinker gains in the same breath.
#[test]
fn combat_damage_events_agree_with_the_blocks_and_the_step() {
    let (mut state, reg) = base();
    let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let blocker = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let bystander = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.step = Step::CombatDamage;
    let mut c = mtg_engine::state::CombatState::new();
    c.any_attackers_declared = true;
    c.attackers.insert(attacker, P1);
    c.blocker_assignments.insert(attacker, vec![blocker]);
    c.blocked_attackers.insert(attacker);
    state.combat = Some(c);

    // The ordinary shape: the attacker and its blocker trade, each hit
    // marked on the object that took it.
    let mut trade = state.clone();
    for (id, by) in [(blocker, attacker), (attacker, blocker)] {
        trade.get_object_mut(id).unwrap().damage_marked = 2;
        trade.get_object_mut(id).unwrap().damaged_by.push(by);
    }
    trade.events = vec![
        GameEvent::CombatDamageDealt { source: attacker, target: DamageTarget::Object(blocker), amount: 2 },
        GameEvent::CombatDamageDealt { source: blocker, target: DamageTarget::Object(attacker), amount: 2 },
    ];
    clean(&trade, &reg);

    // CR 510.1d: a blocker's damage goes to the attacker it is blocking.
    let mut s = trade.clone();
    s.get_object_mut(bystander).unwrap().damage_marked = 2;
    s.get_object_mut(bystander).unwrap().damaged_by.push(blocker);
    s.events = vec![GameEvent::CombatDamageDealt {
        source: blocker, target: DamageTarget::Object(bystander), amount: 2 }];
    flags(&s, &reg, &format!("a blocker of #{} hit something else (CR 510.1d)", attacker.0));

    // CR 510.1c: a blocked attacker reaches the player only with trample.
    let mut s = state.clone();
    s.get_player_mut(P1).life = 18;
    s.events = vec![GameEvent::LifeChanged { player: P1, old: 20, new_life: 18 },
                    GameEvent::CombatDamageDealt {
                        source: attacker, target: DamageTarget::Player(P1), amount: 2 }];
    flags(&s, &reg, "a blocked attacker without trample reached the player (CR 510.1c)");
    grant_keyword(&mut s, attacker, Keyword::Trample);
    quiet_about(&s, &reg, "without trample reached the player");

    // CR 510.1b: an unblocked attacker hitting the player it is attacking is
    // the ordinary case, and is not flagged.
    let mut unblocked = state.clone();
    unblocked.combat.as_mut().unwrap().blocker_assignments.clear();
    unblocked.combat.as_mut().unwrap().blocked_attackers.clear();
    let mut s = unblocked.clone();
    s.get_player_mut(P1).life = 18;
    s.events = vec![GameEvent::LifeChanged { player: P1, old: 20, new_life: 18 },
                    GameEvent::CombatDamageDealt {
                        source: attacker, target: DamageTarget::Player(P1), amount: 2 }];
    clean(&s, &reg);

    // CR 120.3a: the life loss is that player's, and is the damage dealt.
    let mut s = unblocked.clone();
    s.get_player_mut(P0).life = 18;
    s.events = vec![GameEvent::LifeChanged { player: P0, old: 20, new_life: 18 },
                    GameEvent::CombatDamageDealt {
                        source: attacker, target: DamageTarget::Player(P1), amount: 2 }];
    flags(&s, &reg, "no matching life loss for p1 (CR 120.3a)");
    let mut s = unblocked.clone();
    s.get_player_mut(P1).life = 19;
    s.events = vec![GameEvent::LifeChanged { player: P1, old: 20, new_life: 19 },
                    GameEvent::CombatDamageDealt {
                        source: attacker, target: DamageTarget::Player(P1), amount: 2 }];
    flags(&s, &reg, "no matching life loss for p1 (CR 120.3a)");

    // CR 702.15b: lifelink gains its controller that much life, in the same
    // window, after the damage.
    let mut s = unblocked.clone();
    grant_keyword(&mut s, attacker, Keyword::Lifelink);
    s.get_player_mut(P1).life = 18;
    s.events = vec![GameEvent::LifeChanged { player: P1, old: 20, new_life: 18 },
                    GameEvent::CombatDamageDealt {
                        source: attacker, target: DamageTarget::Player(P1), amount: 2 }];
    flags(&s, &reg, "lifelink but no life gain for its controller (CR 702.15b)");
    s.get_player_mut(P0).life = 22;
    s.events.push(GameEvent::LifeChanged { player: P0, old: 20, new_life: 22 });
    quiet_about(&s, &reg, "lifelink but no life gain");

    // CR 510.4: the first-strike step is for first and double strikers.
    let mut s = trade.clone();
    s.combat_damage_step_pending = true;
    s.events = vec![GameEvent::CombatDamageDealt {
        source: attacker, target: DamageTarget::Object(blocker), amount: 2 }];
    flags(&s, &reg, "dealt in the first-strike step without first strike (CR 510.4)");
    grant_keyword(&mut s, attacker, Keyword::FirstStrike);
    quiet_about(&s, &reg, "without first strike");
    let mut s2 = s.clone();
    s2.until_end_of_turn.clear();
    grant_keyword(&mut s2, attacker, Keyword::DoubleStrike);
    quiet_about(&s2, &reg, "without first strike");
}

/// CR 106.4/504.1: what an event window says about the step it sits in —
/// mana of a real size, one draw for the active player, and combat damage
/// only in a combat damage step with a combat.
#[test]
fn the_events_of_a_step_are_checked_against_the_step() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let other = named_permanent(&mut state, &reg, "Grizzly Bears", P1);

    // CR 106.4: mana is added in some amount; zero is not an amount.
    let mut s = state.clone();
    s.get_player_mut(P0).mana_pool.mana.insert(ManaType::Green, 1);
    s.events = vec![GameEvent::ManaAdded { player: P0, mana_type: ManaType::Green, amount: 1 }];
    quiet_about(&s, &reg, "ManaAdded of nothing");
    let mut s = state.clone();
    s.events = vec![GameEvent::ManaAdded { player: P0, mana_type: ManaType::Green, amount: 0 }];
    flags(&s, &reg, "ManaAdded of nothing");

    // CR 510.2: combat damage is dealt in a combat damage step, with a
    // combat — either half missing is the violation.
    let mut fighting = state.clone();
    fighting.step = Step::CombatDamage;
    let mut c = mtg_engine::state::CombatState::new();
    c.any_attackers_declared = true;
    c.attackers.insert(bear, P1);
    c.blocker_assignments.insert(bear, vec![]);
    fighting.combat = Some(c);
    fighting.get_player_mut(P1).life = 18;
    fighting.events = vec![
        GameEvent::LifeChanged { player: P1, old: 20, new_life: 18 },
        GameEvent::CombatDamageDealt { source: bear, target: DamageTarget::Player(P1), amount: 2 },
    ];
    quiet_about(&fighting, &reg, "(CR 510.2)");
    let mut s = fighting.clone();
    s.step = Step::PrecombatMain;
    flags(&s, &reg, "combat damage dealt in PrecombatMain (CR 510.2)");
    let mut s = fighting.clone();
    s.combat = None;
    flags(&s, &reg, "combat damage dealt in CombatDamage (CR 510.2)");
    let _ = other;
}

/// CR 701.9a/302.6: a discard is of that player's own card, and a tap is of
/// something still on the battlefield.
#[test]
fn a_discard_names_its_players_card_and_a_tap_a_permanent() {
    let (mut state, reg) = base();
    let mine = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    let mut s = state.clone();
    s.move_object(mine, Zone::Graveyard, &reg);
    s.events.push(GameEvent::Discarded { player: P0, object: mine });
    quiet_about(&s, &reg, "(CR 701.9a)");

    let mut s = state.clone();
    s.move_object(theirs, Zone::Graveyard, &reg);
    s.events.push(GameEvent::Discarded { player: P0, object: theirs });
    flags(&s, &reg, "not that player's card (CR 701.9a)");

    // A token is nobody's card to discard (CR 111.8).
    let mut s = state.clone();
    let token = s.create_token_with_subtypes("", P0, 2, 2, vec![Color::Green],
        vec![CardType::Creature], vec![], vec!["Wolf".into()], &reg)[0];
    s.events.push(GameEvent::Discarded { player: P0, object: token });
    flags(&s, &reg, "not that player's card (CR 701.9a)");

    // A permanent that was tapped in this window is still on the
    // battlefield at the end of it, unless something announced it leaving.
    let mut s = base().0;
    let bear = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    s.get_object_mut(bear).unwrap().tapped = true;
    s.events = vec![GameEvent::Tapped { object: bear }];
    quiet_about(&s, &reg, "but is in");
    s.get_object_mut(bear).unwrap().zone = Zone::Graveyard;
    flags(&s, &reg, &format!("#{} was tapped but is in Graveyard", bear.0));
}

/// CR 400.7/302.6/306.5b: what the entry and departure events promise about
/// the object afterwards — it is where the last move said, a creature that
/// just arrived is summoning sick, and a planeswalker arrives on its
/// printed loyalty.
#[test]
fn an_entry_event_describes_the_permanent_that_arrived() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.get_object_mut(bear).unwrap().summoning_sick = true;
    state.events = vec![GameEvent::EnteredBattlefield { object: bear, controller: P0 }];
    clean(&state, &reg);

    // CR 302.6: it came under its controller's command this turn.
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().summoning_sick = false;
    flags(&s, &reg, "entered this action but is not summoning sick (CR 302.6)");
    // A token is not exempt for being a token: the exemption is for one
    // still being asked whom it attacks, and that prompt has to be up.
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().summoning_sick = false;
    s.get_object_mut(bear).unwrap().is_token = true;
    flags(&s, &reg, "entered this action but is not summoning sick (CR 302.6)");

    // CR 400.7: the object is where the last move it announced put it.
    let mut s = base().0;
    let card = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    s.events = vec![GameEvent::LeftBattlefield {
        object: card, to: Zone::Graveyard, last_controller: P0 }];
    flags(&s, &reg, "last moved to Graveyard but is in Battlefield");

    let mut s = base().0;
    let card = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    s.get_object_mut(card).unwrap().zone = Zone::Exile;
    s.events = vec![GameEvent::ObjectMoved {
        object: card, from: Zone::Battlefield, to: Zone::Graveyard }];
    flags(&s, &reg, "last moved to Graveyard but is in Exile");

    // CR 400.7: an entry event is about a permanent that is on the
    // battlefield afterwards.
    let mut s = base().0;
    let card = spell_in_hand(&mut s, &reg, "Grizzly Bears", P0);
    s.events = vec![GameEvent::EnteredBattlefield { object: card, controller: P0 }];
    flags(&s, &reg, "last moved to Battlefield but is in Hand");

    // CR 111.8 is about tokens: a card that leaves and comes back in one
    // window is an ordinary flicker, not a token returning from nowhere.
    let mut s = base().0;
    let card = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    s.events = vec![
        GameEvent::LeftBattlefield { object: card, to: Zone::Exile, last_controller: P0 },
        GameEvent::EnteredBattlefield { object: card, controller: P0 },
    ];
    s.get_object_mut(card).unwrap().summoning_sick = true;
    quiet_about(&s, &reg, "(CR 111.8)");

    // And the tap ledger is cleared by leaving: tapped, then gone, is not a
    // permanent that "was tapped but is in Graveyard".
    let mut s = base().0;
    let card = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    s.get_object_mut(card).unwrap().tapped = true;
    s.events = vec![
        GameEvent::Tapped { object: card },
        GameEvent::LeftBattlefield { object: card, to: Zone::Graveyard, last_controller: P0 },
    ];
    s.get_object_mut(card).unwrap().zone = Zone::Graveyard;
    quiet_about(&s, &reg, "was tapped but is in");

    // CR 306.5b: a planeswalker enters with the loyalty its card prints.
    let mut s = base().0;
    let walker = named_permanent(&mut s, &reg, "Liliana of the Veil", P0);
    let printed = counters_of(&s, walker, CounterType::Loyalty);
    assert!(printed > 0, "test setup: Liliana enters on her printed loyalty");
    s.events = vec![GameEvent::EnteredBattlefield { object: walker, controller: P0 }];
    quiet_about(&s, &reg, "(CR 306.5b)");
    set_loyalty(&mut s, walker, printed + 1);
    flags(&s, &reg, "(CR 306.5b)");
}

/// CR 121.1/701.8a/701.17a: what the zone-change events say about where the
/// card they name ended up.
#[test]
fn a_zone_change_event_leaves_the_card_where_it_says() {
    let (mut state, reg) = base();
    let drawn = stock_library(&mut state, &reg, P0, 1)[0];
    state.get_object_mut(drawn).unwrap().name = "Forest".into();
    let elsewhere = named_permanent(&mut state, &reg, "Grizzly Bears", P0);

    // A clean draw: out of the library, off the library order, into hand.
    let mut clean_draw = state.clone();
    clean_draw.get_player_mut(P0).library_order.retain(|id| *id != drawn);
    clean_draw.get_object_mut(drawn).unwrap().zone = Zone::Hand;
    clean_draw.events = vec![GameEvent::CardDrawn { player: P0, object: drawn }];
    clean(&clean_draw, &reg);

    // Still listed in the library it was drawn out of.
    let mut s = clean_draw.clone();
    s.get_player_mut(P0).library_order.push(drawn);
    flags(&s, &reg, "is still listed in p0's library");

    // CR 121.1: the card ends up in hand, unless a later event in the same
    // action says where it went instead — and "later event" means one about
    // this card, not any card at all.
    let mut s = clean_draw.clone();
    s.get_object_mut(drawn).unwrap().zone = Zone::Stack;
    s.stack.push(StackEntry::Spell(drawn));
    s.events.push(GameEvent::ObjectMoved { object: drawn, from: Zone::Hand, to: Zone::Stack });
    quiet_about(&s, &reg, "(CR 121.1)");
    let mut s = clean_draw.clone();
    s.get_object_mut(drawn).unwrap().zone = Zone::Stack;
    s.stack.push(StackEntry::Spell(drawn));
    s.get_object_mut(elsewhere).unwrap().tapped = true;
    s.events.push(GameEvent::Tapped { object: elsewhere });
    flags(&s, &reg, "but it is in Stack (CR 121.1)");

    // Drawn but somewhere other than hand, with nothing later moving it.
    let mut s = clean_draw.clone();
    s.get_object_mut(drawn).unwrap().zone = Zone::Exile;
    flags(&s, &reg, "but it is in Exile (CR 121.1)");

    // CR 701.8a: discarding puts the card in the graveyard.
    let mut s = state.clone();
    let card = spell_in_hand(&mut s, &reg, "Moment of Heroism", P0);
    s.events = vec![GameEvent::Discarded { player: P0, object: card }];
    flags(&s, &reg, "but it is in Hand (CR 701.8a)");
    s.get_object_mut(card).unwrap().zone = Zone::Graveyard;
    quiet_about(&s, &reg, "(CR 701.8a)");

    // CR 701.9a: and it is that player's own card.
    let mut s = state.clone();
    let theirs = spell_in_hand(&mut s, &reg, "Moment of Heroism", P1);
    s.get_object_mut(theirs).unwrap().zone = Zone::Graveyard;
    s.events = vec![GameEvent::Discarded { player: P0, object: theirs }];
    flags(&s, &reg, "not that player's card (CR 701.9a)");

    // CR 701.17a: milling is out of that player's own library.
    let mut s = state.clone();
    let milled = named_card_in_graveyard(&mut s, &reg, "Grizzly Bears", P1);
    s.events = vec![GameEvent::CreatureCardMilled { object: milled, milled_player: P0 }];
    flags(&s, &reg, "not that player's card (CR 701.17a)");

    // CR 104.2a: a loss ends the game in the same breath.
    let mut s = state.clone();
    s.get_player_mut(P1).lost = true;
    s.get_player_mut(P1).loss_reason = Some(LossReason::Conceded);
    s.events = vec![GameEvent::PlayerLost { player: P1, reason: LossReason::Conceded }];
    flags(&s, &reg, "without the game ending afterwards (CR 104.2a)");
    s.result = Some(mtg_engine::state::GameResult::Winner(P0));
    s.events.push(GameEvent::GameEnded {
        result: mtg_engine::state::GameResult::Winner(P0) });
    quiet_about(&s, &reg, "(CR 104.2a)");

    // The last zone event about an object agrees with where it is now.
    let mut s = state.clone();
    let bear = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    s.events = vec![GameEvent::ObjectMoved { object: bear, from: Zone::Hand, to: Zone::Exile }];
    flags(&s, &reg, "last moved to Exile but is in Battlefield");
}

/// CR 700.4: dying is leaving the battlefield for the graveyard, reported
/// once, with the controller it had.
#[test]
fn a_death_event_is_a_zone_change_to_the_graveyard() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let card_id = state.get_object(bear).unwrap().card_id;
    let died = |controller: PlayerId, is_token: bool| GameEvent::CreatureDied {
        object: bear, name: "Grizzly Bears".into(), card_id, controller, damaged_by: vec![],
        last_known_toughness: 2, is_token, subtypes: vec![] };

    let mut dead = state.clone();
    dead.creature_died_this_turn = true;
    dead.get_object_mut(bear).unwrap().zone = Zone::Graveyard;
    dead.events = vec![died(P0, false),
        GameEvent::LeftBattlefield { object: bear, to: Zone::Graveyard, last_controller: P0 }];
    clean(&dead, &reg);

    // It left for somewhere else, or under someone else.
    let mut s = dead.clone();
    s.get_object_mut(bear).unwrap().zone = Zone::Exile;
    s.events[1] = GameEvent::LeftBattlefield { object: bear, to: Zone::Exile, last_controller: P0 };
    flags(&s, &reg, "but it left for Exile from p0");
    let mut s = dead.clone();
    s.events[1] = GameEvent::LeftBattlefield { object: bear, to: Zone::Graveyard, last_controller: P1 };
    flags(&s, &reg, "but it left for Graveyard from p1");

    // A card that died is still a card.
    let mut s = dead.clone();
    s.objects.remove(&bear);
    flags(&s, &reg, "a card that ceased to exist");
    // Unless it was a token, which ceases to exist by rule (CR 111.7).
    let mut s = dead.clone();
    s.objects.remove(&bear);
    s.events[0] = died(P0, true);
    quiet_about(&s, &reg, "a card that ceased to exist");

    // Still standing where it died, with nothing having put it back.
    let mut s = dead.clone();
    s.get_object_mut(bear).unwrap().zone = Zone::Battlefield;
    flags(&s, &reg, "but it is on the battlefield with no re-entry");

    // Morbid reads the flag; every death path sets it.
    let mut s = dead.clone();
    s.creature_died_this_turn = false;
    flags(&s, &reg, "but creature_died_this_turn is false");
}

/// CR 502.3: the untap step untaps the active player's permanents, and
/// nothing else.
#[test]
fn the_untap_step_untaps_only_the_active_players_permanents() {
    let (mut state, reg) = base();
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.step = Step::Untap;
    state.priority_player = None;

    let mut s = state.clone();
    s.events = vec![GameEvent::TurnStarted { player: P0, turn: 3 },
                    GameEvent::StepStarted { step: Step::Untap },
                    GameEvent::Untapped { object: mine }];
    quiet_about(&s, &reg, "(CR 502.3)");

    let mut s = state.clone();
    s.events = vec![GameEvent::TurnStarted { player: P0, turn: 3 },
                    GameEvent::StepStarted { step: Step::Untap },
                    GameEvent::Untapped { object: theirs }];
    flags(&s, &reg, "which p0 does not control (CR 502.3)");

    // Outside the untap step the same event is unremarkable.
    let mut s = state.clone();
    s.step = Step::PrecombatMain;
    s.events = vec![GameEvent::Untapped { object: theirs }];
    quiet_about(&s, &reg, "(CR 502.3)");
}

/// The game's own result events agree with the state, and there is one of
/// them (CR 104.2a).
#[test]
fn the_result_events_agree_with_the_result() {
    let (mut state, reg) = base();
    named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let ended = mtg_engine::state::GameResult::Winner(P0);

    let mut over = state.clone();
    over.get_player_mut(P1).lost = true;
    over.get_player_mut(P1).loss_reason = Some(LossReason::Conceded);
    over.result = Some(ended.clone());
    over.events = vec![GameEvent::PlayerLost { player: P1, reason: LossReason::Conceded },
                       GameEvent::GameEnded { result: ended.clone() }];
    clean(&over, &reg);

    // The event says one thing, the player record another.
    let mut s = over.clone();
    s.events[0] = GameEvent::PlayerLost { player: P1, reason: LossReason::LifeReachedZero };
    flags(&s, &reg, "but lost=true reason=Some(Conceded)");

    // The event says one result, the state another.
    let mut s = over.clone();
    s.events[1] = GameEvent::GameEnded { result: mtg_engine::state::GameResult::Draw };
    flags(&s, &reg, "GameEnded Draw but the result is");

    // The game ends once.
    let mut s = over.clone();
    s.events.push(GameEvent::GameEnded { result: ended });
    flags(&s, &reg, "the game ended 2 times in one action");

    // CR 106.4: adding no mana is not adding mana.
    let mut s = state.clone();
    s.events = vec![GameEvent::ManaAdded { player: P0, mana_type: ManaType::Green, amount: 0 }];
    flags(&s, &reg, "ManaAdded of nothing");
}

/// CR 500.2/500.5: a step boundary finds the stack empty, the pools empty,
/// nothing half-cast, and the passes reset — and an untap step arrives with
/// its turn.
#[test]
fn a_step_boundary_leaves_nothing_straddling_it() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.events = vec![GameEvent::StepStarted { step: Step::PrecombatMain }];
    clean(&state, &reg);

    // CR 500.2: only triggers can have joined the stack since.
    let mut s = state.clone();
    let spell = castable_spell(&mut s, &reg, "Moment of Heroism", P0);
    s.get_object_mut(spell).unwrap().zone = Zone::Stack;
    s.stack.push(StackEntry::Spell(spell));
    flags(&s, &reg, "a spell or ability survived a step boundary (CR 500.2)");
    flags(&s, &reg, "an object is in the stack zone right after a step change");

    let mut s = state.clone();
    s.resolving_spell = Some(ObjectId(1));
    flags(&s, &reg, "a cast or resolution straddles a step boundary");

    let mut s = state.clone();
    s.consecutive_passes = 1;
    flags(&s, &reg, "1 passes carried across a step boundary");

    // CR 500.2: nothing a step boundary crosses is mid-flight — each of the
    // three ways a resolution can straddle one.
    let straddling = |f: &dyn Fn(&mut GameState)| {
        let mut c = state.clone();
        f(&mut c);
        c
    };
    flags(&straddling(&|c| {
        c.pending_ability_effect = Some(mtg_engine::state::PendingAbilityEffect {
            source_id: bear, ability_index: 0,
            behavior_card_id: c.get_object(bear).unwrap().card_id,
            targets: vec![], description: "an ability".into(), activator: P0,
            target_requirement: None, unpaid: None,
        });
    }), &reg, "a cast or resolution straddles a step boundary");

    // CR 502.1: the untap step is the first step of a turn.
    let mut s = state.clone();
    s.step = Step::Untap;
    s.priority_player = None;
    s.events = vec![GameEvent::StepStarted { step: Step::Untap }];
    flags(&s, &reg, "an untap step started without a turn starting");
    s.events.insert(0, GameEvent::TurnStarted { player: P0, turn: 3 });
    quiet_about(&s, &reg, "an untap step started without a turn starting");
}

/// CR 502.3/505.6b/514.2: a turn begins clean — no damage, no shields, no
/// remembered activations, no floating mana, per-turn counters reset, and
/// the land drop back.
#[test]
fn a_turn_start_finds_the_board_reset() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.step = Step::Upkeep;
    state.events = vec![GameEvent::TurnStarted { player: P0, turn: 3 }];
    clean(&state, &reg);

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().abilities_activated_this_turn.insert(0);
    flags(&s, &reg, "remembers activations from last turn");

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().regeneration_shields = 1;
    flags(&s, &reg, "keeps a regeneration shield (CR 514.2)");

    // CR 514.1: a turn ends with its player at seven cards, and the count is
    // seven, not six.
    let mut s = state.clone();
    for _ in 0..7 {
        spell_in_hand(&mut s, &reg, "Forest", P1);
    }
    quiet_about(&s, &reg, "(CR 514.1)");
    spell_in_hand(&mut s, &reg, "Forest", P1);
    flags(&s, &reg, "holds 8 cards after their cleanup (CR 514.1)");

    // Each of the three leftovers a combat can leave behind, alone.
    let mut s = state.clone();
    s.combat_damage_step_pending = true;
    flags(&s, &reg, "combat state survives");
    let mut s = state.clone();
    let card = s.get_object(bear).unwrap().card_id;
    s.end_of_combat_exiles.push(mtg_engine::state::EndOfCombatExileEntry {
        target_id: bear, source_id: bear, source_card_id: card, controller: P0,
        description: "a delayed exile".into(),
    });
    flags(&s, &reg, "combat state survives");

    // CR 305.2/103.7a: the turn starts in one of its opening steps, for the
    // player it names, on the turn it names.
    let mut s = state.clone();
    s.step = Step::EndStep;
    flags(&s, &reg, "is active on turn 3 in EndStep");
    let mut s = state.clone();
    s.events = vec![GameEvent::TurnStarted { player: P1, turn: 3 }];
    flags(&s, &reg, "is active on turn 3");
    let mut s = state.clone();
    s.events = vec![GameEvent::TurnStarted { player: P0, turn: 4 }];
    flags(&s, &reg, "is active on turn 3");

    // CR 514.2 removes all three marks the turn leaves on a permanent, and
    // each of them alone is enough to say the cleanup did not happen.
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().damage_marked = 1;
    flags(&s, &reg, "carries damage from last turn (CR 514.2)");
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().damaged_by.push(bear);
    flags(&s, &reg, "carries damage from last turn (CR 514.2)");
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().dealt_deathtouch_damage = true;
    flags(&s, &reg, "carries damage from last turn (CR 514.2)");

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().summoning_sick = true;
    flags(&s, &reg, "is summoning sick at the start of its controller's turn");

    let mut s = state.clone();
    s.creature_died_this_turn = true;
    flags(&s, &reg, "per-turn counters not reset");
    let mut s = state.clone();
    s.num_spells_cast_this_turn.insert(P0, 1);
    flags(&s, &reg, "per-turn counters not reset");

    let mut s = state.clone();
    s.combat = Some(mtg_engine::state::CombatState::new());
    flags(&s, &reg, "combat state survives");

    let mut s = state.clone();
    s.get_player_mut(P0).land_plays_remaining = 0;
    flags(&s, &reg, "the land drop was not reset (CR 305.2)");

    let mut s = state.clone();
    add_mana(&mut s, P0, &[(ManaType::Green, 1)]);
    flags(&s, &reg, "has mana floating");

    // The turn that started is the turn the state is on.
    let mut s = state.clone();
    s.events = vec![GameEvent::TurnStarted { player: P1, turn: 3 }];
    flags(&s, &reg, "p0 is active on turn 3 in Upkeep");
}

/// CR 506.2/506.3/508.1: what an `AttackersDeclared` event says about each
/// creature it names — whom it attacks, that combat knows it, that it is a
/// creature its controller controls, and that the declaration stamped it.
#[test]
fn an_attackers_declared_event_describes_a_legal_attack() {
    let (mut state, reg) = base();
    let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.step = Step::DeclareAttackers;
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    state.priority_player = Some(P0);
    clean(&state, &reg);

    // A declaration with no combat at all.
    let mut s = state.clone();
    s.combat = None;
    flags(&s, &reg, "AttackersDeclared but no combat state");

    // CR 506.2: an attack is at the defending player.
    let mut s = state.clone();
    s.events = vec![GameEvent::AttackersDeclared { attackers: vec![(attacker, P0)] }];
    flags(&s, &reg, "attacks p0, not the defending player (CR 506.2)");

    // Combat's own map knows it.
    let mut s = state.clone();
    s.combat.as_mut().unwrap().attackers.remove(&attacker);
    flags(&s, &reg, "is not in combat");

    // CR 508.1: the declaration stamps the turn it attacked on.
    let mut s = state.clone();
    s.get_object_mut(attacker).unwrap().attacked_on_turn = None;
    flags(&s, &reg, "is not stamped as attacking this turn (CR 508.1)");

    // CR 508.1a: an attacker is its controller's own.
    let mut s = state.clone();
    s.get_object_mut(attacker).unwrap().controller = P1;
    flags(&s, &reg, "is controlled by p1 (CR 508.1a)");

    // CR 506.3: and it is a creature.
    let mut s = state.clone();
    let land = named_permanent(&mut s, &reg, "Forest", P0);
    s.get_object_mut(land).unwrap().attacked_on_turn = Some(s.turn_number);
    s.get_object_mut(land).unwrap().tapped = true;
    s.combat.as_mut().unwrap().attackers.insert(land, P1);
    s.events.push(GameEvent::Tapped { object: land });
    s.events = s.events.iter().cloned().map(|e| match e {
        GameEvent::AttackersDeclared { mut attackers } => {
            attackers.push((land, P1));
            GameEvent::AttackersDeclared { attackers }
        }
        other => other,
    }).collect();
    flags(&s, &reg, "is not a creature (CR 506.3)");

    // CR 508.1c: nothing that can't attack was declared.
    let mut s = state.clone();
    s.get_object_mut(attacker).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::PreventAttack { scope: EffectScope::OnSelf },
    ]);
    flags(&s, &reg, "can't attack (CR 508.1c)");
}

/// CR 509.1a/509.1b/702.9b/702.13b/702.16f: an evasion ability a blocker
/// cannot answer makes the declared block illegal.
#[test]
fn a_block_that_evasion_forbids_is_flagged() {
    let reg = registry();

    let blocked = |grant: Option<Keyword>| {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        state.turn_number = 3;
        // A green attacker and a white blocker, so they share no color and
        // intimidate has something to say (CR 702.13b).
        let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
        let blocker = named_permanent(&mut state, &reg, "Doomed Traveler", P1);
        state.step = Step::DeclareAttackers;
        submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
        state.step = Step::DeclareBlockers;
        submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);
        state.priority_player = Some(P0);
        // Granted after the declaration: the engine would refuse to declare
        // the block, and the clause exists to catch a block that became
        // illegal some other way.
        if let Some(kw) = grant {
            grant_keyword(&mut state, attacker, kw);
        }
        (state, blocker)
    };

    let (clean_state, blocker) = blocked(None);
    quiet_about(&clean_state, &reg, "declared block");

    let (s, _) = blocked(Some(Keyword::Flying));
    flags(&s, &reg, "a flier blocked by neither flying nor reach (CR 702.9b)");

    let (s, _) = blocked(Some(Keyword::Intimidate));
    flags(&s, &reg, "intimidate blocked by a non-artifact sharing no color (CR 702.13b)");

    // A blocker that is tapped is no blocker at all (CR 509.1a).
    let (mut s, _) = blocked(None);
    s.get_object_mut(blocker).unwrap().tapped = true;
    flags(&s, &reg, "blocker is tapped (CR 509.1a)");
}

/// CR 509.1b/702.16e/702.16f/302.6: the remaining ways a declared block or
/// a dealt damage is one the rules forbid, and the summoning sickness a
/// creature that arrived this action carries.
#[test]
fn protection_and_summoning_sickness_are_checked_in_the_event_window() {
    let (mut state, reg) = base();
    let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let blocker = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.step = Step::DeclareAttackers;
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    state.step = Step::DeclareBlockers;
    submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);
    state.priority_player = Some(P0);
    clean(&state, &reg);

    // CR 702.16f: an attacker with protection from the blocker.
    let mut s = state.clone();
    s.get_object_mut(attacker).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::ProtectionFromSubtype { subtype: "Bear".into(), scope: EffectScope::OnSelf },
    ]);
    flags(&s, &reg, "the attacker has protection from the blocker (CR 702.16f)");

    // A blocker that cannot block at all.
    let mut s = state.clone();
    s.get_object_mut(blocker).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::PreventBlock { scope: EffectScope::OnSelf },
    ]);
    flags(&s, &reg, "blocker can't block");

    // CR 702.16e: damage from a source the target has protection from.
    let mut s = state.clone();
    s.step = Step::CombatDamage;
    s.get_object_mut(blocker).unwrap().damage_marked = 2;
    s.get_object_mut(blocker).unwrap().damaged_by.push(attacker);
    s.get_object_mut(blocker).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::ProtectionFromSubtype { subtype: "Bear".into(), scope: EffectScope::OnSelf },
    ]);
    s.events = vec![GameEvent::CombatDamageDealt {
        source: attacker, target: DamageTarget::Object(blocker), amount: 2 }];
    flags(&s, &reg, "the target has protection from the source (CR 702.16e)");

    // CR 302.6: a creature that arrived this action is summoning sick.
    let (mut s, _) = (base().0, ());
    let arrived = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    s.get_object_mut(arrived).unwrap().summoning_sick = false;
    s.events = vec![GameEvent::EnteredBattlefield { object: arrived, controller: P0 }];
    flags(&s, &reg, "entered this action but is not summoning sick (CR 302.6)");
}
