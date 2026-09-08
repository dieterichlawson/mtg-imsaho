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

fn cast_stash(state: &GameState, card: ObjectId) -> mtg_engine::state::PendingSpellCast {
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

    // Each of the three queues a trigger can be waiting in is its own way
    // of not being quiet yet (CR 603.3b).
    let waiting = mtg_engine::triggers::PendingTrigger::new(
        mtg_engine::triggers::TriggerSource::new(bear, card_id, P0, "a triggered ability"),
        mtg_engine::triggers::TriggerEvent::Upkeep);
    for queue in [0, 1, 2] {
        let mut s = state.clone();
        match queue {
            0 => s.pending_triggers.push(waiting.clone()),
            1 => s.pending_trigger_pushes_ap.push(waiting.clone()),
            _ => s.pending_trigger_pushes_nap.push(waiting.clone()),
        }
        flags(&s, &reg, "attackers prompt with triggers still queued");
    }

    // And each of the three ways a cast or a resolution can still be in
    // flight (CR 601.2, 608.2).
    let mut s = state.clone();
    s.resolving_spell = Some(bear);
    flags(&s, &reg, "with a cast or resolution in progress");
    let mut s = state.clone();
    s.pending_spell_cast = Some(cast_stash(&state, bear));
    flags(&s, &reg, "with a cast or resolution in progress");
    let mut s = state.clone();
    s.pending_ability_effect = Some(mtg_engine::state::PendingAbilityEffect {
        source_id: bear, ability_index: 0, behavior_card_id: card_id,
        targets: vec![], description: "an ability".into(), activator: P0,
        target_requirement: None, unpaid: None,
    });
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

    // Turn one, the first turn, in the untap step: each alone, because a
    // chain of three is only tested by breaking each of them by itself.
    let mut s = keeping.clone();
    s.turn_number = 2;
    s.is_first_turn = false;
    flags(&s, &reg, "mulligan phase on turn 2");

    let mut s = keeping.clone();
    s.turn_number = 2;
    flags(&s, &reg, "mulligan phase on turn 2");

    let mut s = keeping.clone();
    s.is_first_turn = false;
    flags(&s, &reg, "mulligan phase on turn 1");

    let mut s = keeping.clone();
    s.step = Step::Upkeep;
    flags(&s, &reg, "mulligan phase on turn 1 in Upkeep");

    // CR 103.4: a queued bottoming is one player's, and never more cards
    // than an opening hand holds.
    let mut s = keeping.clone();
    s.pending_mulligan_bottoms.push((P0, 1));
    assert!(!check_core(&s, &reg).iter().any(|m| m.contains("queued bottoming")),
        "one card for a real player is the ordinary case: {:?}", check_core(&s, &reg));
    // Seven is the opening hand, and the most a keep can ever owe.
    let mut s = keeping.clone();
    s.pending_mulligan_bottoms.push((P0, 7));
    assert!(!check_core(&s, &reg).iter().any(|m| m.contains("queued bottoming")),
        "a whole hand is a legal bottoming: {:?}", check_core(&s, &reg));
    let mut s = keeping.clone();
    s.pending_mulligan_bottoms.push((P0, 8));
    flags(&s, &reg, "queued bottoming of 8 for p0");
    let mut s = keeping.clone();
    s.pending_mulligan_bottoms.push((PlayerId(9), 1));
    flags(&s, &reg, "queued bottoming of 1 for p9");

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
    s.get_player_mut(P0).lost = true;
    flags(&s, &reg, "already has turn state");

    let mut s = keeping.clone();
    s.get_player_mut(P0).has_drawn_from_empty = true;
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
    // The two prompts that name their source under a different field name
    // are read the same way.
    let mut s = state.clone();
    s.awaiting_action = Some(prompt(ResolutionChoiceKind::YesNo {
        description: "d".into(), source_card: other }));
    flags(&s, &reg, "carries a choice for #");
    let mut s = state.clone();
    s.awaiting_action = Some(prompt(ResolutionChoiceKind::PayOrNot {
        description: "d".into(), spell_id: bear, source_spell_id: other,
        cost: ManaCost::new(vec![ManaSymbol::Generic(1)]) }));
    flags(&s, &reg, "carries a choice for #");

    // And a valid player among the options is not a missing one: the range
    // check is a check, not a blanket refusal of players.
    let mut s = state.clone();
    s.awaiting_action = Some(prompt(ResolutionChoiceKind::ChooseTarget {
        description: "d".into(), options: vec![Target::Player(P1)], optional: false,
        effect: PendingEffect::DealDamage { amount: 2, source_id: bear } }));
    quiet_about(&s, &reg, "who is not a player");
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

    // The two effects that debuff or forbid a block act on creatures on the
    // battlefield the same way destroy does, and are read the same way.
    for (what, effect) in [
        ("debuff", PendingEffect::DebuffUntilEOT { power: -2, toughness: -2, source_name: "x".into() }),
        ("can't-block", PendingEffect::CantBlockThisTurn { source_name: "x".into() }),
    ] {
        let mut s = state.clone();
        s.awaiting_action = Some(prompt(effect.clone(), vec![Target::Object(land)]));
        flags(&s, &reg, &format!("{what} prompt offers"));
        let mut s = state.clone();
        s.awaiting_action = Some(prompt(effect, vec![Target::Object(theirs)]));
        quiet_about(&s, &reg, &format!("{what} prompt offers"));
    }
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

    // The group is "legends of that name that THIS player controls on the
    // battlefield" — each half of that alone, since a copy under the
    // opponent, a same-named non-legend, or one in another zone is not part
    // of the group and offering it is a permanent destroyed for nothing.
    let mut s = state.clone();
    let theirs = named_permanent(&mut s, &reg, "Geist of Saint Traft", P1);
    s.awaiting_action = Some(legend(
        vec![Target::Object(a), Target::Object(b), Target::Object(theirs)], false, P0));
    flags(&s, &reg, "(CR 704.5j)");

    let mut s = state.clone();
    let elsewhere = named_card_in_graveyard(&mut s, &reg, "Geist of Saint Traft", P0);
    s.awaiting_action = Some(legend(
        vec![Target::Object(a), Target::Object(b), Target::Object(elsewhere)], false, P0));
    flags(&s, &reg, "(CR 704.5j)");

    // A legend of another name is another group (CR 704.5j is per name).
    let mut s = state.clone();
    let other_legend = named_permanent(&mut s, &reg, "Mikaeus, the Lunarch", P0);
    s.awaiting_action = Some(legend(
        vec![Target::Object(a), Target::Object(b), Target::Object(other_legend)], false, P0));
    flags(&s, &reg, "(CR 704.5j)");

    // Three of them is still one group, and still not flagged.
    let mut s = state.clone();
    let c = named_permanent(&mut s, &reg, "Geist of Saint Traft", P0);
    s.awaiting_action = Some(legend(
        vec![Target::Object(a), Target::Object(b), Target::Object(c)], false, P0));
    quiet_about(&s, &reg, "(CR 704.5j)");
}

/// CR 608.2d/701.23a/508.4b: the things a prompt offers are things the
/// effect could really act on — a battlefield creature or planeswalker for
/// damage, a card in the searcher's own library for a search, and an
/// opponent or their planeswalker for a token that entered attacking.
#[test]
fn a_prompts_options_are_ones_its_effect_could_act_on() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let walker = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    let in_gy = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    let source = named_permanent(&mut state, &reg, "Rage Thrower", P0);

    let damage = |options: Vec<Target>| AwaitingAction::ResolutionChoice {
        player: P0, source,
        choice: ResolutionChoiceKind::ChooseTarget {
            description: "d".into(), options, optional: false,
            effect: PendingEffect::DealDamage { amount: 1, source_id: source } } };

    // A creature and a planeswalker on the battlefield are both damageable.
    let mut s = state.clone();
    s.awaiting_action = Some(damage(vec![Target::Object(bear), Target::Object(walker)]));
    quiet_about(&s, &reg, "damage prompt offers");
    // A creature in a graveyard is not.
    let mut s = state.clone();
    s.awaiting_action = Some(damage(vec![Target::Object(bear), Target::Object(in_gy)]));
    flags(&s, &reg, "which is no battlefield creature or planeswalker");
    // Nor is a land.
    let mut s = state.clone();
    let land = named_permanent(&mut s, &reg, "Forest", P0);
    s.awaiting_action = Some(damage(vec![Target::Object(bear), Target::Object(land)]));
    flags(&s, &reg, "which is no battlefield creature or planeswalker");

    // CR 508.4b: a token put onto the battlefield attacking is sent at an
    // opponent or a planeswalker they control.
    let mut with_token = state.clone();
    with_token.combat = Some(mtg_engine::state::CombatState::new());
    let token = with_token.create_token_with_subtypes("", P0, 2, 2, vec![Color::Green],
        vec![CardType::Creature], vec![], vec!["Wolf".into()], &reg)[0];
    let attacks = |s: &GameState, options: Vec<Target>| AwaitingAction::ResolutionChoice {
        player: P0, source: token,
        choice: ResolutionChoiceKind::ChooseTarget {
            description: "d".into(), options, optional: false,
            effect: PendingEffect::TokenAttacks {
                token_id: token, remaining: vec![], source_id: s.get_object(token).unwrap().id } } };

    let mut s = with_token.clone();
    s.awaiting_action = Some(attacks(&with_token, vec![Target::Player(P1), Target::Object(walker)]));
    quiet_about(&s, &reg, "(CR 508.4b)");
    // Its own controller, or their own planeswalker, is not an option.
    let mut s = with_token.clone();
    s.awaiting_action = Some(attacks(&with_token, vec![Target::Player(P1), Target::Player(P0)]));
    flags(&s, &reg, "(CR 508.4b)");
    let mut s = with_token.clone();
    let mine = named_permanent(&mut s, &reg, "Liliana of the Veil", P0);
    s.awaiting_action = Some(attacks(&with_token, vec![Target::Player(P1), Target::Object(mine)]));
    flags(&s, &reg, "(CR 508.4b)");
    // Nor is a creature the opponent controls: a token attacks a player or a
    // planeswalker, never a creature.
    let mut s = with_token.clone();
    let theirs = named_permanent(&mut s, &reg, "Grizzly Bears", P1);
    s.awaiting_action = Some(attacks(&with_token, vec![Target::Player(P1), Target::Object(theirs)]));
    flags(&s, &reg, "(CR 508.4b)");

    // CR 701.23a: a search offers cards from the searcher's own library.
    let searching = |options: Vec<Target>| AwaitingAction::ResolutionChoice {
        player: P0, source,
        choice: ResolutionChoiceKind::ChooseTarget {
            description: "d".into(), options, optional: false,
            effect: PendingEffect::FinishLibrarySearch {
                searcher: P0, destination: Zone::Hand, tapped: false } } };
    let mut s = state.clone();
    let mine = s.create_object(s.get_object(bear).unwrap().card_id, P0, Zone::Library, Some(2), Some(2));
    s.get_player_mut(P0).library_order.push(mine);
    let theirs = s.create_object(s.get_object(bear).unwrap().card_id, P1, Zone::Library, Some(2), Some(2));
    s.get_player_mut(P1).library_order.push(theirs);
    let mut ok = s.clone();
    ok.awaiting_action = Some(searching(vec![Target::Object(mine)]));
    quiet_about(&ok, &reg, "(CR 701.23a)");
    // Somebody else's library.
    let mut bad = s.clone();
    bad.awaiting_action = Some(searching(vec![Target::Object(theirs)]));
    flags(&bad, &reg, "(CR 701.23a)");
    // A card that says it is in the library but is in no library order.
    let mut bad = s.clone();
    bad.get_player_mut(P0).library_order.retain(|&id| id != mine);
    bad.awaiting_action = Some(searching(vec![Target::Object(mine)]));
    flags(&bad, &reg, "(CR 701.23a)");
    // And a search offers cards, not players.
    let mut bad = s.clone();
    bad.awaiting_action = Some(searching(vec![Target::Player(P1)]));
    flags(&bad, &reg, "library search offers");
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

    // The prompt the front of the queue really is waiting on.
    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(queued(ghoul, P0, &s));
    s.awaiting_action = Some(prompt(ghoul,
        vec![Target::Object(ghoul), Target::Object(other)], false, P0));
    assert!(!check_core(&s, &reg).iter().any(|m| m.contains("but the queue's front is #")),
        "the prompt matches the queue's front: {:?}", check_core(&s, &reg));

    // Each way it can fail to match, one at a time — a chain of three
    // conditions is only tested by breaking each of them alone.
    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(queued(other, P0, &s));
    s.awaiting_action = Some(prompt(ghoul,
        vec![Target::Object(ghoul), Target::Object(other)], false, P0));
    flags(&s, &reg, "but the queue's front is #");

    // The same trigger, but its controller is not the player being asked
    // (CR 603.3d: the trigger's controller chooses its targets).
    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(queued(ghoul, P1, &s));
    s.awaiting_action = Some(prompt(ghoul,
        vec![Target::Object(ghoul), Target::Object(other)], false, P0));
    flags(&s, &reg, "but the queue's front is #");

    // The same trigger, already carrying a target: the question was
    // answered once and is being asked again.
    let mut s = state.clone();
    let mut answered = queued(ghoul, P0, &s);
    answered.source.chosen_targets = vec![Target::Object(other)];
    s.pending_trigger_pushes_ap.push(answered);
    s.awaiting_action = Some(prompt(ghoul,
        vec![Target::Object(ghoul), Target::Object(other)], false, P0));
    flags(&s, &reg, "but the queue's front is #");

    // A prompt with nothing to choose between, or an optional one. Two
    // options is the smallest real choice and is not one of those.
    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(queued(ghoul, P0, &s));
    s.awaiting_action = Some(prompt(ghoul, vec![Target::Object(other)], false, P0));
    flags(&s, &reg, "trigger-target prompt with 1 options, optional=false");
    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(queued(ghoul, P0, &s));
    s.awaiting_action = Some(prompt(ghoul,
        vec![Target::Object(ghoul), Target::Object(other)], false, P0));
    quiet_about(&s, &reg, "options, optional=");

    // CR 603.3b: the active player's triggers go first — which is a rule
    // about the NON-active player answering while they wait. The active
    // player answering out of that same queue is the queue being worked.
    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(queued(ghoul, P0, &s));
    s.awaiting_action = Some(prompt(ghoul,
        vec![Target::Object(ghoul), Target::Object(other)], false, P1));
    flags(&s, &reg, "while the active player's triggers wait (CR 603.3b)");
    let mut s = state.clone();
    s.pending_trigger_pushes_ap.push(queued(ghoul, P0, &s));
    s.awaiting_action = Some(prompt(ghoul,
        vec![Target::Object(ghoul), Target::Object(other)], false, P0));
    quiet_about(&s, &reg, "while the active player's triggers wait");
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
