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
use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
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
    let mut l = legal.clone();
    l.actions.push(Action::MulliganKeep);
    flags(&s, P0, &l, &reg, "bottoming prompt offers MulliganKeep");

    // A discard prompt.
    let mut s = state.clone();
    s.priority_player = None;
    s.step = Step::Cleanup;
    s.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P0, discard_count: 1 });
    let legal = mtg_engine::engine::legal_actions(&s, &reg);
    clean(&s, P0, &legal, &reg);
    flags(&s, P1, &legal, &reg, "discard prompt offered to p1, not p0");
    let mut l = legal.clone();
    l.actions.clear();
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

    // CR 115.5: a spell does not target itself.
    let mut l = legal.clone();
    l.actions.insert(1, cast_action(pump, vec![Target::Object(pump)]));
    flags(&state, P0, &l, &reg, "targets itself (CR 115.5)");

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
fn a_choose_n_of_your_hand_prompt_offers_every_subset_once() {
    let (mut state, reg) = base();
    let hand: Vec<ObjectId> = (0..5)
        .map(|_| spell_in_hand(&mut state, &reg, "Moment of Heroism", P0))
        .collect();
    state.priority_player = None;
    state.step = Step::Cleanup;
    state.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P0, discard_count: 2 });
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    assert_eq!(legal.actions.len(), 10, "precondition: C(5, 2) offers");
    clean(&state, P0, &legal, &reg);

    // One subset missing.
    let mut l = legal.clone();
    l.actions.pop();
    flags(&state, P0, &l, &reg, "9 discard offers for 2 cards of 5 (CR 514.1)");

    // One subset offered twice — a repeat, and one too few distinct sets.
    let mut l = legal.clone();
    let dup = l.actions[0].clone();
    l.actions[9] = dup;
    flags(&state, P0, &l, &reg, "repeats a set");

    // A subset of the wrong size, or naming a card that is not in hand.
    let mut l = legal.clone();
    l.actions[0] = Action::DiscardCards { cards: vec![hand[0]] };
    flags(&state, P0, &l, &reg, "is not 2 cards of p0's hand (CR 514.1)");
    let mut l = legal.clone();
    l.actions[0] = Action::DiscardCards { cards: vec![hand[0], hand[0]] };
    flags(&state, P0, &l, &reg, "discard offer lists #");

    // The same rule for the bottoming prompt after a mulligan (CR 103.5).
    let mut s = state.clone();
    s.awaiting_action = Some(AwaitingAction::BottomAfterMulligan { player: P0, count: 2 });
    s.step = Step::PrecombatMain;
    let legal = mtg_engine::engine::legal_actions(&s, &reg);
    assert_eq!(legal.actions.len(), 10, "precondition: C(5, 2) offers");
    clean(&s, P0, &legal, &reg);
    let mut l = legal.clone();
    l.actions.pop();
    flags(&s, P0, &l, &reg, "9 bottom offers for 2 cards of 5 (CR 103.5)");
    let mut l = legal.clone();
    l.actions[0] = Action::BottomCards { cards: vec![hand[0]] };
    flags(&s, P0, &l, &reg, "is not 2 cards of p0's hand (CR 103.5)");
}

/// The subset count is a binomial coefficient, and a prompt over a hand
/// where n and k are far apart is where an arithmetic slip in it shows.
#[test]
fn the_subset_count_is_the_binomial_coefficient() {
    let reg = registry();
    // C(6, 4) = 15, and 4 is the half that is NOT the smaller one — the
    // count has to come out the same computed either way round.
    for (n, k, expected) in [(5usize, 2usize, 10usize), (6, 4, 15), (4, 1, 4)] {
        let mut state = game_at_step(Step::Cleanup, P0);
        state.turn_number = 3;
        for _ in 0..n {
            spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);
        }
        state.priority_player = None;
        state.awaiting_action = Some(AwaitingAction::DiscardToHandSize {
            player: P0, discard_count: k });
        let legal = mtg_engine::engine::legal_actions(&state, &reg);
        assert_eq!(legal.actions.len(), expected,
            "precondition: C({n}, {k}) = {expected} offers");
        clean(&state, P0, &legal, &reg);

        let mut l = legal.clone();
        l.actions.pop();
        flags(&state, P0, &l, &reg,
            &format!("{} discard offers for {k} cards of {n} (CR 514.1)", expected - 1));
    }
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
            ap_queue: true, indices: vec![0, 1] },
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

    // CR 702.111: menace asks for two, and the prompt says so.
    let mut s = state.clone();
    grant_keyword(&mut s, attacker, Keyword::Menace);
    let mut l = mtg_engine::engine::legal_actions(&s, &reg);
    if let Some(CombatPrompt::ChooseBlockers { min_blockers, .. }) = &mut l.combat_prompt {
        min_blockers.clear();
    }
    flags(&s, P1, &l, &reg, "which has menace (CR 702.111)");

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
