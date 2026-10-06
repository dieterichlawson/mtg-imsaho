
use crate::cards::CardRegistry;
use crate::events::{GameEvent, DamageTarget};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{CombatState, GameState};
use crate::types::{Keyword, Zone, ContinuousEffect};

/// Set up attackers. Validates and taps them.
/// Creatures with vigilance don't tap when attacking.
pub fn declare_attackers(
    state: &mut GameState,
    attackers: &[(ObjectId, PlayerId)],
    planeswalker_attacks: &[(ObjectId, ObjectId)],
    registry: &CardRegistry,
) {
    let mut combat = CombatState::new();

    // An attacker sent at a planeswalker still has a defending PLAYER — the
    // planeswalker's controller (CR 508.1a) — so it joins the same list, and
    // the walker is remembered separately for the damage step.
    let mut attackers: Vec<(ObjectId, PlayerId)> = attackers.to_vec();
    for &(attacker_id, walker) in planeswalker_attacks {
        let Some(controller) = state.get_object(walker).map(|o| o.controller) else { continue };
        attackers.push((attacker_id, controller));
        combat.planeswalker_defenders.insert(attacker_id, walker);
    }
    let attackers = &attackers[..];

    for &(attacker_id, defending_player) in attackers {
        // Vigilance: don't tap when attacking.
        let has_vigilance = state.has_keyword(attacker_id, Keyword::Vigilance, registry);
        if !has_vigilance {
            state.tap(attacker_id);
        }
        combat.attackers.insert(attacker_id, defending_player);
        combat.blocker_assignments.insert(attacker_id, Vec::new());
        // CR 508.1: this is the moment a creature "attacked this turn". Cards
        // that ask the question later (Homicidal Brute) read the stamp rather
        // than each keeping their own record.
        let turn = state.turn_number;
        if let Some(obj) = state.get_object_mut(attacker_id) {
            obj.attacked_on_turn = Some(turn);
        }
    }

    state.events.push(GameEvent::AttackersDeclared {
        attackers: attackers.to_vec(),
    });

    combat.any_attackers_declared = !combat.attackers.is_empty();
    state.combat = Some(combat);
}


/// Split (blocker, attacker) pairs into the blocks that stand and the blocks
/// CR 509.1b refuses because too few creatures blocked an attacker that
/// can't be blocked by fewer than N of them.
///
/// `min_for` gives one attacker's minimum — `minimum_blockers` against live
/// state here in the engine, the `min_blockers` map the prompt carries for a
/// player implementation.
///
/// The rule lives in one place because the two copies disagreed: handed the
/// same answer, the engine keeps every legal block and drops only the
/// under-minimum pairs, while the LLM seat's own validator discarded the
/// WHOLE declaration and declared no blocks, leaving the seat strictly worse
/// off for having answered (issue #496).
pub fn partition_under_minimum_blocks<F: Fn(ObjectId) -> u32>(
    assignments: &[(ObjectId, ObjectId)],
    min_for: F,
) -> (Vec<(ObjectId, ObjectId)>, Vec<(ObjectId, ObjectId)>) {
    let mut counts: std::collections::HashMap<ObjectId, usize> = std::collections::HashMap::new();
    for &(_, attacker) in assignments {
        *counts.entry(attacker).or_insert(0) += 1;
    }
    assignments.iter().copied().partition(|&(_, attacker)| {
        counts.get(&attacker).copied().unwrap_or(0) >= min_for(attacker) as usize
    })
}

/// Set up blockers. Validates assignments.
pub fn declare_blockers(
    state: &mut GameState,
    assignments: &[(ObjectId, ObjectId)], // (blocker, attacker)
) {
    if let Some(combat) = &mut state.combat {
        for &(blocker_id, attacker_id) in assignments {
            if let Some(blockers) = combat.blocker_assignments.get_mut(&attacker_id) {
                blockers.push(blocker_id);
                // CR 509.2: blocked-ness is permanent for this combat, even
                // if every blocker later leaves combat.
                combat.blocked_attackers.insert(attacker_id);
            }
        }
    }

    state.events.push(GameEvent::BlockersDeclared {
        assignments: assignments.to_vec(),
    });
}

/// Set up blockers with validation. Filters out illegal block assignments
/// (e.g., non-flyer blocking a flyer, or menace with only 1 blocker).
pub fn declare_blockers_with_registry(
    state: &mut GameState,
    assignments: &[(ObjectId, ObjectId)],
    registry: &CardRegistry,
) {
    // Only the defending player's creatures may block, and only creatures
    // that are actually attacking may be blocked (CR 509.1a). In a two-player
    // game the defender is the non-active player.
    let defender = state.opponent(state.active_player);
    let mut valid: Vec<(ObjectId, ObjectId)> = Vec::new();
    for &(blocker, attacker) in assignments {
        // The same pair submitted twice is one block, not two (seen from a
        // pasted line of repeated pairs — issue #50).
        if valid.contains(&(blocker, attacker)) {
            continue;
        }
        // CR 509.1b: a creature can block only one attacker. No card in this
        // set lifts that (there is no "can block an additional creature"
        // effect in the engine), so a blocker already spoken for refuses any
        // further assignment — without this, one blocker blocked two
        // attackers and dealt its full power to each (issue #62).
        if let Some(&(_, first)) = valid.iter().find(|&&(b, _)| b == blocker) {
            state.log(crate::state::LogLevel::Info, format!(
                "ignored illegal block: {} is already blocking {} and can block \
                 only one attacker (CR 509.1b)",
                state.obj_name(blocker), state.obj_name(first)));
            continue;
        }
        let is_attacking = state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&attacker));
        let defenders_creature = state.get_object(blocker).is_some_and(|o| o.controller == defender);
        if is_attacking && defenders_creature && can_block_attacker(state, blocker, attacker, registry) {
            valid.push((blocker, attacker));
        } else {
            // A submitted pair the rules refuse (CR 509.1) is dropped — but
            // never silently: a block that vanishes without a trace cost
            // real games before the log said anything (issue #40). The
            // prompt's `legal_blocks` is the up-front source of truth; this
            // line is the audit trail for anything that slips past it.
            state.log(crate::state::LogLevel::Info, format!(
                "ignored illegal block: {} can't block {}",
                state.obj_name(blocker), state.obj_name(attacker)));
        }
    }

    // Minimum blocker enforcement: for attackers with menace or MinimumBlockers
    // effects, verify they have enough blockers. If not, remove the block
    // assignments for that attacker (can't be blocked by fewer than N creatures).
    let mut blocker_counts: std::collections::HashMap<ObjectId, usize> = std::collections::HashMap::new();
    for &(_, attacker) in &valid {
        *blocker_counts.entry(attacker).or_insert(0) += 1;
    }

    // The same never-silently rule as the per-pairing filter above: a
    // dropped under-minimum block turned "Bears blocks Terror" into
    // "declared no blockers" with no trace, and the defender ate 10 damage
    // without ever learning why (issue #72). The prompt's `min_blockers`
    // is the up-front source of truth; this line is the audit trail.
    let (valid, dropped) = partition_under_minimum_blocks(
        &valid, |attacker| minimum_blockers(state, attacker, registry));
    for (blocker, attacker) in dropped {
        let min = minimum_blockers(state, attacker, registry);
        state.log(crate::state::LogLevel::Info, format!(
            "ignored illegal block: {} blocking {} — it can't be blocked by \
             fewer than {} creatures (CR 509.1b), and only {} blocked it",
            state.obj_name(blocker), state.obj_name(attacker), min,
            blocker_counts.get(&attacker).copied().unwrap_or(0)));
    }

    declare_blockers(state, &valid);
}

/// Announce the damage assignment order for every blocked attacker
/// (CR 509.2), prompting the attacking player once per attacker blocked by
/// two or more creatures.
///
/// This is a turn-based action of the declare blockers step: it happens
/// after blockers are declared and before any player gets priority, and the
/// announced order stands for both combat damage steps (CR 510.4). An
/// attacker blocked by exactly one creature has only one possible order and
/// is recorded without asking.
///
/// Sets `awaiting_action` and returns when a real choice is needed; the
/// answer comes back through `ChooseDamageAssignmentOrder`, which appends
/// the chosen blocker and calls this again.
pub fn announce_damage_assignment_order(state: &mut GameState, registry: &CardRegistry) {
    let attacking_player = state.active_player;
    loop {
        let Some((attacker, remaining)) = next_unordered_attacker(state) else {
            return;
        };
        if remaining.len() == 1 {
            // One blocker left to place: the order is forced.
            place_in_damage_assignment_order(state, attacker, remaining[0]);
            log_completed_order(state, attacker);
            continue;
        }
        let options = damage_order_labels(state, &remaining, registry);
        let placed = state.combat.as_ref()
            .and_then(|c| c.damage_assignment_order.get(&attacker))
            .map_or(0, Vec::len);
        // One sentence, single-spaced: this string is the CLI's takeover
        // screen, the prompt an LLM seat is sent, and the question the
        // stall report names, so a wrapped literal's continuation indent
        // left two 14-space runs mid-sentence on all three (#511).
        //
        // It describes the whole order, because that is what the prompt
        // takes: `ChosenOrder` places every remaining blocker at once
        // (#325), and `ChosenIndex` names the next one alone. The old
        // wording asked for one blocker while the reader under it said
        // "type the numbers in order".
        let description = if placed == 0 {
            format!(
                "Damage assignment order for {} (CR 509.2): put its blockers in the order damage is assigned to them — each one must be assigned lethal damage before any is assigned to the next",
                state.obj_name(attacker),
            )
        } else {
            format!(
                "Damage assignment order for {} (CR 509.2): {placed} already placed; put the rest in the order damage is assigned to them, starting from the {} — each one must be assigned lethal damage before any is assigned to the next",
                state.obj_name(attacker),
                ordinal(placed + 1),
            )
        };
        state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
            player: attacking_player,
            source: attacker,
            choice: crate::state::ResolutionChoiceKind::ChooseDamageAssignmentOrder {
                description,
                attacker,
                remaining,
                options,
            },
        });
        return;
    }
}

/// Append `blocker` to `attacker`'s announced damage assignment order.
pub(crate) fn place_in_damage_assignment_order(
    state: &mut GameState,
    attacker: ObjectId,
    blocker: ObjectId,
) {
    if let Some(combat) = state.combat.as_mut() {
        combat.damage_assignment_order.entry(attacker).or_default().push(blocker);
    }
}

/// The next blocked attacker whose order is incomplete, with the blockers
/// that still have to be placed. Attackers are taken in object-id order so
/// the sequence of prompts is deterministic.
fn next_unordered_attacker(state: &GameState) -> Option<(ObjectId, Vec<ObjectId>)> {
    let combat = state.combat.as_ref()?;
    for attacker in combat.attackers.keys().copied() {
        let blockers = combat.blocker_assignments.get(&attacker)?;
        if blockers.is_empty() {
            continue;
        }
        let placed = combat.damage_assignment_order.get(&attacker);
        let remaining: Vec<ObjectId> = blockers.iter().copied()
            .filter(|b| !placed.is_some_and(|o| o.contains(b)))
            .collect();
        if !remaining.is_empty() {
            return Some((attacker, remaining));
        }
    }
    None
}

/// Log an attacker's order once every blocker has a place in it. Only worth
/// saying when there was a choice to make.
pub(crate) fn log_completed_order(state: &mut GameState, attacker: ObjectId) {
    let order = state.combat.as_ref()
        .and_then(|c| c.damage_assignment_order.get(&attacker))
        .cloned()
        .unwrap_or_default();
    if order.len() < 2 {
        return;
    }
    let names: Vec<String> = order.iter().map(|&b| state.obj_name(b)).collect();
    let attacker_name = state.obj_name(attacker);
    state.log(crate::state::LogLevel::Event, format!(
        "p{} announced the damage assignment order for {}: {} (CR 509.2)",
        state.active_player.0, attacker_name, names.join(", ")));
}

/// Labels for the blockers still to be ordered. Two same-named creatures
/// blocking one attacker are exactly the case where the order matters and
/// the names do not distinguish them, so every repeated label carries the
/// P/T, its marked damage and the object id — the tail the target pickers
/// use.
fn damage_order_labels(
    state: &GameState,
    blockers: &[ObjectId],
    registry: &CardRegistry,
) -> Vec<String> {
    let mut labels: Vec<String> = blockers.iter()
        .map(|&b| {
            let pt = match (state.effective_power(b, registry), state.effective_toughness(b, registry)) {
                (Some(p), Some(t)) => format!(" {p}/{t}"),
                _ => String::new(),
            };
            let name = state.get_object(b).map_or_else(|| "?".to_string(), |o| o.name.clone());
            format!("{name}{pt}")
        })
        .collect();
    let repeated: Vec<bool> = labels.iter()
        .map(|l| labels.iter().filter(|x| *x == l).count() > 1)
        .collect();
    for (i, &b) in blockers.iter().enumerate() {
        if repeated[i] {
            let damage = state.get_object(b).map_or(0, |o| o.damage_marked);
            let dmg = if damage > 0 { format!(", {damage} damage") } else { String::new() };
            labels[i] = format!("{} [#{}{dmg}]", labels[i], b.0);
        }
    }
    labels
}

fn ordinal(n: usize) -> String {
    match n {
        1 => "first".into(),
        2 => "second".into(),
        3 => "third".into(),
        4 => "fourth".into(),
        5 => "fifth".into(),
        _ => format!("{n}th"),
    }
}

/// The minimum number of creatures that must block `att_id` for any block to
/// be legal (CR 509.1b): 1 for most creatures, 2+ under menace
/// (CR 702.111b) or a `MinimumBlockers` continuous effect (e.g. Terror of
/// Kruin Pass). Exposed so `legal_actions` can tell the players about the
/// requirement up front instead of the engine silently discarding an
/// under-minimum declaration (issue #72).
pub fn minimum_blockers(state: &GameState, att_id: ObjectId, registry: &CardRegistry) -> u32 {
    let mut min_req: u32 = 1; // default: any single creature can block
    // Menace keyword: need at least 2 blockers.
    if state.has_keyword(att_id, Keyword::Menace, registry) {
        min_req = min_req.max(2);
    }
    // MinimumBlockers continuous effects (e.g., Terror of Kruin Pass).
    state.walk_effects(
        att_id,
        &|e| matches!(e, ContinuousEffect::MinimumBlockers { .. }),
        registry,
        &mut |e, _| {
            if let ContinuousEffect::MinimumBlockers { count, .. } = e {
                min_req = min_req.max(*count);
            }
            true
        },
    );
    min_req
}

/// Deal combat damage with full keyword support.
/// Handles first strike, trample, deathtouch, and lifelink.
pub fn deal_combat_damage(state: &mut GameState, registry: &CardRegistry) {
    let combat = match &state.combat {
        Some(c) => c.clone(),
        None => return,
    };

    // Check if any creature has first/double strike to determine damage steps.
    let any_first_strike = combat.attackers.keys().chain(
        combat.blocker_assignments.values().flat_map(|v| v.iter())
    ).any(|&id| {
        state.has_keyword(id, Keyword::FirstStrike, registry)
            || state.has_keyword(id, Keyword::DoubleStrike, registry)
    });

    if any_first_strike {
        // First strike damage step: only first/double strikers deal damage.
        deal_damage_step(state, &combat, registry, true);
        // Run SBAs between first strike and normal damage. Creatures that
        // leave combat during this pass (e.g. a blocker that regenerated —
        // CR 701.15c) are skipped in the normal step via the liveness checks
        // in deal_damage_step; blocked-ness persists via blocked_attackers
        // (a blocked attacker stays blocked even if its blockers leave,
        // CR 510.1c).
        //
        // NOTE: this combined entry point runs both damage steps with no
        // priority window and is kept for tests and direct callers. The game
        // loop instead runs the two steps as two Step::CombatDamage
        // instances (CR 510.5) via `combat_damage_step`, with SBAs,
        // triggers, and priority between them — and asks the attacking
        // player how to divide damage among blockers, which this entry
        // point never does: it always makes the default division.
        while crate::sba::check_state_based_actions(state, registry) {}
        // Normal damage step: non-first-strikers + double strikers.
        deal_damage_step(state, &combat, registry, false);
    } else {
        // No first strike: everyone deals damage simultaneously.
        deal_damage_step(state, &combat, registry, false);
    }
}

/// True if any creature in the current combat has first or double strike —
/// i.e. the combat damage step happens twice (CR 510.5).
#[must_use]
pub fn any_first_strike_in_combat(state: &GameState, registry: &CardRegistry) -> bool {
    let Some(combat) = &state.combat else { return false };
    combat.attackers.keys()
        .chain(combat.blocker_assignments.values().flat_map(|v| v.iter()))
        .any(|&id| {
            state.has_keyword(id, Keyword::FirstStrike, registry)
                || state.has_keyword(id, Keyword::DoubleStrike, registry)
        })
}

/// Run one combat damage step for the turn machinery (CR 510.1-510.2,
/// with `first_strike_only` choosing the first-strike step of CR 510.4).
///
/// Before any damage is dealt the attacking player divides each blocked
/// attacker's damage among its blockers (CR 510.1c-d), one blocker at a
/// time, wherever there is a choice to make. This sets `awaiting_action`
/// and returns with nothing dealt when a choice is needed; the answer
/// comes back through `AssignCombatDamage`, which records it and calls this
/// again. With every choice made, the step's damage is dealt.
pub fn combat_damage_step(state: &mut GameState, registry: &CardRegistry, first_strike_only: bool) {
    if ask_next_damage_division(state, registry, first_strike_only) {
        return;
    }
    let Some(combat) = state.combat.clone() else { return };
    deal_damage_step(state, &combat, registry, first_strike_only);
}

/// Fight: each creature deals damage equal to its power to the other.
/// Used by Prey Upon and similar "fight" cards. Fight damage is noncombat
/// damage: protection, deathtouch, lifelink, and noncombat replacement
/// effects apply; combat-only modifiers (Inquisitor's Flail, Moonmist,
/// Ghostly Possession) do not.
pub fn fight(state: &mut GameState, a: ObjectId, b: ObjectId, registry: &CardRegistry) {
    // CR 701.12b: "If one or both creatures instructed to fight are no longer
    // on the battlefield or are no longer creatures, neither of them fights or
    // deals damage." So killing Nightfall Predator in response to its own fight
    // ability spares the target entirely — this used to read the dead
    // creature's printed power off its face and deal damage anyway.
    //
    // The ability itself still resolves (CR 113.7a); it just does nothing.
    let fights = |id: ObjectId| {
        state.get_object(id).is_some_and(|o| o.zone == Zone::Battlefield)
            && state.is_creature(id, registry)
    };
    if !fights(a) || !fights(b) {
        return;
    }

    // Both powers are read before either damage is dealt: a fight is one
    // simultaneous exchange (CR 701.12a), so the second creature's damage is
    // not reduced by the first's.
    //
    // A creature that fights itself deals damage to itself twice, which falls
    // out of `a == b` here rather than needing a case.
    let power_a = u32::try_from(state.effective_power(a, registry).unwrap_or(0).max(0)).unwrap_or(0);
    let power_b = u32::try_from(state.effective_power(b, registry).unwrap_or(0).max(0)).unwrap_or(0);

    // Queued together and dealt together, for the same reason: neither half
    // lands while a choice about the other is open.
    crate::damage::queue_damage(state, a, DamageTarget::Object(b), power_a, crate::damage::DamageKind::NonCombat);
    crate::damage::queue_damage(state, b, DamageTarget::Object(a), power_b, crate::damage::DamageKind::NonCombat);
    crate::damage::process_pending_damage(state, registry);
}

/// Execute one combat damage step.
/// If `first_strike_only`, only creatures with first/double strike deal damage.
/// If not, creatures without first strike deal damage (plus double strikers again).
///
/// All of the step's damage is queued first and dealt together at the end
/// (CR 510.2: combat damage is dealt simultaneously). Where the affected
/// player has a choice to make about one event (CR 616.1), the step waits
/// on it with nothing dealt yet; the answer deals the rest.
fn deal_damage_step(
    state: &mut GameState,
    combat: &CombatState,
    registry: &CardRegistry,
    first_strike_only: bool,
) {
    queue_damage_step(state, combat, registry, first_strike_only);
    crate::damage::process_pending_damage(state, registry);
}

/// Queue every event of one combat damage step, in attacker order.
fn queue_damage_step(
    state: &mut GameState,
    combat: &CombatState,
    registry: &CardRegistry,
    first_strike_only: bool,
) {
    for (&attacker_id, &defending_player) in &combat.attackers {
        if state.get_object(attacker_id).is_none_or(|o| o.zone != Zone::Battlefield) {
            continue;
        }
        // An attacker removed from combat since the snapshot (e.g. it
        // regenerated between damage steps) neither deals nor receives
        // combat damage (CR 506.4c).
        if state.combat.as_ref().is_some_and(|c| !c.attackers.contains_key(&attacker_id)) {
            continue;
        }

        let attacker_deals = deals_in_step(state, attacker_id, first_strike_only, registry);
        if attacker_deals && first_strike_only {
            // CR 510.5: record membership so the regular step knows this
            // creature already dealt its damage (unless double strike).
            if let Some(c) = state.combat.as_mut() {
                c.dealt_first_strike.insert(attacker_id);
            }
        }

        let attacker_power = if attacker_deals {
            u32::try_from(state.effective_power(attacker_id, registry).unwrap_or(0).max(0)).unwrap_or(0)
        } else {
            0
        };

        let has_trample = state.has_keyword(attacker_id, Keyword::Trample, registry);
        let blockers = assignment_order(combat, attacker_id);

        let was_blocked = combat.blocked_attackers.contains(&attacker_id)
            || state.combat.as_ref().is_some_and(|c| c.blocked_attackers.contains(&attacker_id));
        // An attacker sent at a planeswalker deals its unblocked/overflow
        // damage to the walker instead of the player. If the walker is no
        // longer on the battlefield under the defending player, there is
        // nothing being attacked and that damage simply is not dealt
        // (CR 510.1c) — it does NOT fall through to the player (the 2018
        // removal of the redirect rule).
        let attacked_walker = combat.planeswalker_defenders.get(&attacker_id).copied();
        let walker_still_there = attacked_walker.is_some_and(|w|
            state.get_object(w).is_some_and(|o|
                o.zone == Zone::Battlefield && o.controller == defending_player));

        if blockers.is_empty() && !was_blocked {
            // Unblocked: deal damage to what it attacks.
            if attacker_power > 0 {
                match attacked_walker {
                    // All of it lands on the walker, trample or not. A
                    // creature attacking a planeswalker assigns its combat
                    // damage among that walker and its blockers and nowhere
                    // else (CR 510.1a); trample carries excess past
                    // *blockers*, not past what is being attacked (CR
                    // 702.19b). Damage beyond a walker's loyalty is simply
                    // not dealt anywhere — it used to spill onto the
                    // defending player, which no version of trample has ever
                    // done (issue #246).
                    Some(walker) if walker_still_there => {
                        queue_combat_damage_to_creature(
                            state, attacker_id, walker, attacker_power);
                    }
                    Some(_) => {} // attacked walker is gone: no combat damage
                    None => queue_combat_damage_to_player(
                        state, attacker_id, defending_player, attacker_power),
                }
            }
        } else {
            // Blocked: the blockers deal their damage to the attacker, and
            // the attacker's is divided among them as the attacking player
            // chose (CR 510.1c-d), with any trample overflow beyond them.
            let live = live_blockers(state, attacker_id, &blockers);
            let division = divide_combat_damage(
                attacker_power,
                &with_lethal(state, attacker_id, &live, registry),
                has_trample,
                combat.chosen_damage.get(&attacker_id).map_or(&[], Vec::as_slice),
            );

            for (&blocker_id, &(_, assigned)) in live.iter().zip(&division.to_blockers) {
                // Blocker deals damage to attacker.
                let blocker_deals = deals_in_step(state, blocker_id, first_strike_only, registry);
                if blocker_deals && first_strike_only {
                    if let Some(c) = state.combat.as_mut() {
                        c.dealt_first_strike.insert(blocker_id);
                    }
                }
                if blocker_deals {
                    let blocker_power = u32::try_from(state.effective_power(blocker_id, registry).unwrap_or(0).max(0)).unwrap_or(0);
                    if blocker_power > 0 {
                        queue_combat_damage_to_creature(state, blocker_id, attacker_id, blocker_power);
                    }
                }

                // Attacker deals damage to blocker.
                if assigned > 0 {
                    queue_combat_damage_to_creature(state, attacker_id, blocker_id, assigned);
                }
            }

            // Trample: once every blocker has been assigned lethal damage,
            // the rest goes to what is being attacked — the defending
            // player, or the attacked planeswalker, and never both
            // (CR 702.19b).
            if division.overflow > 0 {
                match attacked_walker {
                    Some(walker) if walker_still_there => {
                        queue_combat_damage_to_creature(
                            state, attacker_id, walker, division.overflow);
                    }
                    Some(_) => {} // attacked walker is gone: overflow lands nowhere
                    None => queue_combat_damage_to_player(
                        state, attacker_id, defending_player, division.overflow),
                }
            }
        }
    }
    // The step's choices are spent: the regular step after a first-strike
    // step asks again, against the damage the first one marked (CR 510.4).
    if let Some(c) = state.combat.as_mut() {
        c.chosen_damage.clear();
    }
}

/// Whether `id` deals combat damage in this step (CR 510.4-510.5): in the
/// first-strike step, creatures with first or double strike; in the
/// regular step, double strikers and every creature that dealt none in the
/// first-strike step.
fn deals_in_step(state: &GameState, id: ObjectId, first_strike_only: bool, registry: &CardRegistry) -> bool {
    let double_strike = state.has_keyword(id, Keyword::DoubleStrike, registry);
    if first_strike_only {
        double_strike || state.has_keyword(id, Keyword::FirstStrike, registry)
    } else {
        double_strike
            || !state.combat.as_ref().is_some_and(|c| c.dealt_first_strike.contains(&id))
    }
}

/// An attacker's blockers in damage assignment order.
///
/// CR 510.1c: damage is assigned to the blockers in the damage assignment
/// order the attacking player announced in the declare blockers step
/// (CR 509.2) — not in the order the blocks happened to be declared in.
/// Both damage steps use the same announced order (CR 510.4). Anything the
/// announcement did not cover (a state saved before it existed) keeps
/// declaration order, and any blocker missing from the order is assigned
/// last rather than dropped.
fn assignment_order(combat: &CombatState, attacker: ObjectId) -> Vec<ObjectId> {
    let declared = combat.blocker_assignments.get(&attacker).cloned().unwrap_or_default();
    let mut blockers: Vec<ObjectId> = combat.damage_assignment_order.get(&attacker)
        .map(|order| order.iter().copied().filter(|b| declared.contains(b)).collect())
        .unwrap_or_default();
    let unannounced: Vec<ObjectId> = declared.iter().copied()
        .filter(|b| !blockers.contains(b))
        .collect();
    blockers.extend(unannounced);
    blockers
}

/// The blockers that are still in this combat: on the battlefield and
/// still blocking `attacker`. One removed since blockers were declared
/// (it regenerated between damage steps, or left the battlefield) neither
/// deals nor is assigned combat damage (CR 506.4c); the attacker remains
/// blocked (CR 510.1c), and its damage is divided among the rest.
fn live_blockers(state: &GameState, attacker: ObjectId, blockers: &[ObjectId]) -> Vec<ObjectId> {
    blockers.iter().copied()
        .filter(|&b| state.get_object(b).is_some_and(|o| o.zone == Zone::Battlefield))
        .filter(|&b| state.combat.as_ref().is_some_and(|c|
            c.blocker_assignments.get(&attacker).is_some_and(|v| v.contains(&b))))
        .collect()
}

/// Each blocker with the lethal damage it must be assigned (CR 510.1c):
/// its toughness less the damage already marked on it, or 1 from a source
/// with deathtouch (CR 702.2c).
fn with_lethal(state: &GameState, attacker: ObjectId, blockers: &[ObjectId], registry: &CardRegistry) -> Vec<(ObjectId, u32)> {
    let deathtouch = state.has_keyword(attacker, Keyword::Deathtouch, registry);
    blockers.iter().map(|&b| {
        let lethal = if deathtouch {
            1
        } else {
            let toughness = state.effective_toughness(b, registry).unwrap_or(0);
            let marked = state.get_object(b).map_or(0, |o| o.damage_marked);
            u32::try_from((toughness - i32::try_from(marked).unwrap_or(i32::MAX)).max(0)).unwrap_or(0)
        };
        (b, lethal)
    }).collect()
}

/// One attacker's combat damage divided among its blockers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DamageDivision {
    /// Each blocker, in damage assignment order, with the damage assigned
    /// to it.
    pub to_blockers: Vec<(ObjectId, u32)>,
    /// What tramples over to the player or planeswalker being attacked.
    /// Always 0 without trample.
    pub overflow: u32,
    /// The first blocker whose amount is still the attacking player's to
    /// choose, with the least and most it may be assigned. `to_blockers`
    /// gives it, and every blocker after it, the default.
    pub open: Option<(ObjectId, u32, u32)>,
}

/// Divide `power` among `blockers` — each paired with its lethal damage, in
/// damage assignment order — as CR 510.1c-d allows, taking the attacking
/// player's `chosen` amounts where they have made them.
///
/// Every blocker is assigned at least lethal damage before the next is
/// assigned any; one may be assigned more, up to everything left. The last
/// blocker of an attacker without trample takes everything left; with
/// trample, everything left after the last blocker is overflow.
///
/// The player is asked only when there is a choice: the power exceeds the
/// total lethal damage, and there is somewhere else for a surplus to go —
/// a second blocker, or trample. Otherwise, and for any blocker not yet
/// chosen, each blocker gets exactly lethal and the rest goes forward.
#[must_use]
pub fn divide_combat_damage(
    power: u32,
    blockers: &[(ObjectId, u32)],
    trample: bool,
    chosen: &[(ObjectId, u32)],
) -> DamageDivision {
    let total_lethal = blockers.iter().fold(0u32, |sum, &(_, l)| sum.saturating_add(l));
    let choice = power > total_lethal && (trample || blockers.len() >= 2);
    let mut remaining = power;
    let mut open = None;
    let mut to_blockers = Vec::with_capacity(blockers.len());
    for (i, &(blocker, lethal)) in blockers.iter().enumerate() {
        let least = remaining.min(lethal);
        let amount = if i + 1 == blockers.len() && !trample {
            remaining
        } else if !choice || remaining == least {
            least
        } else if let Some(&(_, a)) = chosen.iter().find(|&&(b, _)| b == blocker) {
            a.clamp(least, remaining)
        } else {
            open.get_or_insert((blocker, least, remaining));
            least
        };
        to_blockers.push((blocker, amount));
        remaining -= amount;
    }
    DamageDivision {
        to_blockers,
        overflow: if trample { remaining } else { 0 },
        open,
    }
}

/// Ask the attacking player the next open division of this step's combat
/// damage, if there is one (CR 510.1c-d). True when a prompt was raised.
fn ask_next_damage_division(state: &mut GameState, registry: &CardRegistry, first_strike_only: bool) -> bool {
    let Some(combat) = state.combat.clone() else { return false };
    for (&attacker, &defending_player) in &combat.attackers {
        if state.get_object(attacker).is_none_or(|o| o.zone != Zone::Battlefield)
            || !deals_in_step(state, attacker, first_strike_only, registry)
        {
            continue;
        }
        let live = live_blockers(state, attacker, &assignment_order(&combat, attacker));
        if live.is_empty() {
            continue;
        }
        let power = u32::try_from(state.effective_power(attacker, registry).unwrap_or(0).max(0)).unwrap_or(0);
        let trample = state.has_keyword(attacker, Keyword::Trample, registry);
        let chosen = combat.chosen_damage.get(&attacker).map_or(&[][..], Vec::as_slice);
        let division = divide_combat_damage(power, &with_lethal(state, attacker, &live, registry), trample, chosen);
        let Some((blocker, min, max)) = division.open else { continue };

        // What a surplus left here goes on to: the next blocker, or what
        // the trampler is attacking.
        let position = live.iter().position(|&b| b == blocker).unwrap_or(0);
        let onward = match live.get(position + 1) {
            Some(&next) => state.obj_name(next),
            None => match combat.planeswalker_defenders.get(&attacker) {
                Some(&walker) => state.obj_name(walker),
                None => format!("p{}", defending_player.0),
            },
        };
        let blocker_name = state.obj_name(blocker);
        let options: Vec<String> = (min..=max).map(|a| {
            let note = if a == min { " (lethal)" } else { "" };
            format!("{a} to {blocker_name}{note}, {} on to {onward}", max - a)
        }).collect();
        let description = format!(
            "Combat damage from {} ({power} power, CR 510.1c): how much of the {max} left goes to {blocker_name}? At least {min} (lethal), at most {max}; the rest goes on to {onward}",
            state.obj_name(attacker),
        );
        state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
            player: state.active_player,
            source: attacker,
            choice: crate::state::ResolutionChoiceKind::AssignCombatDamage {
                description, attacker, blocker, min, max, options, first_strike_only,
            },
        });
        return true;
    }
    false
}

/// Record the attacking player's answer at an `AssignCombatDamage` prompt.
pub(crate) fn record_damage_assignment(state: &mut GameState, attacker: ObjectId, blocker: ObjectId, amount: u32) {
    if let Some(c) = state.combat.as_mut() {
        c.chosen_damage.entry(attacker).or_default().push((blocker, amount));
    }
}

/// Get all subtypes of a creature (from both card data and object-level subtypes).
/// Transform-aware: uses back-face data for transformed DFCs.
#[must_use]
pub fn get_subtypes(state: &GameState, creature_id: ObjectId, registry: &CardRegistry) -> Vec<String> {
    state.subtypes_of(creature_id, registry)
}

/// Queue combat damage from a source creature to a target creature.
fn queue_combat_damage_to_creature(
    state: &mut GameState,
    source: ObjectId,
    target: ObjectId,
    amount: u32,
) {
    crate::damage::queue_damage(state, source, DamageTarget::Object(target), amount, crate::damage::DamageKind::Combat);
}

/// Queue combat damage from a source creature to a player.
fn queue_combat_damage_to_player(
    state: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    amount: u32,
) {
    crate::damage::queue_damage(state, source, DamageTarget::Player(player), amount, crate::damage::DamageKind::Combat);
}

/// Clean up combat state at end of combat. Any delayed triggered abilities
/// scheduled for end of combat (e.g. Geist of Saint Traft's Angel token exile)
/// are drained onto the stack separately by `triggers::collect_triggers` when
/// it processes the `StepStarted { EndCombat }` event — they must go through
/// the stack with priority windows, not be applied as turn-based actions.
pub fn end_combat(state: &mut GameState, _registry: &crate::cards::CardRegistry) {
    state.combat = None;
    // Defensive: never carry a pending second combat damage step out of combat.
    state.combat_damage_step_pending = false;
}

/// Get all creatures a player controls that are eligible to attack.
/// Checks keywords (defender, haste) and continuous effects (Pacifism).
#[must_use]
pub fn eligible_attackers(state: &GameState, player: PlayerId, registry: &CardRegistry) -> Vec<ObjectId> {
    state.objects_in_id_order().into_iter()
        .filter(|o| {
            o.zone == Zone::Battlefield
                && o.controller == player
                && state.is_creature(o.id, registry)
                && !o.tapped
                // CR 302.6, haste and all: one definition, in the engine.
                && !state.has_summoning_sickness(o.id, registry)
                // Defender can't attack.
                && !state.has_keyword(o.id, Keyword::Defender, registry)
                // Check aura-based restrictions (Pacifism).
                && state.can_attack(o.id, registry)
        })
        .map(|o| o.id)
        .collect()
}

/// Get all creatures a player controls that are eligible to block.
/// Checks continuous effects (Pacifism, can't block, etc.).
#[must_use]
pub fn eligible_blockers(state: &GameState, player: PlayerId, registry: &CardRegistry) -> Vec<ObjectId> {
    state.objects_in_id_order().into_iter()
        .filter(|o| {
            o.zone == Zone::Battlefield
                && o.controller == player
                && state.is_creature(o.id, registry)
                && !o.tapped
        })
        .map(|o| o.id)
        .collect::<Vec<_>>()
        .into_iter()
        .filter(|&id| can_block_at_all(state, id, registry))
        .collect()
}

/// Whether `blocker_id` may block *anything* this combat — the restrictions
/// that do not depend on which attacker is being blocked (CR 509.1a/509.1b).
///
/// This is the half of blocking legality that `eligible_blockers` used to own
/// alone. `can_block_attacker` did not ask it, so the two paths disagreed:
/// a Vampire Interloper ("can't block") was filtered out of the prompt but
/// accepted by `declare_blockers_with_registry`, which validates through
/// `can_block_attacker`. Both now go through here.
#[must_use]
pub fn can_block_at_all(state: &GameState, blocker_id: ObjectId, registry: &CardRegistry) -> bool {
    // A blocker must be an untapped creature on the battlefield (CR 509.1a).
    let Some(blocker) = state.get_object(blocker_id) else { return false };
    if blocker.zone != Zone::Battlefield || blocker.tapped || !state.is_creature(blocker_id, registry) {
        return false;
    }
    // `can_block` is the whole "can't block" question — a static ability
    // (Vampire Interloper, Bonds of Faith) or a "can't block this turn" effect
    // (Nightbird's Clutches, Crossway Vampire).
    state.can_block(blocker_id, registry)
}

/// Check if a blocker can legally block a specific attacker: the blanket
/// restrictions of [`can_block_at_all`], plus evasion (flying, intimidate,
/// protection) evaluated against this particular attacker.
///
/// Whether the attacker is actually attacking is enforced by the caller with
/// combat context (`declare_blockers_with_registry`).
#[must_use]
pub fn can_block_attacker(state: &GameState, blocker_id: ObjectId, attacker_id: ObjectId, registry: &CardRegistry) -> bool {
    AttackerEvasion::of(state, attacker_id, registry)
        .admits(state, &BlockerReach::of(state, blocker_id, registry), registry)
}

/// Every attacker each blocker may block, for the declare-blockers prompt.
///
/// Each attacker's evasion and each blocker's reach are read off the board
/// once, and only the pair-dependent part — a filter or protection matched
/// against this blocker — is evaluated per pair. Calling
/// [`can_block_attacker`] per pair walked the battlefield's effects several
/// times for every (blocker, attacker), O(blockers × attackers × board):
/// 27 s for one decision at 2,000 permanents, in every seat (#641).
#[must_use]
pub fn legal_blocks(
    state: &GameState,
    blockers: &[ObjectId],
    attackers: &[ObjectId],
    registry: &CardRegistry,
) -> std::collections::HashMap<ObjectId, Vec<ObjectId>> {
    let evasions: Vec<AttackerEvasion> = attackers.iter()
        .map(|&a| AttackerEvasion::of(state, a, registry))
        .collect();
    blockers.iter().map(|&b| {
        let reach = BlockerReach::of(state, b, registry);
        let can: Vec<ObjectId> = evasions.iter()
            .filter(|e| e.admits(state, &reach, registry))
            .map(|e| e.id)
            .collect();
        (b, can)
    }).collect()
}

/// What an attacker's evasion asks of any blocker.
struct AttackerEvasion {
    id: ObjectId,
    flying: bool,
    intimidate: bool,
    palette: Vec<crate::types::Color>,
    unblockable: bool,
    /// `CanOnlyBeBlockedBy` filters, with the source each is read against.
    only_by: Vec<(crate::types::CreatureFilter, ObjectId, PlayerId)>,
    protections: crate::state::Protections,
}

/// What a blocker brings to any attacker.
struct BlockerReach {
    id: ObjectId,
    able: bool,
    flying_or_reach: bool,
    artifact: bool,
    palette: Vec<crate::types::Color>,
}

impl BlockerReach {
    fn of(state: &GameState, id: ObjectId, registry: &CardRegistry) -> Self {
        let able = can_block_at_all(state, id, registry);
        BlockerReach {
            id,
            able,
            flying_or_reach: able && (state.has_keyword(id, Keyword::Flying, registry)
                || state.has_keyword(id, Keyword::Reach, registry)),
            artifact: able && state.has_card_type(id, crate::types::CardType::Artifact, registry),
            palette: if able { state.colors_of(id, registry) } else { Vec::new() },
        }
    }
}

impl AttackerEvasion {
    fn of(state: &GameState, id: ObjectId, registry: &CardRegistry) -> Self {
        let intimidate = state.has_keyword(id, Keyword::Intimidate, registry);
        let mut only_by = Vec::new();
        // Block restriction (e.g., Orchard Spirit: only flying/reach can
        // block). The filter is read against the effect source's controller,
        // which is why this keeps the source and not just the effect.
        state.walk_effects(
            id,
            &|e| matches!(e, ContinuousEffect::CanOnlyBeBlockedBy { .. }),
            registry,
            &mut |e, source| {
                if let ContinuousEffect::CanOnlyBeBlockedBy { allowed_blockers, .. } = e {
                    only_by.push((allowed_blockers.clone(), source.id, source.controller));
                }
                true
            },
        );
        AttackerEvasion {
            id,
            flying: state.has_keyword(id, Keyword::Flying, registry),
            intimidate,
            palette: if intimidate { state.colors_of(id, registry) } else { Vec::new() },
            // "Can't be blocked" (e.g., Invisible Stalker).
            unblockable: state.cant_be_blocked(id, registry),
            only_by,
            protections: state.protection_set(id, registry),
        }
    }

    fn admits(&self, state: &GameState, blocker: &BlockerReach, registry: &CardRegistry) -> bool {
        if !blocker.able || self.unblockable {
            return false;
        }
        // Flying: can only be blocked by creatures with flying or reach.
        if self.flying && !blocker.flying_or_reach {
            return false;
        }
        // Intimidate: can only be blocked by artifact creatures or creatures
        // that share a color.
        if self.intimidate && !blocker.artifact
            && !self.palette.iter().any(|c| blocker.palette.contains(c))
        {
            return false;
        }
        // Menace: must be blocked by two or more creatures (handled at
        // validation, not per-blocker).
        if self.only_by.iter().any(|(filter, source, controller)|
            !state.matches_filter(blocker.id, filter, *source, *controller, registry))
        {
            return false;
        }
        // Protection: a creature with protection from X can't be BLOCKED BY
        // X. Only the ATTACKER's protection from the blocker prevents it; a
        // BLOCKER with protection from the attacker may still block.
        !self.protections.covers(state, blocker.id, registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::CardRegistry;
    use crate::ids::CardId;

    #[test]
    fn unblocked_attacker_deals_damage() {
        let registry = CardRegistry::with_all_cards();
        let mut state = GameState::new(2);
        let attacker = state.create_object(
            CardId(1), PlayerId(0), Zone::Battlefield, Some(3), Some(3),
        );
        state.get_object_mut(attacker).unwrap().summoning_sick = false;

        let defending = PlayerId(1);
        declare_attackers(&mut state, &[(attacker, defending)], &[], &registry);
        declare_blockers(&mut state, &[]);
        deal_combat_damage(&mut state, &registry);

        assert_eq!(state.get_player(defending).life, 40 - 3);
    }

    #[test]
    fn blocked_creature_trades() {
        let registry = CardRegistry::with_all_cards();
        let mut state = GameState::new(2);
        let attacker = state.create_object(
            CardId(1), PlayerId(0), Zone::Battlefield, Some(2), Some(2),
        );
        state.get_object_mut(attacker).unwrap().summoning_sick = false;

        let blocker = state.create_object(
            CardId(2), PlayerId(1), Zone::Battlefield, Some(2), Some(2),
        );

        let defending = PlayerId(1);
        declare_attackers(&mut state, &[(attacker, defending)], &[], &registry);
        declare_blockers(&mut state, &[(blocker, attacker)]);
        deal_combat_damage(&mut state, &registry);

        // Both should have lethal damage marked.
        assert_eq!(state.get_object(attacker).unwrap().damage_marked, 2);
        assert_eq!(state.get_object(blocker).unwrap().damage_marked, 2);
        // Defending player takes no damage (attacker was blocked).
        assert_eq!(state.get_player(defending).life, 40);
    }

    #[test]
    fn non_flyer_cannot_block_flyer() {
        let registry = CardRegistry::with_all_cards();
        let mut state = GameState::new(2);
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);

        // Vampire Interloper (2/1, flying) attacks — use registry card_id,
        // do NOT set keywords on object (has_keyword reads from registry for registered cards).
        let vi_id = registry.get_id_by_name("Vampire Interloper").unwrap();
        let attacker = state.create_object(vi_id, p0, Zone::Battlefield, Some(2), Some(1));
        state.get_object_mut(attacker).unwrap().summoning_sick = false;

        // Verify the registry knows Vampire Interloper has flying
        assert!(state.has_keyword(attacker, Keyword::Flying, &registry),
            "Vampire Interloper should have flying via registry");

        // Geist-Honored Monk (0/0, vigilance, no flying) tries to block
        let ghm_id = registry.get_id_by_name("Geist-Honored Monk").unwrap();
        let blocker = state.create_object(ghm_id, p1, Zone::Battlefield, Some(3), Some(3));

        // Verify Geist-Honored Monk does NOT have flying
        assert!(!state.has_keyword(blocker, Keyword::Flying, &registry),
            "Geist-Honored Monk should not have flying");

        // The block should be illegal
        assert!(!can_block_attacker(&state, blocker, attacker, &registry),
            "Non-flyer Geist-Honored Monk should not be able to block flying Vampire Interloper");

        // Verify the block gets filtered out by declare_blockers_with_registry
        declare_attackers(&mut state, &[(attacker, p1)], &[], &registry);
        declare_blockers_with_registry(&mut state, &[(blocker, attacker)], &registry);

        // Attacker should be unblocked — damage goes to player
        deal_combat_damage(&mut state, &registry);
        assert_eq!(state.get_player(p1).life, 40 - 2,
            "Vampire Interloper should deal damage to player since block was illegal");
    }

    #[test]
    fn eligible_attackers_excludes_sick_and_tapped() {
        let mut state = GameState::new(2);
        let p0 = PlayerId(0);

        // Ready to attack.
        let a = state.create_object(CardId(1), p0, Zone::Battlefield, Some(2), Some(2));
        state.get_object_mut(a).unwrap().summoning_sick = false;

        // Summoning sick — can't attack.
        state.create_object(CardId(1), p0, Zone::Battlefield, Some(2), Some(2));

        // Tapped — can't attack.
        let c = state.create_object(CardId(1), p0, Zone::Battlefield, Some(2), Some(2));
        state.get_object_mut(c).unwrap().summoning_sick = false;
        state.get_object_mut(c).unwrap().tapped = true;

        let registry = CardRegistry::with_all_cards();
        let eligible = eligible_attackers(&state, p0, &registry);
        assert_eq!(eligible.len(), 1);
        assert_eq!(eligible[0], a);
    }

    /// Building the declare-blockers offer walks the battlefield's effects a
    /// bounded number of times per attacker and per blocker, not per pair:
    /// per pair it was O(blockers × attackers × board) and took 27 s for
    /// one decision at 2,000 permanents (#641).
    #[test]
    fn the_blocks_offer_walks_the_board_per_creature_not_per_pair() {
        let registry = CardRegistry::with_all_cards();
        let mut state = GameState::new(2);
        let make = |state: &mut GameState, p: u8| {
            let id = state.create_object(CardId(1), PlayerId(p), Zone::Battlefield, Some(2), Some(2));
            let o = state.get_object_mut(id).unwrap();
            o.summoning_sick = false;
            o.card_types = vec![crate::types::CardType::Creature];
            id
        };
        let attackers: Vec<_> = (0..30).map(|_| make(&mut state, 0)).collect();
        let blockers: Vec<_> = (0..30).map(|_| make(&mut state, 1)).collect();

        let before = crate::state::EFFECT_WALKS.with(std::cell::Cell::get);
        let blocks = legal_blocks(&state, &blockers, &attackers, &registry);
        let walks = crate::state::EFFECT_WALKS.with(std::cell::Cell::get) - before;
        assert!(blocks.values().all(|v| v.len() == attackers.len()), "every blocker may block every attacker");
        assert!(walks <= 20 * (attackers.len() + blockers.len()) as u64,
            "{walks} effect walks for {} attackers x {} blockers — per pair, not per creature",
            attackers.len(), blockers.len());
        // And it is the same answer the per-pair question gives.
        for (&b, can) in &blocks {
            for &a in &attackers {
                assert_eq!(can.contains(&a), can_block_attacker(&state, b, a, &registry));
            }
        }
    }
}
