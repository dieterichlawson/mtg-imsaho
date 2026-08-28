use crate::actions::Action;
use crate::cards::CardRegistry;
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{Zone, CardType, Keyword, Color};

/// Check targeting legality, including protection from the source.
/// `source_id` is the spell or permanent whose ability is targeting.
#[must_use]
pub fn can_be_targeted_by(state: &GameState, target_id: ObjectId, caster: PlayerId, source_id: Option<ObjectId>, registry: &CardRegistry) -> bool {
    if state.has_keyword(target_id, Keyword::Hexproof, registry) {
        let controller = state.get_object(target_id)
            .map_or(PlayerId(255), |o| o.controller);
        if controller != caster {
            return false; // hexproof: can't be targeted by opponents
        }
    }
    // Check protection from the source.
    if let Some(sid) = source_id {
        if state.has_protection_from(target_id, sid, registry) {
            return false;
        }
    }
    true
}
/// Whether `caster` may target `target_player` — the whole rule, in one place.
///
/// It used to be written out three different ways: here, again inline in
/// `stack.rs`'s CR 608.2b re-check, and again in `helpers::any_targets` and
/// `any_targets_except`. Only the callers of *this* one also checked `lost`,
/// and they did it themselves, so "a player who has left the game" was a
/// legal target for an "any target" spell and for every re-check on
/// resolution. Witchbane Orb is the only card in the pool that grants a
/// player hexproof, so each divergent copy was a way for its one static
/// ability to be quietly skipped.
pub(crate) fn can_target_player(state: &GameState, target_player: PlayerId, caster: PlayerId, registry: &CardRegistry) -> bool {
    // CR 104.3a: a player who has lost has left the game and is not there to
    // be targeted.
    if state.players.iter().any(|p| p.id == target_player && p.lost) {
        return false;
    }
    // CR 702.11b: hexproof stops spells and abilities your OPPONENTS control.
    // Your own still reach you.
    if target_player != caster && state.player_has_hexproof(target_player, registry) {
        return false;
    }
    true
}
/// Determine which mode of a `ModalChoice` was selected, based on the chosen targets.
/// For each mode, checks if all chosen targets are valid. Returns the first matching
/// mode index, defaulting to 0 if ambiguous (e.g. empty targets valid for all modes).
pub(crate) fn detect_modal_choice_mode(
    state: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    targets: &[crate::actions::Target],
    modes: &[crate::cards::TargetRequirement],
    behavior: &dyn crate::cards::CardBehavior,
    registry: &CardRegistry,
) -> usize {
    // For non-empty targets, find the first mode whose valid targets contain all chosen targets.
    if !targets.is_empty() {
        for (i, mode_req) in modes.iter().enumerate() {
            let valid = valid_targets_for_mode(state, caster, spell_id, mode_req, behavior, registry);
            if targets.iter().all(|t| valid.contains(t)) {
                return i;
            }
        }
    }
    // For empty targets (or no mode matched), default to mode 0.
    0
}
/// Get valid targets for a single mode requirement, unwrapping `UpToTargets`.
pub(crate) fn valid_targets_for_mode(
    state: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    mode_req: &crate::cards::TargetRequirement,
    behavior: &dyn crate::cards::CardBehavior,
    registry: &CardRegistry,
) -> Vec<crate::actions::Target> {
    use crate::cards::TargetRequirement;
    match mode_req {
        TargetRequirement::UpToTargets(_, inner) => valid_targets_for_req(state, caster, spell_id, inner, behavior, registry),
        other => valid_targets_for_req(state, caster, spell_id, other, behavior, registry),
    }
}
/// Generate `CastSpell` actions with all valid target combinations.
/// Every k-sized combination of `targets`, order-insensitive.
pub(crate) fn target_combinations(targets: &[crate::actions::Target], k: usize) -> Vec<Vec<crate::actions::Target>> {
    if k == 0 { return vec![vec![]]; }
    if targets.len() < k { return vec![]; }
    let mut result = Vec::new();
    for i in 0..=targets.len() - k {
        for mut combo in target_combinations(&targets[i + 1..], k - 1) {
            combo.insert(0, targets[i].clone());
            result.push(combo);
        }
    }
    result
}
/// Whether two target requirements ask for the same thing, so a pair drawn
/// from them is a set rather than an ordered pair.
///
/// Compared by shape rather than by `PartialEq`, which `TargetRequirement`
/// does not derive: what matters is that both slots draw from one candidate
/// pool under one restriction.
fn same_requirement(a: &crate::cards::TargetRequirement, b: &crate::cards::TargetRequirement) -> bool {
    format!("{a:?}") == format!("{b:?}")
}

/// Drop cast actions whose target *sets* have already been produced.
fn dedup_by_target_set(actions: &mut Vec<Action>) {
    let mut seen: Vec<Vec<String>> = Vec::new();
    actions.retain(|a| {
        let Action::CastSpell { targets, .. } = a else { return true };
        let mut key: Vec<String> = targets.iter().map(|t| format!("{t:?}")).collect();
        key.sort();
        if seen.contains(&key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
}

pub(crate) fn generate_cast_actions_with_targets(
    state: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    target_req: &crate::cards::TargetRequirement,
    behavior: &dyn crate::cards::CardBehavior,
    registry: &CardRegistry,
) -> Vec<Action> {
    use crate::cards::TargetRequirement;

    match target_req {
        TargetRequirement::None => {
            vec![Action::CastSpell { object_id: spell_id, targets: vec![], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] }]
        }
        TargetRequirement::ModalChoice(ref modes) => {
            let mut actions = Vec::new();
            for mode_req in modes {
                actions.extend(generate_cast_actions_with_targets(state, caster, spell_id, mode_req, behavior, registry));
            }
            actions
        }
        TargetRequirement::TwoTargets(ref req1, ref req2) => {
            let targets1 = valid_targets_for_req(state, caster, spell_id, req1, behavior, registry);
            let mut actions = Vec::new();

            // The second slot may itself be "up to N", in which case the pair
            // is one first target plus 0..=N of the second — not exactly one
            // each. Memory's Journey is `TwoTargets(PlayerOnly, UpToTargets(3,
            // ...))` and produced no action at all under the exactly-one rule.
            let (max2, inner2) = match req2.as_ref() {
                TargetRequirement::UpToTargets(max, inner) => (*max, inner.as_ref()),
                other => (1, other),
            };

            for t1 in &targets1 {
                // "from THEIR graveyard" — the second slot's candidates can
                // depend on the first target, which is only known here.
                let mut options = valid_targets_for_req(state, caster, spell_id, inner2, behavior, registry);
                if matches!(inner2, TargetRequirement::GraveyardCardOwnedByTargetPlayer) {
                    if let crate::actions::Target::Player(pid) = t1 {
                        options.retain(|t| match t {
                            crate::actions::Target::Object(id) =>
                                state.get_object(*id).is_some_and(|o| o.owner == *pid),
                            crate::actions::Target::Player(_) => false,
                            // CR 608.2b: a target that stopped being legal is skipped.
                            crate::actions::Target::Illegal => false,
                        });
                    }
                }
                options.retain(|t| t != t1);

                let lower = if max2 == 1 { 1 } else { 0 };
                for k in lower..=max2.min(options.len()) {
                    for mut combo in target_combinations(&options, k) {
                        let mut pair = vec![t1.clone()];
                        pair.append(&mut combo);
                        actions.push(Action::CastSpell {
                            object_id: spell_id,
                            targets: pair,
                            sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![],
                        });
                    }
                }
            }
            // When both slots want the same thing — Ghoulcaller's Chant's
            // "return two target Zombie creature cards" — the pair is a set,
            // and pairing every candidate with every other produced each set
            // twice, once in each order. That is not a second choice; it just
            // doubles the branching factor for whoever is picking.
            //
            // Where the slots differ (Prey Upon's "creature you control fights
            // creature you don't", Memory's Journey's player-then-their-cards)
            // the order carries meaning and both orderings are real.
            if same_requirement(req1, req2) {
                dedup_by_target_set(&mut actions);
            }
            actions
        }
        TargetRequirement::UpToTargets(max, ref inner_req) => {
            // Generate all combinations of 1..=max targets for LLM/random expanded list.
            let options = valid_targets_for_req(state, caster, spell_id, inner_req, behavior, registry);
            let mut actions = Vec::new();
            // Start from 0 to allow "up to N" to mean "0 or more" (e.g., Memory's Journey
            // can be cast targeting just a player with 0 cards).
            for k in 0..=(*max).min(options.len()) {
                for combo in target_combinations(&options, k) {
                    actions.push(Action::CastSpell {
                        object_id: spell_id,
                        targets: combo,
                        sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![],
                    });
                }
            }
            actions
        }
        // All single-target requirement kinds share the canonical target
        // enumeration in `valid_targets_for_req` — one target per action.
        _ => {
            valid_targets_for_req(state, caster, spell_id, target_req, behavior, registry)
                .into_iter()
                .map(|t| Action::CastSpell { object_id: spell_id, targets: vec![t], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] })
                .collect()
        }
    }
}
/// Drop a target named twice within one instance of the word "target"
/// (CR 601.2c).
///
/// "Put a +1/+1 counter on each of **up to two target creatures**" is one
/// instance covering both slots, so the same creature cannot fill both — the
/// ruling on Travel Preparations says it outright: "You can't target the same
/// creature twice to put two +1/+1 counters on it."
///
/// `generate_cast_actions_with_targets` already honours this, because it
/// enumerates *combinations*. What did not was the submitted list: both
/// clients build their `CastSpell` from a per-slot choice rather than picking
/// a whole offered action, so an LLM answering `[0, 0]` put two counters on
/// one creature. The engine is the authority for a declaration it is handed —
/// the same stance `declare_attackers` takes — so the duplicate is dropped
/// here rather than trusted.
///
/// Only `UpToTargets` is one instance covering many slots. `TwoTargets` is two
/// separate instances, which CR 601.2c does not join, so each half is checked
/// on its own.
pub(crate) fn distinct_within_each_target_instance(
    req: &crate::cards::TargetRequirement,
    targets: &[crate::actions::Target],
) -> Vec<crate::actions::Target> {
    use crate::actions::Target;
    use crate::cards::TargetRequirement as R;
    match req {
        R::UpToTargets(_, _) => {
            let mut seen: Vec<Target> = Vec::new();
            let mut out = Vec::new();
            for t in targets {
                if matches!(t, Target::Object(_)) {
                    if seen.contains(t) {
                        continue;
                    }
                    seen.push(t.clone());
                }
                out.push(t.clone());
            }
            out
        }
        R::TwoTargets(first, second) => {
            let split = targets.len().min(1);
            let mut out = distinct_within_each_target_instance(first, &targets[..split]);
            out.extend(distinct_within_each_target_instance(second, &targets[split..]));
            out
        }
        _ => targets.to_vec(),
    }
}

/// Helper: collect all valid targets for a single-target requirement.
pub(crate) fn valid_targets_for_req(
    state: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    req: &crate::cards::TargetRequirement,
    behavior: &dyn crate::cards::CardBehavior,
    registry: &CardRegistry,
) -> Vec<crate::actions::Target> {
    use crate::actions::Target;
    use crate::cards::TargetRequirement;

    match req {
        TargetRequirement::Creature => {
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| state.is_creature(o.id, registry))
                .filter(|o| can_be_targeted_by(state, o.id, caster, Some(spell_id), registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::CreatureWithFilter(filter) => {
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| state.is_creature(o.id, registry))
                .filter(|o| matches_target_filter(state, o, filter, caster, Some(spell_id), registry))
                .filter(|o| can_be_targeted_by(state, o.id, caster, Some(spell_id), registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::Spell => {
            // Only spells on the stack can be targeted (not triggered abilities).
            state.stack.iter()
                .filter_map(crate::state::StackEntry::as_spell)
                .filter(|&id| id != spell_id)
                .map(Target::Object)
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::PermanentWithFilter(filter) => {
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| matches_target_filter(state, o, filter, caster, Some(spell_id), registry))
                .filter(|o| can_be_targeted_by(state, o.id, caster, Some(spell_id), registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::AnyTarget => {
            let mut targets: Vec<Target> = state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| state.is_creature(o.id, registry)
                    || state.has_card_type(o.id, CardType::Planeswalker, registry))
                .filter(|o| can_be_targeted_by(state, o.id, caster, Some(spell_id), registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect();
            for p in &state.players {
                if can_target_player(state, p.id, caster, registry) {
                    let t = Target::Player(p.id);
                    if behavior.is_valid_target(state, caster, &t, registry) {
                        targets.push(t);
                    }
                }
            }
            targets
        }
        TargetRequirement::PlayerOnly => {
            state.players.iter()
                .filter(|p| can_target_player(state, p.id, caster, registry))
                .map(|p| Target::Player(p.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        // CR 102.1: "target opponent" is every player but the controller.
        TargetRequirement::OpponentOnly => {
            state.players.iter()
                .filter(|p| p.id != caster)
                .filter(|p| can_target_player(state, p.id, caster, registry))
                .map(|p| Target::Player(p.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::PlayerOrPlaneswalker => {
            let mut targets: Vec<Target> = state.players.iter()
                .filter(|p| can_target_player(state, p.id, caster, registry))
                .map(|p| Target::Player(p.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect();
            for obj in state.all_objects_in_zone(Zone::Battlefield) {
                let is_pw = state.has_card_type(obj.id, CardType::Planeswalker, registry);
                if is_pw && can_be_targeted_by(state, obj.id, caster, Some(spell_id), registry) {
                    let t = Target::Object(obj.id);
                    if behavior.is_valid_target(state, caster, &t, registry) {
                        targets.push(t);
                    }
                }
            }
            targets
        }
        TargetRequirement::GraveyardCard => {
            // All cards in all graveyards. CR 109.1: a token is not a card, and
            // CR 704.5e leaves one in a graveyard until the next state-based
            // action pass, so an enumeration taken in between can see one.
            //
            // `o.id != spell_id`, here and on every arm below that enumerates a
            // zone of cards: a spell cast from its graveyard is not in that
            // graveyard any more. CR 601.2a moves the card to the stack and
            // CR 601.2c chooses targets after that, so it cannot be one of its
            // own. Purify the Grave was offered a cast targeting itself.
            state.objects_in_id_order().into_iter()
                .filter(|o| o.id != spell_id && o.zone == Zone::Graveyard && state.is_card(o.id))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::GraveyardCreature => {
            // Creature cards in caster's graveyard.
            state.objects_in_id_order().into_iter()
                .filter(|o| {
                    o.id != spell_id
                        && o.zone == Zone::Graveyard
                        && o.owner == caster
                        && state.is_card(o.id)
                        && state.is_creature(o.id, registry)
                })
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::GraveyardCreatureOfSubtype(ref subtype) => {
            // Creature cards of that subtype in the caster's graveyard —
            // "from your graveyard", the same scope as `GraveyardCreature`
            // above. This used to say "in all graveyards" and omit the owner
            // check, and its only card (Ghoulcaller's Chant) put the check
            // back in its own `is_valid_target`. Two sibling requirements
            // disagreeing about whose graveyard they mean, with the card that
            // uses the looser one compensating, is a trap for the next card.
            state.objects_in_id_order().into_iter()
                .filter(|o| {
                    o.id != spell_id
                        && o.zone == Zone::Graveyard
                        && o.owner == caster
                        && state.is_card(o.id)
                        && state.is_creature(o.id, registry)
                        && state.has_subtype(o.id, subtype, registry)
                })
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::GraveyardCardOwnedByCaster => {
            // Cards in the caster's own graveyard.
            state.objects_in_id_order().into_iter()
                .filter(|o| o.id != spell_id && o.zone == Zone::Graveyard && o.owner == caster && state.is_card(o.id))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::GraveyardCardOwnedByOpponent => {
            // Cards in any opponent's graveyard.
            state.objects_in_id_order().into_iter()
                .filter(|o| o.id != spell_id && o.zone == Zone::Graveyard && o.owner != caster && state.is_card(o.id))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::ExileCard => {
            // All cards in exile owned by the caster.
            state.objects_in_id_order().into_iter()
                .filter(|o| o.id != spell_id && o.zone == Zone::Exile && o.owner == caster && state.is_card(o.id))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        TargetRequirement::UpToTargets(_, inner) => {
            // "Up to N target X" offers the same candidates as "target X"; the
            // count is applied where the combinations are built. Falling
            // through to the catch-all returned an empty list, which made
            // Memory's Journey — whose second slot is `UpToTargets` nested in
            // `TwoTargets` — produce an empty Cartesian product and therefore
            // no cast action at all. The card was uncastable.
            valid_targets_for_req(state, caster, spell_id, inner, behavior, registry)
        }
        TargetRequirement::GraveyardCardOwnedByTargetPlayer => {
            // Which player is only known once the co-target is chosen, so the
            // pairing in `generate_cast_actions_with_targets` narrows this.
            // Unconstrained here, it is every graveyard card but this spell.
            state.objects_in_id_order().into_iter()
                .filter(|o| o.id != spell_id && o.zone == Zone::Graveyard && state.is_card(o.id))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect()
        }
        _ => vec![],
    }
}
/// Build a `CastTargetSpec` for a spell, describing what targets the player needs to choose.
pub(crate) fn build_cast_target_spec(
    state: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    target_req: &crate::cards::TargetRequirement,
    behavior: &dyn crate::cards::CardBehavior,
    registry: &CardRegistry,
) -> crate::actions::CastTargetSpec {
    use crate::actions::CastTargetSpec;
    use crate::cards::TargetRequirement;

    match target_req {
        TargetRequirement::None => CastTargetSpec::NoTargets,
        TargetRequirement::TwoTargets(req1, req2) => {
            let t1 = valid_targets_for_req(state, caster, spell_id, req1, behavior, registry);
            let t2 = valid_targets_for_req(state, caster, spell_id, req2, behavior, registry);
            CastTargetSpec::TwoTargets(t1, t2)
        }
        TargetRequirement::UpToTargets(max, inner_req) => {
            let options = valid_targets_for_req(state, caster, spell_id, inner_req, behavior, registry);
            CastTargetSpec::UpToTargets { max: *max, options }
        }
        TargetRequirement::ModalChoice(ref modes) => {
            // Collect all possible targets across all modes.
            let mut all_options = Vec::new();
            for mode_req in modes {
                all_options.extend(valid_targets_for_req(state, caster, spell_id, mode_req, behavior, registry));
            }
            all_options.dedup();
            CastTargetSpec::SingleTarget(all_options)
        }
        // All single-target types
        _ => {
            let options = valid_targets_for_req(state, caster, spell_id, target_req, behavior, registry);
            CastTargetSpec::SingleTarget(options)
        }
    }
}
/// Check whether an object satisfies a `TargetFilter`.
///
/// The single canonical filter matcher — used by spell targeting, ability
/// targeting, and resolution-time legality checks (stack.rs). All
/// characteristic lookups go through the `GameState` characteristics layer,
/// so non-token permanents (empty object-level fields) and transformed DFCs
/// are handled uniformly.
///
/// `source_id` is the permanent or spell the targeting originates from; it
/// only affects `Another` and `SameNameAsSource`. Pass `None` when no source
/// is available (resolution-time recheck), which leaves `Another`
/// unrestricted.
pub(crate) fn matches_target_filter(
    state: &GameState,
    obj: &crate::state::GameObject,
    filter: &crate::cards::TargetFilter,
    controller: PlayerId,
    source_id: Option<ObjectId>,
    registry: &CardRegistry,
) -> bool {
    use crate::cards::TargetFilter;
    match filter {
        TargetFilter::Any => true,
        TargetFilter::YouControl => obj.controller == controller,
        TargetFilter::YouDontControl => obj.controller != controller,
        TargetFilter::Nonblack => !state.colors_of(obj.id, registry).contains(&Color::Black),
        TargetFilter::NotSubtypes(types) => {
            let subtypes = state.subtypes_of(obj.id, registry);
            !types.iter().any(|t| subtypes.contains(t))
        }
        TargetFilter::PowerAtLeast(n) => {
            state.effective_power(obj.id, registry).unwrap_or(0) >= *n
        }
        TargetFilter::Attacking => {
            state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&obj.id))
        }
        TargetFilter::Noncreature => !state.is_creature(obj.id, registry),
        TargetFilter::HasCardType(types) => {
            types.iter().any(|t| state.has_card_type(obj.id, *t, registry))
        }
        TargetFilter::SubtypeOrCardType { subtypes, card_types } => {
            subtypes.iter().any(|s| state.has_subtype(obj.id, s, registry))
                || card_types.iter().any(|t| state.has_card_type(obj.id, *t, registry))
        }
        TargetFilter::HasSubtype(subtype) => state.has_subtype(obj.id, subtype, registry),
        TargetFilter::HasKeyword(keyword) => state.has_keyword(obj.id, *keyword, registry),
        TargetFilter::Another => source_id.is_none_or(|s| obj.id != s),
        TargetFilter::SameNameAsSource => {
            // `name_of`, not `obj.name`: a name comparison is a rules decision
            // and has to read the active face. CR 712.8a — a double-faced
            // permanent has only the name of the face that is up.
            source_id.is_some_and(|s| state.name_of(s, registry) == state.name_of(obj.id, registry))
        }
    }
}
/// Generate valid targets for a targeted activated ability.
pub(crate) fn generate_ability_targets(
    state: &GameState,
    source_id: ObjectId,
    ab: &crate::cards::ActivatedAbilityDef,
    controller: PlayerId,
    registry: &CardRegistry,
    behavior: &dyn crate::cards::CardBehavior,
) -> Vec<crate::actions::Target> {
    use crate::actions::Target;
    use crate::cards::TargetRequirement;

    let Some(target_req) = &ab.target_requirement else { return vec![]; };

    match target_req {
        TargetRequirement::Creature => {
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| state.is_creature(o.id, registry))
                .filter(|o| can_be_targeted_by(state, o.id, controller, Some(source_id), registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        TargetRequirement::CreatureWithFilter(filter) => {
            // CR 702.6a: equip is "Attach this permanent to target creature you
            // control" — nothing excludes the creature it is already attached
            // to. This used to filter that creature out as a UX shortcut, which
            // made a legal play unavailable: re-equipping to the same host is
            // the point whenever the equip COST is what you want (Demonmail
            // Hauberk sacrificing a different creature), and with only one
            // creature on the battlefield it removed the ability entirely.
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| state.is_creature(o.id, registry))
                .filter(|o| can_be_targeted_by(state, o.id, controller, Some(source_id), registry))
                .filter(|o| matches_target_filter(state, o, filter, controller, Some(source_id), registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        TargetRequirement::PlayerOnly => {
            state.players.iter()
                .filter(|p| can_target_player(state, p.id, controller, registry))
                .map(|p| Target::Player(p.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        // CR 602.2a: the ability's controller is the activator, so "opponent"
        // is measured from them, not from whoever holds the source.
        TargetRequirement::OpponentOnly => {
            state.players.iter()
                .filter(|p| p.id != controller)
                .filter(|p| can_target_player(state, p.id, controller, registry))
                .map(|p| Target::Player(p.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        TargetRequirement::PlayerOrPlaneswalker => {
            let mut targets: Vec<Target> = state.players.iter()
                .filter(|p| can_target_player(state, p.id, controller, registry))
                .map(|p| Target::Player(p.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect();
            for obj in state.all_objects_in_zone(Zone::Battlefield) {
                let is_pw = state.has_card_type(obj.id, CardType::Planeswalker, registry);
                if is_pw && can_be_targeted_by(state, obj.id, controller, Some(source_id), registry) {
                    let t = Target::Object(obj.id);
                    if behavior.is_valid_target(state, controller, &t, registry) {
                        targets.push(t);
                    }
                }
            }
            targets
        }
        TargetRequirement::AnyTarget => {
            let mut targets: Vec<Target> = state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| state.is_creature(o.id, registry)
                    || state.has_card_type(o.id, CardType::Planeswalker, registry))
                .filter(|o| can_be_targeted_by(state, o.id, controller, Some(source_id), registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect();
            for p in &state.players {
                if can_target_player(state, p.id, controller, registry) {
                    let t = Target::Player(p.id);
                    if behavior.is_valid_target(state, controller, &t, registry) {
                        targets.push(t);
                    }
                }
            }
            targets
        }
        TargetRequirement::PermanentWithFilter(filter) => {
            state.all_objects_in_zone(Zone::Battlefield).iter()
                .filter(|o| can_be_targeted_by(state, o.id, controller, Some(source_id), registry))
                .filter(|o| matches_target_filter(state, o, filter, controller, Some(source_id), registry))
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        TargetRequirement::GraveyardCard => {
            state.objects_in_id_order().into_iter()
                .filter(|o| o.zone == Zone::Graveyard)
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        TargetRequirement::ExileCard => {
            state.objects_in_id_order().into_iter()
                .filter(|o| o.zone == Zone::Exile && o.owner == controller)
                .map(|o| Target::Object(o.id))
                .filter(|t| behavior.is_valid_target(state, controller, t, registry))
                .collect()
        }
        _ => vec![],
    }
}
pub(crate) fn combinations(items: &[ObjectId], k: usize) -> Vec<Vec<ObjectId>> {
    if k == 0 {
        return vec![vec![]];
    }
    if items.len() < k {
        return vec![];
    }
    let mut result = Vec::new();
    for i in 0..=items.len() - k {
        let rest = combinations(&items[i + 1..], k - 1);
        for mut combo in rest {
            combo.insert(0, items[i]);
            result.push(combo);
        }
    }
    result
}
