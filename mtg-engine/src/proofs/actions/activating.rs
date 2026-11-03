//! Activating an ability (`engine::actions::abilities::activate_ability`,
//! CR 602.2): what puts an ability on the stack with the targets its
//! resolution reads (`resolving`, `an_ability_resolving`). As for a cast
//! (`casting`), the targets an activation names are checked against the
//! rule the offered lists are built by before anything else is done
//! (602.2b via 601.2c), and that check (`targeting::targets_are_legal`) is
//! any answer here.

use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{acted, asked_for, card_proof, Act, Blank};
use crate::state::{GameState, StackEntry};
use crate::types::{ManaCost, Zone};

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
static mut LEGAL: Option<bool> = None;

/// Stands in for `targeting::targets_are_legal`: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn targets_are_legal(_: &GameState, _: &TargetRequirement, _: &[Target], _: PlayerId, _: ObjectId, _: &dyn CardBehavior, _: &CardRegistry) -> bool {
    // SAFETY: a proof runs on one thread, and only this function writes LEGAL.
    *unsafe { &mut *std::ptr::addr_of_mut!(LEGAL) }.get_or_insert_with(kani::any::<bool>)
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
    let legal = unsafe { *std::ptr::addr_of!(LEGAL) } == Some(true);
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
