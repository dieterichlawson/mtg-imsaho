use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Curse of Oblivion — {3}{B} Enchantment — Aura Curse.
/// Enchant player.
/// At the beginning of enchanted player's upkeep, that player exiles two cards
/// from their graveyard.
pub struct CurseOfOblivion;

impl CardBehavior for CurseOfOblivion {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Curse of Oblivion".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Enchantment],
            subtypes: vec!["Aura".into(), "Curse".into()],
            oracle_text: "Enchant player\nAt the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "enchanted player exiles two cards from their graveyard".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_curse(state, object_id, targets, registry);
    }

    /// "At the beginning of ENCHANTED PLAYER's upkeep" — CR 603.2: the trigger
    /// event is that player's upkeep beginning, so it must not go on the stack
    /// during anyone else's.
    fn step_trigger_scope(&self, kind: &TriggerKind, _is_back_face: bool) -> crate::cards::TriggerScope {
        match kind {
            TriggerKind::Upkeep => crate::cards::TriggerScope::AttachedPlayer,
            _ => crate::cards::TriggerScope::Each,
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        // CR 113.7a: destroying the Curse in response does not counter its
        // trigger, and `attached_player` still knows whom it cursed.
        //
        // Whose upkeep this is was settled when the trigger was collected —
        // `TriggerScope::AttachedPlayer` in `triggers/collect/timing.rs`
        // implements CR 603.2 for every Curse at once. Re-checking it here
        // would also be wrong: once the ability is on the stack it resolves
        // whatever the turn has done since.
        let Some(cursed_player) = state.attached_player(self_id) else { return };
        // "That player exiles two cards from their graveyard."
        // The cursed player chooses which cards to exile.
        // "exiles two **cards** from their graveyard" — CR 109.1, so a token
        // sitting in the graveyard until the next SBA check is not one.
        let gy_cards: Vec<Target> = state.objects_in_zone(Zone::Graveyard, cursed_player)
            .iter()
            .filter(|o| state.is_card(o.id))
            .map(|o| Target::Object(o.id))
            .collect();
        // "Exiles two cards" is one question with two answers, not two
        // questions: asking twice is two screens of the same graveyard.
        // Fewer than two cards there and it takes what there is (CR 
        // 608.2: do as much as you can).
        let ids: Vec<ObjectId> = gy_cards.iter()
            // This list was built from the graveyard a line above, so it
            // holds only `Target::Object`.
            .filter_map(|t| match t { Target::Object(id) => Some(*id), _ => None })
            .collect();
        if ids.is_empty() {
            return;
        }
        // Nothing to decide when the graveyard holds exactly what is owed.
        if ids.len() <= 2 {
            let count = ids.len();
            for id in ids {
                state.move_object(id, Zone::Exile, registry);
            }
            state.log(crate::state::LogLevel::Event,
                format!("Curse of Oblivion: p{} exiled {count} card(s) from graveyard",
                    cursed_player.0));
            return;
        }
        let owed = 2.min(ids.len());
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: cursed_player,
            source: self_id,
            choice: ResolutionChoiceKind::ChooseObjectSet {
                description: format!(
                    "Curse of Oblivion: choose {owed} cards to exile from your graveyard"),
                options: ids,
                min: owed,
                max: owed,
                effect: PendingEffect::CardEffect { source_id: self_id, key: String::new() },
            },
        });
    }

    /// One exile, run once per card the set prompt above came back with.
    ///
    /// It used to be the card's own loop, re-raising a prompt from inside
    /// the answer to the last one and carrying the remaining count in `key`.
    fn resolve_card_effect(&self, state: &mut GameState, _source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Object(id) = target else { return };
        let owner = state.get_object(*id).map_or(crate::ids::PlayerId(0), |o| o.owner);
        state.move_object(*id, Zone::Exile, registry);
        state.log(crate::state::LogLevel::Event,
            format!("Curse of Oblivion: exiled a card from p{}'s graveyard", owner.0));
    }
}
