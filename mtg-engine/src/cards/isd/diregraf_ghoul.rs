use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Diregraf Ghoul — 2/2 for {B}. Enters the battlefield tapped.
/// Note: "enters tapped" is a static/replacement ability, NOT a triggered ability.
pub struct DiregrafGhoul;

impl CardBehavior for DiregrafGhoul {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Diregraf Ghoul".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Zombie".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "This creature enters tapped.".into(),
            ..Default::default()
        }
    }

    /// CR 614.1c: "enters tapped" is a replacement effect — the permanent never
    /// enters untapped and is then turned. Moving it to the battlefield and
    /// setting `tapped` afterwards, as this used to, fires `EnteredBattlefield`
    /// with an untapped Ghoul for every watcher to see, and does the engine's
    /// resolution work besides. The same helper the ISD dual lands use, with no
    /// condition — this one is always tapped.
    fn replace_event(
        &self,
        _state: &mut GameState,
        self_id: ObjectId,
        event: &crate::replacement::ReplaceableEvent,
        _registry: &CardRegistry,
    ) -> Option<crate::replacement::Replacement> {
        crate::cards::helpers::enters_tapped_unless(self_id, event, || false)
    }
}
