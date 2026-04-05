
use crate::cards::CardRegistry;
use crate::events::{GameEvent, DamageTarget};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{CombatState, GameState, LogLevel};
use crate::types::{Keyword, Zone};

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
    let valid: Vec<_> = assignments.iter()
        .filter(|&&(blocker, attacker)| can_block_attacker(state, blocker, attacker, registry))
        .cloned()
        .collect();

    // Minimum blocker enforcement: for attackers with menace or MinimumBlockers
    // effects, verify they have enough blockers. If not, remove the block
    // assignments for that attacker (can't be blocked by fewer than N creatures).
    let mut blocker_counts: std::collections::HashMap<ObjectId, usize> = std::collections::HashMap::new();
    for &(_, attacker) in &valid {
        *blocker_counts.entry(attacker).or_insert(0) += 1;
    }

    // Determine minimum blocker requirement for each attacker.
    let attacker_ids: Vec<ObjectId> = blocker_counts.keys().cloned().collect();
    let mut min_blockers: std::collections::HashMap<ObjectId, u32> = std::collections::HashMap::new();
    for &att_id in &attacker_ids {
        let mut min_req: u32 = 1; // default: any single creature can block
        // Menace keyword: need at least 2 blockers.
        if state.has_keyword(att_id, Keyword::Menace, registry) {
            min_req = min_req.max(2);
        }
        // MinimumBlockers continuous effects (e.g., Terror of Kruin Pass).
        for source in state.objects.values() {
            if source.zone != crate::types::Zone::Battlefield {
                continue;
            }
            let effects = if let Some(ref inst) = source.instance_continuous_effects {
                inst.clone()
            } else if let Some(behavior) = registry.get(source.card_id) {
                if source.is_transformed {
                    behavior.back_face_data().map(|d| d.continuous_effects).unwrap_or_default()
                } else {
                    behavior.card_data().continuous_effects
                }
            } else {
                vec![]
            };
            for effect in &effects {
                if let crate::types::ContinuousEffect::MinimumBlockers { count, scope } = effect {
                    if state.effect_applies_to(att_id, scope, source.id, source.controller, registry) {
                        min_req = min_req.max(*count);
                    }
                }
            }
        }
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
        // Run SBAs between first strike and normal damage.
        while crate::sba::check_state_based_actions_with_registry(state, Some(registry)) {}
        // Normal damage step: non-first-strikers + double strikers.
        deal_damage_step(state, &combat, registry, false);
    } else {
        // No first strike: everyone deals damage simultaneously.
        deal_damage_step(state, &combat, registry, false);
    }
}

/// Fight: each creature deals damage equal to its power to the other.
/// Used by Prey Upon and similar "fight" cards.
pub fn fight(state: &mut GameState, a: ObjectId, b: ObjectId, registry: &CardRegistry) {
    let power_a = state.effective_power(a, registry).unwrap_or(0).max(0) as u32;
    let power_b = state.effective_power(b, registry).unwrap_or(0).max(0) as u32;

    if power_a > 0 {
        deal_fight_damage(state, a, b, power_a, registry);
    }
    if power_b > 0 {
        deal_fight_damage(state, b, a, power_b, registry);
    }
}

/// Deal fight damage (non-combat). Respects protection, deathtouch, lifelink,
/// but NOT combat-only modifiers (Inquisitor's Flail, Moonmist, Ghostly Possession).
fn deal_fight_damage(
    state: &mut GameState,
    source: ObjectId,
    target: ObjectId,
    amount: u32,
    registry: &CardRegistry,
) {
    // Protection: if target has protection from the source, prevent damage.
    if has_protection_from_creature(state, target, source, registry) {
        return;
    }

    let has_deathtouch = state.has_keyword(source, Keyword::Deathtouch, registry);
    if let Some(obj) = state.get_object_mut(target) {
        obj.damage_marked += amount;
        if has_deathtouch {
            obj.dealt_deathtouch_damage = true;
        }
        if !obj.damaged_by.contains(&source) {
            obj.damaged_by.push(source);
        }
    }
    state.events.push(GameEvent::NonCombatDamageDealt {
        source,
        target: DamageTarget::Object(target),
        amount,
    });

    // Lifelink: source's controller gains life.
    if state.has_keyword(source, Keyword::Lifelink, registry) {
        let controller = state.get_object(source).expect("damage source must exist").controller;
        let old_life = state.get_player(controller).life;
        let new_life = old_life + amount as i32;
        state.get_player_mut(controller).life = new_life;
        state.events.push(GameEvent::LifeChanged {
            player: controller,
            old: old_life,
            new_life,
        });
    }
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
        if state.get_object(attacker_id).map(|o| o.zone != Zone::Battlefield).unwrap_or(true) {
            continue;
        }

        let has_fs = state.has_keyword(attacker_id, Keyword::FirstStrike, registry);
        let has_ds = state.has_keyword(attacker_id, Keyword::DoubleStrike, registry);
        let attacker_deals = if first_strike_only {
            has_fs || has_ds
        } else {
            !has_fs || has_ds // normal strikers + double strikers
        };

        let attacker_power = if attacker_deals {
            state.effective_power(attacker_id, registry).unwrap_or(0).max(0) as u32
        } else {
            0
        };

        let has_trample = state.has_keyword(attacker_id, Keyword::Trample, registry);
        let has_deathtouch_attacker = state.has_keyword(attacker_id, Keyword::Deathtouch, registry);

        let blockers = combat.blocker_assignments.get(&attacker_id)
            .cloned()
            .unwrap_or_default();

        if blockers.is_empty() {
            // Unblocked: deal damage to defending player.
            if attacker_power > 0 {
                deal_damage_to_player(state, attacker_id, defending_player, attacker_power, registry);
            }
        } else {
            // Blocked: distribute damage to blockers, with trample overflow.
            let mut remaining_power = attacker_power;

            for &blocker_id in &blockers {
                if state.get_object(blocker_id).map(|o| o.zone != Zone::Battlefield).unwrap_or(true) {
                    continue;
                }

                // Blocker deals damage to attacker.
                let blocker_has_fs = state.has_keyword(blocker_id, Keyword::FirstStrike, registry);
                let blocker_has_ds = state.has_keyword(blocker_id, Keyword::DoubleStrike, registry);
                let blocker_deals = if first_strike_only {
                    blocker_has_fs || blocker_has_ds
                } else {
                    !blocker_has_fs || blocker_has_ds
                };

                if blocker_deals {
                    let blocker_power = state.effective_power(blocker_id, registry).unwrap_or(0).max(0) as u32;
                    if blocker_power > 0 {
                        deal_damage_to_creature(state, blocker_id, attacker_id, blocker_power, registry);
                    }
                }

                // Attacker deals damage to blocker.
                if remaining_power > 0 {
                    let blocker_toughness = state.effective_toughness(blocker_id, registry).unwrap_or(0);
                    let blocker_damage = state.get_object(blocker_id).map(|o| o.damage_marked).unwrap_or(0);
                    let lethal = if has_deathtouch_attacker {
                        1 // deathtouch: 1 damage is lethal
                    } else {
                        (blocker_toughness - blocker_damage as i32).max(0) as u32
                    };

                    let assigned = if has_trample {
                        remaining_power.min(lethal) // assign minimum lethal, save rest for trample
                    } else {
                        remaining_power // assign all to this blocker
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

/// Check if a creature has a "prevent damage, remove counter" replacement effect
/// (e.g., Unbreathing Horde). If so, prevent the damage and remove a +1/+1 counter.
/// Returns true if damage was prevented.
fn apply_prevent_damage_remove_counter(state: &mut GameState, target: ObjectId, registry: &CardRegistry) -> bool {
    let has_effect = state.has_continuous_effect(target, &|e| {
        match e {
            crate::types::ContinuousEffect::PreventDamageRemoveCounter { scope } => Some(scope),
            _ => None,
        }
    }, registry);
    if has_effect {
        let counter_count = state.get_object(target)
            .and_then(|o| o.counters.get(&crate::types::CounterType::PlusOnePlusOne).copied())
            .unwrap_or(0);
        if counter_count > 0 {
            if let Some(obj) = state.get_object_mut(target) {
                let entry = obj.counters.entry(crate::types::CounterType::PlusOnePlusOne).or_insert(0);
                *entry = entry.saturating_sub(1);
                if *entry == 0 {
                    obj.counters.remove(&crate::types::CounterType::PlusOnePlusOne);
                }
            }
            let name = state.get_object(target).map(|o| o.name.clone()).unwrap_or_default();
            state.log(crate::state::LogLevel::Event,
                format!("{}: damage prevented, removed a +1/+1 counter", name));
        }
        // Damage is always prevented even if no counters remain.
        true
    } else {
        false
    }
}

/// Check if a creature's combat damage should be prevented because it's not a Wolf/Werewolf
/// (set by Moonmist's effect for the rest of the turn).
fn is_non_wolf_damage_prevented(state: &GameState, source: ObjectId, registry: &CardRegistry) -> bool {
    if !state.prevent_non_wolf_werewolf_combat_damage {
        return false;
    }
    let subtypes = get_subtypes(state, source, registry);
    !subtypes.iter().any(|s| s == "Wolf" || s == "Werewolf")
}

/// Compute the combat damage multiplier from DoubleCombatDamage effects (e.g., Inquisitor's Flail).
/// Each source doubles independently: 1 Flail = x2, 2 Flails = x4, 3 = x8, etc.
fn combat_damage_multiplier(state: &GameState, creature_id: ObjectId, registry: &CardRegistry) -> u32 {
    let count = state.count_continuous_effect(creature_id, &|e| {
        match e {
            crate::types::ContinuousEffect::DoubleCombatDamage { scope } => Some(scope),
            _ => None,
        }
    }, registry);
    1u32 << count // 2^count
}

/// Check if a creature has combat damage prevented (e.g., Ghostly Possession).
fn has_damage_prevention(state: &GameState, creature_id: ObjectId, registry: &CardRegistry) -> bool {
    state.has_continuous_effect(creature_id, &|e| {
        match e {
            crate::types::ContinuousEffect::PreventCombatDamage { scope } => Some(scope),
            _ => None,
        }
    }, registry)
}

/// Check if a creature has protection from a specific subtype.
fn has_protection_from(state: &GameState, creature_id: ObjectId, subtype: &str, registry: &CardRegistry) -> bool {
    for source in state.objects.values() {
        if source.zone != crate::types::Zone::Battlefield {
            continue;
        }
        let effects = if let Some(ref instance_effects) = source.instance_continuous_effects {
            instance_effects.clone()
        } else if let Some(behavior) = registry.get(source.card_id) {
            behavior.card_data().continuous_effects
        } else {
            continue;
        };
        for effect in &effects {
            if let crate::types::ContinuousEffect::ProtectionFromSubtype { subtype: prot_sub, scope } = effect {
                if prot_sub == subtype && state.effect_applies_to(creature_id, scope, source.id, source.controller, registry) {
                    return true;
                }
            }
        }
    }
    false
}

/// Get all subtypes of a creature (from both card data and object-level subtypes).
fn get_subtypes(state: &GameState, creature_id: ObjectId, registry: &CardRegistry) -> Vec<String> {
    let mut subtypes = Vec::new();
    if let Some(obj) = state.get_object(creature_id) {
        subtypes.extend(obj.subtypes.iter().cloned());
        if let Some(data) = registry.card_data(obj.card_id) {
            for s in &data.subtypes {
                if !subtypes.contains(s) {
                    subtypes.push(s.clone());
                }
            }
        }
    }
    subtypes
}

/// Check if creature_a has protection from creature_b.
/// Checks all protection-from-subtype effects and until-EOT protection grants.
fn has_protection_from_creature(state: &GameState, protected: ObjectId, attacker: ObjectId, registry: &CardRegistry) -> bool {
    let attacker_subtypes = get_subtypes(state, attacker, registry);

    // Check static protection-from-subtype effects.
    for subtype in &attacker_subtypes {
        if has_protection_from(state, protected, subtype, registry) {
            return true;
        }
    }

    // Check static ProtectionFrom (filter-based) effects.
    for source in state.objects.values() {
        if source.zone != crate::types::Zone::Battlefield {
            continue;
        }
        let effects = if let Some(ref instance_effects) = source.instance_continuous_effects {
            instance_effects.clone()
        } else if let Some(behavior) = registry.get(source.card_id) {
            behavior.card_data().continuous_effects
        } else {
            continue;
        };
        for effect in &effects {
            if let crate::types::ContinuousEffect::ProtectionFrom { filter, scope } = effect {
                if state.effect_applies_to(protected, scope, source.id, source.controller, registry) {
                    if state.matches_filter(attacker, filter, source.controller, registry) {
                        return true;
                    }
                }
            }
        }
    }

    // Check until-end-of-turn protection grants.
    for prot in &state.until_end_of_turn_protection {
        if prot.target == protected {
            // Check if the attacker matches the filter.
            // Use controller of the protected creature for filter context.
            let controller = state.get_object(protected).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
            if state.matches_filter(attacker, &prot.filter, controller, registry) {
                return true;
            }
        }
    }

    false
}

/// Deal damage from a source creature to a target creature. Handles lifelink.
fn deal_damage_to_creature(
    state: &mut GameState,
    source: ObjectId,
    target: ObjectId,
    amount: u32,
    registry: &CardRegistry,
) {
    // Skip if source or target has combat damage prevention (e.g., Ghostly Possession).
    if has_damage_prevention(state, source, registry) || has_damage_prevention(state, target, registry) {
        return;
    }

    // Moonmist: prevent combat damage from non-Wolf/non-Werewolf creatures.
    if is_non_wolf_damage_prevented(state, source, registry) {
        return;
    }

    // Protection: if target has protection from the source creature, prevent damage.
    if has_protection_from_creature(state, target, source, registry) {
        return;
    }

    // Unbreathing Horde: prevent damage, remove counter.
    if apply_prevent_damage_remove_counter(state, target, registry) {
        return;
    }

    // Inquisitor's Flail: multiply damage for each DoubleCombatDamage source.
    // Dealing: source's Flails multiply the damage dealt.
    // Receiving: target's Flails multiply the damage received.
    let mut amount = amount;
    amount *= combat_damage_multiplier(state, source, registry);
    amount *= combat_damage_multiplier(state, target, registry);

    let has_deathtouch = state.has_keyword(source, Keyword::Deathtouch, registry);
    if let Some(obj) = state.get_object_mut(target) {
        obj.damage_marked += amount;
        if has_deathtouch {
            obj.dealt_deathtouch_damage = true;
        }
        // Track which creatures dealt damage to this creature (for Abattoir Ghoul).
        if !obj.damaged_by.contains(&source) {
            obj.damaged_by.push(source);
        }
    }
    state.events.push(GameEvent::CombatDamageDealt {
        source,
        target: DamageTarget::Object(target),
        amount,
    });

    // Lifelink: source's controller gains life.
    if state.has_keyword(source, Keyword::Lifelink, registry) {
        let controller = state.get_object(source).expect("damage source must exist").controller;
        let old_life = state.get_player(controller).life;
        let new_life = old_life + amount as i32;
        state.get_player_mut(controller).life = new_life;
        state.events.push(GameEvent::LifeChanged {
            player: controller,
            old: old_life,
            new_life,
        });
    }
}

/// Deal damage from a source creature to a player. Handles lifelink.
fn deal_damage_to_player(
    state: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    amount: u32,
    registry: &CardRegistry,
) {
    // Skip if source has combat damage prevention (e.g., Ghostly Possession).
    if has_damage_prevention(state, source, registry) {
        return;
    }

    // Moonmist: prevent combat damage from non-Wolf/non-Werewolf creatures.
    if is_non_wolf_damage_prevented(state, source, registry) {
        return;
    }

    // Inquisitor's Flail: multiply damage for each DoubleCombatDamage source.
    let mut amount = amount;
    amount *= combat_damage_multiplier(state, source, registry);

    let old_life = state.get_player(player).life;
    let new_life = old_life - amount as i32;
    state.get_player_mut(player).life = new_life;

    state.events.push(GameEvent::CombatDamageDealt {
        source,
        target: DamageTarget::Player(player),
        amount,
    });
    state.events.push(GameEvent::LifeChanged {
        player,
        old: old_life,
        new_life,
    });

    let name = state.get_object(source)
        .map(|o| {
            // Use object name directly (works for tokens); fall back to registry.
            if !o.name.is_empty() {
                o.name.clone()
            } else {
                registry.card_data(o.card_id)
                    .map(|d| d.name)
                    .unwrap_or_else(|| "?".into())
            }
        })
        .unwrap_or_else(|| "?".into());
    state.log(LogLevel::Event, format!("p{} took {} combat damage ({}) from {}", player.0, amount, new_life, name));

    // Lifelink: source's controller gains life.
    if state.has_keyword(source, Keyword::Lifelink, registry) {
        let controller = state.get_object(source).expect("damage source must exist").controller;
        let old = state.get_player(controller).life;
        let new = old + amount as i32;
        state.get_player_mut(controller).life = new;
        state.events.push(GameEvent::LifeChanged {
            player: controller,
            old,
            new_life: new,
        });
    }
}

/// Clean up combat state at end of combat.
pub fn end_combat(state: &mut GameState) {
    // Process delayed exile triggers (e.g., Geist of Saint Traft's Angel token).
    // These fire independently of the source permanent's presence on the battlefield.
    let exiles: Vec<_> = state.end_of_combat_exiles.drain(..).collect();
    for exile_id in exiles {
        if state.get_object(exile_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            state.move_object(exile_id, Zone::Exile);
            state.log(crate::state::LogLevel::Event,
                "Token exiled at end of combat (delayed trigger)".into());
        }
    }
    state.combat = None;
}

/// Get all creatures a player controls that are eligible to attack.
/// Checks keywords (defender, haste) and continuous effects (Pacifism).
pub fn eligible_attackers(state: &GameState, player: PlayerId, registry: &CardRegistry) -> Vec<ObjectId> {
    state.objects.values()
        .filter(|o| {
            o.zone == Zone::Battlefield
                && o.controller == player
                && o.power.is_some()
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
pub fn eligible_blockers(state: &GameState, player: PlayerId, registry: &CardRegistry) -> Vec<ObjectId> {
    state.objects.values()
        .filter(|o| {
            o.zone == Zone::Battlefield
                && o.controller == player
                && o.power.is_some()
                && !o.tapped
        })
        .map(|o| o.id)
        .collect::<Vec<_>>()
        .into_iter()
        .filter(|&id| state.can_block(id, registry))
        // "Can't block" (e.g., Vampire Interloper) — check continuous effects.
        .filter(|&id| {
            !state.has_continuous_effect(id, &|e| {
                match e {
                    crate::types::ContinuousEffect::PreventBlock { scope } => Some(scope),
                    _ => None,
                }
            }, registry)
        })
        // "Can't block this turn" (e.g., Nightbird's Clutches).
        .filter(|&id| !state.until_end_of_turn_cant_block.contains(&id))
        .collect()
}

/// Check if a blocker can legally block a specific attacker.
/// Enforces flying (only blocked by flying/reach) and intimidate (only by artifact/same color).
pub fn can_block_attacker(state: &GameState, blocker_id: ObjectId, attacker_id: ObjectId, registry: &CardRegistry) -> bool {
    // Flying: can only be blocked by creatures with flying or reach.
    if state.has_keyword(attacker_id, Keyword::Flying, registry) {
        if !state.has_keyword(blocker_id, Keyword::Flying, registry)
            && !state.has_keyword(blocker_id, Keyword::Reach, registry) {
            return false;
        }
    }

    // Intimidate: can only be blocked by artifact creatures or creatures that share a color.
    if state.has_keyword(attacker_id, Keyword::Intimidate, registry) {
        let blocker = match state.get_object(blocker_id) {
            Some(o) => o,
            None => return false,
        };
        let is_artifact = registry.card_data(blocker.card_id)
            .map(|d| d.card_types.contains(&crate::types::CardType::Artifact))
            .unwrap_or(false);
        if !is_artifact {
            let attacker = match state.get_object(attacker_id) {
                Some(o) => o,
                None => return false,
            };
            let shares_color = attacker.colors.iter().any(|c| blocker.colors.contains(c));
            if !shares_color {
                return false;
            }
        }
    }

    // Menace: must be blocked by two or more creatures (handled at validation, not per-blocker).

    // Block restriction (e.g., Orchard Spirit: only flying/reach can block).
    for source in state.objects.values() {
        if source.zone != crate::types::Zone::Battlefield {
            continue;
        }
        // Check instance-level effects first (e.g., equipment with BlockRestriction).
        if let Some(ref instance_effects) = source.instance_continuous_effects {
            for effect in instance_effects {
                if let crate::types::ContinuousEffect::BlockRestriction { allowed_blockers, scope } = effect {
                    if state.effect_applies_to(attacker_id, scope, source.id, source.controller, registry) {
                        if !state.matches_filter(blocker_id, allowed_blockers, source.controller, registry) {
                            return false;
                        }
                    }
                }
            }
        } else if let Some(behavior) = registry.get(source.card_id) {
            // Use back face effects when transformed (for DFCs like werewolves).
            let effects = if source.is_transformed {
                behavior.back_face_data().map(|d| d.continuous_effects).unwrap_or_default()
            } else {
                behavior.card_data().continuous_effects
            };
            for effect in &effects {
                if let crate::types::ContinuousEffect::BlockRestriction { allowed_blockers, scope } = effect {
                    if state.effect_applies_to(attacker_id, scope, source.id, source.controller, registry) {
                        // This attacker has a block restriction. Check if the blocker passes the filter.
                        if !state.matches_filter(blocker_id, allowed_blockers, source.controller, registry) {
                            return false;
                        }
                    }
                }
            }
        }
    }

    // "Can't be blocked" (e.g., Invisible Stalker) — check continuous effects.
    if state.has_continuous_effect(attacker_id, &|e| {
        match e {
            crate::types::ContinuousEffect::CantBeBlocked { scope } => Some(scope),
            _ => None,
        }
    }, registry) {
        return false;
    }

    // Protection: a creature with protection from X can't be BLOCKED BY X.
    // Only check if the ATTACKER has protection from the blocker — that prevents the block.
    // A BLOCKER having protection from the attacker does NOT prevent it from blocking.
    if has_protection_from_creature(state, attacker_id, blocker_id, registry) {
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
