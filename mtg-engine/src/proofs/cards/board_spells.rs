//! The spells that read the board as they resolve: each creature, each
//! enchantment, each card in a graveyard or a library that their text
//! names, and only those.
//!
//! Scope: proved here is what each card declares (its abilities, costs,
//! targets, triggers and static effects) and what its own code writes to
//! the game state (a question asked, an until-end-of-turn effect added).
//! What a resolution does through the engine, the moves, damage, draws
//! and destruction, is not proved here, and where the text below speaks
//! of one it is of that part alone: each card is played end to end with
//! the real engine by `tests/card_semantics.rs`.
//!
//! - Hysterical Blindness, {2}{U} Instant: "Creatures your opponents
//!   control get -4/-0 until end of turn."
//! - Rally the Peasants, {2}{W} Instant: "Creatures you control get +2/+0
//!   until end of turn. Flashback {2}{R}"
//! - Vampiric Fury, {1}{R} Instant: "Vampire creatures you control get
//!   +2/+0 and gain first strike until end of turn."
//! - Spare from Evil, {1}{W} Instant: "Creatures you control gain protection
//!   from non-Human creatures until end of turn."
//! - Rolling Temblor, {2}{R} Sorcery: "Rolling Temblor deals 2 damage to
//!   each creature without flying. Flashback {4}{R}{R}"
//! - Moonmist, {1}{G} Instant: "Transform all Humans. Prevent all combat
//!   damage that would be dealt this turn by creatures other than
//!   Werewolves and Wolves."
//! - Paraselene, {2}{W} Sorcery: "Destroy all enchantments. You gain 1 life
//!   for each enchantment destroyed this way."
//! - Gnaw to the Bone, {2}{G} Instant: "You gain 2 life for each creature
//!   card in your graveyard. Flashback {2}{G}"
//! - Spider Spawning, {4}{G} Sorcery: "Create a 1/2 green Spider creature
//!   token with reach for each creature card in your graveyard. Flashback
//!   {6}{B}"
//! - Make a Wish, {3}{G} Sorcery: "Return two cards at random from your
//!   graveyard to your hand."
//! - Army of the Damned, {5}{B}{B}{B} Sorcery: "Create thirteen tapped 2/2
//!   black Zombie creature tokens. Flashback {7}{B}{B}{B}"
//! - Infernal Plunge, {R} Sorcery: "As an additional cost to cast this
//!   spell, sacrifice a creature. Add {R}{R}{R}."
//! - Desperate Ravings, {1}{R} Instant: "Draw two cards, then discard a card
//!   at random. Flashback {2}{U}"
//! - Mulch, {1}{G} Sorcery: "Reveal the top four cards of your library. Put
//!   all land cards revealed this way into your hand and the rest into your
//!   graveyard."
//! - Past in Flames, {3}{R} Sorcery: "Each instant and sorcery card in your
//!   graveyard gains flashback until end of turn. The flashback cost is
//!   equal to its mana cost. Flashback {4}{R}"
//!
//! The board, unless a proof says otherwise: the spell, cast by player 0,
//! and two other objects, a card and a token ([`any_object`]), each either
//! player's and of any characteristics the spell's words could ask about.
//! "You" is the spell's controller (CR 109.5); "creatures" are the
//! permanents that are creatures (CR 110.1, 302); "each" is every one,
//! in the order the game lists them.

use super::{
    acted, any_object, any_object_of, board, both, card_proof, case_by_case, each_is,
    has_subtype, kept, Form, Kinds, Snapshot,
};
use crate::cards::{CardBehavior, CardRegistry};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{GameObject, GameState, TemporaryEffect as T};
use crate::types::{CardType, CreatureFilter as F, Keyword, Zone};

use crate::cards::isd;

/// A spell resolving on the board above: the game after, the spell, the
/// two objects, and the board before.
struct Resolved {
    _registry: CardRegistry,
    state: GameState,
    others: [ObjectId; 2],
    before: Snapshot,
}

impl Resolved {
    /// The objects, in the game's order, that `pred` holds for as they
    /// were when the spell began to resolve: the recorders change nothing,
    /// so as they are now.
    fn those(&self, pred: impl Fn(&GameObject) -> bool) -> Vec<ObjectId> {
        kept(&self.others, |id| pred(self.state.get_object(id).expect("on the board")))
    }

    /// The until-end-of-turn effects the spell added.
    fn added(&self) -> &[T] {
        &self.state.until_end_of_turn[self.before.until_end_of_turn()..]
    }

    /// Nothing changed but by the actions asked for and the effects added.
    fn nothing_else_changed(&self) -> bool {
        Snapshot::of(&self.state).without_until_end_of_turn() == self.before.without_until_end_of_turn()
    }
}

fn resolve(card: Box<dyn CardBehavior>, kinds: &Kinds) -> Resolved {
    let (registry, mut state, blank, card) = board(card);
    let others = [any_object(&mut state, blank, kinds, Form::Card), any_object(&mut state, blank, kinds, Form::Token)];
    resolve_on(registry, state, card, others)
}

fn resolve_on(registry: CardRegistry, mut state: GameState, card: CardId, others: [ObjectId; 2]) -> Resolved {
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let before = Snapshot::of(&state);
    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);
    Resolved { _registry: registry, state, others, before }
}

fn creature(o: &GameObject) -> bool {
    o.zone == Zone::Battlefield && o.card_types.contains(&CardType::Creature)
}

fn yours(o: &GameObject) -> bool {
    o.controller == PlayerId(0)
}

const CREATURES: Kinds = Kinds { types: &[CardType::Creature, CardType::Enchantment], subtypes: &[], keywords: &[] };

card_proof! {
    /// Hysterical Blindness: -4/-0 until end of turn on each creature an
    /// opponent controls, and nothing else (CR 611.2c: the creatures it
    /// affects are those there as it resolves).
    fn hysterical_blindness_shrinks_each_opposing_creature() {
        let r = resolve(Box::new(isd::hysterical_blindness::HystericalBlindness), &CREATURES);
        let ids = r.those(|o| creature(o) && !yours(o));
        assert!(each_is(r.added(), ids.len(), |i| T::ModifyPT { target: ids[i], power_mod: -4, toughness_mod: 0 }));
        assert!(acted() == 0 && r.nothing_else_changed());
        std::mem::forget(r);
    }
}

card_proof! {
    /// Rally the Peasants: +2/+0 until end of turn on each creature its
    /// controller controls, and nothing else.
    fn rally_the_peasants_pumps_each_of_your_creatures() {
        let r = resolve(Box::new(isd::rally_the_peasants::RallyThePeasants), &CREATURES);
        let ids = r.those(|o| creature(o) && yours(o));
        assert!(each_is(r.added(), ids.len(), |i| T::ModifyPT { target: ids[i], power_mod: 2, toughness_mod: 0 }));
        assert!(acted() == 0 && r.nothing_else_changed());
        std::mem::forget(r);
    }
}

/// Vampiric Fury's board: [`resolve_of`]'s, each of the other two objects
/// a Vampire or not as `vampires` says, with room for the four effects two
/// Vampires are given and no more. The list's next entry is written at a
/// place the solver leaves open (how many of the objects were pumped), and
/// such a write is of every byte of the list, each a choice among all of
/// them: over the board's room for eight effects of 64 bytes that was two
/// thirds of the proof's formula.
fn vampiric_fury_resolves(vampires: [bool; 2]) -> Resolved {
    const CREATURE: Kinds = Kinds { types: &[CardType::Creature], subtypes: &[], keywords: &[] };
    let (registry, mut state, blank, card) = board(Box::new(isd::vampiric_fury::VampiricFury));
    state.until_end_of_turn = Vec::with_capacity(4);
    // Each list picked by an `if`, not by `vampires.map`: an array mapped
    // goes through iterators the solver follows, and the lists came out of
    // it as values it left open, and every check of a subtype with them
    // (731 thousand steps of symbolic execution, where this is 558).
    let subtypes = |v: bool| -> &'static [&'static str] { if v { &["Vampire"] } else { &[] } };
    let others = [
        any_object_of(&mut state, blank, &CREATURE, Form::Card, subtypes(vampires[0])),
        any_object_of(&mut state, blank, &CREATURE, Form::Token, subtypes(vampires[1])),
    ];
    resolve_on(registry, state, card, others)
}

/// Vampiric Fury: +2/+0 and first strike until end of turn on each
/// Vampire creature its controller controls. With each of the other two
/// objects a Vampire or not, a constant in each harness: over subtypes the
/// solver chose, the proof ran out of memory at four and a half gigabytes.
/// That it does nothing else is the next proof's: the two in one harness
/// ran out of memory at twelve, and each alone passes at six.
fn vampiric_fury_pumps_each_of_your_vampires(vampires: [bool; 2]) {
    let r = vampiric_fury_resolves(vampires);
    let ids = r.those(|o| creature(o) && yours(o) && o.subtypes.iter().any(|s| s == "Vampire"));
    assert!(crate::proofs::cards::each_matches(r.added(), 2 * ids.len(), |i, t| if i % 2 == 0 {
        matches!(t, T::ModifyPT { target, power_mod: 2, toughness_mod: 0 } if *target == ids[i / 2])
    } else {
        matches!(t, T::GrantKeyword { target, keyword: Keyword::FirstStrike } if *target == ids[i / 2])
    }));
    std::mem::forget((r, ids));
}

case_by_case! { vampiric_fury_pumps_each_of_your_vampires;
    vampiric_fury_with_neither_other_object_a_vampire: [false, false],
    vampiric_fury_with_the_token_a_vampire: [false, true],
    vampiric_fury_with_the_card_a_vampire: [true, false],
    vampiric_fury_with_both_vampires: [true, true],
}

/// Vampiric Fury, as above: nothing changed but by the effects it adds,
/// and nothing asked for.
fn vampiric_fury_does_nothing_else(vampires: [bool; 2]) {
    let r = vampiric_fury_resolves(vampires);
    assert!(acted() == 0 && r.nothing_else_changed());
    std::mem::forget(r);
}

case_by_case! { vampiric_fury_does_nothing_else;
    vampiric_fury_does_nothing_else_with_neither_other_object_a_vampire: [false, false],
    vampiric_fury_does_nothing_else_with_the_token_a_vampire: [false, true],
    vampiric_fury_does_nothing_else_with_the_card_a_vampire: [true, false],
    vampiric_fury_does_nothing_else_with_both_vampires: [true, true],
}

card_proof! {
    /// Spare from Evil: protection from non-Human creatures until end of
    /// turn on each creature its controller controls, and nothing else.
    /// What protection from those does is `proofs::actions`'.
    fn spare_from_evil_protects_each_of_your_creatures_from_non_humans() {
        let r = resolve(Box::new(isd::spare_from_evil::SpareFromEvil), &CREATURES);
        let non_human_creatures = |f: &F| both(f, |f| matches!(f, F::HasCardType(CardType::Creature)),
            |f| matches!(f, F::Not(g) if has_subtype(g, "Human")));
        let ids = r.those(|o| creature(o) && yours(o));
        let added = r.added();
        assert!(added.len() == ids.len() && (0..ids.len()).all(|i| {
            matches!(&added[i], T::GrantProtection { target, filter } if *target == ids[i] && non_human_creatures(filter))
        }));
        assert!(acted() == 0 && r.nothing_else_changed());
        std::mem::forget(r);
    }
}

/// Past in Flames resolving over its board: an instant, a sorcery and a
/// creature card, each in the graveyard of player 1 if `theirs` says so
/// and of player 0 otherwise, and the instant granted flashback already if
/// `granted`; both constants in each harness. Over owners the solver
/// chose, the cards its controller's graveyard listed were a list it left
/// open, every cost copied from them a copy of a length it left open, and
/// the proof ran out of memory at twelve gigabytes. The game after, the
/// three cards, the instant's and the sorcery's mana costs, and the board
/// before; the effects' list has room for the three the board can hold.
struct PastInFlamesBoard {
    _registry: CardRegistry,
    state: GameState,
    objects: Vec<ObjectId>,
    costs: [crate::types::ManaCost; 2],
    before: Snapshot,
}

fn past_in_flames_resolves(theirs: [bool; 3], granted: bool) -> PastInFlamesBoard {
    // Four cards, no more: a registry's lists start empty and are given
    // room for four, and a fifth card would grow them.
    let mut registry = CardRegistry::new();
    let cards = [
        registry.register(Box::new(crate::cards::lightning_bolt::LightningBolt)),
        registry.register(Box::new(crate::cards::lava_axe::LavaAxe)),
        registry.register(Box::new(crate::cards::grizzly_bears::GrizzlyBears)),
    ];
    let past = registry.register(Box::new(isd::past_in_flames::PastInFlames));
    let mut state = super::new_game();
    state.until_end_of_turn = Vec::with_capacity(3);
    let mut objects = Vec::with_capacity(3);
    for i in 0..3 {
        let owner = PlayerId(u8::from(theirs[i]));
        let id = state.create_object(cards[i], owner, Zone::Graveyard, None, None);
        state.players[usize::from(owner.0)].graveyard_order.push(id);
        objects.push(id);
    }
    let bolt_cost = registry.get(cards[0]).expect("registered").card_data().cost.expect("a cost");
    if granted {
        state.until_end_of_turn.push(T::GrantFlashback { target: objects[0], cost: bolt_cost.clone() });
    }
    let spell = state.create_object(past, PlayerId(0), Zone::Stack, None, None);
    let before = Snapshot::of(&state);
    registry.get(past).expect("registered").on_resolve(&mut state, spell, &[], &registry);
    let costs = [bolt_cost, crate::cards::lava_axe::LavaAxe.card_data().cost.expect("a cost")];
    PastInFlamesBoard { _registry: registry, state, objects, costs, before }
}

/// Past in Flames: flashback until end of turn, at its mana cost, for each
/// instant and sorcery card in its controller's graveyard that does not
/// have it from an earlier one; no other card, and nothing else.
fn past_in_flames_gives_flashback_to_each_instant_and_sorcery_in_your_graveyard((theirs, granted): ([bool; 3], bool)) {
    let p = past_in_flames_resolves(theirs, granted);
    let granted_now = kept(&[0, 1], |i: usize| !theirs[i] && !(i == 0 && granted));
    let added = &p.state.until_end_of_turn[p.before.until_end_of_turn()..];
    // Matched by kind, not compared whole (`each_matches` says why).
    assert!(crate::proofs::cards::each_matches(added, granted_now.len(), |k, t| {
        matches!(t, T::GrantFlashback { target, cost } if *target == p.objects[granted_now[k]] && *cost == p.costs[granted_now[k]])
    }));
    assert!(acted() == 0);
    assert!(Snapshot::of(&p.state).without_until_end_of_turn() == p.before.without_until_end_of_turn());
    std::mem::forget((p, granted_now));
}

case_by_case! { past_in_flames_gives_flashback_to_each_instant_and_sorcery_in_your_graveyard;
    past_in_flames_over_your_instant_sorcery_and_creature: ([false, false, false], false),
    past_in_flames_over_your_instant_sorcery_and_creature_the_instant_with_flashback: ([false, false, false], true),
    past_in_flames_over_your_instant_and_sorcery_and_their_creature: ([false, false, true], false),
    past_in_flames_over_your_instant_and_sorcery_and_their_creature_the_instant_with_flashback: ([false, false, true], true),
    past_in_flames_over_your_instant_and_creature_and_their_sorcery: ([false, true, false], false),
    past_in_flames_over_your_instant_and_creature_and_their_sorcery_the_instant_with_flashback: ([false, true, false], true),
    past_in_flames_over_your_instant_and_their_sorcery_and_creature: ([false, true, true], false),
    past_in_flames_over_your_instant_and_their_sorcery_and_creature_the_instant_with_flashback: ([false, true, true], true),
    past_in_flames_over_your_sorcery_and_creature_and_their_instant: ([true, false, false], false),
    past_in_flames_over_your_sorcery_and_creature_and_their_instant_with_flashback: ([true, false, false], true),
    past_in_flames_over_your_sorcery_and_their_instant_and_creature: ([true, false, true], false),
    past_in_flames_over_your_sorcery_and_their_instant_with_flashback_and_creature: ([true, false, true], true),
    past_in_flames_over_your_creature_and_their_instant_and_sorcery: ([true, true, false], false),
    past_in_flames_over_your_creature_and_their_instant_with_flashback_and_sorcery: ([true, true, false], true),
    past_in_flames_over_their_instant_sorcery_and_creature: ([true, true, true], false),
    past_in_flames_over_their_instant_with_flashback_sorcery_and_creature: ([true, true, true], true),
}
