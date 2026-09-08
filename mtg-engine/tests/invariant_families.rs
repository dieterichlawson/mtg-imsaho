//! Self-tests for the invariant families added by the rulebook sweep: each
//! family must flag the exact corruption it claims to catch (a mutant that
//! blinds a clause would otherwise go unnoticed — the fuzzer only reports
//! what the checker reports), and the healthy version of every structure
//! it polices must flag nothing (see `a_clean_state_has_no_violations`).

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::{DamageTarget, GameEvent};
use mtg_engine::ids::{ObjectId, PlayerId};
use mtg_engine::invariants::{check_core, check_settled};
use mtg_engine::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind, StackEntry, TemporaryEffect};
use mtg_engine::types::*;

fn base() -> (GameState, CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.turn_number = 3;
    (state, reg)
}

#[track_caller]
fn flags_core(state: &GameState, reg: &CardRegistry, needle: &str) {
    let v = check_core(state, reg);
    assert!(v.iter().any(|m| m.contains(needle)), "expected a core violation containing {needle:?}, got: {v:?}");
}

#[track_caller]
fn quiet_core_about(state: &GameState, reg: &CardRegistry, needle: &str) {
    let v = check_core(state, reg);
    assert!(!v.iter().any(|m| m.contains(needle)),
        "expected no core violation containing {needle:?}, got: {v:?}");
}

#[track_caller]
fn flags_settled(state: &GameState, reg: &CardRegistry, needle: &str) {
    let v = check_settled(state, reg);
    assert!(v.iter().any(|m| m.contains(needle)), "expected a settled violation containing {needle:?}, got: {v:?}");
}

/// A hand-built fixture never ran the trigger collector; the game loop
/// checks a state only after it has, so the clean baselines look at the
/// state the way the loop would (`trigger_event_index` caught up).
fn as_collected(state: &GameState) -> GameState {
    let mut s = state.clone();
    s.trigger_event_index = s.events.len();
    s
}

#[track_caller]
fn clean(state: &GameState, reg: &CardRegistry) {
    assert_eq!(check_settled(&as_collected(state), reg), Vec::<String>::new());
}

#[track_caller]
fn clean_core(state: &GameState, reg: &CardRegistry) {
    assert_eq!(check_core(&as_collected(state), reg), Vec::<String>::new());
}

// ── objects ──────────────────────────────────────────────────────────────

#[test]
fn object_zone_and_identity_rules_are_checked() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    clean(&state, &reg);

    let mut s = state.clone();
    s.move_object(bear, Zone::Graveyard, &reg);
    s.get_object_mut(bear).unwrap().controller = P1;
    flags_core(&s, &reg, "controlled by p1 but owned by p0 (CR 108.4)");

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().card_id = mtg_engine::ids::CardId(424_242);
    flags_core(&s, &reg, "is not in the registry");

    let mut s = state.clone();
    let bolt = spell_in_hand(&mut s, &reg, "Moment of Heroism", P0);
    s.get_object_mut(bolt).unwrap().zone = Zone::Battlefield;
    flags_core(&s, &reg, "instant/sorcery on the battlefield");

    let mut s = state.clone();
    let land = spell_in_hand(&mut s, &reg, "Forest", P0);
    s.get_object_mut(land).unwrap().zone = Zone::Stack;
    s.stack.push(StackEntry::Spell(land));
    flags_core(&s, &reg, "land on the stack (CR 305.9)");

    let mut s = state.clone();
    let play = spell_in_hand(&mut s, &reg, "Devil's Play", P0);
    s.get_object_mut(play).unwrap().x_value = Some(3);
    flags_core(&s, &reg, "carries x_value");
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().x_value = Some(1);
    flags_core(&s, &reg, "its cost has no X");

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().is_transformed = true;
    flags_core(&s, &reg, "has no back face (CR 712.9)");
    let mut s = state.clone();
    let smith = named_permanent(&mut s, &reg, "Village Ironsmith", P0);
    s.move_object(smith, Zone::Graveyard, &reg);
    s.get_object_mut(smith).unwrap().is_transformed = true;
    flags_core(&s, &reg, "is transformed in Graveyard (CR 712.8a)");

    let mut s = state.clone();
    s.move_object(bear, Zone::Graveyard, &reg);
    s.get_object_mut(bear).unwrap().copy_grantor = Some(s.get_object(bear).unwrap().card_id);
    flags_core(&s, &reg, "is still a copy (CR 400.7)");

    let mut s = state.clone();
    let rites = spell_in_hand(&mut s, &reg, "Unburial Rites", P0);
    s.get_object_mut(rites).unwrap().zone = Zone::Graveyard;
    s.get_object_mut(rites).unwrap().cast_with_flashback = true;
    s.get_object_mut(rites).unwrap().cast_from_zone = Some(Zone::Graveyard);
    flags_core(&s, &reg, "cast with flashback is in Graveyard (CR 702.34a)");

    let mut s = state.clone();
    s.move_object(bear, Zone::Graveyard, &reg);
    s.get_object_mut(bear).unwrap().keywords.push(Keyword::Flying);
    flags_core(&s, &reg, "keeps runtime characteristics");
    let mut s = state.clone();
    s.move_object(bear, Zone::Graveyard, &reg);
    s.get_object_mut(bear).unwrap().power = Some(9);
    flags_core(&s, &reg, "printed Some(2)/Some(2) (CR 400.7)");

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().is_legendary = true;
    flags_core(&s, &reg, "flagged legendary but its face is not");

    let mut s = state.clone();
    s.move_object(bear, Zone::Graveyard, &reg);
    s.get_object_mut(bear).unwrap().summoning_sick = true;
    flags_core(&s, &reg, "in Graveyard is summoning sick");
    let mut s = state.clone();
    s.move_object(bear, Zone::Graveyard, &reg);
    s.get_object_mut(bear).unwrap().abilities_activated_this_turn.insert(0);
    flags_core(&s, &reg, "remembers activations this turn");
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().abilities_activated_this_turn.insert(999);
    flags_core(&s, &reg, "used a loyalty ability but is no planeswalker");

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().damage_marked = 1;
    flags_core(&s, &reg, "no record of what dealt it");
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().dealt_deathtouch_damage = true;
    flags_core(&s, &reg, "dealt deathtouch damage but has none marked");
    let mut s = state.clone();
    let land = named_permanent(&mut s, &reg, "Forest", P0);
    s.get_object_mut(land).unwrap().damage_marked = 2;
    s.get_object_mut(land).unwrap().damaged_by.push(bear);
    flags_core(&s, &reg, "no battlefield creature (CR 120.3)");
    let mut s = state.clone();
    let lili = named_permanent(&mut s, &reg, "Liliana of the Veil", P0);
    s.get_object_mut(lili).unwrap().damage_marked = 1;
    s.get_object_mut(lili).unwrap().damaged_by.push(bear);
    flags_core(&s, &reg, "planeswalker with damage marked (CR 120.3c)");

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().attached_to_player = Some(P1);
    flags_core(&s, &reg, "attached to a player but is no Aura (CR 303.4)");
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().last_attached_to_player = Some(P1);
    flags_core(&s, &reg, "keeps a last-attached-to-player shadow");

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().subtypes.push("Aura".into());
    flags_core(&s, &reg, "has subtype Aura without type Enchantment (CR 205.3)");

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().name = "Grizzly Bear".into();
    flags_core(&s, &reg, "name cache says \"Grizzly Bear\"");

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().chosen_mode = Some(0);
    s.get_object_mut(bear).unwrap().zone = Zone::Stack;
    s.stack.push(StackEntry::Spell(bear));
    flags_core(&s, &reg, "has a chosen mode but is not modal");

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().toughness = None;
    flags_core(&s, &reg, "(CR 208.1)");

    let mut s = state.clone();
    s.day_night = Some(mtg_engine::state::DayNight::Day);
    flags_core(&s, &reg, "day/night designation set");
}

#[test]
fn token_shape_rules_are_checked() {
    let (mut state, reg) = base();
    let wolf = state.create_token_with_subtypes("", P0, 2, 2, vec![Color::Green], vec![CardType::Creature],
        vec![], vec!["Wolf".into()], &reg)[0];
    clean(&state, &reg);

    // CR 111.4: the name is the subtype(s) — the wrong word is wrong, and so
    // is the right word with " Token" welded on (issues #331, #334).
    let mut s = state.clone();
    s.get_object_mut(wolf).unwrap().name = "Wolf Token".into();
    flags_core(&s, &reg, "is not its subtypes");
    let mut s = state.clone();
    s.get_object_mut(wolf).unwrap().name = "Spirit".into();
    flags_core(&s, &reg, "is not its subtypes");
    let mut s = state.clone();
    s.get_object_mut(wolf).unwrap().zone_change_count = 1;
    flags_core(&s, &reg, "changed zones 1 time(s) and is on the battlefield");
    let mut s = state.clone();
    s.get_object_mut(wolf).unwrap().card_types.clear();
    flags_core(&s, &reg, "token with subtypes");
}

// ── stack ────────────────────────────────────────────────────────────────

#[test]
fn stack_entry_rules_are_checked() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let bolt = castable_spell(&mut state, &reg, "Moment of Heroism", P0);
    let state = cast_onto_stack(&state, &reg, bolt, vec![Target::Object(bear)]);
    clean_core(&state, &reg);

    let mut s = state.clone();
    s.get_object_mut(bolt).unwrap().targets = vec![Target::Illegal];
    flags_core(&s, &reg, "stores an Illegal target");
    let mut s = state.clone();
    s.get_object_mut(bolt).unwrap().targets = vec![Target::Object(bear), Target::Object(bear)];
    flags_core(&s, &reg, "twice (CR 115.3)");
    let mut s = state.clone();
    s.get_object_mut(bolt).unwrap().targets.clear();
    flags_core(&s, &reg, "has 0 targets for requirement");
    let mut s = state.clone();
    s.get_object_mut(bolt).unwrap().chosen_mode = Some(2);
    flags_core(&s, &reg, "has a chosen mode but is not modal");

    // A creature spell above another entry, in the wrong step, on the
    // wrong turn: sorcery speed was violated three ways.
    let mut s = state.clone();
    let creature = spell_in_hand(&mut s, &reg, "Grizzly Bears", P1);
    s.get_object_mut(creature).unwrap().zone = Zone::Stack;
    s.stack.push(StackEntry::Spell(creature));
    s.step = Step::DeclareAttackers;
    flags_core(&s, &reg, "sits above 1 stack entries");
    flags_core(&s, &reg, "sorcery-speed on the stack in DeclareAttackers");
    flags_core(&s, &reg, "on p0's turn");

    let mut s = state.clone();
    s.stack.push(StackEntry::Ability {
        source_id: bear, ability_index: 0, behavior_card_id: s.get_object(bear).unwrap().card_id,
        targets: vec![Target::Object(bolt)], activator: P0, x_value: None, target_requirement: None,
        sacrificed: None, sacrificed_toughness: Some(2), loyalty: false,
    });
    flags_core(&s, &reg, "has targets but no requirement");
    flags_core(&s, &reg, "remembers a sacrificed creature's toughness but no sacrifice");

    let mut s = state.clone();
    s.stack.push(StackEntry::Spell(bolt));
    flags_core(&s, &reg, "is on two stack entries");
}

#[test]
fn trigger_queue_and_resolution_bookkeeping_are_checked() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let geist = named_permanent(&mut state, &reg, "Geist of Saint Traft", P1);
    clean(&state, &reg);
    let trigger = |src: ObjectId, controller: PlayerId, s: &GameState| mtg_engine::triggers::PendingTrigger::new(
        mtg_engine::triggers::TriggerSource::new(src, s.get_object(src).unwrap().card_id, controller, "t"),
        mtg_engine::triggers::TriggerEvent::Attacks { attacker: src, defending_player: s.opponent(controller) },
    );

    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(trigger(geist, P1, &s));
    flags_core(&s, &reg, "AP push queue holds p1's trigger");
    let mut s = state.clone();
    s.pending_trigger_pushes_nap.push(trigger(bear, P0, &s));
    flags_core(&s, &reg, "NAP push queue holds the active player's trigger");
    let mut s = state.clone();
    s.pending_triggers.push(trigger(bear, P0, &s));
    flags_core(&s, &reg, "only state and copy-ETB triggers are queued there");

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().state_trigger_on_stack = true;
    flags_core(&s, &reg, "state_trigger_on_stack=true but 0 such trigger(s)");

    let mut s = state.clone();
    s.resolving_spell = Some(bear);
    flags_core(&s, &reg, "with no choice pending");
    flags_core(&s, &reg, "is in Battlefield");
    let mut s = state.clone();
    s.resolving_ability_activator = Some(P0);
    flags_core(&s, &reg, "resolving_ability_activator set with no choice pending");
    let mut s = state.clone();
    s.resolving_trigger_from_back_face = Some(false);
    flags_core(&s, &reg, "survived past a trigger's hook");
}

#[test]
fn a_cast_in_progress_is_checked_against_its_prompt_and_zones() {
    let (mut state, reg) = base();
    let play = castable_spell(&mut state, &reg, "Devil's Play", P0);
    // Mana beyond the non-X part, so there is an X to fund.
    add_mana(&mut state, P0, &[(ManaType::Red, 2)]);
    let state = cast_onto_stack(&state, &reg, play, vec![Target::Player(P1)]);
    assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseXFunding { .. }, .. })), "test precondition: funding prompt");
    clean_core(&state, &reg);

    let mut s = state.clone();
    s.get_object_mut(play).unwrap().zone = Zone::Battlefield;
    flags_core(&s, &reg, "the spell is in Battlefield before its costs are paid");
    let mut s = state.clone();
    s.pending_spell_cast.as_mut().unwrap().player = P1;
    flags_core(&s, &reg, "but the stash is for");
    let mut s = state.clone();
    s.pending_spell_cast.as_mut().unwrap().tap_plan.push((play, 0));
    flags_core(&s, &reg, "plans to tap");
    let mut s = state.clone();
    s.pending_spell_cast.as_mut().unwrap().exile_ids.push(play);
    flags_core(&s, &reg, "would exile");
}

/// CR 603.3d: a trigger is given a target only when the ability that
/// triggered asks for one, and never more than one. A trigger carrying a
/// target its ability never declared resolves against something nobody
/// chose — and `chosen_targets` is the only record, so nothing else notices.
#[test]
fn a_trigger_carrying_a_target_its_ability_never_asked_for_is_flagged() {
    let (mut state, reg) = base();
    // Rage Thrower's death trigger targets a player; Grizzly Bears has no
    // triggered ability at all.
    let thrower = named_permanent(&mut state, &reg, "Rage Thrower", P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let dead = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.get_object_mut(dead).unwrap().zone = Zone::Graveyard;

    let died = || mtg_engine::triggers::TriggerEvent::CreatureDied {
        dead: mtg_engine::triggers::DeadCreature {
            id: dead, name: "Grizzly Bears".into(), controller: P1, damaged_by: vec![],
            toughness: 2, is_token: false, subtypes: vec!["Bear".into()],
        },
    };
    let with_targets = |src: ObjectId, s: &GameState, targets: Vec<Target>| {
        let mut source = mtg_engine::triggers::TriggerSource::new(
            src, s.get_object(src).unwrap().card_id, P0, "t");
        source.chosen_targets = targets;
        mtg_engine::triggers::PendingTrigger::new(source, died())
    };

    // Rage Thrower's morbid trigger does target, so one target is right.
    let mut s = state.clone();
    s.stack.push(StackEntry::Trigger(with_targets(thrower, &s, vec![Target::Player(P1)])));
    clean_core(&s, &reg);

    let mut s = state.clone();
    s.stack.push(StackEntry::Trigger(with_targets(bear, &s, vec![Target::Player(P1)])));
    flags_core(&s, &reg, "carries a target but the ability does not target");

    let mut s = state.clone();
    s.stack.push(StackEntry::Trigger(with_targets(thrower, &s, vec![Target::Player(P1), Target::Player(P0)])));
    flags_core(&s, &reg, "has 2 targets");

    // The ability that targets is matched by kind, not merely by existing:
    // Rage Thrower's targeting ability is its morbid one, so an upkeep
    // trigger from the same card carrying a target is still wrong.
    let mut s = state.clone();
    let mut source = mtg_engine::triggers::TriggerSource::new(
        thrower, s.get_object(thrower).unwrap().card_id, P0, "t");
    source.chosen_targets = vec![Target::Player(P1)];
    s.stack.push(StackEntry::Trigger(mtg_engine::triggers::PendingTrigger::new(
        source, mtg_engine::triggers::TriggerEvent::Upkeep)));
    flags_core(&s, &reg, "carries a target but the ability does not target");
}

/// CR 303.4a: an Aura spell targets what its enchant ability names. An Aura
/// that enchants players but is on the stack targeting an object resolves
/// onto something it cannot legally be attached to.
#[test]
fn an_aura_on_the_stack_targeting_the_wrong_kind_of_thing_is_flagged() {
    let (mut state, reg) = base();
    // Curse of the Pierced Heart enchants a player; Bonds of Faith a creature.
    let curse = spell_in_hand(&mut state, &reg, "Curse of the Pierced Heart", P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.get_object_mut(curse).unwrap().zone = Zone::Stack;
    state.get_object_mut(curse).unwrap().targets = vec![Target::Player(P1)];
    state.stack.push(StackEntry::Spell(curse));
    clean_core(&state, &reg);

    let mut s = state.clone();
    s.get_object_mut(curse).unwrap().targets = vec![Target::Object(bear)];
    flags_core(&s, &reg, "enchants players but targets an object");
}

/// CR 601.2h/602.2: the cost a cast is still paying names permanents its
/// caster controls and untapped, each once, and cards in their own
/// graveyard, each once. A stash that says otherwise taps or exiles
/// something the player never offered.
#[test]
fn a_stashed_payment_that_names_the_wrong_permanents_is_flagged() {
    let (mut state, reg) = base();
    let forest = named_permanent(&mut state, &reg, "Forest", P0);
    let theirs = named_permanent(&mut state, &reg, "Forest", P1);
    let tapped = named_permanent(&mut state, &reg, "Forest", P0);
    state.get_object_mut(tapped).unwrap().tapped = true;
    let fodder = named_card_in_graveyard(&mut state, &reg, "Forest", P0);
    let theirs_gy = named_card_in_graveyard(&mut state, &reg, "Forest", P1);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let their_bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    // A real mid-payment cast: the stash and the prompt it waits on.
    let play = castable_spell(&mut state, &reg, "Devil's Play", P0);
    add_mana(&mut state, P0, &[(ManaType::Red, 2)]);
    let state = cast_onto_stack(&state, &reg, play, vec![Target::Player(P1)]);
    assert!(state.pending_spell_cast.is_some(), "test precondition: the cast is stashed");
    clean_core(&state, &reg);

    let with = |f: &dyn Fn(&mut mtg_engine::state::PendingSpellCast)| {
        let mut s = state.clone();
        f(s.pending_spell_cast.as_mut().unwrap());
        s
    };

    clean_core(&with(&|c| { c.tap_plan.push((forest, 0)); c.exile_ids.push(fodder); }), &reg);

    flags_core(&with(&|c| { c.tap_plan.push((forest, 0)); c.tap_plan.push((forest, 0)); }), &reg,
        &format!("taps #{} twice", forest.0));
    flags_core(&with(&|c| c.tap_plan.push((theirs, 0))), &reg,
        &format!("plans to tap #{} which is not an untapped permanent of the caster", theirs.0));
    flags_core(&with(&|c| c.tap_plan.push((tapped, 0))), &reg,
        &format!("plans to tap #{} which is not an untapped permanent of the caster", tapped.0));
    flags_core(&with(&|c| { c.exile_ids.push(fodder); c.exile_ids.push(fodder); }), &reg,
        &format!("exiles #{} twice", fodder.0));
    flags_core(&with(&|c| c.exile_ids.push(forest)), &reg,
        &format!("would exile #{} which is not in the caster's graveyard", forest.0));
    flags_core(&with(&|c| c.exile_ids.push(theirs_gy)), &reg,
        &format!("would exile #{} which is not in the caster's graveyard", theirs_gy.0));

    // CR 701.21a: the creature an additional cost sacrifices is one the
    // caster controls on the battlefield, and never the spell itself.
    clean_core(&with(&|c| c.sacrifice = Some(bear)), &reg);
    flags_core(&with(&|c| c.sacrifice = Some(forest)), &reg,
        &format!("would sacrifice #{} which is not a creature the caster controls (CR 701.21a)", forest.0));
    flags_core(&with(&|c| c.sacrifice = Some(their_bear)), &reg,
        &format!("would sacrifice #{} which is not a creature the caster controls (CR 701.21a)", their_bear.0));
    flags_core(&with(&|c| c.sacrifice = Some(fodder)), &reg,
        &format!("would sacrifice #{} which is not a creature the caster controls (CR 701.21a)", fodder.0));
    flags_core(&with(&|c| c.sacrifice = Some(c.object_id)), &reg,
        "which is not a creature the caster controls (CR 701.21a)");
}

/// CR 601.2h/608.2m: the shapes a mid-cast prompt has that are healthy —
/// an exile cost asking for an exact number of cards, a resolving spell
/// that is on the stack, and one state-triggered ability in flight.
#[test]
fn a_healthy_cast_time_prompt_is_not_flagged() {
    let (mut state, reg) = base();
    let drake = castable_spell(&mut state, &reg, "Stitched Drake", P0);
    for _ in 0..2 {
        named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    }
    let after = mtg_engine::engine::submit_action(&state, &cast_action(drake, vec![]), &reg);
    let (min, max) = match &after.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { choice: ResolutionChoiceKind::ChooseExileFromGraveyard {
            min, max, .. }, .. }) => (*min, *max),
        other => panic!("test precondition: an exile-cost prompt, got {other:?}"),
    };
    assert_eq!(min, max, "Stitched Drake exiles exactly one creature card");
    clean_core(&after, &reg);

    // CR 608.2m: a spell whose resolution is paused is still on the stack.
    let (mut state, reg) = base();
    let bolt = castable_spell(&mut state, &reg, "Brimstone Volley", P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let mut s = cast_onto_stack(&state, &reg, bolt, vec![Target::Object(bear)]);
    // A resolving spell has left the stack list but is still in the stack
    // zone until CR 608.2m moves it.
    s.stack.retain(|e| e.as_spell() != Some(bolt));
    s.resolving_spell = Some(bolt);
    s.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: P0, source: bolt,
        choice: ResolutionChoiceKind::YesNo { description: "d".into(), source_card: bolt },
    });
    let s = as_collected(&s);
    quiet_core_about(&s, &reg, "resolving_spell");
    let mut gone = s.clone();
    gone.get_object_mut(bolt).unwrap().zone = Zone::Graveyard;
    flags_core(&gone, &reg, "is in Graveyard");
}

/// CR 601.2b/602.2: the prompt a payment is waiting on is *that* payment's
/// prompt. A mid-cast stash under somebody else's question is a cast that
/// will be finished by an answer given to a different spell.
#[test]
fn a_payment_waiting_under_the_wrong_prompt_is_flagged() {
    let (mut state, reg) = base();
    let play = castable_spell(&mut state, &reg, "Devil's Play", P0);
    add_mana(&mut state, P0, &[(ManaType::Red, 2)]);
    let state = cast_onto_stack(&state, &reg, play, vec![Target::Player(P1)]);
    let (funding_source, options) = match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { choice: ResolutionChoiceKind::ChooseXFunding {
            source_id, options, .. }, .. }) => (*source_id, options.clone()),
        other => panic!("test precondition: an X-funding prompt, got {other:?}"),
    };
    clean_core(&state, &reg);

    // The prompt names the stashed cast; a prompt for anything else is one
    // the answer would finish the wrong payment with.
    let mut s = state.clone();
    s.pending_spell_cast.as_mut().unwrap().object_id = play;
    if let Some(AwaitingAction::ResolutionChoice { choice: ResolutionChoiceKind::ChooseXFunding {
        source_id, .. }, source, .. }) = &mut s.awaiting_action {
        let decoy = ObjectId(funding_source.0 + 1000);
        *source_id = decoy;
        *source = decoy;
    }
    flags_core(&s, &reg, "but the stash is for");

    // The same for an activation: its stash and its funding prompt are one
    // ability's, not two.
    let mut s = state.clone();
    s.pending_spell_cast = None;
    s.pending_ability_effect = Some(mtg_engine::state::PendingAbilityEffect {
        source_id: funding_source,
        ability_index: 0,
        behavior_card_id: s.get_object(funding_source).unwrap().card_id,
        targets: vec![],
        description: "an ability".into(),
        activator: P0,
        target_requirement: None,
        unpaid: None,
    });
    if let Some(AwaitingAction::ResolutionChoice { choice: ResolutionChoiceKind::ChooseXFunding {
        is_ability, options: o, .. }, .. }) = &mut s.awaiting_action {
        *is_ability = true;
        *o = options;
    }
    clean_core(&s, &reg);

    let mut wrong = s.clone();
    wrong.pending_ability_effect.as_mut().unwrap().activator = P1;
    flags_core(&wrong, &reg, "but the pending prompt is for something else");

    let mut wrong = s.clone();
    if let Some(AwaitingAction::ResolutionChoice { choice: ResolutionChoiceKind::ChooseXFunding {
        is_ability, .. }, .. }) = &mut wrong.awaiting_action {
        *is_ability = false;
    }
    flags_core(&wrong, &reg, "but the pending prompt is for something else");
}

/// The healthy shapes the stack checker polices, which a clause that fires
/// on everything would flag: an instant sitting above the sorcery it was
/// cast in response to (CR 307.1 is about sorceries, not instants), an
/// ability that really did sacrifice something, one state trigger in
/// flight, and an exile prompt asking for an exact number of cards.
#[test]
fn the_healthy_shapes_of_the_stack_are_not_flagged() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let sorcery = spell_in_hand(&mut state, &reg, "Rolling Temblor", P0);
    let instant = spell_in_hand(&mut state, &reg, "Brimstone Volley", P0);
    for id in [sorcery, instant] {
        state.get_object_mut(id).unwrap().zone = Zone::Stack;
    }
    state.get_object_mut(instant).unwrap().targets = vec![Target::Object(bear)];
    state.stack.push(StackEntry::Spell(sorcery));
    state.stack.push(StackEntry::Spell(instant));
    clean_core(&state, &reg);

    // An activated ability that paid by sacrificing a creature remembers
    // both the creature and its toughness (morbid, Brimstone Volley's
    // "that creature's toughness" family).
    let mut s = state.clone();
    s.stack.push(StackEntry::Ability {
        source_id: bear,
        ability_index: 0,
        activator: P0,
        targets: vec![],
        target_requirement: None,
        behavior_card_id: s.get_object(bear).unwrap().card_id,
        sacrificed: Some(bear),
        sacrificed_toughness: Some(2),
        x_value: None,
        loyalty: false,
    });
    clean_core(&s, &reg);

    // CR 603.8: one state-triggered ability in flight is the normal case;
    // two of the same is the leak.
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().state_trigger_on_stack = true;
    let bear_card = s.get_object(bear).unwrap().card_id;
    let state_trigger = || StackEntry::Trigger(mtg_engine::triggers::PendingTrigger::new(
        mtg_engine::triggers::TriggerSource::new(bear, bear_card, P0, "t"),
        mtg_engine::triggers::TriggerEvent::StateTriggered,
    ));
    s.stack.push(state_trigger());
    clean_core(&s, &reg);
    s.stack.push(state_trigger());
    flags_core(&s, &reg, &format!("#{} has 2 state-triggered abilities in flight (CR 603.8)", bear.0));
}

// ── prompts ──────────────────────────────────────────────────────────────

#[test]
fn turn_based_action_prompts_are_checked() {
    let (state, reg) = base();

    let mut s = state.clone();
    s.step = Step::DeclareAttackers;
    s.awaiting_action = Some(AwaitingAction::DeclareAttackers);
    s.priority_player = Some(P0);
    clean(&s, &reg);
    s.combat = Some(mtg_engine::state::CombatState::new());
    flags_core(&s, &reg, "attackers prompt with combat state already present");
    s.combat = None;
    s.step = Step::PrecombatMain;
    flags_core(&s, &reg, "attackers prompt in PrecombatMain");
    s.step = Step::DeclareAttackers;
    s.stack.push(StackEntry::Spell(ObjectId(999)));
    flags_core(&s, &reg, "attackers prompt with 1 entries on the stack");

    let mut s = state.clone();
    s.step = Step::DeclareBlockers;
    s.awaiting_action = Some(AwaitingAction::DeclareBlockers { defending_player: P0 });
    s.priority_player = Some(P1);
    flags_core(&s, &reg, "for p0 who is not the defending player");
    s.awaiting_action = Some(AwaitingAction::DeclareBlockers { defending_player: P1 });
    flags_core(&s, &reg, "blockers prompt with no combat");

    let mut s = state.clone();
    s.step = Step::Cleanup;
    s.priority_player = Some(P0);
    s.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P0, discard_count: 2 });
    flags_core(&s, &reg, "asks for 2 discards from a hand of 0");

    let mut s = state.clone();
    s.awaiting_action = Some(AwaitingAction::MulliganDecision { player: P0 });
    flags_core(&s, &reg, "mulligan phase on turn 3");
}

#[test]
fn choice_prompts_offer_real_things() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let card = spell_in_hand(&mut state, &reg, "Forest", P1);
    let prompt = |choice: ResolutionChoiceKind| AwaitingAction::ResolutionChoice { player: P0, source: bear, choice };

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(ResolutionChoiceKind::ChooseCardFromHand {
        description: "d".into(), player: P0, cards: vec![card], discard_immediately: true, remaining: 1 }));
    flags_core(&s, &reg, "which is not in p0's hand");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(ResolutionChoiceKind::ChooseFromLibrary {
        description: "d".into(), options: vec![card], searcher: P1, source_id: bear, destination: Zone::Hand, tapped: false }));
    flags_core(&s, &reg, "which is not in p1's library (CR 701.23a)");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(ResolutionChoiceKind::ChoosePile {
        description: "d".into(), pile_1: vec![bear], pile_2: vec![bear], source_id: bear }));
    flags_core(&s, &reg, "is in both piles (CR 700.3a)");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(ResolutionChoiceKind::ChooseTarget {
        description: "d".into(), options: vec![Target::Object(bear)], optional: false,
        effect: PendingEffect::LegendRuleKeep { player: P0, legend_name: "Grizzly Bears".into() } }));
    flags_core(&s, &reg, "but the duplicate group is");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(ResolutionChoiceKind::ChooseTriggerOrder {
        description: "d".into(), options: vec!["a".into(), "b".into()], ap_queue: true, indices: vec![0, 5], details: vec![] }));
    flags_core(&s, &reg, "index 0 is past the queue of 0");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(ResolutionChoiceKind::ChooseTarget {
        description: "d".into(), options: vec![Target::Object(bear), Target::Object(bear)], optional: false,
        effect: PendingEffect::AttachTargetToPendingTrigger }));
    flags_core(&s, &reg, "trigger-target prompt with no queued trigger");
    flags_core(&s, &reg, "offers Object(ObjectId(");
}

// ── turn ─────────────────────────────────────────────────────────────────

#[test]
fn turn_and_result_bookkeeping_is_checked() {
    let (mut state, reg) = base();
    named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    clean(&state, &reg);

    let mut s = state.clone();
    s.is_first_turn = true;
    flags_core(&s, &reg, "is_first_turn=true on turn 3");
    let mut s = state.clone();
    s.step = Step::Untap;
    flags_core(&s, &reg, "holds priority in the untap step (CR 502.4)");
    let mut s = state.clone();
    s.get_player_mut(P1).lost = true;
    flags_core(&s, &reg, "lost=true but loss_reason=None");
    let mut s = state.clone();
    s.get_player_mut(P0).land_plays_remaining = 2;
    flags_core(&s, &reg, "has 2 land plays remaining");

    let mut s = state.clone();
    s.consecutive_passes = 2;
    flags_settled(&s, &reg, "(CR 117.4)");
    let mut s = state.clone();
    s.get_player_mut(P1).lost = true;
    s.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::Conceded);
    flags_settled(&s, &reg, "has lost but the game has no result (CR 104.2a)");
    s.result = Some(mtg_engine::state::GameResult::Winner(P1));
    flags_settled(&s, &reg, "p1 is the winner but the loss flags say otherwise");
    s.result = Some(mtg_engine::state::GameResult::Winner(P0));
    s.priority_player = Some(P1);
    flags_settled(&s, &reg, "p1 holds priority after losing");
}

#[test]
fn combat_bookkeeping_is_step_gated_and_names_creatures() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let land = named_permanent(&mut state, &reg, "Forest", P0);
    let lili = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);

    let mut s = state.clone();
    s.combat_damage_step_pending = true;
    flags_settled(&s, &reg, "second combat damage step pending in PrecombatMain (CR 510.4)");

    let mut s = state.clone();
    s.step = Step::DeclareBlockers;
    s.combat = Some(mtg_engine::state::CombatState::new());
    flags_settled(&s, &reg, "reached without attackers declared (CR 508.8)");

    let mut s = state.clone();
    s.step = Step::DeclareBlockers;
    let mut c = mtg_engine::state::CombatState::new();
    c.any_attackers_declared = true;
    c.attackers.insert(land, P1);
    c.attackers.insert(bear, P0);
    c.blocker_assignments.insert(bear, vec![]);
    c.planeswalker_defenders.insert(bear, lili);
    s.combat = Some(c);
    flags_settled(&s, &reg, "not a creature but still in combat (CR 506.4)");
    flags_settled(&s, &reg, "attacks p0, not the defending player p1 (CR 506.2)");
    flags_settled(&s, &reg, "which is not a planeswalker of the defending player");

    // CR 506.2, the other direction: a planeswalker the DEFENDING player
    // controls is a legal thing to attack, and saying so is the half a
    // corruption test cannot see. Every case above has Liliana on the
    // attacker's own side, so a clause that flagged every planeswalker
    // defender — or none — reads the same from there.
    let mut s = state.clone();
    s.step = Step::DeclareBlockers;
    let theirs = named_permanent(&mut s, &reg, "Garruk Relentless", P1);
    let mut c = mtg_engine::state::CombatState::new();
    c.any_attackers_declared = true;
    c.attackers.insert(bear, P1);
    c.blocker_assignments.insert(bear, vec![]);
    c.planeswalker_defenders.insert(bear, theirs);
    s.combat = Some(c);
    clean(&s, &reg);

    // And the clause only speaks about a planeswalker that is still there.
    // One that died mid-combat is off the battlefield, and CR 506.4 removes
    // it from combat rather than making the attack illegal — so an attacker
    // still pointed at it is not a violation of this rule.
    let mut gone = s.clone();
    gone.move_object(theirs, Zone::Graveyard, &reg);
    clean(&gone, &reg);
}

// ── events ───────────────────────────────────────────────────────────────

#[test]
fn cast_and_land_events_are_checked() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let bolt = castable_spell(&mut state, &reg, "Moment of Heroism", P0);
    let cast = cast_onto_stack(&state, &reg, bolt, vec![Target::Object(bear)]);
    assert!(cast.events.iter().any(|e| matches!(e, GameEvent::SpellCast { .. })));
    clean_core(&cast, &reg);

    let mut s = cast.clone();
    s.stack.clear();
    flags_core(&s, &reg, "is on no stack entry (CR 112.1)");
    let mut s = cast.clone();
    s.priority_player = Some(P1);
    flags_core(&s, &reg, "cast a spell but priority is Some(PlayerId(1)) (CR 117.3c)");
    let mut s = cast.clone();
    s.num_spells_cast_this_turn.insert(P0, 0);
    flags_core(&s, &reg, "but the turn's count says 0");
    let mut s = cast.clone();
    grant_keyword(&mut s, bear, Keyword::Hexproof);
    s.get_object_mut(bear).unwrap().controller = P1;
    flags_core(&s, &reg, "which has hexproof from p0 (CR 702.11b)");

    let mut s = state.clone();
    let land = named_permanent(&mut s, &reg, "Forest", P0);
    s.events = vec![GameEvent::EnteredBattlefield { object: land, controller: P0 }, GameEvent::LandPlayed { player: P0, object: land }];
    s.get_player_mut(P0).land_plays_remaining = 0;
    clean_core(&s, &reg);
    s.get_player_mut(P0).land_plays_remaining = 1;
    flags_core(&s, &reg, "the land drop was not spent (CR 305.2)");
    s.active_player = P1;
    s.priority_player = Some(P1);
    flags_core(&s, &reg, "on p1's turn in PrecombatMain (CR 305.1)");
}

#[test]
fn combat_declaration_events_are_checked() {
    let (mut state, reg) = base();
    let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let blocker = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.step = Step::DeclareAttackers;
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    state.priority_player = Some(P0);
    assert!(state.events.iter().any(|e| matches!(e, GameEvent::AttackersDeclared { .. })));
    clean(&state, &reg);

    let mut s = state.clone();
    s.get_object_mut(attacker).unwrap().tapped = false;
    flags_core(&s, &reg, "was not tapped by attacking (CR 508.1f)");
    let mut s = state.clone();
    grant_keyword(&mut s, attacker, Keyword::Defender);
    flags_core(&s, &reg, "has defender (CR 702.3b)");
    let mut s = state.clone();
    s.get_object_mut(attacker).unwrap().summoning_sick = true;
    flags_core(&s, &reg, "is summoning sick without haste (CR 302.6)");
    let mut s = state.clone();
    s.combat.as_mut().unwrap().attackers.insert(blocker, P1);
    s.get_object_mut(blocker).unwrap().controller = P0;
    flags_core(&s, &reg, "is attacking but was not declared (CR 508.1)");

    let mut blocked = state.clone();
    blocked.step = Step::DeclareBlockers;
    submit_declare_blockers(&mut blocked, P1, &[(blocker, attacker)], &reg);
    blocked.priority_player = Some(P0);
    assert!(blocked.events.iter().any(|e| matches!(e, GameEvent::BlockersDeclared { .. })));
    clean(&blocked, &reg);

    let mut s = blocked.clone();
    grant_keyword(&mut s, attacker, Keyword::Flying);
    flags_core(&s, &reg, "a flier blocked by neither flying nor reach (CR 702.9b)");
    let mut s = blocked.clone();
    grant_keyword(&mut s, attacker, Keyword::Menace);
    flags_core(&s, &reg, "has menace but was blocked by 1 creature (CR 702.111b)");
    let mut s = blocked.clone();
    s.get_object_mut(blocker).unwrap().tapped = true;
    flags_core(&s, &reg, "blocker is tapped (CR 509.1a)");
    let mut s = blocked.clone();
    s.combat.as_mut().unwrap().blocked_attackers.clear();
    flags_core(&s, &reg, "is not recorded in combat (CR 509.1h)");
}

#[test]
fn damage_events_are_checked() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let victim = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.step = Step::CombatDamage;
    let mut c = mtg_engine::state::CombatState::new();
    c.any_attackers_declared = true;
    c.attackers.insert(bear, P1);
    c.blocker_assignments.insert(bear, vec![victim]);
    c.blocked_attackers.insert(bear);
    state.combat = Some(c);
    // Non-lethal damage: the settled state must be a live one.
    state.get_object_mut(victim).unwrap().damage_marked = 1;
    state.get_object_mut(victim).unwrap().damaged_by.push(bear);
    state.events = vec![GameEvent::CombatDamageDealt { source: bear, target: DamageTarget::Object(victim), amount: 1 }];
    clean(&state, &reg);

    let mut s = state.clone();
    s.events = vec![GameEvent::CombatDamageDealt { source: bear, target: DamageTarget::Object(victim), amount: 0 }];
    flags_core(&s, &reg, "a zero-damage event (CR 120.8)");
    let mut s = state.clone();
    s.events = vec![GameEvent::CombatDamageDealt { source: bear, target: DamageTarget::Player(P1), amount: 2 }];
    flags_core(&s, &reg, "no matching life loss for p1 (CR 120.3a)");
    flags_core(&s, &reg, "a blocked attacker without trample reached the player (CR 510.1c)");
    let mut s = state.clone();
    grant_keyword(&mut s, bear, Keyword::Lifelink);
    flags_core(&s, &reg, "lifelink but no life gain for its controller (CR 702.15b)");
    let mut s = state.clone();
    s.step = Step::PrecombatMain;
    s.combat = None;
    flags_core(&s, &reg, "combat damage dealt in PrecombatMain (CR 510.2)");
    let mut s = state.clone();
    s.combat_damage_step_pending = true;
    flags_core(&s, &reg, "dealt in the first-strike step without first strike (CR 510.4)");
    let mut s = state.clone();
    s.events = vec![GameEvent::CombatDamageDealt { source: victim, target: DamageTarget::Player(P1), amount: 2 },
                    GameEvent::LifeChanged { player: P1, old: 20, new_life: 18 }];
    s.events.swap(0, 1);
    flags_core(&s, &reg, "a blocker of #");
}

#[test]
fn zone_change_and_tap_events_are_checked() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let wolf = state.create_token_with_subtypes("", P0, 2, 2, vec![Color::Green], vec![CardType::Creature],
        vec![], vec!["Wolf".into()], &reg)[0];
    clean(&state, &reg);

    let mut s = state.clone();
    s.events = vec![GameEvent::CreatureDied { object: bear, name: "Grizzly Bears".into(), card_id: s.get_object(bear).unwrap().card_id, controller: P0,
        damaged_by: vec![], last_known_toughness: 2, is_token: false, subtypes: vec![] }];
    flags_core(&s, &reg, "without leaving the battlefield afterwards (CR 700.4)");
    let mut s = state.clone();
    s.move_object(bear, Zone::Graveyard, &reg);
    s.events.retain(|e| !matches!(e, GameEvent::CreatureDied { .. }));
    flags_core(&s, &reg, "went to the graveyard without dying (CR 700.4)");

    let mut s = state.clone();
    s.events = vec![GameEvent::LeftBattlefield { object: wolf, to: Zone::Graveyard, last_controller: P0 },
                    GameEvent::EnteredBattlefield { object: wolf, controller: P0 }];
    flags_core(&s, &reg, "changed zones again after leaving the battlefield (CR 111.8)");

    let mut s = state.clone();
    s.events = vec![GameEvent::Tapped { object: bear }, GameEvent::Tapped { object: bear }];
    flags_core(&s, &reg, "was tapped twice in a row (CR 701.26)");
    let mut s = state.clone();
    s.events = vec![GameEvent::Tapped { object: bear }];
    flags_core(&s, &reg, "was tapped but tapped=false now");

    let mut s = state.clone();
    let card = spell_in_hand(&mut s, &reg, "Forest", P1);
    s.events = vec![GameEvent::CardDrawn { player: P0, object: card }];
    flags_core(&s, &reg, "not that player's card out of their library (CR 121.1)");

    let mut s = state.clone();
    let lili = named_permanent(&mut s, &reg, "Liliana of the Veil", P0);
    s.get_object_mut(lili).unwrap().counters.insert(CounterType::Loyalty, 1);
    s.events = vec![GameEvent::EnteredBattlefield { object: lili, controller: P0 }];
    flags_core(&s, &reg, "entered with 1 loyalty in Battlefield, expected 3 (CR 306.5b)");
}

#[test]
fn step_and_turn_start_windows_are_checked() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.events = vec![GameEvent::StepStarted { step: Step::PrecombatMain }];
    clean(&state, &reg);

    let mut s = state.clone();
    add_mana(&mut s, P0, &[(ManaType::Green, 1)]);
    flags_core(&s, &reg, "has mana floating across a step boundary (CR 500.5)");
    let mut s = state.clone();
    s.events = vec![GameEvent::StepStarted { step: Step::Draw }];
    s.step = Step::Draw;
    flags_core(&s, &reg, "the draw step drew [] for p0 (CR 504.1)");
    let mut s = state.clone();
    s.events = vec![GameEvent::StepStarted { step: Step::EndStep }];
    flags_core(&s, &reg, "the last step to start was EndStep but the state is in PrecombatMain");

    let mut s = state.clone();
    s.events = vec![GameEvent::TurnStarted { player: P0, turn: 3 }];
    clean(&s, &reg);
    s.get_object_mut(bear).unwrap().damage_marked = 1;
    s.get_object_mut(bear).unwrap().damaged_by.push(bear);
    flags_core(&s, &reg, "carries damage from last turn (CR 514.2)");
    let mut s = state.clone();
    s.events = vec![GameEvent::TurnStarted { player: P0, turn: 3 }];
    s.until_end_of_turn.push(TemporaryEffect::ModifyPT { target: bear, power_mod: 1, toughness_mod: 1 });
    flags_core(&s, &reg, "until-end-of-turn effects survive (CR 514.2)");
    let mut s = state.clone();
    s.events = vec![GameEvent::TurnStarted { player: P0, turn: 3 }];
    s.get_object_mut(bear).unwrap().tapped = true;
    flags_core(&s, &reg, "did not untap (CR 502.3)");
    let mut s = state.clone();
    s.events = vec![GameEvent::TurnStarted { player: P0, turn: 3 }];
    for _ in 0..8 {
        spell_in_hand(&mut s, &reg, "Forest", P1);
    }
    flags_core(&s, &reg, "holds 8 cards after their cleanup (CR 514.1)");
}

// ── effects and permanents (settled) ─────────────────────────────────────

#[test]
fn effect_records_point_at_battlefield_permanents() {
    let (mut state, reg) = base();
    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let vampire = named_permanent(&mut state, &reg, "Markov Patrician", P1);
    state.gain_control_while_source_controlled(vampire, olivia, &reg);
    clean(&state, &reg);

    let mut s = state.clone();
    s.until_end_of_turn.push(TemporaryEffect::GrantKeyword { target: ObjectId(4242), keyword: Keyword::Flying });
    flags_settled(&s, &reg, "which is not on the battlefield (CR 400.7)");
    let mut s = state.clone();
    s.get_object_mut(vampire).unwrap().zone = Zone::Graveyard;
    s.get_object_mut(vampire).unwrap().controller = P1;
    flags_settled(&s, &reg, "survives the object leaving the battlefield (CR 400.7)");
    let mut s = state.clone();
    s.get_object_mut(olivia).unwrap().controller = P1;
    flags_settled(&s, &reg, "leaving p0's control (CR 611.2b)");
}

#[test]
fn attachment_kinds_match_their_enchant_abilities() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let land = named_permanent(&mut state, &reg, "Forest", P0);
    let curse = attach_curse_to_player(&mut state, &reg, "Curse of the Nightly Hunt", P1, P0);
    clean(&state, &reg);

    let mut s = state.clone();
    s.get_object_mut(curse).unwrap().attached_to_player = None;
    s.get_object_mut(curse).unwrap().attached_to = Some(bear);
    flags_settled(&s, &reg, "enchants players but is attached to an object (CR 702.5d)");

    let mut s = state.clone();
    let aura = named_permanent(&mut s, &reg, "Pacifism", P0);
    s.get_object_mut(aura).unwrap().attached_to = Some(land);
    flags_settled(&s, &reg, "enchants creatures but is attached to non-creature");

    let mut s = state.clone();
    let blade = named_permanent(&mut s, &reg, "Trepanation Blade", P0);
    s.get_object_mut(blade).unwrap().attached_to_player = Some(P1);
    flags_core(&s, &reg, "attached to a player but is no Aura");
}

// ── trigger collection and cast-time prompts ─────────────────────────────

/// CR 603.3: at a decision point every event has been scanned and every
/// SBA-queued trigger bucketed. A checker that saw the state after
/// `submit_action` but before the loop's collector would be looking at
/// exactly the window a missed trigger hides in.
#[test]
fn unscanned_events_and_unbucketed_triggers_are_flagged() {
    let (mut state, reg) = base();
    let src = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.events.push(GameEvent::TurnStarted { player: P0, turn: 3 });
    state.trigger_event_index = state.events.len();
    clean(&state, &reg);

    let mut s = state.clone();
    s.trigger_event_index = 0;
    flags_core(&s, &reg, "0 of 1 events scanned for triggers at a decision point (CR 603.3)");

    let mut s = state.clone();
    let card_id = s.get_object(src).unwrap().card_id;
    s.pending_triggers.push(mtg_engine::triggers::PendingTrigger::new(
        mtg_engine::triggers::TriggerSource::new(src, card_id, P0, "t"),
        mtg_engine::triggers::TriggerEvent::StateTriggered,
    ));
    s.priority_player = None;
    flags_core(&s, &reg, "1 trigger(s) collected but not bucketed at a decision point (CR 603.3b)");
}

/// CR 601.2/602.2: the player casting or activating holds priority through
/// the funding prompt, and the prompt offers exactly what could fund X.
#[test]
fn cast_time_prompts_keep_priority_and_offer_a_real_ceiling() {
    let (mut state, reg) = base();
    let play = castable_spell(&mut state, &reg, "Devil's Play", P0);
    add_mana(&mut state, P0, &[(ManaType::Red, 2)]);
    let state = cast_onto_stack(&state, &reg, play, vec![Target::Player(P1)]);
    clean_core(&state, &reg);

    let mut s = state.clone();
    s.priority_player = Some(P1);
    flags_core(&s, &reg, "but priority is Some(PlayerId(1)) (CR 601.2)");

    let mut s = state.clone();
    if let Some(AwaitingAction::ResolutionChoice { choice: ResolutionChoiceKind::ChooseXFunding { options, .. }, .. }) =
        &mut s.awaiting_action
    {
        options.max_x = 0;
    }
    flags_core(&s, &reg, "with nothing to fund");
    flags_core(&s, &reg, "but a ceiling of 0");

    // An activated X ability: the prompt is built from the live pool.
    let (mut state, reg) = base();
    let run = named_permanent(&mut state, &reg, "Kessig Wolf Run", P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    add_mana(&mut state, P0, &[(ManaType::Red, 1), (ManaType::Green, 2)]);
    let state = activate_onto_stack(&state, &reg, run, Some(Target::Object(bear)));
    assert!(state.pending_ability_effect.is_some(), "test precondition: X activation stashed");
    clean_core(&state, &reg);

    let mut s = state.clone();
    s.priority_player = None;
    flags_core(&s, &reg, "but priority is None (CR 602.2)");

    let mut s = state.clone();
    s.get_player_mut(P0).mana_pool.mana.clear();
    flags_core(&s, &reg, "offers");

    // Nothing is paid while the prompt is up (CR 601.2b before 601.2h via
    // 602.2b, issue #290) — a source already tapped for its own {T} cost is
    // an activation that charged before it asked.
    let mut s = state.clone();
    s.tap(run);
    flags_core(&s, &reg, "{T} cost is already paid");
}

// ── the card-code contract ───────────────────────────────────────────────

#[test]
fn object_contract_violations_are_flagged() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let aura = named_permanent(&mut state, &reg, "Pacifism", P0);
    state.get_object_mut(aura).unwrap().attached_to = Some(bear);
    clean(&state, &reg);

    // CR 614.12b: an entry waiting on an enters-as-a-copy choice has not
    // happened, so the permanent cannot be on the battlefield meanwhile.
    let mut s = state.clone();
    s.pending_entry_choices.push(bear);
    flags_core(&s, &reg, "still queued for its enters-as-copy choice (CR 614.12b)");
    // Queued while still in the zone it is coming from is the normal state.
    s.move_object(bear, Zone::Hand, &reg);
    assert!(!check_core(&s, &reg).iter().any(|m| m.contains("614.12b")), "{:?}", check_core(&s, &reg));

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().keywords.push(Keyword::Flying);
    flags_core(&s, &reg, "carries Flying which its face does not print (CR 707.2)");
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().card_types.push(CardType::Artifact);
    flags_core(&s, &reg, "carries type Artifact which its face does not print (CR 707.2)");

    let mut s = state.clone();
    s.get_object_mut(aura).unwrap().power = Some(1);
    flags_core(&s, &reg, "has power Some(1) but the card prints None (CR 208.1)");

    let mut s = state.clone();
    s.get_object_mut(aura).unwrap().regeneration_shields = 1;
    flags_core(&s, &reg, "regeneration shield(s) but is no creature (CR 701.15)");

    let mut s = state.clone();
    s.get_object_mut(aura).unwrap().counters.insert(CounterType::PlusOnePlusOne, 1);
    flags_core(&s, &reg, "+1/+1 counter(s) but is no creature");
    let mut s = state.clone();
    s.move_object(bear, Zone::Graveyard, &reg);
    s.get_object_mut(bear).unwrap().counters.insert(CounterType::Slime, 2);
    flags_core(&s, &reg, "has 2 Slime counter(s) in Graveyard (CR 122.1)");

    let mut s = state.clone();
    let lili = named_permanent(&mut s, &reg, "Liliana of the Veil", P1);
    s.get_object_mut(lili).unwrap().abilities_activated_this_turn.insert(999);
    flags_core(&s, &reg, "used a loyalty ability this turn but p1 is not the active player (CR 606.3)");

    let mut s = state.clone();
    s.get_object_mut(aura).unwrap().attached_to = Some(ObjectId(424_242));
    flags_core(&s, &reg, "is attached to #424242 which does not exist");
}

#[test]
fn delayed_effect_records_name_the_right_kind_of_object() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let aura = named_permanent(&mut state, &reg, "Pacifism", P0);
    state.get_object_mut(aura).unwrap().attached_to = Some(bear);
    let spell = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);
    state.get_object_mut(spell).unwrap().zone = Zone::Graveyard;
    let cost = state.face_data(spell, &reg).unwrap().cost.unwrap();
    state.until_end_of_turn.push(TemporaryEffect::GrantFlashback { target: spell, cost: cost.clone() });
    state.until_end_of_turn.push(TemporaryEffect::ModifyPT { target: bear, power_mod: 1, toughness_mod: 1 });
    clean(&state, &reg);

    let mut s = state.clone();
    s.until_end_of_turn.push(TemporaryEffect::ModifyPT { target: aura, power_mod: 1, toughness_mod: 1 });
    flags_core(&s, &reg, "until-end-of-turn effect on #");
    flags_core(&s, &reg, "which is no creature");

    let mut s = state.clone();
    s.until_end_of_turn.push(TemporaryEffect::GrantFlashback { target: spell, cost: ManaCost::free() });
    flags_core(&s, &reg, "but its cost is");
    let mut s = state.clone();
    let dead = named_permanent(&mut s, &reg, "Grizzly Bears", P1);
    s.move_object(dead, Zone::Graveyard, &reg);
    s.until_end_of_turn.push(TemporaryEffect::GrantFlashback { target: dead, cost: cost.clone() });
    flags_core(&s, &reg, "which is no instant or sorcery (CR 702.34a)");

    let mut s = state.clone();
    s.step = Step::DeclareBlockers;
    s.combat = Some(mtg_engine::state::CombatState::default());
    s.combat.as_mut().unwrap().any_attackers_declared = true;
    s.end_of_combat_exiles.push(mtg_engine::state::EndOfCombatExileEntry {
        target_id: bear, source_id: aura, source_card_id: s.get_object(aura).unwrap().card_id,
        controller: P0, description: String::new() });
    flags_core(&s, &reg, "delayed exile of #");
    flags_core(&s, &reg, "which is a card, not a token");
}

#[test]
fn prompt_sources_and_option_zones_are_checked() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let dead = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(dead, Zone::Graveyard, &reg);
    let card = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);
    let prompt = |source: ObjectId, options: Vec<Target>, effect: PendingEffect| AwaitingAction::ResolutionChoice {
        player: P0, source, choice: ResolutionChoiceKind::ChooseTarget { description: String::new(), options, optional: false, effect } };

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(bear, vec![Target::Object(theirs)], PendingEffect::CardEffect { source_id: theirs, key: "k".into() }));
    flags_core(&s, &reg, "carries a choice for #");
    flags_core(&s, &reg, "(CR 608.2)");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(bear, vec![Target::Object(dead)], PendingEffect::Destroy { source_name: "x".into() }));
    flags_core(&s, &reg, "destroy prompt offers #");
    flags_core(&s, &reg, "in Graveyard (CR 608.2d)");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(bear, vec![Target::Object(theirs)], PendingEffect::SacrificeCreature { source_name: "x".into() }));
    flags_core(&s, &reg, "which p0 does not control (CR 701.17a)");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(bear, vec![Target::Object(bear)], PendingEffect::TokenAttacks {
        token_id: bear, remaining: vec![], source_id: bear }));
    flags_core(&s, &reg, "which is no opponent or opposing planeswalker (CR 508.4b)");

    let mut s = state.clone();
    s.awaiting_action = Some(AwaitingAction::ResolutionChoice { player: P0, source: bear,
        choice: ResolutionChoiceKind::ChooseFromLookedAt { description: String::new(), looked_at: vec![card] } });
    flags_core(&s, &reg, "looked-at prompt offers #");
    flags_core(&s, &reg, "which is not in p0's library");
}

#[test]
fn verb_events_leave_the_object_where_the_verb_puts_it() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let card = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);
    state.events.push(GameEvent::CardDrawn { player: P0, object: card });
    clean(&state, &reg);

    let mut s = state.clone();
    s.get_object_mut(card).unwrap().zone = Zone::Graveyard;
    flags_core(&s, &reg, "but it is in Graveyard (CR 121.1)");

    let mut s = state.clone();
    s.events = vec![GameEvent::Discarded { player: P0, object: card }];
    flags_core(&s, &reg, "but it is in Hand (CR 701.8a)");

    let mut s = state.clone();
    s.events = vec![GameEvent::PlayerLost { player: P1, reason: mtg_engine::events::LossReason::Conceded }];
    flags_core(&s, &reg, "PlayerLost p1 without the game ending afterwards (CR 104.2a)");

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().summoning_sick = false;
    s.events = vec![GameEvent::EnteredBattlefield { object: bear, controller: P0 }];
    flags_core(&s, &reg, "entered this action but is not summoning sick (CR 302.6)");

    let mut s = state.clone();
    s.creature_died_this_turn = false;
    s.events = vec![GameEvent::CreatureDied { object: bear, name: "Grizzly Bears".into(), card_id: s.get_object(bear).unwrap().card_id, controller: P0,
        damaged_by: vec![], last_known_toughness: 2, is_token: false, subtypes: vec![] }];
    flags_core(&s, &reg, "but creature_died_this_turn is false");
}

// ── transitions ──────────────────────────────────────────────────────────

use mtg_engine::invariants::check_transition;
use mtg_engine::actions::Action;

#[track_caller]
fn flags_transition(prev: &GameState, action: Option<&Action>, cur: &GameState, reg: &CardRegistry, needle: &str) {
    let v = check_transition(prev, action, cur, reg);
    assert!(v.iter().any(|m| m.contains(needle)), "expected a transition violation containing {needle:?}, got: {v:?}");
}

#[track_caller]
fn quiet_transition_about(prev: &GameState, action: Option<&Action>, cur: &GameState,
                          reg: &CardRegistry, needle: &str) {
    let v = check_transition(prev, action, cur, reg);
    assert!(!v.iter().any(|m| m.contains(needle)),
        "expected no transition violation containing {needle:?}, got: {v:?}");
}

#[track_caller]
fn clean_transition(prev: &GameState, action: Option<&Action>, cur: &GameState, reg: &CardRegistry) {
    assert_eq!(check_transition(prev, action, cur, reg), Vec::<String>::new());
}

/// A mid-payment cast of `card`, the way `cast_spell` stashes one.
fn stash(state: &GameState, card: ObjectId) -> mtg_engine::state::PendingSpellCast {
    mtg_engine::state::PendingSpellCast {
        object_id: card,
        player: state.get_object(card).unwrap().controller,
        card_id: state.get_object(card).unwrap().card_id,
        targets: vec![], sacrifice: None, exile_ids: vec![], exile_count: None,
        tap_plan: vec![], alternative_cost: None,
        non_x_mana_cost: ManaCost::new(vec![]), is_flashback: false,
        cast_from_graveyard: false,
    }
}

/// The next decision point, one action later, with nothing having happened.
fn next(prev: &GameState) -> GameState {
    let mut cur = prev.clone();
    cur.submit_seq = prev.submit_seq + 1;
    cur.events.clear();
    cur
}

/// CR 500.1/501-514: the steps and turns a transition walks through are a
/// legal succession, and the events say so. This is the checker's only look
/// at the shape of a turn: a step skipped, repeated where the rules do not
/// allow it, or handed to the wrong player is reported here or nowhere.
#[test]
fn the_step_and_turn_succession_of_a_transition_is_checked() {
    let (prev, reg) = base();
    // `base()` sits in a precombat main phase on turn 3.
    let stepped = |from: Step, to: Step, events: Vec<GameEvent>| {
        let mut p = prev.clone();
        p.step = from;
        let mut c = next(&p);
        c.step = to;
        c.events = events;
        (p, c)
    };
    let started = |s: Step| GameEvent::StepStarted { step: s };

    // The ordinary case: one step to the next, announced.
    let (p, c) = stepped(Step::PrecombatMain, Step::BeginCombat, vec![started(Step::BeginCombat)]);
    clean_transition(&p, None, &c, &reg);

    // A step out of order (CR 500.1).
    let (p, c) = stepped(Step::PrecombatMain, Step::EndStep, vec![started(Step::EndStep)]);
    flags_transition(&p, None, &c, &reg, "StepStarted EndStep after PrecombatMain (CR 500.1)");

    // Each succession the rules DO allow, which a narrowed clause would
    // start reporting: the first turn skips its draw step (CR 103.7a); an
    // attack nobody declared skips to end of combat (CR 508.8); first
    // strike gives two combat damage steps (CR 510.5); a cleanup that
    // opened a priority window is followed by another (CR 514.3a).
    let mut p = prev.clone();
    p.step = Step::Upkeep;
    p.turn_number = 1;
    p.is_first_turn = true;
    let mut c = next(&p);
    c.step = Step::PrecombatMain;
    c.events = vec![started(Step::PrecombatMain)];
    clean_transition(&p, None, &c, &reg);

    for (from, to) in [
        (Step::DeclareAttackers, Step::EndCombat),
        (Step::CombatDamage, Step::CombatDamage),
        (Step::Cleanup, Step::Cleanup),
    ] {
        let (p, c) = stepped(from, to, vec![started(to)]);
        clean_transition(&p, None, &c, &reg);
    }

    // Cleanup to untap, but only across a turn that started.
    let mut p = prev.clone();
    p.step = Step::Cleanup;
    let mut c = next(&p);
    c.step = Step::Untap;
    c.turn_number = prev.turn_number + 1;
    c.active_player = prev.opponent(prev.active_player);
    c.events = vec![
        GameEvent::TurnStarted { player: c.active_player, turn: c.turn_number },
        started(Step::Untap),
    ];
    clean_transition(&p, None, &c, &reg);
    // The same step change with no turn starting is not that exception.
    let (p2, mut c2) = stepped(Step::Cleanup, Step::Untap, vec![started(Step::Untap)]);
    c2.turn_number = p2.turn_number;
    flags_transition(&p2, None, &c2, &reg, "StepStarted Untap after Cleanup (CR 500.1)");

    // A turn that starts belongs to the other player, one higher, out of a
    // cleanup step.
    let turn_started = |turn: u32, player: PlayerId| {
        let mut p = prev.clone();
        p.step = Step::Cleanup;
        let mut c = next(&p);
        c.step = Step::Untap;
        c.turn_number = turn;
        c.active_player = player;
        c.events = vec![GameEvent::TurnStarted { player, turn }, started(Step::Untap)];
        (p, c)
    };
    let other = prev.opponent(prev.active_player);
    let (p, c) = turn_started(prev.turn_number + 1, other);
    clean_transition(&p, None, &c, &reg);
    let (p, c) = turn_started(prev.turn_number + 2, other);
    flags_transition(&p, None, &c, &reg, "TurnStarted");
    let (p, c) = turn_started(prev.turn_number + 1, prev.active_player);
    flags_transition(&p, None, &c, &reg, "TurnStarted");
    // And out of a cleanup step, not out of the middle of a turn.
    let mut p = prev.clone();
    let mut c = next(&p);
    c.turn_number = prev.turn_number + 1;
    c.active_player = other;
    c.step = Step::Untap;
    c.events = vec![GameEvent::TurnStarted { player: other, turn: c.turn_number }, started(Step::Untap)];
    flags_transition(&p, None, &c, &reg, "TurnStarted");
    let _ = &mut p;

    // CR 500.2/turn order: the active player changes with each turn, and
    // the count of TurnStarted events matches the counter's move.
    let (p, mut c) = turn_started(prev.turn_number + 1, other);
    c.active_player = prev.active_player;
    flags_transition(&p, None, &c, &reg, "active player");
    let (p, mut c) = turn_started(prev.turn_number + 1, other);
    c.events.retain(|e| !matches!(e, GameEvent::TurnStarted { .. }));
    flags_transition(&p, None, &c, &reg, "TurnStarted event(s) for a turn counter that moved by 1");

    // Each exception is a pair, not a licence for either half: leaving the
    // step it names for somewhere else, or arriving at the step it names
    // from somewhere else, is still out of order.
    for (from, to) in [
        (Step::Upkeep, Step::EndStep),
        (Step::DeclareAttackers, Step::EndStep),
        (Step::CombatDamage, Step::Untap),
        (Step::Cleanup, Step::DeclareBlockers),
        (Step::Draw, Step::EndCombat),
        (Step::BeginCombat, Step::CombatDamage),
    ] {
        let (p, c) = stepped(from, to, vec![started(to)]);
        flags_transition(&p, None, &c, &reg, &format!("StepStarted {to:?} after {from:?} (CR 500.1)"));
    }
    // Turn one's exception is turn one's: the same jump later is not it.
    let mut p = prev.clone();
    p.step = Step::Upkeep;
    p.turn_number = 5;
    let mut c = next(&p);
    c.step = Step::PrecombatMain;
    c.events = vec![started(Step::PrecombatMain)];
    flags_transition(&p, None, &c, &reg, "StepStarted PrecombatMain after Upkeep (CR 500.1)");

    // Turn 1 announced outside the opening hands is a turn out of order,
    // not a mulligan-phase exemption.
    let mut p = prev.clone();
    p.step = Step::Cleanup;
    let mut c = next(&p);
    c.step = Step::Untap;
    c.events = vec![
        GameEvent::TurnStarted { player: prev.opponent(prev.active_player), turn: 1 },
        started(Step::Untap),
    ];
    flags_transition(&p, None, &c, &reg, "TurnStarted {turn 1");

    // A step change with nothing announced, and an announcement that does
    // not end where the state is.
    let (p, c) = stepped(Step::PrecombatMain, Step::BeginCombat, vec![]);
    flags_transition(&p, None, &c, &reg, "with no StepStarted");
    let (p, c) = stepped(Step::PrecombatMain, Step::BeginCombat, vec![started(Step::DeclareAttackers)]);
    flags_transition(&p, None, &c, &reg, "names DeclareAttackers but the step is BeginCombat");
}

/// CR 108.3/707.2/104.3: what a transition may not do to an object's
/// identity or to a record that only moves one way.
#[test]
fn identity_and_monotone_edges_are_checked_one_at_a_time() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let geist = named_permanent(&mut prev, &reg, "Geist of Saint Traft", P0);
    let geist_card = prev.get_object(geist).unwrap().card_id;
    let bear_card = prev.get_object(bear).unwrap().card_id;

    // CR 707.2: an object's card changes only by becoming a copy, or by
    // ceasing to be one as it changes zones.
    let mut s = next(&prev);
    {
        let o = s.get_object_mut(bear).unwrap();
        o.card_id = geist_card;
        o.copy_grantor = Some(bear_card);
    }
    clean_transition(&prev, None, &s, &reg);
    // The same copy off the battlefield is not a copy any more.
    let mut s2 = s.clone();
    s2.get_object_mut(bear).unwrap().zone = Zone::Graveyard;
    s2.get_object_mut(bear).unwrap().zone_change_count += 1;
    s2.events.push(GameEvent::LeftBattlefield { object: bear, to: Zone::Graveyard, last_controller: P0 });
    flags_transition(&prev, None, &s2, &reg, "without a copy or a zone change");
    // A copy in the previous state loses the copied card by changing zones.
    let mut p = prev.clone();
    p.get_object_mut(bear).unwrap().copy_grantor = Some(bear_card);
    let mut s = next(&p);
    {
        let o = s.get_object_mut(bear).unwrap();
        o.card_id = geist_card;
        o.zone_change_count += 1;
        o.zone = Zone::Graveyard;
    }
    s.events.push(GameEvent::ObjectMoved { object: bear, from: Zone::Battlefield, to: Zone::Graveyard });
    s.events.push(GameEvent::LeftBattlefield { object: bear, to: Zone::Graveyard, last_controller: P0 });
    clean_transition(&p, None, &s, &reg);
    // A copy that stops being one without changing zones has no licence to
    // change its card. (Still a copy, still on the battlefield, is licence:
    // it re-copied something else.)
    let mut s = next(&p);
    s.get_object_mut(bear).unwrap().card_id = geist_card;
    quiet_transition_about(&p, None, &s, &reg, "without a copy or a zone change");
    let mut s = next(&p);
    {
        let o = s.get_object_mut(bear).unwrap();
        o.card_id = geist_card;
        o.copy_grantor = None;
    }
    flags_transition(&p, None, &s, &reg, "without a copy or a zone change");
    // And with no copy anywhere in the picture at all.
    let mut s = next(&prev);
    s.get_object_mut(bear).unwrap().card_id = geist_card;
    flags_transition(&prev, None, &s, &reg, "without a copy or a zone change");

    // A decision point the engine reached without an action of its own —
    // advancing a step — does not move `submit_seq`, and that is not a
    // counter going backwards.
    let mut s = prev.clone();
    s.events.clear();
    s.step = Step::BeginCombat;
    s.events.push(GameEvent::StepStarted { step: Step::BeginCombat });
    clean_transition(&prev, None, &s, &reg);
    let mut counted = prev.clone();
    counted.submit_seq = 5;
    let mut s = next(&counted);
    s.submit_seq = 4;
    flags_transition(&counted, None, &s, &reg, "submit_seq went back");

    // CR 508.1: an attack stamp is not forgotten — unless it is replaced by
    // one for the turn now being played.
    let mut p = prev.clone();
    p.get_object_mut(bear).unwrap().attacked_on_turn = Some(1);
    let mut s = next(&p);
    s.get_object_mut(bear).unwrap().attacked_on_turn = Some(p.turn_number);
    s.events.push(GameEvent::AttackersDeclared { attackers: vec![(bear, P1)] });
    clean_transition(&p, None, &s, &reg);
    let mut s = next(&p);
    s.get_object_mut(bear).unwrap().attacked_on_turn = None;
    flags_transition(&p, None, &s, &reg, "forgot attacking on turn 1");

    // CR 104.3: a loss is final, and keeps the reason it was given.
    let mut p = prev.clone();
    p.get_player_mut(P1).lost = true;
    p.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::Conceded);
    p.result = Some(mtg_engine::state::GameResult::Winner(P0));
    let s = next(&p);
    clean_transition(&p, None, &s, &reg);
    let mut s = next(&p);
    s.get_player_mut(P1).lost = false;
    flags_transition(&p, None, &s, &reg, "un-lost the game (CR 104.3)");
    let mut s = next(&p);
    s.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::LifeReachedZero);
    flags_transition(&p, None, &s, &reg, "un-lost the game (CR 104.3)");

    // CR 104.4: a result that exists does not change. One appearing for the
    // first time is the game ending, which is not a change.
    let mut s = next(&prev);
    s.result = Some(mtg_engine::state::GameResult::Winner(P0));
    s.get_player_mut(P1).lost = true;
    s.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::Conceded);
    s.events.push(GameEvent::PlayerLost { player: P1, reason: mtg_engine::events::LossReason::Conceded });
    quiet_transition_about(&prev, None, &s, &reg, "(CR 104.4)");
    let mut s = next(&p);
    s.result = Some(mtg_engine::state::GameResult::Winner(P1));
    flags_transition(&p, None, &s, &reg, "the result changed");
}

/// CR 305.2/606.3/morbid: the per-turn records move the way the turn
/// allows, and by exactly what the events say.
#[test]
fn the_per_turn_records_are_checked_against_the_events() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let land = spell_in_hand(&mut prev, &reg, "Forest", P0);
    let spell = spell_in_hand(&mut prev, &reg, "Moment of Heroism", P0);

    // A land drop and a cast, each spending exactly what its event says.
    let mut s = next(&prev);
    s.get_player_mut(P0).land_plays_remaining -= 1;
    s.move_object(land, Zone::Battlefield, &reg);
    s.events.push(GameEvent::LandPlayed { player: P0, object: land });
    clean_transition(&prev, None, &s, &reg);
    let mut s = next(&prev);
    s.get_player_mut(P0).land_plays_remaining -= 1;
    flags_transition(&prev, None, &s, &reg, "land drops 1 -> 0 with 0 LandPlayed (CR 305.2)");

    let mut s = next(&prev);
    *s.num_spells_cast_this_turn.entry(P0).or_insert(0) += 1;
    s.get_object_mut(spell).unwrap().zone = Zone::Stack;
    s.get_object_mut(spell).unwrap().zone_change_count += 1;
    s.stack.push(StackEntry::Spell(spell));
    s.events.push(GameEvent::ObjectMoved { object: spell, from: Zone::Hand, to: Zone::Stack });
    s.events.push(GameEvent::SpellCast { player: P0, object: spell });
    clean_transition(&prev, None, &s, &reg);
    let mut s = next(&prev);
    *s.num_spells_cast_this_turn.entry(P0).or_insert(0) += 1;
    flags_transition(&prev, None, &s, &reg, "spells cast this turn 0 -> 1 with 0 SpellCast");

    // Across a turn boundary the count of last turn's spells is this turn's
    // record of what came before the turn started.
    let mut p = prev.clone();
    p.step = Step::Cleanup;
    *p.num_spells_cast_this_turn.entry(P0).or_insert(0) = 2;
    let boundary = |last: u32, cast_before: bool| {
        let mut c = next(&p);
        c.step = Step::Untap;
        c.turn_number = p.turn_number + 1;
        c.active_player = p.opponent(p.active_player);
        c.num_spells_cast_this_turn = std::collections::BTreeMap::new();
        c.num_spells_cast_last_turn.insert(P0, last);
        for pl in [P0, P1] {
            c.get_player_mut(pl).land_plays_remaining = 1;
        }
        c.events.clear();
        if cast_before {
            // A cast that happened before the turn turned over counts to
            // the turn that was ending.
            let o = c.get_object_mut(spell).unwrap();
            o.zone = Zone::Stack;
            o.zone_change_count += 1;
            c.stack.push(StackEntry::Spell(spell));
            c.events.push(GameEvent::ObjectMoved { object: spell, from: Zone::Hand, to: Zone::Stack });
            c.events.push(GameEvent::SpellCast { player: P0, object: spell });
        }
        c.events.push(GameEvent::TurnStarted { player: c.active_player, turn: c.turn_number });
        c.events.push(GameEvent::StepStarted { step: Step::Untap });
        c
    };
    clean_transition(&p, None, &boundary(2, false), &reg);
    flags_transition(&p, None, &boundary(3, false), &reg, "cast 2 spells last turn but the record says 3");
    clean_transition(&p, None, &boundary(3, true), &reg);
    flags_transition(&p, None, &boundary(2, true), &reg, "cast 3 spells last turn but the record says 2");

    // Morbid is a per-turn flag: it is not reset mid-turn, and it is not
    // set without a death.
    let mut p = prev.clone();
    p.creature_died_this_turn = true;
    let mut s = next(&p);
    s.creature_died_this_turn = false;
    flags_transition(&p, None, &s, &reg, "the morbid flag was reset mid-turn");
    let mut s = next(&prev);
    s.creature_died_this_turn = true;
    flags_transition(&prev, None, &s, &reg, "the morbid flag was set with no creature dying");

    // CR 606.3: an activation this turn is not forgotten within the turn,
    // and is gone by the next one.
    let mut p = prev.clone();
    p.get_object_mut(bear).unwrap().abilities_activated_this_turn.insert(0);
    let mut s = next(&p);
    s.get_object_mut(bear).unwrap().abilities_activated_this_turn.clear();
    flags_transition(&p, None, &s, &reg, "forgot an activation this turn");
}

/// CR 120.3/701.15a/508.1/121.3: the marks a permanent carries move only
/// the way the events say — damage grows by what was dealt and shrinks only
/// through regeneration or cleanup, regeneration taps and leaves combat, an
/// attack stamp comes from a declaration, and a draw comes off the top.
#[test]
fn the_status_ledgers_of_a_permanent_are_checked() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let other = named_permanent(&mut prev, &reg, "Grizzly Bears", P1);

    // CR 701.20a/701.21a: tapping and untapping are edges, each with its
    // own event, about this permanent.
    let turned = |tapped_before: bool, tapped_after: bool, event: Option<GameEvent>| {
        let mut p = prev.clone();
        p.get_object_mut(bear).unwrap().tapped = tapped_before;
        let mut c = next(&p);
        c.get_object_mut(bear).unwrap().tapped = tapped_after;
        if let Some(e) = event {
            c.events.push(e);
        }
        (p, c)
    };
    let (p, c) = turned(false, true, Some(GameEvent::Tapped { object: bear }));
    quiet_transition_about(&p, None, &c, &reg, "with no Tapped event");
    let (p, c) = turned(true, false, Some(GameEvent::Untapped { object: bear }));
    quiet_transition_about(&p, None, &c, &reg, "with no Untapped event");
    let (p, c) = turned(false, true, None);
    flags_transition(&p, None, &c, &reg, "became tapped with no Tapped event");
    let (p, c) = turned(true, false, None);
    flags_transition(&p, None, &c, &reg, "became untapped with no Untapped event");
    // The event has to be the right verb, and about the right permanent.
    let (p, c) = turned(false, true, Some(GameEvent::Untapped { object: bear }));
    flags_transition(&p, None, &c, &reg, "became tapped with no Tapped event");
    let (p, c) = turned(true, false, Some(GameEvent::Tapped { object: bear }));
    flags_transition(&p, None, &c, &reg, "became untapped with no Untapped event");
    let (p, c) = turned(false, true, Some(GameEvent::Tapped { object: other }));
    flags_transition(&p, None, &c, &reg, "became tapped with no Tapped event");
    let (p, c) = turned(true, false, Some(GameEvent::Untapped { object: other }));
    flags_transition(&p, None, &c, &reg, "became untapped with no Untapped event");

    // Damage grows by exactly what was dealt.
    let dealt = |n: u32, marked: u32| {
        let mut c = next(&prev);
        c.get_object_mut(bear).unwrap().damage_marked = marked;
        c.get_object_mut(bear).unwrap().damaged_by.push(other);
        if n > 0 {
            c.events.push(GameEvent::NonCombatDamageDealt {
                source: other, target: DamageTarget::Object(bear), amount: n });
            c.get_player_mut(P0).life = prev.get_player(P0).life;
        }
        c
    };
    clean_transition(&prev, None, &dealt(2, 2), &reg);
    flags_transition(&prev, None, &dealt(2, 3), &reg, "has 3 damage marked after 0 + 2 dealt (CR 120.3)");
    flags_transition(&prev, None, &dealt(0, 1), &reg, "has 1 damage marked after 0 + 0 dealt (CR 120.3)");

    // The damage dealt is the damage dealt to THIS permanent: a bystander
    // in the same window has not lost anything.
    let s = dealt(2, 2);
    let bystander = s.get_object(other).unwrap().damage_marked;
    assert_eq!(bystander, 0, "test setup: the other creature took nothing");
    clean_transition(&prev, None, &s, &reg);

    // CR 306.7: a planeswalker takes damage as loyalty, so the marked-damage
    // ledger is not about it.
    let mut walkers = prev.clone();
    let walker = named_permanent(&mut walkers, &reg, "Liliana of the Veil", P0);
    let mut s = next(&walkers);
    s.events.push(GameEvent::NonCombatDamageDealt {
        source: other, target: DamageTarget::Object(walker), amount: 1 });
    set_loyalty(&mut s, walker, counters_of(&walkers, walker, CounterType::Loyalty) - 1);
    quiet_transition_about(&walkers, None, &s, &reg, "marked damage");

    // And shrinks only through regeneration or cleanup.
    let mut hurt = prev.clone();
    hurt.get_object_mut(bear).unwrap().damage_marked = 2;
    let mut s = next(&hurt);
    s.get_object_mut(bear).unwrap().damage_marked = 0;
    flags_transition(&hurt, None, &s, &reg, "lost marked damage (2 + 0 -> 0) with no regeneration or cleanup");
    let mut s = next(&hurt);
    s.get_object_mut(bear).unwrap().damage_marked = 0;
    s.events.push(GameEvent::StepStarted { step: Step::Cleanup });
    s.step = Step::Cleanup;
    quiet_transition_about(&hurt, None, &s, &reg, "lost marked damage");
    // A shield that was there, or one that moves, is the other way out.
    let mut shielded = hurt.clone();
    shielded.get_object_mut(bear).unwrap().regeneration_shields = 1;
    let mut s = next(&shielded);
    {
        let o = s.get_object_mut(bear).unwrap();
        o.damage_marked = 0;
        o.regeneration_shields = 0;
        o.tapped = true;
    }
    s.events.push(GameEvent::Tapped { object: bear });
    quiet_transition_about(&shielded, None, &s, &reg, "lost marked damage");

    // A shield that was already there is licence enough: it is the shield
    // being spent that clears the damage, whether or not the count moved in
    // this window.
    let mut s = next(&shielded);
    s.get_object_mut(bear).unwrap().damage_marked = 0;
    quiet_transition_about(&shielded, None, &s, &reg, "lost marked damage");

    // CR 701.15a: regenerating taps the permanent and removes it from combat.
    let mut s = next(&shielded);
    s.get_object_mut(bear).unwrap().regeneration_shields = 0;
    flags_transition(&shielded, None, &s, &reg, "regenerated without tapping (CR 701.15a)");
    let mut s = next(&shielded);
    {
        let o = s.get_object_mut(bear).unwrap();
        o.regeneration_shields = 0;
        o.tapped = true;
    }
    s.events.push(GameEvent::Tapped { object: bear });
    let mut combat = mtg_engine::state::CombatState::new();
    combat.attackers.insert(bear, P1);
    combat.blocker_assignments.insert(bear, vec![]);
    combat.any_attackers_declared = true;
    s.combat = Some(combat);
    flags_transition(&shielded, None, &s, &reg, "regenerated but is still in combat (CR 701.15a)");
    // Somebody else's block does not keep this one in combat.
    let mut s = next(&shielded);
    {
        let o = s.get_object_mut(bear).unwrap();
        o.regeneration_shields = 0;
        o.tapped = true;
    }
    s.events.push(GameEvent::Tapped { object: bear });
    let mut combat = mtg_engine::state::CombatState::new();
    combat.attackers.insert(other, P0);
    combat.blocker_assignments.insert(other, vec![]);
    combat.any_attackers_declared = true;
    s.combat = Some(combat);
    quiet_transition_about(&shielded, None, &s, &reg, "still in combat");
    // A permanent that was already tapped regenerates without a new tap.
    let mut tapped_shield = shielded.clone();
    tapped_shield.get_object_mut(bear).unwrap().tapped = true;
    let mut s = next(&tapped_shield);
    s.get_object_mut(bear).unwrap().regeneration_shields = 0;
    quiet_transition_about(&tapped_shield, None, &s, &reg, "regenerated without tapping");

    // CR 508.1: an attack stamp names this turn and comes with a declaration.
    let stamped = |turn: Option<u32>, declared: bool| {
        let mut c = next(&prev);
        c.get_object_mut(bear).unwrap().attacked_on_turn = turn;
        if declared {
            c.events.push(GameEvent::AttackersDeclared { attackers: vec![(bear, P1)] });
        }
        c
    };
    clean_transition(&prev, None, &stamped(Some(prev.turn_number), true), &reg);
    flags_transition(&prev, None, &stamped(Some(prev.turn_number), false), &reg,
        "was stamped as attacking without a declaration (CR 508.1)");
    flags_transition(&prev, None, &stamped(Some(prev.turn_number - 1), true), &reg,
        "was stamped as attacking without a declaration (CR 508.1)");

    // CR 121.3: a drawn card is one the player's library held, off the top.
    let mut lib = prev.clone();
    let cards = stock_library(&mut lib, &reg, P0, 3);
    let draw = |take: usize| {
        let mut c = next(&lib);
        let id = cards[take];
        c.get_player_mut(P0).library_order.retain(|&x| x != id);
        {
            let o = c.get_object_mut(id).unwrap();
            o.zone = Zone::Hand;
            o.zone_change_count += 1;
        }
        c.events.push(GameEvent::ObjectMoved { object: id, from: Zone::Library, to: Zone::Hand });
        c.events.push(GameEvent::CardDrawn { player: P0, object: id });
        c
    };
    clean_transition(&lib, None, &draw(0), &reg);
    flags_transition(&lib, None, &draw(2), &reg, "from below the top 1");

    // A card that was never in that library at all.
    let mut c = next(&lib);
    {
        let o = c.get_object_mut(other).unwrap();
        o.zone = Zone::Hand;
        o.zone_change_count += 1;
    }
    c.events.push(GameEvent::ObjectMoved { object: other, from: Zone::Library, to: Zone::Hand });
    c.events.push(GameEvent::CardDrawn { player: P0, object: other });
    flags_transition(&lib, None, &c, &reg, "which was not in p0's library (CR 121.1)");
}

/// CR 119/104.3/704.5a-b/121.4: life moves only through its events, and a
/// loss names a reason the state can show.
#[test]
fn the_life_and_loss_ledger_is_checked() {
    let (prev, reg) = base();

    // The chain of LifeChanged events starts where the player was and ends
    // where they are.
    let lost_life = |first_old: i32, last_new: i32, life: i32| {
        let mut c = next(&prev);
        c.get_player_mut(P1).life = life;
        c.events.push(GameEvent::LifeChanged { player: P1, old: first_old, new_life: last_new });
        c
    };
    clean_transition(&prev, None, &lost_life(20, 18, 18), &reg);
    flags_transition(&prev, None, &lost_life(19, 18, 18), &reg, "life chain starts at 19 but they had 20");
    flags_transition(&prev, None, &lost_life(20, 18, 17), &reg, "life chain ends at 18 but they have 17");

    // The chain is that player's own events: somebody else's life moving in
    // the same window says nothing about theirs.
    let mut s = next(&prev);
    s.get_player_mut(P0).life = 18;
    s.events.push(GameEvent::LifeChanged { player: P0, old: 20, new_life: 18 });
    clean_transition(&prev, None, &s, &reg);

    // CR 704.5a: losing to zero life needs the life to have reached zero.
    let dies = |life: i32, chain_to: i32| {
        let mut c = next(&prev);
        c.get_player_mut(P1).life = life;
        c.get_player_mut(P1).lost = true;
        c.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::LifeReachedZero);
        c.result = Some(mtg_engine::state::GameResult::Winner(P0));
        c.events.push(GameEvent::LifeChanged { player: P1, old: 20, new_life: chain_to });
        c.events.push(GameEvent::PlayerLost { player: P1, reason: mtg_engine::events::LossReason::LifeReachedZero });
        c
    };
    clean_transition(&prev, None, &dies(0, 0), &reg);
    clean_transition(&prev, None, &dies(-3, -3), &reg);
    flags_transition(&prev, None, &dies(5, 5), &reg, "lost to 0 life without their life reaching 0 (CR 704.5a)");
    // A player who was already at zero when the window opened, and a window
    // whose chain dips to zero and comes back, are both the rule being met.
    let mut at_zero = prev.clone();
    at_zero.get_player_mut(P1).life = 0;
    let mut c = next(&at_zero);
    c.get_player_mut(P1).lost = true;
    c.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::LifeReachedZero);
    c.result = Some(mtg_engine::state::GameResult::Winner(P0));
    c.events.push(GameEvent::PlayerLost { player: P1, reason: mtg_engine::events::LossReason::LifeReachedZero });
    quiet_transition_about(&at_zero, None, &c, &reg, "(CR 704.5a)");
    // A player whose life ends above zero did not lose to it, however far
    // it dipped in between: CR 704.5a is a state-based action, and the state
    // it is checked against is the one at the end.
    let mut c = next(&prev);
    c.get_player_mut(P1).life = 3;
    c.get_player_mut(P1).lost = true;
    c.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::LifeReachedZero);
    c.result = Some(mtg_engine::state::GameResult::Winner(P0));
    c.events.push(GameEvent::LifeChanged { player: P1, old: 20, new_life: 0 });
    c.events.push(GameEvent::LifeChanged { player: P1, old: 0, new_life: 3 });
    c.events.push(GameEvent::PlayerLost { player: P1, reason: mtg_engine::events::LossReason::LifeReachedZero });
    flags_transition(&prev, None, &c, &reg, "(CR 704.5a)");

    // A loss with no PlayerLost event at all.
    let mut c = next(&prev);
    c.get_player_mut(P1).life = 0;
    c.get_player_mut(P1).lost = true;
    c.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::LifeReachedZero);
    c.result = Some(mtg_engine::state::GameResult::Winner(P0));
    c.events.push(GameEvent::LifeChanged { player: P1, old: 20, new_life: 0 });
    flags_transition(&prev, None, &c, &reg, "with no PlayerLost event");

    // CR 704.5b: an empty-library loss is recorded on the player.
    let empty_draw = |recorded: bool| {
        let mut c = next(&prev);
        c.get_player_mut(P1).lost = true;
        c.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::DrewFromEmptyLibrary);
        c.get_player_mut(P1).has_drawn_from_empty = recorded;
        c.result = Some(mtg_engine::state::GameResult::Winner(P0));
        c.events.push(GameEvent::PlayerLost {
            player: P1, reason: mtg_engine::events::LossReason::DrewFromEmptyLibrary });
        c
    };
    clean_transition(&prev, None, &empty_draw(true), &reg);
    flags_transition(&prev, None, &empty_draw(false), &reg,
        "lost to an empty-library draw that is not recorded (CR 704.5b)");

    // CR 104.3a: a concede is the conceding player's own action, taken with
    // priority. Each half alone.
    let conceded = |priority: Option<PlayerId>| {
        let mut p = prev.clone();
        p.priority_player = priority;
        let mut c = next(&p);
        c.get_player_mut(P1).lost = true;
        c.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::Conceded);
        c.result = Some(mtg_engine::state::GameResult::Winner(P0));
        c.events.push(GameEvent::PlayerLost { player: P1, reason: mtg_engine::events::LossReason::Conceded });
        (p, c)
    };
    let concede = Action::Concede;
    let (p, c) = conceded(Some(P1));
    quiet_transition_about(&p, Some(&concede), &c, &reg, "conceded without holding priority");
    let (p, c) = conceded(Some(P1));
    flags_transition(&p, None, &c, &reg, "conceded without holding priority on a Concede action");
    let (p, c) = conceded(Some(P0));
    flags_transition(&p, Some(&concede), &c, &reg, "conceded without holding priority on a Concede action");

    // CR 121.4: "drew from an empty library" is about an empty library.
    let mut c = next(&prev);
    c.get_player_mut(P1).has_drawn_from_empty = true;
    quiet_transition_about(&prev, None, &c, &reg, "(CR 121.4)");
    let mut with_library = prev.clone();
    stock_library(&mut with_library, &reg, P1, 2);
    let mut c = next(&with_library);
    c.get_player_mut(P1).has_drawn_from_empty = true;
    flags_transition(&with_library, None, &c, &reg,
        "recorded as drawing from an empty library that holds 2 cards (CR 121.4)");
    // Unless the library was refilled in the same window — the draw failed
    // against the library as it was, and CR 701.20a put cards back after.
    let mut refilled = next(&with_library);
    refilled.get_player_mut(P1).has_drawn_from_empty = true;
    let put_back: Vec<mtg_engine::ids::ObjectId> = with_library
        .objects_in_zone(Zone::Graveyard, P1).iter().map(|o| o.id).collect();
    let _ = put_back;
    let card = spell_in_hand(&mut refilled, &reg, "Forest", P1);
    {
        let o = refilled.get_object_mut(card).unwrap();
        o.zone = Zone::Library;
        o.zone_change_count += 1;
    }
    refilled.get_player_mut(P1).library_order.push(card);
    quiet_transition_about(&with_library, None, &refilled, &reg, "(CR 121.4)");
}

/// CR 106.4/500.4: mana appears only through `ManaAdded`, and leaves only
/// by a payment or the end of a step.
#[test]
fn the_mana_ledger_is_checked() {
    let (prev, reg) = base();

    let mut s = next(&prev);
    s.get_player_mut(P0).mana_pool.mana.insert(ManaType::Green, 2);
    s.events.push(GameEvent::ManaAdded { player: P0, mana_type: ManaType::Green, amount: 2 });
    clean_transition(&prev, None, &s, &reg);

    let mut s = next(&prev);
    s.get_player_mut(P0).mana_pool.mana.insert(ManaType::Green, 3);
    s.events.push(GameEvent::ManaAdded { player: P0, mana_type: ManaType::Green, amount: 2 });
    flags_transition(&prev, None, &s, &reg, "has 3 Green mana after 0 + 2 added (CR 106.4)");
    // Added for somebody else, or of another colour, is not added here.
    let mut s = next(&prev);
    s.get_player_mut(P0).mana_pool.mana.insert(ManaType::Green, 2);
    s.events.push(GameEvent::ManaAdded { player: P1, mana_type: ManaType::Green, amount: 2 });
    flags_transition(&prev, None, &s, &reg, "(CR 106.4)");
    let mut s = next(&prev);
    s.get_player_mut(P0).mana_pool.mana.insert(ManaType::Green, 2);
    s.events.push(GameEvent::ManaAdded { player: P0, mana_type: ManaType::Red, amount: 2 });
    flags_transition(&prev, None, &s, &reg, "(CR 106.4)");

    // CR 500.4: mana leaves at the end of a step, or to pay for something.
    let mut floating = prev.clone();
    floating.get_player_mut(P0).mana_pool.mana.insert(ManaType::Green, 2);
    let mut s = next(&floating);
    s.get_player_mut(P0).mana_pool.mana.remove(&ManaType::Green);
    flags_transition(&floating, None, &s, &reg, "with nothing paid and no step ending (CR 500.4)");
    let mut s = next(&floating);
    s.get_player_mut(P0).mana_pool.mana.remove(&ManaType::Green);
    s.events.push(GameEvent::ManaPoolEmptied { player: P0 });
    quiet_transition_about(&floating, None, &s, &reg, "(CR 500.4)");
}

/// CR 305.1/601.2h/602.2f/605.3: what an action that spends something has
/// to show for it — a land that moved from hand to battlefield with its
/// event, and a pool that went down by what the cost demanded.
#[test]
fn the_costs_an_action_pays_are_checked_against_the_pool() {
    let (mut prev, reg) = base();
    let land = spell_in_hand(&mut prev, &reg, "Forest", P0);
    prev.priority_player = Some(P0);

    // CR 305.1: a land play moves the card and says so. Each half alone.
    let play = mtg_engine::actions::Action::PlayLand { object_id: land };
    let played = |from_hand: bool, to_battlefield: bool, announced: bool| {
        let mut p = prev.clone();
        if !from_hand {
            p.get_object_mut(land).unwrap().zone = Zone::Graveyard;
        }
        let mut c = next(&p);
        c.get_player_mut(P0).land_plays_remaining -= 1;
        {
            let o = c.get_object_mut(land).unwrap();
            o.zone = if to_battlefield { Zone::Battlefield } else { Zone::Graveyard };
            o.zone_change_count += 1;
        }
        if announced {
            c.events.push(GameEvent::LandPlayed { player: P0, object: land });
        }
        (p, c)
    };
    let (p, c) = played(true, true, true);
    quiet_transition_about(&p, Some(&play), &c, &reg, "(CR 305.1)");
    let (p, c) = played(false, true, true);
    flags_transition(&p, Some(&play), &c, &reg, "(CR 305.1)");
    let (p, c) = played(true, false, true);
    flags_transition(&p, Some(&play), &c, &reg, "(CR 305.1)");
    let (p, c) = played(true, true, false);
    flags_transition(&p, Some(&play), &c, &reg, "(CR 305.1)");

    // CR 601.2h: casting spends the cost out of the pool — all of it, not
    // just the coloured pips.
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let pump = spell_in_hand(&mut prev, &reg, "Moment of Heroism", P0);
    prev.priority_player = Some(P0);
    // {1}{W}: two mana, one of them white.
    add_mana(&mut prev, P0, &[(ManaType::White, 1), (ManaType::Green, 1)]);
    let cast = mtg_engine::actions::Action::CastSpell {
        object_id: pump, targets: vec![Target::Object(bear)], sacrifice: None,
        exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![],
    };
    let casting = |left: &[(ManaType, u32)]| {
        let mut c = next(&prev);
        c.get_player_mut(P0).mana_pool.mana.clear();
        for (t, n) in left {
            c.get_player_mut(P0).mana_pool.mana.insert(*t, *n);
        }
        {
            let o = c.get_object_mut(pump).unwrap();
            o.zone = Zone::Stack;
            o.zone_change_count += 1;
        }
        c.stack.push(StackEntry::Spell(pump));
        *c.num_spells_cast_this_turn.entry(P0).or_insert(0) += 1;
        c.events.push(GameEvent::ObjectMoved { object: pump, from: Zone::Hand, to: Zone::Stack });
        c.events.push(GameEvent::SpellCast { player: P0, object: pump });
        c
    };
    quiet_transition_about(&prev, Some(&cast), &casting(&[]), &reg, "(CR 601.2h)");

    // Mana tapped for the cost inside the same window counts as paid: the
    // pool ends where it started, having gained and spent two.
    let mut with_taps = prev.clone();
    with_taps.get_player_mut(P0).mana_pool.mana.clear();
    let mut c = next(&with_taps);
    {
        let o = c.get_object_mut(pump).unwrap();
        o.zone = Zone::Stack;
        o.zone_change_count += 1;
    }
    c.stack.push(StackEntry::Spell(pump));
    *c.num_spells_cast_this_turn.entry(P0).or_insert(0) += 1;
    c.events.push(GameEvent::ManaAdded { player: P0, mana_type: ManaType::White, amount: 1 });
    c.events.push(GameEvent::ManaAdded { player: P0, mana_type: ManaType::Green, amount: 1 });
    c.events.push(GameEvent::ObjectMoved { object: pump, from: Zone::Hand, to: Zone::Stack });
    c.events.push(GameEvent::SpellCast { player: P0, object: pump });
    quiet_transition_about(&with_taps, Some(&cast), &c, &reg, "(CR 601.2h)");
    // Mana added for somebody else, or of another colour, pays nothing.
    let mut s = c.clone();
    s.events.retain(|e| !matches!(e, GameEvent::ManaAdded { .. }));
    s.events.insert(0, GameEvent::ManaAdded { player: P1, mana_type: ManaType::White, amount: 2 });
    flags_transition(&with_taps, Some(&cast), &s, &reg, "(CR 601.2h)");
    let mut s = c.clone();
    s.events.retain(|e| !matches!(e, GameEvent::ManaAdded { mana_type: ManaType::White, .. }));
    s.events.insert(0, GameEvent::ManaAdded { player: P0, mana_type: ManaType::Green, amount: 1 });
    flags_transition(&with_taps, Some(&cast), &s, &reg, "White mana after 0 + 0 for a cost of 1 (CR 601.2h)");
    // The generic half never left.
    flags_transition(&prev, Some(&cast), &casting(&[(ManaType::Green, 1)]), &reg,
        "for a total cost of 2 (CR 601.2h)");
    // The coloured pip never left.
    flags_transition(&prev, Some(&cast), &casting(&[(ManaType::White, 1)]), &reg,
        "White mana after 1 + 0 for a cost of 1 (CR 601.2h)");

    // CR 605.3: a mana ability does not touch the step, priority or stack,
    // and does not tap what is already tapped.
    let (mut prev, reg) = base();
    let forest = named_permanent(&mut prev, &reg, "Forest", P0);
    prev.priority_player = Some(P0);
    let tap = mtg_engine::actions::Action::ActivateManaAbility { object_id: forest, ability_index: 0 };
    let mut c = next(&prev);
    c.get_object_mut(forest).unwrap().tapped = true;
    c.get_player_mut(P0).mana_pool.mana.insert(ManaType::Green, 1);
    c.events.push(GameEvent::Tapped { object: forest });
    c.events.push(GameEvent::ManaAdded { player: P0, mana_type: ManaType::Green, amount: 1 });
    quiet_transition_about(&prev, Some(&tap), &c, &reg, "(CR 605.3)");
    let mut s = c.clone();
    s.step = Step::BeginCombat;
    s.events.push(GameEvent::StepStarted { step: Step::BeginCombat });
    flags_transition(&prev, Some(&tap), &s, &reg, "changed the step, priority, or the stack (CR 605.3)");
    let mut s = c.clone();
    s.priority_player = Some(P1);
    flags_transition(&prev, Some(&tap), &s, &reg, "changed the step, priority, or the stack (CR 605.3)");
    // Tapping what was already tapped.
    let mut already = prev.clone();
    already.get_object_mut(forest).unwrap().tapped = true;
    let mut s = next(&already);
    s.get_player_mut(P0).mana_pool.mana.insert(ManaType::Green, 1);
    s.events.push(GameEvent::Tapped { object: forest });
    s.events.push(GameEvent::ManaAdded { player: P0, mana_type: ManaType::Green, amount: 1 });
    flags_transition(&already, Some(&tap), &s, &reg, "tapped #");
}

/// CR 514.1/103.5: the hand-size discard, the mulligan and the bottoming
/// each move exactly the cards they name, in the order the rules give.
#[test]
fn the_hand_shaping_actions_move_exactly_what_they_name() {
    let reg = registry();
    let mut prev = game_at_step(Step::Cleanup, P0);
    prev.turn_number = 3;
    let hand: Vec<mtg_engine::ids::ObjectId> =
        (0..3).map(|_| spell_in_hand(&mut prev, &reg, "Forest", P0)).collect();
    prev.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P0, discard_count: 1 });
    prev.priority_player = None;

    let discard = mtg_engine::actions::Action::DiscardCards { cards: vec![hand[0]] };
    let discarded = |ids: &[mtg_engine::ids::ObjectId], announce: &[mtg_engine::ids::ObjectId]| {
        let mut c = next(&prev);
        c.awaiting_action = None;
        for &id in ids {
            let o = c.get_object_mut(id).unwrap();
            o.zone = Zone::Graveyard;
            o.zone_change_count += 1;
            c.events.push(GameEvent::ObjectMoved { object: id, from: Zone::Hand, to: Zone::Graveyard });
        }
        for &id in announce {
            c.events.push(GameEvent::Discarded { player: P0, object: id });
        }
        c
    };
    quiet_transition_about(&prev, Some(&discard), &discarded(&hand[..1], &hand[..1]), &reg, "(CR 514.1)");
    // Announced for a card the action did not name.
    flags_transition(&prev, Some(&discard), &discarded(&hand[1..2], &hand[1..2]), &reg, "(CR 514.1)");
    // Named but never moved.
    let mut c = discarded(&[], &hand[..1]);
    quiet_transition_about(&prev, Some(&discard), &c, &reg, "(CR 514.1)");
    flags_transition(&prev, Some(&discard), &c, &reg, "was not moved out of p0's hand");
    let _ = &mut c;

    // CR 103.5: a mulligan shuffles, then draws, and moves the count.
    let mut opening = game_at_step(Step::Untap, P0);
    opening.turn_number = 1;
    opening.is_first_turn = true;
    opening.priority_player = None;
    let library: Vec<mtg_engine::ids::ObjectId> = stock_library(&mut opening, &reg, P0, 20);
    let old_hand: Vec<mtg_engine::ids::ObjectId> =
        (0..7).map(|_| spell_in_hand(&mut opening, &reg, "Forest", P0)).collect();
    opening.awaiting_action = Some(AwaitingAction::MulliganDecision { player: P0 });

    let mull = |shuffle_first: bool| {
        let mut c = next(&opening);
        c.awaiting_action = None;
        c.get_player_mut(P0).mulligan_count += 1;
        for &id in &old_hand {
            let o = c.get_object_mut(id).unwrap();
            o.zone = Zone::Library;
            o.zone_change_count += 1;
            c.get_player_mut(P0).library_order.push(id);
            c.events.push(GameEvent::ObjectMoved { object: id, from: Zone::Hand, to: Zone::Library });
        }
        let mut draws = Vec::new();
        for &id in library.iter().take(7) {
            let o = c.get_object_mut(id).unwrap();
            o.zone = Zone::Hand;
            o.zone_change_count += 1;
            c.get_player_mut(P0).library_order.retain(|&x| x != id);
            draws.push(GameEvent::ObjectMoved { object: id, from: Zone::Library, to: Zone::Hand });
            draws.push(GameEvent::CardDrawn { player: P0, object: id });
        }
        if shuffle_first {
            c.events.push(GameEvent::LibraryShuffled { player: P0 });
            c.events.extend(draws);
        } else {
            c.events.extend(draws);
            c.events.push(GameEvent::LibraryShuffled { player: P0 });
        }
        c
    };
    quiet_transition_about(&opening, Some(&mtg_engine::actions::Action::MulliganMull), &mull(true), &reg, "(CR 103.5)");
    flags_transition(&opening, Some(&mtg_engine::actions::Action::MulliganMull), &mull(false), &reg,
        "drew the new hand before shuffling (CR 103.5)");
    let mut no_shuffle = mull(true);
    no_shuffle.events.retain(|e| !matches!(e, GameEvent::LibraryShuffled { .. }));
    flags_transition(&opening, Some(&mtg_engine::actions::Action::MulliganMull), &no_shuffle, &reg,
        "mulliganed without shuffling (CR 103.5)");
    let mut no_count = mull(true);
    no_count.get_player_mut(P0).mulligan_count = opening.get_player(P0).mulligan_count;
    flags_transition(&opening, Some(&mtg_engine::actions::Action::MulliganMull), &no_count, &reg,
        "mulliganed without the count moving (CR 103.5)");
    // CR 103.4: the new hand is the cards that were drawn for it.
    let mut short = mull(true);
    let extra = spell_in_hand(&mut short, &reg, "Forest", P0);
    let _ = extra;
    flags_transition(&opening, Some(&mtg_engine::actions::Action::MulliganMull), &short, &reg,
        "drew 7 cards for a new hand of 8");
    // And a card of the old hand left behind is not a new hand.
    let mut kept = mull(true);
    {
        let o = kept.get_object_mut(old_hand[0]).unwrap();
        o.zone = Zone::Hand;
        o.zone_change_count = opening.get_object(old_hand[0]).unwrap().zone_change_count;
    }
    kept.get_player_mut(P0).library_order.retain(|&x| x != old_hand[0]);
    flags_transition(&opening, Some(&mtg_engine::actions::Action::MulliganMull), &kept, &reg,
        "stayed in hand");

    // CR 103.5: bottoming puts exactly the cards asked for on the bottom.
    let mut bottoming = opening.clone();
    bottoming.awaiting_action = Some(AwaitingAction::BottomAfterMulligan { player: P0, count: 1 });
    let bottom = |ids: Vec<mtg_engine::ids::ObjectId>, to_top: bool| {
        let mut c = next(&bottoming);
        c.awaiting_action = None;
        for &id in &ids {
            let o = c.get_object_mut(id).unwrap();
            o.zone = Zone::Library;
            o.zone_change_count += 1;
            if to_top {
                c.get_player_mut(P0).library_order.insert(0, id);
            } else {
                c.get_player_mut(P0).library_order.push(id);
            }
            c.events.push(GameEvent::ObjectMoved { object: id, from: Zone::Hand, to: Zone::Library });
        }
        c
    };
    let put = mtg_engine::actions::Action::BottomCards { cards: vec![old_hand[0]] };
    quiet_transition_about(&bottoming, Some(&put), &bottom(vec![old_hand[0]], false), &reg, "(CR 103.5)");
    quiet_transition_about(&bottoming, Some(&put), &bottom(vec![old_hand[0]], false), &reg,
        "did not go from hand to library");
    quiet_transition_about(&bottoming, Some(&put), &bottom(vec![old_hand[0]], false), &reg,
        "are not the bottom of");
    // The wrong number of cards.
    let two = mtg_engine::actions::Action::BottomCards { cards: vec![old_hand[0], old_hand[1]] };
    flags_transition(&bottoming, Some(&two), &bottom(vec![old_hand[0], old_hand[1]], false), &reg,
        "bottomed 2 cards, asked for 1 (CR 103.5)");
    // On the top instead of the bottom.
    flags_transition(&bottoming, Some(&put), &bottom(vec![old_hand[0]], true), &reg,
        "are not the bottom of p0's library");
    // Named but left in hand.
    let mut c = next(&bottoming);
    c.awaiting_action = None;
    flags_transition(&bottoming, Some(&put), &c, &reg, "did not go from hand to library");
    let _ = &mut c;
}

/// CR 103.4: the opening hands are outside the turn structure — the untap
/// step is announced without a turn starting, and turn 1 is announced
/// without the counter moving.
#[test]
fn the_mulligan_phases_own_succession_is_checked() {
    // The opening-hand phase as the game really reaches it: turn one, untap
    // step, everything still in libraries and hands.
    let reg = registry();
    let mut prev = game_at_step(Step::Untap, P0);
    prev.turn_number = 1;
    prev.is_first_turn = true;
    prev.priority_player = None;
    for p in [P0, P1] {
        for id in stock_library(&mut prev, &reg, p, 20) {
            prev.get_object_mut(id).unwrap().name = "Forest".into();
        }
        for _ in 0..7 {
            spell_in_hand(&mut prev, &reg, "Moment of Heroism", p);
        }
    }
    prev.awaiting_action = Some(AwaitingAction::MulliganDecision { player: P0 });

    let mut c = next(&prev);
    c.awaiting_action = None;
    c.events = vec![
        GameEvent::TurnStarted { player: P0, turn: 1 },
        GameEvent::StepStarted { step: Step::Untap },
    ];
    clean_transition(&prev, None, &c, &reg);

    // A second turn starting, or the counter moving, is not the opening hand.
    let mut s = c.clone();
    s.events.push(GameEvent::TurnStarted { player: P1, turn: 2 });
    flags_transition(&prev, None, &s, &reg, "while leaving the opening hands");
    let mut s = c.clone();
    s.turn_number = 2;
    flags_transition(&prev, None, &s, &reg, "while leaving the opening hands");
}

/// CR 508.1: the attackers the engine declares are the ones the player
/// submitted, plus the ones an effect forces. Anything else is the engine
/// attacking with a creature nobody chose — the one class of combat bug
/// that leaves no other trace, since the declaration event *is* the record.
#[test]
fn a_declaration_that_disagrees_with_the_submitted_attackers_is_flagged() {
    let (mut prev, reg) = base();
    prev.step = Step::DeclareAttackers;
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let sick = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    prev.get_object_mut(sick).unwrap().summoning_sick = true;
    let forced = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let furor = named_permanent(&mut prev, &reg, "Furor of the Bitten", P0);
    prev.get_object_mut(furor).unwrap().attached_to = Some(forced);
    prev.awaiting_action = Some(AwaitingAction::DeclareAttackers);

    let declare = |ids: &[ObjectId]| mtg_engine::actions::Action::DeclareAttackers {
        attackers: ids.iter().map(|&id| (id, P1)).collect(),
        planeswalker_attacks: vec![],
    };
    // What the engine really does: everything submitted, plus the forced
    // attacker it adds itself, announced and recorded.
    let declared = |prev: &GameState, ids: &[ObjectId]| {
        let mut cur = next(prev);
        cur.awaiting_action = None;
        let attackers: Vec<(ObjectId, PlayerId)> = ids.iter().map(|&id| (id, P1)).collect();
        let mut combat = mtg_engine::state::CombatState::new();
        for &(id, who) in &attackers {
            combat.attackers.insert(id, who);
            combat.blocker_assignments.insert(id, vec![]);
            combat.any_attackers_declared = true;
            cur.get_object_mut(id).unwrap().tapped = true;
            cur.events.push(GameEvent::Tapped { object: id });
        }
        cur.combat = Some(combat);
        cur.events.push(GameEvent::AttackersDeclared { attackers });
        cur
    };

    // The forced attacker is legitimately declared without being submitted
    // (CR 508.1d), and that is not a violation.
    clean_transition(&prev, Some(&declare(&[bear])), &declared(&prev, &[bear, forced]), &reg);

    // A creature nobody submitted and nothing forces.
    let vanilla = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    flags_transition(&prev, Some(&declare(&[bear])), &declared(&prev, &[bear, forced, vanilla]), &reg,
        &format!("#{} was declared attacking but was neither submitted nor forced", vanilla.0));

    // Submitted, but not eligible to attack in the first place.
    flags_transition(&prev, Some(&declare(&[sick])), &declared(&prev, &[sick, forced]), &reg,
        &format!("#{} was declared attacking but was not eligible (CR 508.1c)", sick.0));

    // In combat, but in no declaration: the engine put it there itself.
    let mut cur = declared(&prev, &[bear, forced]);
    if let Some(c) = cur.combat.as_mut() { c.attackers.insert(vanilla, P1); c.blocker_assignments.insert(vanilla, vec![]); }
    flags_transition(&prev, Some(&declare(&[bear])), &cur, &reg,
        &format!("#{} is attacking without having been declared", vanilla.0));
}

/// CR 509.1a: every block the engine records is one the defender submitted,
/// by an untapped creature of theirs, against a creature that is actually
/// attacking.
#[test]
fn a_declaration_that_disagrees_with_the_submitted_blocks_is_flagged() {
    let (mut prev, reg) = base();
    prev.step = Step::DeclareBlockers;
    let attacker = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let blocker = named_permanent(&mut prev, &reg, "Grizzly Bears", P1);
    let tapped = named_permanent(&mut prev, &reg, "Grizzly Bears", P1);
    prev.get_object_mut(tapped).unwrap().tapped = true;
    let mine = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let mut combat = mtg_engine::state::CombatState::new();
    combat.attackers.insert(attacker, P1);
    combat.blocker_assignments.insert(attacker, vec![]);
    combat.any_attackers_declared = true;
    prev.combat = Some(combat);
    prev.awaiting_action = Some(AwaitingAction::DeclareBlockers { defending_player: P1 });
    prev.priority_player = Some(P1);

    let submit = |pairs: &[(ObjectId, ObjectId)]| mtg_engine::actions::Action::DeclareBlockers {
        assignments: pairs.to_vec(),
    };
    let declared = |pairs: &[(ObjectId, ObjectId)]| {
        let mut cur = next(&prev);
        cur.awaiting_action = None;
        if let Some(c) = cur.combat.as_mut() {
            for &(b, a) in pairs {
                c.blocker_assignments.entry(a).or_default().push(b);
                c.blocked_attackers.insert(a);
            }
        }
        cur.events.push(GameEvent::BlockersDeclared { assignments: pairs.to_vec() });
        cur
    };

    clean_transition(&prev, Some(&submit(&[(blocker, attacker)])), &declared(&[(blocker, attacker)]), &reg);

    flags_transition(&prev, Some(&submit(&[])), &declared(&[(blocker, attacker)]), &reg,
        &format!("#{} blocking #{} was declared but never submitted", blocker.0, attacker.0));
    flags_transition(&prev, Some(&submit(&[(tapped, attacker)])), &declared(&[(tapped, attacker)]), &reg,
        &format!("#{} blocking #{} was not a legal block (CR 509.1a)", tapped.0, attacker.0));
    flags_transition(&prev, Some(&submit(&[(mine, attacker)])), &declared(&[(mine, attacker)]), &reg,
        &format!("#{} blocking #{} was not a legal block (CR 509.1a)", mine.0, attacker.0));
}

/// CR 601.2: a cast either goes on the stack, stops on a cost it is waiting
/// to have paid, or is refused — and a refusal leaves the game exactly as it
/// was. "Refused, but the card moved anyway" is a spell that half-happened.
#[test]
fn a_cast_that_neither_resolved_nor_was_cleanly_refused_is_flagged() {
    let (mut prev, reg) = base();
    let card = spell_in_hand(&mut prev, &reg, "Grizzly Bears", P0);
    let other = spell_in_hand(&mut prev, &reg, "Grizzly Bears", P0);
    let cast = mtg_engine::actions::Action::CastSpell {
        object_id: card, targets: vec![], sacrifice: None, exile_count: None,
        exile_ids: vec![], alternative_cost: None, tap_plan: vec![],
    };

    // Refused: nothing moved, nothing announced.
    clean_transition(&prev, Some(&cast), &next(&prev), &reg);

    // Refused, but the card left the hand.
    let mut s = next(&prev);
    s.move_object(card, Zone::Graveyard, &reg);
    flags_transition(&prev, Some(&cast), &s, &reg,
        &format!("CastSpell #{} was refused but left traces", card.0));

    // Refused, but another spell appeared on the stack. The stack comparison
    // ignores triggers, so a trigger sitting there does not mask it.
    let mut p = prev.clone();
    let bear = named_permanent(&mut p, &reg, "Grizzly Bears", P0);
    p.pending_trigger_pushes_ap.clear();
    p.stack.push(StackEntry::Trigger(mtg_engine::triggers::PendingTrigger::new(
        mtg_engine::triggers::TriggerSource {
            id: bear, card_id: p.get_object(bear).unwrap().card_id, controller: P0,
            description: "a trigger".into(), chosen_targets: vec![], from_back_face: false,
        },
        mtg_engine::triggers::TriggerEvent::Upkeep,
    )));
    let mut s = next(&p);
    s.get_object_mut(other).unwrap().zone = Zone::Stack;
    s.stack.push(StackEntry::Spell(other));
    flags_transition(&p, Some(&cast), &s, &reg,
        &format!("CastSpell #{} was refused but left traces", card.0));

    // Waiting on a cost: the card stays where it is until the cost is paid.
    let mut waiting = next(&prev);
    waiting.pending_spell_cast = Some(stash(&prev, card));
    let mut s = waiting.clone();
    s.move_object(card, Zone::Stack, &reg);
    flags_transition(&prev, Some(&cast), &s, &reg,
        &format!("CastSpell #{} is waiting on a cost but the card moved", card.0));
}

/// CR 602.2: an activation goes on the stack, stops on a cost, or is refused
/// — and a refusal may have paid mana and tapped things on the way, nothing
/// more.
#[test]
fn an_activation_that_neither_went_on_the_stack_nor_backed_out_is_flagged() {
    let (mut prev, reg) = base();
    let land = named_permanent(&mut prev, &reg, "Forest", P0);
    let bear = spell_in_hand(&mut prev, &reg, "Grizzly Bears", P0);
    let activate = mtg_engine::actions::Action::ActivateAbility {
        object_id: land, ability_index: 0, targets: vec![], tap_plan: vec![],
        sacrifice: None, x_value: None, source_card_id: None,
    };

    clean_transition(&prev, Some(&activate), &next(&prev), &reg);

    let mut s = next(&prev);
    s.get_object_mut(bear).unwrap().zone = Zone::Stack;
    s.stack.push(StackEntry::Spell(bear));
    flags_transition(&prev, Some(&activate), &s, &reg,
        &format!("ActivateAbility #{}/0 neither went on the stack nor was refused cleanly", land.0));

    let mut s = next(&prev);
    s.events.push(GameEvent::CardDrawn { player: P0, object: bear });
    flags_transition(&prev, Some(&activate), &s, &reg,
        &format!("ActivateAbility #{}/0 neither went on the stack nor was refused cleanly", land.0));
}

/// CR 104.3a: conceding is losing, recorded as such. A concede that leaves
/// the player in the game, or blames something else, is the one action whose
/// whole effect is a single flag.
#[test]
fn a_concede_that_does_not_record_the_loss_is_flagged() {
    let (prev, reg) = base();
    let concede = mtg_engine::actions::Action::Concede;

    let mut s = next(&prev);
    s.player_loses(P0, mtg_engine::events::LossReason::Conceded);
    clean_transition(&prev, Some(&concede), &s, &reg);

    flags_transition(&prev, Some(&concede), &next(&prev), &reg,
        "p0 conceded but is not recorded as having lost that way");

    let mut s = next(&prev);
    s.player_loses(P0, mtg_engine::events::LossReason::LifeReachedZero);
    flags_transition(&prev, Some(&concede), &s, &reg,
        "p0 conceded but is not recorded as having lost that way");
}

/// A cast that was waiting on a cost either finishes as a cast spell or is
/// still waiting. Losing the stash while the card moves is a spell that was
/// never cast (no `SpellCast`, so nothing that watches casts ever hears it).
#[test]
fn a_pending_cast_that_vanishes_with_the_card_is_flagged() {
    let (mut prev, reg) = base();
    let card = spell_in_hand(&mut prev, &reg, "Grizzly Bears", P0);
    prev.pending_spell_cast = Some(stash(&prev, card));

    // The stash is only ever resolved by an answer to the prompt it is
    // waiting on (CR 601.2), so that is the action the contract is checked
    // against.
    let answer = mtg_engine::actions::Action::ResolveChoice {
        choice: mtg_engine::actions::ResolvedChoice::YesNoDecision(true),
    };

    // Finished: the card is on the stack and the cast was announced.
    let mut s = next(&prev);
    s.pending_spell_cast = None;
    s.move_object(card, Zone::Stack, &reg);
    s.stack.push(StackEntry::Spell(card));
    s.events.push(GameEvent::SpellCast { player: P0, object: card });
    *s.num_spells_cast_this_turn.entry(P0).or_insert(0) += 1;
    clean_transition(&prev, Some(&answer), &s, &reg);

    // The stash is gone, the card moved, and no cast was ever announced.
    let mut s = next(&prev);
    s.pending_spell_cast = None;
    s.move_object(card, Zone::Stack, &reg);
    s.stack.push(StackEntry::Spell(card));
    flags_transition(&prev, Some(&answer), &s, &reg,
        &format!("pending cast of #{} ended with the card moved but no SpellCast", card.0));
}

#[test]
fn transition_identity_and_monotone_rules_are_checked() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let cur = next(&prev);
    clean_transition(&prev, None, &cur, &reg);

    let mut s = cur.clone();
    s.get_object_mut(bear).unwrap().owner = P1;
    flags_transition(&prev, None, &s, &reg, "changed owner p0 -> p1 (CR 108.3)");

    let mut s = cur.clone();
    s.objects.remove(&bear);
    flags_transition(&prev, None, &s, &reg, "ceased to exist (CR 108.3)");

    let mut s = cur.clone();
    s.turn_number = 2;
    flags_transition(&prev, None, &s, &reg, "turn_number went back 3 -> 2");

    let mut s = cur.clone();
    s.get_object_mut(bear).unwrap().zone = Zone::Graveyard;
    flags_transition(&prev, None, &s, &reg, "without a zone change being counted (CR 400.7)");

    let mut s = cur.clone();
    s.get_player_mut(P0).land_plays_remaining = 1;
    let mut p = prev.clone();
    p.get_player_mut(P0).land_plays_remaining = 0;
    flags_transition(&p, None, &s, &reg, "regained a land drop mid-turn (CR 305.2)");

    let mut s = cur.clone();
    s.step = Step::Upkeep;
    flags_transition(&prev, None, &s, &reg, "step went back");
}

#[test]
fn transition_zone_and_status_ledgers_are_checked() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let mut cur = next(&prev);
    cur.move_object(bear, Zone::Graveyard, &reg);
    assert!(cur.events.iter().any(|e| matches!(e, GameEvent::ObjectMoved { .. })), "every move is announced");
    clean_transition(&prev, None, &cur, &reg);

    let mut s = cur.clone();
    s.events.retain(|e| !matches!(e, GameEvent::ObjectMoved { .. }));
    flags_transition(&prev, None, &s, &reg, "moved 1 time(s) but announced 0 (CR 400.7)");
    flags_transition(&prev, None, &s, &reg, "LeftBattlefield #");
    flags_transition(&prev, None, &s, &reg, "without the matching zone change");

    let mut s = next(&prev);
    s.get_object_mut(bear).unwrap().tapped = true;
    flags_transition(&prev, None, &s, &reg, "became tapped with no Tapped event");

    let mut s = next(&prev);
    let wolf = s.create_token_with_subtypes("", P0, 2, 2, vec![Color::Green], vec![CardType::Creature],
        vec![], vec!["Wolf".into()], &reg)[0];
    clean_transition(&prev, None, &s, &reg);
    s.events.retain(|e| !matches!(e, GameEvent::EnteredBattlefield { object, .. } if *object == wolf));
    flags_transition(&prev, None, &s, &reg, "appeared without entering the battlefield (CR 111.2)");

    let mut s = next(&prev);
    s.get_object_mut(bear).unwrap().damage_marked = 1;
    flags_transition(&prev, None, &s, &reg, "damage marked after 0 + 0 dealt (CR 120.3)");

    let mut s = next(&prev);
    s.get_object_mut(bear).unwrap().controller = P1;
    s.get_object_mut(bear).unwrap().summoning_sick = false;
    flags_transition(&prev, None, &s, &reg, "without summoning sickness (CR 302.6)");

    let mut s = next(&prev);
    s.get_player_mut(P1).life = 10;
    flags_transition(&prev, None, &s, &reg, "life 20 -> 10 with no LifeChanged (CR 119)");

    let mut s = next(&prev);
    s.get_player_mut(P0).mana_pool.mana.insert(ManaType::Green, 1);
    flags_transition(&prev, None, &s, &reg, "(CR 106.4)");

    let mut s = next(&prev);
    s.get_player_mut(P1).lost = true;
    s.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::LifeReachedZero);
    flags_transition(&prev, None, &s, &reg, "with no PlayerLost event");
}

#[test]
fn transition_action_contracts_are_checked() {
    let (mut prev, reg) = base();
    let land = spell_in_hand(&mut prev, &reg, "Forest", P0);
    prev.priority_player = Some(P0);
    let play = Action::PlayLand { object_id: land };
    let cur = mtg_engine::engine::submit_action(&prev, &play, &reg);
    assert_eq!(cur.submit_seq, prev.submit_seq + 1);
    clean_transition(&prev, Some(&play), &cur, &reg);

    let mut s = cur.clone();
    s.events.retain(|e| !matches!(e, GameEvent::LandPlayed { .. }));
    flags_transition(&prev, Some(&play), &s, &reg, "did not put the land from hand onto the battlefield with its event (CR 305.1)");

    let mut s = cur.clone();
    s.priority_player = Some(P1);
    flags_transition(&prev, Some(&play), &s, &reg, "PlayLand handed priority Some(PlayerId(0)) -> Some(PlayerId(1)) (CR 117.3c)");

    // A lone pass moves priority and nothing else.
    let mut p = prev.clone();
    p.consecutive_passes = 0;
    let pass = Action::PassPriority;
    let mut cur = mtg_engine::engine::submit_action(&p, &pass, &reg);
    cur.priority_player = Some(P1); // the loop hands priority over after the pass
    clean_transition(&p, Some(&pass), &cur, &reg);
    let mut s = cur.clone();
    s.get_object_mut(land).unwrap().tapped = true;
    s.events.push(GameEvent::Tapped { object: land });
    flags_transition(&p, Some(&pass), &s, &reg, "a lone pass by p0 produced 2 event(s)");
    flags_transition(&p, Some(&pass), &s, &reg, "changed the game (CR 117.4)");
}

// ── the legal action set ─────────────────────────────────────────────────

use mtg_engine::invariants::check_legal;

#[track_caller]
fn flags_legal(state: &GameState, acting: PlayerId, legal: &mtg_engine::engine::LegalActions, reg: &CardRegistry, needle: &str) {
    let v = check_legal(state, acting, legal, reg);
    assert!(v.iter().any(|m| m.contains(needle)), "expected a legal-set violation containing {needle:?}, got: {v:?}");
}

#[test]
fn the_priority_offer_shape_and_sorcery_timing_are_checked() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let land = spell_in_hand(&mut state, &reg, "Forest", P0);
    let creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    add_mana(&mut state, P0, &[(ManaType::Green, 2)]);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    assert!(legal.actions.iter().any(|a| matches!(a, Action::PlayLand { .. })), "precondition: a land drop is offered");
    assert_eq!(check_legal(&state, P0, &legal, &reg), Vec::<String>::new());

    flags_legal(&state, P1, &legal, &reg, "priority offer to p1 who does not hold priority (CR 117.1)");

    let mut l = legal.clone();
    l.actions.retain(|a| !matches!(a, Action::Concede));
    flags_legal(&state, P0, &l, &reg, "does not start with PassPriority and end with Concede");

    let mut l = legal.clone();
    l.actions.insert(1, Action::PlayLand { object_id: land });
    flags_legal(&state, P0, &l, &reg, "offered twice");

    // The same offers during combat are sorcery-speed violations.
    let mut s = state.clone();
    s.step = Step::BeginCombat;
    flags_legal(&s, P0, &legal, &reg, "outside a main phase with an empty stack on p0's turn (CR 305.1)");
    flags_legal(&s, P0, &legal, &reg, "at sorcery speed outside p0's main phase with an empty stack (CR 307.1)");

    let mut s = state.clone();
    s.get_player_mut(P0).land_plays_remaining = 0;
    flags_legal(&s, P0, &legal, &reg, "with no land drop left (CR 305.2)");

    let mut s = state.clone();
    s.get_object_mut(creature).unwrap().owner = P1;
    flags_legal(&s, P0, &legal, &reg, "owned by p1 offered to p0 (CR 601.3a)");

    let mut l = legal.clone();
    l.actions.insert(1, Action::ActivateManaAbility { object_id: bear, ability_index: 0 });
    flags_legal(&state, P0, &l, &reg, "offered but not available to p0 (CR 605.3a)");

    // The collapsed views the CLI and LLM players act through offer the
    // same game as the flat list.
    let mut l = legal.clone();
    l.castable_spells.clear();
    flags_legal(&state, P0, &l, &reg, "do not match the cast actions");
}

#[test]
fn combat_prompt_offers_are_checked_against_the_board() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let sick = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.get_object_mut(sick).unwrap().summoning_sick = true;
    state.step = Step::DeclareAttackers;
    state.awaiting_action = Some(AwaitingAction::DeclareAttackers);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    assert!(matches!(legal.combat_prompt, Some(mtg_engine::actions::CombatPrompt::ChooseAttackers { .. })));
    assert_eq!(check_legal(&state, P0, &legal, &reg), Vec::<String>::new());

    let mut l = legal.clone();
    if let Some(mtg_engine::actions::CombatPrompt::ChooseAttackers { eligible, .. }) = &mut l.combat_prompt {
        eligible.push(sick);
    }
    flags_legal(&state, P0, &l, &reg, "but the creatures able to attack are");
    let mut l = legal.clone();
    if let Some(mtg_engine::actions::CombatPrompt::ChooseAttackers { eligible, .. }) = &mut l.combat_prompt {
        eligible.clear();
    }
    flags_legal(&state, P0, &l, &reg, "(CR 508.1a)");
    let _ = bear;

    // Blockers: a flyer cannot be blocked by a ground creature.
    let (mut state, reg) = base();
    let flyer = named_permanent(&mut state, &reg, "Chapel Geist", P0);
    let ground = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.step = Step::DeclareAttackers;
    submit_declare_attackers(&mut state, &[(flyer, P1)], &reg);
    state.step = Step::DeclareBlockers;
    state.awaiting_action = Some(AwaitingAction::DeclareBlockers { defending_player: P1 });
    state.priority_player = Some(P1);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    assert!(matches!(legal.combat_prompt, Some(mtg_engine::actions::CombatPrompt::ChooseBlockers { .. })));
    assert_eq!(check_legal(&state, P1, &legal, &reg), Vec::<String>::new());
    let mut l = legal.clone();
    if let Some(mtg_engine::actions::CombatPrompt::ChooseBlockers { legal_blocks, .. }) = &mut l.combat_prompt {
        legal_blocks.entry(ground).or_default().push(flyer);
    }
    flags_legal(&state, P1, &l, &reg, "which evades it (CR 509.1b)");
}

#[test]
fn resolution_prompt_enumerations_are_checked() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let other = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice { player: P0, source: bear, choice: ResolutionChoiceKind::ChooseTarget {
        description: String::new(), options: vec![Target::Object(other)], optional: true,
        effect: PendingEffect::DestroyCreature { source_name: "x".into() } } });
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    assert_eq!(check_legal(&state, P0, &legal, &reg), Vec::<String>::new());

    let mut l = legal.clone();
    l.actions.pop();
    flags_legal(&state, P0, &l, &reg, "but the prompt enumerates to");
    flags_legal(&state, P1, &legal, &reg, "resolution prompt offered to p1, not p0");
}

// ── checks driven by the mutation audit ──────────────────────────────────

#[test]
fn untap_scope_theft_sickness_and_shuffles_are_checked() {
    let (mut state, reg) = base();
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Grizzly Bears", P1);

    // CR 502.3: the untap step touches only the active player's permanents.
    let mut s = state.clone();
    s.step = Step::Upkeep;
    s.events = vec![GameEvent::StepStarted { step: Step::Untap }, GameEvent::Untapped { object: mine },
        GameEvent::StepStarted { step: Step::Upkeep }];
    clean(&s, &reg);
    s.events[1] = GameEvent::Untapped { object: theirs };
    flags_core(&s, &reg, "the untap step untapped #");
    flags_core(&s, &reg, "which p0 does not control (CR 502.3)");

    // CR 302.6: a creature taken this turn is summoning sick.
    let mut s = state.clone();
    s.change_control(theirs, P0);
    let timestamp = s.next_control_timestamp();
    s.until_end_of_turn.push(TemporaryEffect::ChangeControl { target: theirs, controller: P0, timestamp });
    clean(&s, &reg);
    s.get_object_mut(theirs).unwrap().summoning_sick = false;
    flags_core(&s, &reg, "was taken from p1 this turn but is not summoning sick (CR 302.6)");

    // CR 701.20a: a library keeps its order unless shuffled.
    let mut prev = state.clone();
    for name in ["Island", "Swamp", "Plains"] {
        let c = spell_in_hand(&mut prev, &reg, name, P0);
        prev.get_object_mut(c).unwrap().zone = Zone::Library;
        prev.get_player_mut(P0).library_order.push(c);
    }
    let mut cur = next(&prev);
    cur.get_player_mut(P0).library_order.reverse();
    flags_transition(&prev, None, &cur, &reg, "p0's library was reordered without a shuffle (CR 701.20a)");
    cur.events.push(GameEvent::LibraryShuffled { player: P0 });
    clean_transition(&prev, None, &cur, &reg);
}

#[test]
fn the_legend_rule_keep_and_colored_payment_are_checked() {
    let (mut state, reg) = base();
    let a = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let b = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice { player: P0, source: a, choice: ResolutionChoiceKind::ChooseTarget {
        description: String::new(), options: vec![Target::Object(a), Target::Object(b)], optional: false,
        effect: PendingEffect::LegendRuleKeep { player: P0, legend_name: "Grimgrin, Corpse-Born".into() } } });
    state.priority_player = Some(P0);
    let keep = Action::ResolveChoice { choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(a))) };
    let cur = mtg_engine::engine::submit_action(&state, &keep, &reg);
    assert_eq!(cur.get_object(b).unwrap().zone, Zone::Graveyard, "precondition: the duplicate went to the graveyard");
    clean_transition(&state, Some(&keep), &cur, &reg);

    let mut s = cur.clone();
    s.move_object(b, Zone::Battlefield, &reg);
    flags_transition(&state, Some(&keep), &s, &reg, "was not kept but is still on the battlefield (CR 704.5j)");
    // Leaving the battlefield during the same action is legitimate (the kept
    // legend can still die), and so is changing hands (the loser may have
    // been the source of a control effect over the winner) — vanishing is not.
    let mut s = cur.clone();
    s.get_object_mut(a).unwrap().zone = Zone::Exile;
    s.get_object_mut(a).unwrap().zone_change_count += 1;
    s.events.push(GameEvent::ObjectMoved { object: a, from: Zone::Battlefield, to: Zone::Exile });
    flags_transition(&state, Some(&keep), &s, &reg, "the kept #");
    flags_transition(&state, Some(&keep), &s, &reg, "did not stay on the battlefield (CR 704.5j)");
    let mut s = cur.clone();
    s.move_object(a, Zone::Graveyard, &reg);
    assert!(!check_transition(&state, Some(&keep), &s, &reg).iter().any(|m| m.contains("704.5j")),
        "a kept legend that died in the same action is not a legend-rule violation");

    // CR 601.2h: casting spends the colored part of the cost.
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let pump = castable_spell(&mut prev, &reg, "Moment of Heroism", P0);
    add_mana(&mut prev, P0, &[(ManaType::White, 2)]);
    prev.priority_player = Some(P0);
    let cast = Action::CastSpell { object_id: pump, targets: vec![Target::Object(bear)], sacrifice: None, exile_count: None,
        exile_ids: vec![], alternative_cost: None, tap_plan: vec![] };
    let cur = mtg_engine::engine::submit_action(&prev, &cast, &reg);
    assert!(cur.events.iter().any(|e| matches!(e, GameEvent::SpellCast { .. })), "precondition: cast");
    clean_transition(&prev, Some(&cast), &cur, &reg);
    // The white pip never left the pool.
    let mut s = cur.clone();
    let unpaid = prev.get_player(P0).mana_pool.mana.get(&ManaType::White).copied().unwrap_or(0);
    s.get_player_mut(P0).mana_pool.mana.insert(ManaType::White, unpaid);
    flags_transition(&prev, Some(&cast), &s, &reg, "(CR 601.2h)");
}

// ── the gaps mutation testing found ──────────────────────────────────────

#[test]
fn a_cards_own_target_restriction_is_part_of_the_offer() {
    let (mut state, reg) = base();
    // Avacynian Priest taps a *non-Human* creature; the restriction lives in
    // the card, not in the shared target requirement, so an enumerator that
    // drops it offers Humans and nothing else here would notice.
    let priest = named_permanent(&mut state, &reg, "Avacynian Priest", P0);
    state.get_object_mut(priest).unwrap().summoning_sick = false;
    let human = named_permanent(&mut state, &reg, "Elder Cathar", P1);
    let wolf = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    add_mana(&mut state, P0, &[(ManaType::Colorless, 1)]);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    assert_eq!(check_legal(&state, P0, &legal, &reg), Vec::<String>::new());
    assert!(legal.actions.iter().any(|a| matches!(a, Action::ActivateAbility { object_id, targets, .. }
        if *object_id == priest && targets.contains(&Target::Object(wolf)))), "the non-Human is offered");

    let mut l = legal.clone();
    if let Some(a) = l.actions.iter_mut().find(|a| matches!(a, Action::ActivateAbility { object_id, .. } if *object_id == priest)) {
        if let Action::ActivateAbility { targets, .. } = a {
            *targets = vec![Target::Object(human)];
        }
    }
    flags_legal(&state, P0, &l, &reg, "which the card's own restriction rejects (CR 601.2c)");
}

#[test]
fn a_draw_comes_off_the_top_and_a_blocked_attacker_spares_the_player() {
    let (mut prev, reg) = base();
    for name in ["Island", "Swamp", "Plains"] {
        let c = spell_in_hand(&mut prev, &reg, name, P0);
        prev.get_object_mut(c).unwrap().zone = Zone::Library;
        prev.get_player_mut(P0).library_order.push(c);
    }
    let top = prev.get_player(P0).library_order[0];
    let bottom = *prev.get_player(P0).library_order.last().unwrap();

    // Drawing the top card is what a draw is.
    let mut cur = next(&prev);
    cur.move_object(top, Zone::Hand, &reg);
    cur.get_player_mut(P0).library_order.retain(|id| *id != top);
    cur.events.push(GameEvent::CardDrawn { player: P0, object: top });
    clean_transition(&prev, None, &cur, &reg);

    // Taking the bottom one instead preserves the order of what is left, so
    // only the draw rule sees it.
    let mut cur = next(&prev);
    cur.move_object(bottom, Zone::Hand, &reg);
    cur.get_player_mut(P0).library_order.retain(|id| *id != bottom);
    cur.events.push(GameEvent::CardDrawn { player: P0, object: bottom });
    flags_transition(&prev, None, &cur, &reg, "from below the top 1 of a library that starts");

    // CR 510.1c: a blocked attacker's damage goes to its blockers.
    let (mut state, reg) = base();
    let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let blocker = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.step = Step::DeclareAttackers;
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    mtg_engine::combat::declare_blockers(&mut state, &[(blocker, attacker)]);
    state.step = Step::CombatDamage;
    state.events = vec![GameEvent::CombatDamageDealt {
        source: attacker, target: DamageTarget::Player(P1), amount: 2 }];
    flags_core(&state, &reg, "a blocked attacker without trample reached the player (CR 510.1c)");
}

#[test]
fn the_whole_cost_leaves_the_pool_not_just_its_colored_part() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let pump = castable_spell(&mut prev, &reg, "Moment of Heroism", P0);
    add_mana(&mut prev, P0, &[(ManaType::White, 2)]);
    prev.priority_player = Some(P0);
    let cast = Action::CastSpell { object_id: pump, targets: vec![Target::Object(bear)], sacrifice: None,
        exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] };
    let cur = mtg_engine::engine::submit_action(&prev, &cast, &reg);
    clean_transition(&prev, Some(&cast), &cur, &reg);

    // The generic pip never left: the per-colour ledger is satisfied and only
    // the total catches it.
    let mut s = cur.clone();
    let pool = &mut s.get_player_mut(P0).mana_pool.mana;
    let white = pool.get(&ManaType::White).copied().unwrap_or(0);
    pool.insert(ManaType::White, white + 1);
    flags_transition(&prev, Some(&cast), &s, &reg, "for a total cost of");
    flags_transition(&prev, Some(&cast), &s, &reg, "(CR 601.2h)");
}

// ── the pass contract and the trigger ledger ─────────────────────────────

/// The state the game loop reaches after a `PassPriority`. `submit_action`
/// only counts the pass; moving priority, resolving the top of the stack
/// and ending the step are the loop's half of CR 117.4, and the transition
/// checker judges the pair together.
fn after_pass(prev: &GameState, reg: &CardRegistry) -> GameState {
    let mut state = mtg_engine::engine::submit_action(prev, &Action::PassPriority, reg);
    let n = u32::try_from(prev.players.len()).unwrap_or(u32::MAX);
    if state.consecutive_passes >= n {
        if state.stack.is_empty() {
            state.priority_player = None;
            mtg_engine::engine::advance_step(&mut state, reg);
        } else {
            mtg_engine::stack::resolve_top_of_stack(&mut state, reg);
            state.consecutive_passes = 0;
            state.priority_player = Some(state.active_player);
        }
    } else if let Some(current) = state.priority_player {
        state.priority_player = Some(state.next_player(current));
    }
    state
}

#[track_caller]
fn no_transition_flag(prev: &GameState, action: Option<&Action>, cur: &GameState,
                      reg: &CardRegistry, needle: &str) {
    let v = check_transition(prev, action, cur, reg);
    assert!(!v.iter().any(|m| m.contains(needle)),
        "expected no transition violation containing {needle:?}, got: {v:?}");
}

/// CR 117.4: a lone pass moves priority and changes nothing else.
///
/// `pass_contract` is the transition checker's statement of that, and it is
/// the clause the fuzzer leans on to notice a pass that quietly did
/// something. Each half of its "left passes=N priority=P prompt=B" test is
/// its own way for a pass to be wrong.
#[test]
fn a_lone_pass_must_move_priority_and_nothing_else() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    prev.priority_player = Some(P0);
    prev.consecutive_passes = 0;
    let pass = Action::PassPriority;
    let cur = after_pass(&prev, &reg);
    assert_eq!(cur.priority_player, Some(P1), "precondition: priority moved");
    clean_transition(&prev, Some(&pass), &cur, &reg);

    // The one event has to be the passer's own.
    let mut s = cur.clone();
    s.events = vec![GameEvent::PriorityPassed { player: P1 }];
    flags_transition(&prev, Some(&pass), &s, &reg, "a lone pass by p0 produced 1 event(s)");

    // Passes stand at exactly one afterwards.
    let mut s = cur.clone();
    s.consecutive_passes = 2;
    flags_transition(&prev, Some(&pass), &s, &reg, "left passes=2");

    // Priority is with the other player.
    let mut s = cur.clone();
    s.priority_player = Some(P0);
    flags_transition(&prev, Some(&pass), &s, &reg, "priority=Some(PlayerId(0))");

    // And a pass raises no prompt.
    let mut s = cur.clone();
    s.awaiting_action = Some(AwaitingAction::MulliganDecision { player: P0 });
    flags_transition(&prev, Some(&pass), &s, &reg, "prompt=true");

    // "Changed nothing else" is judged on a digest of the whole position,
    // counters included — a pass that quietly grew a +1/+1 counter is
    // exactly the corruption this clause exists to catch.
    let mut s = cur.clone();
    s.add_counters(bear, CounterType::PlusOnePlusOne, 1);
    flags_transition(&prev, Some(&pass), &s, &reg, "changed the game (CR 117.4)");
}

/// CR 117.4 again, for the second pass: on an empty stack it ends the step,
/// and over a stack it resolves the top.
#[test]
fn the_pass_that_empties_the_stack_must_resolve_its_top() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let pump = castable_spell(&mut prev, &reg, "Moment of Heroism", P0);
    add_mana(&mut prev, P0, &[(ManaType::White, 2)]);
    prev.priority_player = Some(P0);
    let prev = cast_onto_stack(&prev, &reg, pump, vec![Target::Object(bear)]);

    // Both players have passed with the spell on the stack: this pass
    // resolves it.
    let mut prev = prev;
    prev.consecutive_passes = 1;
    prev.priority_player = Some(P0);
    let pass = Action::PassPriority;
    let cur = after_pass(&prev, &reg);
    assert!(cur.stack.is_empty(), "precondition: the pass resolved the spell");
    clean_transition(&prev, Some(&pass), &cur, &reg);

    // The pass count resets.
    let mut s = cur.clone();
    s.consecutive_passes = 1;
    flags_transition(&prev, Some(&pass), &s, &reg, "passes stand at 1 after everyone passed");

    // The resolved spell is off the stack.
    let mut s = cur.clone();
    s.stack = prev.stack.clone();
    flags_transition(&prev, Some(&pass), &s, &reg, "is still on the stack after resolving");

    // Priority goes back to the active player (CR 117.3b).
    let mut s = cur.clone();
    s.priority_player = Some(P1);
    flags_transition(&prev, Some(&pass), &s, &reg, "(CR 117.3b)");

    // And the spell actually resolved rather than being lost.
    let mut s = cur.clone();
    s.events.retain(|e| !matches!(e, GameEvent::SpellResolved { .. }));
    s.get_object_mut(pump).unwrap().zone = Zone::Stack;
    flags_transition(&prev, Some(&pass), &s, &reg, "left the top of the stack without resolving");
    flags_transition(&prev, Some(&pass), &s, &reg,
        "resolved but is still in the stack zone with no resolution in progress");

    // A card still in the stack zone is fine while its resolution is in
    // progress — that is what a prompt raised mid-resolution looks like.
    s.resolving_spell = Some(pump);
    no_transition_flag(&prev, Some(&pass), &s, &reg, "still in the stack zone");
    no_transition_flag(&prev, Some(&pass), &s, &reg, "left the top of the stack without resolving");

    // A spell that DID resolve and is somehow still in the stack zone is
    // the first complaint, not the second: it did not leave without
    // resolving, it resolved and stayed.
    let mut s = cur.clone();
    s.get_object_mut(pump).unwrap().zone = Zone::Stack;
    flags_transition(&prev, Some(&pass), &s, &reg,
        "resolved but is still in the stack zone with no resolution in progress");
    no_transition_flag(&prev, Some(&pass), &s, &reg,
        "left the top of the stack without resolving");
}

/// Resolution takes the top off and may remove things under it; it never
/// adds a non-trigger entry or reorders the survivors (CR 608.2n).
#[test]
fn resolving_the_top_leaves_a_subsequence_of_what_was_under_it() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let first = castable_spell(&mut prev, &reg, "Moment of Heroism", P0);
    let second = castable_spell(&mut prev, &reg, "Moment of Heroism", P0);
    add_mana(&mut prev, P0, &[(ManaType::White, 4)]);
    prev.priority_player = Some(P0);
    let prev = cast_onto_stack(&prev, &reg, first, vec![Target::Object(bear)]);
    let mut prev = cast_onto_stack(&prev, &reg, second, vec![Target::Object(bear)]);
    assert_eq!(prev.stack.len(), 2, "precondition: two spells on the stack");

    prev.consecutive_passes = 1;
    prev.priority_player = Some(P0);
    let pass = Action::PassPriority;
    let cur = after_pass(&prev, &reg);
    assert_eq!(cur.stack.len(), 1, "precondition: the top one resolved");
    clean_transition(&prev, Some(&pass), &cur, &reg);

    // The top left in place: not a subsequence of what was under it, and
    // still on the stack besides.
    let mut s = cur.clone();
    s.stack = prev.stack.clone();
    flags_transition(&prev, Some(&pass), &s, &reg, "(CR 608.2n)");
}

/// CR 117.4 on an empty stack: the pass walks the turn forward, and where
/// the walk stops is where the state says it is.
#[test]
fn the_pass_that_ends_a_step_must_start_one() {
    let (mut prev, reg) = base();
    named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    prev.consecutive_passes = 1;
    prev.priority_player = Some(P0);
    let pass = Action::PassPriority;
    let cur = after_pass(&prev, &reg);
    assert_ne!(cur.step, prev.step, "precondition: the pass ended the step");
    clean_transition(&prev, Some(&pass), &cur, &reg);

    let mut s = cur.clone();
    s.events.retain(|e| !matches!(e, GameEvent::StepStarted { .. }));
    flags_transition(&prev, Some(&pass), &s, &reg,
        "everyone passed on an empty stack but no step started (CR 117.4)");

    let mut s = cur.clone();
    s.step = Step::Cleanup;
    flags_transition(&prev, Some(&pass), &s, &reg, "the step walk ended at");
}

/// The pass contract is a statement about a pass in a game that is still
/// running, at a point where nobody is being asked anything. Both of its
/// stand-downs are load-bearing: a pass that ended the game changes the
/// position by definition, and a pass with a prompt outstanding is not the
/// second pass of a round.
#[test]
fn the_pass_contract_stands_down_once_the_game_is_over_or_a_prompt_is_up() {
    let (mut prev, reg) = base();
    named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    prev.priority_player = Some(P0);
    prev.consecutive_passes = 0;
    let pass = Action::PassPriority;

    // The baseline: a pass that changed the position is a violation.
    let mut s = after_pass(&prev, &reg);
    s.get_player_mut(P1).life = 3;
    flags_transition(&prev, Some(&pass), &s, &reg, "changed the game (CR 117.4)");

    // The same change with the game over is the game ending, not a pass
    // doing something.
    let mut s = after_pass(&prev, &reg);
    s.get_player_mut(P1).life = 3;
    s.get_player_mut(P1).lost = true;
    s.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::LifeReachedZero);
    s.result = Some(mtg_engine::state::GameResult::Winner(P0));
    no_transition_flag(&prev, Some(&pass), &s, &reg, "changed the game (CR 117.4)");

    // A pass while a prompt is outstanding is neither of the two passes the
    // contract describes — not the first of a round, and not the second.
    for passes in [0, 1] {
        let mut p = prev.clone();
        p.consecutive_passes = passes;
        p.awaiting_action = Some(AwaitingAction::MulliganDecision { player: P0 });
        let s = mtg_engine::engine::submit_action(&p, &pass, &reg);
        no_transition_flag(&p, Some(&pass), &s, &reg, "after everyone passed");
        no_transition_flag(&p, Some(&pass), &s, &reg, "a lone pass by");
    }
}

/// CR 608.2/117.3b: the pass that resolves an ACTIVATED ability leaves the
/// state agreeing with what that ability was activated with. The spell half
/// of the same clause had a test; the ability half — the announced X and
/// the sacrifice paid — had none.
#[test]
fn the_pass_that_resolves_an_ability_matches_what_it_was_activated_with() {
    let (mut prev, reg) = base();
    let priest = named_permanent(&mut prev, &reg, "Avacynian Priest", P0);
    prev.get_object_mut(priest).unwrap().summoning_sick = false;
    let victim = named_permanent(&mut prev, &reg, "Grizzly Bears", P1);
    add_mana(&mut prev, P0, &[(ManaType::White, 1)]);
    prev.priority_player = Some(P0);
    let mut prev = activate_onto_stack(&prev, &reg, priest, Some(Target::Object(victim)));
    assert!(matches!(prev.stack.last(), Some(StackEntry::Ability { .. })),
        "precondition: the ability is on the stack");

    prev.consecutive_passes = 1;
    prev.priority_player = Some(P0);
    let pass = Action::PassPriority;
    let cur = after_pass(&prev, &reg);
    assert!(cur.stack.is_empty(), "precondition: the pass resolved the ability");
    clean_transition(&prev, Some(&pass), &cur, &reg);

    // Each half of the pairing on its own.
    let mut s = cur.clone();
    s.last_activated_x_value = Some(3);
    flags_transition(&prev, Some(&pass), &s, &reg, "ability resolved with X=");
    let mut s = cur.clone();
    s.last_activated_sacrifice = Some(victim);
    flags_transition(&prev, Some(&pass), &s, &reg, "ability resolved with X=");

    // CR 117.3b: after a resolution the active player gets priority.
    let mut s = cur.clone();
    s.priority_player = Some(P1);
    flags_transition(&prev, Some(&pass), &s, &reg, "(CR 117.3b)");
}

/// CR 603.2: an ability triggers when its event happens. Every trigger that
/// appeared over a transition therefore has that event in the buffer — and
/// it has to be the event about *this* trigger's object, not merely an
/// event of the right kind.
#[test]
fn every_trigger_that_appeared_names_the_event_that_made_it() {
    use mtg_engine::triggers::{DeadCreature, PendingTrigger, TriggerEvent, TriggerSource};

    let (mut prev, reg) = base();
    let watcher = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let other = named_permanent(&mut prev, &reg, "Grizzly Bears", P1);
    let card_id = prev.get_object(watcher).unwrap().card_id;
    let other_card = prev.get_object(other).unwrap().card_id;
    let spell = spell_in_hand(&mut prev, &reg, "Moment of Heroism", P0);

    let died = |id: ObjectId, cid: mtg_engine::ids::CardId, who: PlayerId| GameEvent::CreatureDied {
        object: id, name: String::new(), card_id: cid, controller: who, damaged_by: vec![],
        last_known_toughness: 2, is_token: false, subtypes: vec![] };
    let dead_creature = DeadCreature {
        name: String::new(),
        id: other, controller: P1, damaged_by: vec![], toughness: 2,
        is_token: false, subtypes: vec![] };

    // Each row: the trigger, the event that witnesses it, and an event of
    // the same kind that names something else and must NOT witness it.
    let cases: Vec<(&str, TriggerEvent, GameEvent, GameEvent)> = vec![
        ("its own death", TriggerEvent::SelfDies,
            died(watcher, card_id, P0), died(other, other_card, P1)),
        ("another creature's death", TriggerEvent::CreatureDied { dead: dead_creature },
            died(other, other_card, P1), died(watcher, card_id, P0)),
        ("its own arrival", TriggerEvent::SelfEntered,
            GameEvent::EnteredBattlefield { object: watcher, controller: P0 },
            GameEvent::EnteredBattlefield { object: other, controller: P1 }),
        ("another creature's arrival",
            TriggerEvent::CreatureEntered { entered: other, entered_controller: P1 },
            GameEvent::EnteredBattlefield { object: other, controller: P1 },
            GameEvent::EnteredBattlefield { object: watcher, controller: P0 }),
        ("an attack", TriggerEvent::Attacks { attacker: watcher, defending_player: P1 },
            GameEvent::AttackersDeclared { attackers: vec![(watcher, P1)] },
            GameEvent::AttackersDeclared { attackers: vec![(other, P0)] }),
        ("a spell being cast", TriggerEvent::SpellCast { caster: P0, spell_id: spell },
            GameEvent::SpellCast { player: P0, object: spell },
            GameEvent::SpellCast { player: P0, object: other }),
        ("leaving the battlefield", TriggerEvent::LeftBattlefield,
            GameEvent::LeftBattlefield { object: watcher, to: Zone::Graveyard, last_controller: P0 },
            GameEvent::LeftBattlefield { object: other, to: Zone::Graveyard, last_controller: P1 }),
    ];

    for (what, event, witness, decoy) in cases {
        let trigger = PendingTrigger::new(
            TriggerSource::new(watcher, card_id, P0, "a triggered ability"), event);

        let mut s = next(&prev);
        s.pending_triggers.push(trigger.clone());
        flags_transition(&prev, None, &s, &reg,
            "appeared with no event to trigger it (CR 603.2)");

        let mut s = next(&prev);
        s.pending_triggers.push(trigger.clone());
        s.events.push(witness);
        no_transition_flag(&prev, None, &s, &reg,
            "appeared with no event to trigger it (CR 603.2)");

        let mut s = next(&prev);
        s.pending_triggers.push(trigger);
        s.events.push(decoy);
        let v = check_transition(&prev, None, &s, &reg);
        assert!(v.iter().any(|m| m.contains("appeared with no event to trigger it")),
            "a trigger on {what} is not witnessed by the same event about \
             something else, got: {v:?}");
    }
}

/// A turn-based trigger needs both halves: the step started, and the state
/// is in that step. Neither alone is the event (CR 603.2).
#[test]
fn a_step_trigger_needs_the_step_it_names() {
    use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};

    let (mut prev, reg) = base();
    let watcher = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let card_id = prev.get_object(watcher).unwrap().card_id;

    for (event, step) in [(TriggerEvent::Upkeep, Step::Upkeep),
                          (TriggerEvent::EndStep, Step::EndStep),
                          (TriggerEvent::EndCombat, Step::EndCombat)] {
        let trigger = PendingTrigger::new(
            TriggerSource::new(watcher, card_id, P0, "at the beginning of"), event);

        // Both halves: witnessed.
        let mut s = next(&prev);
        s.step = step;
        s.pending_triggers.push(trigger.clone());
        s.events.push(GameEvent::StepStarted { step });
        no_transition_flag(&prev, None, &s, &reg, "no event to trigger it");

        // The step started but the state moved on: not this step's trigger.
        let mut s = next(&prev);
        s.pending_triggers.push(trigger.clone());
        s.events.push(GameEvent::StepStarted { step });
        flags_transition(&prev, None, &s, &reg, "no event to trigger it");

        // The state is in the step but nothing started it.
        let mut s = next(&prev);
        s.step = step;
        s.pending_triggers.push(trigger);
        flags_transition(&prev, None, &s, &reg, "no event to trigger it");
    }
}

/// The ledger counts: a trigger already queued before the transition is not
/// a new one, and a *second* copy of it is.
#[test]
fn the_trigger_ledger_counts_copies_rather_than_kinds() {
    use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};

    let (mut prev, reg) = base();
    let watcher = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let card_id = prev.get_object(watcher).unwrap().card_id;
    let trigger = PendingTrigger::new(
        TriggerSource::new(watcher, card_id, P0, "a triggered ability"),
        TriggerEvent::SelfDies);

    prev.pending_triggers.push(trigger.clone());

    // Carried over unchanged: its event was witnessed on the transition
    // that made it, not on this one.
    let s = next(&prev);
    no_transition_flag(&prev, None, &s, &reg, "no event to trigger it");

    // A second copy is a second trigger, and needs its own event.
    let mut s = next(&prev);
    s.pending_triggers.push(trigger.clone());
    flags_transition(&prev, None, &s, &reg, "no event to trigger it");

    // The queue it sits in doesn't matter — moving it from the pending
    // bucket onto the stack is not a new trigger.
    let mut s = next(&prev);
    s.pending_triggers.clear();
    s.pending_trigger_pushes_ap.push(trigger);
    no_transition_flag(&prev, None, &s, &reg, "no event to trigger it");
}

/// The ledger is not applied while a prompt is up or after the game is
/// over: mid-prompt the event buffer belongs to an older transition, and a
/// finished game has stopped accounting.
#[test]
fn the_trigger_ledger_stands_down_mid_prompt_and_after_the_game() {
    use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};

    let (mut prev, reg) = base();
    let watcher = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let card_id = prev.get_object(watcher).unwrap().card_id;
    let trigger = PendingTrigger::new(
        TriggerSource::new(watcher, card_id, P0, "a triggered ability"),
        TriggerEvent::SelfDies);

    // The baseline: unwitnessed, and flagged.
    let mut s = next(&prev);
    s.pending_triggers.push(trigger.clone());
    flags_transition(&prev, None, &s, &reg, "no event to trigger it");

    // A prompt was up before the transition.
    let mut p = prev.clone();
    p.awaiting_action = Some(AwaitingAction::MulliganDecision { player: P0 });
    let mut s = next(&p);
    s.pending_triggers.push(trigger.clone());
    no_transition_flag(&p, None, &s, &reg, "no event to trigger it");

    // The game is over.
    let mut s = next(&prev);
    s.pending_triggers.push(trigger);
    s.result = Some(mtg_engine::state::GameResult::Winner(P0));
    no_transition_flag(&prev, None, &s, &reg, "no event to trigger it");
}

// ── turn structure, the result, and combat bookkeeping ───────────────────

/// The clauses of the turn-structure family that had no violating state of
/// their own. CR 103.7a (the first turn skips its draw step), CR 104.4a (a
/// draw is a draw for everybody), CR 104.3a (a player who has left the game
/// is never asked anything) and CR 603.7 (the delayed exiles live inside
/// combat).
#[test]
fn turn_and_result_clauses_each_have_a_violating_state() {
    let (mut state, reg) = base();
    named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    clean(&state, &reg);

    // Turn one is a real turn: the counter starts at 1, not 0.
    let mut first = state.clone();
    first.turn_number = 1;
    first.is_first_turn = true;
    clean(&first, &reg);
    let mut s = first.clone();
    s.turn_number = 0;
    s.is_first_turn = false;
    flags_core(&s, &reg, "turn_number is 0");

    // CR 103.7a: the player who goes first skips their draw step.
    let mut s = first.clone();
    s.step = Step::Draw;
    flags_core(&s, &reg, "a draw step on the first turn (CR 103.7a)");
    let mut s = state.clone();
    s.step = Step::Draw;
    assert!(!check_core(&as_collected(&s), &reg).iter()
        .any(|m| m.contains("draw step on the first turn")),
        "a draw step on turn 3 is an ordinary draw step");

    // A two-player engine has two players. (Nothing else in the state may
    // name the seat that goes, so this is checked on a bare board.)
    let mut s = base().0;
    s.players.pop();
    flags_core(&s, &reg, "1 players in a two-player engine");

    // CR 104.4a: a draw is a draw for every player.
    let mut s = state.clone();
    s.result = Some(mtg_engine::state::GameResult::Draw);
    flags_settled(&s, &reg, "a draw with a player who has not lost (CR 104.4a)");

    // A winner has to be a player at all.
    let mut s = state.clone();
    s.result = Some(mtg_engine::state::GameResult::Winner(
        mtg_engine::ids::PlayerId(u8::try_from(s.players.len()).unwrap())));
    flags_settled(&s, &reg, "is not a player");

    // CR 104.2a: the winner is the one who did not lose, and everyone else
    // did. A clean win says nothing; each half of that on its own does.
    let mut won = state.clone();
    won.get_player_mut(P1).lost = true;
    won.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::Conceded);
    won.result = Some(mtg_engine::state::GameResult::Winner(P0));
    let needle = "the loss flags say otherwise";
    assert!(!check_settled(&as_collected(&won), &reg).iter().any(|m| m.contains(needle)),
        "p0 won and p1 lost, which is what the flags say: {:?}",
        check_settled(&as_collected(&won), &reg));

    // The winner lost too.
    let mut s = won.clone();
    s.get_player_mut(P0).lost = true;
    s.get_player_mut(P0).loss_reason = Some(mtg_engine::events::LossReason::Conceded);
    flags_settled(&s, &reg, needle);

    // Or somebody else did not lose.
    let mut s = won.clone();
    s.get_player_mut(P1).lost = false;
    s.get_player_mut(P1).loss_reason = None;
    flags_settled(&s, &reg, needle);

    // "Lost because the opponent won" names the opponent who won.
    let mut s = state.clone();
    s.get_player_mut(P1).lost = true;
    s.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::OpponentWon);
    s.result = Some(mtg_engine::state::GameResult::Winner(P1));
    flags_settled(&s, &reg, "lost because the opponent won, but the result is");
    s.result = Some(mtg_engine::state::GameResult::Winner(P0));
    assert!(!check_settled(&as_collected(&s), &reg).iter()
        .any(|m| m.contains("lost because the opponent won")),
        "p1 lost to p0's win, which is what the result says");

    // CR 104.3a: a player who has left the game is not prompted either.
    let mut s = state.clone();
    s.get_player_mut(P1).lost = true;
    s.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::Conceded);
    s.result = Some(mtg_engine::state::GameResult::Winner(P0));
    s.priority_player = None;
    s.awaiting_action = Some(AwaitingAction::MulliganDecision { player: P1 });
    flags_settled(&s, &reg, "p1 is prompted after losing");

    // CR 603.7: the delayed end-of-combat exiles exist only inside combat,
    // name a card the registry knows, and never name their own source.
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let source = state.objects_in_id_order()[0].id;
    let source_card = state.get_object(source).unwrap().card_id;
    let exile = |target: ObjectId, card: mtg_engine::ids::CardId| mtg_engine::state::EndOfCombatExileEntry {
        target_id: target,
        source_id: source,
        source_card_id: card,
        controller: P0,
        description: "exile it at end of combat".into(),
    };

    let mut s = state.clone();
    s.end_of_combat_exiles.push(exile(bear, source_card));
    flags_core(&s, &reg, "end-of-combat exiles scheduled outside combat");

    let mut s = state.clone();
    s.end_of_combat_exiles.push(exile(source, source_card));
    flags_core(&s, &reg, "schedules its own end-of-combat exile");

    let mut s = state.clone();
    s.end_of_combat_exiles.push(exile(bear, mtg_engine::ids::CardId(424_242)));
    flags_core(&s, &reg, "from unregistered card 424242");

    // "Inside combat" is both halves: a combat state AND a combat step.
    let in_combat = |step: Step, combat: bool| {
        let mut s = state.clone();
        s.step = step;
        if combat {
            let mut c = mtg_engine::state::CombatState::new();
            c.any_attackers_declared = true;
            c.attackers.insert(bear, P0);
            s.combat = Some(c);
        }
        s.end_of_combat_exiles.push(exile(bear, source_card));
        s
    };
    let needle = "end-of-combat exiles scheduled outside combat";
    assert!(!check_core(&as_collected(&in_combat(Step::DeclareBlockers, true)), &reg)
        .iter().any(|m| m.contains(needle)),
        "a delayed exile inside combat is where it belongs");
    flags_core(&in_combat(Step::PrecombatMain, true), &reg, needle);
    flags_core(&in_combat(Step::DeclareBlockers, false), &reg, needle);
}

/// Combat bookkeeping the suite never corrupted: an attacker that is gone,
/// blockers without a blocked attacker, a permanent attacking itself, and
/// first-strike damage recorded outside the damage step.
#[test]
fn combat_bookkeeping_clauses_each_have_a_violating_state() {
    let (mut state, reg) = base();
    let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let blocker = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.step = Step::DeclareBlockers;
    let mut c = mtg_engine::state::CombatState::new();
    c.any_attackers_declared = true;
    c.attackers.insert(attacker, P1);
    c.blocker_assignments.insert(attacker, vec![blocker]);
    c.blocked_attackers.insert(attacker);
    state.combat = Some(c);
    clean(&state, &reg);

    // CR 509.1h: an attacker with blockers is blocked.
    let mut s = state.clone();
    s.combat.as_mut().unwrap().blocked_attackers.clear();
    flags_settled(&s, &reg, "has blockers but is not marked blocked (CR 509.1h)");

    // A combatant that no longer exists.
    let mut s = state.clone();
    s.objects.remove(&blocker);
    flags_settled(&s, &reg, "does not exist but is still in combat");

    // Attackers with nothing declared.
    let mut s = state.clone();
    s.combat.as_mut().unwrap().any_attackers_declared = false;
    flags_settled(&s, &reg, "attackers in combat but none declared");

    // CR 510.4: first-strike damage is recorded in the damage step only.
    let mut s = state.clone();
    s.combat.as_mut().unwrap().dealt_first_strike.insert(attacker);
    flags_settled(&s, &reg, "first-strike damage recorded in DeclareBlockers");

    // CR 510.4 again: a second damage step is pending only inside the damage
    // step of a combat that is happening — each half on its own.
    let pending = "second combat damage step pending";
    let mut ok = state.clone();
    ok.step = Step::CombatDamage;
    ok.combat_damage_step_pending = true;
    assert!(!check_settled(&as_collected(&ok), &reg).iter().any(|m| m.contains(pending)),
        "the first of two damage steps is where a second one is pending: {:?}",
        check_settled(&as_collected(&ok), &reg));

    let mut s = ok.clone();
    s.step = Step::DeclareBlockers;
    flags_settled(&s, &reg, pending);

    let mut s = ok.clone();
    s.combat = None;
    flags_settled(&s, &reg, pending);

    // A permanent cannot attack itself.
    let mut s = state.clone();
    s.combat.as_mut().unwrap().planeswalker_defenders.insert(attacker, attacker);
    flags_settled(&s, &reg, "attacks itself");

    // The declare-attackers step is past its declaration once the prompt is
    // answered, so a missing combat state there is a lost declaration.
    let mut s = state.clone();
    s.step = Step::DeclareAttackers;
    s.combat = None;
    flags_settled(&s, &reg, "declare attackers step past its declaration with no combat state");
    s.awaiting_action = Some(AwaitingAction::DeclareAttackers);
    assert!(!check_settled(&as_collected(&s), &reg).iter()
        .any(|m| m.contains("declare attackers step past its declaration")),
        "the step before the declaration has no combat state yet");
}

/// Every player id stored in game-level bookkeeping names a player. The
/// checker runs on states that are already corrupt, so each of these is a
/// range check standing between it and a panic.
#[test]
fn every_player_id_in_the_bookkeeping_is_range_checked() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let source = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let ghost = PlayerId(u8::try_from(state.players.len()).unwrap());

    let combat_for = |d: PlayerId| {
        let mut c = mtg_engine::state::CombatState::new();
        c.any_attackers_declared = true;
        c.attackers.insert(bear, d);
        c
    };

    // Each corruption, and the same structure naming a real seat.
    let cases: Vec<(&str, Box<dyn Fn(&mut GameState, PlayerId)>)> = vec![
        ("attacker #1 attacks p", Box::new(move |s: &mut GameState, p: PlayerId| {
            s.step = Step::DeclareBlockers;
            s.combat = Some(combat_for(p));
        })),
        ("control effect over #", Box::new(move |s: &mut GameState, p: PlayerId| {
            s.control_effects.push(mtg_engine::state::ControlEffect {
                object: bear, controller: p, original_controller: P0,
                source, source_controller: P0, timestamp: 1 });
        })),
        ("queued bottoming for p", Box::new(|s: &mut GameState, p: PlayerId| {
            s.pending_mulligan_bottoms.push((p, 1));
        })),
        ("spell count for p", Box::new(|s: &mut GameState, p: PlayerId| {
            s.num_spells_cast_this_turn.insert(p, 1);
        })),
        ("control change of #", Box::new(move |s: &mut GameState, p: PlayerId| {
            s.until_end_of_turn.push(TemporaryEffect::ChangeControl {
                target: bear, controller: p, timestamp: 1 });
        })),
        ("blockers prompt for p", Box::new(|s: &mut GameState, p: PlayerId| {
            s.step = Step::DeclareBlockers;
            s.awaiting_action = Some(AwaitingAction::DeclareBlockers { defending_player: p });
        })),
        ("end-of-combat exile of #", Box::new(move |s: &mut GameState, p: PlayerId| {
            s.step = Step::DeclareAttackers;
            s.combat = Some(combat_for(P1));
            s.end_of_combat_exiles.push(mtg_engine::state::EndOfCombatExileEntry {
                target_id: bear, source_id: source,
                source_card_id: s.get_object(source).unwrap().card_id,
                controller: p, description: "exile it at end of combat".into() });
        })),
    ];

    for (needle, corrupt) in cases {
        let mut s = state.clone();
        corrupt(&mut s, ghost);
        let v = check_core(&s, &reg);
        assert!(v.iter().any(|m| m.contains(needle) && m.contains("who is not a player")),
            "{needle:?} for a seat that does not exist, got: {v:?}");

        let mut s = state.clone();
        corrupt(&mut s, P1);
        assert!(!check_core(&s, &reg).iter().any(|m| m.contains("who is not a player")),
            "{needle:?} naming a real seat is not a range violation");
    }
}

/// CR 108.3/111.7/707.2: a card is the same card from one decision point to
/// the next. Only a token may appear or vanish, only a copy or a transform
/// may change what a permanent is, and a new object gets a fresh id.
#[test]
fn an_objects_identity_survives_every_transition() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let cur = next(&prev);
    clean_transition(&prev, None, &cur, &reg);

    let mut s = cur.clone();
    s.get_object_mut(bear).unwrap().is_token = true;
    flags_transition(&prev, None, &s, &reg, "changed token-ness");

    // CR 707.2: the printed card underneath a copy is fixed.
    let other = reg.get_id_by_name("Forest").unwrap();
    let mut s = cur.clone();
    s.get_object_mut(bear).unwrap().copy_grantor = Some(other);
    flags_transition(&prev, None, &s, &reg, "(CR 707.2)");

    // A rename with no copy and no transform behind it.
    let mut s = cur.clone();
    s.get_object_mut(bear).unwrap().name = "Something Else".into();
    flags_transition(&prev, None, &s, &reg, "without a copy or transform");

    // The card itself changing, with neither a copy nor a zone change.
    let mut s = cur.clone();
    s.get_object_mut(bear).unwrap().card_id = other;
    flags_transition(&prev, None, &s, &reg, "without a copy or a zone change");

    // A card that was not there before is a card that appeared from nowhere.
    let mut s = cur.clone();
    let appeared = s.create_object(other, P0, Zone::Battlefield, None, None);
    s.get_object_mut(appeared).unwrap().name = "Forest".into();
    flags_transition(&prev, None, &s, &reg, "appeared mid-game (CR 108.3)");

    // A token is allowed to appear, but not on an id the allocator already
    // handed out.
    let mut p = prev.clone();
    p.next_object_id += 5;
    let mut s = next(&p);
    let recycled = mtg_engine::ids::ObjectId(p.next_object_id - 1);
    let mut ghost = s.get_object(bear).unwrap().clone();
    ghost.id = recycled;
    ghost.is_token = true;
    s.objects.insert(recycled, ghost);
    flags_transition(&p, None, &s, &reg, "reuses an id below the allocator's");
}

/// The records that only ever move one way: the allocators, the counts, the
/// mulligan and loss flags, the result, and the log.
#[test]
fn the_one_way_records_never_go_back() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    prev.log(mtg_engine::state::LogLevel::Info, "something happened".to_string());
    let cur = next(&prev);
    clean_transition(&prev, None, &cur, &reg);

    let mut s = cur.clone();
    s.next_object_id -= 1;
    flags_transition(&prev, None, &s, &reg, "next_object_id went back");

    let mut p = prev.clone();
    p.submit_seq = 5;
    let mut s = next(&p);
    s.submit_seq = 4;
    flags_transition(&p, None, &s, &reg, "submit_seq went back 5 -> 4");

    let mut p = prev.clone();
    p.get_object_mut(bear).unwrap().zone_change_count = 2;
    let mut s = next(&p);
    s.get_object_mut(bear).unwrap().zone_change_count = 1;
    flags_transition(&p, None, &s, &reg, "zone_change_count went back 2 -> 1");

    // CR 506.4: a creature that attacked this turn remembers it.
    let mut p = prev.clone();
    p.get_object_mut(bear).unwrap().attacked_on_turn = Some(3);
    let mut s = next(&p);
    s.get_object_mut(bear).unwrap().attacked_on_turn = None;
    flags_transition(&p, None, &s, &reg, "forgot attacking on turn 3");

    let mut p = prev.clone();
    p.get_player_mut(P0).mulligan_count = 1;
    p.get_player_mut(P0).mulligan_kept = true;
    let mut s = next(&p);
    s.get_player_mut(P0).mulligan_count = 0;
    flags_transition(&p, None, &s, &reg, "p0 mulligan count went back");
    let mut s = next(&p);
    s.get_player_mut(P0).mulligan_kept = false;
    flags_transition(&p, None, &s, &reg, "p0 un-kept their hand");

    // CR 104.3: leaving the game is permanent, and so is the reason.
    let mut p = prev.clone();
    p.get_player_mut(P1).lost = true;
    p.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::Conceded);
    p.result = Some(mtg_engine::state::GameResult::Winner(P0));
    let mut s = next(&p);
    s.get_player_mut(P1).lost = false;
    s.get_player_mut(P1).loss_reason = None;
    flags_transition(&p, None, &s, &reg, "p1 un-lost the game (CR 104.3)");
    let mut s = next(&p);
    s.get_player_mut(P1).loss_reason = Some(mtg_engine::events::LossReason::LifeReachedZero);
    flags_transition(&p, None, &s, &reg, "p1 un-lost the game (CR 104.3)");

    // CR 104.4: a game that has a result keeps it.
    let mut s = next(&p);
    s.result = Some(mtg_engine::state::GameResult::Draw);
    flags_transition(&p, None, &s, &reg, "the result changed");

    // The log is append-only.
    let mut s = cur.clone();
    s.game_log.pop();
    flags_transition(&prev, None, &s, &reg, "the game log shrank");
    let mut s = cur.clone();
    let last = s.game_log.len() - 1;
    s.game_log[last].message = "a different line".into();
    flags_transition(&prev, None, &s, &reg, "the game log was rewritten");
}

/// CR 305.2/602.5/morbid: the per-turn bookkeeping moves the way the turn
/// allows, and exactly as the events say.
#[test]
fn the_per_turn_bookkeeping_matches_the_events_that_moved_it() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let cur = next(&prev);
    clean_transition(&prev, None, &cur, &reg);

    // A land drop is spent by a LandPlayed and by nothing else.
    let mut s = next(&prev);
    s.get_player_mut(P0).land_plays_remaining = 0;
    flags_transition(&prev, None, &s, &reg, "land drops 1 -> 0 with 0 LandPlayed (CR 305.2)");

    // The spells-cast count moves with SpellCast events, and only with them.
    let mut s = next(&prev);
    s.num_spells_cast_this_turn.insert(P0, 1);
    flags_transition(&prev, None, &s, &reg, "spells cast this turn 0 -> 1 with 0 SpellCast");
    let mut p = prev.clone();
    p.num_spells_cast_this_turn.insert(P0, 2);
    let mut s = next(&p);
    s.num_spells_cast_this_turn.insert(P0, 1);
    flags_transition(&p, None, &s, &reg, "spells-cast-this-turn count went back");

    // Morbid is set by a death and survives the turn.
    let mut s = next(&prev);
    s.creature_died_this_turn = true;
    flags_transition(&prev, None, &s, &reg, "the morbid flag was set with no creature dying");
    let mut p = prev.clone();
    p.creature_died_this_turn = true;
    let mut s = next(&p);
    s.creature_died_this_turn = false;
    flags_transition(&p, None, &s, &reg, "the morbid flag was reset mid-turn");

    // CR 602.5: an activation this turn is remembered for the rest of it,
    // and forgotten by the next one.
    let mut p = prev.clone();
    p.get_object_mut(bear).unwrap().abilities_activated_this_turn.insert(0);
    let mut s = next(&p);
    s.get_object_mut(bear).unwrap().abilities_activated_this_turn.clear();
    flags_transition(&p, None, &s, &reg, "forgot an activation this turn");

    let mut s = next(&prev);
    s.turn_number = 4;
    s.active_player = P1;
    s.step = Step::Untap;
    s.get_object_mut(bear).unwrap().abilities_activated_this_turn.insert(0);
    flags_transition(&prev, None, &s, &reg, "remembers activations from a previous turn");
}

/// CR 500.1/103.7a: steps advance in order, turns alternate, and every step
/// change was announced by a `StepStarted` naming where it went.
#[test]
fn the_step_walk_is_announced_step_by_step() {
    let (mut prev, reg) = base();
    named_permanent(&mut prev, &reg, "Grizzly Bears", P0);

    // A step change with nothing announcing it.
    let mut s = next(&prev);
    s.step = Step::BeginCombat;
    flags_transition(&prev, None, &s, &reg, "with no StepStarted");

    // An announcement that does not lead where the state ended up.
    let mut s = next(&prev);
    s.step = Step::BeginCombat;
    s.events = vec![GameEvent::StepStarted { step: Step::DeclareAttackers }];
    flags_transition(&prev, None, &s, &reg, "the last StepStarted names");

    // CR 500.1: the steps come in order.
    let mut s = next(&prev);
    s.step = Step::EndStep;
    s.events = vec![GameEvent::StepStarted { step: Step::EndStep }];
    flags_transition(&prev, None, &s, &reg, "(CR 500.1)");

    // A turn counter that moved without a TurnStarted to move it.
    let mut s = next(&prev);
    s.turn_number = 4;
    s.active_player = P1;
    flags_transition(&prev, None, &s, &reg, "TurnStarted event(s) for a turn counter that moved by 1");

    // CR 103.7a: the turn passes to the other player, and only at cleanup.
    let mut s = next(&prev);
    s.turn_number = 4;
    s.active_player = P1;
    s.step = Step::Untap;
    s.events = vec![GameEvent::TurnStarted { player: P1, turn: 4 },
                    GameEvent::StepStarted { step: Step::Untap }];
    flags_transition(&prev, None, &s, &reg, "after turn 3 (PrecombatMain, p0 active)");

    // And the active player alternates with the turn count.
    let mut s = next(&prev);
    s.active_player = P1;
    flags_transition(&prev, None, &s, &reg, "active player p0 -> p1 over 0 turn(s)");
}

/// CR 400.7/121.3/701.20a: every zone change is announced from the zone the
/// object was in, and the verbs pair with the moves they name.
#[test]
fn the_zone_ledger_pairs_every_verb_with_its_move() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let card = spell_in_hand(&mut prev, &reg, "Moment of Heroism", P0);
    let library = stock_library(&mut prev, &reg, P0, 3);
    for id in &library {
        prev.get_object_mut(*id).unwrap().name = "Forest".into();
    }

    // A move announced from the wrong zone.
    let mut s = next(&prev);
    s.get_object_mut(bear).unwrap().zone = Zone::Graveyard;
    s.get_object_mut(bear).unwrap().zone_change_count += 1;
    s.events = vec![GameEvent::ObjectMoved { object: bear, from: Zone::Hand, to: Zone::Graveyard }];
    flags_transition(&prev, None, &s, &reg, "announced a move from Hand while in Battlefield");

    // A move announced to somewhere the object did not end up.
    let mut s = next(&prev);
    s.get_object_mut(bear).unwrap().zone = Zone::Graveyard;
    s.get_object_mut(bear).unwrap().zone_change_count += 1;
    s.events = vec![GameEvent::ObjectMoved { object: bear, from: Zone::Battlefield, to: Zone::Exile }];
    flags_transition(&prev, None, &s, &reg, "last announced moving to Exile but is in Graveyard");

    // The verbs: each names a move the ledger has to contain.
    let mut s = next(&prev);
    s.events = vec![GameEvent::CardDrawn { player: P0, object: library[0] }];
    flags_transition(&prev, None, &s, &reg, "CardDrawn #");
    flags_transition(&prev, None, &s, &reg, "without the matching zone change");
    let mut s = next(&prev);
    s.events = vec![GameEvent::Discarded { player: P0, object: card }];
    flags_transition(&prev, None, &s, &reg, "Discarded #");
    let mut s = next(&prev);
    s.events = vec![GameEvent::CreatureCardMilled { object: library[0], milled_player: P0 }];
    flags_transition(&prev, None, &s, &reg, "CreatureCardMilled #");
    let mut s = next(&prev);
    s.events = vec![GameEvent::SpellCast { player: P0, object: card }];
    flags_transition(&prev, None, &s, &reg, "SpellCast #");
    let mut s = next(&prev);
    s.events = vec![GameEvent::LandPlayed { player: P0, object: card }];
    flags_transition(&prev, None, &s, &reg, "LandPlayed #");
    let mut s = next(&prev);
    s.events = vec![GameEvent::EnteredBattlefield { object: card, controller: P0 }];
    flags_transition(&prev, None, &s, &reg, "EnteredBattlefield #");

    // CR 121.3: a draw takes the top card.
    let mut s = next(&prev);
    s.get_player_mut(P0).library_order.retain(|id| *id != library[2]);
    s.move_object(library[2], Zone::Hand, &reg);
    s.events.push(GameEvent::CardDrawn { player: P0, object: library[2] });
    flags_transition(&prev, None, &s, &reg, "from below the top 1 of a library that starts");

    // CR 701.20a: without a shuffle, the order that stays is the order it was.
    let mut s = next(&prev);
    s.get_player_mut(P0).library_order.swap(0, 2);
    flags_transition(&prev, None, &s, &reg, "was reordered without a shuffle (CR 701.20a)");
    s.events.push(GameEvent::LibraryShuffled { player: P0 });
    no_transition_flag(&prev, None, &s, &reg, "(CR 701.20a)");

    // CR 121.1: a drawn card came out of that player's library.
    let mut s = next(&prev);
    s.move_object(card, Zone::Hand, &reg);
    s.events.push(GameEvent::CardDrawn { player: P0, object: card });
    flags_transition(&prev, None, &s, &reg, "which was not in p0's library (CR 121.1)");
}

/// CR 120.3/302.6/508.1/701.15a: the per-object status ledgers each need a
/// witness in the event buffer.
#[test]
fn the_status_ledgers_each_need_their_witness() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);

    // Tapping and untapping are edges, each with its own event.
    let mut s = next(&prev);
    s.get_object_mut(bear).unwrap().tapped = true;
    flags_transition(&prev, None, &s, &reg, "became tapped with no Tapped event");
    let mut p = prev.clone();
    p.get_object_mut(bear).unwrap().tapped = true;
    let mut s = next(&p);
    s.get_object_mut(bear).unwrap().tapped = false;
    flags_transition(&p, None, &s, &reg, "became untapped with no Untapped event");

    // CR 120.3: marked damage shrinks only through regeneration or cleanup.
    let mut p = prev.clone();
    p.get_object_mut(bear).unwrap().damage_marked = 1;
    let mut s = next(&p);
    s.get_object_mut(bear).unwrap().damage_marked = 0;
    flags_transition(&p, None, &s, &reg, "lost marked damage (1 + 0 -> 0) with no regeneration or cleanup");

    // Deathtouch is a property of damage that was actually dealt.
    let mut s = next(&prev);
    s.get_object_mut(bear).unwrap().dealt_deathtouch_damage = true;
    flags_transition(&prev, None, &s, &reg, "marked with deathtouch damage that was never dealt");

    // CR 701.15a: regenerating taps and removes from combat.
    let mut p = prev.clone();
    p.get_object_mut(bear).unwrap().regeneration_shields = 1;
    let mut s = next(&p);
    s.get_object_mut(bear).unwrap().regeneration_shields = 0;
    flags_transition(&p, None, &s, &reg, "regenerated without tapping (CR 701.15a)");

    let mut p = prev.clone();
    p.get_object_mut(bear).unwrap().regeneration_shields = 1;
    p.get_object_mut(bear).unwrap().tapped = true;
    let mut s = next(&p);
    s.get_object_mut(bear).unwrap().regeneration_shields = 0;
    let mut c = mtg_engine::state::CombatState::new();
    c.any_attackers_declared = true;
    c.attackers.insert(bear, P1);
    s.combat = Some(c);
    flags_transition(&p, None, &s, &reg, "regenerated but is still in combat (CR 701.15a)");

    // The last controller is written when the object leaves, not before.
    let mut s = next(&prev);
    s.get_object_mut(bear).unwrap().last_controller = Some(P1);
    flags_transition(&prev, None, &s, &reg, "rewrote its last controller without leaving");

    // CR 508.1: an attack stamp comes from a declaration.
    let mut s = next(&prev);
    s.get_object_mut(bear).unwrap().attacked_on_turn = Some(3);
    flags_transition(&prev, None, &s, &reg, "was stamped as attacking without a declaration (CR 508.1)");
}

/// CR 119/704.5a/704.5b/121.4: life moves through its events, and a loss
/// says why in a way the state bears out.
#[test]
fn every_loss_says_why_in_a_way_the_state_bears_out() {
    let (mut prev, reg) = base();
    named_permanent(&mut prev, &reg, "Grizzly Bears", P0);

    // The chain has to start where the player was and end where they are.
    let mut s = next(&prev);
    s.get_player_mut(P1).life = 18;
    s.events = vec![GameEvent::LifeChanged { player: P1, old: 19, new_life: 18 }];
    flags_transition(&prev, None, &s, &reg, "life chain starts at 19 but they had 20");
    let mut s = next(&prev);
    s.events = vec![GameEvent::LifeChanged { player: P1, old: 20, new_life: 18 }];
    flags_transition(&prev, None, &s, &reg, "life chain ends at 18 but they have 20");

    let lost = |life: i32, reason: mtg_engine::events::LossReason, prev: &GameState| {
        let mut s = next(prev);
        s.get_player_mut(P1).life = life;
        s.get_player_mut(P1).lost = true;
        s.get_player_mut(P1).loss_reason = Some(reason);
        s.result = Some(mtg_engine::state::GameResult::Winner(P0));
        s.events = vec![GameEvent::PlayerLost { player: P1, reason },
                        GameEvent::GameEnded { result: mtg_engine::state::GameResult::Winner(P0) }];
        s
    };

    // CR 704.5a: losing to 0 life means the life went to 0.
    let s = lost(20, mtg_engine::events::LossReason::LifeReachedZero, &prev);
    flags_transition(&prev, None, &s, &reg, "without their life reaching 0 (CR 704.5a)");

    // CR 704.5b: losing to an empty draw is recorded as one.
    let s = lost(20, mtg_engine::events::LossReason::DrewFromEmptyLibrary, &prev);
    flags_transition(&prev, None, &s, &reg, "that is not recorded (CR 704.5b)");

    // Conceding is an action the conceding player took, holding priority.
    let mut p = prev.clone();
    p.priority_player = Some(P0);
    let s = lost(20, mtg_engine::events::LossReason::Conceded, &p);
    flags_transition(&p, None, &s, &reg, "conceded without holding priority on a Concede action");

    // "The opponent won" is a claim about the result.
    let mut s = lost(20, mtg_engine::events::LossReason::OpponentWon, &prev);
    s.result = Some(mtg_engine::state::GameResult::Winner(P1));
    flags_transition(&prev, None, &s, &reg, "lost because the opponent won, but the result is");

    // CR 121.4: a failed draw is from a library that is actually empty.
    let mut p = prev.clone();
    stock_library(&mut p, &reg, P1, 2);
    let mut s = next(&p);
    s.get_player_mut(P1).has_drawn_from_empty = true;
    flags_transition(&p, None, &s, &reg, "empty library that holds 2 cards (CR 121.4)");
}

/// CR 106.4/500.4: mana appears only through `ManaAdded` and leaves only by
/// payment, an emptying, or the end of a step.
#[test]
fn mana_appears_and_leaves_only_the_ways_the_rules_allow() {
    let (mut prev, reg) = base();
    named_permanent(&mut prev, &reg, "Forest", P0);
    add_mana(&mut prev, P0, &[(ManaType::Green, 2)]);

    // Mana out of nowhere.
    let mut s = next(&prev);
    s.get_player_mut(P0).mana_pool.mana.insert(ManaType::Green, 3);
    flags_transition(&prev, None, &s, &reg, "3 Green mana after 2 + 0 added (CR 106.4)");
    s.events = vec![GameEvent::ManaAdded { player: P0, mana_type: ManaType::Green, amount: 1 }];
    no_transition_flag(&prev, None, &s, &reg, "(CR 106.4)");

    // Mana that vanished with nothing paid.
    let mut s = next(&prev);
    s.get_player_mut(P0).mana_pool.mana.insert(ManaType::Green, 1);
    flags_transition(&prev, None, &s, &reg, "with nothing paid and no step ending (CR 500.4)");
    s.events = vec![GameEvent::ManaPoolEmptied { player: P0 }];
    no_transition_flag(&prev, None, &s, &reg, "(CR 500.4)");
}

/// CR 405.2: only the action that put something on the stack put something
/// on the stack, and it went on top of what was there.
#[test]
fn an_action_grows_the_stack_only_from_the_top() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let land = spell_in_hand(&mut prev, &reg, "Forest", P0);
    let pump = castable_spell(&mut prev, &reg, "Moment of Heroism", P0);
    prev.priority_player = Some(P0);

    let play = Action::PlayLand { object_id: land };
    let cur = mtg_engine::engine::submit_action(&prev, &play, &reg);
    clean_transition(&prev, Some(&play), &cur, &reg);

    // A land drop that also put a spell on the stack.
    let mut s = cur.clone();
    s.get_object_mut(pump).unwrap().zone = Zone::Stack;
    s.stack.push(StackEntry::Spell(pump));
    flags_transition(&prev, Some(&play), &s, &reg, "non-trigger entries on the stack");

    // A cast that also removed what was under it.
    let mut p = prev.clone();
    let under = castable_spell(&mut p, &reg, "Moment of Heroism", P0);
    p.get_object_mut(under).unwrap().zone = Zone::Stack;
    p.stack.push(StackEntry::Spell(under));
    let mut s = next(&p);
    s.stack.clear();
    let cast = cast_action(pump, vec![Target::Object(bear)]);
    flags_transition(&p, Some(&cast), &s, &reg, "disturbed the stack below the top");
}

/// CR 601.2h/602.2f: casting and activating pay their costs out of the
/// pool, and the ledger sees the whole cost, not only its coloured pips.
#[test]
fn a_cast_and_an_activation_each_spend_their_whole_cost() {
    let (mut prev, reg) = base();
    let bear = named_permanent(&mut prev, &reg, "Grizzly Bears", P0);
    let pump = castable_spell(&mut prev, &reg, "Moment of Heroism", P0);
    add_mana(&mut prev, P0, &[(ManaType::White, 2)]);
    prev.priority_player = Some(P0);
    let cast = cast_action(pump, vec![Target::Object(bear)]);
    let cur = mtg_engine::engine::submit_action(&prev, &cast, &reg);
    clean_transition(&prev, Some(&cast), &cur, &reg);

    // CR 601.2a: the card moved.
    let mut s = cur.clone();
    s.get_object_mut(pump).unwrap().zone_change_count = prev.get_object(pump).unwrap().zone_change_count;
    flags_transition(&prev, Some(&cast), &s, &reg, "announced but the card never moved (CR 601.2a)");

    // The whole cost, generic included: the pool may not come out ahead of
    // what it started with less the cost.
    let mut s = cur.clone();
    let started = prev.get_player(P0).mana_pool.total();
    s.get_player_mut(P0).mana_pool.mana.insert(ManaType::White, started);
    flags_transition(&prev, Some(&cast), &s, &reg, "for a total cost of");

    // CR 602.2f: an activation cost is paid the same way.
    let (mut prev, reg) = base();
    let priest = named_permanent(&mut prev, &reg, "Avacynian Priest", P0);
    prev.get_object_mut(priest).unwrap().summoning_sick = false;
    named_permanent(&mut prev, &reg, "Grizzly Bears", P1);
    add_mana(&mut prev, P0, &[(ManaType::White, 1)]);
    prev.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&prev, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == priest))
        .expect("the ability is offered").clone();
    let cur = mtg_engine::engine::submit_action(&prev, &activate, &reg);
    clean_transition(&prev, Some(&activate), &cur, &reg);
    let mut s = cur.clone();
    let started = prev.get_player(P0).mana_pool.total();
    s.get_player_mut(P0).mana_pool.mana.insert(ManaType::White, started);
    flags_transition(&prev, Some(&activate), &s, &reg, "(CR 602.2f)");
}

/// CR 605.3/606.5: a mana ability changes nothing but the pool, and a
/// loyalty ability goes on the stack.
#[test]
fn a_mana_ability_changes_only_the_pool_and_a_loyalty_ability_uses_the_stack() {
    let (mut prev, reg) = base();
    let forest = named_permanent(&mut prev, &reg, "Forest", P0);
    prev.priority_player = Some(P0);
    let tap = Action::ActivateManaAbility { object_id: forest, ability_index: 0 };
    let cur = mtg_engine::engine::submit_action(&prev, &tap, &reg);
    clean_transition(&prev, Some(&tap), &cur, &reg);

    let mut s = cur.clone();
    s.step = Step::BeginCombat;
    flags_transition(&prev, Some(&tap), &s, &reg, "changed the step, priority, or the stack (CR 605.3)");

    // Tapping something that was already tapped.
    let mut p = prev.clone();
    p.get_object_mut(forest).unwrap().tapped = true;
    let mut s = next(&p);
    s.events = vec![GameEvent::Tapped { object: forest }];
    flags_transition(&p, Some(&tap), &s, &reg, "which was already tapped");

    // CR 606.5: a loyalty ability uses the stack.
    let (mut prev, reg) = base();
    let lili = named_permanent(&mut prev, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut prev, lili, 3);
    prev.priority_player = Some(P0);
    let plus = Action::ActivateLoyaltyAbility { object_id: lili, ability_index: 0, targets: vec![] };
    let cur = mtg_engine::engine::submit_action(&prev, &plus, &reg);
    clean_transition(&prev, Some(&plus), &cur, &reg);
    let mut s = cur.clone();
    s.stack.clear();
    flags_transition(&prev, Some(&plus), &s, &reg, "did not go on the stack (CR 606.5)");
}

/// CR 103.5/514.1: the mulligan and cleanup actions do what they say.
#[test]
fn the_mulligan_and_discard_actions_each_do_what_they_say() {
    let (mut prev, reg) = base();
    stock_library(&mut prev, &reg, P0, 20);
    for _ in 0..7 {
        spell_in_hand(&mut prev, &reg, "Moment of Heroism", P0);
    }
    prev.priority_player = None;
    prev.awaiting_action = Some(AwaitingAction::MulliganDecision { player: P0 });

    // Keeping draws nothing and is recorded.
    let keep = Action::MulliganKeep;
    let cur = mtg_engine::engine::submit_action(&prev, &keep, &reg);
    let mut s = cur.clone();
    s.get_player_mut(P0).mulligan_kept = false;
    flags_transition(&prev, Some(&keep), &s, &reg, "kept but is not recorded as having kept");
    let mut s = cur.clone();
    s.events.push(GameEvent::CardDrawn { player: P0, object: prev.get_player(P0).library_order[0] });
    flags_transition(&prev, Some(&keep), &s, &reg, "keeping a hand drew cards");

    // CR 103.5: a mulligan shuffles, then draws, and the count moves.
    let mull = Action::MulliganMull;
    let cur = mtg_engine::engine::submit_action(&prev, &mull, &reg);
    let mut s = cur.clone();
    s.events.retain(|e| !matches!(e, GameEvent::LibraryShuffled { .. }));
    flags_transition(&prev, Some(&mull), &s, &reg, "mulliganed without shuffling (CR 103.5)");
    let mut s = cur.clone();
    s.get_player_mut(P0).mulligan_count = prev.get_player(P0).mulligan_count;
    flags_transition(&prev, Some(&mull), &s, &reg, "mulliganed without the count moving (CR 103.5)");

    // CR 514.1: a cleanup discard moves exactly the cards it names.
    let mut p = base().0;
    let hand: Vec<ObjectId> = (0..3)
        .map(|_| spell_in_hand(&mut p, &reg, "Moment of Heroism", P0))
        .collect();
    p.priority_player = None;
    p.step = Step::Cleanup;
    p.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P0, discard_count: 1 });
    let discard = Action::DiscardCards { cards: vec![hand[0]] };
    let cur = mtg_engine::engine::submit_action(&p, &discard, &reg);
    clean_transition(&p, Some(&discard), &cur, &reg);

    let mut s = cur.clone();
    s.events.retain(|e| !matches!(e, GameEvent::Discarded { .. }));
    flags_transition(&p, Some(&discard), &s, &reg, "(CR 514.1)");
    let mut s = cur.clone();
    s.move_object(hand[0], Zone::Hand, &reg);
    flags_transition(&p, Some(&discard), &s, &reg, "hand went");
}

/// CR 601.2c/602.2b: the number of targets a stack entry stores is a
/// number its requirement allows — for every shape of requirement, not
/// just the one-target one.
#[test]
fn the_target_arity_of_every_requirement_shape_is_checked() {
    use mtg_engine::cards::TargetRequirement as R;

    let (mut state, reg) = base();
    let source = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let card_id = state.get_object(source).unwrap().card_id;
    let creatures: Vec<ObjectId> = (0..3)
        .map(|_| named_permanent(&mut state, &reg, "Grizzly Bears", P1))
        .collect();

    let with = |req: R, n: usize| {
        let mut s = state.clone();
        s.stack.push(StackEntry::Ability {
            source_id: source, ability_index: 0, behavior_card_id: card_id,
            targets: creatures[..n].iter().map(|id| Target::Object(*id)).collect(),
            activator: P0, x_value: None, target_requirement: Some(req),
            sacrificed: None, sacrificed_toughness: None, loyalty: false,
        });
        s
    };
    #[track_caller]
    fn allows(s: &GameState, reg: &CardRegistry, yes: bool, what: &str) {
        let flagged = check_core(&as_collected(s), reg).iter()
            .any(|m| m.contains("targets for requirement"));
        assert_eq!(flagged, !yes, "{what}");
    }

    // "None" takes none; a plain kind takes exactly one.
    allows(&with(R::None, 0), &reg, true, "no requirement, no targets");
    allows(&with(R::None, 1), &reg, false, "no requirement, one target");
    allows(&with(R::Creature, 1), &reg, true, "one creature, one target");
    allows(&with(R::Creature, 0), &reg, false, "one creature, no target");
    allows(&with(R::Creature, 2), &reg, false, "one creature, two targets");

    // "Up to k" takes anything through k, and k itself.
    let up_to_2 = || R::UpToTargets(2, Box::new(R::Creature));
    for n in 0..=2 {
        allows(&with(up_to_2(), n), &reg, true, "up to two");
    }
    allows(&with(up_to_2(), 3), &reg, false, "up to two stops at two");

    // Two requirements together take the sum of what each takes: a
    // mandatory slot plus an "up to one" is one or two, never none.
    let two = || R::TwoTargets(Box::new(R::Creature), Box::new(R::UpToTargets(1, Box::new(R::Creature))));
    allows(&with(two(), 0), &reg, false, "a mandatory slot needs its target");
    allows(&with(two(), 1), &reg, true, "the optional slot may be empty");
    allows(&with(two(), 2), &reg, true, "or filled");
    allows(&with(two(), 3), &reg, false, "but not twice over");

    // A modal requirement takes whatever any one of its modes takes.
    let modal = || R::ModalChoice(vec![R::None, R::Creature]);
    allows(&with(modal(), 0), &reg, true, "the untargeted mode");
    allows(&with(modal(), 1), &reg, true, "the targeted mode");
    allows(&with(modal(), 2), &reg, false, "neither mode takes two");
}

/// CR 603.3d/603.8: a trigger on a queue or the stack names a real
/// controller, carries a target only if its ability targets, and a
/// state-triggered ability is in flight exactly once.
#[test]
fn a_queued_triggers_shape_is_checked() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let ghoul = named_permanent(&mut state, &reg, "Abattoir Ghoul", P0);
    let card_id = state.get_object(ghoul).unwrap().card_id;
    let ghost = PlayerId(u8::try_from(state.players.len()).unwrap());
    let trigger = |controller: PlayerId, targets: Vec<Target>| {
        let mut src = mtg_engine::triggers::TriggerSource::new(ghoul, card_id, controller, "t");
        src.chosen_targets = targets;
        mtg_engine::triggers::PendingTrigger::new(src, mtg_engine::triggers::TriggerEvent::StateTriggered)
    };

    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(trigger(ghost, vec![]));
    flags_core(&s, &reg, "is controlled by p2 who is not a player");

    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(trigger(P0, vec![Target::Object(bear), Target::Object(ghoul)]));
    flags_core(&s, &reg, "has 2 targets");

    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(trigger(P0, vec![Target::Illegal]));
    flags_core(&s, &reg, "stores an Illegal target");

    let mut s = state.clone();
    let src = mtg_engine::triggers::TriggerSource::new(ghoul, mtg_engine::ids::CardId(424_242), P0, "t");
    s.pending_trigger_pushes_ap.push(mtg_engine::triggers::PendingTrigger::new(
        src, mtg_engine::triggers::TriggerEvent::StateTriggered));
    flags_core(&s, &reg, "has no behavior in the registry (card 424242)");

    // CR 603.8: one state-triggered ability in flight per source, and the
    // source's own flag agrees.
    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(trigger(P0, vec![]));
    s.pending_trigger_pushes_ap.push(trigger(P0, vec![]));
    s.get_object_mut(ghoul).unwrap().state_trigger_on_stack = true;
    flags_core(&s, &reg, "has 2 state-triggered abilities in flight (CR 603.8)");

    // A pending enters-trigger whose source is not on the battlefield.
    let mut s = state.clone();
    s.get_object_mut(ghoul).unwrap().zone = Zone::Graveyard;
    s.pending_triggers.push(mtg_engine::triggers::PendingTrigger::new(
        mtg_engine::triggers::TriggerSource::new(ghoul, card_id, P0, "t"),
        mtg_engine::triggers::TriggerEvent::SelfEntered));
    flags_core(&s, &reg, "whose source is not on the battlefield");
}

/// CR 303.4a: an Aura spell has exactly one target, of the kind its enchant
/// ability names.
#[test]
fn an_aura_spell_targets_exactly_what_it_enchants() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let aura = castable_spell(&mut state, &reg, "Pacifism", P0);
    let state = cast_onto_stack(&state, &reg, aura, vec![Target::Object(bear)]);
    clean_core(&state, &reg);

    let mut s = state.clone();
    s.get_object_mut(aura).unwrap().targets.clear();
    flags_core(&s, &reg, "is an Aura spell with no target (CR 303.4a)");

    let mut s = state.clone();
    s.get_object_mut(aura).unwrap().targets = vec![Target::Player(P1)];
    flags_core(&s, &reg, "enchants permanents but targets a player");

    let mut s = state.clone();
    s.get_object_mut(aura).unwrap().targets = vec![Target::Object(bear), Target::Player(P1)];
    flags_core(&s, &reg, "is an Aura spell with 2 targets (CR 303.4a)");
}

/// CR 601.2b: a spell with X in its cost announced an X.
#[test]
fn an_x_spell_on_the_stack_announced_its_x() {
    let (mut state, reg) = base();
    let play = castable_spell(&mut state, &reg, "Devil's Play", P0);
    add_mana(&mut state, P0, &[(ManaType::Red, 2)]);
    let state = resolve_funding_max(&cast_onto_stack(&state, &reg, play, vec![Target::Player(P1)]), &reg);
    assert!(state.stack.iter().any(|e| e.as_spell() == Some(play)), "precondition: on the stack");
    clean_core(&state, &reg);

    let mut s = state.clone();
    s.get_object_mut(play).unwrap().x_value = None;
    flags_core(&s, &reg, "has an X cost but no X announced (CR 601.2b)");
}

/// CR 702.34a/601.2b: flashback that a spell was GRANTED costs what the
/// grant naming that spell says, and X is read off that cost. Past in Flames
/// grants flashback to every instant and sorcery in a graveyard at once, so
/// the grants sit side by side and the wrong one carries the wrong cost —
/// Devil's Play is the one card in the pool whose mana cost has an X in it,
/// and reading its grant for someone else's spell invents an unannounced X.
#[test]
fn a_granted_flashback_cost_is_read_off_the_grant_that_names_the_spell() {
    let (mut state, reg) = base();
    let play = named_card_in_graveyard(&mut state, &reg, "Devil's Play", P0);
    let volley = castable_spell(&mut state, &reg, "Brimstone Volley", P0);
    let mut state = cast_onto_stack(&state, &reg, volley, vec![Target::Player(P1)]);

    // Both cards sat in the graveyard when Past in Flames resolved; the X
    // one is listed first, so a lookup that ignores the target finds it.
    let x_cost = reg.card_data(state.get_object(play).unwrap().card_id).unwrap().cost.unwrap();
    assert!(x_cost.has_x(), "precondition: Devil's Play costs an X");
    let volley_cost = reg.card_data(state.get_object(volley).unwrap().card_id).unwrap().cost.unwrap();
    assert!(!volley_cost.has_x(), "precondition: Brimstone Volley does not");
    state.until_end_of_turn.push(TemporaryEffect::GrantFlashback { target: play, cost: x_cost });
    state.until_end_of_turn.push(TemporaryEffect::GrantFlashback { target: volley, cost: volley_cost });
    {
        // CR 702.34a: flashback casts the card from the graveyard.
        let o = state.get_object_mut(volley).unwrap();
        o.cast_with_flashback = true;
        o.cast_from_zone = Some(Zone::Graveyard);
    }
    assert!(reg.card_data(state.get_object(volley).unwrap().card_id).unwrap().flashback_cost.is_none(),
        "precondition: the grant is the only flashback cost this spell has");

    // The engine's own reachable board: Past in Flames grants flashback to
    // every instant and sorcery in the graveyard at once, so Devil's Play's
    // {X}{R} grant really does sit next to Brimstone Volley's {1}{R}{R} one.
    // Only the grant naming the spell being cast may be read.
    assert_eq!(check_core(&as_collected(&state), &reg), Vec::<String>::new());
}

/// CR 601.2/601.2h: a cast still being paid for is described consistently
/// by its stash — the caster, the card, the zone, the costs it plans to pay
/// with, and the prompt that is asking about it.
#[test]
fn a_cast_in_progress_is_described_consistently_by_its_stash() {
    let (mut state, reg) = base();
    let play = castable_spell(&mut state, &reg, "Devil's Play", P0);
    add_mana(&mut state, P0, &[(ManaType::Red, 2)]);
    let state = cast_onto_stack(&state, &reg, play, vec![Target::Player(P1)]);
    assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseXFunding { .. }, .. })),
        "precondition: a funding prompt is up");
    clean_core(&state, &reg);
    let ghost = PlayerId(u8::try_from(state.players.len()).unwrap());

    // CR 601.2: the caster holds priority throughout.
    let mut s = state.clone();
    s.priority_player = Some(P1);
    flags_core(&s, &reg, "but priority is Some(PlayerId(1)) (CR 601.2)");

    let mut s = state.clone();
    s.pending_spell_cast.as_mut().unwrap().player = ghost;
    flags_core(&s, &reg, "by p2 who is not a player");

    // The stash and the object agree about what card this is and whose.
    let mut s = state.clone();
    s.pending_spell_cast.as_mut().unwrap().card_id = mtg_engine::ids::CardId(424_242);
    flags_core(&s, &reg, "but the object is card");

    // CR 702.34a: a flashback cast comes from the graveyard.
    let mut s = state.clone();
    s.pending_spell_cast.as_mut().unwrap().is_flashback = true;
    flags_core(&s, &reg, "with flashback from Hand");

    // The cast-time marks are written when the cast completes, not before.
    let mut s = state.clone();
    s.get_object_mut(play).unwrap().x_value = Some(2);
    flags_core(&s, &reg, "already carries cast-time marks");

    // The prompt that is up is the prompt this cast raised.
    let mut s = state.clone();
    if let Some(AwaitingAction::ResolutionChoice { source, .. }) = &mut s.awaiting_action {
        *source = ObjectId(4242);
    }
    flags_core(&s, &reg, "but the pending prompt is for #4242");

    // The additional costs it plans to pay with are payable.
    let mut s = state.clone();
    s.pending_spell_cast.as_mut().unwrap().sacrifice = Some(play);
    flags_core(&s, &reg, "which is not a creature the caster controls (CR 701.21a)");

    let mut s = state.clone();
    s.pending_spell_cast.as_mut().unwrap().exile_ids = vec![play, play];
    flags_core(&s, &reg, "exiles #");
    flags_core(&s, &reg, "which is not in the caster's graveyard");
}

/// CR 602.2/602.2b: the same, for an activation whose X is still being
/// funded — and for the funding prompt's own arithmetic.
#[test]
fn an_activation_in_progress_is_described_consistently_by_its_stash() {
    let (mut state, reg) = base();
    let wolf_run = named_permanent(&mut state, &reg, "Kessig Wolf Run", P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    // {X}{R}{G}, {T}: with a floating {R}{G} plus two untapped Mountains
    // there is an X worth funding.
    named_permanent(&mut state, &reg, "Mountain", P0);
    named_permanent(&mut state, &reg, "Mountain", P0);
    add_mana(&mut state, P0, &[(ManaType::Red, 1), (ManaType::Green, 1)]);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let activate = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == wolf_run))
        .expect("Kessig Wolf Run's X ability is offered").clone();
    let state = mtg_engine::engine::submit_action(&state, &activate, &reg);
    assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseXFunding { is_ability: true, .. }, .. })),
        "precondition: an ability funding prompt is up, got {:?}", state.awaiting_action);
    assert!(state.pending_ability_effect.is_some(), "precondition: the activation is stashed");
    clean_core(&state, &reg);

    // CR 602.2: the activator holds priority throughout.
    let mut s = state.clone();
    s.priority_player = Some(P1);
    flags_core(&s, &reg, "but priority is Some(PlayerId(1)) (CR 602.2)");

    // The stash and the prompt name the same source.
    let mut s = state.clone();
    s.pending_ability_effect.as_mut().unwrap().source_id = bear;
    flags_core(&s, &reg, "but the stash is for #");

    // The behaviour the ability came from is a card the registry knows.
    let mut s = state.clone();
    s.pending_ability_effect.as_mut().unwrap().behavior_card_id = mtg_engine::ids::CardId(424_242);
    flags_core(&s, &reg, "has no behavior in the registry (card 424242)");

    // The funding prompt's ceiling is what it actually offers.
    let mut s = state.clone();
    if let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseXFunding { options, .. }, .. }) = &mut s.awaiting_action {
        options.max_x += 1;
    }
    flags_core(&s, &reg, "tappable but a ceiling of");

    // A prompt with nothing to fund is a prompt that should not exist.
    let mut s = state.clone();
    if let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseXFunding { options, .. }, .. }) = &mut s.awaiting_action {
        options.max_x = 0;
        options.groups.clear();
        options.pool.clear();
    }
    flags_core(&s, &reg, "with nothing to fund");
}

/// CR 608.2m/602.2a: the resolution bookkeeping names things that are there
/// and does not overlap with a cast.
#[test]
fn the_resolution_bookkeeping_names_what_is_actually_resolving() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let pump = castable_spell(&mut state, &reg, "Moment of Heroism", P0);
    let cast = cast_onto_stack(&state, &reg, pump, vec![Target::Object(bear)]);
    let ghost = PlayerId(u8::try_from(state.players.len()).unwrap());
    let prompt = AwaitingAction::ResolutionChoice {
        player: P0, source: bear,
        choice: ResolutionChoiceKind::YesNo { description: "?".into(), source_card: bear } };

    // A spell that is resolving but does not exist.
    let mut s = state.clone();
    s.resolving_spell = Some(ObjectId(4242));
    s.awaiting_action = Some(prompt.clone());
    flags_core(&s, &reg, "resolving_spell #4242 does not exist");

    // One that is resolving and still on the stack.
    let mut s = cast.clone();
    s.resolving_spell = Some(pump);
    s.awaiting_action = Some(prompt.clone());
    flags_core(&s, &reg, "is still on the stack");

    // A spell resolving while another is being cast.
    let mut s = cast.clone();
    s.resolving_spell = Some(pump);
    s.awaiting_action = Some(prompt);
    s.stack.clear();
    s.get_object_mut(pump).unwrap().zone = Zone::Stack;
    let dp = castable_spell(&mut s, &reg, "Devil's Play", P0);
    add_mana(&mut s, P0, &[(ManaType::Red, 2)]);
    let cast_dp = cast_onto_stack(&s, &reg, dp, vec![Target::Player(P1)]);
    let mut s = cast_dp;
    s.resolving_spell = Some(pump);
    flags_core(&s, &reg, "a spell is resolving while another is being cast");

    // CR 602.2a: the activator of a resolving ability is a player.
    let mut s = state.clone();
    s.resolving_ability_activator = Some(ghost);
    flags_core(&s, &reg, "resolving_ability_activator p2 is not a player");
}

/// CR 601.2h: an exile-from-graveyard cost prompt asks for a count it can
/// be given, out of the caster's own graveyard.
#[test]
fn an_exile_cost_prompt_offers_the_casters_own_graveyard() {
    let (mut state, reg) = base();
    let pyre = castable_spell(&mut state, &reg, "Harvest Pyre", P0);
    let mine = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P1);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let state = cast_onto_stack(&state, &reg, pyre, vec![Target::Object(bear)]);
    let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseExileFromGraveyard { .. }, .. }) =
        &state.awaiting_action else {
            panic!("precondition: an exile-cost prompt is up, got {:?}", state.awaiting_action)
        };
    clean_core(&state, &reg);

    // A count nobody can satisfy.
    let mut s = state.clone();
    if let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseExileFromGraveyard { min, max, .. }, .. }) =
        &mut s.awaiting_action {
        *min = *max + 1;
    }
    flags_core(&s, &reg, "exile prompt asks for");

    // The other player's graveyard, and the same card twice.
    let mut s = state.clone();
    if let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseExileFromGraveyard { options, .. }, .. }) =
        &mut s.awaiting_action {
        options.push(theirs);
    }
    flags_core(&s, &reg, "which is not in p0's graveyard");

    let mut s = state.clone();
    if let Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseExileFromGraveyard { options, .. }, .. }) =
        &mut s.awaiting_action {
        options.push(mine);
    }
    flags_core(&s, &reg, "exile prompt offers #");
}
