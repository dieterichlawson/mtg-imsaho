use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Night Terrors — {2}{B} Sorcery.
/// Target player reveals their hand. You choose a nonland card from it. Exile that card.
pub struct NightTerrors;

/// The names of the cards a hand reveals, joined with ", ", or "(empty)":
/// written only when the log line is. Joined first, the names were a
/// string built from every card of the hand, which a proof of the card
/// followed name by name, over an empty hand too.
struct Revealed<'a>(&'a GameState, &'a CardRegistry, &'a [ObjectId]);

impl std::fmt::Display for Revealed<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.2.is_empty() {
            return f.write_str("(empty)");
        }
        for (i, id) in self.2.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            f.write_str(&self.0.name_of(*id, self.1))?;
        }
        Ok(())
    }
}

impl CardBehavior for NightTerrors {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Night Terrors".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Target player reveals their hand. You choose a nonland card from it. Exile that card.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);
        if let Some(Target::Player(target_player)) = targets.first() {
            // "Target player reveals their hand" — the WHOLE hand becomes
            // public to every player (CR 701.16a), lands included, and the
            // log records what was shown. Only the selection below is
            // restricted to nonland cards; the reveal used to show nothing
            // but the choosable ones (issue #133).
            let hand = state.ids_in_zone(Zone::Hand, *target_player, |_| true);
            state.log(crate::state::LogLevel::Event, format!(
                "Night Terrors: p{} reveals hand: {}",
                target_player.0, Revealed(state, registry, &hand)));

            // Reveal target player's hand — find all nonland cards.
            let nonland_cards = state.ids_in_zone(Zone::Hand, *target_player, |o| {
                let is_land = state.face_card_types(o.id, registry).contains(&CardType::Land);
                !is_land
            });

            if nonland_cards.is_empty() {
                state.log(crate::state::LogLevel::Event,
                    format!("Night Terrors: no nonland card in p{}'s hand", target_player.0));
            } else if nonland_cards.len() == 1 {
                // Only one option — auto-select.
                let exile_id = nonland_cards[0];
                let name = state.get_object(exile_id).map(|o| o.name.clone()).unwrap_or_default();
                state.move_object(exile_id, Zone::Exile, registry);
                state.log(crate::state::LogLevel::Event,
                    format!("Night Terrors: exiled {} from p{}'s hand", name, target_player.0));
            } else {
                // Multiple nonland cards — controller chooses which to exile.
                crate::cards::helpers::present_target_choice(
                    state, object_id, controller, nonland_cards.iter().map(|&id| Target::Object(id)).collect(),
                    crate::state::PendingEffect::CardEffect { source_id: object_id, key: String::new() },
                    "Night Terrors: choose a nonland card to exile",
                    false,
                    registry,
                );
                // Nothing more to do here: the choice is pending, and the
                // engine leaves the spell on the stack until it is answered.
                return;
            }
        }
    }

    /// "...Exile that card." Moving the chosen card is all this does. The
    /// spell's own trip to the graveyard is the engine's —
    /// `engine::finish_spell_resolution_if_idle` runs once the choice chain
    /// empties (CR 608.2m: the graveyard is the last step of resolution). This
    /// comment used to claim the cleanup as the card's business, which is both
    /// untrue and the one thing card code must never do.
    fn resolve_card_effect(&self, state: &mut GameState, _source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Object(id) = target else { return };
        let name = state.obj_name(*id);
        state.move_object(*id, Zone::Exile, registry);
        state.log(crate::state::LogLevel::Event,
            format!("Night Terrors exiled {name} from hand"));
    }
}
