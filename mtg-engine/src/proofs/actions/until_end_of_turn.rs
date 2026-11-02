//! What an effect that lasts until end of turn does once a card has made
//! it. A card proof stops at the effect: Giant Growth's says the card adds
//! a `TemporaryEffect::ModifyPT` of +3/+3 on its target to
//! `until_end_of_turn` and changes nothing else. These say what the engine
//! then makes of each kind of effect a card in the pool adds there, by the
//! questions that read it: a creature's power and toughness (CR 613.4c),
//! its keywords (613.1f), whether it can block (509.1a) and what it has
//! protection from (702.16).
//!
//! Each proof is over two creatures (either player's, power 0 to 3 and
//! toughness 1 to 3) of the blank card unless it says otherwise, and the
//! effects under proof, each on either creature; nothing on the board
//! prints an effect of its own. Where the question asked is one only of a
//! permanent, each creature is on the battlefield or in a graveyard: the
//! effects on a creature that leaves end with it (CR 400.7,
//! `prune_effects_on_departed_objects`'s proof), and what stays to be
//! shown is that one reaches nothing off the battlefield.

use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::proofs::cards::{card_proof, case_by_case, Blank};
use crate::state::{GameState, TemporaryEffect};
use crate::types::{CardType, CreatureFilter, Keyword, Zone};

/// A card that prints exactly one keyword.
struct Printing(Keyword);

impl CardBehavior for Printing {
    fn card_data(&self) -> CardData {
        CardData { name: "Printing".into(), keywords: vec![self.0], ..CardData::default() }
    }
}

/// The board every proof here is over.
struct Two {
    state: GameState,
    creatures: [ObjectId; 2],
    /// Each creature's printed power and toughness.
    base: [(i32, i32); 2],
    /// Whether each creature is on the battlefield.
    on: [bool; 2],
}

/// Two creatures of `cards`, each on the battlefield, or when `anywhere`
/// on the battlefield or in a graveyard.
fn two_creatures(cards: [CardId; 2], anywhere: bool) -> Two {
    let mut state = crate::proofs::cards::new_game();
    let creature = |state: &mut GameState, card: CardId| {
        let (p, t): (u8, u8) = (kani::any(), kani::any());
        kani::assume(p <= 3 && (1..=3).contains(&t));
        let on = !anywhere || kani::any();
        let owner = PlayerId(u8::from(kani::any::<bool>()));
        let id = state.create_object(card, owner, if on { Zone::Battlefield } else { Zone::Graveyard },
            Some(i32::from(p)), Some(i32::from(t)));
        state.get_object_mut(id).expect("just created").card_types = vec![CardType::Creature];
        (id, (i32::from(p), i32::from(t)), on)
    };
    let (a, base_a, on_a) = creature(&mut state, cards[0]);
    let (b, base_b, on_b) = creature(&mut state, cards[1]);
    Two { state, creatures: [a, b], base: [base_a, base_b], on: [on_a, on_b] }
}

/// Which of the two creatures an effect is on: either.
fn either() -> usize {
    usize::from(kani::any::<bool>())
}

/// "Gets +P/+T until end of turn" (CR 613.4c): on the battlefield, a
/// creature's power and toughness are its own plus what each such effect
/// on it adds, and an effect on the other creature adds nothing to it;
/// off the battlefield, they are its own (613.1: only a permanent's are
/// changed). For `n` effects, each on either creature and each of any
/// amounts from -8 to 8: a card adds a constant (Giant Growth's +3/+3,
/// Hysterical Blindness's -4/-0) or a count (Kessig Wolf Run's X, Elder of
/// Laurels' creatures), and the engine's own -N/-N is one too.
fn modifying(n: usize) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut b = two_creatures([blank, blank], true);
    let mut added = [(0, 0); 2];
    for _ in 0..n {
        let i = either();
        let (p, t): (i8, i8) = (kani::any(), kani::any());
        kani::assume((-8..=8).contains(&p) && (-8..=8).contains(&t));
        let (p, t) = (i32::from(p), i32::from(t));
        b.state.until_end_of_turn.push(TemporaryEffect::ModifyPT { target: b.creatures[i], power_mod: p, toughness_mod: t });
        added[i] = (added[i].0 + p, added[i].1 + t);
    }
    for i in 0..2 {
        let (bp, bt) = b.base[i];
        let (dp, dt) = if b.on[i] { added[i] } else { (0, 0) };
        assert!(b.state.effective_power(b.creatures[i], &registry) == Some(bp + dp));
        assert!(b.state.effective_toughness(b.creatures[i], &registry) == Some(bt + dt));
    }
    std::mem::forget((b, registry));
}

/// "Gains <keyword> until end of turn" (CR 613.1f): a creature on the
/// battlefield has the keyword exactly when such an effect is on it, and
/// one on the other creature gives it nothing; off the battlefield, it
/// has none. And it gains that keyword only: a creature with it has no
/// `other`.
fn granting(keyword: Keyword, other: Keyword) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut b = two_creatures([blank, blank], true);
    let on = either();
    b.state.until_end_of_turn.push(TemporaryEffect::GrantKeyword { target: b.creatures[on], keyword });
    for i in 0..2 {
        assert!(b.state.has_keyword(b.creatures[i], keyword, &registry) == (b.on[i] && i == on));
        assert!(!b.state.has_keyword(b.creatures[i], other, &registry));
    }
    std::mem::forget((b, registry));
}

/// A proof above for each kind and amount a card in the pool adds, or the
/// number of them.
macro_rules! each_effect {
    ($($name:ident: $proof:ident($($arg:expr),*)),* $(,)?) => {$(
        card_proof! {
            /// The proof above it, for one effect a card in the pool adds.
            fn $name() {
                $proof($($arg),*);
            }
        }
    )*};
}

each_effect! {
    one_power_and_toughness_change_until_end_of_turn_reaches_the_creature_it_is_on: modifying(1),
    two_power_and_toughness_changes_until_end_of_turn_add_up_on_the_creature_they_are_on: modifying(2),
    first_strike_until_end_of_turn_reaches_the_creature_it_is_on: granting(Keyword::FirstStrike, Keyword::Flying),
    flying_until_end_of_turn_reaches_the_creature_it_is_on: granting(Keyword::Flying, Keyword::Reach),
    haste_until_end_of_turn_reaches_the_creature_it_is_on: granting(Keyword::Haste, Keyword::Flying),
    hexproof_until_end_of_turn_reaches_the_creature_it_is_on: granting(Keyword::Hexproof, Keyword::Flying),
    lifelink_until_end_of_turn_reaches_the_creature_it_is_on: granting(Keyword::Lifelink, Keyword::Flying),
    reach_until_end_of_turn_reaches_the_creature_it_is_on: granting(Keyword::Reach, Keyword::Flying),
    trample_until_end_of_turn_reaches_the_creature_it_is_on: granting(Keyword::Trample, Keyword::Flying),
}

/// "Loses defender until end of turn" (CR 613.1f), as Manor Gargoyle's
/// ability takes it from Manor Gargoyle: a creature on the battlefield
/// that has defender, printed, has it no longer while such an effect is
/// on it, and one on the other creature takes nothing from it; a
/// creature without defender, or off the battlefield, has none. Each
/// creature of a card printing defender or of the blank card as `prints`
/// says, a constant in each harness: a card the solver chose was every
/// card's keywords to read.
fn losing_defender(prints: [bool; 2]) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let defender = registry.register(Box::new(Printing(Keyword::Defender)));
    let mut b = two_creatures([if prints[0] { defender } else { blank }, if prints[1] { defender } else { blank }], true);
    let on = either();
    b.state.until_end_of_turn.push(TemporaryEffect::RemoveKeyword { target: b.creatures[on], keyword: Keyword::Defender });
    for i in 0..2 {
        assert!(b.state.has_keyword(b.creatures[i], Keyword::Defender, &registry) == (b.on[i] && prints[i] && i != on));
    }
    std::mem::forget((b, registry));
}

case_by_case! { losing_defender;
    losing_defender_until_end_of_turn_reaches_the_defender_it_is_on_beside_another: [true, true],
    losing_defender_until_end_of_turn_reaches_the_defender_it_is_on_beside_a_creature_without: [true, false],
    losing_defender_until_end_of_turn_reaches_the_defender_it_is_on_after_a_creature_without: [false, true],
    losing_defender_until_end_of_turn_takes_nothing_from_a_creature_without_it: [false, false],
}

card_proof! {
    /// "Can't block this turn" (CR 509.1a), as Nightbird's Clutches and
    /// the engine's own pending effect add it: a creature can't block
    /// exactly when such an effect is on it, and one on the other creature
    /// leaves it free to. Both creatures on the battlefield.
    fn cant_block_this_turn_reaches_the_creature_it_is_on() {
        let mut registry = CardRegistry::new();
        let blank = registry.register(Box::new(Blank));
        let mut b = two_creatures([blank, blank], false);
        let on = either();
        b.state.until_end_of_turn.push(TemporaryEffect::CantBlock { target: b.creatures[on] });
        for i in 0..2 {
            assert!(b.state.can_block(b.creatures[i], &registry) == (i != on));
        }
        std::mem::forget((b, registry));
    }
}

/// "Gains protection from non-Human creatures until end of turn" (CR
/// 702.16), as Spare from Evil adds it: of two creatures, the one the
/// effect is on has protection from a source, either player's, exactly
/// when the source matches the effect's filter, and the other has none.
/// The filter stands in as any answer, checked to be the effect's own
/// (`filters::stand_in`); that it matches exactly a creature that is not
/// a Human is `filters`'s proof.
#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::matches_filter, super::filters::stand_in)]
fn protection_until_end_of_turn_is_from_what_its_filter_matches() {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut b = two_creatures([blank, blank], false);
    let source = b.state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, Some(1), Some(1));
    let on = either();
    b.state.until_end_of_turn.push(TemporaryEffect::GrantProtection {
        target: b.creatures[on],
        filter: CreatureFilter::And(vec![
            CreatureFilter::HasCardType(CardType::Creature),
            CreatureFilter::Not(Box::new(CreatureFilter::HasSubtype("Human".into()))),
        ]),
    });
    let TemporaryEffect::GrantProtection { filter, .. } = &b.state.until_end_of_turn[0] else { unreachable!("just pushed") };
    super::filters::asked_of(filter);
    for i in 0..2 {
        assert!(b.state.has_protection_from(b.creatures[i], source, &registry) == (i == on && super::filters::matches(source)));
    }
    std::mem::forget((b, registry));
}

/// Stands in for `GameState::has_protection_from`: nothing has protection
/// here. Protection's prevention is `applicable_effects`' own proof
/// (`damage::on_a_board_printing_none_only_protection_applies`), and what
/// has protection from what is the one above.
#[allow(dead_code)] // named only by `kani::stub`
fn protected_from_nothing(_: &GameState, _: ObjectId, _: ObjectId, _: &CardRegistry) -> bool {
    false
}

/// "Prevent all combat damage that would be dealt this turn by creatures
/// other than Werewolves and Wolves" (CR 615.1), as Moonmist adds it: the
/// one effect that applies to damage (616.1) is that prevention, named
/// for the card that made it, exactly when the damage is combat damage
/// and its source does not match the effect's filter; to other damage,
/// none applies. For damage combat or not, from a creature of either
/// player's, to another creature or to either player. The filter stands
/// in as any answer, checked to be the effect's own
/// (`filters::stand_in`); that it matches exactly a Wolf or a Werewolf is
/// `filters`'s proof.
#[kani::proof]
#[kani::unwind(7)]
#[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
#[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
#[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
#[kani::stub(crate::state::GameState::has_protection_from, protected_from_nothing)]
#[kani::stub(crate::state::GameState::matches_filter, super::filters::stand_in)]
fn combat_damage_this_turn_is_prevented_from_what_the_filter_does_not_match() {
    use crate::damage::{applicable_effects, DamageEffect, DamageKind, PendingDamage, Prevention};
    use crate::events::DamageTarget;
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let b = two_creatures([blank, blank], false);
    let mut state = b.state;
    let (source, other) = (b.creatures[0], b.creatures[1]);
    state.until_end_of_turn.push(TemporaryEffect::PreventCombatDamageExcept {
        filter: CreatureFilter::Or(vec![CreatureFilter::HasSubtype("Wolf".into()), CreatureFilter::HasSubtype("Werewolf".into())]),
        source_name: "a prevention".into(),
    });
    let TemporaryEffect::PreventCombatDamageExcept { filter, .. } = &state.until_end_of_turn[0] else { unreachable!("just pushed") };
    super::filters::asked_of(filter);
    let combat: bool = kani::any();
    let target = if kani::any() { DamageTarget::Object(other) } else { DamageTarget::Player(PlayerId(u8::from(kani::any::<bool>()))) };
    let pd = PendingDamage {
        source, target, amount: 2, kind: if combat { DamageKind::Combat } else { DamageKind::NonCombat },
        applied: Vec::new(), settled: false,
    };

    let effects = applicable_effects(&state, &pd, &registry);

    if combat && !super::filters::matches(source) {
        assert!(effects.len() == 1 && matches!(&effects[0], DamageEffect::PreventAll(Prevention::ThisTurn(name)) if name == "a prevention"));
    } else {
        assert!(effects.is_empty());
    }
    std::mem::forget((registry, state, pd, effects));
}

/// The cleanup step's end of the turn's effects (CR 514.2,
/// `engine::end_until_end_of_turn_effects`): every until-end-of-turn
/// effect ends, and each permanent that one of them, on the creatures `on`
/// names, handed to a player ("gain control of target creature until end of turn", as
/// Traitorous Blood adds it) goes, in their order, to whom the control
/// effects still in force give it (613.7a): a lasting one on it ("for as
/// long as you control Olivia Voldaren"), whatever its timestamp, or else
/// its default controller (110.2a). One that has left the battlefield
/// goes nowhere. Nothing else changes, the lasting effect included. Over
/// two creatures each on the battlefield or in a graveyard, each
/// control-changing effect to either player, a power and toughness change
/// beside them, and a lasting effect on either creature or on neither,
/// before or after them. Which creature each control change is on is a
/// constant of each harness: two left to the solver ran it out of memory
/// at 7 GB.
fn ending_the_turns_effects(on: &[usize]) {
    let n = on.len();
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut b = two_creatures([blank, blank], true);
    let mut targets = [ObjectId(0); 2];
    b.state.until_end_of_turn.push(TemporaryEffect::ModifyPT { target: b.creatures[either()], power_mod: 1, toughness_mod: 1 });
    for (k, target) in targets.iter_mut().enumerate().take(n) {
        *target = b.creatures[on[k]];
        let controller = PlayerId(u8::from(kani::any::<bool>()));
        b.state.until_end_of_turn.push(TemporaryEffect::ChangeControl { target: *target, controller, timestamp: 2 + 2 * k as u64 });
    }
    let lasting = kani::any::<u8>() % 3;
    let lasting_controller = PlayerId(u8::from(kani::any::<bool>()));
    b.state.control_effects = Vec::with_capacity(1);
    let effect = (lasting < 2).then(|| crate::state::ControlEffect {
        object: b.creatures[usize::from(lasting)], controller: lasting_controller, original_controller: PlayerId(0),
        source: b.creatures[1 - usize::from(lasting)], source_controller: lasting_controller,
        timestamp: if kani::any() { 1 } else { 5 },
    });
    if let Some(e) = &effect {
        b.state.control_effects.push(e.clone());
    }
    let before = crate::proofs::cards::Snapshot::of(&b.state);

    crate::engine::end_until_end_of_turn_effects(&mut b.state);

    assert!(b.state.until_end_of_turn.is_empty());
    // The lasting effect as it was made, compared field by field rather than
    // against a clone of the list, which a proof copies imprecisely.
    assert!(match &effect {
        Some(e) => b.state.control_effects.len() == 1 && b.state.control_effects[0] == *e,
        None => b.state.control_effects.is_empty(),
    });
    assert!(crate::proofs::cards::Snapshot::of(&b.state) == before.without_until_end_of_turn());
    let goes_to = |target: ObjectId| {
        if lasting < 2 && b.creatures[usize::from(lasting)] == target {
            lasting_controller
        } else {
            b.state.get_object(target).expect("on the board").base_controller
        }
    };
    let returned = |target: ObjectId| b.state.get_object(target).is_some_and(|o| o.zone == Zone::Battlefield);
    // Each case's actions written out, a list of a length known: one built
    // by pushing as each target returns is of a length and contents the
    // solver leaves open, and each comparison against it was against every
    // kind of action there is.
    use crate::proofs::cards::{acted, asked_for, Act};
    let back = |t: ObjectId| Act::Control(t, goes_to(t));
    let (t0, t1) = (targets[0], targets[1]);
    assert!(match (n, returned(t0), n == 2 && returned(t1)) {
        (_, true, true) => asked_for(&[back(t0), back(t1)]),
        (_, true, false) => asked_for(&[back(t0)]),
        (_, false, true) => asked_for(&[back(t1)]),
        (_, false, false) => acted() == 0,
    });
    std::mem::forget((b, registry, effect, before));
}

/// [`ending_the_turns_effects`] for one and for two control-changing
/// effects, on each creature.
macro_rules! each_count {
    ($($name:ident: $on:expr),* $(,)?) => {$(
        #[kani::proof]
        #[kani::unwind(7)]
        #[kani::stub(std::fmt::format, crate::proofs::cards::no_format)]
        #[kani::stub(crate::state::GameState::obj_name, crate::proofs::cards::no_name)]
        #[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
        #[kani::stub(alloc::alloc::Global::deallocate_impl_runtime, crate::proofs::cards::no_freeing)]
        #[kani::stub(crate::state::GameState::log, crate::proofs::cards::no_log)]
        #[kani::stub(crate::state::GameState::change_control, crate::proofs::cards::record_control)]
        /// The proof above it, for one number of control-changing effects.
        fn $name() {
            ending_the_turns_effects(&$on);
        }
    )*};
}

each_count! {
    the_cleanup_step_ends_the_turns_effects_and_a_control_change_of_the_first: [0],
    the_cleanup_step_ends_the_turns_effects_and_a_control_change_of_the_second: [1],
    the_cleanup_step_ends_the_turns_effects_and_two_control_changes_of_the_first: [0, 0],
    the_cleanup_step_ends_the_turns_effects_and_two_control_changes_of_the_second: [1, 1],
    the_cleanup_step_ends_the_turns_effects_and_control_changes_of_each_in_order: [0, 1],
    the_cleanup_step_ends_the_turns_effects_and_control_changes_of_each_the_other_way: [1, 0],
}

/// The effects [`granted_as_given`] answers from: the harness's own list,
/// each effect on the card or not, and how many there are.
static mut GIVEN: (*const (bool, crate::types::ManaCost), usize) = (std::ptr::null(), 0);

/// Stands in for `engine::granted_flashback_costs`, as its proof
/// ([`reading_the_grants`]) shows it answers: the costs of the effects on
/// the card, in the order granted. Read off [`GIVEN`], the harness's own
/// list, which the effects on the board were copied from: off the board's
/// list of effects, which card each is on is left open to symbolic
/// execution (that proof says why), and each cost after was walked every
/// way.
#[allow(dead_code)] // named only by `kani::stub`
fn granted_as_given(_: &GameState, id: ObjectId) -> Vec<&crate::types::ManaCost> {
    assert!(id == ObjectId(1), "asked of the card");
    // SAFETY: a proof runs on one thread; GIVEN points at the harness's
    // list, which outlives the call.
    let (effects, n) = unsafe { *std::ptr::addr_of!(GIVEN) };
    let mut costs = Vec::with_capacity(2);
    for i in 0..n {
        // SAFETY: as above, `i` within the list.
        let (on_it, cost) = unsafe { &*effects.add(i) };
        if *on_it {
            costs.push(cost);
        }
    }
    costs
}

/// "Gains flashback until end of turn" (CR 702.34a), as Snapcaster Mage
/// and Past in Flames add it, the cost each gives a card's own mana cost:
/// a card in its owner's graveyard may be cast from there at each cost
/// such an effect on it gives, each once however many give it, and an
/// effect on another card gives it nothing. Over a card of the blank card
/// (which prints no flashback and no permission to cast from a graveyard,
/// and costs nothing to change) and another, either player's, and an
/// effect for each of `effects`, on the card or on the other as it says,
/// a constant in each harness. Which of the turn's effects are on the
/// card is `engine::granted_flashback_costs`, proved apart
/// ([`reading_the_grants`]) and standing in here as that proof shows it
/// answers ([`granted_as_given`]).
fn granting_flashback<const N: usize>(effects: [(bool, crate::types::ManaCost); N]) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let card = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Graveyard, None, None);
    let _other = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Graveyard, None, None);
    assert!(card == ObjectId(1));
    // SAFETY: a proof runs on one thread, and `effects` outlives every
    // call made below.
    unsafe { *std::ptr::addr_of_mut!(GIVEN) = (effects.as_ptr(), N) };
    let mut expected: Vec<&crate::types::ManaCost> = Vec::with_capacity(N);
    for (on_it, cost) in &effects {
        if *on_it && !expected.contains(&cost) {
            expected.push(cost);
        }
    }
    let owner = state.get_object(card).expect("on the board").owner;
    let offered = crate::engine::graveyard_cast_costs(&state, &registry, card, owner);
    assert!(offered.len() == expected.len() && offered.iter().zip(&expected).all(|(a, b)| a == *b));
    std::mem::forget((expected, offered));
    std::mem::forget((registry, state, effects));
}

/// What `engine::granted_flashback_costs` reads off the turn's effects:
/// for each of two cards, exactly the costs of the effects that grant it
/// flashback, each the effect's own (the same object, not one like it), in
/// the order granted. Over `n` effects granting flashback, each on either
/// card, and the card and the other each either player's.
///
/// Read back off the turn's list, an effect is a union to symbolic
/// execution: which card one is on it does not see, and the costs walked
/// after it walked every way, until the solver ran out of memory. Here
/// nothing is read of a cost but where it is.
fn reading_the_grants(n: usize) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let card = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Graveyard, None, None);
    let other = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Graveyard, None, None);
    let mut on_card = [false; 2];
    state.until_end_of_turn = Vec::with_capacity(2);
    for on in on_card.iter_mut().take(n) {
        *on = kani::any();
        state.until_end_of_turn.push(TemporaryEffect::GrantFlashback {
            target: if *on { card } else { other },
            cost: crate::types::ManaCost::new(Vec::new()),
        });
    }
    let costs: [Option<&crate::types::ManaCost>; 2] = std::array::from_fn(|i| match state.until_end_of_turn.get(i) {
        Some(TemporaryEffect::GrantFlashback { cost, .. }) => Some(cost),
        _ => None,
    });
    for (id, on) in [(card, true), (other, false)] {
        let granted = crate::engine::granted_flashback_costs(&state, id);
        let mut expected = costs.iter().zip(on_card).take(n).filter(|(_, o)| *o == on).map(|(c, _)| c.expect("an effect"));
        for cost in &granted {
            assert!(expected.next().is_some_and(|e| std::ptr::eq(*cost, e)));
        }
        assert!(expected.next().is_none());
        std::mem::forget(granted);
    }
    std::mem::forget((registry, state));
}

/// {1}{U}, Snapcaster Mage's own mana cost, as a cost it gives.
fn one_and_blue() -> crate::types::ManaCost {
    use crate::types::{Color, ManaSymbol};
    crate::types::ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Blue)])
}

/// {R}, a second cost.
fn red() -> crate::types::ManaCost {
    use crate::types::{Color, ManaSymbol};
    crate::types::ManaCost::new(vec![ManaSymbol::Colored(Color::Red)])
}

card_proof! {
    #[kani::stub(crate::engine::granted_flashback_costs, granted_as_given)]
    /// [`granting_flashback`] for one effect on the card.
    fn flashback_until_end_of_turn_reaches_the_card_it_is_on() {
        granting_flashback([(true, one_and_blue())]);
    }
}

card_proof! {
    #[kani::stub(crate::engine::granted_flashback_costs, granted_as_given)]
    /// [`granting_flashback`] for one effect on the other card.
    fn flashback_until_end_of_turn_on_another_card_gives_this_one_none() {
        granting_flashback([(false, one_and_blue())]);
    }
}

card_proof! {
    #[kani::stub(crate::engine::granted_flashback_costs, granted_as_given)]
    /// [`granting_flashback`] for two effects of one cost on the card, as
    /// two Past in Flames give it: one way to cast it, not two.
    fn flashback_granted_twice_at_one_cost_is_one_way_to_cast_it() {
        granting_flashback([(true, one_and_blue()), (true, one_and_blue())]);
    }
}

card_proof! {
    #[kani::stub(crate::engine::granted_flashback_costs, granted_as_given)]
    /// [`granting_flashback`] for two effects of two costs on the card.
    fn flashback_granted_at_two_costs_is_two_ways_to_cast_it() {
        granting_flashback([(true, one_and_blue()), (true, red())]);
    }
}

card_proof! {
    #[kani::stub(crate::engine::granted_flashback_costs, granted_as_given)]
    /// [`granting_flashback`] for an effect on the card and one on the
    /// other.
    fn flashback_granted_to_it_and_another_is_one_way_to_cast_it() {
        granting_flashback([(true, one_and_blue()), (false, red())]);
    }
}

card_proof! {
    /// [`reading_the_grants`] with no effect granting flashback.
    fn no_flashback_granted_until_end_of_turn_is_read_as_none() {
        reading_the_grants(0);
    }
}

card_proof! {
    /// [`reading_the_grants`] for one effect, on either card.
    fn flashback_granted_until_end_of_turn_is_read_as_the_effects_on_each_card() {
        reading_the_grants(1);
    }
}

card_proof! {
    /// [`reading_the_grants`] for two effects, each on either card.
    fn flashback_granted_twice_until_end_of_turn_is_read_in_order_for_each_card() {
        reading_the_grants(2);
    }
}
