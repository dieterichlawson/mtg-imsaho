//! Shared helper functions for common card resolution patterns.
//!
//! Includes:
//! - Spell resolution helpers (`resolve_aura`, `resolve_damage`, `resolve_destroy`)
//! - Choice presentation helpers (`present_target_choice`, `present_yes_no`, etc.)
//! - Target collection helpers (`any_targets`, `creature_targets`, etc.)
//! - Trigger-time condition helpers (`werewolf_should_trigger`, `morbid_should_trigger`)
//! - Library search (`search_library`, `finish_library_search`)

use crate::actions::Target;
use crate::cards::{CardBehavior, CardRegistry, TriggerKind};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::Zone;

/// Resolve an aura spell: attach it to the target creature on the battlefield.
/// If the target is no longer on the battlefield, the aura goes to graveyard.
/// Returns true if the aura was successfully attached.
pub fn resolve_aura(state: &mut GameState, aura_id: ObjectId, targets: &[Target], registry: &CardRegistry) -> bool {
    if let Some(Target::Object(target_id)) = targets.first() {
        if state.get_object(*target_id).is_some_and(|o| o.zone == Zone::Battlefield) {
            state.move_object(aura_id, Zone::Battlefield, registry);
            if let Some(obj) = state.get_object_mut(aura_id) {
                obj.attached_to = Some(*target_id);
                obj.summoning_sick = false;
            }
            return true;
        }
    }
    false
}

/// Resolve an equip ability: attach the Equipment to the target creature.
///
/// CR 702.6b: "Equip [cost] means '[cost]: Attach this permanent to target
/// creature you control. Activate only as a sorcery.'" That is the whole of
/// it, and it was written out eleven times — once per Equipment in the set,
/// four identical lines each, with no shared place for the two rules that are
/// not in those four lines to live.
///
/// The first is target legality on resolution (CR 608.2b): the engine's
/// re-check runs `is_target_legal` and the card's own `is_valid_target`, and
/// for `CreatureWithFilter` the former only re-runs the *filter* — it accepts
/// a target in the Stack zone and asks nothing about creature-ness. So the
/// check is re-run here, where the attachment actually happens, rather than
/// trusted to have happened.
///
/// The second is CR 301.5c: an Equipment that is also a creature does not
/// become attached. Nothing in this set animates an Equipment, so this has
/// never mattered — but it is one line, and the alternative is eleven cards
/// that would each need it if anything ever did.
pub fn resolve_equip(
    state: &mut GameState,
    equipment_id: ObjectId,
    targets: &[Target],
    registry: &CardRegistry,
) -> bool {
    let Some(Target::Object(creature_id)) = targets.first() else { return false };
    let controller = controller_of(state, equipment_id);
    if !equip_target_is_legal(state, controller, &Target::Object(*creature_id), registry) {
        return false;
    }
    // CR 301.5c: an Equipment that's also a creature can't equip a creature.
    if state.is_creature(equipment_id, registry) {
        return false;
    }
    if let Some(obj) = state.get_object_mut(equipment_id) {
        obj.attached_to = Some(*creature_id);
    }
    true
}

/// Whether `target` is something an equip ability could attach to: a creature
/// on the battlefield that the equipping player controls (CR 702.6b).
///
/// Ten Equipment cards carried this as a byte-identical `is_valid_target`.
#[must_use]
pub fn equip_target_is_legal(
    state: &GameState,
    caster: PlayerId,
    target: &Target,
    registry: &CardRegistry,
) -> bool {
    match target {
        Target::Object(id) => state.get_object(*id).is_some_and(|o| {
            o.zone == Zone::Battlefield && state.is_creature(o.id, registry) && o.controller == caster
        }),
        // Equip attaches to a creature, never a player, and CR 608.2b skips a
        // target that stopped being legal.
        Target::Player(_) | Target::Illegal => false,
    }
}

/// What an equip ability costs. CR 702.6a lets equip take any cost; in this set
/// that is mana everywhere except Demonmail Hauberk, whose cost is a sacrifice.
pub enum EquipCost {
    Mana(crate::types::ManaCost),
    SacrificeACreature,
}

/// Equip's index among a card's activated abilities. Every Equipment declares
/// it first; Blazing Torch is the only one with a second ability, and that one
/// is granted to the equipped creature rather than being the Equipment's own.
pub const EQUIP_ABILITY_INDEX: usize = 0;

/// The equip keyword ability (CR 702.6a-b): "[cost]: Attach this permanent to
/// target creature you control. Activate only as a sorcery."
///
/// Eleven Equipment cards each spelled this `ActivatedAbilityDef` out in full.
/// The costs genuinely differ; nothing else did, which meant eleven cards had
/// to independently agree on `sorcery_speed_only`, `once_per_turn`, the target
/// requirement, and the gate below — and any future rule about equip would
/// have eleven places to be added to. `resolve_equip` and
/// `equip_target_is_legal` above were already shared; this is the third and
/// last piece.
///
/// The gate is CR 301.5c — an Equipment that's also a creature can't equip a
/// creature — and it is load-bearing for a second, engine-shaped reason:
/// `legal_actions` asks each attached object for its abilities using the
/// *equipped creature's* id, so without it every equip ability would be
/// offered a second time, keyed to the creature that is wearing it.
#[must_use]
pub fn equip_ability(
    state: &GameState,
    object_id: ObjectId,
    registry: &CardRegistry,
    cost: EquipCost,
) -> Vec<crate::cards::ActivatedAbilityDef> {
    use crate::cards::{ActivatedAbilityDef, SacrificeCost, TargetFilter, TargetRequirement};

    let on_battlefield = state.get_object(object_id)
        .is_some_and(|o| o.zone == Zone::Battlefield);
    if !on_battlefield || state.is_creature(object_id, registry) {
        return vec![];
    }

    // The printed reminder text is the cost, so it is rendered from the cost
    // rather than repeated as a string a card could get out of step with.
    let (description, mana, sacrifice_cost) = match cost {
        EquipCost::Mana(c) => (format!("Equip {c}"), c, SacrificeCost::None),
        EquipCost::SacrificeACreature => (
            "Equip—Sacrifice a creature".to_string(),
            crate::types::ManaCost::free(),
            SacrificeCost::SacrificeCreature,
        ),
    };

    vec![ActivatedAbilityDef {
        ability_index: EQUIP_ABILITY_INDEX,
        description,
        cost: mana,
        requires_tap: false,
        sacrifice_cost,
        target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)),
        once_per_turn: false,
        sorcery_speed_only: true,
        counter_cost: None,
    }]
}

/// `equip_ability` for the common case: a mana cost of `n` generic.
#[must_use]
pub fn equip_for_generic(
    state: &GameState,
    object_id: ObjectId,
    registry: &CardRegistry,
    n: u32,
) -> Vec<crate::cards::ActivatedAbilityDef> {
    equip_ability(state, object_id, registry,
        EquipCost::Mana(crate::types::ManaCost::new(vec![crate::types::ManaSymbol::Generic(n)])))
}

// ═══════════════════════════════════════════════════════════════════
// Countering (CR 701.5)
// ═══════════════════════════════════════════════════════════════════

/// Whether `target` is still a spell on the stack — the whole of "target
/// spell" legality, and what CR 608.2b re-checks on the way down.
#[must_use]
pub fn spell_target_is_legal(state: &GameState, target: &Target) -> bool {
    match target {
        Target::Object(id) => state.get_object(*id).is_some_and(|o| o.zone == Zone::Stack),
        // A spell is never a player, and CR 608.2b skips a target that stopped
        // being legal.
        Target::Player(_) | Target::Illegal => false,
    }
}

/// CR 701.5a: to counter a spell is to remove it from the stack and put it
/// into its owner's graveyard — or into exile if it was cast with flashback
/// (CR 702.33a), which `move_countered_spell` applies.
///
/// Returns whether anything was countered. `false` means the target is no
/// longer on the stack, which CR 608.2b makes an ordinary outcome and not an
/// error: something else countered it first, or it already resolved.
///
/// Four places did this by hand — Counterspell, Dissipate, Lost in the Mist,
/// and the choice handler that finishes Frightful Delusion's "unless its
/// controller pays {1}". Removing the stack entry and disposing of the card
/// are two steps that must not come apart, and one of the four had already
/// drifted: Dissipate removed the entry and then called `move_object(Exile)`
/// itself. Exile is the right destination for *that* card, so it was not a
/// bug — but it reached it without going through the pipeline, which is how
/// the next card to want a destination gets it wrong.
pub fn counter_spell(state: &mut GameState, spell_id: ObjectId, registry: &CardRegistry) -> bool {
    counter_spell_inner(state, spell_id, false, registry)
}

/// `counter_spell`, for a card that replaces where the countered spell goes:
/// "exile it instead of putting it into its owner's graveyard" (Dissipate).
/// CR 614.1a — a replacement effect over CR 701.5a's default.
pub fn counter_spell_exiling(state: &mut GameState, spell_id: ObjectId, registry: &CardRegistry) -> bool {
    counter_spell_inner(state, spell_id, true, registry)
}

fn counter_spell_inner(
    state: &mut GameState,
    spell_id: ObjectId,
    exile: bool,
    registry: &CardRegistry,
) -> bool {
    if !state.get_object(spell_id).is_some_and(|o| o.zone == Zone::Stack) {
        return false;
    }
    let name = state.obj_name(spell_id);
    state.stack.retain(|e| e.as_spell() != Some(spell_id));
    if exile {
        state.move_object(spell_id, Zone::Exile, registry);
        state.log(crate::state::LogLevel::Event,
            format!("{name} was countered and exiled"));
    } else {
        state.move_countered_spell(spell_id, registry);
        state.log(crate::state::LogLevel::Event, format!("{name} was countered"));
    }
    true
}

/// Resolve a curse aura: attach to a target player and move to battlefield.
pub fn resolve_curse(state: &mut GameState, curse_id: ObjectId, targets: &[Target], registry: &CardRegistry) -> bool {
    if let Some(Target::Player(player_id)) = targets.first() {
        state.move_object(curse_id, Zone::Battlefield, registry);
        if let Some(obj) = state.get_object_mut(curse_id) {
            obj.attached_to_player = Some(*player_id);
            obj.summoning_sick = false;
        }
        return true;
    }
    false
}

/// Resolve a damage spell: deal `amount` damage to the first target
/// (creature or player), then move the spell to the appropriate zone.
pub fn resolve_damage(state: &mut GameState, spell_id: ObjectId, targets: &[Target], amount: u32, registry: &CardRegistry) {
    if let Some(target) = targets.first() {
        let source_name = state.get_object(spell_id)
            .map_or_else(|| "spell".into(), |o| o.name.clone());
        let effect = PendingEffect::DealDamage {
            amount,
            source_id: spell_id,
            source_name,
        };
        crate::engine::apply_pending_effect(state, target, &effect, registry);
    }
}

/// Resolve a targeted destruction spell: destroy the first target creature
/// via the destruction pipeline (checks indestructible/regeneration).
///
/// The spell's own trip to the graveyard is the engine's — see
/// [`GameState::resolving_spell`] — so this takes no spell id.
pub fn resolve_destroy(
    state: &mut GameState,
    targets: &[Target],
    registry: &crate::cards::CardRegistry,
) {
    if let Some(Target::Object(target_id)) = targets.first() {
        if let Some(obj) = state.get_object(*target_id) {
            if obj.zone == Zone::Battlefield {
                crate::destruction::try_destroy(state, *target_id, registry);
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Choice presentation helpers
//
// These set up AwaitingAction::ResolutionChoice so the game loop
// asks the player to make a decision. The CLI and LLM player both
// know how to render these choices.
// ═══════════════════════════════════════════════════════════════════

/// Present a "choose one target" choice to the player.
///
/// - If `targets` is empty, does nothing.
/// - If mandatory (`optional == false`) and exactly 1 target, auto-applies the effect.
/// - Otherwise, sets up a `ResolutionChoice` for the player to pick.
pub fn present_target_choice(
    state: &mut GameState,
    source_id: ObjectId,
    controller: PlayerId,
    targets: Vec<Target>,
    effect: PendingEffect,
    description: &str,
    optional: bool,
    registry: &CardRegistry,
) {
    if targets.is_empty() {
        return;
    }
    if targets.len() == 1 && !optional {
        // Mandatory with exactly 1 target — auto-apply. Through the caller's
        // registry: building a fresh one here rebuilt all 249 card behaviours
        // mid-resolution, and quietly ignored a caller that had registered an
        // extra card in its own.
        crate::engine::apply_pending_effect(state, &targets[0], &effect, registry);
        return;
    }
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: controller,
        source: source_id,
        choice: ResolutionChoiceKind::ChooseTarget {
            description: description.into(),
            options: targets,
            optional,
            effect,
        },
    });
}

/// Present a "choose one target" choice that is optional ("you may").
pub fn present_optional_target_choice(
    state: &mut GameState,
    source_id: ObjectId,
    controller: PlayerId,
    targets: Vec<Target>,
    effect: PendingEffect,
    description: &str,
    registry: &CardRegistry,
) {
    present_target_choice(state, source_id, controller, targets, effect, description, true, registry);
}

// ═══════════════════════════════════════════════════════════════════
// Target collection helpers
//
// Build lists of valid targets for common patterns.
// ═══════════════════════════════════════════════════════════════════

/// Put a list of targets into a stable order.
///
/// Every list below is built by iterating `state.objects`, which is a HashMap
/// and yields in arbitrary order. Players — and the LLM player especially —
/// pick from these lists by position, so an unstable order means the same game
/// replays differently and a recorded decision means something else on the way
/// back. Objects sort by id, players after objects and by id among themselves.
fn stable(mut targets: Vec<Target>) -> Vec<Target> {
    targets.sort_by_key(|t| match t {
        Target::Object(id) => (0u8, id.0),
        Target::Player(p) => (1u8, u64::from(p.0)),
        Target::Illegal => (2u8, 0),
    });
    targets
}

/// All targetable creatures on the battlefield (respects hexproof/protection).
#[must_use]
pub fn creature_targets(state: &GameState, source_id: ObjectId, controller: PlayerId, registry: &CardRegistry) -> Vec<Target> {
    stable(state.all_objects_in_zone(Zone::Battlefield).into_iter()
        .filter(|o| state.is_creature(o.id, registry))
        .filter(|o| crate::engine::can_be_targeted_by(state, o.id, controller, Some(source_id), registry))
        .map(|o| Target::Object(o.id))
        .collect())
}

/// All targetable creatures on the battlefield except a specific one.
#[must_use]
pub fn creature_targets_except(state: &GameState, exclude: ObjectId, source_id: ObjectId, controller: PlayerId, registry: &CardRegistry) -> Vec<Target> {
    stable(state.all_objects_in_zone(Zone::Battlefield).into_iter()
        .filter(|o| state.is_creature(o.id, registry) && o.id != exclude)
        .filter(|o| crate::engine::can_be_targeted_by(state, o.id, controller, Some(source_id), registry))
        .map(|o| Target::Object(o.id))
        .collect())
}

/// Every creature on the battlefield except one — for effects that CHOOSE a
/// creature rather than target it.
///
/// CR 115.1: an effect targets only where the word "target" appears. Hexproof
/// (702.11) and protection (702.16) restrict targeting, so neither applies
/// here. Evil Twin's "a copy of any creature on the battlefield" is a
/// replacement-effect choice (CR 614.12b), and an opponent's hexproof creature
/// is a legal thing to copy — using the targeting helper wrongly hid it.
#[must_use]
pub fn creature_choices_except(state: &GameState, exclude: ObjectId, registry: &CardRegistry) -> Vec<Target> {
    stable(state.all_objects_in_zone(Zone::Battlefield).into_iter()
        .filter(|o| o.id != exclude)
        .filter(|o| state.is_creature(o.id, registry))
        .map(|o| Target::Object(o.id))
        .collect())
}

/// All targetable creatures + planeswalkers + all players ("any target").
#[must_use]
pub fn any_targets(state: &GameState, source_id: ObjectId, controller: PlayerId, registry: &CardRegistry) -> Vec<Target> {
    let mut targets = creature_targets(state, source_id, controller, registry);
    // Add planeswalkers (which have power = None, so creature_targets misses them)
    for o in state.all_objects_in_zone(Zone::Battlefield) {
        if !state.is_creature(o.id, registry)
            && state.has_card_type(o.id, crate::types::CardType::Planeswalker, registry)
            && crate::engine::can_be_targeted_by(state, o.id, controller, Some(source_id), registry)
        {
            targets.push(Target::Object(o.id));
        }
    }
    for player in &state.players {
        if crate::engine::can_target_player(state, player.id, controller, registry) {
            targets.push(Target::Player(player.id));
        }
    }
    stable(targets)
}

/// All targetable creatures + planeswalkers + all players, excluding a specific object.
#[must_use]
pub fn any_targets_except(state: &GameState, exclude: ObjectId, source_id: ObjectId, controller: PlayerId, registry: &CardRegistry) -> Vec<Target> {
    let mut targets = creature_targets_except(state, exclude, source_id, controller, registry);
    // Add planeswalkers (which have power = None, so creature_targets misses them)
    for o in state.all_objects_in_zone(Zone::Battlefield) {
        if !state.is_creature(o.id, registry) && o.id != exclude
            && state.has_card_type(o.id, crate::types::CardType::Planeswalker, registry)
            && crate::engine::can_be_targeted_by(state, o.id, controller, Some(source_id), registry)
        {
            targets.push(Target::Object(o.id));
        }
    }
    for player in &state.players {
        if crate::engine::can_target_player(state, player.id, controller, registry) {
            targets.push(Target::Player(player.id));
        }
    }
    stable(targets)
}

/// All creatures controlled by a specific player.
#[must_use]
pub fn creatures_controlled_by(state: &GameState, player: PlayerId, registry: &CardRegistry) -> Vec<Target> {
    stable(state.objects_in_zone(Zone::Battlefield, player).into_iter()
        .filter(|o| state.is_creature(o.id, registry))
        .map(|o| Target::Object(o.id))
        .collect())
}

/// The single opponent in a 2-player game (auto-target convenience).
#[must_use]
pub fn opponent_player(state: &GameState, controller: PlayerId) -> Target {
    Target::Player(state.opponent(controller))
}

/// The controller of the activated ability currently resolving — the player
/// its "you" refers to.
///
/// CR 602.2a: an ability's controller is the player who activated it, fixed
/// when it went on the stack. That is not always the source's controller by
/// the time it resolves: take the source in response and `controller_of` names
/// the thief, so "you gain 2 life" paid the wrong player and "for as long as
/// you control it" handed over the wrong permanent.
///
/// Falls back to the source's last known controller, which is the right answer
/// for a triggered ability or an effect reached outside a resolving activated
/// ability.
#[must_use]
pub fn ability_controller(state: &GameState, source_id: ObjectId) -> PlayerId {
    state.resolving_ability_activator
        .unwrap_or_else(|| controller_of(state, source_id))
}

/// The controller of an ability's source — the player the ability's "you"
/// refers to.
///
/// CR 608.2g: an ability that resolves after its source has left the
/// battlefield uses the source's last known information, and that includes who
/// controlled it. Reading `o.controller` instead gave the wrong answer in
/// exactly that case, because leaving the battlefield resets `controller` to
/// `owner` — so a card whose comment said "this still happens if the source is
/// destroyed in response" (CR 113.7a) handed the effect to the owner. For a
/// source still on the battlefield the two are the same.
#[must_use]
pub fn controller_of(state: &GameState, object_id: ObjectId) -> PlayerId {
    state.last_known_controller(object_id)
}

/// Is this object still on the battlefield?
///
/// The companion question to `controller_of`, and a separate one. Cards kept
/// asking both at once:
///
/// ```ignore
/// let controller = match state.get_object(self_id) {
///     Some(o) if o.zone == Zone::Battlefield => o.controller,
///     _ => return,
/// };
/// ```
///
/// which reads as "who is my controller" but *behaves* as "do nothing at all
/// if my source is gone" — and that is wrong for a triggered ability, which
/// resolves whether or not its source survived (CR 113.7a). Hamlet Captain
/// stopped pumping the team, and Ghoulraiser stopped returning a Zombie, if
/// the source was killed in response to its own trigger.
///
/// Splitting the two makes each card say which one it meant. An ability whose
/// effect genuinely needs the permanent — Tree of Redemption exchanging a life
/// total with *its* toughness — asks this; everything else just asks
/// `controller_of` and lets the individual effects (`add_counters`,
/// `apply_transform`) decline on their own, which they already do.
#[must_use]
pub fn still_on_battlefield(state: &GameState, object_id: ObjectId) -> bool {
    state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield)
}

// ═══════════════════════════════════════════════════════════════════
// Transform helpers
//
// Generic transform logic for double-faced cards.
// ═══════════════════════════════════════════════════════════════════

/// Transform a double-faced permanent.
///
/// This flips `is_transformed` and nothing else that matters: every
/// characteristics accessor resolves through `GameState::face_data`, which
/// reads that flag, so the permanent's subtypes, keywords, card types and
/// colors all follow automatically. `name` is refreshed only because it is a
/// display cache for logging with no registry lookup behind it — rules code
/// reads `GameState::name_of`.
///
/// It used to copy the new face's name, keywords and subtypes onto the object,
/// which made those fields a second source of truth that a card hand-rolling
/// its own transform could leave stale. There is nothing left to leave stale.
pub fn apply_transform(state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
    let (card_id, was_transformed) = match state.get_object(object_id) {
        // A token copy of a double-faced card has only the copied face — it is
        // not itself a double-faced card, so it cannot transform (CR 111.7,
        // and the Back from the Brink ruling says so explicitly). A token
        // stamped with a DFC's `card_id` would otherwise pick up that card's
        // upkeep trigger and flip.
        Some(o) if o.zone == Zone::Battlefield && !o.is_token => (o.card_id, o.is_transformed),
        _ => return,
    };
    let Some(behavior) = registry.get(card_id) else { return; };

    // CR 701.28c: only a double-faced permanent can transform. Without this,
    // "transform all Humans" flipped `is_transformed` on a single-faced Human
    // — a permanent claiming to show a face it does not have. Moonmist filtered
    // for a back face itself before asking, which made the card right and left
    // this function willing to corrupt anything else that asked.
    //
    // `back_face_data` is the whole test: `every_card_with_a_back_face_declares_it`
    // holds every card Scryfall gives a back face to declaring one, Garruk
    // Relentless included.
    if behavior.back_face_data().is_none() {
        return;
    }

    // Refresh the display cache with the face now showing.
    let new_name = if was_transformed {
        Some(behavior.card_data().name)
    } else {
        behavior.back_face_data().map(|back| back.name)
    };

    let old_name = state.obj_name(object_id);
    if let Some(obj) = state.get_object_mut(object_id) {
        obj.is_transformed = !was_transformed;
        if let Some(name) = new_name {
            obj.name = name;
        }
    }
    // The log belongs here, with the flip, and not in each card. Nineteen
    // cards used to write their own line around this call, which meant they
    // announced a transform on the paths where this function refuses one — a
    // token copy of a double-faced card logged "Reckless Waif transforms into
    // Merciless Predator" and then stayed a Waif. Several also hardcoded both
    // face names, and one ("Transforms into Stalking Vampire") named neither
    // the permanent nor its controller.
    let new_name = state.obj_name(object_id);
    state.log(crate::state::LogLevel::Event,
        format!("{old_name} transforms into {new_name}"));
}

/// The werewolf upkeep trigger: "At the beginning of each upkeep, if <no
/// spells were cast last turn / a player cast two or more spells last turn>,
/// transform this creature."
///
/// Whether it transforms is `should_transform`, which every werewolf answers
/// with `werewolf_should_transform`; whether it triggers at all is
/// `werewolf_should_trigger`. What is left — the battlefield check and the
/// flip — is the same twelve times over, so it lives here.
///
/// CR 603.4 makes the condition an intervening-if, re-checked on resolution,
/// which is why `should_transform` is asked again here and not only at
/// trigger time.
pub fn werewolf_on_upkeep(
    behavior: &dyn CardBehavior,
    state: &mut GameState,
    self_id: ObjectId,
    registry: &CardRegistry,
) {
    if state.get_object(self_id).is_none_or(|o| o.zone != Zone::Battlefield) {
        return;
    }
    if behavior.should_transform(state, self_id, registry) {
        apply_transform(state, self_id, registry);
    }
}

/// Format a tap plan as a short human-readable string like `tap 2x Swamp, Sol Ring`.
/// Groups identical names with a count prefix; returns an empty string if the
/// tap plan is empty. Engine-side analogue of `format_tap_plan` in mtg-player,
/// for use in resolution-prompt descriptions (e.g. Screeching Bat's may-pay).
#[must_use]
pub fn format_tap_plan_names(state: &GameState, tap_plan: &[(ObjectId, usize)]) -> String {
    if tap_plan.is_empty() {
        return String::new();
    }
    let names: Vec<String> = tap_plan.iter()
        .map(|&(id, _)| state.obj_name(id))
        .collect();
    // Group consecutive identical names into "Nx Name" form, preserving
    // the tap plan's order.
    let mut groups: Vec<(String, usize)> = Vec::new();
    for name in names {
        if let Some(last) = groups.last_mut() {
            if last.0 == name {
                last.1 += 1;
                continue;
            }
        }
        groups.push((name, 1));
    }
    let parts: Vec<String> = groups.into_iter()
        .map(|(n, c)| if c == 1 { n } else { format!("{c}x {n}") })
        .collect();
    format!("tap {}", parts.join(", "))
}

/// CR 603.4 intervening-if gate for the werewolf day/night transform trigger.
///
/// Every werewolf DFC in the set reads "At the beginning of each upkeep, **if**
/// [no spells were cast last turn | a player cast two or more spells last
/// turn], transform this creature." That `if` is an intervening-if clause: the
/// ability only triggers when the condition already holds at the beginning of
/// upkeep. When it doesn't, nothing goes on the stack — and so no player gets
/// the priority window that a stack entry would open.
///
/// Delegating to the card's own `should_transform` is what keeps the
/// dispatch-time check and the resolution-time check from ever disagreeing.
/// Werewolves pass this straight through from `CardBehavior::should_trigger`.
///
/// Only `Upkeep` is gated; a werewolf face with an unconditional trigger of
/// another kind (Howlpack Alpha's end-step Wolf token) still fires normally.
/// The Innistrad werewolf transform condition, both directions.
///
/// Front face: "At the beginning of each upkeep, if no spells were cast last
/// turn, transform this creature."
/// Back face: "...if a player cast two or more spells last turn, transform
/// this creature."
///
/// Twelve cards carried a byte-identical private copy of this, and every copy
/// carried the same invention: `&& !state.is_first_turn`. There is no such
/// clause in the oracle text. "No spells were cast last turn" is satisfied
/// when there was no last turn — zero spells were cast in it.
#[must_use]
// ═══════════════════════════════════════════════════════════════════
// Replacement effects (CR 614)
//
// Two shapes cover nearly every replacement a permanent has about its own
// arrival, so they are written once here rather than at each card.
// ═══════════════════════════════════════════════════════════════════

/// "This permanent enters the battlefield tapped unless <condition>"
/// (CR 614.1d) — the five Innistrad check lands.
pub fn enters_tapped_unless(
    self_id: ObjectId,
    event: &crate::replacement::ReplaceableEvent,
    untapped_if: impl FnOnce() -> bool,
) -> Option<crate::replacement::Replacement> {
    use crate::replacement::{ReplaceableEvent, Replacement};
    let ReplaceableEvent::EntersBattlefield(e) = event else { return None };
    if e.object != self_id || e.tapped || untapped_if() {
        return None;
    }
    let mut e = e.clone();
    e.tapped = true;
    Some(Replacement::Modified(ReplaceableEvent::EntersBattlefield(e)))
}

/// "This permanent enters the battlefield with N counters on it"
/// (CR 614.1c). `counters` is only consulted for this permanent's own arrival.
pub fn enters_with_counters(
    self_id: ObjectId,
    event: &crate::replacement::ReplaceableEvent,
    counters: impl FnOnce() -> Vec<(crate::types::CounterType, u32)>,
) -> Option<crate::replacement::Replacement> {
    use crate::replacement::{ReplaceableEvent, Replacement};
    let ReplaceableEvent::EntersBattlefield(e) = event else { return None };
    if e.object != self_id {
        return None;
    }
    let extra = counters();
    if extra.is_empty() {
        return None;
    }
    let mut e = e.clone();
    e.counters.extend(extra);
    Some(Replacement::Modified(ReplaceableEvent::EntersBattlefield(e)))
}

pub fn werewolf_should_transform(state: &GameState, object_id: ObjectId) -> bool {
    // Which face's condition to test. Inside a trigger resolution it is the
    // face the ability triggered from (CR 603.4 re-checks *that* ability's
    // condition, and CR 712.8 keeps it the same object across a flip); outside
    // one — the trigger-time check — it is the face that is up now.
    //
    // These differ when the permanent transforms between the trigger and its
    // resolution: Moonmist in response to a front-face Werewolf's upkeep
    // trigger flips it forward, and the front face's ability must still resolve
    // and flip it back, because "no spells were cast last turn" is unchanged by
    // anything cast this turn. Reading the current face instead tested the back
    // face's condition and did nothing.
    let back_face = state.resolving_trigger_from_back_face
        .unwrap_or_else(|| state.get_object(object_id).is_some_and(|o| o.is_transformed));
    if back_face {
        state.num_spells_cast_last_turn.values().any(|&count| count >= 2)
    } else {
        state.num_spells_cast_last_turn.values().sum::<u32>() == 0
    }
}

pub fn werewolf_should_trigger(
    behavior: &dyn CardBehavior,
    state: &GameState,
    self_id: ObjectId,
    kind: &TriggerKind,
    registry: &CardRegistry,
) -> bool {
    if *kind == TriggerKind::Upkeep {
        // A token copy of a werewolf cannot transform, so its transform
        // ability has nothing to do and should not reach the stack at all.
        if state.get_object(self_id).is_some_and(|o| o.is_token) {
            return false;
        }
        return behavior.should_transform(state, self_id, registry);
    }
    true
}

/// CR 603.4 intervening-if gate for the morbid enters-the-battlefield trigger.
///
/// "Morbid — When this creature enters, **if** a creature died this turn, ..."
/// is an intervening-if clause, so with no creature dead this turn the ability
/// doesn't trigger and no stack entry appears. Resolution handlers keep their
/// own `creature_died_this_turn` guard, which is what made the *outcome*
/// correct before this gate existed — only the phantom stack entry was wrong.
pub fn morbid_should_trigger(state: &GameState, kind: &TriggerKind) -> bool {
    if *kind == TriggerKind::EntersBattlefield {
        return state.creature_died_this_turn;
    }
    true
}

/// CR 701.19: search a library, take at most one matching card, then shuffle.
///
/// Handles the whole shape once: no match (shuffle and move on), exactly one
/// (take it without asking — there is no choice to make), or several (ask, and
/// finish in `finish_library_search` when the answer comes back). Five cards
/// each had their own copy of this, with subtly different behaviour in the
/// zero- and one-candidate cases.
///
/// `candidates` is what the card considers a legal find; `destination` and
/// `tapped` are where it ends up. The shuffle always happens, even when
/// nothing is found — you searched.
pub fn search_library(
    state: &mut GameState,
    source_id: ObjectId,
    searcher: PlayerId,
    candidates: Vec<ObjectId>,
    destination: Zone,
    tapped: bool,
    optional: bool,
    description: &str,
) {
    // "You MAY search" is a real decision even when only one card qualifies,
    // or when nothing does — the player is entitled to decline, and a player
    // who declines never searched, so they do not shuffle either. Asking
    // regardless also keeps the prompt from leaking whether the library holds
    // a match: an unconditional shuffle told the table there was nothing to
    // find. With no candidates the only answer available is "decline", which
    // is the right outcome, not a reason to skip the question.
    if optional {
        let options: Vec<Target> = candidates.into_iter().map(Target::Object).collect();
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: searcher,
            source: source_id,
            choice: ResolutionChoiceKind::ChooseTarget {
                description: description.to_string(),
                options,
                optional: true,
                effect: PendingEffect::FinishLibrarySearch { searcher, destination, tapped },
            },
        });
        return;
    }

    // Mandatory search from here on. CR 701.19a: the search happened whether or
    // not anything was found, so the shuffle happens either way.
    //
    // CR 701.19b: a player searching a hidden zone "isn't required to find some
    // or all of those cards even if they're present in that zone". Mandatory
    // means the player must search and shuffle — not that they must take a
    // card. So the choice is offered even when exactly one card qualifies,
    // where this used to take it for them.
    if candidates.is_empty() {
        // Nothing to offer, so nothing to decline. Debug level on purpose: a
        // player who searched and came back with nothing is not obliged to say
        // whether there was anything to find, and this line would say it for
        // them.
        state.log(crate::state::LogLevel::Debug,
            format!("{}: no matching card in library", state.obj_name(source_id)));
        shuffle_library(state, searcher);
        return;
    }
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: searcher,
        source: source_id,
        choice: ResolutionChoiceKind::ChooseFromLibrary {
            description: description.to_string(),
            options: candidates,
            searcher,
            source_id,
            destination,
            tapped,
        },
    });
}

/// Move a found card out of the library to its destination, then shuffle.
/// Shared by the auto-pick path in `search_library` and the engine's handler
/// for the player's answer, so both behave identically.
pub fn finish_library_search(
    state: &mut GameState,
    searcher: PlayerId,
    found: ObjectId,
    destination: Zone,
    tapped: bool,
    registry: &CardRegistry,
) {
    let name = state.obj_name(found);
    state.move_object(found, destination, registry);
    if destination == Zone::Battlefield {
        if let Some(obj) = state.get_object_mut(found) {
            obj.summoning_sick = false;
            if tapped {
                obj.tapped = true;
            }
        }
    }
    state.log(crate::state::LogLevel::Event,
        format!("p{} searched their library and found {name}", searcher.0));
    shuffle_library(state, searcher);
}

/// CR 701.20: shuffle a player's library.
pub fn shuffle_library(state: &mut GameState, player: PlayerId) {
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    state.get_player_mut(player).library_order.shuffle(&mut rng);
}

/// CR 104.3 / "at random": take up to `n` of `candidates`, chosen at random.
///
/// Five cards wrote this as "shuffle my candidate vector, then `.take(n)`" and
/// a sixth as `.choose()` — Charmbreaker Devils, Ghoulraiser, Make a Wish,
/// Moldgraf Monstrosity, Woodland Sleuth, Desperate Ravings. Same rule, six
/// copies, and six places that would each need a seed. Fewer than `n`
/// candidates yields all of them, which is what "at random" does with a short
/// list.
#[must_use]
pub fn choose_at_random(candidates: &[ObjectId], n: usize) -> Vec<ObjectId> {
    use rand::seq::SliceRandom;
    let mut pool = candidates.to_vec();
    pool.shuffle(&mut rand::thread_rng());
    pool.truncate(n);
    pool
}

/// CR 705.2: flip a coin. True is "you win the flip".
#[must_use]
pub fn flip_coin() -> bool {
    use rand::Rng;
    rand::thread_rng().gen_bool(0.5)
}
