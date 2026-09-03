//! Per-object invariants: what a `GameObject`'s fields may say about it given
//! the zone it is in and the card it is printed as. All card-independent and
//! all atomic with respect to decision points, so they hold at the core tier.

use super::{player_ok, Violations};
use crate::cards::CardRegistry;
use crate::ids::CardId;
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind, StackEntry};
use crate::triggers::TriggerEvent;
use crate::types::{CardType, CounterType, Supertype, Zone};

pub(super) fn check_core(state: &GameState, registry: &CardRegistry, v: &mut Violations) {
    contract(state, registry, v);
    let n_players = state.players.len();
    for obj in state.objects_in_id_order() {
        let id = obj.id;
        let tag = format!("{} ({})", id.0, obj.name);
        let on_bf = obj.zone == Zone::Battlefield;
        let printed = obj.copy_grantor.unwrap_or(obj.card_id);

        // CR 102.1: every player id stored on an object names a real player.
        for (what, p) in [
            ("owner", Some(obj.owner)),
            ("controller", Some(obj.controller)),
            ("last_controller", obj.last_controller),
            ("attached_to_player", obj.attached_to_player),
            ("last_attached_to_player", obj.last_attached_to_player),
        ] {
            if let Some(p) = p {
                if !player_ok(state, p) {
                    v.push(format!("{tag}: {what} p{} is not a player (of {n_players})", p.0));
                }
            }
        }

        // CR 108.4/109.4: only a permanent or a spell has a controller of its
        // own; elsewhere the owner stands in. (Stack included: nothing in
        // this pool casts another player's card.)
        if !on_bf && obj.controller != obj.owner {
            v.push(format!("{tag} in {:?} is controlled by p{} but owned by p{} (CR 108.4)",
                obj.zone, obj.controller.0, obj.owner.0));
        }

        // CR 200.1/205.2c: a card has a face in the registry; a token copy
        // points at the card it copies; everything has at least one type.
        // (A card that is a copy of a token has no face either: Evil Twin
        // copying a Wolf token carries the token's characteristics on the
        // object, with `copy_grantor` remembering the printed card.)
        if !obj.is_token && registry.get(obj.card_id).is_none()
            && !(obj.card_id == CardId(0) && obj.copy_grantor.is_some())
        {
            v.push(format!("{tag}: card {} is not in the registry", obj.card_id.0));
        }
        if obj.is_token && obj.card_id != CardId(0) && registry.get(obj.card_id).is_none() {
            v.push(format!("{tag}: token copy of unregistered card {}", obj.card_id.0));
        }
        let types = state.card_types_of(id, registry);
        if types.is_empty() {
            v.push(format!("{tag} has no card type"));
        }

        // CR 110.4/304.4/307.4: only permanents on the battlefield.
        if on_bf {
            if types.contains(&CardType::Instant) || types.contains(&CardType::Sorcery) {
                v.push(format!("{tag} is an instant/sorcery on the battlefield (CR 304.4/307.4)"));
            }
            if !types.iter().any(CardType::is_permanent) {
                v.push(format!("{tag} on the battlefield has no permanent type (CR 110.4)"));
            }
        }
        // CR 305.9: a land is never a spell.
        if obj.zone == Zone::Stack && types.contains(&CardType::Land) {
            v.push(format!("{tag} is a land on the stack (CR 305.9)"));
        }

        // CR 107.3a/107.3g: X means something on the stack and on the
        // permanent the cast produced, only for a card whose cost has X.
        if obj.x_value.is_some() {
            if obj.is_token || !matches!(obj.zone, Zone::Stack | Zone::Battlefield) {
                v.push(format!("{tag} in {:?} carries x_value {:?} (CR 107.3g)", obj.zone, obj.x_value));
            } else if !registry.card_data(printed).and_then(|d| d.cost).is_some_and(|c| c.has_x()) {
                v.push(format!("{tag} carries x_value but its cost has no X"));
            }
        }

        // CR 712.8a/712.9: a back face is up only on the battlefield, and only
        // a card that has one can show it.
        if obj.is_transformed {
            if !on_bf {
                v.push(format!("{tag} is transformed in {:?} (CR 712.8a)", obj.zone));
            }
            if registry.get(obj.card_id).and_then(|b| b.back_face_data()).is_none() {
                v.push(format!("{tag} is transformed but card {} has no back face (CR 712.9)", obj.card_id.0));
            }
        }

        // CR 707.2/400.7: copy identity is battlefield-only and names real cards.
        if let Some(grantor) = obj.copy_grantor {
            if !on_bf {
                v.push(format!("{tag} in {:?} is still a copy (CR 400.7)", obj.zone));
            }
            if !obj.is_token && registry.get(grantor).is_none() {
                v.push(format!("{tag}: copy_grantor {} is not in the registry", grantor.0));
            }
        }

        // CR 702.34a: a flashback-cast spell is on the stack or exiled, never
        // anywhere else, and only an instant or sorcery card can be one.
        if obj.cast_with_flashback {
            if !matches!(obj.zone, Zone::Stack | Zone::Exile) {
                v.push(format!("{tag} cast with flashback is in {:?} (CR 702.34a)", obj.zone));
            }
            if obj.is_token {
                v.push(format!("{tag} is a token cast with flashback"));
            }
            if !types.contains(&CardType::Instant) && !types.contains(&CardType::Sorcery) {
                v.push(format!("{tag} cast with flashback is neither instant nor sorcery"));
            }
        }

        // CR 400.7: a non-token card off the battlefield is its printed self —
        // no runtime characteristics, no copy identity, printed P/T.
        if !obj.is_token && !on_bf {
            if !obj.subtypes.is_empty() || !obj.colors.is_empty()
                || !obj.card_types.is_empty() || !obj.keywords.is_empty()
            {
                v.push(format!("{tag} in {:?} keeps runtime characteristics {:?}/{:?}/{:?}/{:?} (CR 400.7)",
                    obj.zone, obj.card_types, obj.subtypes, obj.colors, obj.keywords));
            }
            if obj.instance_continuous_effects.is_some() || obj.instance_oracle_text.is_some() {
                v.push(format!("{tag} in {:?} keeps instance effects/text (CR 400.7)", obj.zone));
            }
            if let Some(d) = registry.card_data(obj.card_id) {
                if (obj.power, obj.toughness) != (d.power, d.toughness) {
                    v.push(format!("{tag} in {:?} has P/T {:?}/{:?}, printed {:?}/{:?} (CR 400.7)",
                        obj.zone, obj.power, obj.toughness, d.power, d.toughness));
                }
            }
        }

        // CR 205.4b: the legendary cache never claims more than the face.
        if !obj.is_token && obj.is_legendary
            && !registry.card_data(obj.card_id).is_some_and(|d| d.supertypes.contains(&Supertype::Legendary))
        {
            v.push(format!("{tag} is flagged legendary but its face is not"));
        }

        // Battlefield-only status is gone off the battlefield (CR 400.7).
        if !on_bf {
            if obj.summoning_sick {
                v.push(format!("{tag} in {:?} is summoning sick", obj.zone));
            }
            if obj.attacked_on_turn.is_some() {
                v.push(format!("{tag} in {:?} remembers attacking", obj.zone));
            }
            if !obj.damaged_by.is_empty() || obj.dealt_deathtouch_damage {
                v.push(format!("{tag} in {:?} keeps a damage record", obj.zone));
            }
            if !obj.abilities_activated_this_turn.is_empty() {
                v.push(format!("{tag} in {:?} remembers activations this turn (CR 400.7)", obj.zone));
            }
        }
        // CR 606.3: the loyalty sentinel is only ever on a planeswalker.
        if obj.abilities_activated_this_turn.contains(&999)
            && !state.has_card_type(id, CardType::Planeswalker, registry)
        {
            v.push(format!("{tag} used a loyalty ability but is no planeswalker"));
        }

        // CR 120.3/302.7: damage is marked on creatures; a planeswalker takes
        // it as loyalty; the deathtouch flag and the damage record live with
        // marked damage (CR 704.5h's "since the last SBA check").
        if obj.damage_marked > 0 {
            if !on_bf || !state.is_creature(id, registry) {
                v.push(format!("{tag} in {:?} has {} damage marked but is no battlefield creature (CR 120.3)",
                    obj.zone, obj.damage_marked));
            }
            if obj.damaged_by.is_empty() {
                v.push(format!("{tag} has {} damage marked but no record of what dealt it", obj.damage_marked));
            }
        }
        if obj.dealt_deathtouch_damage && obj.damage_marked == 0 {
            v.push(format!("{tag} was dealt deathtouch damage but has none marked"));
        }
        if state.has_card_type(id, CardType::Planeswalker, registry) && obj.damage_marked > 0 {
            v.push(format!("{tag} is a planeswalker with damage marked (CR 120.3c)"));
        }

        // CR 701.3a/303.4: attachment is a battlefield fact; a player is only
        // ever enchanted by an Aura whose enchant ability names players.
        if (obj.attached_to.is_some() || obj.attached_to_player.is_some()) && !on_bf {
            v.push(format!("{tag} in {:?} is attached to something", obj.zone));
        }
        if obj.attached_to_player.is_some() {
            if !state.has_subtype(id, "Aura", registry) {
                v.push(format!("{tag} is attached to a player but is no Aura (CR 303.4)"));
            } else if let Some(b) = registry.get(obj.card_id) {
                use crate::cards::TargetRequirement as R;
                if !matches!(b.target_requirement(), R::PlayerOnly | R::OpponentOnly) {
                    v.push(format!("{tag} enchants a player but its enchant ability does not allow one"));
                }
            }
        }
        if on_bf && obj.last_attached_to_player.is_some() {
            v.push(format!("{tag} on the battlefield keeps a last-attached-to-player shadow"));
        }

        // CR 205.3: a subtype belongs to its card type.
        for (sub, ty) in [("Aura", CardType::Enchantment), ("Curse", CardType::Enchantment),
                          ("Equipment", CardType::Artifact)] {
            if state.has_subtype(id, sub, registry) && !state.has_card_type(id, ty, registry) {
                v.push(format!("{tag} has subtype {sub} without type {ty:?} (CR 205.3)"));
            }
        }
        if obj.is_token && !obj.subtypes.is_empty() && !obj.card_types.contains(&CardType::Creature) {
            v.push(format!("{tag} is a token with subtypes {:?} but no creature type", obj.subtypes));
        }

        // CR 111.4: an unnamed token is named after its subtypes.
        if obj.is_token && obj.card_id == CardId(0) {
            match obj.name.strip_suffix(" Token") {
                Some(words) => {
                    if !words.split_whitespace().all(|w| obj.subtypes.iter().any(|s| s == w)) {
                        v.push(format!("{tag}: token name is not its subtypes {:?} plus \"Token\" (CR 111.4)", obj.subtypes));
                    }
                }
                None => v.push(format!("{tag}: token name does not end in \"Token\" (CR 111.4)")),
            }
        }

        // The name cache agrees with the face that is up. A card's name comes
        // from the decklist, which may use the "Front // Back" form.
        let face_name = state.name_of(id, registry);
        if obj.is_token || obj.copy_grantor.is_some() {
            if obj.name != face_name {
                v.push(format!("{tag}: name cache says {:?} but the face is {:?} (CR 707.8)", obj.name, face_name));
            }
        } else if registry.get(obj.card_id).is_some()
            && obj.name != face_name
            && !obj.name.starts_with(&format!("{face_name} // "))
        {
            v.push(format!("{tag}: name cache says {:?} but the face is {:?}", obj.name, face_name));
        }

        // CR 700.2: a modal spell on the stack has exactly one mode chosen.
        if let Some(b) = registry.get(obj.card_id) {
            if obj.zone == Zone::Stack {
                match (b.target_requirement(), obj.chosen_mode) {
                    (crate::cards::TargetRequirement::ModalChoice(modes), Some(i)) => {
                        if i >= modes.len() || modes.len() < 2 {
                            v.push(format!("{tag}: chosen mode {i} of {} modes (CR 700.2)", modes.len()));
                        }
                    }
                    (crate::cards::TargetRequirement::ModalChoice(_), None) => {
                        v.push(format!("{tag} is a modal spell on the stack with no mode chosen (CR 700.2)"));
                    }
                    (_, Some(_)) => {
                        v.push(format!("{tag} has a chosen mode but is not modal"));
                    }
                    _ => {}
                }
            }
        }

        // CR 701.19: shields are a battlefield thing (promoted from settled:
        // the only writer refuses off the battlefield).
        if obj.regeneration_shields > 0 && !on_bf {
            v.push(format!("{tag} in {:?} keeps a regeneration shield", obj.zone));
        }

        // CR 208.1: power and toughness come as a pair.
        if obj.power.is_some() != obj.toughness.is_some() {
            v.push(format!("{tag} has power {:?} but toughness {:?} (CR 208.1)", obj.power, obj.toughness));
        }
        // CR 111: a token is created on the battlefield and never comes back
        // to it, so a battlefield token has never changed zones.
        if obj.is_token && on_bf && obj.zone_change_count != 0 {
            v.push(format!("{tag} is a token that changed zones {} time(s) and is on the battlefield (CR 111.8)", obj.zone_change_count));
        }

        // Loyalty counters on a non-planeswalker anywhere.
        if obj.counters.get(&CounterType::Loyalty).copied().unwrap_or(0) > 0
            && !state.has_card_type(id, CardType::Planeswalker, registry)
        {
            v.push(format!("{tag} holds loyalty counters but is no planeswalker"));
        }
    }

    // The unused designation stays unused: nothing writes it.
    if state.day_night.is_some() {
        v.push("day/night designation set but nothing in this pool uses it".into());
    }
}

/// Whether an enters-as-copy decision for `id` is still in flight: its
/// enters trigger is queued or on the stack, or its copy prompt is up.
fn copy_choice_live(state: &GameState, id: ObjectId) -> bool {
    let trigger_waiting = state.stack.iter()
        .filter_map(|e| match e { StackEntry::Trigger(t) => Some(t), _ => None })
        .chain(state.pending_triggers.iter())
        .chain(state.pending_trigger_pushes_ap.iter())
        .chain(state.pending_trigger_pushes_nap.iter())
        .any(|t| t.source.id == id && matches!(t.event, TriggerEvent::SelfEntered));
    let prompt_up = matches!(&state.awaiting_action,
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTarget { effect: PendingEffect::CopyCreature { source_id }, .. }, ..
        }) if *source_id == id);
    trigger_waiting || prompt_up
}

/// What every card hook leaves behind on an object — the contract card code
/// is held to whichever card wrote the field.
fn contract(state: &GameState, registry: &CardRegistry, v: &mut Violations) {
    for obj in state.objects_in_id_order() {
        let id = obj.id;
        let tag = format!("#{} ({})", id.0, obj.name);
        let on_bf = obj.zone == Zone::Battlefield;

        // CR 614.1d: the state-based-action copy-guard is armed only while
        // the enters-as-copy choice is live; afterwards the permanent is an
        // ordinary one again.
        if obj.entering_copy_source && !copy_choice_live(state, id) {
            v.push(format!("{tag} is exempt from state-based actions with no enters-as-copy choice in flight (CR 614.1d)"));
        }

        if !obj.is_token {
            // CR 707.2/613: the object-level vectors are written only by copy
            // effects, from the copied card's active face, so they never say
            // more than that face does (grants go through effects instead).
            if let Some(face) = state.face_data(id, registry) {
                for k in &obj.keywords {
                    if !face.keywords.contains(k) {
                        v.push(format!("{tag} carries {k:?} which its face does not print (CR 707.2)"));
                    }
                }
                for t in &obj.card_types {
                    if !face.card_types.contains(t) {
                        v.push(format!("{tag} carries type {t:?} which its face does not print (CR 707.2)"));
                    }
                }
            }
            // CR 208.1: a card has a P/T box exactly when it is printed with
            // one (every face of every double-faced card in this pool agrees).
            if let Some(d) = registry.card_data(obj.card_id) {
                if obj.power.is_some() != d.power.is_some() {
                    v.push(format!("{tag} has power {:?} but the card prints {:?} (CR 208.1)", obj.power, d.power));
                }
            }
        }

        // CR 701.15: a regeneration shield is on a creature.
        if obj.regeneration_shields > 0 && !state.is_creature(id, registry) {
            v.push(format!("{tag} has {} regeneration shield(s) but is no creature (CR 701.15)", obj.regeneration_shields));
        }

        // CR 122.1: counters are on permanents, and +1/+1 counters on creatures.
        for (kind, n) in &obj.counters {
            if *n == 0 {
                continue;
            }
            if !on_bf {
                v.push(format!("{tag} has {n} {kind:?} counter(s) in {:?} (CR 122.1)", obj.zone));
            } else if *kind == CounterType::PlusOnePlusOne && !state.is_creature(id, registry) {
                v.push(format!("{tag} has {n} +1/+1 counter(s) but is no creature"));
            }
        }

        // CR 606.3: a loyalty ability is activated by its controller on
        // their own turn, so the sentinel sits on the active player's walker.
        if on_bf && obj.abilities_activated_this_turn.contains(&999) && obj.controller != state.active_player {
            v.push(format!("{tag} used a loyalty ability this turn but p{} is not the active player (CR 606.3)", obj.controller.0));
        }

        // CR 301.5/303.4: an attachment names something that exists.
        if let Some(h) = obj.attached_to {
            if state.get_object(h).is_none() {
                v.push(format!("{tag} is attached to #{} which does not exist", h.0));
            }
        }
    }
}
