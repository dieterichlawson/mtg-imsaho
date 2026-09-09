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
            if !arity_ok(mode_req, targets.len()) {
                continue;
            }
            let valid = valid_targets_for_req(state, caster, spell_id, mode_req, behavior, registry);
            if targets.iter().all(|t| valid.contains(t)) {
                return i;
            }
        }
    }
    // For empty targets (or no mode matched), default to mode 0.
    0
}
/// Whether `n` chosen targets is a count the requirement allows (CR 601.2c).
///
/// The counting half of the vocabulary, beside the half that decides the
/// candidates. `detect_modal_choice_mode` reads a cast's mode back off its
/// targets and needs both — "return target creature card" and "return two
/// target Zombie cards" can name the same two cards, and only the number
/// tells them apart — and the stack invariant asks the same question of a
/// spell already on it.
pub(crate) fn arity_ok(req: &crate::cards::TargetRequirement, n: usize) -> bool {
    use crate::cards::TargetRequirement as R;
    match req {
        R::None => n == 0,
        R::UpToTargets(k, _) => n <= *k,
        R::TwoTargets(a, b) => (0..=n).any(|x| arity_ok(a, x) && arity_ok(b, n - x)),
        R::ModalChoice(modes) => modes.iter().any(|m| arity_ok(m, n)),
        _ => n == 1,
    }
}

/// The requirement that decides the *candidates*, with any "up to N" peeled
/// off.
///
/// CR 601.2c picks the number of targets and then the targets themselves out
/// of one pool, so "up to two target creatures" draws from the same pool as
/// "target creature" — the count is a separate question, answered by
/// [`most_targets`] and [`fewest_targets`]. Only `second_slot_options` needs
/// this, to see the requirement *kind* through the wrapper;
/// `valid_targets_for_req` recurses through `UpToTargets` on its own.
fn candidate_req(req: &crate::cards::TargetRequirement) -> &crate::cards::TargetRequirement {
    match req {
        crate::cards::TargetRequirement::UpToTargets(_, inner) => candidate_req(inner),
        other => other,
    }
}

/// How many targets a requirement takes at most: N for "up to N", one
/// otherwise.
/// The "up to N" slot of a requirement, if it has one: the options for it,
/// how many may be chosen, and how many targets come before it.
///
/// `UpToTargets` is the slot itself; `TwoTargets(a, UpToTargets(..))` has
/// one fixed target in front of it (Memory's Journey names a player, then
/// up to three cards from their graveyard). Everything else has none, and
/// its targets are enumerated as before.
pub(crate) fn up_to_slot(
    state: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    req: &crate::cards::TargetRequirement,
    chosen: &[crate::actions::Target],
    behavior: &dyn crate::cards::CardBehavior,
    registry: &CardRegistry,
) -> Option<(Vec<crate::actions::Target>, usize, usize)> {
    use crate::cards::TargetRequirement as R;
    match req {
        R::UpToTargets(max, _) => {
            let options = valid_targets_for_req(state, caster, spell_id, req, behavior, registry);
            let max = (*max).min(options.len());
            Some((options, 0, max))
        }
        R::TwoTargets(_, second) if matches!(**second, R::UpToTargets(..)) => {
            // The first slot has to be named before the second's options
            // are known — Memory's Journey searches the named player's
            // graveyard.
            let first = chosen.first()?;
            let options = second_slot_options(state, caster, spell_id, second, first, behavior, registry);
            let max = most_targets(second).min(options.len());
            Some((options, fewest_targets(second), max))
        }
        _ => None,
    }
}

fn most_targets(req: &crate::cards::TargetRequirement) -> usize {
    match req {
        crate::cards::TargetRequirement::UpToTargets(max, _) => *max,
        _ => 1,
    }
}

/// How many it takes at least: none for "up to N" — CR 601.2c lets you choose
/// zero — and one otherwise.
///
/// Read off the requirement's shape rather than from `most_targets(req) == 1`,
/// which both call sites used to do and which says "exactly one" for an
/// `UpToTargets(1, _)`.
fn fewest_targets(req: &crate::cards::TargetRequirement) -> usize {
    usize::from(!matches!(req, crate::cards::TargetRequirement::UpToTargets(..)))
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
            let lower = fewest_targets(req2);
            let max2 = most_targets(req2);

            // A second slot that is itself "up to N" is chosen through the
            // prompt the cast raises, not enumerated: one action per first
            // target, with the second slot empty. Memory's Journey is
            // `TwoTargets(PlayerOnly, UpToTargets(3, ...))`, and enumerating
            // it over a fifteen-card graveyard is about 1,150 actions.
            let up_to_second = matches!(**req2, TargetRequirement::UpToTargets(..));
            for t1 in &targets1 {
                if up_to_second {
                    actions.push(Action::CastSpell {
                        object_id: spell_id,
                        targets: vec![t1.clone()],
                        sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![],
                    });
                    continue;
                }
                let options = second_slot_options(state, caster, spell_id, req2, t1, behavior, registry);

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
        TargetRequirement::UpToTargets(..) => {
            // One cast, with the targets left to the prompt the cast raises
            // (CR 601.2c). This used to enumerate every subset of size
            // 0..=max, which is `sum(C(n, k))` actions — a menu that grows
            // exponentially in the board and that a non-interactive seat
            // reads in full.
            vec![Action::CastSpell {
                object_id: spell_id,
                targets: vec![],
                sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![],
            }]
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
/// The legal second-slot candidates of a `TwoTargets` requirement once the
/// first target is known. "From THEIR graveyard" — the second slot's
/// candidates can depend on the first target, so every path that offers or
/// enumerates second-slot choices (expanded actions AND the interactive
/// `CastTargetSpec`) must narrow through here, not call
/// `valid_targets_for_req` on the raw requirement (issue #46).
fn second_slot_options(
    state: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    inner2: &crate::cards::TargetRequirement,
    first: &crate::actions::Target,
    behavior: &dyn crate::cards::CardBehavior,
    registry: &CardRegistry,
) -> Vec<crate::actions::Target> {
    use crate::cards::TargetRequirement;
    let mut options = valid_targets_for_req(state, caster, spell_id, inner2, behavior, registry);
    if matches!(candidate_req(inner2), TargetRequirement::GraveyardCardOwnedByTargetPlayer) {
        if let crate::actions::Target::Player(pid) = first {
            options.retain(|t| match t {
                crate::actions::Target::Object(id) =>
                    state.get_object(*id).is_some_and(|o| o.owner == *pid),
                crate::actions::Target::Player(_) => false,
                // CR 608.2b: a target that stopped being legal is skipped.
                crate::actions::Target::Illegal => false,
            });
        }
    }
    options.retain(|t| t != first);
    options
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

/// Whether a submitted list of targets is one the requirement actually allows
/// (CR 601.2c).
///
/// `legal_actions` only ever *offers* legal target sets, but nothing re-read
/// the list a caller handed back, and neither client picks a whole offered
/// action — both assemble their own from per-slot choices. So a list the
/// engine would never have produced went straight onto the stack: a creature
/// card in an opponent's graveyard for Unburial Rites, a spell as its own
/// target for Purify the Grave, the same creature twice for Travel
/// Preparations. Each was caught by a different half-measure; this is the
/// question those cards were each asking on their own.
///
/// Slots are positional here, which is what makes this stricter than
/// `stack::is_target_legal`. That one is asked one target at a time at
/// resolution, where which slot a target came from is no longer knowable, so
/// it accepts a `TwoTargets` target that is legal under *either* slot. At cast
/// time the position is the answer, and each slot is checked against its own
/// requirement.
///
/// Both halves of legality, the way `generate_cast_actions_with_targets`
/// applies them: the generic zone/hexproof/filter check and the card's own
/// `is_valid_target`.
pub(crate) fn targets_are_legal(
    state: &GameState,
    target_req: &crate::cards::TargetRequirement,
    targets: &[crate::actions::Target],
    caster: PlayerId,
    source_id: ObjectId,
    behavior: &dyn crate::cards::CardBehavior,
    registry: &CardRegistry,
) -> bool {
    use crate::cards::TargetRequirement as R;
    let one = |t: &crate::actions::Target| {
        crate::stack::is_target_legal(state, t, target_req, caster, Some(source_id), registry)
            && behavior.is_valid_target(state, caster, t, registry)
    };
    match target_req {
        R::None => targets.is_empty(),
        R::TwoTargets(first, second) => {
            let split = targets.len().min(1);
            if !(targets_are_legal(state, first, &targets[..split], caster, source_id, behavior, registry)
                && targets_are_legal(state, second, &targets[split..], caster, source_id, behavior, registry))
            {
                return false;
            }
            // Cross-slot restriction the per-slot checks cannot see: "from
            // THEIR graveyard" ties the second slot's cards to the player
            // named by the first target (issue #46). The per-target
            // `is_target_legal` is asked one target at a time and accepts any
            // graveyard card, so a submitted declaration is re-checked here.
            if matches!(candidate_req(second), R::GraveyardCardOwnedByTargetPlayer) {
                if let Some(crate::actions::Target::Player(pid)) = targets.first() {
                    if !targets[split..].iter().all(|t| match t {
                        crate::actions::Target::Object(id) =>
                            state.get_object(*id).is_some_and(|o| o.owner == *pid),
                        _ => false,
                    }) {
                        return false;
                    }
                }
            }
            true
        }
        R::UpToTargets(max, inner) => {
            targets.len() <= *max
                && targets.iter().all(|t| {
                    crate::stack::is_target_legal(state, t, inner, caster, Some(source_id), registry)
                        && behavior.is_valid_target(state, caster, t, registry)
                })
        }
        // Legal under any one mode, which is what choosing a mode means.
        R::ModalChoice(modes) => modes.iter().any(|mode| {
            targets_are_legal(state, mode, targets, caster, source_id, behavior, registry)
        }),
        // Every other requirement names exactly one target, and a spell that
        // names one cannot be cast without it (CR 601.2c).
        _ => targets.len() == 1 && one(&targets[0]),
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
            let mut v: Vec<Target> = state.players.iter()
                .filter(|p| can_target_player(state, p.id, caster, registry))
                .map(|p| Target::Player(p.id))
                .filter(|t| behavior.is_valid_target(state, caster, t, registry))
                .collect();
            // The chooser reads this list as "You / Opponent". Emitting
            // players in seat order made the order flip with the choosing
            // seat — '0: You' on one card, '0: Opponent' on the next — and
            // muscle memory mis-targeted a trigger (issue #138). The
            // chooser always comes first.
            v.sort_by_key(|t| match t {
                Target::Player(p) if *p == caster => 0,
                _ => 1,
            });
            v
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
        // Two instances of the word "target" name two things, and what this
        // answers is "which things could either of them name" — the union of
        // the two slots. Whether a particular pair is legal is a positional
        // question that `targets_are_legal` answers slot by slot, and how the
        // pairs are enumerated is `generate_cast_actions_with_targets`'s job;
        // both match `TwoTargets` themselves before reaching here.
        //
        // Falling through to the catch-all returned nothing, and the one
        // caller that hands a `TwoTargets` down whole read that as "no legal
        // target": `detect_modal_choice_mode` could never recognise
        // Ghoulcaller's Chant's second mode, so a Chant returning two Zombies
        // recorded itself as the one-card mode (CR 601.2b).
        TargetRequirement::TwoTargets(first, second) => {
            let mut targets = valid_targets_for_req(state, caster, spell_id, first, behavior, registry);
            for t in valid_targets_for_req(state, caster, spell_id, second, behavior, registry) {
                if !targets.contains(&t) {
                    targets.push(t);
                }
            }
            targets
        }
        // "Up to N target X" offers the same candidates as "target X" — CR
        // 601.2c chooses the number of targets first and then the targets
        // themselves out of one pool. This is the only place that knows it.
        // Each of the four callers that needed the number used to peel the
        // wrapper off itself and pass `inner` down, which left this arm
        // unreachable, the knowledge in four places, and one of them wrong:
        // `second_slot_options` matched the requirement *kind* through a
        // wrapper it had already removed on one path and not the other. They
        // now ask `most_targets` / `fewest_targets` for the count and hand the
        // requirement down whole.
        TargetRequirement::UpToTargets(_, inner) => {
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
            // Mirror `generate_cast_actions_with_targets`: the second slot may
            // be "up to N", and its candidates can depend on the chosen first
            // target — so each first option carries its own narrowed list
            // (issue #46: the flat pair of independent lists offered every
            // graveyard card in the game for Memory's Journey's card slot,
            // whichever player was chosen).
            let second_max = most_targets(req2);
            let second_min = fewest_targets(req2);

            let mut first = Vec::new();
            let mut second = Vec::new();
            for t1 in valid_targets_for_req(state, caster, spell_id, req1, behavior, registry) {
                let options = second_slot_options(state, caster, spell_id, req2, &t1, behavior, registry);
                // A first target with no legal second choice is not castable
                // when the second slot is mandatory — don't offer it.
                if options.len() < second_min {
                    continue;
                }
                first.push(t1);
                second.push(options);
            }
            CastTargetSpec::TwoTargets { first, second, second_min, second_max }
        }
        TargetRequirement::UpToTargets(max, _) => {
            let options = valid_targets_for_req(state, caster, spell_id, target_req, behavior, registry);
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
/// Every legal target for a targeted activated ability.
///
/// The same question `valid_targets_for_req` answers for a spell, asked from
/// an ability's source instead of a spell on the stack — CR 602.2b makes
/// choosing targets for an activated ability step 601.2c of casting a spell,
/// so there is one answer, not two. This used to be a second `match` over
/// `TargetRequirement` and the two drifted: the ability copy offered a token
/// in a graveyard for "target card in a graveyard" (CR 109.1 — a token is not
/// a card), knew nine of the sixteen requirements and silently returned no
/// targets for the rest, which is indistinguishable from "the ability has no
/// legal target" and would have removed the ability from the offer entirely.
///
/// `source_id` stands in for the spell: it is what `Another` and
/// `SameNameAsSource` are measured against (CR 602.2a — the ability's
/// controller is the activator, so "opponent" and "you control" are measured
/// from them, not from whoever happens to hold the source).
///
/// Note what is deliberately *not* filtered out: CR 702.6a's equip is "attach
/// this permanent to target creature you control", and nothing in it excludes
/// the creature the Equipment is already attached to. Re-equipping to the same
/// host is the point whenever the equip COST is what you want (Demonmail
/// Hauberk sacrificing a different creature), and with one creature on the
/// battlefield removing it removed the ability.
pub(crate) fn generate_ability_targets(
    state: &GameState,
    source_id: ObjectId,
    ab: &crate::cards::ActivatedAbilityDef,
    controller: PlayerId,
    registry: &CardRegistry,
    behavior: &dyn crate::cards::CardBehavior,
) -> Vec<crate::actions::Target> {
    let Some(target_req) = &ab.target_requirement else { return vec![]; };
    valid_targets_for_req(state, controller, source_id, target_req, behavior, registry)
}
