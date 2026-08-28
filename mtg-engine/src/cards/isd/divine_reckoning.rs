use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Divine Reckoning — {2}{W}{W} Sorcery.
/// Each player chooses a creature they control. Destroy the rest.
/// Flashback {5}{W}{W}.
pub struct DivineReckoning;

impl CardBehavior for DivineReckoning {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Divine Reckoning".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Each player chooses a creature they control. Destroy the rest.\nFlashback {5}{W}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        // Players choose in turn order starting with the active player (CR 101.4).
        let active = state.active_player;
        let mut player_order: Vec<PlayerId> = state.players.iter().map(|p| p.id).collect();
        if let Some(pos) = player_order.iter().position(|&p| p == active) {
            player_order.rotate_left(pos);
        }
        Self::advance(state, object_id, Vec::new(), player_order, registry);
    }

    /// Continue the chain after a player picked the creature they keep.
    fn resolve_card_effect(&self, state: &mut GameState, source_id: ObjectId, key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Object(id) = target else { return };
        let (mut kept, remaining) = Self::decode(key);
        let chooser = state.get_object(*id).map_or(PlayerId(0), |o| o.controller);
        state.log(LogLevel::Event,
            format!("Divine Reckoning: p{} keeps {}", chooser.0, state.obj_name(*id)));
        kept.push(*id);
        Self::advance(state, source_id, kept, remaining, registry);
    }
}

impl DivineReckoning {
    /// The chain state — which creatures are already spoken for, and which
    /// players have yet to choose — round-trips through the `CardEffect` key.
    /// The key is opaque to the engine by design: the shape of a card's
    /// intermediate state is the card's business, not something the engine
    /// should carry a variant for.
    fn encode(kept: &[ObjectId], remaining: &[PlayerId]) -> String {
        let k: Vec<String> = kept.iter().map(|o| o.0.to_string()).collect();
        let r: Vec<String> = remaining.iter().map(|p| p.0.to_string()).collect();
        format!("{}|{}", k.join(","), r.join(","))
    }

    fn decode(key: &str) -> (Vec<ObjectId>, Vec<PlayerId>) {
        let mut parts = key.splitn(2, '|');
        let kept = parts.next().unwrap_or("").split(',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .map(ObjectId)
            .collect();
        let remaining = parts.next().unwrap_or("").split(',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .map(PlayerId)
            .collect();
        (kept, remaining)
    }

    /// Walk the remaining players, auto-keeping for anyone with 0 or 1
    /// creature and stopping to ask anyone with a real choice. When nobody is
    /// left, "destroy the rest".
    ///
    /// Spell cleanup is engine-owned (CR 608.2m) and runs once the choice
    /// chain finishes, so this deliberately does not move the spell itself.
    fn advance(
        state: &mut GameState,
        source_id: ObjectId,
        mut kept: Vec<ObjectId>,
        remaining: Vec<PlayerId>,
        registry: &CardRegistry,
    ) {
        let mut queue = remaining;
        while let Some(player) = queue.first().copied() {
            queue.remove(0);
            // Through `objects_in_zone`, which sorts by id, rather than
            // `state.objects.values()`, which is a HashMap iterator in
            // arbitrary order. The player picks from this list by position, so
            // an unstable order makes the same game replay differently.
            let options: Vec<Target> = state.objects_in_zone(Zone::Battlefield, player)
                .iter()
                .filter(|o| state.is_creature(o.id, registry))
                .map(|o| Target::Object(o.id))
                .collect();

            match options.len() {
                0 => {}
                1 => {
                    if let Some(Target::Object(only)) = options.first() {
                        kept.push(*only);
                        state.log(LogLevel::Event,
                            format!("Divine Reckoning: p{} keeps {} (only creature)",
                                player.0, state.obj_name(*only)));
                    }
                }
                _ => {
                    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                        player,
                        source: source_id,
                        choice: ResolutionChoiceKind::ChooseTarget {
                            description: "Divine Reckoning: choose a creature you control to keep".into(),
                            options,
                            optional: false,
                            effect: PendingEffect::CardEffect {
                                source_id,
                                key: Self::encode(&kept, &queue),
                            },
                        },
                    });
                    return;
                }
            }
        }

        // Everyone has chosen — destroy the rest.
        let mut doomed: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && state.is_creature(o.id, registry))
            .map(|o| o.id)
            .filter(|id| !kept.contains(id))
            .collect();
        // Sorted for a reproducible log; the destruction itself is simultaneous
        // so the order does not affect the outcome.
        doomed.sort_by_key(|id| id.0);
        // "Destroy the rest" is one event (CR 700.2c). Destroying them one at
        // a time lets each death change the answer for the next — an Angelic
        // Overseer and the last Human its controller has are both doomed here,
        // and the Overseer must survive because that Human is still alive when
        // destruction happens.
        crate::destruction::try_destroy_all(state, &doomed, registry);
    }
}
