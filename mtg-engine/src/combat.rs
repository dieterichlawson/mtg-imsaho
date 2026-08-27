
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
    registry: &CardRegistry,
) {
    let mut combat = CombatState::new();

    for &(attacker_id, defending_player) in attackers {
        // Vigilance: don't tap when attacking.
        let has_vigilance = state.has_keyword(attacker_id, Keyword::Vigilance, registry);
        if !has_vigilance {
            if let Some(obj) = state.get_object_mut(attacker_id) {
                obj.tapped = true;
                state.events.push(GameEvent::Tapped { object: attacker_id });
            }
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

    state.combat = Some(combat);
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
    let valid: Vec<_> = assignments.iter()
        .filter(|&&(_, attacker)| state.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&attacker)))
        .filter(|&&(blocker, _)| state.get_object(blocker).is_some_and(|o| o.controller == defender))
        .filter(|&&(blocker, attacker)| can_block_attacker(state, blocker, attacker, registry))
        .copied()
        .collect();

    // Minimum blocker enforcement: for attackers with menace or MinimumBlockers
    // effects, verify they have enough blockers. If not, remove the block
    // assignments for that attacker (can't be blocked by fewer than N creatures).
    let mut blocker_counts: std::collections::HashMap<ObjectId, usize> = std::collections::HashMap::new();
    for &(_, attacker) in &valid {
        *blocker_counts.entry(attacker).or_insert(0) += 1;
    }

    // Determine minimum blocker requirement for each attacker.
    let attacker_ids: Vec<ObjectId> = blocker_counts.keys().copied().collect();
    let mut min_blockers: std::collections::HashMap<ObjectId, u32> = std::collections::HashMap::new();
    for &att_id in &attacker_ids {
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
        if min_req > 1 {
            min_blockers.insert(att_id, min_req);
        }
    }

    let valid: Vec<_> = valid.into_iter()
        .filter(|&(_, attacker)| {
            if let Some(&min) = min_blockers.get(&attacker) {
                blocker_counts.get(&attacker).copied().unwrap_or(0) >= min as usize
            } else {
                true
            }
        })
        .collect();

    declare_blockers(state, &valid);
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
        // instances (CR 510.5) via deal_first_strike_damage_pass /
        // deal_regular_damage_pass, with SBAs, triggers, and priority
        // between them.
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

/// Deal the FIRST-STRIKE combat damage step's damage (CR 510.5). Used by the
/// turn machinery, which then gives players a full SBA/trigger/priority
/// round before the regular combat damage step.
pub fn deal_first_strike_damage_pass(state: &mut GameState, registry: &CardRegistry) {
    let Some(combat) = state.combat.clone() else { return };
    deal_damage_step(state, &combat, registry, true);
}

/// Deal the REGULAR combat damage step's damage: creatures that didn't deal
/// first-strike damage, plus double strikers.
pub fn deal_regular_damage_pass(state: &mut GameState, registry: &CardRegistry) {
    let Some(combat) = state.combat.clone() else { return };
    deal_damage_step(state, &combat, registry, false);
}

/// Fight: each creature deals damage equal to its power to the other.
/// Used by Prey Upon and similar "fight" cards. Fight damage is noncombat
/// damage: protection, deathtouch, lifelink, and noncombat replacement
/// effects apply; combat-only modifiers (Inquisitor's Flail, Moonmist,
/// Ghostly Possession) do not.
pub fn fight(state: &mut GameState, a: ObjectId, b: ObjectId, registry: &CardRegistry) {
    let power_a = u32::try_from(state.effective_power(a, registry).unwrap_or(0).max(0)).unwrap_or(0);
    let power_b = u32::try_from(state.effective_power(b, registry).unwrap_or(0).max(0)).unwrap_or(0);

    crate::damage::deal_damage(state, a, DamageTarget::Object(b), power_a, crate::damage::DamageKind::NonCombat, registry);
    crate::damage::deal_damage(state, b, DamageTarget::Object(a), power_b, crate::damage::DamageKind::NonCombat, registry);
}

/// Execute one combat damage step.
/// If `first_strike_only`, only creatures with first/double strike deal damage.
/// If not, creatures without first strike deal damage (plus double strikers again).
fn deal_damage_step(
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

        let has_first_strike = state.has_keyword(attacker_id, Keyword::FirstStrike, registry);
        let has_double_strike = state.has_keyword(attacker_id, Keyword::DoubleStrike, registry);
        let attacker_deals = if first_strike_only {
            let deals = has_first_strike || has_double_strike;
            if deals {
                // CR 510.5: record membership so the regular step knows this
                // creature already dealt its damage (unless double strike).
                if let Some(c) = state.combat.as_mut() {
                    c.dealt_first_strike.insert(attacker_id);
                }
            }
            deals
        } else {
            // Regular step: creatures that didn't deal first-strike damage,
            // plus double strikers (CR 510.5).
            has_double_strike
                || !state.combat.as_ref().is_some_and(|c| c.dealt_first_strike.contains(&attacker_id))
        };

        let attacker_power = if attacker_deals {
            u32::try_from(state.effective_power(attacker_id, registry).unwrap_or(0).max(0)).unwrap_or(0)
        } else {
            0
        };

        let has_trample = state.has_keyword(attacker_id, Keyword::Trample, registry);
        let has_deathtouch_attacker = state.has_keyword(attacker_id, Keyword::Deathtouch, registry);

        let blockers = combat.blocker_assignments.get(&attacker_id)
            .cloned()
            .unwrap_or_default();

        let was_blocked = combat.blocked_attackers.contains(&attacker_id)
            || state.combat.as_ref().is_some_and(|c| c.blocked_attackers.contains(&attacker_id));
        if blockers.is_empty() && !was_blocked {
            // Unblocked: deal damage to defending player.
            if attacker_power > 0 {
                deal_damage_to_player(state, attacker_id, defending_player, attacker_power, registry);
            }
        } else {
            // Blocked: distribute damage to blockers, with trample overflow.
            let mut remaining_power = attacker_power;

            let blocker_count = blockers.len();
            for (idx, &blocker_id) in blockers.iter().enumerate() {
                if state.get_object(blocker_id).is_none_or(|o| o.zone != Zone::Battlefield) {
                    continue;
                }
                // A blocker removed from combat since the snapshot (e.g. it
                // regenerated between damage steps) neither deals nor
                // receives combat damage (CR 506.4c). The attacker remains
                // blocked (CR 510.1c) — handled by the snapshot itself.
                if state.combat.as_ref().is_some_and(|c|
                    !c.blocker_assignments.get(&attacker_id).is_some_and(|v| v.contains(&blocker_id))) {
                    continue;
                }
                let is_last_blocker = idx == blocker_count - 1;

                // Blocker deals damage to attacker.
                let blocker_has_first_strike = state.has_keyword(blocker_id, Keyword::FirstStrike, registry);
                let blocker_has_double_strike = state.has_keyword(blocker_id, Keyword::DoubleStrike, registry);
                let blocker_deals = if first_strike_only {
                    let deals = blocker_has_first_strike || blocker_has_double_strike;
                    if deals {
                        if let Some(c) = state.combat.as_mut() {
                            c.dealt_first_strike.insert(blocker_id);
                        }
                    }
                    deals
                } else {
                    blocker_has_double_strike
                        || !state.combat.as_ref().is_some_and(|c| c.dealt_first_strike.contains(&blocker_id))
                };

                if blocker_deals {
                    let blocker_power = u32::try_from(state.effective_power(blocker_id, registry).unwrap_or(0).max(0)).unwrap_or(0);
                    if blocker_power > 0 {
                        deal_damage_to_creature(state, blocker_id, attacker_id, blocker_power, registry);
                    }
                }

                // Attacker deals damage to blocker.
                if remaining_power > 0 {
                    let blocker_toughness = state.effective_toughness(blocker_id, registry).unwrap_or(0);
                    let blocker_damage = state.get_object(blocker_id).map_or(0, |o| o.damage_marked);
                    let lethal = if has_deathtouch_attacker {
                        1 // deathtouch: 1 damage is lethal
                    } else {
                        u32::try_from((blocker_toughness - i32::try_from(blocker_damage).unwrap_or(i32::MAX)).max(0)).unwrap_or(0)
                    };

                    // CR 510.1c/d: at least lethal damage to each blocker in
                    // damage assignment order. Excess damage goes to the next
                    // blocker (or to player if trample). For the last blocker,
                    // dump all remaining damage on it (no point holding back).
                    let assigned = if is_last_blocker && !has_trample {
                        remaining_power
                    } else {
                        remaining_power.min(lethal)
                    };

                    if assigned > 0 {
                        deal_damage_to_creature(state, attacker_id, blocker_id, assigned, registry);
                        remaining_power -= assigned;
                    }
                }
            }

            // Trample: remaining damage goes to the defending player.
            if has_trample && remaining_power > 0 {
                deal_damage_to_player(state, attacker_id, defending_player, remaining_power, registry);
            }
        }
    }
}

/// Get all subtypes of a creature (from both card data and object-level subtypes).
/// Transform-aware: uses back-face data for transformed DFCs.
#[must_use]
pub fn get_subtypes(state: &GameState, creature_id: ObjectId, registry: &CardRegistry) -> Vec<String> {
    state.subtypes_of(creature_id, registry)
}

/// Deal combat damage from a source creature to a target creature.
fn deal_damage_to_creature(
    state: &mut GameState,
    source: ObjectId,
    target: ObjectId,
    amount: u32,
    registry: &CardRegistry,
) {
    crate::damage::deal_damage(state, source, DamageTarget::Object(target), amount, crate::damage::DamageKind::Combat, registry);
}

/// Deal combat damage from a source creature to a player.
fn deal_damage_to_player(
    state: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    amount: u32,
    registry: &CardRegistry,
) {
    crate::damage::deal_damage(state, source, DamageTarget::Player(player), amount, crate::damage::DamageKind::Combat, registry);
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
    state.objects.values()
        .filter(|o| {
            o.zone == Zone::Battlefield
                && o.controller == player
                && state.is_creature(o.id, registry)
                && !o.tapped
                // Haste overrides summoning sickness.
                && (!o.summoning_sick || state.has_keyword(o.id, Keyword::Haste, registry))
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
    state.objects.values()
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
    // `can_block` is the "can't block" query (Vampire Interloper, and Bonds of
    // Faith's conditional form).
    if !state.can_block(blocker_id, registry) {
        return false;
    }
    // "Can't block this turn" (e.g., Nightbird's Clutches).
    !state.until_end_of_turn.iter().any(|e| matches!(e,
        crate::state::TemporaryEffect::CantBlock { target } if *target == blocker_id
    ))
}

/// Check if a blocker can legally block a specific attacker: the blanket
/// restrictions of [`can_block_at_all`], plus evasion (flying, intimidate,
/// protection) evaluated against this particular attacker.
///
/// Whether the attacker is actually attacking is enforced by the caller with
/// combat context (`declare_blockers_with_registry`).
#[must_use]
pub fn can_block_attacker(state: &GameState, blocker_id: ObjectId, attacker_id: ObjectId, registry: &CardRegistry) -> bool {
    if !can_block_at_all(state, blocker_id, registry) {
        return false;
    }

    // Flying: can only be blocked by creatures with flying or reach.
    if state.has_keyword(attacker_id, Keyword::Flying, registry)
        && !state.has_keyword(blocker_id, Keyword::Flying, registry)
        && !state.has_keyword(blocker_id, Keyword::Reach, registry)
    {
        return false;
    }

    // Intimidate: can only be blocked by artifact creatures or creatures that share a color.
    if state.has_keyword(attacker_id, Keyword::Intimidate, registry) {
        let is_artifact = state.has_card_type(blocker_id, crate::types::CardType::Artifact, registry);
        if !is_artifact {
            let attacker_colors = state.colors_of(attacker_id, registry);
            let blocker_colors = state.colors_of(blocker_id, registry);
            let shares_color = attacker_colors.iter().any(|c| blocker_colors.contains(c));
            if !shares_color {
                return false;
            }
        }
    }

    // Menace: must be blocked by two or more creatures (handled at validation, not per-blocker).

    // Block restriction (e.g., Orchard Spirit: only flying/reach can block).
    // The filter is read against the effect source's controller, which is why
    // this needs the source and not just the effect.
    let mut restricted = false;
    state.walk_effects(
        attacker_id,
        &|e| matches!(e, ContinuousEffect::CanOnlyBeBlockedBy { .. }),
        registry,
        &mut |e, source| {
            if let ContinuousEffect::CanOnlyBeBlockedBy { allowed_blockers, .. } = e {
                if !state.matches_filter(blocker_id, allowed_blockers, source.controller, registry) {
                    restricted = true;
                    return false;
                }
            }
            true
        },
    );
    if restricted {
        return false;
    }

    // "Can't be blocked" (e.g., Invisible Stalker) — check continuous effects.
    if state.cant_be_blocked(attacker_id, registry) {
        return false;
    }

    // Protection: a creature with protection from X can't be BLOCKED BY X.
    // Only check if the ATTACKER has protection from the blocker — that prevents the block.
    // A BLOCKER having protection from the attacker does NOT prevent it from blocking.
    if state.has_protection_from(attacker_id, blocker_id, registry) {
        return false;
    }

    true
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
        declare_attackers(&mut state, &[(attacker, defending)], &registry);
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
        declare_attackers(&mut state, &[(attacker, defending)], &registry);
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
        declare_attackers(&mut state, &[(attacker, p1)], &registry);
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
}
