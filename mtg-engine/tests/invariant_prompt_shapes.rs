//! Self-tests for the prompt-coherence invariants
//! (`mtg_engine::invariants`'s `prompts` family): a prompt the game raises
//! is answerable, addressed to the right player, and asks about things that
//! are there.
//!
//! Same contract as the other invariant self-tests — the checker is the
//! fuzzer's only pair of eyes, so every clause needs a prompt that violates
//! it, and every conditional clause a neighbouring prompt that does not.

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::ObjectId;
use mtg_engine::invariants::check_core;
use mtg_engine::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind, StackEntry};
use mtg_engine::types::*;

fn base() -> (GameState, CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.turn_number = 3;
    (state, reg)
}

/// The opening-hand phase as the game really reaches it: turn one, untap
/// step, everything still in libraries and hands.
fn opening_hands(reg: &CardRegistry) -> GameState {
    let mut state = game_at_step(Step::Untap, P0);
    state.turn_number = 1;
    state.is_first_turn = true;
    state.priority_player = None;
    for p in [P0, P1] {
        for id in stock_library(&mut state, reg, p, 20) {
            state.get_object_mut(id).unwrap().name = "Forest".into();
        }
        for _ in 0..7 {
            spell_in_hand(&mut state, reg, "Moment of Heroism", p);
        }
    }
    state
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

/// CR 500.2/703.3: every turn-based-action prompt is raised on an empty
/// stack, with no triggers waiting, nothing half-resolved, and no mana
/// floating.
#[test]
fn a_turn_based_prompt_is_raised_on_a_quiet_game() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let card_id = state.get_object(bear).unwrap().card_id;
    state.step = Step::DeclareAttackers;
    state.awaiting_action = Some(AwaitingAction::DeclareAttackers);
    state.priority_player = Some(P0);
    assert_eq!(check_core(&state, &reg), Vec::<String>::new());

    let mut s = state.clone();
    s.stack.push(StackEntry::Spell(bear));
    flags(&s, &reg, "with 1 entries on the stack (CR 500.2)");

    let mut s = state.clone();
    s.pending_triggers.push(mtg_engine::triggers::PendingTrigger::new(
        mtg_engine::triggers::TriggerSource::new(bear, card_id, P0, "a triggered ability"),
        mtg_engine::triggers::TriggerEvent::Upkeep));
    flags(&s, &reg, "attackers prompt with triggers still queued");

    let mut s = state.clone();
    s.resolving_spell = Some(bear);
    flags(&s, &reg, "with a cast or resolution in progress");

    let mut s = state.clone();
    add_mana(&mut s, P0, &[(ManaType::Green, 1)]);
    flags(&s, &reg, "has mana floating (CR 500.5)");

    // CR 508.1: and the attackers prompt in particular finds no combat
    // state at all, and nothing left over from an earlier one.
    let mut s = state.clone();
    s.combat_damage_step_pending = true;
    flags(&s, &reg, "with leftovers from a previous combat");
    let mut s = state.clone();
    s.priority_player = Some(P1);
    flags(&s, &reg, "but priority is Some(PlayerId(1)), not the active player's");
}

/// CR 508.8/509.1: the blockers prompt is raised on a combat that has
/// attackers, in the blockers step, for the defending player.
#[test]
fn the_blockers_prompt_is_raised_on_a_declared_attack() {
    let (mut state, reg) = base();
    let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let blocker = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.step = Step::DeclareAttackers;
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    state.events.clear();
    state.trigger_event_index = 0;
    state.step = Step::DeclareBlockers;
    state.awaiting_action = Some(AwaitingAction::DeclareBlockers { defending_player: P1 });
    state.priority_player = Some(P1);
    assert_eq!(check_core(&state, &reg), Vec::<String>::new());

    let mut s = state.clone();
    s.step = Step::CombatDamage;
    flags(&s, &reg, "blockers prompt in CombatDamage (CR 509.1)");

    let mut s = state.clone();
    s.priority_player = Some(P0);
    flags(&s, &reg, "but priority is Some(PlayerId(0)), not the defender's");

    let mut s = state.clone();
    s.combat.as_mut().unwrap().any_attackers_declared = false;
    flags(&s, &reg, "with no attackers declared (CR 508.8)");

    let mut s = state.clone();
    s.combat.as_mut().unwrap().attackers.insert(attacker, P0);
    flags(&s, &reg, "but an attacker attacks someone other than p1");

    let mut s = state.clone();
    s.combat.as_mut().unwrap().blocked_attackers.insert(attacker);
    flags(&s, &reg, "but blocks or damage are already recorded");
    let mut s = state.clone();
    s.combat.as_mut().unwrap().blocker_assignments.insert(attacker, vec![blocker]);
    flags(&s, &reg, "but blocks or damage are already recorded");
    let mut s = state.clone();
    s.combat.as_mut().unwrap().dealt_first_strike.insert(attacker);
    flags(&s, &reg, "but blocks or damage are already recorded");

    let mut s = state.clone();
    s.combat_damage_step_pending = true;
    flags(&s, &reg, "with a second damage step pending");
}

/// CR 514.1: the cleanup discard is asked of the active player, in their
/// cleanup, for exactly the cards over seven.
#[test]
fn the_cleanup_discard_prompt_asks_for_the_cards_over_seven() {
    let (mut state, reg) = base();
    for _ in 0..9 {
        spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);
    }
    state.step = Step::Cleanup;
    state.priority_player = Some(P0);
    state.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P0, discard_count: 2 });
    assert_eq!(check_core(&state, &reg), Vec::<String>::new());

    let mut s = state.clone();
    s.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P0, discard_count: 1 });
    flags(&s, &reg, "asks for 1 discards from a hand of 9 (CR 514.1)");

    let mut s = state.clone();
    s.step = Step::EndStep;
    flags(&s, &reg, "discard-to-hand-size prompt in EndStep (CR 514.1)");

    let mut s = state.clone();
    s.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P1, discard_count: 2 });
    flags(&s, &reg, "it is p0's cleanup");

    let mut s = state.clone();
    s.combat = Some(mtg_engine::state::CombatState::new());
    flags(&s, &reg, "with combat state present");
}

/// CR 103: the opening-hand phase is turn one before anything has
/// happened, and the bottoming prompt asks for one card per mulligan.
#[test]
fn the_mulligan_phase_is_turn_one_before_anything_happened() {
    let reg = registry();
    let state = opening_hands(&reg);

    let mut s = state.clone();
    s.awaiting_action = Some(AwaitingAction::MulliganDecision { player: P0 });
    assert_eq!(check_core(&s, &reg), Vec::<String>::new());
    let keeping = s.clone();

    let mut s = keeping.clone();
    s.turn_number = 2;
    s.is_first_turn = false;
    flags(&s, &reg, "mulligan phase on turn 2");

    let mut s = keeping.clone();
    s.priority_player = Some(P0);
    flags(&s, &reg, "mulligan phase with a priority holder");

    let mut s = keeping.clone();
    s.combat = Some(mtg_engine::state::CombatState::new());
    flags(&s, &reg, "with a stack, combat, or a result");

    let mut s = keeping.clone();
    s.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantKeyword {
        target: ObjectId(1), keyword: Keyword::Flying });
    flags(&s, &reg, "mulligan phase with effects in force");

    // Nothing is on the battlefield yet, and nothing is a token.
    let mut s = keeping.clone();
    named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    flags(&s, &reg, "is a card in Battlefield");

    let mut s = keeping.clone();
    s.get_player_mut(P0).land_plays_remaining = 0;
    flags(&s, &reg, "already has turn state");

    let mut s = keeping.clone();
    s.pending_mulligan_bottoms.push((P0, 8));
    flags(&s, &reg, "queued bottoming of 8 for p0");

    // A hand that is not seven cards is not a hand a mulligan decision is
    // made on (CR 103.4 redraws to seven every time).
    let mut s = keeping.clone();
    spell_in_hand(&mut s, &reg, "Moment of Heroism", P0);
    flags(&s, &reg, "mulligan prompt for p0 holding 8 cards (CR 103.5)");

    // A player who kept is not asked again.
    let mut s = keeping.clone();
    s.get_player_mut(P0).mulligan_kept = true;
    flags(&s, &reg, "who already kept");

    // CR 103.4: bottoming is one card per mulligan, after everyone kept.
    let mut s = state.clone();
    s.get_player_mut(P0).mulligan_count = 2;
    s.get_player_mut(P0).mulligan_kept = true;
    s.get_player_mut(P1).mulligan_kept = true;
    s.awaiting_action = Some(AwaitingAction::BottomAfterMulligan { player: P0, count: 2 });
    assert_eq!(check_core(&s, &reg), Vec::<String>::new());

    let mut bad = s.clone();
    bad.awaiting_action = Some(AwaitingAction::BottomAfterMulligan { player: P0, count: 1 });
    flags(&bad, &reg, "bottom 1 after 2 mulligans");

    let mut bad = s.clone();
    bad.get_player_mut(P1).mulligan_kept = false;
    flags(&bad, &reg, "bottoming started before every player kept (CR 103.5)");
}

/// CR 608.2: a resolution prompt asks about things that are there, once
/// each, of the player who has to answer.
#[test]
fn a_resolution_prompt_offers_real_things_once_each() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let other = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let ghost = PlayerId(u8::try_from(state.players.len()).unwrap());
    let prompt = |choice: ResolutionChoiceKind| AwaitingAction::ResolutionChoice {
        player: P0, source: bear, choice };

    let target = |options: Vec<Target>| ResolutionChoiceKind::ChooseTarget {
        description: "d".into(), options, optional: false,
        effect: PendingEffect::DestroyCreature { source_name: "x".into() } };

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(target(vec![Target::Object(other)])));
    quiet_about(&s, &reg, "offers");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(target(vec![Target::Object(ObjectId(4242))])));
    flags(&s, &reg, "offers missing #4242");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(target(vec![Target::Player(ghost)])));
    flags(&s, &reg, "offers p2 who is not a player");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(target(vec![Target::Illegal])));
    flags(&s, &reg, "offers an Illegal target");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(target(vec![Target::Object(other), Target::Object(other)])));
    flags(&s, &reg, "twice");

    // CR 608.2: the prompt's source and the choice's source are the same
    // object.
    let mut s = state.clone();
    s.awaiting_action = Some(prompt(ResolutionChoiceKind::ChoosePile {
        description: "d".into(), pile_1: vec![bear], pile_2: vec![other], source_id: other }));
    flags(&s, &reg, "carries a choice for #");
}

/// CR 608.2d: a target prompt offers things the effect can act on — on the
/// battlefield, of the kind the effect names, and the chooser's own where
/// the effect says so.
#[test]
fn a_target_prompt_offers_what_its_effect_can_act_on() {
    let (mut state, reg) = base();
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let land = named_permanent(&mut state, &reg, "Forest", P1);
    let buried = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P1);
    let prompt = |effect: PendingEffect, options: Vec<Target>| {
        AwaitingAction::ResolutionChoice {
            player: P0, source: mine,
            choice: ResolutionChoiceKind::ChooseTarget {
                description: "d".into(), options, optional: false, effect } }
    };

    // Somewhere other than the battlefield.
    let mut s = state.clone();
    s.awaiting_action = Some(prompt(
        PendingEffect::DestroyCreature { source_name: "x".into() },
        vec![Target::Object(buried)]));
    flags(&s, &reg, "destroy-creature prompt offers");
    flags(&s, &reg, "(CR 608.2d)");

    // On the battlefield but not a creature.
    let mut s = state.clone();
    s.awaiting_action = Some(prompt(
        PendingEffect::DestroyCreature { source_name: "x".into() },
        vec![Target::Object(land)]));
    flags(&s, &reg, "which is no creature");

    // CR 701.17a: a sacrifice is of your own.
    let mut s = state.clone();
    s.awaiting_action = Some(prompt(
        PendingEffect::SacrificeCreature { source_name: "x".into() },
        vec![Target::Object(theirs)]));
    flags(&s, &reg, "which p0 does not control (CR 701.17a)");

    // A player where the effect acts on objects.
    let mut s = state.clone();
    s.awaiting_action = Some(prompt(
        PendingEffect::DestroyCreature { source_name: "x".into() },
        vec![Target::Player(P1)]));
    flags(&s, &reg, "destroy-creature prompt offers Player");

    // CR 120.1a: damage lands on creatures and planeswalkers.
    let mut s = state.clone();
    s.awaiting_action = Some(prompt(
        PendingEffect::DealDamage { amount: 2, source_id: mine },
        vec![Target::Object(land)]));
    flags(&s, &reg, "which is no battlefield creature or planeswalker");
}

/// CR 704.5j: the legend-rule prompt is exactly the duplicate group, is
/// mandatory, and is answered by the player who controls them.
#[test]
fn the_legend_rule_prompt_is_the_duplicate_group() {
    let (mut state, reg) = base();
    let a = named_permanent(&mut state, &reg, "Geist of Saint Traft", P0);
    let b = named_permanent(&mut state, &reg, "Geist of Saint Traft", P0);
    let legend = |options: Vec<Target>, optional: bool, who: PlayerId| {
        AwaitingAction::ResolutionChoice {
            player: P0, source: a,
            choice: ResolutionChoiceKind::ChooseTarget {
                description: "d".into(), options, optional,
                effect: PendingEffect::LegendRuleKeep {
                    player: who, legend_name: "Geist of Saint Traft".into() } } }
    };

    let mut s = state.clone();
    s.awaiting_action = Some(legend(vec![Target::Object(a), Target::Object(b)], false, P0));
    quiet_about(&s, &reg, "(CR 704.5j)");

    // Half the group.
    let mut s = state.clone();
    s.awaiting_action = Some(legend(vec![Target::Object(a)], false, P0));
    flags(&s, &reg, "(CR 704.5j)");

    // Optional, or answered by the wrong player.
    let mut s = state.clone();
    s.awaiting_action = Some(legend(vec![Target::Object(a), Target::Object(b)], true, P0));
    flags(&s, &reg, "legend-rule prompt for p0 answered by p0, optional=true");
    let mut s = state.clone();
    s.awaiting_action = Some(legend(vec![Target::Object(a), Target::Object(b)], false, P1));
    flags(&s, &reg, "legend-rule prompt for p1 answered by p0");
}

/// CR 603.3d/603.3b: a trigger-target prompt is for the trigger the answer
/// will pop, is mandatory, and waits for the active player's queue.
#[test]
fn a_trigger_target_prompt_is_for_the_front_of_the_queue() {
    let (mut state, reg) = base();
    let ghoul = named_permanent(&mut state, &reg, "Abattoir Ghoul", P0);
    let other = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let card_id = state.get_object(ghoul).unwrap().card_id;
    let queued = |src: ObjectId, controller: PlayerId, s: &GameState| {
        mtg_engine::triggers::PendingTrigger::new(
            mtg_engine::triggers::TriggerSource::new(src, s.get_object(src).unwrap().card_id,
                controller, "t"),
            mtg_engine::triggers::TriggerEvent::StateTriggered)
    };
    let prompt = |source: ObjectId, options: Vec<Target>, optional: bool, player: PlayerId| {
        AwaitingAction::ResolutionChoice {
            player, source,
            choice: ResolutionChoiceKind::ChooseTarget {
                description: "d".into(), options, optional,
                effect: PendingEffect::AttachTargetToPendingTrigger } }
    };

    // No queued trigger at all.
    let mut s = state.clone();
    s.awaiting_action = Some(prompt(ghoul,
        vec![Target::Object(ghoul), Target::Object(other)], false, P0));
    flags(&s, &reg, "trigger-target prompt with no queued trigger");

    // A queue whose front is a different trigger.
    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(queued(other, P0, &s));
    s.awaiting_action = Some(prompt(ghoul,
        vec![Target::Object(ghoul), Target::Object(other)], false, P0));
    flags(&s, &reg, "but the queue's front is #");

    // A prompt with nothing to choose between, or an optional one.
    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(queued(ghoul, P0, &s));
    s.awaiting_action = Some(prompt(ghoul, vec![Target::Object(other)], false, P0));
    flags(&s, &reg, "trigger-target prompt with 1 options, optional=false");

    // CR 603.3b: the active player's triggers go first.
    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(queued(ghoul, P0, &s));
    s.awaiting_action = Some(prompt(ghoul,
        vec![Target::Object(ghoul), Target::Object(other)], false, P1));
    flags(&s, &reg, "while the active player's triggers wait (CR 603.3b)");
    let _ = card_id;
}

/// CR 700.3a/700.3c: a pile division and a pile choice are about
/// battlefield permanents of the player who will sacrifice them.
#[test]
fn the_pile_prompts_are_about_battlefield_permanents() {
    let (mut state, reg) = base();
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let buried = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P1);
    let ghost = PlayerId(u8::try_from(state.players.len()).unwrap());

    let divide = |permanents: Vec<ObjectId>, target: PlayerId| AwaitingAction::ResolutionChoice {
        player: P0, source: mine,
        choice: ResolutionChoiceKind::DividePermanentsIntoPiles {
            description: "d".into(), permanents, target_player: target, source_id: mine } };

    let mut s = state.clone();
    s.awaiting_action = Some(divide(vec![theirs], P1));
    quiet_about(&s, &reg, "pile prompt");

    let mut s = state.clone();
    s.awaiting_action = Some(divide(vec![theirs], ghost));
    flags(&s, &reg, "pile prompt for p2 who is not a player");

    let mut s = state.clone();
    s.awaiting_action = Some(divide(vec![buried], P1));
    flags(&s, &reg, "does not control on the battlefield (CR 700.3c)");

    let mut s = state.clone();
    s.awaiting_action = Some(divide(vec![theirs, theirs], P1));
    flags(&s, &reg, "pile prompt lists #");

    // The choice between the two piles.
    let pile = |p1: Vec<ObjectId>, p2: Vec<ObjectId>| AwaitingAction::ResolutionChoice {
        player: P1, source: mine,
        choice: ResolutionChoiceKind::ChoosePile {
            description: "d".into(), pile_1: p1, pile_2: p2, source_id: mine } };

    let mut s = state.clone();
    s.awaiting_action = Some(pile(vec![theirs], vec![]));
    quiet_about(&s, &reg, "pile choice");

    let mut s = state.clone();
    s.awaiting_action = Some(pile(vec![], vec![]));
    flags(&s, &reg, "pile choice between two empty piles");

    let mut s = state.clone();
    s.awaiting_action = Some(pile(vec![buried], vec![]));
    flags(&s, &reg, "is not on the battlefield (CR 700.3c)");
}

/// CR 509.2: the damage assignment order is announced by the attacking
/// player, over the creatures blocking one of their attackers, and only
/// where there is something to choose between.
#[test]
fn the_damage_assignment_order_prompt_orders_one_attackers_blockers() {
    let (mut state, reg) = base();
    let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let first = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let second = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.step = Step::DeclareAttackers;
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    state.step = Step::DeclareBlockers;
    submit_declare_blockers(&mut state, P1, &[(first, attacker), (second, attacker)], &reg);
    state.events.clear();
    state.trigger_event_index = 0;
    state.priority_player = Some(P0);

    let order = |source: ObjectId, attacker: ObjectId, remaining: Vec<ObjectId>,
                 options: Vec<String>, who: PlayerId| AwaitingAction::ResolutionChoice {
        player: who, source,
        choice: ResolutionChoiceKind::ChooseDamageAssignmentOrder {
            description: "d".into(), attacker, remaining, options } };

    let both = || vec![first, second];
    let labels = || vec!["a".into(), "b".into()];

    let mut s = state.clone();
    s.awaiting_action = Some(order(attacker, attacker, both(), labels(), P0));
    quiet_about(&s, &reg, "damage-assignment-order prompt");

    // One blocker is not an order to choose.
    let mut s = state.clone();
    s.awaiting_action = Some(order(attacker, attacker, vec![first], vec!["a".into()], P0));
    flags(&s, &reg, "with 1 options for 1 blockers");

    // The defending player does not announce it.
    let mut s = state.clone();
    s.awaiting_action = Some(order(attacker, attacker, both(), labels(), P1));
    flags(&s, &reg, "not the attacking player p0");

    // The prompt's source is the attacker being ordered.
    let mut s = state.clone();
    s.awaiting_action = Some(order(first, attacker, both(), labels(), P0));
    flags(&s, &reg, "while ordering #");

    // Something that is not blocking that attacker.
    let mut s = state.clone();
    let bystander = named_permanent(&mut s, &reg, "Grizzly Bears", P1);
    s.awaiting_action = Some(order(attacker, attacker, vec![first, bystander], labels(), P0));
    flags(&s, &reg, "which is not blocking #");
}

/// CR 601.2b/608.2: a pay-or-not prompt is about a spell on the stack,
/// raised by the spell that is resolving, for a cost with no unannounced X.
#[test]
fn a_pay_or_not_prompt_names_the_spell_it_is_about() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let pump = castable_spell(&mut state, &reg, "Moment of Heroism", P0);
    let mut state = cast_onto_stack(&state, &reg, pump, vec![Target::Object(bear)]);
    state.resolving_spell = Some(pump);

    let pay = |spell: ObjectId, source: ObjectId, cost: mtg_engine::types::ManaCost|
        AwaitingAction::ResolutionChoice {
            player: P0, source,
            choice: ResolutionChoiceKind::PayOrNot {
                description: "d".into(), cost, spell_id: spell, source_spell_id: source } };

    let free = mtg_engine::types::ManaCost::free();

    let mut s = state.clone();
    s.awaiting_action = Some(pay(bear, pump, free.clone()));
    flags(&s, &reg, "which is not on the stack");

    let mut s = state.clone();
    s.awaiting_action = Some(pay(pump, bear, free));
    flags(&s, &reg, "which is not the resolving spell");

    let mut s = state.clone();
    let with_x = mtg_engine::types::ManaCost::new(vec![mtg_engine::types::ManaSymbol::X]);
    s.awaiting_action = Some(pay(pump, pump, with_x));
    flags(&s, &reg, "pay-or-not prompt with an unannounced X");
}

/// CR 603.3b: a trigger-order prompt orders the asking player's own
/// triggers, from the queue their seat owns, by increasing position, and
/// only where there is something to order.
#[test]
fn a_trigger_order_prompt_orders_its_own_queue() {
    let (mut state, reg) = base();
    let ghoul = named_permanent(&mut state, &reg, "Abattoir Ghoul", P0);
    let other = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let queued = |src: ObjectId, controller: PlayerId, s: &GameState| {
        mtg_engine::triggers::PendingTrigger::new(
            mtg_engine::triggers::TriggerSource::new(src, s.get_object(src).unwrap().card_id,
                controller, "t"),
            mtg_engine::triggers::TriggerEvent::StateTriggered)
    };
    let prompt = |source: ObjectId, indices: Vec<usize>, options: Vec<String>,
                  ap_queue: bool, player: PlayerId| AwaitingAction::ResolutionChoice {
        player, source,
        choice: ResolutionChoiceKind::ChooseTriggerOrder {
            description: "d".into(), options, ap_queue, indices, details: vec![] } };
    let labels = || vec!["a".into(), "b".into()];

    let mut ap = state.clone();
    ap.pending_trigger_pushes_ap.push(queued(ghoul, P0, &ap));
    ap.pending_trigger_pushes_ap.push(queued(other, P0, &ap));

    let mut s = ap.clone();
    s.awaiting_action = Some(prompt(ghoul, vec![0, 1], labels(), true, P0));
    quiet_about(&s, &reg, "trigger-order prompt");

    // One trigger is not an order to choose.
    let mut s = ap.clone();
    s.awaiting_action = Some(prompt(ghoul, vec![0], vec!["a".into()], true, P0));
    flags(&s, &reg, "with 1 options for 1 indices");

    // The positions are increasing.
    let mut s = ap.clone();
    s.awaiting_action = Some(prompt(ghoul, vec![1, 0], labels(), true, P0));
    flags(&s, &reg, "are not increasing");

    // A position past the end of the queue.
    let mut s = ap.clone();
    s.awaiting_action = Some(prompt(ghoul, vec![0, 5], labels(), true, P0));
    flags(&s, &reg, "is past the queue of 2");

    // The prompt's source is the first trigger it orders.
    let mut s = ap.clone();
    s.awaiting_action = Some(prompt(other, vec![0, 1], labels(), true, P0));
    flags(&s, &reg, "but its first trigger is from #");

    // Somebody else's trigger.
    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(queued(ghoul, P0, &s));
    s.pending_trigger_pushes_ap.push(queued(theirs, P1, &s));
    s.awaiting_action = Some(prompt(ghoul, vec![0, 1], labels(), true, P0));
    flags(&s, &reg, "orders p1's trigger");

    // CR 603.3b: the AP queue belongs to the active player, and the NAP's
    // triggers wait for it.
    let mut s = ap.clone();
    s.awaiting_action = Some(prompt(ghoul, vec![0, 1], labels(), false, P0));
    flags(&s, &reg, "ap_queue=false for p0 while p0 is active (CR 603.3b)");

    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(queued(ghoul, P0, &s));
    s.pending_trigger_pushes_nap.push(queued(theirs, P1, &s));
    s.pending_trigger_pushes_nap.push(queued(theirs, P1, &s));
    s.awaiting_action = Some(prompt(theirs, vec![0, 1], labels(), false, P1));
    flags(&s, &reg, "while active-player triggers wait (CR 603.3b)");
}

/// CR 701.23a: every prompt that reads a library reads the searcher's own,
/// and only cards that are still in it.
#[test]
fn a_library_prompt_offers_cards_that_are_in_that_library() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let mine = stock_library(&mut state, &reg, P0, 2);
    let theirs = stock_library(&mut state, &reg, P1, 1);
    for id in mine.iter().chain(&theirs) {
        state.get_object_mut(*id).unwrap().name = "Forest".into();
    }
    let ghost = PlayerId(u8::try_from(state.players.len()).unwrap());

    let search = |options: Vec<ObjectId>, searcher: PlayerId| AwaitingAction::ResolutionChoice {
        player: P0, source: bear,
        choice: ResolutionChoiceKind::ChooseFromLibrary {
            description: "d".into(), options, searcher, source_id: bear,
            destination: Zone::Hand, tapped: false } };

    let mut s = state.clone();
    s.awaiting_action = Some(search(mine.clone(), P0));
    quiet_about(&s, &reg, "library prompt");

    let mut s = state.clone();
    s.awaiting_action = Some(search(vec![theirs[0]], P0));
    flags(&s, &reg, "which is not in p0's library (CR 701.23a)");

    let mut s = state.clone();
    s.awaiting_action = Some(search(vec![mine[0], mine[0]], P0));
    flags(&s, &reg, "library prompt lists #");

    let mut s = state.clone();
    s.awaiting_action = Some(search(mine.clone(), ghost));
    flags(&s, &reg, "library prompt for p2 who is not a player");

    // CR 701.16a: looking moves nothing — the cards looked at are still in
    // the library until the answer, and are never tokens.
    let looked = |cards: Vec<ObjectId>| AwaitingAction::ResolutionChoice {
        player: P0, source: bear,
        choice: ResolutionChoiceKind::ChooseFromLookedAt {
            description: "d".into(), looked_at: cards } };

    let mut s = state.clone();
    s.awaiting_action = Some(looked(mine.clone()));
    quiet_about(&s, &reg, "looked-at prompt");

    let mut s = state.clone();
    s.awaiting_action = Some(looked(vec![ObjectId(4242)]));
    flags(&s, &reg, "looked-at prompt offers missing #4242");

    let mut s = state.clone();
    let token = s.create_token_with_subtypes("", P0, 2, 2, vec![Color::Green],
        vec![CardType::Creature], vec![], vec!["Wolf".into()], &reg)[0];
    s.awaiting_action = Some(looked(vec![token]));
    flags(&s, &reg, "looked-at prompt offers token #");
}

/// A hand prompt asks the player whose hand it is, for cards that are in
/// it, while there is still something to take.
#[test]
fn a_hand_prompt_reads_the_hand_of_the_player_it_asks() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let mine = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);
    let theirs = spell_in_hand(&mut state, &reg, "Moment of Heroism", P1);

    let hand = |who: PlayerId, cards: Vec<ObjectId>, remaining: usize|
        AwaitingAction::ResolutionChoice {
            player: P0, source: bear,
            choice: ResolutionChoiceKind::ChooseCardFromHand {
                description: "d".into(), player: who, cards,
                discard_immediately: true, remaining } };

    let mut s = state.clone();
    s.awaiting_action = Some(hand(P0, vec![mine], 1));
    quiet_about(&s, &reg, "hand prompt");

    let mut s = state.clone();
    s.awaiting_action = Some(hand(P1, vec![theirs], 1));
    flags(&s, &reg, "hand prompt for p1 answered by p0");

    let mut s = state.clone();
    s.awaiting_action = Some(hand(P0, vec![theirs], 1));
    flags(&s, &reg, "which is not in p0's hand");

    let mut s = state.clone();
    s.awaiting_action = Some(hand(P0, vec![mine], 0));
    flags(&s, &reg, "hand prompt with nothing left to choose");

    let mut s = state.clone();
    s.awaiting_action = Some(hand(P0, vec![mine, mine], 1));
    flags(&s, &reg, "hand prompt lists #");

    // A name or type prompt with nothing to name.
    let mut s = state.clone();
    s.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: P0, source: bear,
        choice: ResolutionChoiceKind::ChooseCardType {
            description: "d".into(), options: vec![], controller: P0 } });
    flags(&s, &reg, "a name/type prompt offers nothing");

    // And a card-type prompt is answered by the player it names.
    let mut s = state.clone();
    s.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: P0, source: bear,
        choice: ResolutionChoiceKind::ChooseCardType {
            description: "d".into(), options: vec!["Creature".into()], controller: P1 } });
    flags(&s, &reg, "card-type prompt for p1 answered by p0");
}

/// CR 508.4b: a token put onto the battlefield attacking chooses whom it
/// attacks — a token its controller controls, inside combat, from a list
/// that does not include itself.
#[test]
fn a_token_attacks_prompt_is_about_a_token_in_combat() {
    let (mut state, reg) = base();
    let source = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let lili = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    set_loyalty(&mut state, lili, 3);
    state.step = Step::DeclareAttackers;
    let mut c = mtg_engine::state::CombatState::new();
    c.any_attackers_declared = true;
    c.attackers.insert(source, P1);
    state.combat = Some(c);
    let token = state.create_token_with_subtypes("", P0, 2, 2, vec![Color::Green],
        vec![CardType::Creature], vec![], vec!["Wolf".into()], &reg)[0];
    let other_token = state.create_token_with_subtypes("", P0, 2, 2, vec![Color::Green],
        vec![CardType::Creature], vec![], vec!["Wolf".into()], &reg)[0];

    let prompt = |token_id: ObjectId, remaining: Vec<ObjectId>, options: Vec<Target>|
        AwaitingAction::ResolutionChoice {
            player: P0, source,
            choice: ResolutionChoiceKind::ChooseTarget {
                description: "d".into(), options, optional: false,
                effect: PendingEffect::TokenAttacks {
                    token_id, remaining, source_id: source } } };

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(token, vec![other_token],
        vec![Target::Player(P1), Target::Object(lili)]));
    quiet_about(&s, &reg, "token-attacks prompt");

    // A token the asking player does not control.
    let mut s = state.clone();
    s.get_object_mut(token).unwrap().controller = P1;
    s.awaiting_action = Some(prompt(token, vec![], vec![Target::Player(P1)]));
    flags(&s, &reg, "which p0 does not control on the battlefield");

    // Outside combat there is nothing to attack into.
    let mut s = state.clone();
    s.combat = None;
    s.step = Step::PrecombatMain;
    s.awaiting_action = Some(prompt(token, vec![], vec![Target::Player(P1)]));
    flags(&s, &reg, "token-attacks prompt outside combat");

    // The token being asked about is not one of the ones still to ask.
    let mut s = state.clone();
    s.awaiting_action = Some(prompt(token, vec![token], vec![Target::Player(P1)]));
    flags(&s, &reg, "lists the token among the remaining ones");

    // CR 508.4b: what it may attack is an opponent or their planeswalker.
    let mut s = state.clone();
    s.awaiting_action = Some(prompt(token, vec![], vec![Target::Player(P0)]));
    flags(&s, &reg, "which is no opponent or opposing planeswalker (CR 508.4b)");
    let mut s = state.clone();
    s.awaiting_action = Some(prompt(token, vec![], vec![Target::Object(source)]));
    flags(&s, &reg, "which is no opponent or opposing planeswalker (CR 508.4b)");
}
