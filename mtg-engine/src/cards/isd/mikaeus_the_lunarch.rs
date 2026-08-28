use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Supertype, Zone, CounterType};

/// Mikaeus, the Lunarch {X}{W} 0/0 Legendary Human Cleric.
/// Mikaeus enters the battlefield with X +1/+1 counters on it.
/// {T}: Put a +1/+1 counter on Mikaeus.
/// {T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature
/// you control.
pub struct MikaeusTheLunarch;

impl CardBehavior for MikaeusTheLunarch {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Mikaeus, the Lunarch".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::X,
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![Supertype::Legendary],
            subtypes: vec!["Human".into(), "Cleric".into()],
            power: Some(0),
            toughness: Some(0),
            oracle_text: "Mikaeus enters with X +1/+1 counters on it.\n{T}: Put a +1/+1 counter on Mikaeus.\n{T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature you control.".into(),
            ..Default::default()
        }
    }

    /// "Mikaeus enters with X +1/+1 counters on it" is a replacement effect
    /// (CR 614.1c), not something to do to it after it has arrived.
    ///
    /// This used to override `on_resolve` to `move_object` and then
    /// `add_counters`. `move_object` emits `EnteredBattlefield` as part of the
    /// move, so every ETB watcher saw a 0/0 Mikaeus and the counters landed
    /// afterwards — and a 0/0 creature is one state-based-action check away
    /// from the graveyard (CR 704.5f). The same override also re-did the trait
    /// default's "move a permanent to the battlefield" and its `is_legendary`
    /// stamping.
    fn replace_event(
        &self,
        state: &mut GameState,
        self_id: ObjectId,
        event: &crate::replacement::ReplaceableEvent,
        _registry: &CardRegistry,
    ) -> Option<crate::replacement::Replacement> {
        let x = state.get_object(self_id).and_then(|o| o.x_value).unwrap_or(0);
        crate::cards::helpers::enters_with_counters(self_id, event, || {
            if x > 0 { vec![(CounterType::PlusOnePlusOne, x)] } else { vec![] }
        })
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => {}
            _ => return vec![],
        }

        let mut abilities = vec![];

        // Ability 0: {T}: Put a +1/+1 counter on Mikaeus.
        abilities.push(ActivatedAbilityDef {
            ability_index: 0,
            description: "{T}: Put a +1/+1 counter on Mikaeus".into(),
            cost: ManaCost::free(),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
        });

        // Ability 1: {T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1
        // counter on each other creature you control.
        //
        // Everything before the colon is cost (CR 602.2b), so the removal is
        // declared as `counter_cost` and paid on activation. It used to be
        // done by hand in `resolve_activated_ability`, which put it on the
        // wrong side of the priority window — an opponent responding still saw
        // the counter on Mikaeus, and countering the ability handed it back.
        // Declaring it also lets the engine enforce CR 601.2h; the card no
        // longer hides the ability itself when there is no counter to remove.
        abilities.push(ActivatedAbilityDef {
            ability_index: 1,
            description: "{T}, Remove a +1/+1 counter: +1/+1 counter on each other creature you control".into(),
            cost: ManaCost::free(),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: Some((CounterType::PlusOnePlusOne, 1)),
        });

        abilities
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        // CR 602.2a: an activated ability's controller is the player who
        // activated it, which the engine records; CR 608.2g falls back to the
        // source's last known controller. Reading `o.controller` here gave the
        // *current* controller, so an opponent taking the permanent in
        // response to the ability collected the effect — and `None => return`
        // threw the whole effect away if the source had left, against
        // CR 113.7a.
        let controller = crate::cards::helpers::ability_controller(state, object_id);

        match ability_index {
            0 => {
                // Put a +1/+1 counter on Mikaeus.
                state.add_counters(object_id, CounterType::PlusOnePlusOne, 1);
                state.log(crate::state::LogLevel::Event,
                    "Mikaeus, the Lunarch: +1/+1 counter added".into());
            }
            1 => {
                // The counter removal was the cost, paid on activation.
                // Put a +1/+1 counter on each other creature you control.
                let other_creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller)
                    .iter()
                    .filter(|o| o.id != object_id && state.is_creature(o.id, registry))
                    .map(|o| o.id)
                    .collect();
                for cid in &other_creatures {
                    state.add_counters(*cid, CounterType::PlusOnePlusOne, 1);
                }
                state.log(crate::state::LogLevel::Event,
                    format!("Mikaeus, the Lunarch: +1/+1 counter on {} other creatures", other_creatures.len()));
            }
            _ => {}
        }
    }
}
