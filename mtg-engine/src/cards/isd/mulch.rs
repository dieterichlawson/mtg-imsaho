use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// The names of the objects `.1` that are still in the game, joined by ", ".
struct Names<'a>(&'a GameState, &'a [ObjectId]);

impl std::fmt::Display for Names<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for o in self.1.iter().filter_map(|id| self.0.get_object(*id)) {
            if !first {
                f.write_str(", ")?;
            }
            f.write_str(&o.name)?;
            first = false;
        }
        Ok(())
    }
}

/// Mulch — {1}{G} Sorcery.
/// Reveal the top four cards of your library. Put all land cards revealed this way
/// into your hand and the rest into your graveyard.
pub struct Mulch;

impl CardBehavior for Mulch {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Mulch".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard.".into(),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);

        // Reveal the top four cards. Every one of them is going somewhere, so
        // the ids are all this needs — `move_object` takes each out of the
        // library's order as it leaves (CR 401.1). Draining them out here
        // first left four cards whose zone said Library while the library did
        // not list them, which is a state nothing should be able to observe
        // and nothing here needs.
        let library = &state.get_player(controller).library_order;
        let revealed: Vec<ObjectId> = library.iter().take(4).copied().collect();

        // Each list allocated at the most it can hold: a list that is empty,
        // and so unallocated, until its first push is one whose allocation
        // is a question about the board, which a proof of the card has to
        // take both ways.
        let mut lands = Vec::with_capacity(revealed.len());
        let mut non_lands = Vec::with_capacity(revealed.len());

        for &card_id in &revealed {
            let is_land = state.has_card_type(card_id, CardType::Land, registry);

            if is_land {
                lands.push(card_id);
            } else {
                non_lands.push(card_id);
            }
        }

        // Log the reveal: the names written as the line is, not copied out
        // and joined first, which builds strings of lengths the board
        // decides even where nothing reads the line.
        state.log(crate::state::LogLevel::Event,
            format!("Mulch revealed: {}", Names(state, &revealed)));

        // Lands go to hand.
        for &land_id in &lands {
            let name = state.get_object(land_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(land_id, Zone::Hand, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Mulch: {name} put into hand"));
        }

        // Non-lands go to graveyard. Routed through `mill_one` so a creature
        // card among them emits CreatureCardMilled — moving it directly meant
        // Undead Alchemist never saw it.
        for &non_land_id in &non_lands {
            crate::engine::mill_one(state, non_land_id, registry);
        }

    }
}
