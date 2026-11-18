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

use crate::proofs::held::Held;
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
    /// The permanent printing the effect.
    permanent: ObjectId,
}

fn attached(effect: ContinuousEffect, subtypes: &[&str]) -> (Attached, CardRegistry) {
    attached_printing(Box::new(Prints(effect)), subtypes)
}

/// A card that prints exactly one continuous effect, the one its function
/// builds where it is printed: an effect that holds a filter, held and
/// cloned, came back from the clone as one symbolic execution could not
/// tell from any other, and matching against it was matching every filter
/// there is (as `scopes::Prints` says).
struct Builds(fn() -> ContinuousEffect);

impl CardBehavior for Builds {
    fn card_data(&self) -> CardData {
        CardData { name: "Builds".into(), continuous_effects: vec![(self.0)()], ..CardData::default() }
    }
}

/// The board, its permanent of the card `printer`.
fn attached_printing(printer: Box<dyn CardBehavior>, subtypes: &[&str]) -> (Attached, CardRegistry) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let printer = registry.register(printer);
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
    (Attached { state, creatures: [a, b], reaches, base: [base_a, base_b], permanent }, registry)
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

card_proof! {
    /// "Equipped creature can't be blocked" (CR 509.1b), as Invisible
    /// Stalker's own "can't be blocked": exactly the creature it reaches
    /// can't be.
    fn cant_be_blocked_reaches_the_creature_it_is_attached_to() {
        let (a, registry) = attached(ContinuousEffect::CantBeBlocked { scope: EffectScope::Attached }, &[]);
        for i in 0..2 {
            assert!(a.state.cant_be_blocked(a.creatures[i], &registry) == a.reaches[i]);
        }
        std::mem::forget((a, registry));
    }
}

/// Points `filters::stand_in` at the filter the board's permanent prints.
fn print_filter(a: &Attached, registry: &CardRegistry) {
    let card = a.state.get_object(a.permanent).expect("on the board").card_id;
    let ContinuousEffect::CanOnlyBeBlockedBy { allowed_blockers, .. } = &registry.printed_continuous_effects(card, false)[0]
    else {
        panic!("the permanent prints the restriction")
    };
    super::filters::asked_of(allowed_blockers);
}

/// "Can't be blocked except by <filter>" (CR 509.1b): of the two
/// creatures, either attacking and each with flying, reach, both or
/// neither (as [`flying_or_reach`] answers), the other may block it
/// exactly when the effect does not reach the attacker or the blocker
/// matches the effect's filter, and the attacker has no flying or the
/// blocker has flying or reach (702.9b). On this board nothing else
/// restricts a block: no creature has intimidate or protection, and each
/// is an untapped creature that can block.
fn only_blocked_by(printed: fn() -> ContinuousEffect) {
    let (a, registry) = attached_printing(Box::new(Builds(printed)), &[]);
    assert!(a.creatures == [ObjectId(1), ObjectId(2)]);
    print_filter(&a, &registry);
    let attacker = usize::from(kani::any::<bool>());
    let (at, bl) = (a.creatures[attacker], a.creatures[1 - attacker]);
    let can = crate::combat::can_block_attacker(&a.state, bl, at, &registry);
    let up = flying_or_reach(&a.state, bl, Keyword::Flying, &registry) || flying_or_reach(&a.state, bl, Keyword::Reach, &registry);
    let flies = flying_or_reach(&a.state, at, Keyword::Flying, &registry);
    assert!(can == ((!a.reaches[attacker] || super::filters::matches(bl)) && (!flies || up)));
    std::mem::forget((a, registry));
}

/// The proofs of "can't be blocked except by", the filter standing in as
/// above.
macro_rules! each_restriction {
    ($($name:ident: $printed:expr,)*) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::has_keyword, flying_or_reach)]
        #[kani::stub(crate::state::GameState::matches_filter, super::filters::stand_in)]
        /// [`only_blocked_by`] for the restriction of its name.
        fn $name() {
            only_blocked_by($printed);
        }
    )*};
}

each_restriction! {
    // Stromkirk Noble's "can't be blocked by Humans".
    cant_be_blocked_by_humans_reaches_the_creature_it_is_on: || ContinuousEffect::CanOnlyBeBlockedBy {
        allowed_blockers: crate::types::CreatureFilter::Not(Box::new(crate::types::CreatureFilter::HasSubtype("Human".into()))),
        scope: EffectScope::Attached,
    },
    // Blazing Torch's "can't be blocked by Vampires or Zombies".
    cant_be_blocked_by_vampires_or_zombies_reaches_the_creature_it_is_attached_to: || ContinuousEffect::CanOnlyBeBlockedBy {
        allowed_blockers: crate::types::CreatureFilter::Not(Box::new(crate::types::CreatureFilter::Or(vec![
            crate::types::CreatureFilter::HasSubtype("Vampire".into()),
            crate::types::CreatureFilter::HasSubtype("Zombie".into()),
        ]))),
        scope: EffectScope::Attached,
    },
}

/// Whether each of the two creatures has flying and reach, as
/// [`flying_or_reach`] answers: chosen once, the same after.
static mut WINGS: Held<[[Option<bool>; 2]; 2]> = Held::new([[None; 2]; 2], (&raw const WINGS).cast());

/// Stands in for `GameState::has_keyword` on the board of
/// [`blocked_only_by_flying_or_reach`]: flying and reach any answer for
/// each creature, the same each time; anything else none, as the blank
/// card prints none (`predicates::an_objects_keywords_are_its_tokens_or_its_cards`).
#[allow(dead_code)] // named only by `kani::stub`
fn flying_or_reach(_: &GameState, id: ObjectId, keyword: Keyword, _: &CardRegistry) -> bool {
    let k = match keyword {
        Keyword::Flying => 0,
        Keyword::Reach => 1,
        _ => return false,
    };
    // The creatures are the board's first two objects, ids 1 and 2.
    let i = match id.0 {
        1 => 0,
        2 => 1,
        _ => return false,
    };
    // SAFETY: a proof runs on one thread, and only this function names WINGS.
    let wings = unsafe { &mut *std::ptr::addr_of_mut!(WINGS.value) };
    *wings[i][k].get_or_insert_with(kani::any::<bool>)
}

#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::has_keyword, flying_or_reach)]
#[kani::stub(crate::state::GameState::matches_filter, super::filters::stand_in)]
/// "Can't be blocked except by creatures with flying or reach" (CR
/// 509.1b), Orchard Spirit's: of the two creatures, either attacking and
/// each with flying, reach, both or neither, the other may block it
/// exactly when the effect does not reach the attacker or the blocker
/// matches the effect's filter (which is having flying or reach,
/// `filters`), and the attacker has no flying or the blocker has flying
/// or reach (702.9b).
fn blocked_only_by_flying_or_reach() {
    use crate::types::CreatureFilter as F;
    let (a, registry) = attached_printing(Box::new(Builds(|| ContinuousEffect::CanOnlyBeBlockedBy {
        allowed_blockers: F::Or(vec![F::HasKeyword(Keyword::Flying), F::HasKeyword(Keyword::Reach)]),
        scope: EffectScope::Attached,
    })), &[]);
    assert!(a.creatures == [ObjectId(1), ObjectId(2)]);
    print_filter(&a, &registry);
    let attacker = usize::from(kani::any::<bool>());
    let (at, bl) = (a.creatures[attacker], a.creatures[1 - attacker]);
    let can = crate::combat::can_block_attacker(&a.state, bl, at, &registry);
    let up = flying_or_reach(&a.state, bl, Keyword::Flying, &registry) || flying_or_reach(&a.state, bl, Keyword::Reach, &registry);
    let flies = flying_or_reach(&a.state, at, Keyword::Flying, &registry);
    assert!(can == ((!a.reaches[attacker] || super::filters::matches(bl)) && (!flies || up)));
    std::mem::forget((a, registry));
}

/// Stands in for `GameState::has_protection_from`: nothing on these
/// boards has protection (`predicates::nothing_has_protection_on_a_board_that_grants_none`).
#[allow(dead_code)] // named only by `kani::stub`
fn protected_from_nothing(_: &GameState, _: ObjectId, _: ObjectId, _: &CardRegistry) -> bool {
    false
}

/// Damage from either creature to the other or to either player, combat
/// damage or not, and the effects that apply to it.
fn damage_on(a: &Attached, registry: &CardRegistry) -> (usize, Option<usize>, bool, Vec<crate::damage::DamageEffect>) {
    damage_of(a, registry, kani::any(), kani::any())
}

/// [`damage_on`] for damage combat or not as `combat` says, to the other
/// creature or to either player as `to_creature` says.
fn damage_of(a: &Attached, registry: &CardRegistry, combat: bool, to_creature: bool)
    -> (usize, Option<usize>, bool, Vec<crate::damage::DamageEffect>)
{
    use crate::damage::{DamageKind, PendingDamage};
    use crate::events::DamageTarget;
    let source = usize::from(kani::any::<bool>());
    let target = if to_creature {
        DamageTarget::Object(a.creatures[1 - source])
    } else {
        DamageTarget::Player(PlayerId(u8::from(kani::any::<bool>())))
    };
    let pd = PendingDamage {
        source: a.creatures[source], target, amount: 2,
        kind: if combat { DamageKind::Combat } else { DamageKind::NonCombat },
        applied: Vec::new(), settled: false,
    };
    let effects = crate::damage::applicable_effects(&a.state, &pd, registry);
    std::mem::forget(pd);
    (source, to_creature.then_some(1 - source), combat, effects)
}

/// The proofs of what applies to damage, with nothing protected.
macro_rules! damage_proof {
    ($(#[$meta:meta])* fn $name:ident() $body:block) => {
        $(#[$meta])*
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::has_protection_from, protected_from_nothing)]
        fn $name() $body
    };
}

/// "Prevent all combat damage that would be dealt to and dealt by
/// enchanted creature" (CR 615.1, Ghostly Possession): the one effect that
/// applies to damage (616.1) is that prevention, named for its permanent,
/// exactly when the damage is combat damage and the creature it reaches
/// deals it or is dealt it; to any other damage, none applies. Damage
/// combat or not and to a creature or a player as `combat` and
/// `to_creature` say, a constant in each harness: left to the solver, the
/// formula ran the solver out of memory at 7 GB.
fn preventing_combat_damage(combat: bool, to_creature: bool) {
    use crate::damage::{DamageEffect, Prevention};
    let (a, registry) = attached(ContinuousEffect::PreventCombatDamage { scope: EffectScope::Attached }, &[]);
    let (source, target, _, effects) = damage_of(&a, &registry, combat, to_creature);
    if combat && (a.reaches[source] || target.is_some_and(|t| a.reaches[t])) {
        assert!(matches!(effects.as_slice(), [DamageEffect::PreventAll(Prevention::Permanent(p))] if *p == a.permanent));
    } else {
        assert!(effects.is_empty());
    }
    std::mem::forget((a, registry, effects));
}

/// "If equipped creature would deal combat damage, it deals double that
/// damage instead" (CR 614.1a, Inquisitor's Flail, which doubles what its
/// creature is dealt as well): the one effect that applies is that
/// doubling, named for its permanent, exactly when the damage is combat
/// damage and the creature it reaches deals it or is dealt it; to any
/// other damage, none applies. Each kind of damage and target a constant
/// of its own harness, as for [`preventing_combat_damage`].
fn doubling_combat_damage(combat: bool, to_creature: bool) {
    use crate::damage::DamageEffect;
    let (a, registry) = attached(ContinuousEffect::DoubleCombatDamage { scope: EffectScope::Attached }, &[]);
    let (source, target, _, effects) = damage_of(&a, &registry, combat, to_creature);
    if combat && (a.reaches[source] || target.is_some_and(|t| a.reaches[t])) {
        assert!(matches!(effects.as_slice(), [DamageEffect::Double { by }] if *by == a.permanent));
    } else {
        assert!(effects.is_empty());
    }
    std::mem::forget((a, registry, effects));
}

/// The two proofs above for each kind of damage and each target.
macro_rules! each_damage {
    ($($name:ident: $proof:ident($combat:expr, $to_creature:expr)),* $(,)?) => {$(
        damage_proof! {
            /// The proof above it, for one kind of damage and one target.
            fn $name() {
                $proof($combat, $to_creature);
            }
        }
    )*};
}

each_damage! {
    combat_damage_to_a_creature_is_prevented_to_and_by_the_creature_it_is_attached_to: preventing_combat_damage(true, true),
    combat_damage_to_a_player_is_prevented_by_the_creature_it_is_attached_to: preventing_combat_damage(true, false),
    noncombat_damage_to_a_creature_is_not_prevented_by_combat_damage_prevention: preventing_combat_damage(false, true),
    noncombat_damage_to_a_player_is_not_prevented_by_combat_damage_prevention: preventing_combat_damage(false, false),
    combat_damage_to_a_creature_is_doubled_to_and_by_the_creature_it_is_attached_to: doubling_combat_damage(true, true),
    combat_damage_to_a_player_is_doubled_by_the_creature_it_is_attached_to: doubling_combat_damage(true, false),
    noncombat_damage_to_a_creature_is_not_doubled_by_combat_damage_doubling: doubling_combat_damage(false, true),
    noncombat_damage_to_a_player_is_not_doubled_by_combat_damage_doubling: doubling_combat_damage(false, false),
}

damage_proof! {
    /// "If it would be dealt damage, prevent that damage and remove a
    /// +1/+1 counter from it" (CR 615.1, Unbreathing Horde's, of itself):
    /// the one effect that applies is that prevention, named for its
    /// permanent, exactly when the creature it reaches is dealt the
    /// damage, combat damage or not; to any other damage, none applies.
    fn preventing_damage_for_a_counter_reaches_the_creature_it_is_attached_to() {
        use crate::damage::DamageEffect;
        let (a, registry) = attached(ContinuousEffect::PreventDamageRemoveCounter { scope: EffectScope::Attached }, &[]);
        let (_, target, _, effects) = damage_on(&a, &registry);
        if target.is_some_and(|t| a.reaches[t]) {
            assert!(matches!(effects.as_slice(), [DamageEffect::PreventAndRemoveCounter { by }] if *by == a.permanent));
        } else {
            assert!(effects.is_empty());
        }
        std::mem::forget((a, registry, effects));
    }
}
