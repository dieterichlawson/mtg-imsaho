//! Continuous effects printed on a permanent attached to a creature, an
//! Aura's or an Equipment's (CR 303.4, 301.5, 613): which object each
//! reaches, and what each question that reads effects then answers.
//!
//! Every proof here is over a board of two creatures of the blank card
//! (either player's, with power and toughness from 0/1 to 3/3) and one
//! permanent of a card printing exactly the effect under proof; that
//! permanent on the battlefield or in a graveyard, and attached to the
//! first creature, the second, or nothing. The effect reaches a creature
//! exactly when its permanent is on the battlefield and attached to it
//! (CR 611.3a, 613.1).
//!
//! An effect that takes a number, a keyword or a condition is proved for
//! each one an Aura or Equipment in the pool prints, a proof for each and
//! the effect a constant in it, as a card prints it. One the solver chose
//! was not a value CBMC propagates (it propagates none holding a symbol),
//! so the effect copied into its card's `Box` was bytes it could not take
//! apart, its scope read back as every scope, and each of these proofs ran
//! out of memory.

use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{card_proof, Blank};
use crate::state::GameState;
use crate::types::{CardType, ContinuousEffect, EffectCondition, EffectScope, Keyword, Zone};

/// A card that prints exactly one continuous effect.
struct Prints(ContinuousEffect);

impl CardBehavior for Prints {
    fn card_data(&self) -> CardData {
        CardData { name: "Prints".into(), continuous_effects: vec![self.0.clone()], ..CardData::default() }
    }
}

/// The board every proof here is over.
struct Attached {
    state: GameState,
    creatures: [ObjectId; 2],
    /// Whether the effect reaches each creature.
    reaches: [bool; 2],
    /// Each creature's printed power and toughness.
    base: [(i32, i32); 2],
}

fn attached(effect: ContinuousEffect, subtypes: &[&str]) -> (Attached, CardRegistry) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let printer = registry.register(Box::new(Prints(effect)));
    let mut state = crate::proofs::cards::new_game();
    let creature = |state: &mut GameState| {
        let (p, t): (u8, u8) = (kani::any(), kani::any());
        kani::assume(p <= 3 && (1..=3).contains(&t));
        let owner = PlayerId(u8::from(kani::any::<bool>()));
        let id = state.create_object(blank, owner, Zone::Battlefield, Some(i32::from(p)), Some(i32::from(t)));
        let o = state.get_object_mut(id).expect("just created");
        o.card_types = vec![CardType::Creature];
        o.subtypes = Vec::with_capacity(subtypes.len());
        for s in subtypes {
            if kani::any() {
                o.subtypes.push((*s).to_string());
            }
        }
        (id, (i32::from(p), i32::from(t)))
    };
    let (a, base_a) = creature(&mut state);
    let (b, base_b) = creature(&mut state);
    let on = kani::any::<bool>();
    let permanent = state.create_object(printer, PlayerId(u8::from(kani::any::<bool>())),
        if on { Zone::Battlefield } else { Zone::Graveyard }, None, None);
    let host = match kani::any::<u8>() % 3 {
        0 => Some(a),
        1 => Some(b),
        _ => None,
    };
    state.get_object_mut(permanent).expect("just created").attached_to = host;
    let reaches = [on && host == Some(a), on && host == Some(b)];
    (Attached { state, creatures: [a, b], reaches, base: [base_a, base_b] }, registry)
}

/// "Enchanted creature gets +P/+T" (CR 613.4c): the creature it reaches
/// has its power and toughness changed by that much, and no other
/// creature's changes.
fn changing(p: i32, t: i32) {
    let (a, registry) = attached(ContinuousEffect::ModifyPT { power: p, toughness: t, scope: EffectScope::Attached }, &[]);
    for i in 0..2 {
        let (bp, bt) = a.base[i];
        let (dp, dt) = if a.reaches[i] { (p, t) } else { (0, 0) };
        assert!(a.state.effective_power(a.creatures[i], &registry) == Some(bp + dp));
        assert!(a.state.effective_toughness(a.creatures[i], &registry) == Some(bt + dt));
    }
    kani::cover!(a.reaches[0], "attached to the first");
    std::mem::forget((a, registry));
}

/// "Enchanted creature has <keyword>" (CR 613.1f): the creature it
/// reaches has that keyword, and no other creature has it.
fn granting(keyword: Keyword) {
    let (a, registry) = attached(ContinuousEffect::GrantKeyword { keyword, scope: EffectScope::Attached }, &[]);
    for i in 0..2 {
        assert!(a.state.has_keyword(a.creatures[i], keyword, &registry) == a.reaches[i]);
    }
    std::mem::forget((a, registry));
}

card_proof! {
    /// "Enchanted creature can't attack" (CR 508.1c): exactly the creature
    /// it reaches can't.
    fn cant_attack_reaches_the_creature_it_is_attached_to() {
        let (a, registry) = attached(ContinuousEffect::PreventAttack { scope: EffectScope::Attached }, &[]);
        for i in 0..2 {
            assert!(a.state.can_attack(a.creatures[i], &registry) == !a.reaches[i]);
        }
        std::mem::forget((a, registry));
    }
}

card_proof! {
    /// "Enchanted creature can't block" (CR 509.1b): exactly the creature
    /// it reaches can't.
    fn cant_block_reaches_the_creature_it_is_attached_to() {
        let (a, registry) = attached(ContinuousEffect::PreventBlock { scope: EffectScope::Attached }, &[]);
        for i in 0..2 {
            assert!(a.state.can_block(a.creatures[i], &registry) == !a.reaches[i]);
        }
        std::mem::forget((a, registry));
    }
}

card_proof! {
    /// "Enchanted creature attacks each combat if able" (CR 508.1d):
    /// exactly the creature it reaches must.
    fn attacks_each_combat_reaches_the_creature_it_is_attached_to() {
        let (a, registry) = attached(ContinuousEffect::ForceAttack { scope: EffectScope::Attached }, &[]);
        for i in 0..2 {
            assert!(a.state.must_attack(a.creatures[i], &registry) == a.reaches[i]);
        }
        std::mem::forget((a, registry));
    }
}

card_proof! {
    /// "Enchanted creature doesn't untap during its controller's untap step"
    /// (CR 502.3): exactly the creature it reaches doesn't.
    fn doesnt_untap_reaches_the_creature_it_is_attached_to() {
        let (a, registry) = attached(ContinuousEffect::PreventUntap { scope: EffectScope::Attached }, &[]);
        for i in 0..2 {
            assert!(a.state.untaps_normally(a.creatures[i], &registry) == !a.reaches[i]);
        }
        std::mem::forget((a, registry));
    }
}

/// "As long as it's a Human" / "Otherwise" (CR 611.3a): an effect with a
/// condition on the enchanted creature's subtype reaches it exactly when
/// the condition holds, Human or not, read from the creature as it is.
fn conditioned(lacks: bool) {
    let condition = if lacks {
        EffectCondition::AttachedLacksSubtype("Human".into())
    } else {
        EffectCondition::AttachedHasSubtype("Human".into())
    };
    let (a, registry) = attached(ContinuousEffect::when(condition, ContinuousEffect::PreventAttack { scope: EffectScope::Attached }), &["Human"]);
    for i in 0..2 {
        let human = a.state.get_object(a.creatures[i]).expect("on the board").subtypes.iter().any(|s| s == "Human");
        assert!(a.state.can_attack(a.creatures[i], &registry) == !(a.reaches[i] && human != lacks));
    }
    std::mem::forget((a, registry));
}

/// A proof above for each effect the pool's Auras and Equipment print with
/// a number, a keyword or a condition.
macro_rules! each_printed {
    ($($name:ident: $proof:ident($($arg:expr),*)),* $(,)?) => {$(
        card_proof! {
            /// The proof above it, for one effect a card in the pool prints.
            fn $name() {
                $proof($($arg),*);
            }
        }
    )*};
}

each_printed! {
    plus_2_plus_2_reaches_the_creature_it_is_attached_to: changing(2, 2),
    plus_1_plus_2_reaches_the_creature_it_is_attached_to: changing(1, 2),
    plus_1_plus_1_reaches_the_creature_it_is_attached_to: changing(1, 1),
    plus_1_plus_0_reaches_the_creature_it_is_attached_to: changing(1, 0),
    plus_2_plus_0_reaches_the_creature_it_is_attached_to: changing(2, 0),
    plus_3_plus_0_reaches_the_creature_it_is_attached_to: changing(3, 0),
    plus_4_plus_2_reaches_the_creature_it_is_attached_to: changing(4, 2),
    minus_2_minus_2_reaches_the_creature_it_is_attached_to: changing(-2, -2),
    minus_3_plus_0_reaches_the_creature_it_is_attached_to: changing(-3, 0),
    flying_reaches_the_creature_it_is_attached_to: granting(Keyword::Flying),
    first_strike_reaches_the_creature_it_is_attached_to: granting(Keyword::FirstStrike),
    hexproof_reaches_the_creature_it_is_attached_to: granting(Keyword::Hexproof),
    intimidate_reaches_the_creature_it_is_attached_to: granting(Keyword::Intimidate),
    lifelink_reaches_the_creature_it_is_attached_to: granting(Keyword::Lifelink),
    as_long_as_its_a_human_holds_exactly_when_it_is: conditioned(false),
    unless_its_a_human_holds_exactly_when_it_is_not: conditioned(true),
}
