//! What a spell costs to cast (CR 601.2b and 601.2f).
//!
//! A spell's cost used to be adjusted in five unrelated ways — a `ReduceCost`
//! continuous effect, an `AlternativeCost` one, `CardBehavior::modified_cost`,
//! `CardData::flashback_cost`, and `CardData::additional_cost` — each read at
//! whichever call site happened to remember it. Cost reductions reached spells
//! cast from hand and nothing else: not a flashback cost, not an alternative
//! cost. Additional costs were matched inline at six sites, and the flashback
//! path checked one of the three kinds.
//!
//! There is one determination now, in CR 601.2f order.

use crate::cards::{AdditionalCost, CardRegistry};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{CardType, ContinuousEffect, ManaCost, ManaSymbol, SpellFilter, Zone};

/// How a spell is being cast, which decides what its cost starts from.
#[derive(Debug, Clone, PartialEq)]
pub enum CastMethod {
    /// The printed mana cost, as the card's own behavior may modify it.
    Normal,
    /// An alternative cost replaces the mana cost (CR 601.2f): a printed or
    /// granted flashback cost, or Rooftop Storm's `{0}` for Zombies. Cost
    /// reductions still apply on top — this is the base, not the total.
    Alternative(ManaCost),
}

/// The determined cost of casting one spell one way.
#[derive(Debug, Clone)]
pub struct SpellCost {
    /// The mana to pay, after CR 601.2f.
    pub mana: ManaCost,
    /// What must be paid besides mana (CR 601.2b).
    pub additional: Option<AdditionalCost>,
}

/// Determine what casting `card_id` costs `player` right now (CR 601.2f).
///
/// Order: the base cost — an alternative cost if one is replacing it,
/// otherwise the printed cost as the card's own behavior modifies it — and
/// then reductions. There are no cost increases in this card pool; a
/// `ContinuousEffect::IncreaseCost` would be applied between the two.
#[must_use]
pub fn cost_to_cast(
    state: &GameState,
    registry: &CardRegistry,
    card_id: CardId,
    player: PlayerId,
    method: &CastMethod,
) -> SpellCost {
    let data = registry.card_data(card_id);
    let base = match method {
        CastMethod::Alternative(cost) => cost.clone(),
        CastMethod::Normal => registry
            .get(card_id)
            .and_then(|b| b.modified_cost(state, registry))
            .or_else(|| data.as_ref().and_then(|d| d.cost.clone()))
            .unwrap_or_else(|| ManaCost::new(vec![])),
    };
    SpellCost {
        mana: reduce(&base, total_reduction(state, registry, card_id, player)),
        additional: data.and_then(|d| d.additional_cost),
    }
}

/// The mana cost of casting `card_id` normally. The common case of
/// [`cost_to_cast`], for callers that only want the number.
#[must_use]
pub fn effective_spell_cost(
    state: &GameState,
    registry: &CardRegistry,
    card_id: CardId,
    _base_cost: &ManaCost,
    caster: PlayerId,
) -> ManaCost {
    cost_to_cast(state, registry, card_id, caster, &CastMethod::Normal).mana
}

/// Alternative costs available to `caster` for this spell (CR 601.2b) —
/// Rooftop Storm's "you may cast Zombie creature spells without paying their
/// mana cost". Each is a base cost, so it goes through [`cost_to_cast`] as
/// `CastMethod::Alternative` rather than being paid as-is.
#[must_use]
pub fn alternative_costs(
    state: &GameState,
    registry: &CardRegistry,
    card_id: CardId,
    caster: PlayerId,
) -> Vec<ManaCost> {
    state.effects_controlled_by(caster, registry).into_iter()
        .filter_map(|e| match e {
            ContinuousEffect::AlternativeCost { cost, filter }
                if spell_matches(registry, card_id, &filter) => Some(cost),
            _ => None,
        })
        .collect()
}

/// How much generic mana every applicable `ReduceCost` effect takes off.
fn total_reduction(
    state: &GameState,
    registry: &CardRegistry,
    card_id: CardId,
    caster: PlayerId,
) -> u32 {
    state.effects_controlled_by(caster, registry).iter()
        .filter_map(|e| match e {
            ContinuousEffect::ReduceCost { reduction, filter }
                if spell_matches(registry, card_id, filter) => Some(*reduction),
            _ => None,
        })
        .sum()
}

fn spell_matches(registry: &CardRegistry, card_id: CardId, filter: &SpellFilter) -> bool {
    let Some(data) = registry.card_data(card_id) else { return false };
    if !data.card_types.contains(&CardType::Creature) {
        return false;
    }
    match filter {
        SpellFilter::CreatureSpells => true,
        SpellFilter::CreatureWithSubtype(sub) => data.subtypes.iter().any(|s| s == sub),
    }
}

/// CR 601.2f: a reduction comes off the generic portion; coloured
/// requirements are never reduced.
fn reduce(cost: &ManaCost, mut reduction: u32) -> ManaCost {
    if reduction == 0 {
        return cost.clone();
    }
    let mut symbols = Vec::new();
    for sym in &cost.symbols {
        match sym {
            ManaSymbol::Generic(n) if reduction >= *n => reduction -= *n,
            ManaSymbol::Generic(n) => {
                symbols.push(ManaSymbol::Generic(*n - reduction));
                reduction = 0;
            }
            other => symbols.push(other.clone()),
        }
    }
    ManaCost::new(symbols)
}

/// Everything a caller needs to know about a spell's additional cost
/// (CR 601.2b): whether it can be paid at all, what the choices are, and how
/// to describe it.
///
/// The eligibility rules used to be written out at six sites — twice for
/// sacrifice, three times for exile-from-graveyard, once for the label — and
/// the flashback path only ever checked one of the three kinds.
#[derive(Debug, Clone, Default)]
pub struct AdditionalCostPlan {
    pub cost: Option<AdditionalCost>,
    /// False when the cost cannot be paid, so the spell can't be cast at all.
    pub payable: bool,
    /// Creatures that could be sacrificed, for expanding one cast action per
    /// choice. Empty unless the cost is `SacrificeCreature`.
    pub sacrifice_options: Vec<ObjectId>,
    /// Largest X for `ExileXFromGraveyard`, so a UI can label the effect.
    pub exile_x_max: Option<u32>,
    pub label: Option<String>,
}

/// Work out the additional cost for casting `spell` (the object, so it can
/// exclude itself from its own graveyard count).
#[must_use]
pub fn additional_cost_plan(
    state: &GameState,
    registry: &CardRegistry,
    card_id: CardId,
    spell: ObjectId,
    player: PlayerId,
) -> AdditionalCostPlan {
    let cost = registry.card_data(card_id).and_then(|d| d.additional_cost);
    let Some(cost) = cost else {
        return AdditionalCostPlan { payable: true, ..AdditionalCostPlan::default() };
    };
    let graveyard = |only_creatures: bool| {
        state.objects_in_id_order().into_iter()
            .filter(|o| {
                o.zone == Zone::Graveyard && o.owner == player && o.id != spell
                    && (!only_creatures || state.is_creature(o.id, registry))
            })
            .count()
    };
    match &cost {
        AdditionalCost::SacrificeCreature => {
            let options: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, player)
                .iter()
                .filter(|o| state.is_creature(o.id, registry))
                .map(|o| o.id)
                .collect();
            AdditionalCostPlan {
                payable: !options.is_empty(),
                sacrifice_options: options,
                label: Some("sacrifice a creature".into()),
                cost: Some(cost),
                exile_x_max: None,
            }
        }
        AdditionalCost::ExileCreaturesFromGraveyard(n) => AdditionalCostPlan {
            payable: graveyard(true) >= *n,
            label: Some(format!("exile {n} creature{} from GY", if *n == 1 { "" } else { "s" })),
            cost: Some(cost),
            sacrifice_options: vec![],
            exile_x_max: None,
        },
        AdditionalCost::ExileXFromGraveyard => AdditionalCostPlan {
            // X may be zero, so this is always payable.
            payable: true,
            exile_x_max: Some(u32::try_from(graveyard(false)).unwrap_or(u32::MAX)),
            label: Some("exile cards from GY".into()),
            cost: Some(cost),
            sacrifice_options: vec![],
        },
    }
}

/// The choice a spell's additional cost needs from its caster before it can be
/// cast (CR 601.2b), or `None` when nothing is owed or the caster has already
/// chosen.
pub(crate) struct ExilePrompt {
    pub options: Vec<ObjectId>,
    pub min: usize,
    pub max: usize,
    pub description: String,
}

/// Whether casting `spell` needs a "choose what to exile" prompt first.
pub(crate) fn exile_prompt(
    state: &GameState,
    registry: &CardRegistry,
    card_id: CardId,
    spell: ObjectId,
    player: PlayerId,
    exile_count: Option<u32>,
    exile_ids: &[ObjectId],
    spell_name: &str,
) -> Option<ExilePrompt> {
    // A caller that already named the cards (a test, a replay, the resolution
    // handler coming back round) has chosen; don't ask again.
    if !exile_ids.is_empty() {
        return None;
    }
    let gy = |only_creatures: bool| -> Vec<ObjectId> {
        state.objects_in_id_order().into_iter()
            .filter(|o| {
                o.zone == Zone::Graveyard && o.owner == player && o.id != spell
                    && (!only_creatures || state.is_creature(o.id, registry))
            })
            .map(|o| o.id)
            .collect()
    };
    match registry.card_data(card_id).and_then(|d| d.additional_cost)? {
        AdditionalCost::ExileXFromGraveyard => {
            // An explicit count is also a choice.
            if exile_count.is_some() {
                return None;
            }
            let options = gy(false);
            let max = options.len();
            Some(ExilePrompt {
                description: format!(
                    "{spell_name}: choose 0-{max} cards to exile from your graveyard \
                     (each exiled card adds to the spell's X)"),
                options,
                min: 0,
                max,
            })
        }
        AdditionalCost::ExileCreaturesFromGraveyard(n) => Some(ExilePrompt {
            description: format!(
                "{spell_name}: choose exactly {n} creature{} to exile from your graveyard",
                if n == 1 { "" } else { "s" }),
            options: gy(true),
            min: n,
            max: n,
        }),
        AdditionalCost::SacrificeCreature => None,
    }
}

/// Pay a spell's additional cost (CR 601.2b).
///
/// One dispatch on the kind. The cast handler and the exile-choice handler
/// each used to spell all three out, and had drifted apart in how they picked
/// when the player left the choice open.
/// Whether the additional cost the caster proposes to pay is one they CAN pay
/// (CR 601.2h: if you can't pay all costs, the spell is never cast).
///
/// `pay_additional_cost` below used to be the only word on the subject, and it
/// takes what it is given: a cast submitted with nothing to sacrifice went on
/// the stack with the cost unpaid, a named sacrifice was sacrificed without
/// asking whose creature it was, and explicit exile ids were exiled from
/// whichever graveyard they were in. `legal_actions` never offers any of
/// those — but neither client picks a whole offered action, so the submit
/// path has to ask for itself, before anything is paid.
pub(crate) fn additional_cost_is_payable(
    state: &GameState,
    registry: &CardRegistry,
    card_id: CardId,
    spell: ObjectId,
    player: PlayerId,
    sacrifice: Option<ObjectId>,
    exile_ids: &[ObjectId],
) -> bool {
    // A named sacrifice must be the caster's own battlefield creature,
    // whatever the card's cost says: "sacrifice" can only reach what you
    // control (CR 701.17a).
    if let Some(id) = sacrifice {
        let ok = state.get_object(id).is_some_and(|o|
            o.zone == Zone::Battlefield && o.controller == player)
            && state.is_creature(id, registry);
        if !ok {
            return false;
        }
    }
    let Some(cost) = registry.card_data(card_id).and_then(|d| d.additional_cost) else {
        return true;
    };
    // A card named twice is one card (CR 601.2h pays with distinct objects).
    let distinct = |ids: &[ObjectId]| {
        let set: std::collections::HashSet<_> = ids.iter().collect();
        set.len() == ids.len()
    };
    let in_own_graveyard_card = |id: ObjectId| {
        state.get_object(id).is_some_and(|o|
            o.zone == Zone::Graveyard && o.owner == player && o.id != spell)
            && state.is_card(id)
    };
    match cost {
        AdditionalCost::SacrificeCreature => {
            sacrifice.is_some()
                || state.objects_in_zone(Zone::Battlefield, player)
                    .iter().any(|o| state.is_creature(o.id, registry))
        }
        AdditionalCost::ExileCreaturesFromGraveyard(n) => {
            if exile_ids.is_empty() {
                // Unnamed: the auto-picker needs n candidates to take.
                state.objects_in_id_order().iter()
                    .filter(|o| o.zone == Zone::Graveyard && o.owner == player
                        && o.id != spell && state.is_creature(o.id, registry))
                    .count() >= n as usize
            } else {
                exile_ids.len() == n as usize
                    && distinct(exile_ids)
                    && exile_ids.iter().all(|&id|
                        in_own_graveyard_card(id) && state.is_creature(id, registry))
            }
        }
        AdditionalCost::ExileXFromGraveyard => {
            // X may be zero; what is named just has to be really available.
            distinct(exile_ids)
                && exile_ids.iter().all(|&id| in_own_graveyard_card(id))
        }
    }
}

pub(crate) fn pay_additional_cost(
    state: &mut GameState,
    registry: &CardRegistry,
    card_id: CardId,
    spell: ObjectId,
    player: PlayerId,
    sacrifice: Option<ObjectId>,
    exile_count: Option<u32>,
    exile_ids: &[ObjectId],
) {
    // A named sacrifice is paid whether or not the card declares the cost —
    // the caller chose it and legal_actions only offers it where it is owed.
    if let Some(id) = sacrifice {
        let name = super::card_name(state, registry, id);
        crate::destruction::sacrifice(state, id, registry);
        state.log(crate::state::LogLevel::Event,
            format!("Sacrificed {name} as additional cost"));
    }
    let Some(cost) = registry.card_data(card_id).and_then(|d| d.additional_cost) else { return };
    match cost {
        AdditionalCost::SacrificeCreature => {
            if sacrifice.is_some() {
                return; // already paid above
            }
            // Nothing chosen: take the first creature, as before.
            let creature = state.objects_in_zone(Zone::Battlefield, player)
                .iter()
                .find(|o| state.is_creature(o.id, registry))
                .map(|o| o.id);
            if let Some(id) = creature {
                let name = super::card_name(state, registry, id);
                crate::destruction::sacrifice(state, id, registry);
                state.log(crate::state::LogLevel::Event,
                    format!("Sacrificed {name} as additional cost"));
            }
        }
        AdditionalCost::ExileCreaturesFromGraveyard(n) => {
            pay_exile_creatures(state, registry, spell, player, n, exile_ids);
        }
        AdditionalCost::ExileXFromGraveyard => {
            let chosen: Vec<ObjectId> = if exile_ids.is_empty() {
                let x = exile_count.unwrap_or(0) as usize;
                state.objects_in_id_order().into_iter()
                    .filter(|o| o.zone == Zone::Graveyard && o.owner == player && o.id != spell)
                    .map(|o| o.id)
                    .take(x)
                    .collect()
            } else {
                exile_ids.to_vec()
            };
            let count = u32::try_from(chosen.len()).unwrap_or(u32::MAX);
            for id in &chosen {
                state.move_object(*id, Zone::Exile, registry);
            }
            // X is read back at resolution (Harvest Pyre's damage).
            if let Some(obj) = state.get_object_mut(spell) {
                obj.card_state.insert(crate::cards::EXILE_COUNT.into(), ObjectId(u64::from(count)));
            }
            state.log(crate::state::LogLevel::Event,
                format!("Exiled {count} cards from graveyard as additional cost"));
        }
    }
}

/// Pay an `ExileCreaturesFromGraveyard` cost.
///
/// `chosen` is what the player picked; when it is empty the engine picks the
/// highest-power creature cards available. Two copies of this existed — one in
/// the cast handler, one in the choice handler — and they disagreed: the cast
/// handler ranked candidates by `obj.power`, which is `None` for every
/// non-token card, so it was really picking arbitrarily. That is visible on
/// Corpse Lunge, whose damage is the exiled creature's power.
///
/// Records the exiled cards on the spell as
/// [`crate::cards::EXILED_TO_COST`], for a card that wants to ask them
/// something when it resolves.
pub fn pay_exile_creatures(
    state: &mut GameState,
    registry: &CardRegistry,
    spell: ObjectId,
    player: PlayerId,
    count: usize,
    chosen: &[ObjectId],
) {
    let to_exile: Vec<ObjectId> = if chosen.is_empty() {
        let mut candidates: Vec<(ObjectId, i32)> = state.objects_in_id_order().into_iter()
            .filter(|o| {
                o.zone == Zone::Graveyard && o.owner == player && o.id != spell
                    && state.is_creature(o.id, registry)
            })
            // CR 208.2: a characteristic-defining power works in every zone,
            // so Boneyard Wurm in a graveyard is not a 0/0.
            .map(|o| (o.id, state.effective_power(o.id, registry).unwrap_or(0)))
            .collect();
        candidates.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.0.cmp(&b.0.0)));
        candidates.into_iter().take(count).map(|(id, _)| id).collect()
    } else {
        chosen.to_vec()
    };

    for (i, &id) in to_exile.iter().enumerate() {
        if let Some(obj) = state.get_object_mut(spell) {
            obj.card_state.insert(crate::cards::exiled_to_cost_key(i), id);
        }
    }
    for id in &to_exile {
        let name = super::card_name(state, registry, *id);
        state.move_object(*id, Zone::Exile, registry);
        state.log(crate::state::LogLevel::Event,
            format!("Exiled {name} from graveyard as additional cost"));
    }
}
