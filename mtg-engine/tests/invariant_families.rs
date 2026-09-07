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
        object: id, card_id: cid, controller: who, damaged_by: vec![],
        last_known_toughness: 2, is_token: false, subtypes: vec![] };
    let dead_creature = DeadCreature {
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
