use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Moonmist — {1}{G} Instant.
/// Transform all Humans. Prevent all combat damage that would be dealt this turn
/// by creatures other than Werewolves and Wolves.
pub struct Moonmist;

impl CardBehavior for Moonmist {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Moonmist".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves. (Only double-faced cards can be transformed.)".into(),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        // "Transform all Humans." — any creature currently with the Human subtype
        // should transform, regardless of which face is showing. This includes:
        // - Front-face Humans (transform to back face)
        // - Back-face creatures that are still Human (transform back to front face,
        //   e.g., Thraben Militia is Human on its back face)
        // Tokens are excluded here as well as inside `apply_transform`: a
        // token copy of a double-faced card is not itself double-faced and
        // cannot transform (CR 111.7), so it must not be counted either.
        // Every Human on the battlefield is asked to transform. Which of them
        // *can* is not this card's question: "(Only double-faced cards can be
        // transformed.)" is CR 701.28c, and `apply_transform` refuses a
        // single-faced permanent and a token copy of a double-faced one
        // (CR 111.7) without being asked to.
        let humans: Vec<ObjectId> = state.all_objects_in_zone(Zone::Battlefield).into_iter()
            .filter(|o| state.has_subtype(o.id, "Human", registry))
            .map(|o| o.id)
            .collect();

        // Through `apply_transform`, the one place that knows what transforming
        // means. This loop used to flip `is_transformed` by hand and then copy
        // the face's name, power, toughness, keywords and subtypes onto the
        // object — which broke the characteristics model three ways. `obj.power`
        // and `obj.subtypes` hold runtime *grants*, not printed values, so
        // `clone_from`ing a face over them threw away everything granted to that
        // permanent (Olivia Voldaren's "Vampire", Grimoire of the Dead's types,
        // any until-end-of-turn keyword) and pinned its P/T against later
        // effects. It also skipped the CR 111.7 refusal for token copies.
        // Counted after the fact, because asking is not transforming: a
        // single-faced Human is asked and refused.
        let mut count = 0;
        for hid in humans {
            let before = state.get_object(hid).map(|o| o.is_transformed);
            crate::cards::helpers::apply_transform(state, hid, registry);
            if before != state.get_object(hid).map(|o| o.is_transformed) {
                count += 1;
            }
        }

        if count > 0 {
            state.log(crate::state::LogLevel::Event,
                format!("Moonmist transformed {count} Human(s)"));
        }
        // "Prevent all combat damage that would be dealt this turn by creatures
        // other than Werewolves and Wolves." Moonmist names the exceptions; the
        // engine just applies the filter.
        state.until_end_of_turn.push(crate::state::TemporaryEffect::PreventCombatDamageExcept {
            filter: crate::types::CreatureFilter::Or(vec![
                crate::types::CreatureFilter::HasSubtype("Wolf".into()),
                crate::types::CreatureFilter::HasSubtype("Werewolf".into()),
            ]),
        });
        state.log(crate::state::LogLevel::Event,
            "Moonmist: preventing combat damage from non-Wolf/non-Werewolf creatures this turn".into());

    }
}
