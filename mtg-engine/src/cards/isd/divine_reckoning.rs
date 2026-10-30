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
    fn resolve_card_effect(&self, state: &mut GameState, source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Object(id) = target else { return };
        let (mut kept, remaining) = Self::load(state, source_id);
        let chooser = crate::cards::helpers::controller_of(state, *id);
        state.log(LogLevel::Event,
            format!("Divine Reckoning: p{} keeps {}", chooser.0, state.obj_name(*id)));
        kept.push(*id);
        Self::advance(state, source_id, kept, remaining, registry);
    }
}

impl DivineReckoning {
    /// The chain state — which creatures are already spoken for, and which
    /// players have yet to choose — kept on the spell while a player
    /// chooses (`cards::store_ids`), as Liliana of the Veil keeps hers.
    ///
    /// It used to round-trip through the `CardEffect` key as text built with
    /// `format!`, which is for what a person reads: the proofs stub it to
    /// return nothing (`proofs::cards` says why), and a chain whose state
    /// is lost between choices is not one they can follow.
    const KEPT: &'static str = "reckoning_kept";
    const QUEUE: &'static str = "reckoning_queue";

    fn store(state: &mut GameState, source_id: ObjectId, kept: &[ObjectId], remaining: &[PlayerId]) {
        let Some(obj) = state.get_object_mut(source_id) else { return };
        crate::cards::store_ids(&mut obj.card_state, Self::KEPT, kept);
        let players: Vec<ObjectId> = remaining.iter().map(|p| ObjectId(u64::from(p.0))).collect();
        crate::cards::store_ids(&mut obj.card_state, Self::QUEUE, &players);
    }

    fn load(state: &GameState, source_id: ObjectId) -> (Vec<ObjectId>, Vec<PlayerId>) {
        let Some(obj) = state.get_object(source_id) else { return (Vec::new(), Vec::new()) };
        let kept = crate::cards::load_ids(&obj.card_state, Self::KEPT);
        let remaining = crate::cards::load_ids(&obj.card_state, Self::QUEUE).into_iter()
            .map(|id| PlayerId(u8::try_from(id.0).unwrap_or(u8::MAX)))
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
            // In id order (`ids_in_zone`, as `objects_in_zone` lists the
            // battlefield). The player picks from this list by position, so
            // an unstable order makes the same game replay differently.
            let options: Vec<Target> = state.ids_in_zone(Zone::Battlefield, player, |o| state.is_creature(o.id, registry))
                .into_iter()
                .map(Target::Object)
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
                    Self::store(state, source_id, &kept, &queue);
                    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                        player,
                        source: source_id,
                        choice: ResolutionChoiceKind::ChooseTarget {
                            description: "Divine Reckoning: choose a creature you control to keep".into(),
                            options,
                            optional: false,
                            effect: PendingEffect::CardEffect {
                                source_id,
                                key: String::new(),
                            },
                        },
                    });
                    return;
                }
            }
        }

        // Everyone has chosen — destroy the rest.
        if let Some(obj) = state.get_object_mut(source_id) {
            crate::cards::clear_ids(&mut obj.card_state, Self::KEPT);
            crate::cards::clear_ids(&mut obj.card_state, Self::QUEUE);
        }
        // Sorted by id (`all_ids_in_zone`) for a reproducible log; the
        // destruction itself is simultaneous so the order does not affect the
        // outcome.
        let doomed = state.all_ids_in_zone(Zone::Battlefield,
            |o| state.is_creature(o.id, registry) && !kept.contains(&o.id));
        // "Destroy the rest" is one event (CR 700.2c). Destroying them one at
        // a time lets each death change the answer for the next — an Angelic
        // Overseer and the last Human its controller has are both doomed here,
        // and the Overseer must survive because that Human is still alive when
        // destruction happens.
        crate::destruction::try_destroy_all(state, &doomed, registry);
    }
}
