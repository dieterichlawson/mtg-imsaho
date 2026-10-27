//! The spells that read the board as they resolve: each creature, each
//! enchantment, each card in a graveyard or a library that their text
//! names, and only those.
//!
//! - Hysterical Blindness, {2}{U} Instant: "Creatures your opponents
//!   control get -4/-0 until end of turn."
//! - Rally the Peasants, {2}{W} Instant: "Creatures you control get +2/+0
//!   until end of turn. Flashback {2}{R}"
//! - Vampiric Fury, {1}{R} Instant: "Vampire creatures you control get
//!   +2/+0 and gain first strike until end of turn."
//! - Spare from Evil, {W} Instant: "Creatures you control gain protection
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

use super::{act, acted, any_object, asked_for, board, both, card_proof, each_is, either, has_subtype, new_ids, Act, Blank, Form, Kinds, Snapshot};
use crate::cards::{AdditionalCost, CardBehavior, CardRegistry};
use crate::damage::DamageKind;
use crate::destruction::DestroyResult;
use crate::events::DamageTarget;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{GameObject, GameState, TemporaryEffect as T};
use crate::types::{CardType, Color, CreatureFilter as F, Keyword, ManaType, Zone};

use crate::cards::isd;

/// A spell resolving on the board above: the game after, the spell, the
/// two objects, and the board before.
struct Resolved {
    _registry: CardRegistry,
    state: GameState,
    spell: ObjectId,
    others: [ObjectId; 2],
    before: Snapshot,
}

impl Resolved {
    /// The objects, in the game's order, that `pred` holds for as they
    /// were when the spell began to resolve: the recorders change nothing,
    /// so as they are now.
    fn those(&self, pred: impl Fn(&GameObject) -> bool) -> Vec<ObjectId> {
        self.others.iter().copied()
            .filter(|id| pred(self.state.get_object(*id).expect("on the board")))
            .collect()
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
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let before = Snapshot::of(&state);
    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);
    Resolved { _registry: registry, state, spell, others, before }
}

fn creature(o: &GameObject) -> bool {
    o.zone == Zone::Battlefield && o.card_types.contains(&CardType::Creature)
}

fn yours(o: &GameObject) -> bool {
    o.controller == PlayerId(0)
}

/// A creature card in player 0's graveyard (CR 404.1: a graveyard is its
/// owner's).
fn creature_card_in_your_graveyard(o: &GameObject) -> bool {
    o.zone == Zone::Graveyard && o.owner == PlayerId(0) && !o.is_token && o.card_types.contains(&CardType::Creature)
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

card_proof! {
    /// Vampiric Fury: +2/+0 and first strike until end of turn on each
    /// Vampire creature its controller controls, and nothing else.
    fn vampiric_fury_pumps_each_of_your_vampires() {
        const VAMPIRES: Kinds = Kinds { types: &[CardType::Creature], subtypes: &["Vampire"], keywords: &[] };
        let r = resolve(Box::new(isd::vampiric_fury::VampiricFury), &VAMPIRES);
        let ids = r.those(|o| creature(o) && yours(o) && o.subtypes.iter().any(|s| s == "Vampire"));
        assert!(each_is(r.added(), 2 * ids.len(), |i| if i % 2 == 0 {
            T::ModifyPT { target: ids[i / 2], power_mod: 2, toughness_mod: 0 }
        } else {
            T::GrantKeyword { target: ids[i / 2], keyword: Keyword::FirstStrike }
        }));
        assert!(acted() == 0 && r.nothing_else_changed());
        std::mem::forget(r);
    }
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

card_proof! {
    /// Rolling Temblor: 2 damage from the spell to each creature without
    /// flying, all of it dealt at once (CR 120.1, queued and then dealt),
    /// and nothing else.
    fn rolling_temblor_deals_two_to_each_creature_without_flying() {
        const FLIERS: Kinds = Kinds { types: &[CardType::Creature, CardType::Enchantment], subtypes: &[], keywords: &[Keyword::Flying] };
        let r = resolve(Box::new(isd::rolling_temblor::RollingTemblor), &FLIERS);
        let mut expected: Vec<Act> = r.those(|o| creature(o) && !o.keywords.contains(&Keyword::Flying)).into_iter()
            .map(|id| Act::Queue { source: r.spell, target: DamageTarget::Object(id), amount: 2, kind: DamageKind::NonCombat })
            .collect();
        expected.push(Act::DealQueued);
        assert!(asked_for(&expected) && r.added().is_empty() && r.nothing_else_changed());
        std::mem::forget(r);
    }
}

card_proof! {
    /// Moonmist: each Human on the battlefield transformed (whether it can
    /// be is the action's, CR 701.28c), then combat damage prevented this
    /// turn from every creature but Wolves and Werewolves, and nothing else.
    fn moonmist_transforms_humans_and_spares_werewolves_and_wolves() {
        const HUMANS: Kinds = Kinds { types: &[CardType::Creature], subtypes: &["Human", "Wolf"], keywords: &[] };
        let r = resolve(Box::new(isd::moonmist::Moonmist), &HUMANS);
        let transformed: Vec<Act> = r.those(|o| o.zone == Zone::Battlefield && o.subtypes.iter().any(|s| s == "Human")).into_iter()
            .map(Act::Transform).collect();
        let prevents = matches!(r.added(), [T::PreventCombatDamageExcept { filter, source_name }]
            if either(filter, |f| has_subtype(f, "Wolf"), |f| has_subtype(f, "Werewolf")) && source_name == "Moonmist");
        assert!(asked_for(&transformed) && prevents && r.nothing_else_changed());
        std::mem::forget(r);
    }
}

card_proof! {
    /// Paraselene: every enchantment destroyed at once, as one event, then
    /// 1 life for each that was destroyed, which is each one that died,
    /// whatever the others did; and nothing else.
    fn paraselene_destroys_all_enchantments_and_gains_a_life_for_each() {
        const ENCHANTMENTS: Kinds = Kinds { types: &[CardType::Enchantment, CardType::Creature], subtypes: &[], keywords: &[] };
        let r = resolve(Box::new(isd::paraselene::Paraselene), &ENCHANTMENTS);
        let enchantments = r.those(|o| o.zone == Zone::Battlefield && o.card_types.contains(&CardType::Enchantment));
        let Some(Act::DestroyAll(outcomes)) = act(0) else { panic!("destroys first") };
        assert!(outcomes.iter().map(|(id, _)| *id).eq(enchantments.iter().copied()));
        let died = outcomes.iter().filter(|(_, o)| *o == DestroyResult::Died).count();
        if died > 0 {
            assert!(acted() == 2 && act(1) == Some(Act::Life(PlayerId(0), i32::try_from(died).expect("small"))));
        } else {
            assert!(acted() == 1);
        }
        assert!(r.added().is_empty() && r.nothing_else_changed());
        std::mem::forget(r);
    }
}

card_proof! {
    /// Gnaw to the Bone: 2 life for each creature card in its controller's
    /// graveyard; with none, nothing.
    fn gnaw_to_the_bone_gains_two_for_each_creature_card_in_your_graveyard() {
        let r = resolve(Box::new(isd::gnaw_to_the_bone::GnawToTheBone), &CREATURES);
        let n = i32::try_from(r.those(creature_card_in_your_graveyard).len()).expect("small");
        if n > 0 {
            assert!(asked_for(&[Act::Life(PlayerId(0), 2 * n)]));
        } else {
            assert!(acted() == 0);
        }
        assert!(r.added().is_empty() && r.nothing_else_changed());
        std::mem::forget(r);
    }
}

card_proof! {
    /// Spider Spawning: a 1/2 green Spider creature token with reach for
    /// each creature card in its controller's graveyard, under its
    /// controller's control, as one event; and nothing else.
    fn spider_spawning_makes_a_spider_for_each_creature_card_in_your_graveyard() {
        let r = resolve(Box::new(isd::spider_spawning::SpiderSpawning), &CREATURES);
        let n = u32::try_from(r.those(creature_card_in_your_graveyard).len()).expect("small");
        assert!(asked_for(&[Act::Tokens {
            n, controller: PlayerId(0), power: 1, toughness: 2, colors: vec![Color::Green],
            types: vec![CardType::Creature], keywords: vec![Keyword::Reach], subtypes: vec!["Spider".into()],
        }]));
        assert!(r.added().is_empty() && r.nothing_else_changed());
        std::mem::forget(r);
    }
}

card_proof! {
    /// Make a Wish: two of the cards in its controller's graveyard (all of
    /// them, if fewer) chosen at random and returned to their hand, and
    /// nothing else.
    fn make_a_wish_returns_two_at_random() {
        let r = resolve(Box::new(isd::make_a_wish::MakeAWish), &CREATURES);
        let graveyard = r.those(|o| o.zone == Zone::Graveyard && o.owner == PlayerId(0));
        assert!(act(0) == Some(Act::Random(graveyard.clone(), 2)));
        assert!(acted() == 1 + graveyard.len().min(2));
        for i in 1..acted() {
            let Some(Act::Move(id, Zone::Hand)) = act(i) else { panic!("returns to the hand") };
            assert!(graveyard.contains(&id));
            assert!(i == 1 || act(1) != act(i));
        }
        assert!(r.added().is_empty() && r.nothing_else_changed());
        std::mem::forget(r);
    }
}

card_proof! {
    /// Infernal Plunge: sacrificing a creature is its additional cost, paid
    /// as it is cast (CR 601.2h); it adds {R}{R}{R} to its controller's
    /// pool, and nothing else.
    fn infernal_plunge_adds_three_red() {
        assert!(isd::infernal_plunge::InfernalPlunge.card_data().additional_cost == Some(AdditionalCost::SacrificeCreature));
        let r = resolve(Box::new(isd::infernal_plunge::InfernalPlunge), &CREATURES);
        assert!(asked_for(&[Act::Mana(PlayerId(0), ManaType::Red, 3)]));
        assert!(r.added().is_empty() && r.nothing_else_changed());
        std::mem::forget(r);
    }
}

card_proof! { unwind(15)
    /// Army of the Damned: thirteen 2/2 black Zombie creature tokens under
    /// its controller's control, as one event, each put onto the
    /// battlefield tapped; and nothing else.
    fn army_of_the_damned_makes_thirteen_tapped_zombies() {
        let (registry, mut state, _, card) = board(Box::new(isd::army_of_the_damned::ArmyOfTheDamned));
        let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
        let before = Snapshot::of(&state);
        registry.get(card).expect("registered").on_resolve(&mut state, spell, &[], &registry);
        let mut expected = vec![Act::Tokens {
            n: 13, controller: PlayerId(0), power: 2, toughness: 2, colors: vec![Color::Black],
            types: vec![CardType::Creature], keywords: vec![], subtypes: vec!["Zombie".into()],
        }];
        expected.push(Act::ArrivesTapped { first: new_ids(&state, 1)[0], count: 13 });
        assert!(asked_for(&expected) && Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}

/// A board for the spells that read a library or a hand: the spell, cast
/// by player 0, and up to `library` cards in player 0's library and up to
/// `hand` in their hand, each a land or not.
fn cards_in_zones(card: Box<dyn CardBehavior>, library: usize, hand: usize) -> (CardRegistry, GameState, ObjectId, CardId) {
    let (registry, mut state, blank, card) = board(card);
    let (in_library, in_hand): (usize, usize) = (kani::any(), kani::any());
    kani::assume(in_library <= library && in_hand <= hand);
    state.players[0].library_order = Vec::with_capacity(library);
    for i in 0..library + hand {
        if (i < library && i >= in_library) || (i >= library && i - library >= in_hand) {
            continue;
        }
        let zone = if i < library { Zone::Library } else { Zone::Hand };
        let id = state.create_object(blank, PlayerId(0), zone, None, None);
        let o = state.get_object_mut(id).expect("just created");
        o.card_types = Vec::with_capacity(1);
        if kani::any() {
            o.card_types.push(CardType::Land);
        }
        if zone == Zone::Library {
            state.players[0].library_order.push(id);
        }
    }
    let spell = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    (registry, state, spell, blank)
}

card_proof! {
    /// Desperate Ravings: two cards drawn, then one card of the hand as it
    /// is after the draw discarded, chosen at random; with that hand empty,
    /// nothing discarded; and nothing else.
    fn desperate_ravings_draws_two_then_discards_at_random() {
        let (registry, mut state, spell, _) = cards_in_zones(Box::new(isd::desperate_ravings::DesperateRavings), 2, 1);
        registry.get(state.get_object(spell).expect("on the stack").card_id).expect("registered")
            .on_resolve(&mut state, spell, &[], &registry);
        let hand: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, PlayerId(0)).iter().map(|o| o.id).collect();
        assert!(act(0) == Some(Act::Draw(PlayerId(0), 2)) && act(1) == Some(Act::Random(hand.clone(), 1)));
        if hand.is_empty() {
            assert!(acted() == 2);
        } else {
            let Some(Act::Discard(id)) = act(2) else { panic!("discards") };
            assert!(acted() == 3 && hand.contains(&id));
        }
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Mulch: of the top four cards of its controller's library (all of
    /// them, if fewer), the lands put into their hand and the rest milled
    /// into their graveyard, in library order; and nothing else. Proved for
    /// libraries of up to five cards, so that one is past the fourth.
    fn mulch_puts_the_lands_of_the_top_four_in_hand_and_the_rest_in_the_graveyard() {
        let (registry, mut state, spell, _) = cards_in_zones(Box::new(isd::mulch::Mulch), 5, 0);
        let before = Snapshot::of(&state);
        let top: Vec<ObjectId> = state.players[0].library_order.iter().take(4).copied().collect();
        registry.get(state.get_object(spell).expect("on the stack").card_id).expect("registered")
            .on_resolve(&mut state, spell, &[], &registry);
        let land = |id: &ObjectId| state.get_object(*id).expect("on the board").card_types.contains(&CardType::Land);
        let mut expected: Vec<Act> = top.iter().filter(|id| land(id)).map(|id| Act::Move(*id, Zone::Hand)).collect();
        expected.extend(top.iter().filter(|id| !land(id)).map(|id| Act::MillCard(*id)));
        assert!(asked_for(&expected) && Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Past in Flames: flashback until end of turn, at its mana cost, for
    /// each instant and sorcery card in its controller's graveyard that
    /// does not have it from an earlier one; no other card, and nothing
    /// else. The board: an instant, a sorcery and a creature card, each in
    /// either player's graveyard, the instant perhaps granted flashback
    /// already.
    fn past_in_flames_gives_flashback_to_each_instant_and_sorcery_in_your_graveyard() {
        let mut registry = CardRegistry::new();
        let _blank = registry.register(Box::new(Blank));
        let cards = [
            registry.register(Box::new(crate::cards::lightning_bolt::LightningBolt)),
            registry.register(Box::new(crate::cards::lava_axe::LavaAxe)),
            registry.register(Box::new(crate::cards::grizzly_bears::GrizzlyBears)),
        ];
        let past = registry.register(Box::new(isd::past_in_flames::PastInFlames));
        let mut state = super::new_game();
        let mut objects = Vec::with_capacity(3);
        for card in cards {
            let owner: bool = kani::any();
            let id = state.create_object(card, PlayerId(u8::from(owner)), Zone::Graveyard, None, None);
            if owner {
                state.players[1].graveyard_order.push(id);
            } else {
                state.players[0].graveyard_order.push(id);
            }
            objects.push(id);
        }
        let granted: bool = kani::any();
        let bolt_cost = registry.get(cards[0]).expect("registered").card_data().cost.expect("a cost");
        if granted {
            state.until_end_of_turn.push(T::GrantFlashback { target: objects[0], cost: bolt_cost.clone() });
        }
        let spell = state.create_object(past, PlayerId(0), Zone::Stack, None, None);
        let before = Snapshot::of(&state);
        registry.get(past).expect("registered").on_resolve(&mut state, spell, &[], &registry);
        let granted_now: Vec<usize> = (0..2)
            .filter(|i| state.get_object(objects[*i]).expect("on the board").owner == PlayerId(0) && !(*i == 0 && granted))
            .collect();
        let costs = [bolt_cost, crate::cards::lava_axe::LavaAxe.card_data().cost.expect("a cost")];
        let added = &state.until_end_of_turn[before.until_end_of_turn()..];
        assert!(each_is(added, granted_now.len(), |k| T::GrantFlashback { target: objects[granted_now[k]], cost: costs[granted_now[k]].clone() }));
        assert!(acted() == 0);
        assert!(Snapshot::of(&state).without_until_end_of_turn() == before.without_until_end_of_turn());
        std::mem::forget((registry, state));
    }
}
