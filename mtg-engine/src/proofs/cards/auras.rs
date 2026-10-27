//! The Auras that enchant a creature, and what each one's text gives it.
//!
//! An Aura spell's resolution is its becoming attached to the creature it
//! targets (CR 303.4f, 608.3a): proved here for each, with the zone change
//! and the attachment recorded. What it then does is the continuous
//! effects it prints, pinned here to its Oracle text, and applied by the
//! engine as `proofs::actions::effects` proves for each kind. "Enchant
//! creature" is the requirement "target creature" (CR 303.4a, 115.1),
//! proved for each Aura here and in `proofs::actions::targeting`.
//!
//! - Holy Strength: "Enchanted creature gets +1/+2."
//! - Pacifism: "Enchanted creature can't attack or block."
//! - Bonds of Faith: "Enchanted creature gets +2/+2 as long as it's a
//!   Human. Otherwise, it can't attack or block."
//! - Claustrophobia: "When this Aura enters, tap enchanted creature.
//!   Enchanted creature doesn't untap during its controller's untap step."
//!   (its trigger proved below)
//! - Curiosity: "Whenever enchanted creature deals damage to an opponent,
//!   you may draw a card." (its trigger proved below)
//! - Dead Weight: "Enchanted creature gets -2/-2."
//! - Furor of the Bitten: "Enchanted creature gets +2/+2 and attacks each
//!   combat if able."
//! - Ghostly Possession: "Enchanted creature has flying. Prevent all combat
//!   damage that would be dealt to and dealt by enchanted creature."
//! - Gruesome Deformity: "Enchanted creature has intimidate."
//! - Sensory Deprivation: "Enchanted creature gets -3/-0."
//! - Skeletal Grimace: "Enchanted creature gets +1/+1 and has '{B}:
//!   Regenerate this creature.'" (the granted ability proved below)
//! - Spectral Flight: "Enchanted creature gets +2/+2 and has flying."
//! - Wreath of Geists: "Enchanted creature gets +X/+X, where X is the
//!   number of creature cards in your graveyard." (its X proved in
//!   `proofs::cards::counted_pt`)

use super::{acted, any_object, asked_for, board, card_proof, prints_effects, prints_where, triggers, Act, Form, Snapshot, ANY_PERMANENT};
use crate::actions::Target;
use crate::cards::{CardBehavior, TargetRequirement, TriggerKind};
use crate::state::{AwaitingAction, ResolutionChoiceKind};
use crate::ids::PlayerId;
use crate::types::{Color, ContinuousEffect as E, EffectCondition, EffectScope::Attached, Keyword, ManaCost, ManaSymbol, Zone};

/// `card`, an Aura spell, resolving holding the other object on the board
/// (any permanent or card) or a target gone illegal: it asks to move onto
/// the battlefield and become attached to that permanent if it still is
/// one, and for nothing otherwise; nothing else changes.
fn attaches(card: Box<dyn CardBehavior>) {
    let (registry, mut state, blank, card) = board(card);
    let other = any_object(&mut state, blank, &ANY_PERMANENT, Form::Card);
    let aura = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let target = if kani::any() { Target::Object(other) } else { Target::Illegal };
    let before = Snapshot::of(&state);
    registry.get(card).expect("registered").on_resolve(&mut state, aura, &[target.clone()], &registry);
    let on = target == Target::Object(other) && before_zone(&state, other) == Zone::Battlefield;
    if on {
        assert!(asked_for(&[Act::Move(aura, Zone::Battlefield), Act::Attach(aura, other)]));
    } else {
        assert!(acted() == 0);
    }
    assert!(Snapshot::of(&state) == before);
    kani::cover!(on, "attached");
    std::mem::forget((registry, state));
}

fn before_zone(state: &crate::state::GameState, id: crate::ids::ObjectId) -> Zone {
    state.get_object(id).expect("on the board").zone
}

/// `card` enchants a creature, and prints exactly `effects`.
fn prints(card: &dyn CardBehavior, effects: &[E]) {
    assert!(card.target_requirement() == TargetRequirement::Creature);
    assert!(prints_effects(card, effects));
}

macro_rules! aura {
    ($attaches:ident, $prints:ident, $card:expr, where $shape:expr) => {
        card_proof! {
            fn $attaches() {
                attaches(Box::new($card));
            }
        }
        card_proof! {
            fn $prints() {
                assert!($card.target_requirement() == TargetRequirement::Creature);
                assert!(prints_where(&$card, $shape));
            }
        }
    };
    ($attaches:ident, $prints:ident, $card:expr, [$($effect:expr),* $(,)?]) => {
        card_proof! {
            fn $attaches() {
                attaches(Box::new($card));
            }
        }
        card_proof! {
            fn $prints() {
                prints(&$card, &[$($effect),*]);
            }
        }
    };
}

aura!(holy_strength_attaches, holy_strength_gives_plus_one_plus_two, crate::cards::holy_strength::HolyStrength,
    [E::ModifyPT { power: 1, toughness: 2, scope: Attached }]);
aura!(pacifism_attaches, pacifism_stops_attacking_and_blocking, crate::cards::pacifism::Pacifism,
    [E::PreventAttack { scope: Attached }, E::PreventBlock { scope: Attached }]);
aura!(bonds_of_faith_attaches, bonds_of_faith_pumps_a_human_and_holds_anything_else, crate::cards::isd::bonds_of_faith::BondsOfFaith,
    where |e| matches!(e, [
        E::When { condition: EffectCondition::AttachedHasSubtype(a), effect: pump },
        E::When { condition: EffectCondition::AttachedLacksSubtype(b), effect: no_attack },
        E::When { condition: EffectCondition::AttachedLacksSubtype(c), effect: no_block },
    ] if a == "Human" && b == "Human" && c == "Human"
        && matches!(**pump, E::ModifyPT { power: 2, toughness: 2, scope: Attached })
        && matches!(**no_attack, E::PreventAttack { scope: Attached })
        && matches!(**no_block, E::PreventBlock { scope: Attached })));
aura!(claustrophobia_attaches, claustrophobia_keeps_it_tapped, crate::cards::isd::claustrophobia::Claustrophobia,
    [E::PreventUntap { scope: Attached }]);
aura!(curiosity_attaches, curiosity_prints_no_static_effect, crate::cards::isd::curiosity::Curiosity, []);
aura!(dead_weight_attaches, dead_weight_gives_minus_two_minus_two, crate::cards::isd::dead_weight::DeadWeight,
    [E::ModifyPT { power: -2, toughness: -2, scope: Attached }]);
aura!(furor_of_the_bitten_attaches, furor_of_the_bitten_pumps_and_forces_attacks, crate::cards::isd::furor_of_the_bitten::FurorOfTheBitten,
    [E::ModifyPT { power: 2, toughness: 2, scope: Attached }, E::ForceAttack { scope: Attached }]);
aura!(ghostly_possession_attaches, ghostly_possession_gives_flying_and_prevents_combat_damage, crate::cards::isd::ghostly_possession::GhostlyPossession,
    [E::GrantKeyword { keyword: Keyword::Flying, scope: Attached }, E::PreventCombatDamage { scope: Attached }]);
aura!(gruesome_deformity_attaches, gruesome_deformity_gives_intimidate, crate::cards::isd::gruesome_deformity::GruesomeDeformity,
    [E::GrantKeyword { keyword: Keyword::Intimidate, scope: Attached }]);
aura!(sensory_deprivation_attaches, sensory_deprivation_gives_minus_three_minus_zero, crate::cards::isd::sensory_deprivation::SensoryDeprivation,
    [E::ModifyPT { power: -3, toughness: 0, scope: Attached }]);
aura!(skeletal_grimace_attaches, skeletal_grimace_gives_plus_one_plus_one, crate::cards::isd::skeletal_grimace::SkeletalGrimace,
    [E::ModifyPT { power: 1, toughness: 1, scope: Attached }]);
aura!(spectral_flight_attaches, spectral_flight_gives_plus_two_plus_two_and_flying, crate::cards::isd::spectral_flight::SpectralFlight,
    [E::ModifyPT { power: 2, toughness: 2, scope: Attached }, E::GrantKeyword { keyword: Keyword::Flying, scope: Attached }]);
aura!(wreath_of_geists_attaches, wreath_of_geists_prints_no_fixed_effect, crate::cards::isd::wreath_of_geists::WreathOfGeists, []);

/// An Aura on the battlefield under either player, attached to a creature
/// of either player's, or gone from the battlefield since, having been
/// attached to it (`card_state["last_attached_to"]`, which `move_object`
/// writes as it leaves); the board, the Aura, its creature and its
/// controller.
fn on_a_creature(card: Box<dyn CardBehavior>) -> (crate::cards::CardRegistry, crate::state::GameState, crate::ids::ObjectId, crate::ids::ObjectId, PlayerId) {
    let (registry, mut state, blank, card) = board(card);
    let creature = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(creature).expect("just created").card_types = vec![crate::types::CardType::Creature];
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let gone: bool = kani::any();
    let aura = state.create_object(card, PlayerId(0), if gone { Zone::Graveyard } else { Zone::Battlefield }, None, None);
    let o = state.get_object_mut(aura).expect("just created");
    if gone {
        o.last_controller = Some(you);
        o.card_state.insert("last_attached_to".into(), creature);
        state.players[0].graveyard_order.push(aura);
    } else {
        o.controller = you;
        o.attached_to = Some(creature);
    }
    (registry, state, aura, creature, you)
}

card_proof! {
    /// Claustrophobia's trigger: as it enters (CR 603.6a), and resolving,
    /// the creature it enchants tapped: the one it enchanted as it left, if
    /// it has left by then (CR 608.2h, last known information). Nothing
    /// else.
    fn claustrophobia_taps_the_creature_it_enchants() {
        let claustrophobia = crate::cards::isd::claustrophobia::Claustrophobia;
        assert!(claustrophobia.has_etb_handler() && triggers(&claustrophobia) == [(TriggerKind::EntersBattlefield, None)]);
        let (registry, mut state, aura, creature, _) = on_a_creature(Box::new(claustrophobia));
        let before = Snapshot::of(&state);
        let card = state.get_object(aura).expect("on the board").card_id;
        registry.get(card).expect("registered").on_enter_battlefield(&mut state, aura, &[], &registry);
        assert!(asked_for(&[Act::Tap(creature)]) && Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Curiosity: triggers for damage dealt to a player by the creature it
    /// enchants, on the battlefield, when that player is an opponent of its
    /// controller, and no other; resolving, its controller is asked
    /// whether to draw ("you may"), and on yes draws a card. Nothing else.
    fn curiosity_may_draw_when_its_creature_damages_an_opponent() {
        let curiosity = crate::cards::isd::curiosity::Curiosity;
        assert!(triggers(&curiosity) == [(TriggerKind::AnyDamageToPlayer, None)]);
        let (registry, mut state, aura, creature, you) = on_a_creature(Box::new(curiosity));
        let card = state.get_object(aura).expect("on the board").card_id;
        let behavior = registry.get(card).expect("registered");
        let source = if kani::any() { creature } else { aura };
        let damaged = PlayerId(u8::from(kani::any::<bool>()));
        let on = state.get_object(aura).expect("on the board").zone == Zone::Battlefield;
        assert!(behavior.should_trigger_on_damage_to_player(&state, aura, source, damaged, &registry)
            == (on && source == creature && damaged != you));
        let before = Snapshot::of(&state);
        behavior.on_any_damage_to_player(&mut state, aura, creature, damaged, 1, &registry);
        let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source, choice: ResolutionChoiceKind::YesNo { source_card, .. },
        }) if *player == you && *source == aura && *source_card == aura);
        assert!(asked && acted() == 0);
        state.awaiting_action = None;
        let yes: bool = kani::any();
        behavior.on_yes_no_choice(&mut state, aura, yes, &registry);
        if yes {
            assert!(asked_for(&[Act::Draw(you, 1)]));
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Skeletal Grimace's granted ability, asked of the creature it
    /// enchants: "{B}: Regenerate this creature" while that is a creature
    /// on the battlefield, and not otherwise (the engine asks the Aura
    /// about the creature it is attached to, and offers what it grants to
    /// that creature's controller, CR 602.2); resolving, a regeneration
    /// shield on that creature. Asked of anything else, which
    /// the Aura itself is, nothing.
    fn skeletal_grimace_gives_its_creature_regeneration() {
        let (registry, mut state, aura, creature, _) = on_a_creature(Box::new(crate::cards::isd::skeletal_grimace::SkeletalGrimace));
        let card = state.get_object(aura).expect("on the board").card_id;
        let behavior = registry.get(card).expect("registered");
        let creature_on: bool = kani::any();
        if !creature_on {
            state.get_object_mut(creature).expect("on the board").zone = Zone::Exile;
        }
        let defs = std::mem::ManuallyDrop::new(behavior.activated_abilities(&state, creature, &registry));
        if creature_on {
            let a = &defs[..];
            assert!(a.len() == 1 && a[0].ability_index == 0 && a[0].cost == ManaCost::new(vec![ManaSymbol::Colored(Color::Black)])
                && !a[0].requires_tap && a[0].target_requirement.is_none() && a[0].sacrifice_cost == crate::cards::SacrificeCost::None
                && !a[0].once_per_turn && !a[0].sorcery_speed_only && a[0].counter_cost.is_none() && a[0].tap_cost.is_none());
        } else {
            assert!(defs.is_empty());
        }
        assert!(behavior.activated_abilities(&state, aura, &registry).is_empty());
        let before = Snapshot::of(&state);
        behavior.resolve_activated_ability(&mut state, creature, 0, &[], &registry);
        assert!(asked_for(&[Act::Regenerate(creature)]) && Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}
