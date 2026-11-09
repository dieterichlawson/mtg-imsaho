//! Activating an ability (`engine::actions::abilities::activate_ability`,
//! CR 602.2): what puts an ability on the stack with the targets its
//! resolution reads (`resolving`, `an_ability_resolving`). As for a cast
//! (`casting`), the targets an activation names are checked against the
//! rule the offered lists are built by before anything else is done
//! (602.2b via 601.2c), and that check (`targeting::targets_are_legal`) is
//! any answer here.

use crate::proofs::held::Held;
use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, LoyaltyAbilityDef, SacrificeCost, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{acted, asked_for, card_proof, were, Act, Blank};
use crate::state::{GameState, StackEntry};
use crate::types::{CardType, CounterType, ManaCost, Zone};

/// A permanent whose one ability is "{T}: <an effect> to target creature".
struct Activatable;

impl CardBehavior for Activatable {
    fn card_data(&self) -> CardData {
        CardData { name: "Activatable".into(), ..CardData::default() }
    }
    fn activated_abilities(&self, _: &GameState, _: ObjectId, _: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: String::new(),
            cost: ManaCost::free(),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: Some(TargetRequirement::Creature),
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
            tap_cost: None,
        }]
    }
}

/// The check's answer: chosen the first time it is asked, the same after.
static mut LEGAL: Held<Option<bool>> = Held::new(None, (&raw const LEGAL).cast());

/// Stands in for `targeting::targets_are_legal`: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn targets_are_legal(_: &GameState, _: &TargetRequirement, _: &[Target], _: PlayerId, _: ObjectId, _: &dyn CardBehavior, _: &CardRegistry) -> bool {
    // SAFETY: a proof runs on one thread, and only this function writes LEGAL.
    *unsafe { &mut *std::ptr::addr_of_mut!(LEGAL.value) }.get_or_insert_with(kani::any::<bool>)
}

/// Stands in for `engine::demand_reserve`: none, as in `casting`; this
/// ability's cost has no mana in it.
#[allow(dead_code)] // named only by `kani::stub`
fn no_reserve(_: &GameState, _: &CardRegistry, _: PlayerId, _: Option<ObjectId>) -> ManaCost {
    ManaCost::free()
}

/// A permanent's ability activated at a creature (CR 602.2a-b): with the
/// targets it names legal, its source is tapped for its cost, and it goes
/// on top of the stack from that source, as the ability of that index of
/// the card that grants it, holding exactly those targets and the
/// requirement they were chosen under, under the player who activated it
/// (602.2a); with them not, it is refused before anything is done (nothing
/// tapped, nothing on the stack). For an activator either player and a
/// creature either player's.
fn activating() {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let card = registry.register(Box::new(Activatable));
    let mut state = crate::proofs::cards::new_game();
    let creature = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, Some(1), Some(1));
    let activator = PlayerId(u8::from(kani::any::<bool>()));
    let source = state.create_object(card, activator, Zone::Battlefield, None, None);
    state.priority_player = Some(activator);

    let applied = crate::engine::actions::abilities::activate_ability(
        &mut state, source, 0, &[Target::Object(creature)], &[], None, None, &registry);

    // SAFETY: as in `targets_are_legal`.
    let legal = unsafe { *std::ptr::addr_of!(LEGAL.value) } == Some(true);
    if legal {
        assert!(matches!(applied, crate::engine::Applied::Continue));
        assert!(asked_for(&[Act::Tap(source)]));
        assert!(matches!(state.stack.as_slice(), [StackEntry::Ability {
            source_id, ability_index: 0, behavior_card_id, targets, activator: by,
            target_requirement: Some(TargetRequirement::Creature), ..
        }] if *source_id == source && *behavior_card_id == card && *by == activator
            && matches!(targets.as_slice(), [Target::Object(t)] if *t == creature)));
    } else {
        assert!(matches!(applied, crate::engine::Applied::ReturnNow));
        assert!(acted() == 0 && state.stack.is_empty());
    }
    std::mem::forget((registry, state));
}

card_proof! {
    #[kani::stub(crate::engine::targeting::targets_are_legal, targets_are_legal)]
    #[kani::stub(crate::engine::demand_reserve, no_reserve)]
    /// [`activating`].
    fn an_ability_activated_at_a_legal_target_goes_on_the_stack_holding_it_and_an_illegal_one_is_refused() {
        activating();
    }
}

/// A planeswalker whose one loyalty ability, of `.0` loyalty, targets a
/// creature.
struct Walker(i32);

impl CardBehavior for Walker {
    fn card_data(&self) -> CardData {
        CardData { name: "Walker".into(), card_types: vec![CardType::Planeswalker], ..CardData::default() }
    }
    fn loyalty_abilities(&self, _: &GameState, _: ObjectId) -> Vec<LoyaltyAbilityDef> {
        vec![LoyaltyAbilityDef {
            ability_index: 0,
            loyalty_change: self.0,
            description: String::new(),
            target_requirement: Some(TargetRequirement::Creature),
        }]
    }
}

/// A planeswalker's loyalty ability activated at a creature (CR 606.3-5):
/// with the targets it names legal, its cost is the loyalty it changes by
/// (counters put on for a plus, taken off for a minus, none for a zero),
/// the planeswalker is marked as having activated one this turn, and the
/// ability goes on top of the stack from it, flagged as a loyalty ability,
/// holding exactly those targets and the requirement they were chosen
/// under, under the player who activated it; with them not, it is refused
/// before anything is done. For an activator either player and a creature
/// either player's, and the change of each harness.
fn activating_loyalty(change: i32) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let card = registry.register(Box::new(Walker(change)));
    let mut state = crate::proofs::cards::new_game();
    let creature = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, Some(1), Some(1));
    let activator = PlayerId(u8::from(kani::any::<bool>()));
    let source = state.create_object(card, activator, Zone::Battlefield, None, None);
    state.priority_player = Some(activator);

    let applied = crate::engine::actions::abilities::activate_loyalty_ability(
        &mut state, source, 0, &[Target::Object(creature)], &registry);

    // SAFETY: as in `targets_are_legal`.
    let legal = unsafe { *std::ptr::addr_of!(LEGAL.value) } == Some(true);
    let marked = state.get_object(source).expect("on the board").abilities_activated_this_turn.contains(&999);
    if legal {
        assert!(matches!(applied, crate::engine::Applied::Continue));
        let paid = match change {
            0 => acted() == 0,
            n if n > 0 => asked_for(&[Act::Counters(source, CounterType::Loyalty, n.unsigned_abs())]),
            n => asked_for(&[Act::RemoveCounters(source, CounterType::Loyalty, n.unsigned_abs())]),
        };
        assert!(paid && marked);
        assert!(matches!(state.stack.as_slice(), [StackEntry::Ability {
            source_id, ability_index: 0, behavior_card_id, targets, activator: by, loyalty: true,
            target_requirement: Some(TargetRequirement::Creature), ..
        }] if *source_id == source && *behavior_card_id == card && *by == activator
            && matches!(targets.as_slice(), [Target::Object(t)] if *t == creature)));
    } else {
        assert!(matches!(applied, crate::engine::Applied::ReturnNow));
        assert!(acted() == 0 && state.stack.is_empty() && !marked);
    }
    std::mem::forget((registry, state));
}

card_proof! {
    #[kani::stub(crate::engine::targeting::targets_are_legal, targets_are_legal)]
    /// [`activating_loyalty`] for a plus ability.
    fn a_plus_loyalty_ability_at_a_legal_target_puts_counters_on_and_goes_on_the_stack() {
        activating_loyalty(1);
    }
}

card_proof! {
    #[kani::stub(crate::engine::targeting::targets_are_legal, targets_are_legal)]
    /// [`activating_loyalty`] for a zero ability.
    fn a_zero_loyalty_ability_at_a_legal_target_goes_on_the_stack_for_nothing() {
        activating_loyalty(0);
    }
}

card_proof! {
    #[kani::stub(crate::engine::targeting::targets_are_legal, targets_are_legal)]
    /// [`activating_loyalty`] for a minus ability.
    fn a_minus_loyalty_ability_at_a_legal_target_takes_counters_off_and_goes_on_the_stack() {
        activating_loyalty(-2);
    }
}

/// What `GameState::effective_toughness` answered for each object (1 and
/// 2), and how many actions had been asked for when it was first read.
static mut TOUGHNESS: Held<[Option<(Option<i32>, usize)>; 2]> = Held::new([None; 2], (&raw const TOUGHNESS).cast());

/// Stands in for `GameState::effective_toughness`, proved in
/// `power_and_toughness`: any answer, the same for an object each time.
#[allow(dead_code)] // named only by `kani::stub`
fn toughness(_: &GameState, id: ObjectId, _: &CardRegistry) -> Option<i32> {
    let i = usize::try_from(id.0).expect("an id of the board");
    assert!((1..=2).contains(&i), "an object this proof does not make");
    // SAFETY: a proof runs on one thread, and only this function writes TOUGHNESS.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(TOUGHNESS.value) };
    answers[i - 1].get_or_insert_with(|| (kani::any(), acted())).0
}

/// An ability's costs paid (`pay_activation_costs`, CR 602.2b via 601.2h,
/// 118.3), in the order the engine fixes: the source tapped for a {T},
/// then the creatures a "tap N creatures you control" cost chose, then the
/// counters a cost removes, removed from the source (before a sacrifice
/// moves it, so they are removed and not lost), then the sacrifice: the
/// source itself for "sacrifice this", the creature named for "sacrifice a
/// creature" and "sacrifice another creature", none for a cost without
/// one. Which creature was sacrificed, and its toughness as it was on the
/// battlefield (read before the sacrifice, CR 608.2h), are what the
/// ability resolves with (Disciple of Griselbrand's "the sacrificed
/// creature's toughness"); and a once-each-turn ability is marked as
/// activated this turn. A cost that names a creature to sacrifice and is
/// handed none, which no offer is (`offering`, one offer for each creature
/// it may name), sacrifices nothing and stops there. For the kind of
/// sacrifice of each harness (0 none, 1 this, 2 a creature, 3 another
/// creature), the source, the other creature or nothing named, a {T} or
/// not, the other creature tapped for the cost or not, one to three +1/+1
/// counters or none, and once each turn or not, for the ability of index
/// one; the cost's mana is none, its paying the payment proofs'
/// (`paying`).
fn costs_paid(kind: u8) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let player = PlayerId(u8::from(kani::any::<bool>()));
    let source = state.create_object(blank, player, Zone::Battlefield, Some(1), Some(1));
    let other = state.create_object(blank, player, Zone::Battlefield, Some(1), Some(1));
    let sacrifice_cost = match kind {
        0 => SacrificeCost::None,
        1 => SacrificeCost::SacrificeThis,
        2 => SacrificeCost::SacrificeCreature,
        _ => SacrificeCost::SacrificeAnotherCreature,
    };
    let named = match kani::any::<u8>() % 3 {
        0 => None,
        1 => Some(source),
        _ => Some(other),
    };
    let (requires_tap, tap_other, once_per_turn): (bool, bool, bool) = (kani::any(), kani::any(), kani::any());
    let counters: u32 = kani::any();
    kani::assume(counters <= 3);
    let counter_cost = (counters > 0).then_some((CounterType::PlusOnePlusOne, counters));
    // One index: the set of the abilities activated this turn is a tree,
    // and a key the solver chose was put into it every way it might go.
    let index = 1;
    let cost = crate::state::DeferredActivationCost {
        tap_plan: Vec::new(),
        non_x_mana_cost: ManaCost::free(),
        requires_tap,
        counter_cost,
        sacrifice: named,
        sacrifice_cost,
        once_per_turn,
        tapped_creatures: if tap_other { vec![other] } else { Vec::new() },
    };

    crate::engine::actions::abilities::pay_activation_costs(&mut state, player, source, index, &cost, &registry);

    let (sacrificed, stops) = match kind {
        0 => (None, false),
        1 => (Some(source), false),
        _ => (named, named.is_none()),
    };
    // Each action expected checked where it falls, not gathered in a list
    // first: a list of open length, compared, is one the solver follows
    // through every length at once.
    let mut k = 0;
    let mut paid = true;
    if requires_tap {
        paid &= were(k, 1, |_| Act::Tap(source));
        k += 1;
    }
    if tap_other {
        paid &= were(k, 1, |_| Act::Tap(other));
        k += 1;
    }
    if counters > 0 {
        paid &= were(k, 1, |_| Act::RemoveCounters(source, CounterType::PlusOnePlusOne, counters));
        k += 1;
    }
    let before = k;
    if let Some(id) = sacrificed {
        paid &= were(k, 1, |_| Act::Sacrifice(id));
        k += 1;
    }
    assert!(paid && acted() == k);
    assert!(state.last_activated_sacrifice == sacrificed);
    // SAFETY: as in `toughness`.
    let read = unsafe { *std::ptr::addr_of!(TOUGHNESS.value) };
    match sacrificed {
        None => { assert!(state.last_activated_sacrifice_toughness.is_none() && read == [None, None]); }
        Some(id) => {
            let i = usize::try_from(id.0).expect("an id of the board") - 1;
            let (answer, at) = read[i].expect("its toughness read");
            assert!(state.last_activated_sacrifice_toughness == answer && at == before);
        }
    }
    let marked = &state.get_object(source).expect("on the board").abilities_activated_this_turn;
    assert!(marked.contains(&index) == (once_per_turn && !stops) && marked.len() <= 1);
    std::mem::forget((registry, state, cost));
}

/// Stands in for `abilities::tapped_for_cost_line`, the log's line for
/// the creatures a cost tapped, which is for people: none. Built here, the
/// names joined over a list the proof leaves one or none long ran the
/// costs out of 12 GB.
#[allow(dead_code)] // named only by `kani::stub`
fn no_tapped_line(_: &GameState, _: PlayerId, _: ObjectId, _: &[ObjectId]) -> String {
    String::new()
}

/// The proofs of an ability's costs paid, one per kind of sacrifice.
macro_rules! each_sacrifice {
    ($($name:ident: $kind:expr;)*) => {$(
        card_proof! {
            #[kani::stub(crate::engine::demand_reserve, no_reserve)]
            #[kani::stub(crate::state::GameState::effective_toughness, toughness)]
            #[kani::stub(crate::engine::actions::abilities::tapped_for_cost_line, no_tapped_line)]
            /// [`costs_paid`] for the kind of sacrifice of its name.
            fn $name() {
                costs_paid($kind);
            }
        }
    )*};
}

each_sacrifice! {
    an_abilitys_costs_without_a_sacrifice_are_paid_in_order: 0;
    an_abilitys_costs_sacrificing_it_are_paid_in_order_and_recorded: 1;
    an_abilitys_costs_sacrificing_a_creature_are_paid_in_order_and_recorded: 2;
    an_abilitys_costs_sacrificing_another_creature_are_paid_in_order_and_recorded: 3;
}

/// An ability put on the stack (`cards::push_ability`, CR 602.2a, 601.2c,
/// 608.2h, 701.28f, 113.7a): one entry, on top, from its source, as the ability of
/// that index of the card granting it, holding its targets and the
/// requirement they were chosen under (a creature, if `required`, none
/// otherwise: each its own harness, a requirement the solver chose being
/// matched as every requirement there is), under its activator, not a
/// loyalty ability, and carrying what its costs left to resolve with: the
/// X announced, the creature sacrificed and that creature's toughness as
/// last read (`costs_paid`), how often its source had transformed, its
/// source's name as it went on, and its target's zone changes so far, by
/// which a target gone and back by the time it resolves is a new object
/// (CR 400.7). For an activator either player, the source theirs on
/// the battlefield or already sacrificed to the graveyard, and any of
/// each.
fn pushing(required: bool) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let activator = PlayerId(u8::from(kani::any::<bool>()));
    let zone = if kani::any() { Zone::Battlefield } else { Zone::Graveyard };
    let source = state.create_object(blank, activator, zone, Some(1), Some(1));
    let other = state.create_object(blank, activator, Zone::Graveyard, Some(1), Some(1));
    let (transforms, back): (u32, bool) = (kani::any(), kani::any());
    {
        let o = state.get_object_mut(source).expect("just created");
        (o.transforms, o.is_transformed) = (transforms, back);
    }
    let x: Option<u32> = if kani::any() { Some(kani::any()) } else { None };
    let sacrificed = match kani::any::<u8>() % 3 {
        0 => None,
        1 => Some(source),
        _ => Some(other),
    };
    let toughness: Option<i32> = if kani::any() { Some(kani::any()) } else { None };
    (state.last_activated_x_value, state.last_activated_sacrifice, state.last_activated_sacrifice_toughness) =
        (x, sacrificed, toughness);
    let index = usize::from(kani::any::<u8>() % 3);
    let requirement = if required { Some(TargetRequirement::Creature) } else { None };

    crate::cards::push_ability(&mut state, source, index, blank, &[Target::Object(other)], requirement, activator);

    assert!(matches!(state.stack.as_slice(), [StackEntry::Ability {
        source_id, ability_index, behavior_card_id, targets, activator: by, x_value, target_requirement,
        target_zone_changes, sacrificed: s, sacrificed_toughness: t, loyalty: false, source_transforms, source_name,
    }] if *source_id == source && *ability_index == index && *behavior_card_id == blank && *by == activator
        && matches!(targets.as_slice(), [Target::Object(o)] if *o == other)
        && matches!(target_zone_changes.as_slice(), [z] if Some(*z) == state.get_object(other).map(|o| o.zone_change_count))
        && matches!(target_requirement, Some(TargetRequirement::Creature)) == required
        && target_requirement.is_some() == required
        && *x_value == x && *s == sacrificed && *t == toughness
        && *source_transforms == Some(transforms)
        && source_name.as_deref() == state.get_object(source).map(|o| o.name.as_str())));
    std::mem::forget((registry, state));
}

card_proof! {
    /// [`pushing`] for an ability whose target must be a creature.
    fn an_ability_goes_on_the_stack_with_what_its_costs_left() {
        pushing(true);
    }
}

card_proof! {
    /// [`pushing`] for an ability declaring no requirement.
    fn an_ability_of_no_requirement_goes_on_the_stack_with_what_its_costs_left() {
        pushing(false);
    }
}
