//! What a spell costs as the engine works it out (`engine::cost_to_cast`,
//! `engine::alternative_costs`), with a permanent on the board that
//! prints a change to it: Heartless Summoning's "Creature spells you cast
//! cost {2} less to cast" (CR 601.2f) and Rooftop Storm's "You may pay
//! {0} rather than pay the mana cost for Zombie creature spells you cast"
//! (118.9), each the card itself; and what Stony Silence's and
//! Nevermore's prohibitions stop. A card's own proof shows what it
//! prints; these, what the engine makes of it.

use crate::cards::isd::heartless_summoning::HeartlessSummoning;
use crate::cards::isd::rooftop_storm::RooftopStorm;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::engine::{alternative_costs, cost_to_cast, CastMethod};
use crate::ids::{CardId, PlayerId};
use crate::proofs::cards::{case_by_case, card_proof};
use crate::state::GameState;
use crate::types::{CardType, Color, ManaCost, ManaSymbol, Zone};

/// A spell of a constant cost, a creature or an instant, a Zombie or not.
struct Spell {
    cost: &'static [ManaSymbol],
    creature: bool,
    zombie: bool,
}

impl CardBehavior for Spell {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Spell".into(),
            cost: Some(ManaCost::new(self.cost.to_vec())),
            card_types: vec![if self.creature { CardType::Creature } else { CardType::Instant }],
            subtypes: if self.zombie { vec!["Zombie".into()] } else { Vec::new() },
            ..CardData::default()
        }
    }
}

/// `printer` (either player's, on the battlefield or in a graveyard) and
/// `spell`, cast by either player; and whether the permanent's effect is
/// one the caster has: on the battlefield and the caster's.
fn with_a_permanent(printer: Box<dyn CardBehavior>, spell: Spell) -> (CardRegistry, GameState, CardId, PlayerId, bool) {
    let mut registry = CardRegistry::new();
    let printer = registry.register(printer);
    let spell = registry.register(Box::new(spell));
    let mut state = crate::proofs::cards::new_game();
    let controller = PlayerId(u8::from(kani::any::<bool>()));
    let on: bool = kani::any();
    state.create_object(printer, controller, if on { Zone::Battlefield } else { Zone::Graveyard }, None, None);
    let caster = PlayerId(u8::from(kani::any::<bool>()));
    (registry, state, spell, caster, on && controller == caster)
}

/// Heartless Summoning's reduction: a creature spell that the controller
/// of a Heartless Summoning on the battlefield casts costs `reduced`, its
/// cost with up to two generic mana taken off and its colored mana kept
/// (CR 601.2f); a noncreature spell, one the other player casts, or one
/// cast with Heartless Summoning off the battlefield, costs its own.
fn reducing((cost, creature, reduced): (&'static [ManaSymbol], bool, &'static [ManaSymbol])) {
    let (registry, state, spell, caster, has) = with_a_permanent(Box::new(HeartlessSummoning), Spell { cost, creature, zombie: false });
    let paid = cost_to_cast(&state, &registry, spell, caster, &CastMethod::Normal);
    let expected = if has && creature { reduced } else { cost };
    assert!(paid.mana.symbols == expected && paid.x_discount == 0);
    std::mem::forget((registry, state, paid));
}

case_by_case! { reducing;
    a_creature_spell_of_three_and_black_costs_one_and_black_less_two: (&[ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Black)], true, &[ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Black)]),
    a_creature_spell_of_one_and_black_costs_black_less_two: (&[ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Black)], true, &[ManaSymbol::Colored(Color::Black)]),
    a_creature_spell_of_black_costs_black_less_two: (&[ManaSymbol::Colored(Color::Black)], true, &[ManaSymbol::Colored(Color::Black)]),
    a_creature_spell_of_two_costs_nothing_less_two: (&[ManaSymbol::Generic(2)], true, &[]),
    an_instant_of_three_and_black_costs_its_own_less_two_for_creatures: (&[ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Black)], false, &[ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Black)]),
}

/// Rooftop Storm's alternative cost: the controller of a Rooftop Storm on
/// the battlefield may cast a Zombie creature spell for {0} instead of its
/// mana cost (CR 118.9), one way to; for a creature spell that is no
/// Zombie, a noncreature spell, the other player, or with Rooftop Storm
/// off the battlefield, there is no other cost.
fn paying_nothing_instead((creature, zombie): (bool, bool)) {
    let (registry, state, spell, caster, has) = with_a_permanent(Box::new(RooftopStorm),
        Spell { cost: &[ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Black)], creature, zombie });
    let alternatives = alternative_costs(&state, &registry, spell, caster);
    if has && creature && zombie {
        assert!(alternatives.len() == 1 && alternatives[0].symbols.is_empty());
    } else {
        assert!(alternatives.is_empty());
    }
    std::mem::forget((registry, state, alternatives));
}

case_by_case! { paying_nothing_instead;
    a_zombie_creature_spell_may_be_cast_for_nothing: (true, true),
    a_creature_spell_that_is_no_zombie_has_no_other_cost: (true, false),
    a_zombie_instant_has_no_other_cost: (false, true),
}

card_proof! {
    /// "Activated abilities of artifacts can't be activated" (Stony
    /// Silence's), mana abilities among them (CR 605.1a): while a Stony
    /// Silence is on the battlefield, Sol Ring has no mana ability its
    /// controller may activate, and a Forest keeps its own; with Stony
    /// Silence in a graveyard, each has its own. Each of the three either
    /// player's, Sol Ring and the Forest untapped, sick or not (a
    /// noncreature's {T} does not wait, CR 302.6).
    fn stony_silence_stops_an_artifacts_mana_abilities_and_not_a_lands() {
        let mut registry = CardRegistry::new();
        let silence = registry.register(Box::new(crate::cards::isd::stony_silence::StonySilence));
        let ring = registry.register(Box::new(crate::cards::sol_ring::SolRing));
        let forest = registry.register(Box::new(crate::cards::forest::Forest));
        let mut state = crate::proofs::cards::new_game();
        let on: bool = kani::any();
        state.create_object(silence, PlayerId(u8::from(kani::any::<bool>())), if on { Zone::Battlefield } else { Zone::Graveyard }, None, None);
        let mut sources = [crate::ids::ObjectId(0); 2];
        for (id, card) in sources.iter_mut().zip([ring, forest]) {
            *id = state.create_object(card, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, None, None);
            state.get_object_mut(*id).expect("just created").summoning_sick = kani::any();
        }
        assert!(crate::engine::prevents_artifact_abilities(&state, &registry) == on);
        assert!(crate::engine::available_mana_abilities(&state, sources[0], &registry).len() == usize::from(!on));
        assert!(crate::engine::available_mana_abilities(&state, sources[1], &registry).len() == 1);
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// "Spells with the chosen name can't be cast" (Nevermore's, its name
    /// chosen as it entered and kept as an effect of its own): while it is
    /// on the battlefield, the name chosen is the one name no spell of
    /// which may be cast, and with it in a graveyard there is none. Either
    /// player's, with a permanent of the blank card beside it.
    fn nevermore_bans_exactly_the_chosen_name_while_it_is_on_the_battlefield() {
        let mut registry = CardRegistry::new();
        let nevermore = registry.register(Box::new(crate::cards::isd::nevermore::Nevermore));
        let blank = registry.register(Box::new(crate::proofs::cards::Blank));
        let mut state = crate::proofs::cards::new_game();
        let on: bool = kani::any();
        let id = state.create_object(nevermore, PlayerId(u8::from(kani::any::<bool>())), if on { Zone::Battlefield } else { Zone::Graveyard }, None, None);
        state.get_object_mut(id).expect("just created").instance_continuous_effects =
            Some(vec![crate::types::ContinuousEffect::PreventCastingNamed { name: "a named spell".into() }]);
        state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, None, None);
        let banned = crate::engine::names_that_cant_be_cast(&state, &registry);
        if on {
            assert!(banned.len() == 1 && banned[0] == "a named spell");
        } else {
            assert!(banned.is_empty());
        }
        std::mem::forget(banned);
        std::mem::forget((registry, state));
    }
}
