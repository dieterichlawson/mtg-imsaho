//! The rules the cards lean on, each played through the engine and proved
//! of it once. A card's own rows say that it has a keyword
//! (`enters_as_printed`: the engine's `has_keyword` answers with the
//! printed ones and no others). What the keyword does is no card's: it is
//! the game's, and is asked here of a creature that has the keyword and
//! nothing else, in a combat the engine runs from the beginning of combat
//! step to the damage: each player passes, the engine asks for the
//! attackers and then for the blockers, and deals the damage as the
//! combat damage step begins. No function of the engine is called for it
//! but the ones every row of this module plays an action with ([`play`]),
//! and none is stood in for.
//!
//! A row's name begins with the rule it is of (`flying_...`,
//! `first_strike_...`), which is how `scripts/kani-claims.py` knows
//! whether a keyword's rule is proved: every row of that name is.
//!
//! The creatures are the blank card's, with the power, toughness, colour
//! and card types a row gives them and each keyword given by an effect
//! until end of turn. That is one of the ways a creature has a keyword,
//! and the rules ask the same question of every way (`has_keyword`).
//!
//! What is not here yet: a planeswalker attacked, more than two blockers
//! on an attacker, and the turn outside combat.

use super::*;
use crate::actions::ResolvedChoice;
use crate::state::{AwaitingAction, ResolutionChoiceKind, TemporaryEffect};

/// A creature for a row.
#[derive(Clone, Copy)]
struct Fighter {
    power: i32,
    toughness: i32,
    keywords: &'static [Keyword],
    /// Black, where it is otherwise colourless.
    black: bool,
    /// An artifact creature.
    artifact: bool,
    tapped: bool,
    /// Come under its controller's control this turn (CR 302.6).
    sick: bool,
}

const A_BEAR: Fighter = Fighter { power: 2, toughness: 2, keywords: &[], black: false, artifact: false, tapped: false, sick: false };
const A_WALL: Fighter = Fighter { power: 2, toughness: 6, ..A_BEAR };
/// A 4/4 with trample.
const A_TRAMPLER: Fighter = Fighter { power: 4, toughness: 4, keywords: &[Keyword::Trample], ..A_BEAR };

/// A 2/2 with `keywords`.
const fn with(keywords: &'static [Keyword]) -> Fighter {
    Fighter { keywords, ..A_BEAR }
}

fn a_fighter(state: &mut GameState, blank: CardId, whose: u8, fighter: Fighter) -> ObjectId {
    let id = state.create_object(blank, PlayerId(whose), Zone::Battlefield, Some(fighter.power), Some(fighter.toughness));
    let o = state.get_object_mut(id).expect("just created");
    o.card_types = if fighter.artifact { vec![CardType::Artifact, CardType::Creature] } else { vec![CardType::Creature] };
    if fighter.black {
        o.colors = vec![Color::Black];
    }
    o.tapped = fighter.tapped;
    o.summoning_sick = fighter.sick;
    for &keyword in fighter.keywords {
        state.until_end_of_turn.push(TemporaryEffect::GrantKeyword { target: id, keyword });
    }
    id
}

/// The first player's beginning of combat step, priority theirs and the
/// stack empty.
fn at_the_beginning_of_combat(state: &mut GameState) {
    state.active_player = PlayerId(0);
    state.step = Step::BeginCombat;
    state.priority_player = Some(PlayerId(0));
}

/// Each player passes in turn with the stack empty, the active player
/// first: the step ends and the engine begins `next` (CR 117.4, 500.2).
fn the_step_ends(state: &mut GameState, registry: &CardRegistry, next: Step) {
    let active = state.active_player;
    assert!(state.stack.is_empty() && state.priority_player == Some(active) && state.awaiting_action.is_none());
    play(state, registry, &Action::PassPriority);
    assert!(state.priority_player == Some(PlayerId(1 - active.0)));
    play(state, registry, &Action::PassPriority);
    assert!(state.step == next && state.active_player == active);
}

/// The engine has asked the active player for their attackers (CR 508.1),
/// and is answered with `attackers`, each at the other player.
fn attacks_with(state: &mut GameState, registry: &CardRegistry, attackers: &[ObjectId]) {
    assert!(state.step == Step::DeclareAttackers && matches!(state.awaiting_action, Some(AwaitingAction::DeclareAttackers)));
    assert!(state.priority_player == Some(state.active_player));
    let action = Action::DeclareAttackers { attackers: attackers.iter().map(|&a| (a, PlayerId(1))).collect(), planeswalker_attacks: Vec::new() };
    play(state, registry, &action);
    assert!(state.awaiting_action.is_none() && state.priority_player == Some(state.active_player) && state.stack.is_empty());
    std::mem::forget(action);
}

/// The engine has asked the defending player for their blockers (CR
/// 509.1), and is answered with `blocks`, each a blocker and the attacker
/// it blocks.
fn blocks_with(state: &mut GameState, registry: &CardRegistry, blocks: &[(ObjectId, ObjectId)]) {
    assert!(state.step == Step::DeclareBlockers);
    assert!(matches!(state.awaiting_action, Some(AwaitingAction::DeclareBlockers { defending_player }) if defending_player == PlayerId(1)));
    assert!(state.priority_player == Some(PlayerId(1)));
    let action = Action::DeclareBlockers { assignments: blocks.to_vec() };
    play(state, registry, &action);
    assert!(state.awaiting_action.is_none() && state.priority_player == Some(state.active_player) && state.stack.is_empty());
    std::mem::forget(action);
}

fn is_attacking(state: &GameState, id: ObjectId) -> bool {
    state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&id))
}

fn is_blocking(state: &GameState, blocker: ObjectId, attacker: ObjectId) -> bool {
    state.combat.as_ref().is_some_and(|c| c.blocker_assignments.get(&attacker).is_some_and(|b| b.contains(&blocker)))
}

/// The first player declares their one creature, `attacker`, as an
/// attacker. Where the rules let it attack (`attacks`), it is attacking
/// the other player and has been tapped for it unless it has vigilance
/// (CR 508.1f, 702.20b), and the declare blockers step follows. Where
/// they do not (508.1a: it is tapped, or has defender, 702.3b, or came
/// under its controller's control this turn and has no haste, 302.6,
/// 702.10b), the engine leaves it out of the attack: nothing attacks,
/// nothing is tapped, and the turn goes on to the end of combat step with
/// no blockers asked for and no damage dealt (508.8). Either way nothing
/// else of the game has changed.
fn an_attack((attacker, attacks): (Fighter, bool)) {
    let (registry, mut state, blank, _) = board(Box::new(crate::proofs::cards::OtherBlank));
    let creature = a_fighter(&mut state, blank, 0, attacker);
    let theirs = a_fighter(&mut state, blank, 1, A_BEAR);
    at_the_beginning_of_combat(&mut state);
    let mut expected = everything(&state, &registry);

    the_step_ends(&mut state, &registry, Step::DeclareAttackers);
    attacks_with(&mut state, &registry, &[creature]);

    assert!(is_attacking(&state, creature) == attacks && !is_attacking(&state, theirs));
    let taps = attacks && !attacker.keywords.contains(&Keyword::Vigilance);
    expected.of(creature).tapped = attacker.tapped || taps;
    expected.step = Step::DeclareAttackers;
    assert!(everything(&state, &registry) == expected);

    the_step_ends(&mut state, &registry, if attacks { Step::DeclareBlockers } else { Step::EndCombat });
    expected.step = state.step;
    expected.priority = Some(if attacks { PlayerId(1) } else { PlayerId(0) });
    expected.asked = attacks;
    assert!(everything(&state, &registry) == expected);
    std::mem::forget((registry, state));
}

/// The first player's `attacker` attacks and the second player declares
/// `blocker` as blocking it. Where the rules let it block (`blocks`), it
/// is blocking the attacker; where they do not (CR 509.1a-b: the blocker
/// is tapped; the attacker has flying and the blocker neither flying nor
/// reach, 702.9b, 702.17b; the attacker has intimidate and the blocker is
/// no artifact creature and shares no colour with it, 702.13b), the
/// engine leaves the block out and the attacker is unblocked. Either way
/// nothing of the game has changed but the attacker tapped for its
/// attack.
fn a_block((attacker, blocker, blocks): (Fighter, Fighter, bool)) {
    let (registry, mut state, blank, _) = board(Box::new(crate::proofs::cards::OtherBlank));
    let attacking = a_fighter(&mut state, blank, 0, attacker);
    let blocking = a_fighter(&mut state, blank, 1, blocker);
    at_the_beginning_of_combat(&mut state);
    let mut expected = everything(&state, &registry);

    the_step_ends(&mut state, &registry, Step::DeclareAttackers);
    attacks_with(&mut state, &registry, &[attacking]);
    assert!(is_attacking(&state, attacking));
    the_step_ends(&mut state, &registry, Step::DeclareBlockers);
    blocks_with(&mut state, &registry, &[(blocking, attacking)]);

    assert!(is_blocking(&state, blocking, attacking) == blocks && is_attacking(&state, attacking));
    expected.of(attacking).tapped = !attacker.keywords.contains(&Keyword::Vigilance);
    expected.step = Step::DeclareBlockers;
    assert!(everything(&state, &registry) == expected);
    std::mem::forget((registry, state));
}

/// What has become of a creature once combat damage has been dealt and
/// the state-based actions checked.
#[derive(Clone, Copy)]
enum Fate {
    /// On the battlefield with this much damage marked on it.
    Marked(u32),
    /// In its owner's graveyard (CR 704.5g-h).
    Dead,
}

/// What a combat comes to.
#[derive(Clone, Copy)]
struct Outcome {
    /// The life the defending player has lost.
    to_the_player: i32,
    /// The life the attacking player has gained.
    gained: i32,
    attacker: Fate,
    /// The blocker's, where there is one.
    blocker: Fate,
}

/// The first player's `attacker` attacks, the second player blocks it
/// with `blocker` if there is one, and combat damage is dealt: in one
/// combat damage step, or in two if either creature has first strike or
/// double strike (CR 510.4), the state-based actions checked after each.
/// Where an attacker with trample is blocked and has more power than is
/// lethal to its blocker, the engine asks its controller how much of it
/// the blocker is assigned (510.1c, 702.19b: at least lethal damage, the
/// rest to the player), and `assigns` is the answer: that many more than
/// lethal. The engine asks then and at no other time.
/// The game is then as `outcome` says and nothing else of it has changed:
/// the defending player's life, the attacking player's, each creature
/// dead or with its damage marked, the attacker tapped unless it has
/// vigilance. What `outcome` is for a row is written from the rules, in
/// the row: an unblocked attacker deals its power to the player (510.1b),
/// a blocked one and its blocker to each other (510.1c-d), each at once
/// (510.2) but for first strike and double strike (702.7b, 702.4b); a
/// creature with damage at least its toughness, or any damage from a
/// source with deathtouch, is destroyed (704.5g-h, 702.2b) unless it is
/// indestructible (702.12b); and damage from a source with lifelink has
/// its controller gain that much life (702.15b).
fn a_combat((attacker, blocker, assigns, outcome): (Fighter, Option<Fighter>, Option<usize>, Outcome)) {
    let (registry, mut state, blank, _) = board(Box::new(crate::proofs::cards::OtherBlank));
    let attacking = a_fighter(&mut state, blank, 0, attacker);
    let blocking = blocker.map(|b| a_fighter(&mut state, blank, 1, b));
    at_the_beginning_of_combat(&mut state);
    let mut expected = everything(&state, &registry);

    the_step_ends(&mut state, &registry, Step::DeclareAttackers);
    attacks_with(&mut state, &registry, &[attacking]);
    assert!(is_attacking(&state, attacking));
    the_step_ends(&mut state, &registry, Step::DeclareBlockers);
    match blocking {
        Some(b) => {
            blocks_with(&mut state, &registry, &[(b, attacking)]);
            assert!(is_blocking(&state, b, attacking));
        }
        None => blocks_with(&mut state, &registry, &[]),
    }
    the_step_ends(&mut state, &registry, Step::CombatDamage);
    the_damage_is_divided(&mut state, &registry, attacking, assigns);
    crate::engine::before_priority(&mut state, &registry);
    let strikes_first = |f: Fighter| f.keywords.contains(&Keyword::FirstStrike) || f.keywords.contains(&Keyword::DoubleStrike);
    if strikes_first(attacker) || blocker.is_some_and(strikes_first) {
        // A second combat damage step, for the creatures that did not
        // deal theirs in the first and for double strike (CR 510.4).
        the_step_ends(&mut state, &registry, Step::CombatDamage);
        crate::engine::before_priority(&mut state, &registry);
    }

    assert!(state.stack.is_empty() && state.awaiting_action.is_none() && state.pending_damage.is_empty());
    expected.step = Step::CombatDamage;
    expected.players[1].life -= outcome.to_the_player;
    expected.players[0].life += outcome.gained;
    expected.of(attacking).tapped = !attacker.keywords.contains(&Keyword::Vigilance);
    match outcome.attacker {
        Fate::Marked(damage) => expected.of(attacking).damage = damage,
        Fate::Dead => {
            expected.is_buried(attacking, 0);
            expected.a_creature_died = true;
        }
    }
    if let Some(b) = blocking {
        match outcome.blocker {
            Fate::Marked(damage) => expected.of(b).damage = damage,
            Fate::Dead => {
                expected.is_buried(b, 1);
                expected.a_creature_died = true;
            }
        }
    }
    // The effects a creature's keywords were given by are the board's
    // own, and whether the engine keeps one for a creature that has died
    // is its own affair: nothing can ask it of a card in a graveyard.
    expected.effects_until_end_of_turn = state.until_end_of_turn.len();
    assert!(everything(&state, &registry) == expected);
    std::mem::forget((registry, state));
}

const UNHURT: Fate = Fate::Marked(0);

/// As a combat damage step begins the engine asks the attacking player
/// how `attacker`'s damage is divided, or does not: it asks exactly when
/// `assigns` is an answer, and is answered with it (so many more than
/// lethal damage to the blocker asked of), after which the damage has
/// been dealt.
fn the_damage_is_divided(state: &mut GameState, registry: &CardRegistry, attacker: ObjectId, assigns: Option<usize>) {
    let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice { player, source, choice: ResolutionChoiceKind::AssignCombatDamage { .. } })
        if *player == state.active_player && *source == attacker);
    assert!(asked == assigns.is_some() && (asked || state.awaiting_action.is_none()));
    if let Some(more) = assigns {
        let answer = Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(more, String::new()) };
        play(state, registry, &answer);
        assert!(state.awaiting_action.is_none() && state.priority_player == Some(state.active_player));
        std::mem::forget(answer);
    }
}

/// The first player's `attacker` attacks and the second player declares
/// two 2/2 creatures as blocking it. Unless the attacker has a keyword
/// that forbids it they both block (CR 509.1a), and the engine asks the
/// attacking player for the order damage is assigned to them in (509.2),
/// which is `first` and then the other. Combat damage is then dealt: the
/// attacker's to the first in the order up to lethal damage and the rest
/// to the second (510.1c), `more` than lethal to the first if the engine
/// asks; each blocker's to the attacker (510.1d). The game is then as
/// `outcome` says, its `blocker` the fate of the one first in the order
/// and `second` the other's, and nothing else of it has changed.
fn a_double_block((attacker, first, more, outcome, second): (Fighter, usize, Option<usize>, Outcome, Fate)) {
    let (registry, mut state, blank, _) = board(Box::new(crate::proofs::cards::OtherBlank));
    let attacking = a_fighter(&mut state, blank, 0, attacker);
    let blockers = [a_fighter(&mut state, blank, 1, A_BEAR), a_fighter(&mut state, blank, 1, A_BEAR)];
    at_the_beginning_of_combat(&mut state);
    let mut expected = everything(&state, &registry);

    the_step_ends(&mut state, &registry, Step::DeclareAttackers);
    attacks_with(&mut state, &registry, &[attacking]);
    the_step_ends(&mut state, &registry, Step::DeclareBlockers);
    // The two blocks, and the question of their order that follows them.
    assert!(matches!(state.awaiting_action, Some(AwaitingAction::DeclareBlockers { defending_player }) if defending_player == PlayerId(1)));
    let blocks = Action::DeclareBlockers { assignments: vec![(blockers[0], attacking), (blockers[1], attacking)] };
    play(&mut state, &registry, &blocks);
    assert!(is_blocking(&state, blockers[0], attacking) && is_blocking(&state, blockers[1], attacking));
    assert!(matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice { player, source, choice: ResolutionChoiceKind::ChooseDamageAssignmentOrder { .. } })
        if *player == PlayerId(0) && *source == attacking));
    let order = Action::ResolveChoice { choice: ResolvedChoice::ChosenOrder(vec![first, 1 - first]) };
    play(&mut state, &registry, &order);
    assert!(state.awaiting_action.is_none() && state.priority_player == Some(PlayerId(0)));

    the_step_ends(&mut state, &registry, Step::CombatDamage);
    the_damage_is_divided(&mut state, &registry, attacking, more);
    crate::engine::before_priority(&mut state, &registry);

    assert!(state.stack.is_empty() && state.awaiting_action.is_none() && state.pending_damage.is_empty());
    expected.step = Step::CombatDamage;
    expected.players[1].life -= outcome.to_the_player;
    expected.players[0].life += outcome.gained;
    expected.of(attacking).tapped = true;
    let mut fates = [(attacking, 0u8, outcome.attacker), (blockers[first], 1, outcome.blocker), (blockers[1 - first], 1, second)];
    // Dead together, each to its owner's graveyard, the second player's
    // two in the order the engine has them (CR 404.3: their owner's to
    // choose, and the engine's choice is the order of its objects).
    fates[1..].sort_by_key(|(id, _, _)| id.0);
    for (id, owner, fate) in fates {
        match fate {
            Fate::Marked(damage) => expected.of(id).damage = damage,
            Fate::Dead => {
                expected.is_buried(id, owner);
                expected.a_creature_died = true;
            }
        }
    }
    expected.effects_until_end_of_turn = state.until_end_of_turn.len();
    assert!(everything(&state, &registry) == expected);
    std::mem::forget((registry, state, blocks, order));
}

/// The first player's attacker with menace attacks and the second player
/// declares one creature as blocking it: a creature with menace can't be
/// blocked except by two or more creatures (CR 702.111b), the engine
/// leaves the block out, and the attacker is unblocked.
fn menace_is_not_blocked_by_one((): ()) {
    a_block((with(&[Keyword::Menace]), A_BEAR, false));
}

/// A creature with hexproof can't be the target of a spell an opponent
/// controls, and can be of one its controller does (CR 702.11b): Doom
/// Blade cast at the other player's is refused and nothing is done, and
/// Giant Growth cast at its caster's own resolves and gives it +3/+3.
fn hexproof((yours, card): (bool, Box<dyn CardBehavior>)) {
    let (registry, mut state, blank, card) = board(card);
    let creature = a_fighter(&mut state, blank, if yours { 0 } else { 1 }, with(&[Keyword::Hexproof]));
    let spell = a_spell(&mut state, &registry, card, FIRST);
    if yours {
        let mut expected = everything(&state, &registry);
        cast_and_resolve(&mut state, &registry, spell, vec![Target::Object(creature)]);
        let seen = expected.of(creature);
        (seen.power_now, seen.toughness_now) = (Some(5), Some(5));
        expected.effects_until_end_of_turn = state.until_end_of_turn.len();
        expected.has_resolved(spell, FIRST, state.active_player);
        assert!(everything(&state, &registry) == expected);
    } else {
        cast_is_refused(&mut state, &registry, spell, vec![Target::Object(creature)]);
    }
    std::mem::forget((registry, state));
}

// Who may attack (CR 508.1a, 302.6), and what attacking costs (508.1f).
through_the_stack! {
    combat_a_creature_attacks_and_is_tapped_for_it: an_attack((A_BEAR, true));
    combat_a_tapped_creature_does_not_attack: an_attack((Fighter { tapped: true, ..A_BEAR }, false));
    summoning_sickness_keeps_a_creature_from_attacking: an_attack((Fighter { sick: true, ..A_BEAR }, false));
    haste_lets_a_creature_attack_the_turn_it_arrives: an_attack((Fighter { sick: true, ..with(&[Keyword::Haste]) }, true));
    defender_keeps_a_creature_from_attacking: an_attack((with(&[Keyword::Defender]), false));
    vigilance_attacks_without_tapping: an_attack((with(&[Keyword::Vigilance]), true));
}

// Who may block whom (CR 509.1a-b).
through_the_stack! {
    combat_a_creature_blocks_an_attacker: a_block((A_BEAR, A_BEAR, true));
    combat_a_tapped_creature_does_not_block: a_block((A_BEAR, Fighter { tapped: true, ..A_BEAR }, false));
    combat_a_creature_that_arrived_this_turn_blocks: a_block((A_BEAR, Fighter { sick: true, ..A_BEAR }, true));
    flying_is_not_blocked_by_a_creature_on_the_ground: a_block((with(&[Keyword::Flying]), A_BEAR, false));
    flying_is_blocked_by_a_creature_with_flying: a_block((with(&[Keyword::Flying]), with(&[Keyword::Flying]), true));
    flying_does_not_keep_a_creature_from_blocking_on_the_ground: a_block((A_BEAR, with(&[Keyword::Flying]), true));
    reach_blocks_a_creature_with_flying: a_block((with(&[Keyword::Flying]), with(&[Keyword::Reach]), true));
    intimidate_is_not_blocked_by_a_creature_of_no_colour_of_its_own: a_block((Fighter { black: true, ..with(&[Keyword::Intimidate]) }, A_BEAR, false));
    intimidate_is_blocked_by_a_creature_that_shares_a_colour: a_block((Fighter { black: true, ..with(&[Keyword::Intimidate]) }, Fighter { black: true, ..A_BEAR }, true));
    intimidate_is_blocked_by_an_artifact_creature: a_block((Fighter { black: true, ..with(&[Keyword::Intimidate]) }, Fighter { artifact: true, ..A_BEAR }, true));
}

// What combat damage is dealt, to whom and when (CR 510), and what it does.
through_the_stack! {
    combat_an_unblocked_attacker_deals_its_power_to_the_player: a_combat((A_BEAR, None, None, Outcome { to_the_player: 2, gained: 0, attacker: UNHURT, blocker: UNHURT }));
    combat_a_blocked_attacker_and_its_blocker_deal_damage_to_each_other: a_combat((A_BEAR, Some(A_BEAR), None, Outcome { to_the_player: 0, gained: 0, attacker: Fate::Dead, blocker: Fate::Dead }));
    combat_damage_short_of_toughness_is_marked_and_does_not_destroy: a_combat((A_BEAR, Some(A_WALL), None, Outcome { to_the_player: 0, gained: 0, attacker: Fate::Dead, blocker: Fate::Marked(2) }));
    first_strike_kills_a_blocker_before_it_deals_damage: a_combat((with(&[Keyword::FirstStrike]), Some(A_BEAR), None, Outcome { to_the_player: 0, gained: 0, attacker: UNHURT, blocker: Fate::Dead }));
    first_strike_on_a_blocker_kills_the_attacker_first: a_combat((A_BEAR, Some(with(&[Keyword::FirstStrike])), None, Outcome { to_the_player: 0, gained: 0, attacker: Fate::Dead, blocker: UNHURT }));
    first_strike_against_first_strike_is_dealt_at_once: a_combat((with(&[Keyword::FirstStrike]), Some(with(&[Keyword::FirstStrike])), None, Outcome { to_the_player: 0, gained: 0, attacker: Fate::Dead, blocker: Fate::Dead }));
    first_strike_unblocked_deals_its_damage_once: a_combat((with(&[Keyword::FirstStrike]), None, None, Outcome { to_the_player: 2, gained: 0, attacker: UNHURT, blocker: UNHURT }));
    double_strike_unblocked_deals_its_power_twice: a_combat((with(&[Keyword::DoubleStrike]), None, None, Outcome { to_the_player: 4, gained: 0, attacker: UNHURT, blocker: UNHURT }));
    double_strike_deals_damage_in_both_steps_to_a_blocker_that_survives_the_first: a_combat((with(&[Keyword::DoubleStrike]), Some(A_WALL), None, Outcome { to_the_player: 0, gained: 0, attacker: Fate::Dead, blocker: Fate::Marked(4) }));
    deathtouch_destroys_a_blocker_whatever_its_toughness: a_combat((Fighter { power: 1, toughness: 1, ..with(&[Keyword::Deathtouch]) }, Some(A_WALL), None, Outcome { to_the_player: 0, gained: 0, attacker: Fate::Dead, blocker: Fate::Dead }));
    deathtouch_on_a_blocker_destroys_the_attacker: a_combat((A_WALL, Some(Fighter { power: 1, toughness: 1, ..with(&[Keyword::Deathtouch]) }), None, Outcome { to_the_player: 0, gained: 0, attacker: Fate::Dead, blocker: Fate::Dead }));
    lifelink_gains_its_controller_the_damage_dealt_to_a_player: a_combat((with(&[Keyword::Lifelink]), None, None, Outcome { to_the_player: 2, gained: 2, attacker: UNHURT, blocker: UNHURT }));
    lifelink_gains_its_controller_the_damage_dealt_to_a_creature: a_combat((with(&[Keyword::Lifelink]), Some(A_WALL), None, Outcome { to_the_player: 0, gained: 2, attacker: Fate::Dead, blocker: Fate::Marked(2) }));
    indestructible_is_not_destroyed_by_lethal_damage: a_combat((with(&[Keyword::Indestructible]), Some(A_BEAR), None, Outcome { to_the_player: 0, gained: 0, attacker: Fate::Marked(2), blocker: Fate::Dead }));
    indestructible_is_not_destroyed_by_deathtouch: a_combat((with(&[Keyword::Indestructible]), Some(Fighter { power: 1, toughness: 1, ..with(&[Keyword::Deathtouch]) }), None, Outcome { to_the_player: 0, gained: 0, attacker: Fate::Marked(1), blocker: Fate::Dead }));
    vigilance_deals_combat_damage_untapped: a_combat((with(&[Keyword::Vigilance]), None, None, Outcome { to_the_player: 2, gained: 0, attacker: UNHURT, blocker: UNHURT }));
}

// Trample, and how damage is divided among blockers (CR 510.1c, 702.19).
through_the_stack! {
    trample_unblocked_deals_its_power_to_the_player: a_combat((A_TRAMPLER, None, None, Outcome { to_the_player: 4, gained: 0, attacker: UNHURT, blocker: UNHURT }));
    trample_assigns_lethal_damage_to_its_blocker_and_the_rest_to_the_player: a_combat((A_TRAMPLER, Some(A_BEAR), Some(0), Outcome { to_the_player: 2, gained: 0, attacker: Fate::Marked(2), blocker: Fate::Dead }));
    trample_may_assign_all_its_damage_to_its_blocker: a_combat((A_TRAMPLER, Some(A_BEAR), Some(2), Outcome { to_the_player: 0, gained: 0, attacker: Fate::Marked(2), blocker: Fate::Dead }));
    trample_assigns_nothing_to_the_player_past_a_blocker_it_cannot_kill: a_combat((A_TRAMPLER, Some(A_WALL), None, Outcome { to_the_player: 0, gained: 0, attacker: Fate::Marked(2), blocker: Fate::Marked(4) }));
    trample_with_deathtouch_assigns_one_damage_to_its_blocker: a_combat((Fighter { keywords: &[Keyword::Trample, Keyword::Deathtouch], ..A_TRAMPLER }, Some(A_WALL), Some(0), Outcome { to_the_player: 3, gained: 0, attacker: Fate::Marked(2), blocker: Fate::Dead }));
    combat_without_trample_a_blocked_attacker_deals_nothing_to_the_player: a_combat((Fighter { keywords: &[], ..A_TRAMPLER }, Some(A_BEAR), None, Outcome { to_the_player: 0, gained: 0, attacker: Fate::Marked(2), blocker: Fate::Dead }));
    combat_two_blockers_are_dealt_damage_in_the_order_chosen: a_double_block((A_BEAR, 1, None, Outcome { to_the_player: 0, gained: 0, attacker: Fate::Dead, blocker: Fate::Dead }, UNHURT));
    combat_damage_past_lethal_goes_to_the_next_blocker_in_the_order: a_double_block((Fighter { keywords: &[], ..A_TRAMPLER }, 0, Some(0), Outcome { to_the_player: 0, gained: 0, attacker: Fate::Dead, blocker: Fate::Dead }, Fate::Dead));
    menace_is_blocked_by_two_creatures: a_double_block((with(&[Keyword::Menace]), 0, None, Outcome { to_the_player: 0, gained: 0, attacker: Fate::Dead, blocker: Fate::Dead }, UNHURT));
    menace_is_not_blocked_by_one_creature: menace_is_not_blocked_by_one(());
}

// Hexproof is of targeting, not of combat (CR 702.11b).
through_the_stack! {
    hexproof_is_not_the_target_of_an_opponents_spell: hexproof((false, Box::new(DoomBlade)));
    hexproof_is_the_target_of_its_controllers_own_spell: hexproof((true, Box::new(GiantGrowth)));
}
