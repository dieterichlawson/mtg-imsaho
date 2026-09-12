//! Self-tests for the legal-action-set invariants
//! (`mtg_engine::invariants::check_legal`): the menu a player is handed
//! offers exactly the game the rules allow, and nothing else.
//!
//! Same contract as the other invariant self-tests — the checker is the
//! fuzzer's only pair of eyes, so every clause needs an offer that violates
//! it, and every conditional clause a neighbouring offer that does not.

mod common;
use common::*;
use mtg_engine::actions::{Action, CombatPrompt, ResolvedChoice, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::LegalActions;
use mtg_engine::ids::ObjectId;
use mtg_engine::invariants::check_legal;
use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind, StackEntry};
use mtg_engine::types::*;

fn base() -> (GameState, CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.turn_number = 3;
    (state, reg)
}

#[track_caller]
fn flags(state: &GameState, acting: PlayerId, legal: &LegalActions, reg: &CardRegistry, needle: &str) {
    let v = check_legal(state, acting, legal, reg);
    assert!(v.iter().any(|m| m.contains(needle)),
        "expected a legal-set violation containing {needle:?}, got: {v:?}");
}

#[track_caller]
fn quiet_about(state: &GameState, acting: PlayerId, legal: &LegalActions, reg: &CardRegistry, needle: &str) {
    let v = check_legal(state, acting, legal, reg);
    assert!(!v.iter().any(|m| m.contains(needle)),
        "expected no legal-set violation containing {needle:?}, got: {v:?}");
}

/// The offers the engine would make for `state`, for a test that then adds
/// one of its own.
fn wrong_legal(state: &GameState, reg: &CardRegistry) -> LegalActions {
    mtg_engine::engine::legal_actions(state, reg)
}

#[track_caller]
fn clean(state: &GameState, acting: PlayerId, legal: &LegalActions, reg: &CardRegistry) {
    assert_eq!(check_legal(state, acting, legal, reg), Vec::<String>::new());
}

/// CR 117.1/117.3: a priority offer is a pass, a concede, and the things
/// this player may actually do — one of each bookend and no prompts.
#[test]
fn the_priority_offers_bookends_are_exactly_one_each() {
    let (mut state, reg) = base();
    named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    clean(&state, P0, &legal, &reg);

    // Concede offered twice — the menu ends with one, but there are two.
    let mut l = legal.clone();
    l.actions.insert(1, Action::Concede);
    flags(&state, P0, &l, &reg, "has 1 PassPriority and 2 Concede entries");

    // A priority offer carries no prompt of its own.
    let mut l = legal.clone();
    l.resolution_prompt = Some(ResolutionChoiceKind::YesNo {
        description: String::new(), source_card: ObjectId(1) });
    flags(&state, P0, &l, &reg, "priority offer with a combat or resolution prompt attached");

    // And nothing but the actions a player takes with priority.
    let mut l = legal.clone();
    l.actions.insert(1, Action::MulliganKeep);
    flags(&state, P0, &l, &reg, "priority offer offers MulliganKeep");
}

/// Each prompt in the state has exactly one shape of answer, addressed to
/// exactly one player.
#[test]
fn every_prompt_shape_is_checked_against_its_prompt() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let card = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);

    // A mulligan prompt: keep or mulligan, nothing else, to that player.
    let mut s = state.clone();
    s.priority_player = None;
    s.awaiting_action = Some(AwaitingAction::MulliganDecision { player: P0 });
    let legal = mtg_engine::engine::legal_actions(&s, &reg);
    clean(&s, P0, &legal, &reg);
    flags(&s, P1, &legal, &reg, "mulligan prompt offered to p1, not p0");
    let mut l = legal.clone();
    l.actions.push(Action::Concede);
    flags(&s, P0, &l, &reg, "mulligan prompt offers Concede");
    let mut l = legal.clone();
    l.actions.retain(|a| !matches!(a, Action::MulliganMull));
    flags(&s, P0, &l, &reg, "mulligan offer has 1 keep and 0 mulligan entries");
    let mut l = legal.clone();
    l.castable_spells.push(mtg_engine::actions::CastableSpell {
        object_id: card, name: "Moment of Heroism".into(), is_flashback: false,
        target_spec: mtg_engine::actions::CastTargetSpec::NoTargets, tap_plan: vec![],
        exile_x_from_gy_max: None, sacrifice_options: vec![], additional_cost_label: None,
        alternative_cost: None, from_graveyard: false });
    flags(&s, P0, &l, &reg, "mulligan prompt lists castable spells or activatable abilities");

    // A bottoming prompt.
    let mut s = state.clone();
    s.priority_player = None;
    s.awaiting_action = Some(AwaitingAction::BottomAfterMulligan { player: P0, count: 1 });
    let legal = mtg_engine::engine::legal_actions(&s, &reg);
    clean(&s, P0, &legal, &reg);
    flags(&s, P1, &legal, &reg, "bottoming prompt offered to p1, not p0");
    // The answer is a set, so there is no menu beside the prompt at all.
    let mut l = legal.clone();
    l.actions.push(Action::MulliganKeep);
    flags(&s, P0, &l, &reg, "bottoming prompt with 1 flat actions");

    // A discard prompt.
    let mut s = state.clone();
    s.priority_player = None;
    s.step = Step::Cleanup;
    s.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P0, discard_count: 1 });
    let legal = mtg_engine::engine::legal_actions(&s, &reg);
    clean(&s, P0, &legal, &reg);
    flags(&s, P1, &legal, &reg, "discard prompt offered to p1, not p0");
    let mut l = legal.clone();
    l.set_prompt = None;
    flags(&s, P0, &l, &reg, "discard prompt with nothing to choose");

    // A resolution prompt carries the choice that is actually pending.
    let mut s = state.clone();
    s.priority_player = Some(P0);
    s.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: P0, source: bear,
        choice: ResolutionChoiceKind::YesNo { description: "?".into(), source_card: bear } });
    let legal = mtg_engine::engine::legal_actions(&s, &reg);
    clean(&s, P0, &legal, &reg);
    let mut l = legal.clone();
    l.resolution_prompt = None;
    flags(&s, P0, &l, &reg, "resolution prompt carries None, not the pending choice");
    let mut l = legal.clone();
    l.combat_prompt = Some(CombatPrompt::ChooseAttackers {
        eligible: vec![], must_attack: vec![], defending_player: P1, defending_planeswalkers: vec![] });
    flags(&s, P0, &l, &reg, "resolution prompt with a combat prompt attached");
}

/// The combat prompts are the whole of what the state asks: to the right
/// player, with no flat actions beside them.
#[test]
fn a_combat_prompt_is_the_whole_offer() {
    let (mut state, reg) = base();
    named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.step = Step::DeclareAttackers;
    state.awaiting_action = Some(AwaitingAction::DeclareAttackers);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    clean(&state, P0, &legal, &reg);

    flags(&state, P1, &legal, &reg, "attackers prompt offered to p1, not the active player");
    let mut l = legal.clone();
    l.combat_prompt = None;
    flags(&state, P0, &l, &reg, "attackers prompt without a ChooseAttackers prompt");
    let mut l = legal.clone();
    l.actions.push(Action::Concede);
    flags(&state, P0, &l, &reg, "attackers prompt with 1 flat actions");

    // Blockers, the same way.
    let (mut state, reg) = base();
    let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.step = Step::DeclareAttackers;
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    state.step = Step::DeclareBlockers;
    state.awaiting_action = Some(AwaitingAction::DeclareBlockers { defending_player: P1 });
    state.priority_player = Some(P1);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    clean(&state, P1, &legal, &reg);
    flags(&state, P0, &legal, &reg, "blockers prompt offered to p0, not the defender");
    let mut l = legal.clone();
    l.combat_prompt = None;
    flags(&state, P1, &l, &reg, "blockers prompt without a ChooseBlockers prompt");
}

/// CR 601.3a/305.9/307.1: a cast offer names a card its owner may cast,
/// from a zone they may cast it from, at a time they may cast it.
#[test]
fn a_cast_offer_names_a_castable_card() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let pump = castable_spell(&mut state, &reg, "Moment of Heroism", P0);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    assert!(legal.actions.iter().any(|a| matches!(a,
        Action::CastSpell { object_id, .. } if *object_id == pump)),
        "precondition: the spell is offered");
    clean(&state, P0, &legal, &reg);

    // An offer for a card that is not there at all.
    let mut l = legal.clone();
    l.actions.insert(1, cast_action(ObjectId(4242), vec![Target::Object(bear)]));
    flags(&state, P0, &l, &reg, "names a missing object");

    // CR 305.1: a land offered to be played is in the acting player's hand
    // — each half of that alone.
    let mut s = state.clone();
    let mine = spell_in_hand(&mut s, &reg, "Forest", P0);
    let theirs = spell_in_hand(&mut s, &reg, "Forest", P1);
    let played = named_permanent(&mut s, &reg, "Forest", P0);
    let playing = |id: ObjectId| {
        let mut l = legal.clone();
        l.actions.insert(1, Action::PlayLand { object_id: id });
        l
    };
    quiet_about(&s, P0, &playing(mine), &reg, "(CR 305.1)");
    flags(&s, P0, &playing(theirs), &reg, "(CR 305.1)");
    flags(&s, P0, &playing(played), &reg, "(CR 305.1)");

    // CR 305.9: a land is played, not cast.
    let mut s = state.clone();
    let forest = spell_in_hand(&mut s, &reg, "Forest", P0);
    let mut l = legal.clone();
    l.actions.insert(1, cast_action(forest, vec![]));
    flags(&s, P0, &l, &reg, "is a land (CR 305.9)");

    // CR 601.3a: from the graveyard only with permission.
    let mut s = state.clone();
    let buried = named_card_in_graveyard(&mut s, &reg, "Moment of Heroism", P0);
    let mut l = legal.clone();
    l.actions.insert(1, cast_action(buried, vec![Target::Object(bear)]));
    flags(&s, P0, &l, &reg, "from the graveyard with no permission (CR 601.3a)");

    // Nevermore's ban is part of what may be offered.
    let mut s = state.clone();
    let nevermore = named_permanent(&mut s, &reg, "Nevermore", P0);
    s.get_object_mut(nevermore).unwrap().instance_continuous_effects = Some(vec![
        ContinuousEffect::PreventCastingNamed { name: "Moment of Heroism".into() },
    ]);
    flags(&s, P0, &legal, &reg, "is offered while casting it is forbidden");

    // CR 307.1: sorcery speed is for sorceries. An instant is offered
    // outside the main phase and off an empty stack, and so is a permanent
    // spell with flash — being either one is enough.
    let mut s = state.clone();
    s.step = Step::DeclareBlockers;
    let bolt = castable_spell(&mut s, &reg, "Brimstone Volley", P0);
    s.priority_player = Some(P0);
    let mut l = wrong_legal(&s, &reg);
    l.actions.insert(1, cast_action(bolt, vec![Target::Player(P1)]));
    quiet_about(&s, P0, &l, &reg, "(CR 307.1)");
    let creature = castable_spell(&mut s, &reg, "Grizzly Bears", P0);
    let mut l = wrong_legal(&s, &reg);
    l.actions.insert(1, cast_action(creature, vec![]));
    flags(&s, P0, &l, &reg, "at sorcery speed outside p0's main phase with an empty stack (CR 307.1)");

    // CR 115.5: a spell does not target itself.
    let mut l = legal.clone();
    l.actions.insert(1, cast_action(pump, vec![Target::Object(pump)]));
    flags(&state, P0, &l, &reg, "targets itself (CR 115.5)");

    // A target in the stack zone is one the stack actually holds: the spell
    // that is on the only stack entry is a real target, and the clause is
    // about one that is on none.
    let mut s = state.clone();
    let onstack = castable_spell(&mut s, &reg, "Brimstone Volley", P0);
    s.get_object_mut(onstack).unwrap().zone = Zone::Stack;
    s.stack.push(StackEntry::Spell(onstack));
    s.priority_player = Some(P0);
    let mut l = wrong_legal(&s, &reg);
    l.actions.insert(1, cast_action(pump, vec![Target::Object(onstack)]));
    quiet_about(&s, P0, &l, &reg, "in the stack zone that is on no stack entry");
    let mut s2 = s.clone();
    s2.stack.clear();
    let mut l = wrong_legal(&s2, &reg);
    l.actions.insert(1, cast_action(pump, vec![Target::Object(onstack)]));
    flags(&s2, P0, &l, &reg, "in the stack zone that is on no stack entry");

    // A target named twice is one target offered twice.
    let mut l = legal.clone();
    l.actions.insert(1, cast_action(pump, vec![Target::Object(bear), Target::Object(bear)]));
    flags(&state, P0, &l, &reg, "twice");

    // CR 608.2b: an Illegal marker is never offered as a target.
    let mut l = legal.clone();
    l.actions.insert(1, cast_action(pump, vec![Target::Illegal]));
    flags(&state, P0, &l, &reg, "offers an Illegal target");

    // CR 701.17a: an additional sacrifice cost names a creature you control.
    let mut l = legal.clone();
    l.actions.insert(1, Action::CastSpell {
        object_id: pump, targets: vec![Target::Object(bear)], sacrifice: Some(ObjectId(4242)),
        exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] });
    flags(&state, P0, &l, &reg, "which is not a creature p0 controls (CR 701.17a)");

    // An exile cost comes out of the caster's own graveyard.
    let mut l = legal.clone();
    l.actions.insert(1, Action::CastSpell {
        object_id: pump, targets: vec![Target::Object(bear)], sacrifice: None,
        exile_count: Some(1), exile_ids: vec![bear], alternative_cost: None, tap_plan: vec![] });
    flags(&state, P0, &l, &reg, "which is not in p0's graveyard");

    // Each half of "a creature you control on the battlefield", alone: a
    // creature the opponent controls, a permanent that is not a creature,
    // and a creature card in a graveyard.
    let sacrificing = |s: &GameState, victim: ObjectId| {
        let mut l = legal.clone();
        l.actions.insert(1, Action::CastSpell {
            object_id: pump, targets: vec![Target::Object(bear)], sacrifice: Some(victim),
            exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] });
        let _ = s;
        l
    };
    let mut s = state.clone();
    let theirs = named_permanent(&mut s, &reg, "Grizzly Bears", P1);
    flags(&s, P0, &sacrificing(&s, theirs), &reg, "(CR 701.17a)");
    let mut s = state.clone();
    let land = named_permanent(&mut s, &reg, "Forest", P0);
    flags(&s, P0, &sacrificing(&s, land), &reg, "(CR 701.17a)");
    let mut s = state.clone();
    let buried_creature = named_card_in_graveyard(&mut s, &reg, "Grizzly Bears", P0);
    flags(&s, P0, &sacrificing(&s, buried_creature), &reg, "(CR 701.17a)");
    // And the one that is right is not flagged.
    quiet_about(&state, P0, &sacrificing(&state, bear), &reg, "(CR 701.17a)");

    // The same for the exile cost: the spell itself, a card in somebody
    // else's graveyard, and a card that is not in a graveyard at all.
    let exiling = |ids: Vec<ObjectId>| {
        let mut l = legal.clone();
        l.actions.insert(1, Action::CastSpell {
            object_id: pump, targets: vec![Target::Object(bear)], sacrifice: None,
            exile_count: Some(1), exile_ids: ids, alternative_cost: None, tap_plan: vec![] });
        l
    };
    let mut s = state.clone();
    let mine_gy = named_card_in_graveyard(&mut s, &reg, "Forest", P0);
    let theirs_gy = named_card_in_graveyard(&mut s, &reg, "Forest", P1);
    quiet_about(&s, P0, &exiling(vec![mine_gy]), &reg, "which is not in p0's graveyard");
    flags(&s, P0, &exiling(vec![theirs_gy]), &reg, "which is not in p0's graveyard");
    flags(&s, P0, &exiling(vec![pump]), &reg, "which is not in p0's graveyard");

    // CR 601.3a: each permission to cast from a graveyard, alone.
    let from_gy = |s: &GameState, id: ObjectId| {
        let mut l = legal.clone();
        l.actions.insert(1, cast_action(id, vec![Target::Object(bear)]));
        let _ = s;
        l
    };
    // A printed flashback cost.
    let mut s = state.clone();
    let flashback = named_card_in_graveyard(&mut s, &reg, "Silent Departure", P0);
    quiet_about(&s, P0, &from_gy(&s, flashback), &reg, "(CR 601.3a)");
    // A granted one (Snapcaster Mage).
    let mut s = state.clone();
    let granted = named_card_in_graveyard(&mut s, &reg, "Moment of Heroism", P0);
    s.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantFlashback {
        target: granted, cost: ManaCost::new(vec![ManaSymbol::Colored(Color::White)]) });
    quiet_about(&s, P0, &from_gy(&s, granted), &reg, "(CR 601.3a)");
}

/// CR 118.3/602.2b/606.3: the rest of what an activation offer promises —
/// counters it can actually remove, a timing restriction it respects, an
/// artifact ability Stony Silence has not shut off, no targets for an
/// ability that does not target, and loyalty the planeswalker actually has.
#[test]
fn an_activation_offer_can_pay_what_the_ability_costs() {
    let (mut state, reg) = base();
    // Mikaeus the Lunarch: "{T}, Remove a +1/+1 counter from Mikaeus: ...".
    let mikaeus = named_permanent(&mut state, &reg, "Mikaeus, the Lunarch", P0);
    state.get_object_mut(mikaeus).unwrap().summoning_sick = false;
    state.add_counters(mikaeus, CounterType::PlusOnePlusOne, 1);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let remove = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, ability_index, .. }
        if *object_id == mikaeus && *ability_index == 1))
        .expect("the counter-removal ability is offered").clone();
    clean(&state, P0, &legal, &reg);

    // The counter is gone, but the ability is still on the menu.
    let mut s = state.clone();
    s.remove_counters(mikaeus, CounterType::PlusOnePlusOne, 1);
    let mut l = legal.clone();
    l.actions.retain(|a| !matches!(a, Action::ActivateAbility { object_id, ability_index, .. }
        if *object_id == mikaeus && *ability_index == 0));
    flags(&s, P0, &l, &reg, "counters it does not have");

    // CR 602.2b: an ability with no target requirement is offered with no
    // targets.
    let mut targeted = remove.clone();
    if let Action::ActivateAbility { targets, .. } = &mut targeted {
        targets.push(Target::Player(P1));
    }
    let mut l = legal.clone();
    l.actions.insert(1, targeted);
    flags(&state, P0, &l, &reg, "carries targets for an untargeted ability");

    // Equip is sorcery-speed only (CR 702.6b): offered in a main phase with
    // an empty stack, and nowhere else.
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let flail = named_permanent(&mut state, &reg, "Inquisitor's Flail", P0);
    add_mana(&mut state, P0, &[(ManaType::Colorless, 2)]);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let equip = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == flail))
        .expect("equip is offered in a main phase").clone();
    clean(&state, P0, &legal, &reg);
    assert!(matches!(&equip, Action::ActivateAbility { targets, .. }
        if targets.contains(&Target::Object(bear))), "equip targets the creature");

    let mut s = state.clone();
    s.step = Step::DeclareBlockers;
    let mut l = legal.clone();
    l.actions.retain(|a| !matches!(a, Action::CastSpell { .. }));
    flags(&s, P0, &l, &reg, "activates only as a sorcery");

    // CR 602.2: Stony Silence shuts off an artifact's activated abilities.
    let mut s = state.clone();
    named_permanent(&mut s, &reg, "Stony Silence", P1);
    flags(&s, P0, &legal, &reg, "on an artifact under Stony Silence");

    // CR 602.2: an ability offered "through" another card is one that card
    // really grants — attached to this permanent under the acting player
    // (Blazing Torch), or granted to a copy of it (Evil Twin).
    let (mut state, reg) = base();
    let bearer = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.get_object_mut(bearer).unwrap().summoning_sick = false;
    let torch = named_permanent(&mut state, &reg, "Blazing Torch", P0);
    state.get_object_mut(torch).unwrap().attached_to = Some(bearer);
    let torch_card = state.get_object(torch).unwrap().card_id;
    let victim = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let granted = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, source_card_id: Some(cid), .. }
        if *object_id == bearer && *cid == torch_card))
        .expect("the Torch grants its bearer an ability").clone();
    clean(&state, P0, &legal, &reg);

    // The Torch attached elsewhere, or under the opponent, or gone: the
    // offer is through a card that grants this permanent nothing.
    for wrong in [
        {
            let mut s = state.clone();
            s.get_object_mut(torch).unwrap().attached_to = Some(victim);
            s
        },
        {
            let mut s = state.clone();
            s.get_object_mut(torch).unwrap().controller = P1;
            s
        },
        {
            let mut s = state.clone();
            s.get_object_mut(torch).unwrap().zone = Zone::Graveyard;
            s
        },
    ] {
        let mut l = wrong_legal(&wrong, &reg);
        l.actions.insert(1, granted.clone());
        flags(&wrong, P0, &l, &reg, "which neither grants it as a copy nor is attached under p0");
    }

    // A card that is nowhere near this permanent.
    let mut named = granted.clone();
    if let Action::ActivateAbility { source_card_id, .. } = &mut named {
        *source_card_id = Some(state.get_object(victim).unwrap().card_id);
    }
    let mut l = legal.clone();
    l.actions.insert(1, named);
    flags(&state, P0, &l, &reg, "which neither grants it as a copy nor is attached under p0");

    // The other way a card grants an ability to a permanent that is not
    // printed with it: a copy that keeps the copier's own abilities (CR
    // 706.2, Evil Twin). The grantor has to be the card this permanent is a
    // copy of AND a card that grants its abilities to its copies.
    let (mut copies, reg) = base();
    let bear = named_permanent(&mut copies, &reg, "Grizzly Bears", P1);
    let twin = enters_as_copy_of(&mut copies, &reg, "Evil Twin", P0, Some(bear));
    copies.get_object_mut(twin).unwrap().zone = Zone::Battlefield;
    add_mana(&mut copies, P0, &[(ManaType::Blue, 1), (ManaType::Black, 1)]);
    copies.priority_player = Some(P0);
    let evil_twin_card = reg.get_id_by_name("Evil Twin").unwrap();
    assert_eq!(copies.get_object(twin).unwrap().copy_grantor, Some(evil_twin_card),
        "precondition: the copy remembers what printed it");
    let through = |cid: mtg_engine::ids::CardId| {
        let mut l = wrong_legal(&copies, &reg);
        l.actions.insert(1, Action::ActivateAbility {
            object_id: twin, ability_index: 0, targets: vec![Target::Object(bear)],
            tap_plan: vec![], sacrifice: None, x_value: None, source_card_id: Some(cid) });
        l
    };
    quiet_about(&copies, P0, &through(evil_twin_card), &reg, "neither grants it as a copy");
    let grizzly = copies.get_object(bear).unwrap().card_id;
    flags(&copies, P0, &through(grizzly), &reg,
        "which neither grants it as a copy nor is attached under p0");

    // Being the card a permanent is printed as is not enough on its own:
    // under Essence of the Wild a Grizzly Bears enters as a copy of the
    // Essence, so the Bears is what it is printed as and grants a copy
    // nothing (CR 706.2).
    let (mut wild, reg) = base();
    let essence = named_permanent(&mut wild, &reg, "Essence of the Wild", P0);
    let shaped = enters_as_copy_of(&mut wild, &reg, "Grizzly Bears", P0, Some(essence));
    wild.get_object_mut(shaped).unwrap().zone = Zone::Battlefield;
    wild.priority_player = Some(P0);
    let printed = wild.get_object(shaped).unwrap().copy_grantor.expect("printed as a Bears");
    let mut l = wrong_legal(&wild, &reg);
    l.actions.insert(1, Action::ActivateAbility {
        object_id: shaped, ability_index: 0, targets: vec![], tap_plan: vec![],
        sacrifice: None, x_value: None, source_card_id: Some(printed) });
    flags(&wild, P0, &l, &reg, "which neither grants it as a copy nor is attached under p0");

    // CR 605.3a: a mana ability offer names a permanent of the acting
    // player's, on the battlefield, with that ability — each half alone.
    let (mut state, reg) = base();
    let forest = named_permanent(&mut state, &reg, "Forest", P0);
    let theirs = named_permanent(&mut state, &reg, "Forest", P1);
    let in_hand = spell_in_hand(&mut state, &reg, "Forest", P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let mana_from = |id: ObjectId, idx: usize| {
        let mut l = legal.clone();
        l.actions.insert(1, Action::ActivateManaAbility { object_id: id, ability_index: idx });
        l
    };
    quiet_about(&state, P0, &mana_from(forest, 0), &reg, "(CR 605.3a)");
    flags(&state, P0, &mana_from(theirs, 0), &reg, "(CR 605.3a)");
    flags(&state, P0, &mana_from(in_hand, 0), &reg, "(CR 605.3a)");
    flags(&state, P0, &mana_from(bear, 0), &reg, "(CR 605.3a)");
    flags(&state, P0, &mana_from(forest, 7), &reg, "(CR 605.3a)");
    // A tapped land has no mana ability available, and is not offered.
    let mut s = state.clone();
    s.get_object_mut(forest).unwrap().tapped = true;
    flags(&s, P0, &mana_from(forest, 0), &reg, "(CR 605.3a)");

    // CR 701.17a: an activation's sacrifice cost names a creature its
    // controller has on the battlefield — and "another creature" means
    // another one.
    let (mut state, reg) = base();
    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    state.get_object_mut(grimgrin).unwrap().summoning_sick = false;
    state.get_object_mut(grimgrin).unwrap().tapped = true;
    let fodder = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let land = named_permanent(&mut state, &reg, "Forest", P0);
    let buried = named_card_in_graveyard(&mut state, &reg, "Grizzly Bears", P0);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let sacrificing = |victim: ObjectId| {
        let mut l = legal.clone();
        l.actions.insert(1, Action::ActivateAbility {
            object_id: grimgrin, ability_index: 0, targets: vec![], tap_plan: vec![],
            sacrifice: Some(victim), x_value: None, source_card_id: None });
        l
    };
    quiet_about(&state, P0, &sacrificing(fodder), &reg, "(CR 701.17a)");
    flags(&state, P0, &sacrificing(theirs), &reg, "(CR 701.17a)");
    flags(&state, P0, &sacrificing(land), &reg, "(CR 701.17a)");
    flags(&state, P0, &sacrificing(buried), &reg, "(CR 701.17a)");
    // "Sacrifice another creature": not this one.
    flags(&state, P0, &sacrificing(grimgrin), &reg, "(CR 701.17a)");
    // And naming none at all when the cost asks for one.
    let mut l = legal.clone();
    l.actions.insert(1, Action::ActivateAbility {
        object_id: grimgrin, ability_index: 0, targets: vec![], tap_plan: vec![],
        sacrifice: None, x_value: None, source_card_id: None });
    flags(&state, P0, &l, &reg, "names no creature to sacrifice (CR 701.17a)");

    // CR 602.2h: a tap plan taps the caster's own battlefield permanents,
    // each for a mana ability it really has.
    let (mut state, reg) = base();
    let forest = named_permanent(&mut state, &reg, "Forest", P0);
    let theirs = named_permanent(&mut state, &reg, "Forest", P1);
    let in_hand = spell_in_hand(&mut state, &reg, "Forest", P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let pump = castable_spell(&mut state, &reg, "Moment of Heroism", P0);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let tapping = |plan: Vec<(ObjectId, usize)>| {
        let mut l = legal.clone();
        l.actions.insert(1, Action::CastSpell {
            object_id: pump, targets: vec![Target::Object(bear)], sacrifice: None,
            exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: plan });
        l
    };
    quiet_about(&state, P0, &tapping(vec![(forest, 0)]), &reg, "which is not an available untapped source");
    quiet_about(&state, P0, &tapping(vec![(forest, 0)]), &reg, "twice (CR 602.2h)");
    quiet_about(&state, P0, &tapping(vec![(forest, 0)]), &reg, "under Stony Silence");
    flags(&state, P0, &tapping(vec![(forest, 0), (forest, 0)]), &reg, "twice (CR 602.2h)");
    flags(&state, P0, &tapping(vec![(theirs, 0)]), &reg, "which is not an available untapped source");
    flags(&state, P0, &tapping(vec![(in_hand, 0)]), &reg, "which is not an available untapped source");
    flags(&state, P0, &tapping(vec![(forest, 7)]), &reg, "which is not an available untapped source");
    flags(&state, P0, &tapping(vec![(bear, 0)]), &reg, "which is not an available untapped source");

    // CR 118.3: a minus ability the planeswalker cannot pay for.
    let (mut state, reg) = base();
    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 3);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    assert!(legal.actions.iter().any(|a| matches!(a, Action::ActivateLoyaltyAbility { .. })),
        "a planeswalker offers its loyalty abilities");
    clean(&state, P0, &legal, &reg);

    let mut s = state.clone();
    set_loyalty(&mut s, liliana, 1);
    flags(&s, P0, &legal, &reg, "(CR 118.3)");

    // CR 118.3 lets a walker pay its loyalty down to exactly zero, so the
    // ability that costs everything it has is still a legal offer.
    let cost = legal.actions.iter().find_map(|a| match a {
        Action::ActivateLoyaltyAbility { object_id, ability_index, .. } if *object_id == liliana =>
            reg.get(s.get_object(liliana).unwrap().card_id)
                .and_then(|b| b.loyalty_abilities(&s, liliana).into_iter()
                    .find(|d| d.ability_index == *ability_index)
                    .filter(|d| d.loyalty_change < 0)
                    .map(|d| d.loyalty_change.unsigned_abs())),
        _ => None,
    }).expect("Liliana offers a minus ability");
    let mut s = state.clone();
    set_loyalty(&mut s, liliana, cost);
    quiet_about(&s, P0, &legal, &reg, "(CR 118.3)");
}

/// CR 602.2/602.5/701.17a: an activation offer names an ability its source
/// actually has, whose costs its controller can actually pay.
#[test]
fn an_activation_offer_names_an_ability_its_source_has() {
    let (mut state, reg) = base();
    let priest = named_permanent(&mut state, &reg, "Avacynian Priest", P0);
    state.get_object_mut(priest).unwrap().summoning_sick = false;
    let victim = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    add_mana(&mut state, P0, &[(ManaType::White, 1)]);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let activation = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == priest))
        .expect("the Priest's tap ability is offered").clone();
    clean(&state, P0, &legal, &reg);

    let with = |f: &dyn Fn(&mut Action)| {
        let mut a = activation.clone();
        f(&mut a);
        let mut l = legal.clone();
        l.actions.insert(1, a);
        l
    };

    // An ability the card does not have.
    let l = with(&|a| if let Action::ActivateAbility { ability_index, .. } = a { *ability_index = 99 });
    flags(&state, P0, &l, &reg, "which card");

    // CR 302.6: a {T} ability needs a permanent that can tap.
    let mut s = state.clone();
    s.get_object_mut(priest).unwrap().tapped = true;
    flags(&s, P0, &legal, &reg, "needs {T} but the permanent cannot tap (CR 302.6)");

    // CR 602.5: once per turn means once — the Priest's is not one, and is
    // still offered after being used.
    let mut s = state.clone();
    s.get_object_mut(priest).unwrap().abilities_activated_this_turn.insert(0);
    quiet_about(&s, P0, &legal, &reg, "is once per turn and already used (CR 602.5)");

    // An ability with a target requirement takes exactly one target.
    let l = with(&|a| if let Action::ActivateAbility { targets, .. } = a {
        *targets = vec![Target::Object(victim), Target::Object(victim)] });
    flags(&state, P0, &l, &reg, "targets for one requirement");

    // CR 601.2b: X is funded through the prompt, not announced in the offer.
    let l = with(&|a| if let Action::ActivateAbility { x_value, .. } = a { *x_value = Some(2) });
    flags(&state, P0, &l, &reg, "announces X before funding");

    // CR 602.2: the source is on the battlefield AND the acting player
    // controls it. Either one alone is the violation.
    let mut s = state.clone();
    s.get_object_mut(priest).unwrap().controller = P1;
    flags(&s, P0, &legal, &reg, "controlled by p1 offered to p0 (CR 602.2)");
    let mut s = state.clone();
    s.get_object_mut(priest).unwrap().zone = Zone::Graveyard;
    flags(&s, P0, &legal, &reg, "offered to p0 (CR 602.2)");
}

/// CR 606.3/118.3: a loyalty offer is a planeswalker you control, at
/// sorcery speed, once a turn, for loyalty you have.
#[test]
fn a_loyalty_offer_costs_loyalty_the_planeswalker_has() {
    let (mut state, reg) = base();
    let lili = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, lili, 3);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    clean(&state, P0, &legal, &reg);

    // CR 118.3: a minus ability costs loyalty the permanent has.
    let mut l = legal.clone();
    l.actions.insert(1, Action::ActivateLoyaltyAbility {
        object_id: lili, ability_index: 2, targets: vec![Target::Player(P1)] });
    flags(&state, P0, &l, &reg, "costs 6 loyalty of 3 (CR 118.3)");

    // CR 606.3: one loyalty ability per turn, at sorcery speed, on a
    // planeswalker you control.
    let mut s = state.clone();
    s.get_object_mut(lili).unwrap().abilities_activated_this_turn.insert(999);
    flags(&s, P0, &legal, &reg, "after a loyalty ability was used this turn (CR 606.3)");

    let mut s = state.clone();
    s.step = Step::BeginCombat;
    flags(&s, P0, &legal, &reg, "outside p0's main phase with an empty stack (CR 606.3)");

    let mut s = state.clone();
    s.get_object_mut(lili).unwrap().controller = P1;
    flags(&s, P0, &legal, &reg, "is not p0's planeswalker on the battlefield (CR 606.3)");

    // An ability the card does not have.
    let mut l = legal.clone();
    l.actions.insert(1, Action::ActivateLoyaltyAbility {
        object_id: lili, ability_index: 42, targets: vec![] });
    flags(&state, P0, &l, &reg, "which the card does not have");
}

/// CR 602.2h/605.3a: a tap plan taps real, own sources, once each.
#[test]
fn a_tap_plan_taps_each_source_once() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let forest = named_permanent(&mut state, &reg, "Forest", P0);
    let pump = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);

    let with_plan = |plan: Vec<(ObjectId, usize)>| {
        let mut l = legal.clone();
        l.actions.insert(1, Action::CastSpell {
            object_id: pump, targets: vec![Target::Object(bear)], sacrifice: None,
            exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: plan });
        l
    };

    flags(&state, P0, &with_plan(vec![(forest, 0), (forest, 0)]), &reg,
        "tap plan taps #2 twice (CR 602.2h)");
    flags(&state, P0, &with_plan(vec![(bear, 0)]), &reg,
        "which is not an available untapped source of p0");
    let mut s = state.clone();
    s.get_object_mut(forest).unwrap().controller = P1;
    flags(&s, P0, &with_plan(vec![(forest, 0)]), &reg,
        "which is not an available untapped source of p0");
}

/// CR 514.1/103.5: a "choose N of your hand" prompt offers every subset of
/// that size, once each — all C(n, k) of them.
#[test]
fn a_choose_n_of_your_hand_prompt_offers_the_hand_and_the_count() {
    let (mut state, reg) = base();
    let hand: Vec<ObjectId> = (0..5)
        .map(|_| spell_in_hand(&mut state, &reg, "Moment of Heroism", P0))
        .collect();
    state.priority_player = None;
    state.step = Step::Cleanup;
    state.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P0, discard_count: 2 });
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    // The offer is the hand and the number, not one row per subset: the
    // subsets are C(5, 2) here and C(7, 3) at a real bottoming, which is a
    // menu read as a combination lock (issue #360).
    assert!(legal.actions.is_empty(), "no enumerated subsets: {:?}", legal.actions);
    let prompt = legal.set_prompt.clone().expect("a set prompt");
    assert_eq!(prompt.options, hand, "the whole hand, in hand order");
    assert_eq!((prompt.min, prompt.max), (2, 2));
    clean(&state, P0, &legal, &reg);

    // A prompt that leaves a card out, or offers one twice.
    let mut l = legal.clone();
    l.set_prompt.as_mut().unwrap().options.pop();
    flags(&state, P0, &l, &reg, "not the 5 cards of the hand (CR 514.1)");
    let mut l = legal.clone();
    l.set_prompt.as_mut().unwrap().options[1] = hand[0];
    flags(&state, P0, &l, &reg, "discard prompt lists #");

    // A prompt that asks for the wrong number.
    for (min, max) in [(1usize, 1usize), (2, 3), (3, 3)] {
        let mut l = legal.clone();
        l.set_prompt.as_mut().unwrap().min = min;
        l.set_prompt.as_mut().unwrap().max = max;
        flags(&state, P0, &l, &reg, "not 2 (CR 514.1)");
    }

    // No prompt at all, and a menu smuggled in beside it.
    let mut l = legal.clone();
    l.set_prompt = None;
    flags(&state, P0, &l, &reg, "discard prompt with nothing to choose");
    let mut l = legal.clone();
    l.actions.push(Action::DiscardCards { cards: vec![hand[0], hand[1]] });
    flags(&state, P0, &l, &reg, "discard prompt with 1 flat actions");

    // The same rule for the bottoming prompt after a mulligan (CR 103.5),
    // and the two prompts are told apart.
    let mut s = state.clone();
    s.awaiting_action = Some(AwaitingAction::BottomAfterMulligan { player: P0, count: 2 });
    s.step = Step::PrecombatMain;
    let legal = mtg_engine::engine::legal_actions(&s, &reg);
    assert!(legal.actions.is_empty(), "no enumerated subsets: {:?}", legal.actions);
    assert_eq!(legal.set_prompt.as_ref().map(|p| p.kind),
        Some(mtg_engine::actions::SetPromptKind::BottomAfterMulligan));
    clean(&s, P0, &legal, &reg);
    let mut l = legal.clone();
    l.set_prompt.as_mut().unwrap().options.pop();
    flags(&s, P0, &l, &reg, "not the 5 cards of the hand (CR 103.5)");
    let mut l = legal.clone();
    l.set_prompt.as_mut().unwrap().kind = mtg_engine::actions::SetPromptKind::DiscardToHandSize;
    flags(&s, P0, &l, &reg, "carries a DiscardToHandSize set prompt");

    // And a set prompt attached to a question that does not ask for a set.
    let mut s = state.clone();
    s.step = Step::PrecombatMain;
    s.awaiting_action = None;
    s.priority_player = Some(P0);
    let mut l = mtg_engine::engine::legal_actions(&s, &reg);
    clean(&s, P0, &l, &reg);
    l.set_prompt = legal.set_prompt.clone();
    flags(&s, P0, &l, &reg, "priority offer with a BottomAfterMulligan set prompt attached");
}

/// The prompt asks for as many cards as the player has, when they have
/// fewer than the question names: a hand of two discarding three discards
/// both (CR 514.1).
#[test]
fn a_set_prompt_never_asks_for_more_cards_than_the_hand_holds() {
    let (mut state, reg) = base();
    let hand: Vec<ObjectId> = (0..2)
        .map(|_| spell_in_hand(&mut state, &reg, "Moment of Heroism", P0))
        .collect();
    state.priority_player = None;
    state.step = Step::Cleanup;
    state.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P0, discard_count: 3 });
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let prompt = legal.set_prompt.clone().expect("a set prompt");
    assert_eq!((prompt.min, prompt.max), (2, 2), "as many as there are");
    assert_eq!(prompt.options, hand);
    clean(&state, P0, &legal, &reg);
}

/// The answer a set prompt accepts is the one it says it accepts: the
/// right number of cards, all from its own list, none of them twice.
#[test]
fn a_set_prompt_accepts_exactly_the_answers_it_describes() {
    use mtg_engine::actions::{SetPrompt, SetPromptKind};
    let ids: Vec<ObjectId> = (1..=4).map(ObjectId).collect();
    let prompt = SetPrompt {
        kind: SetPromptKind::BottomAfterMulligan, player: P0,
        options: ids.clone(), min: 2, max: 2,
    };
    assert!(prompt.accepts(&[ids[0], ids[3]]));
    assert!(prompt.accepts(&[ids[3], ids[0]]), "order is not part of the answer");
    assert!(!prompt.accepts(&[ids[0]]), "too few");
    assert!(!prompt.accepts(&[ids[0], ids[1], ids[2]]), "too many");
    assert!(!prompt.accepts(&[ids[0], ids[0]]), "the same card twice");
    assert!(!prompt.accepts(&[ids[0], ObjectId(99)]), "a card that is not on the list");
    assert!(matches!(prompt.answer(vec![ids[0], ids[1]]),
        Action::BottomCards { ref cards } if *cards == vec![ids[0], ids[1]]));

    let discard = SetPrompt { kind: SetPromptKind::DiscardToHandSize, min: 0, max: 2, ..prompt };
    assert!(discard.accepts(&[]), "a range that starts at zero takes none");
    assert!(matches!(discard.answer(vec![ids[2]]),
        Action::DiscardCards { ref cards } if *cards == vec![ids[2]]));
}

/// CR 608.2: a resolution prompt's answers are the enumeration of what the
/// prompt asks — one answer per option, in order, plus the ones the rules
/// always add.
#[test]
fn every_resolution_prompt_enumerates_to_its_own_options() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let other = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let card = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);
    let card2 = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);
    state.priority_player = Some(P0);

    let kinds = [
        ResolutionChoiceKind::YesNo { description: "?".into(), source_card: bear },
        ResolutionChoiceKind::ChooseCardFromHand {
            description: "?".into(), player: P0, cards: vec![card, card2],
            discard_immediately: true, remaining: 1 },
        ResolutionChoiceKind::ChooseFromLookedAt {
            description: "?".into(), looked_at: vec![card, card2] },
        ResolutionChoiceKind::ChooseCardType {
            description: "?".into(), options: vec!["Creature".into(), "Land".into()],
            controller: P0 },
        ResolutionChoiceKind::ChooseTriggerOrder {
            description: "?".into(), options: vec!["a".into(), "b".into()],
            ap_queue: true, indices: vec![0, 1], details: vec![] },
    ];

    for choice in kinds {
        let mut s = state.clone();
        s.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: P0, source: bear, choice });
        let legal = mtg_engine::engine::legal_actions(&s, &reg);
        clean(&s, P0, &legal, &reg);

        let mut l = legal.clone();
        l.actions.pop();
        flags(&s, P0, &l, &reg, "but the prompt enumerates to");

        let mut l = legal.clone();
        l.actions.swap(0, 1);
        flags(&s, P0, &l, &reg, "(CR 608.2)");
    }

    // CR 701.19b: a library search always offers taking none of them.
    let mut s = state.clone();
    let library = stock_library(&mut s, &reg, P0, 2);
    for id in &library {
        s.get_object_mut(*id).unwrap().name = "Forest".into();
    }
    s.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: P0, source: bear,
        choice: ResolutionChoiceKind::ChooseFromLibrary {
            description: "?".into(), options: library.clone(), searcher: P0,
            source_id: bear, destination: Zone::Hand, tapped: false } });
    let legal = mtg_engine::engine::legal_actions(&s, &reg);
    clean(&s, P0, &legal, &reg);
    let mut l = legal.clone();
    l.actions.retain(|a| !matches!(a, Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(None) }));
    flags(&s, P0, &l, &reg, "but the prompt enumerates to");

    // A structured prompt is answered off the prompt, not off a flat menu.
    let mut s = state.clone();
    s.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: P0, source: bear,
        choice: ResolutionChoiceKind::DividePermanentsIntoPiles {
            description: "?".into(), permanents: vec![bear, other],
            target_player: P1, source_id: bear } });
    let legal = mtg_engine::engine::legal_actions(&s, &reg);
    clean(&s, P0, &legal, &reg);
    let mut l = legal.clone();
    l.actions.push(Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(0, "x".into()) });
    flags(&s, P0, &l, &reg, "a structured prompt offers 1 flat answers");
}

/// The cheap catch-all: every object an offer names exists.
#[test]
fn an_offer_never_names_an_object_that_does_not_exist() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);

    let mut l = legal.clone();
    l.actions.insert(1, Action::ActivateLoyaltyAbility {
        object_id: bear, ability_index: 0, targets: vec![Target::Object(ObjectId(4242))] });
    flags(&state, P0, &l, &reg, "an offer names #4242 which does not exist");

    // Every kind of offer that names an object is read, not just the one
    // this test started with: each arm of the sweep is its own way for a
    // ghost id to reach a player's menu.
    let ghost = ObjectId(4242);
    let cases: Vec<(&str, Action)> = vec![
        ("a land", Action::PlayLand { object_id: ghost }),
        ("a mana ability", Action::ActivateManaAbility { object_id: ghost, ability_index: 0 }),
        ("a cast", Action::CastSpell {
            object_id: bear, targets: vec![Target::Object(ghost)], sacrifice: None,
            exile_count: None, exile_ids: vec![], tap_plan: vec![], alternative_cost: None }),
        ("an activation", Action::ActivateAbility {
            object_id: bear, ability_index: 0, targets: vec![], tap_plan: vec![(ghost, 0)],
            sacrifice: None, x_value: None, source_card_id: None }),
        ("a discard", Action::DiscardCards { cards: vec![ghost] }),
        ("a bottoming", Action::BottomCards { cards: vec![ghost] }),
    ];
    for (what, action) in cases {
        let mut l = legal.clone();
        l.actions.insert(1, action);
        let v = check_legal(&state, P0, &l, &reg);
        assert!(v.iter().any(|m| m.contains("an offer names #4242 which does not exist")),
            "{what} naming an object that is not there is caught: {v:?}");
    }

    // CR 104.1: a finished game is not offering anything, so nothing about
    // the list it last held is a violation.
    let mut over = state.clone();
    over.result = Some(mtg_engine::state::GameResult::Winner(P0));
    let mut l = legal.clone();
    l.actions.insert(1, Action::PlayLand { object_id: ghost });
    assert_eq!(check_legal(&over, P0, &l, &reg), Vec::<String>::new(),
        "a game with a result is past being offered anything");
}

/// CR 508.1a/508.1d/506.2: the attackers prompt is the board's own answer
/// to "who can attack, who must, and what can be attacked".
#[test]
fn the_attackers_prompt_is_the_board_read_back() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let lili = named_permanent(&mut state, &reg, "Liliana of the Veil", P1);
    state.step = Step::DeclareAttackers;
    state.awaiting_action = Some(AwaitingAction::DeclareAttackers);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    clean(&state, P0, &legal, &reg);

    let with = |f: &dyn Fn(&mut Vec<ObjectId>, &mut Vec<ObjectId>, &mut Vec<ObjectId>, &mut PlayerId)| {
        let mut l = legal.clone();
        if let Some(CombatPrompt::ChooseAttackers {
            eligible, must_attack, defending_player, defending_planeswalkers }) = &mut l.combat_prompt {
            f(eligible, must_attack, defending_planeswalkers, defending_player);
        }
        l
    };

    // CR 506.2: the defender is the non-active player.
    flags(&state, P0, &with(&|_, _, _, d| *d = P0), &reg, "names p0 as defender (CR 506.2)");

    // CR 508.1d: nothing is forced that the requirements do not force.
    flags(&state, P0, &with(&|_, m, _, _| m.push(bear)), &reg, "(CR 508.1d)");

    // The defender's planeswalkers are the ones on their battlefield.
    flags(&state, P0, &with(&|_, _, w, _| w.clear()), &reg, "offers planeswalkers");
    let mut s = state.clone();
    s.get_object_mut(lili).unwrap().controller = P0;
    flags(&s, P0, &legal, &reg, "offers planeswalkers");

    // A menu, not a multiset.
    flags(&state, P0, &with(&|e, _, _, _| e.push(bear)), &reg, "attackers prompt lists #");
}

/// CR 509.1a/509.1b/702.111: the blockers prompt is the board's own answer
/// to "who can block, and what may each of them block".
#[test]
fn the_blockers_prompt_is_the_board_read_back() {
    let (mut state, reg) = base();
    let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let blocker = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.step = Step::DeclareAttackers;
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    state.step = Step::DeclareBlockers;
    state.awaiting_action = Some(AwaitingAction::DeclareBlockers { defending_player: P1 });
    state.priority_player = Some(P1);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    clean(&state, P1, &legal, &reg);

    // The attacker list is combat's own.
    let mut l = legal.clone();
    if let Some(CombatPrompt::ChooseBlockers { attackers, .. }) = &mut l.combat_prompt {
        attackers.clear();
    }
    flags(&state, P1, &l, &reg, "lists attackers {} but combat has");

    // The eligible blockers are the creatures that can block.
    let mut s = state.clone();
    s.get_object_mut(blocker).unwrap().tapped = true;
    flags(&s, P1, &legal, &reg, "but the creatures able to block are");

    // There is one block list per eligible blocker.
    let mut l = legal.clone();
    if let Some(CombatPrompt::ChooseBlockers { legal_blocks, .. }) = &mut l.combat_prompt {
        legal_blocks.clear();
    }
    flags(&state, P1, &l, &reg, "has block lists for");

    // A block list only names creatures that are attacking.
    let mut l = legal.clone();
    if let Some(CombatPrompt::ChooseBlockers { legal_blocks, .. }) = &mut l.combat_prompt {
        legal_blocks.entry(blocker).or_default().push(ObjectId(4242));
    }
    flags(&state, P1, &l, &reg, "which is not attacking");

    // CR 509.1a: a block with nothing in the way is a block that is allowed.
    let mut l = legal.clone();
    if let Some(CombatPrompt::ChooseBlockers { legal_blocks, .. }) = &mut l.combat_prompt {
        legal_blocks.insert(blocker, vec![]);
    }
    flags(&state, P1, &l, &reg, "with nothing in the way (CR 509.1a)");

    // CR 509.1b: each way an attacker evades a blocker, and for each the
    // blocker that answers it — a chain of evasions is only tested by
    // walking every link of it.
    let evasion_offers = |s: &GameState, reg: &CardRegistry| {
        let mut l = mtg_engine::engine::legal_actions(s, reg);
        if let Some(CombatPrompt::ChooseBlockers { legal_blocks, .. }) = &mut l.combat_prompt {
            for (_, list) in legal_blocks.iter_mut() {
                if !list.contains(&attacker) { list.push(attacker); }
            }
        }
        l
    };

    // Flying: a ground blocker is refused; flying or reach may block.
    let mut s = state.clone();
    grant_keyword(&mut s, attacker, Keyword::Flying);
    flags(&s, P1, &evasion_offers(&s, &reg), &reg, "which evades it (CR 509.1b)");
    let mut with_reach = s.clone();
    grant_keyword(&mut with_reach, blocker, Keyword::Reach);
    quiet_about(&with_reach, P1, &evasion_offers(&with_reach, &reg), &reg, "which evades it");
    let mut with_flying = s.clone();
    grant_keyword(&mut with_flying, blocker, Keyword::Flying);
    quiet_about(&with_flying, P1, &evasion_offers(&with_flying, &reg), &reg, "which evades it");

    // Intimidate: only an artifact creature or one sharing a color. The
    // attacker is white and the blocker green, so they share none.
    let mut s = state.clone();
    // A white ground creature: Chapel Geist would evade by flying instead,
    // which would not tell the two clauses apart.
    let ghost = named_permanent(&mut s, &reg, "Doomed Traveler", P0);
    grant_keyword(&mut s, ghost, Keyword::Intimidate);
    if let Some(c) = s.combat.as_mut() {
        c.attackers.insert(ghost, P1);
        c.blocker_assignments.insert(ghost, vec![]);
    }
    let intimidate_offers = |s: &GameState, reg: &CardRegistry| {
        let mut l = mtg_engine::engine::legal_actions(s, reg);
        if let Some(CombatPrompt::ChooseBlockers { legal_blocks, .. }) = &mut l.combat_prompt {
            for (_, list) in legal_blocks.iter_mut() {
                if !list.contains(&ghost) { list.push(ghost); }
            }
        }
        l
    };
    assert!(s.colors_of(ghost, &reg).iter().all(|c| !s.colors_of(blocker, &reg).contains(c)),
        "test setup: the Traveler and the Bears share no color");
    assert!(!s.has_keyword(ghost, Keyword::Flying, &reg), "test setup: it evades by intimidate alone");
    flags(&s, P1, &intimidate_offers(&s, &reg), &reg, "which evades it (CR 509.1b)");
    let mut sharing = s.clone();
    sharing.get_object_mut(blocker).unwrap().colors = sharing.colors_of(ghost, &reg);
    quiet_about(&sharing, P1, &intimidate_offers(&sharing, &reg), &reg, "which evades it");
    let mut artifact = s.clone();
    artifact.get_object_mut(blocker).unwrap().card_types.push(CardType::Artifact);
    quiet_about(&artifact, P1, &intimidate_offers(&artifact, &reg), &reg, "which evades it");

    // Protection from the blocker, and "can't be blocked" outright.
    let mut s = state.clone();
    s.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantProtection {
        target: attacker,
        filter: CreatureFilter::HasCardType(CardType::Creature),
    });
    flags(&s, P1, &evasion_offers(&s, &reg), &reg, "which evades it (CR 509.1b)");

    let mut s = state.clone();
    let stalker = named_permanent(&mut s, &reg, "Invisible Stalker", P0);
    if let Some(c) = s.combat.as_mut() {
        c.attackers.insert(stalker, P1);
        c.blocker_assignments.insert(stalker, vec![]);
    }
    let mut l = mtg_engine::engine::legal_actions(&s, &reg);
    if let Some(CombatPrompt::ChooseBlockers { legal_blocks, .. }) = &mut l.combat_prompt {
        for (_, list) in legal_blocks.iter_mut() { list.push(stalker); }
    }
    flags(&s, P1, &l, &reg, "which evades it (CR 509.1b)");

    // CR 702.111: menace asks for two, and the prompt says so.
    let mut s = state.clone();
    grant_keyword(&mut s, attacker, Keyword::Menace);
    let mut l = mtg_engine::engine::legal_actions(&s, &reg);
    if let Some(CombatPrompt::ChooseBlockers { min_blockers, .. }) = &mut l.combat_prompt {
        min_blockers.clear();
    }
    flags(&s, P1, &l, &reg, "which has menace (CR 702.111)");

    // Two is the number menace asks for, and asking for it is not itself a
    // violation: the clause is about a minimum of one or none.
    let mut s = state.clone();
    grant_keyword(&mut s, attacker, Keyword::Menace);
    let with_two = mtg_engine::engine::legal_actions(&s, &reg);
    assert!(matches!(&with_two.combat_prompt,
        Some(CombatPrompt::ChooseBlockers { min_blockers, .. }) if min_blockers.get(&attacker) == Some(&2)),
        "the prompt asks for two blockers");
    quiet_about(&s, P1, &with_two, &reg, "requires 2 blockers");
    quiet_about(&s, P1, &with_two, &reg, "(CR 702.111)");

    // And a minimum is only asked for an attacker that is attacking, and is
    // only ever more than one.
    let mut l = legal.clone();
    if let Some(CombatPrompt::ChooseBlockers { min_blockers, .. }) = &mut l.combat_prompt {
        min_blockers.insert(attacker, 1);
    }
    flags(&state, P1, &l, &reg, "requires 1 blockers for #");
}

/// Stony Silence turns off artifacts' activated abilities (CR 613.1),
/// mana abilities and tap plans included — an offer that ignores it is an
/// offer of a game the rules do not allow.
#[test]
fn nothing_an_artifact_could_do_is_offered_under_stony_silence() {
    let (mut state, reg) = base();
    let ring = named_permanent(&mut state, &reg, "Sol Ring", P0);
    state.get_object_mut(ring).unwrap().summoning_sick = false;
    let pump = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    assert!(legal.actions.iter().any(|a| matches!(a,
        Action::ActivateManaAbility { object_id, .. } if *object_id == ring)),
        "precondition: the ring's mana ability is offered without the lock");

    // Now the lock is down, and the same menu is illegal.
    let mut s = state.clone();
    named_permanent(&mut s, &reg, "Stony Silence", P0);
    assert!(s.global_effects(&reg).iter().any(|e|
        matches!(e, ContinuousEffect::PreventArtifactAbilities)),
        "precondition: the lock is in force");
    flags(&s, P0, &legal, &reg, "offered under Stony Silence");

    // A tap plan that taps the artifact is the same offer by another name.
    let mut l = legal.clone();
    l.actions.insert(1, Action::CastSpell {
        object_id: pump, targets: vec![Target::Object(bear)], sacrifice: None,
        exile_count: None, exile_ids: vec![], alternative_cost: None,
        tap_plan: vec![(ring, 0)] });
    flags(&s, P0, &l, &reg, "tap plan taps artifact #");

    // The lock is about artifacts: under it, a land still taps for mana and
    // may still be tapped by a plan.
    let forest = named_permanent(&mut s, &reg, "Forest", P0);
    let legal_with_land = mtg_engine::engine::legal_actions(&s, &reg);
    assert!(legal_with_land.actions.iter().any(|a| matches!(a,
        Action::ActivateManaAbility { object_id, .. } if *object_id == forest)),
        "precondition: the Forest's mana ability is offered under the lock");
    quiet_about(&s, P0, &legal_with_land, &reg, "offered under Stony Silence");
    let mut l = legal_with_land.clone();
    l.actions.insert(1, Action::CastSpell {
        object_id: pump, targets: vec![Target::Object(bear)], sacrifice: None,
        exile_count: None, exile_ids: vec![], alternative_cost: None,
        tap_plan: vec![(forest, 0)] });
    quiet_about(&s, P0, &l, &reg, "under Stony Silence");
}

/// The collapsed views the interactive and LLM players act through offer
/// the same game as the flat list.
#[test]
fn the_collapsed_views_offer_the_same_game_as_the_flat_list() {
    let (mut state, reg) = base();
    let priest = named_permanent(&mut state, &reg, "Avacynian Priest", P0);
    state.get_object_mut(priest).unwrap().summoning_sick = false;
    named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    add_mana(&mut state, P0, &[(ManaType::White, 1)]);
    let pump = castable_spell(&mut state, &reg, "Moment of Heroism", P0);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    assert!(!legal.castable_spells.is_empty() && !legal.activatable_abilities.is_empty(),
        "precondition: both collapsed views have an entry");
    clean(&state, P0, &legal, &reg);

    let mut l = legal.clone();
    l.castable_spells.clear();
    flags(&state, P0, &l, &reg, "do not match the cast actions");

    let mut l = legal.clone();
    l.activatable_abilities.clear();
    flags(&state, P0, &l, &reg, "do not match the activation actions");

    let mut l = legal.clone();
    l.activatable_abilities[0].option_combos.clear();
    flags(&state, P0, &l, &reg, "option(s) for");

    // A menu, not a multiset — in the collapsed views too.
    let mut l = legal.clone();
    let dup = l.castable_spells[0].clone();
    l.castable_spells.push(dup);
    flags(&state, P0, &l, &reg, "listed twice");
    let mut l = legal.clone();
    let dup = l.activatable_abilities[0].clone();
    l.activatable_abilities.push(dup);
    flags(&state, P0, &l, &reg, "listed twice");

    // A flashback entry names a card in the graveyard.
    let mut l = legal.clone();
    l.castable_spells[0].is_flashback = true;
    flags(&state, P0, &l, &reg, "is marked flashback but is not in the graveyard");
    let _ = pump;
}

/// CR 601.2b/605.3a: an X-funding prompt offers the acting player's own
/// untapped sources, each in one group, matched to the stash that raised it.
#[test]
fn an_x_funding_offer_names_the_players_own_sources() {
    let (mut state, reg) = base();
    let play = castable_spell(&mut state, &reg, "Devil's Play", P0);
    add_mana(&mut state, P0, &[(ManaType::Red, 2)]);
    let mountain = named_permanent(&mut state, &reg, "Mountain", P0);
    let state = cast_onto_stack(&state, &reg, play, vec![Target::Player(P1)]);
    assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseXFunding { .. }, .. })),
        "precondition: a funding prompt is up");
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    clean(&state, P0, &legal, &reg);

    // A source the acting player does not control.
    let mut s = state.clone();
    s.get_object_mut(mountain).unwrap().controller = P1;
    let mut l = mtg_engine::engine::legal_actions(&state, &reg);
    if let Some(ResolutionChoiceKind::ChooseXFunding { options, .. }) = &mut l.resolution_prompt {
        if let Some(g) = options.groups.first_mut() {
            g.source_ids.push(mountain);
        }
    }
    flags(&s, P0, &l, &reg, "which is not an untapped source of p0");

    // The prompt and the stash name the same spell.
    let mut s = state.clone();
    s.pending_spell_cast.as_mut().unwrap().object_id = mountain;
    flags(&s, P0, &legal, &reg, "with no matching stash");

    // An ability's X is funded through the same prompt and answered by the
    // other stash: the ability one, naming the source that was activated
    // (Kessig Wolf Run's {X}{R}{G}).
    let (mut abil, reg) = base();
    let run = named_permanent(&mut abil, &reg, "Kessig Wolf Run", P0);
    let beast = named_permanent(&mut abil, &reg, "Grizzly Bears", P0);
    abil.get_object_mut(run).unwrap().summoning_sick = false;
    add_mana(&mut abil, P0, &[(ManaType::Red, 1), (ManaType::Green, 1)]);
    named_permanent(&mut abil, &reg, "Mountain", P0);
    named_permanent(&mut abil, &reg, "Forest", P0);
    abil.priority_player = Some(P0);
    let activate = mtg_engine::engine::legal_actions(&abil, &reg).actions.into_iter()
        .find(|a| matches!(a, Action::ActivateAbility { object_id, targets, .. }
            if *object_id == run && targets.contains(&Target::Object(beast))))
        .expect("the Wolf Run's {X}{R}{G} ability is offered");
    let abil = mtg_engine::engine::submit_action(&abil, &activate, &reg);
    assert!(matches!(&abil.awaiting_action, Some(AwaitingAction::ResolutionChoice {
        choice: ResolutionChoiceKind::ChooseXFunding { is_ability: true, .. }, .. })),
        "precondition: an ability funding prompt is up, got {:?}", abil.awaiting_action);
    let legal = mtg_engine::engine::legal_actions(&abil, &reg);
    quiet_about(&abil, P0, &legal, &reg, "with no matching stash");
    let mut s = abil.clone();
    s.pending_ability_effect.as_mut().unwrap().source_id = beast;
    flags(&s, P0, &legal, &reg, "with no matching stash");
}

/// CR 601.2c/702.11/702.16: an offered target is a legal target now — of
/// the right kind, not shielded, and a player who can be targeted.
#[test]
fn an_offered_target_is_a_legal_target_now() {
    let (mut state, reg) = base();
    let mine = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let theirs = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let land = named_permanent(&mut state, &reg, "Forest", P1);
    let pump = castable_spell(&mut state, &reg, "Moment of Heroism", P0);
    state.priority_player = Some(P0);

    let offering = |s: &GameState, targets: Vec<Target>| {
        let mut l = mtg_engine::engine::legal_actions(s, &reg);
        l.actions.insert(1, cast_action(pump, targets));
        l
    };

    // A land where the spell wants a creature.
    flags(&state, P0, &offering(&state, vec![Target::Object(land)]), &reg,
        "which is not a legal target now (CR 601.2c)");

    // CR 702.11b: hexproof shields it from the other player. (The shared
    // legality re-check already rejects it, so the offer is illegal either
    // way — what matters is that it is reported.)
    let mut s = state.clone();
    grant_keyword(&mut s, theirs, Keyword::Hexproof);
    let v = check_legal(&s, P0, &offering(&s, vec![Target::Object(theirs)]), &reg);
    assert!(v.iter().any(|m| m.contains("(CR 601.2c)") || m.contains("(CR 702.11/702.16)")),
        "a hexproof creature is not offered to its controller's opponent: {v:?}");

    // A spell on the stack that is on no stack entry.
    let mut s = state.clone();
    let other = castable_spell(&mut s, &reg, "Moment of Heroism", P0);
    s.get_object_mut(other).unwrap().zone = Zone::Stack;
    flags(&s, P0, &offering(&s, vec![Target::Object(other)]), &reg,
        "in the stack zone that is on no stack entry");
    let _ = mine;
}

/// CR 602.2/602.5/701.17a: an activation offer's own costs — the counters
/// it removes, the timing it demands, and the creature it sacrifices.
#[test]
fn an_activation_offer_can_pay_the_costs_its_ability_asks_for() {
    let (mut state, reg) = base();
    let priest = named_permanent(&mut state, &reg, "Avacynian Priest", P0);
    state.get_object_mut(priest).unwrap().summoning_sick = false;
    let victim = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    add_mana(&mut state, P0, &[(ManaType::White, 1)]);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let activation = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == priest))
        .expect("the Priest's tap ability is offered").clone();

    // The Priest's ability asks for no sacrifice, so naming one is a cost
    // its cost does not ask for.
    let mut l = legal.clone();
    let mut named = activation.clone();
    if let Action::ActivateAbility { sacrifice, .. } = &mut named {
        *sacrifice = Some(victim);
    }
    l.actions.insert(1, named);
    flags(&state, P0, &l, &reg, "names a sacrifice its cost does not ask for");

    // An ability offered through a card that neither grants it as a copy nor
    // is attached to the permanent.
    let mut l = legal.clone();
    let mut borrowed = activation.clone();
    if let Action::ActivateAbility { source_card_id, .. } = &mut borrowed {
        *source_card_id = Some(state.get_object(victim).unwrap().card_id);
    }
    l.actions.insert(1, borrowed);
    flags(&state, P0, &l, &reg, "which neither grants it as a copy nor is attached under p0");
}

/// CR 606.3/103.5/700.3: the offers the prompts do not enumerate — a
/// mulligan count, a pile choice's two answers, and the blockers prompt's
/// per-blocker lists.
#[test]
fn the_remaining_prompt_offers_are_checked_against_their_prompts() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let other = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.priority_player = Some(P0);

    // CR 700.3: a pile choice offers exactly the two piles.
    let mut s = state.clone();
    s.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: P0, source: bear,
        choice: ResolutionChoiceKind::ChoosePile {
            description: "?".into(), pile_1: vec![bear], pile_2: vec![other],
            source_id: bear } });
    let legal = mtg_engine::engine::legal_actions(&s, &reg);
    clean(&s, P0, &legal, &reg);
    let mut l = legal.clone();
    l.actions.pop();
    flags(&s, P0, &l, &reg, "answers, not the two piles");

    // The blockers prompt's block lists are keyed by the eligible blockers.
    let (mut state, reg) = base();
    let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let blocker = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.step = Step::DeclareAttackers;
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    state.step = Step::DeclareBlockers;
    state.awaiting_action = Some(AwaitingAction::DeclareBlockers { defending_player: P1 });
    state.priority_player = Some(P1);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let mut l = legal.clone();
    if let Some(CombatPrompt::ChooseBlockers { legal_blocks, .. }) = &mut l.combat_prompt {
        legal_blocks.remove(&blocker);
        legal_blocks.insert(attacker, vec![]);
    }
    flags(&state, P1, &l, &reg, ", not for the eligible blockers");
}

/// CR 601.2c: a cast whose remaining targets are asked for on their own
/// screen is *announced* with only the targets named ahead of that slot —
/// none at all for the modal set — and the offer invariant may not read that
/// announcement as a cast with too few targets.
///
/// Regression for the nightly-fuzz cluster of 2026-09-09 (#405 and its 49
/// siblings): "a cast asks for its targets instead of enumerating them"
/// taught `legal_actions` to announce, but not this checker to expect it, so
/// every Ghoulcaller's Chant offered over a stocked graveyard was a
/// violation. The exemption is for the announcement only: a count that no
/// mode allows is still a violation, and the stack invariant still holds the
/// spell to a full set of targets once it is cast.
#[test]
fn a_cast_with_a_slot_still_to_ask_is_announced_with_no_targets() {
    let (mut state, reg) = base();
    let chant = castable_spell(&mut state, &reg, "Ghoulcaller's Chant", P0);
    let z1 = named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);
    let z2 = named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);
    let z3 = named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);
    state.priority_player = Some(P0);

    // One announcement, with its targets left to the screen the cast raises,
    // rather than one row per way of filling the slot.
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let offers: Vec<&Action> = legal.actions.iter()
        .filter(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == chant))
        .collect();
    assert_eq!(offers.len(), 1, "expected one Chant announcement, got {offers:?}");
    assert!(matches!(offers[0], Action::CastSpell { targets, .. } if targets.is_empty()),
        "expected the announcement to name no targets, got {:?}", offers[0]);
    clean(&state, P0, &legal, &reg);

    // Not a licence for any count: three targets is neither mode.
    let mut l = legal.clone();
    l.actions.push(cast_action(chant, vec![Target::Object(z1), Target::Object(z2), Target::Object(z3)]));
    flags(&state, P0, &l, &reg, "offers 3 targets");
}

/// CR 602.2h: an ability is offered with a tap plan that, with the pool,
/// pays its cost. `tap_plan_ok` says the plan taps real sources; this says
/// the plan is enough — an ability a mana short is not an offer.
#[test]
fn an_offered_ability_is_funded_by_its_tap_plan() {
    let (mut state, reg) = base();
    let township = named_permanent(&mut state, &reg, "Gavony Township", P0);
    for basic in ["Forest", "Plains", "Plains", "Plains"] {
        named_permanent(&mut state, &reg, basic, P0);
    }
    let legal = wrong_legal(&state, &reg);
    assert!(legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == township)),
        "test precondition: four other lands pay {{2}}{{G}}{{W}}");
    quiet_about(&state, P0, &legal, &reg, "cannot pay it");

    // The same offer one source short.
    let mut short = legal.clone();
    for a in &mut short.actions {
        if let Action::ActivateAbility { object_id, tap_plan, .. } = a {
            if *object_id == township { tap_plan.pop(); }
        }
    }
    flags(&state, P0, &short, &reg, "cannot pay it");

    // Mana already floating counts toward the cost.
    add_mana(&mut state, P0, &[(ManaType::Colorless, 1)]);
    quiet_about(&state, P0, &short, &reg, "cannot pay it");
}
