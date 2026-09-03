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

    let mut s = state.clone();
    s.get_object_mut(wolf).unwrap().name = "Wolf".into();
    flags_core(&s, &reg, "does not end in \"Token\" (CR 111.4)");
    let mut s = state.clone();
    s.get_object_mut(wolf).unwrap().name = "Spirit Token".into();
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
        description: "d".into(), options: vec!["a".into(), "b".into()], ap_queue: true, indices: vec![0, 5] }));
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
    s.events = vec![GameEvent::CreatureDied { object: bear, card_id: s.get_object(bear).unwrap().card_id, controller: P0,
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
    flags_core(&s, &reg, "offers pool");
}

// ── the card-code contract ───────────────────────────────────────────────

#[test]
fn object_contract_violations_are_flagged() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let aura = named_permanent(&mut state, &reg, "Pacifism", P0);
    state.get_object_mut(aura).unwrap().attached_to = Some(bear);
    clean(&state, &reg);

    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().entering_copy_source = true;
    flags_core(&s, &reg, "no enters-as-copy choice in flight (CR 614.1d)");
    // With the copy prompt up the guard is legitimate.
    s.awaiting_action = Some(AwaitingAction::ResolutionChoice { player: P0, source: bear, choice: ResolutionChoiceKind::ChooseTarget {
        description: String::new(), options: vec![Target::Object(aura)], optional: true,
        effect: PendingEffect::CopyCreature { source_id: bear } } });
    assert!(!check_core(&s, &reg).iter().any(|m| m.contains("614.1d")), "{:?}", check_core(&s, &reg));

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
        choice: ResolutionChoiceKind::ChooseFromRevealed { description: String::new(), revealed: vec![card] } });
    flags_core(&s, &reg, "revealed prompt offers #");
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
    s.events = vec![GameEvent::CreatureDied { object: bear, card_id: s.get_object(bear).unwrap().card_id, controller: P0,
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
fn clean_transition(prev: &GameState, action: Option<&Action>, cur: &GameState, reg: &CardRegistry) {
    assert_eq!(check_transition(prev, action, cur, reg), Vec::<String>::new());
}

/// The next decision point, one action later, with nothing having happened.
fn next(prev: &GameState) -> GameState {
    let mut cur = prev.clone();
    cur.submit_seq = prev.submit_seq + 1;
    cur.events.clear();
    cur
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
