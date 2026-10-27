//! The werewolves: double-faced creatures whose front face reads "At the
//! beginning of each upkeep, if no spells were cast last turn, transform
//! this creature." and whose back face reads "At the beginning of each
//! upkeep, if a player cast two or more spells last turn, transform this
//! creature." (CR 712, 701.27).
//!
//! Proved for each of the twelve: the trigger is each upkeep's, not only
//! its controller's (CR 503.1a); it triggers exactly when its face's
//! condition holds of last turn's spells (an intervening "if", 603.4), the
//! face being the one up as the upkeep begins; and as it resolves it asks
//! to transform the werewolf exactly when it is still on the battlefield
//! and the condition still holds, read from the face it triggered from
//! (603.4, 701.27f), and nothing else changes. Each face's characteristics
//! are its Oracle record's, checked for every card; the transform itself is
//! the engine's, proved in `proofs::actions`.
//!
//! Last turn's spells are any number from 0 to 3 for each player.

use super::{acted, asked_for, board, card_proof, triggers, Act, Snapshot};
use crate::cards::{CardBehavior, TriggerKind, TriggerScope};
use crate::ids::PlayerId;
use crate::types::Zone;

/// Any count of spells, 0 to 3.
fn any_count() -> u32 {
    let n: u32 = kani::any();
    kani::assume(n <= 3);
    n
}

fn werewolf(card: Box<dyn CardBehavior>, behavior: &dyn CardBehavior) {
    for back in [false, true] {
        assert!(behavior.step_trigger_scope(&TriggerKind::Upkeep, back) == TriggerScope::Each);
    }
    assert!(triggers(behavior).iter().any(|(kind, _)| *kind == TriggerKind::Upkeep));
    let back = std::mem::ManuallyDrop::new(behavior.back_face_data().expect("a back face"));
    assert!(back.triggered_abilities.iter().any(|t| t.kind == TriggerKind::Upkeep));

    let (registry, mut state, _, card) = board(card);
    let on: bool = kani::any();
    let id = state.create_object(card, PlayerId(u8::from(kani::any::<bool>())),
        if on { Zone::Battlefield } else { Zone::Graveyard }, Some(1), Some(1));
    let transformed = on && kani::any::<bool>();
    state.get_object_mut(id).expect("just created").is_transformed = transformed;
    let (a, b) = (any_count(), any_count());
    state.num_spells_cast_last_turn.insert(PlayerId(0), a);
    state.num_spells_cast_last_turn.insert(PlayerId(1), b);
    let holds = |back: bool| if back { a >= 2 || b >= 2 } else { a + b == 0 };

    // As the upkeep begins: the face that is up.
    assert!(behavior.should_trigger(&state, id, &TriggerKind::Upkeep, &registry) == holds(transformed));

    // As it resolves: the face it triggered from, which may no longer be up.
    let from_back: bool = kani::any();
    state.resolving_trigger_from_back_face = Some(from_back);
    let before = Snapshot::of(&state);
    behavior.on_upkeep(&mut state, id, &[], &registry);
    if on && holds(from_back) {
        assert!(asked_for(&[Act::Transform(id)]));
    } else {
        assert!(acted() == 0);
    }
    assert!(Snapshot::of(&state) == before);
    kani::cover!(on && holds(from_back) && from_back, "a night face turns back");
    std::mem::forget((registry, state));
}

macro_rules! werewolves {
    ($($name:ident: $card:expr),* $(,)?) => {
        $(
            card_proof! {
                fn $name() {
                    werewolf(Box::new($card), &$card);
                }
            }
        )*
    };
}

use crate::cards::isd as isd;

werewolves! {
    daybreak_ranger_transforms_as_a_werewolf: isd::daybreak_ranger::DaybreakRanger,
    gatstaf_shepherd_transforms_as_a_werewolf: isd::gatstaf_shepherd::GatstafShepherd,
    grizzled_outcasts_transforms_as_a_werewolf: isd::grizzled_outcasts::GrizzledOutcasts,
    hanweir_watchkeep_transforms_as_a_werewolf: isd::hanweir_watchkeep::HanweirWatchkeep,
    instigator_gang_transforms_as_a_werewolf: isd::instigator_gang::InstigatorGang,
    kruin_outlaw_transforms_as_a_werewolf: isd::kruin_outlaw::KruinOutlaw,
    mayor_of_avabruck_transforms_as_a_werewolf: isd::mayor_of_avabruck::MayorOfAvabruck,
    reckless_waif_transforms_as_a_werewolf: isd::reckless_waif::RecklessWaif,
    tormented_pariah_transforms_as_a_werewolf: isd::tormented_pariah::TormentedPariah,
    ulvenwald_mystics_transforms_as_a_werewolf: isd::ulvenwald_mystics::UlvenwaldMystics,
    village_ironsmith_transforms_as_a_werewolf: isd::village_ironsmith::VillageIronsmith,
    villagers_of_estwald_transforms_as_a_werewolf: isd::villagers_of_estwald::VillagersOfEstwald,
}
