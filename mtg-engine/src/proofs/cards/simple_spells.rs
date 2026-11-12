//! The spells whose resolution is one action the engine performs for its
//! controller or its target: draw cards, mill cards, create tokens, counter
//! a spell.
//!
//! - Think Twice, {1}{U} Instant: "Draw a card. Flashback {2}{U}"
//! - Divination, {2}{U} Sorcery: "Draw two cards."
//! - Altar's Reap, {1}{B} Instant: "As an additional cost to cast this
//!   spell, sacrifice a creature. Draw two cards."
//! - Dream Twist, {U} Instant: "Target player mills three cards. Flashback
//!   {1}{U}"
//! - Midnight Haunting, {2}{W} Instant: "Create two 1/1 white Spirit
//!   creature tokens with flying."
//! - Moan of the Unhallowed, {2}{B}{B} Sorcery: "Create two 2/2 black
//!   Zombie creature tokens. Flashback {5}{B}{B}"
//! - Counterspell, {U}{U} Instant: "Counter target spell."
//! - Dissipate, {1}{U}{U} Instant: "Counter target spell. If that spell is
//!   countered this way, exile it instead of putting it into its owner's
//!   graveyard."
//!
//! "You" is the spell's controller (CR 109.5); each proof is for either
//! player casting it. The additional cost is the engine's casting, pinned
//! to each card by its data; drawing, milling, creating tokens and
//! countering are the engine's actions, proved in `proofs::actions`.

use super::{acted, any_object, asked_for, board, card_proof, Act, Form, Snapshot, ANY_PERMANENT};
use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::types::{CardType, Color, Keyword, Zone};

/// `card`, cast by either player, resolving holding `target` (picked from
/// the other object on the board and the spell): what it asked for, checked
/// by `asked`, given its caster and target; and nothing else changed.
fn resolves(card: Box<dyn CardBehavior>, target: fn(ObjectId, ObjectId) -> Target, asked: fn(PlayerId, &Target) -> bool) {
    let (registry, mut state, blank, card) = board(card);
    let other = any_object(&mut state, blank, &ANY_PERMANENT, Form::Card);
    let caster = PlayerId(u8::from(kani::any::<bool>()));
    let spell = state.create_object(card, caster, Zone::Stack, None, None);
    let target = target(other, spell);
    let before = Snapshot::of(&state);
    registry.get(card).expect("registered").on_resolve(&mut state, spell, &[target.clone()], &registry);
    assert!(asked(caster, &target));
    assert!(Snapshot::of(&state) == before);
    std::mem::forget((registry, state));
}

/// Either player, or a target gone illegal.
fn any_player_target(_: ObjectId, _: ObjectId) -> Target {
    match kani::any::<u8>() % 3 {
        0 => Target::Player(PlayerId(0)),
        1 => Target::Player(PlayerId(1)),
        _ => Target::Illegal,
    }
}

/// The other object, which may be a spell or not, or a target gone illegal.
fn object_or_illegal(other: ObjectId, _: ObjectId) -> Target {
    if kani::any() { Target::Object(other) } else { Target::Illegal }
}

fn no_target(_: ObjectId, _: ObjectId) -> Target {
    Target::Illegal
}

card_proof! {
    /// Think Twice: its controller draws one card (CR 121.1).
    fn think_twice_draws_one_card() {
        resolves(Box::new(crate::cards::isd::think_twice::ThinkTwice), no_target, |caster, _| asked_for(&[Act::Draw(caster, 1)]));
    }
}

card_proof! {
    /// Divination: its controller draws two cards.
    fn divination_draws_two_cards() {
        resolves(Box::new(crate::cards::divination::Divination), no_target, |caster, _| asked_for(&[Act::Draw(caster, 2)]));
    }
}

card_proof! {
    /// Altar's Reap: its controller draws two cards; sacrificing a creature
    /// is its additional cost (CR 601.2f), which its data names.
    fn altars_reap_draws_two_cards_for_a_sacrifice() {
        assert!(crate::cards::isd::altars_reap::AltarsReap.card_data().additional_cost == Some(AdditionalCost::SacrificeCreature));
        resolves(Box::new(crate::cards::isd::altars_reap::AltarsReap), no_target, |caster, _| asked_for(&[Act::Draw(caster, 2)]));
    }
}

card_proof! {
    /// Dream Twist: "target player" (CR 115.1), and that player mills three
    /// cards (701.17a); with its target gone illegal, nothing.
    fn dream_twist_mills_its_target_three() {
        assert!(crate::cards::isd::dream_twist::DreamTwist.target_requirement() == TargetRequirement::PlayerOnly);
        resolves(Box::new(crate::cards::isd::dream_twist::DreamTwist), any_player_target, |_, target| match target {
            Target::Player(p) => asked_for(&[Act::Mill(*p, 3)]),
            _ => acted() == 0,
        });
    }
}

card_proof! {
    /// Midnight Haunting: its controller creates two 1/1 white Spirit
    /// creature tokens with flying (CR 111.1).
    fn midnight_haunting_creates_two_flying_spirits() {
        resolves(Box::new(crate::cards::isd::midnight_haunting::MidnightHaunting), no_target, |caster, _| asked_for(&[Act::Tokens {
            n: 2, controller: caster, power: 1, toughness: 1, colors: vec![Color::White], types: vec![CardType::Creature],
            keywords: vec![Keyword::Flying], subtypes: vec!["Spirit".into()],
        }]));
    }
}

card_proof! {
    /// Moan of the Unhallowed: its controller creates two 2/2 black Zombie
    /// creature tokens.
    fn moan_of_the_unhallowed_creates_two_zombies() {
        resolves(Box::new(crate::cards::isd::moan_of_the_unhallowed::MoanOfTheUnhallowed), no_target, |caster, _| asked_for(&[Act::Tokens {
            n: 2, controller: caster, power: 2, toughness: 2, colors: vec![Color::Black], types: vec![CardType::Creature],
            keywords: vec![], subtypes: vec!["Zombie".into()],
        }]));
    }
}

card_proof! {
    /// Counterspell: "target spell" (CR 115.1), and that spell countered
    /// (701.6a); with its target gone illegal, nothing.
    fn counterspell_counters_its_target() {
        assert!(crate::cards::counterspell::Counterspell.target_requirement() == TargetRequirement::Spell);
        resolves(Box::new(crate::cards::counterspell::Counterspell), object_or_illegal, |_, target| match target {
            Target::Object(id) => asked_for(&[Act::Counter { spell: *id, exile: false }]),
            _ => acted() == 0,
        });
    }
}

card_proof! {
    /// Dissipate: "target spell", and that spell countered and exiled
    /// instead of put into its owner's graveyard (CR 701.6a, 614.1a); with
    /// its target gone illegal, nothing.
    fn dissipate_counters_and_exiles_its_target() {
        assert!(crate::cards::isd::dissipate::Dissipate.target_requirement() == TargetRequirement::Spell);
        resolves(Box::new(crate::cards::isd::dissipate::Dissipate), object_or_illegal, |_, target| match target {
            Target::Object(id) => asked_for(&[Act::Counter { spell: *id, exile: true }]),
            _ => acted() == 0,
        });
    }
}
