//! Casting a spell (`engine::actions::cast::cast_spell`, CR 601.2): what
//! puts a spell on the stack with the targets its resolution reads back
//! off it (`resolving`, `a_spell_on_top`). A cast's targets are checked
//! against the rule the offered lists are built by before anything else is
//! done (601.2c); that check (`targeting::targets_are_legal`) is any answer
//! here, the same each time it is asked.

use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::events::GameEvent;
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{acted, asked_for, card_proof, Act, Blank};
use crate::state::{GameState, StackEntry};
use crate::types::{CardType, Zone};

/// An instant that costs nothing and targets a creature.
struct Castable;

impl CardBehavior for Castable {
    fn card_data(&self) -> CardData {
        CardData { name: "Castable".into(), card_types: vec![CardType::Instant], ..CardData::default() }
    }
    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
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

/// Stands in for `engine::demand_reserve`, the mana the rest of the
/// caster's cards want kept back as the cost is paid: none. Which mana
/// pays is the payment proofs' (`paying`, for any reserve), and this
/// spell costs nothing, so no reserve changes what is paid; worked out,
/// it read every card's cost on the board and ran symbolic execution out
/// of memory.
#[allow(dead_code)] // named only by `kani::stub`
fn no_reserve(_: &GameState, _: &CardRegistry, _: PlayerId, _: Option<ObjectId>) -> crate::types::ManaCost {
    crate::types::ManaCost::free()
}

/// A spell cast from its caster's hand at a creature (CR 601.2a-i): with
/// the targets it names legal, it moves to the stack (601.2a), holding
/// exactly those targets (601.2c) and the zone it was cast from, goes on
/// top of the stack, and its casting is the one event (601.2i); with them
/// not, the cast is refused before anything is done (no move, nothing on
/// the stack, no event, and the card still holds no targets). For a caster
/// either player and a creature either player's; the spell costs nothing,
/// so what paying does is the payment proofs' (`paying`), and it has no X
/// and no additional cost.
fn casting() {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let card = registry.register(Box::new(Castable));
    let mut state = crate::proofs::cards::new_game();
    let creature = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, Some(1), Some(1));
    let caster = PlayerId(u8::from(kani::any::<bool>()));
    let spell = state.create_object(card, caster, Zone::Hand, None, None);
    state.priority_player = Some(caster);
    let events = state.events.len();

    let applied = crate::engine::actions::cast::cast_spell(
        &mut state, spell, &[Target::Object(creature)], None, None, &[], None, &[], &registry);

    // SAFETY: as in `targets_are_legal`.
    let legal = unsafe { *std::ptr::addr_of!(LEGAL) } == Some(true);
    let obj = state.get_object(spell).expect("still on the board");
    if legal {
        assert!(matches!(applied, crate::engine::Applied::Continue));
        assert!(asked_for(&[Act::Move(spell, Zone::Stack)]));
        assert!(matches!(state.stack.as_slice(), [StackEntry::Spell(s)] if *s == spell));
        assert!(matches!(obj.targets.as_slice(), [Target::Object(t)] if *t == creature));
        assert!(obj.cast_from_zone == Some(Zone::Hand) && !obj.cast_with_flashback);
        assert!(matches!(&state.events[events..], [GameEvent::SpellCast { player, object }] if *player == caster && *object == spell));
    } else {
        assert!(matches!(applied, crate::engine::Applied::ReturnNow));
        assert!(acted() == 0 && state.stack.is_empty() && state.events.len() == events && obj.targets.is_empty());
    }
    std::mem::forget((registry, state));
}

card_proof! {
    #[kani::stub(crate::engine::targeting::targets_are_legal, targets_are_legal)]
    #[kani::stub(crate::engine::demand_reserve, no_reserve)]
    /// [`casting`].
    fn a_spell_cast_at_a_legal_target_goes_on_the_stack_holding_it_and_an_illegal_one_is_refused() {
        casting();
    }
}
