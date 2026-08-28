//! Tests for Innistrad Tier 9 Equipment cards:
//! Cobbled Wings, Mask of Avacyn, Silver-Inlaid Dagger, Sharpened Pitchfork,
//! Butcher's Cleaver, Wooden Stake.
//!
//! Cards covered (5), so this is greppable by name as well as by rule:
//!
//! - Butcher's Cleaver
//! - Cobbled Wings
//! - Mask of Avacyn
//! - Silver-Inlaid Dagger
//! - Wooden Stake

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;
/// Helper: equip an equipment to a creature by activating the ability.
fn equip(
    state: &mtg_engine::state::GameState,
    registry: &CardRegistry,
    equipment_id: mtg_engine::ids::ObjectId,
    creature_id: mtg_engine::ids::ObjectId,
) -> mtg_engine::state::GameState {
    let legal = engine::legal_actions(state, registry);
    let equip_action = legal.actions.iter().find(|a| {
        matches!(a, Action::ActivateAbility { object_id, targets, .. }
            if *object_id == equipment_id && targets == &[Target::Object(creature_id)])
    }).expect("should be able to equip the creature");
    resolve_activated(engine::submit_action(state, equip_action, registry), registry)
}

// ══════════════════════════════════════════════════════════════════
// Cobbled Wings — {2} Equipment. Equipped creature has flying. Equip {1}.
// ══════════════════════════════════════════════════════════════════

#[test]
fn cobbled_wings_enters_as_equipment() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let wings = castable_spell(&mut state, &reg, "Cobbled Wings", P0);

    state = cast_and_resolve(&state, &reg, wings, vec![]);
    let obj = state.get_object(wings).unwrap();
    assert_eq!(obj.zone, Zone::Battlefield);
    assert!(state.is_equipment(obj.id, &reg));
    assert!(obj.attached_to.is_none());
}

#[test]
fn cobbled_wings_equip_only_your_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _opponent_creature = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let _wings = named_permanent(&mut state, &reg, "Cobbled Wings", P0);

    // Add mana for equip cost.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let legal = engine::legal_actions(&state, &reg);
    let equip_actions: Vec<_> = legal.actions.iter().filter(|a| {
        matches!(a, Action::ActivateAbility { .. })
    }).collect();

    // Should have no equip actions (no creatures to target that P0 controls).
    assert!(equip_actions.is_empty(), "Should not be able to equip opponent's creature");
}

// ── What equipping grants ────────────────────────────────────────

/// Equipping grants the printed static bonus. Nine tests used to walk this one
/// equipment at a time — three for the unconditional bonuses, six for the three
/// human-conditional ones in both directions. The conditional half lives in
/// `equipment_human_conditional.rs`, which already tables it and also covers
/// the bonus updating live when the creature stops being a Human; the
/// unconditional half is here.
const UNCONDITIONAL_GRANTS: &[(&str, u32, i32, i32, &[Keyword])] = &[
    // (equipment, equip cost, +power, +toughness, keywords granted)
    ("Cobbled Wings",   1, 0, 0, &[Keyword::Flying]),
    ("Mask of Avacyn",  3, 1, 2, &[Keyword::Hexproof]),
    ("Wooden Stake",    1, 1, 0, &[]),
    ("Butcher's Cleaver", 3, 3, 0, &[]),
    ("Silver-Inlaid Dagger", 2, 2, 0, &[]),
];

#[test]
fn equipping_grants_the_printed_bonus() {
    let reg = registry();
    for (name, cost, dp, dt, keywords) in UNCONDITIONAL_GRANTS {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        // Grizzly Bears is a 2/2 Bear — deliberately not a Human, so only the
        // unconditional half of a conditional equipment applies.
        let creature = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
        let equipment = named_permanent(&mut state, &reg, name, P0);

        for keyword in *keywords {
            assert!(!state.has_keyword(creature, *keyword, &reg),
                "test precondition: the Bear does not already have {keyword:?}");
        }

        state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, *cost);
        state = equip(&state, &reg, equipment, creature);

        assert_eq!(state.effective_power(creature, &reg), Some(2 + dp),
            "{name} should give +{dp} power");
        assert_eq!(state.effective_toughness(creature, &reg), Some(2 + dt),
            "{name} should give +{dt} toughness");
        for keyword in *keywords {
            assert!(state.has_keyword(creature, *keyword, &reg),
                "{name} should grant {keyword:?}");
        }
    }
}

/// "Equipped creature has flying." Flying is not an end in itself — the whole
/// of what the card does is change who may block. Every existing test stopped
/// at `has_keyword`, which is true of an implementation whose granted keywords
/// never reach the blocking rules, so this runs the consequence: a ground
/// creature wearing the Wings cannot be blocked by a ground creature, and can
/// be again the moment the Wings leave the battlefield (CR 509.1b).
#[test]
fn cobbled_wings_flying_reaches_the_blocking_rules() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let ground_blocker = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let wings = named_permanent(&mut state, &reg, "Cobbled Wings", P0);

    assert!(mtg_engine::combat::can_block_attacker(&state, ground_blocker, attacker, &reg),
        "test precondition: a ground creature blocks a ground creature");

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, wings, attacker);

    assert!(!mtg_engine::combat::can_block_attacker(&state, ground_blocker, attacker, &reg),
        "the Wings grant flying, so a creature with neither flying nor reach \
         can no longer block");

    // The grant lives on the Equipment, not on the creature, so destroying it
    // takes the flying with it. Two independent mechanisms hold this up and
    // either alone is enough, which is worth writing down because it makes the
    // obvious single-line mutations of this assertion vacuous: `walk_effects`
    // skips a source that is not on the battlefield, and `move_object` clears
    // `attached_to` on the way out so `EffectScope::Attached` matches nothing.
    state.move_object(wings, Zone::Graveyard, &reg);
    assert!(!state.has_keyword(attacker, Keyword::Flying, &reg),
        "flying came from the Wings and goes with them");
    assert!(mtg_engine::combat::can_block_attacker(&state, ground_blocker, attacker, &reg),
        "with the Wings gone the ground blocker can block again");
}

// ══════════════════════════════════════════════════════════════════
// Mask of Avacyn — {2} Equipment. +1/+2 and hexproof. Equip {3}.
// ══════════════════════════════════════════════════════════════════

/// Scryfall ruling (2011-09-22): "If Mask of Avacyn somehow becomes attached
/// to a creature an opponent controls, that creature can't be the target of
/// spells or abilities **you** control."
///
/// Hexproof is relative to the creature's controller (CR 702.11b), not to
/// whoever controls the thing granting it. Equip can only ever point at a
/// creature you control, so the only way here is a control change afterwards —
/// `change_control` is the engine's documented way to do that, used directly
/// so the test is about the Mask and not about Traitorous Blood's trample and
/// haste riders.
#[test]
fn mask_of_avacyn_turns_against_you_when_the_creature_changes_hands() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let mask = named_permanent(&mut state, &reg, "Mask of Avacyn", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);
    let mut state = equip(&state, &reg, mask, creature);

    // A removal spell in each player's hand, so "offered" and "not offered"
    // are both observable from the same board.
    let mine = castable_spell(&mut state, &reg, "Doom Blade", P0);
    let theirs = castable_spell(&mut state, &reg, "Doom Blade", P1);

    // While P0 controls both, the Mask protects the creature from P1.
    assert!(!offered_targets(&state, &reg, theirs).contains(&Target::Object(creature)),
        "test precondition: the opponent cannot target it");
    state.priority_player = Some(P0);
    assert!(offered_targets(&state, &reg, mine).contains(&Target::Object(creature)),
        "test precondition: its own controller can");

    // P1 takes the creature. The Mask stays where it is, still attached.
    state.change_control(creature, P1);
    assert_eq!(state.get_object(mask).unwrap().attached_to, Some(creature),
        "the Equipment does not fall off when the creature changes hands");
    assert_eq!(state.get_object(mask).unwrap().controller, P0,
        "and P0 still controls the Mask itself");

    // The grant follows the Equipment, not its controller.
    assert!(state.has_keyword(creature, Keyword::Hexproof, &reg),
        "the creature still has hexproof");
    assert_eq!(state.effective_power(creature, &reg), Some(3), "and still +1/+2");
    assert_eq!(state.effective_toughness(creature, &reg), Some(4));

    // The ruling: it now points the other way.
    state.priority_player = Some(P0);
    assert!(!offered_targets(&state, &reg, mine).contains(&Target::Object(creature)),
        "P0 controls the Mask but is now an opponent of the creature's \
         controller, so P0 cannot target it");
    state.priority_player = Some(P1);
    assert!(offered_targets(&state, &reg, theirs).contains(&Target::Object(creature)),
        "and P1, who now controls it, can");
}

// ══════════════════════════════════════════════════════════════════
// Wooden Stake — {2} Equipment. +1/+0; destroy Vampires on block. Equip {1}.
// ══════════════════════════════════════════════════════════════════

#[test]
fn wooden_stake_destroys_vampire_on_block() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Set up: P0 has a creature with Wooden Stake, P1 has a Vampire attacker.
    let creature = named_permanent(&mut state, &reg, "Grizzly Bears", P0); // 2/2
    let stake_obj = named_permanent(&mut state, &reg, "Wooden Stake", P0);

    // Equip.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, stake_obj, creature);

    // P1 has a Vampire attacker (Markov Patrician is a 3/1 Vampire with no evasion).
    let vampire = named_permanent(&mut state, &reg, "Markov Patrician", P1);

    // Move to declare blockers step with the vampire attacking.
    state.step = Step::DeclareBlockers;
    state.active_player = P1;
    // Set up combat with vampire as attacker.
    attacks_blocked_by(&mut state, vampire, P0, &[]);

    // Declare blockers: creature blocks vampire.
    submit_declare_blockers(&mut state, P0, &[(creature, vampire)], &reg);

    // Process triggers — this should fire Wooden Stake's block trigger.
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    // The vampire should be destroyed.
    assert_eq!(state.get_object(vampire).unwrap().zone, Zone::Graveyard,
        "Vampire should be destroyed by Wooden Stake's trigger");
}

#[test]
fn wooden_stake_does_not_destroy_non_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let stake_obj = named_permanent(&mut state, &reg, "Wooden Stake", P0);

    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, stake_obj, creature);

    // P1 has a non-Vampire attacker.
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);

    state.step = Step::DeclareBlockers;
    state.active_player = P1;
    attacks_blocked_by(&mut state, bear, P0, &[]);

    submit_declare_blockers(&mut state, P0, &[(creature, bear)], &reg);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    // Non-Vampire should NOT be destroyed.
    assert_eq!(state.get_object(bear).unwrap().zone, Zone::Battlefield,
        "Non-Vampire should not be destroyed by Wooden Stake");
}

// ══════════════════════════════════════════════════════════════════
// Equipment general mechanics
// ══════════════════════════════════════════════════════════════════

#[test]
fn equipment_detaches_when_creature_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_permanent(&mut state, &reg, "Grizzly Bears", P0); // 2/2
    let wings = named_permanent(&mut state, &reg, "Cobbled Wings", P0);

    // Equip.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, wings, creature);
    assert_eq!(state.get_object(wings).unwrap().attached_to, Some(creature));

    // Kill the creature.
    state.get_object_mut(creature).unwrap().damage_marked = 2;
    check_state_based_actions(&mut state, &reg);

    // Creature is dead but equipment should remain on battlefield, unattached.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(wings).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(wings).unwrap().attached_to, None);
}

#[test]
fn equipment_can_be_moved_to_different_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature1 = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let creature2 = named_permanent(&mut state, &reg, "Savannah Lions", P0);
    let wings = named_permanent(&mut state, &reg, "Cobbled Wings", P0);

    // Equip to first creature.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, wings, creature1);
    assert!(state.has_keyword(creature1, Keyword::Flying, &reg));
    assert!(!state.has_keyword(creature2, Keyword::Flying, &reg));

    // Re-equip to second creature.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, wings, creature2);
    assert!(!state.has_keyword(creature1, Keyword::Flying, &reg));
    assert!(state.has_keyword(creature2, Keyword::Flying, &reg));
}

#[test]
fn equipment_cast_and_equip_full_flow() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_permanent(&mut state, &reg, "Grizzly Bears", P0);

    // Cast Cobbled Wings.
    let wings = castable_spell(&mut state, &reg, "Cobbled Wings", P0);
    state = cast_and_resolve(&state, &reg, wings, vec![]);

    // Equipment should be on battlefield, unattached.
    assert_eq!(state.get_object(wings).unwrap().zone, Zone::Battlefield);
    assert!(state.is_equipment(wings, &reg));
    assert!(state.get_object(wings).unwrap().attached_to.is_none());
    assert!(!state.has_keyword(creature, Keyword::Flying, &reg));

    // Equip.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, wings, creature);
    assert!(state.has_keyword(creature, Keyword::Flying, &reg));
}

/// Being an Equipment is a fact about the card's subtypes (CR 301.5), not a
/// flag something has to remember to set.
///
/// It used to be `GameObject::is_equipment`, set by eleven cards in an
/// `on_resolve` override that otherwise only repeated the trait default's
/// "move a permanent to the battlefield". An Equipment that reached the
/// battlefield any other way left the flag false, and `sba.rs` then read it as
/// an unattached Aura: when the equipped creature died, the Equipment went to
/// the graveyard (CR 704.5m) instead of detaching and staying put.
///
/// Every Equipment in the set, placed on the battlefield directly rather than
/// cast, so the old flag would have been false for all of them.
#[test]
fn an_equipment_that_did_not_resolve_as_a_spell_still_detaches_rather_than_dying() {
    let reg = registry();
    let equipment: Vec<String> = reg.all_names().into_iter()
        .filter(|name| reg.get_id_by_name(name)
            .and_then(|id| reg.card_data(id))
            .is_some_and(|d| d.subtypes.iter().any(|s| s == "Equipment")))
        .map(std::string::ToString::to_string)
        .collect();
    assert!(equipment.len() >= 10,
        "test premise: the set has a pile of Equipment; found {equipment:?}");

    for name in equipment {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let eq = named_permanent(&mut state, &reg, name.as_str(), P0);
        let creature = ready_creature(&mut state, P0, 2, 2);
        state.get_object_mut(eq).unwrap().attached_to = Some(creature);
        assert!(state.is_equipment(eq, &reg), "{name} is an Equipment by its subtype");

        // The creature it was attached to leaves the battlefield.
        mtg_engine::destruction::try_destroy(&mut state, creature, &reg);
        mtg_engine::sba::check_state_based_actions(&mut state, &reg);

        assert_eq!(state.get_object(eq).unwrap().zone, Zone::Battlefield,
            "{name} stays on the battlefield when what it equipped dies — it is \
             an Equipment, not an Aura");
        assert_eq!(state.get_object(eq).unwrap().attached_to, None,
            "{name} detaches");
    }
}

/// Ruling: "The Vampire is destroyed before any combat damage is dealt."
///
/// The trigger fires on blockers being declared, so it resolves during the
/// declare blockers step — a step before combat damage. The equipped creature
/// takes nothing from the Vampire it staked.
#[test]
fn wooden_stakes_vampire_dies_before_it_can_deal_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_permanent(&mut state, &reg, "Grizzly Bears", P0); // 2/2
    let stake_obj = named_permanent(&mut state, &reg, "Wooden Stake", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, stake_obj, creature);

    // Markov Patrician is a 3/1 Vampire — enough to kill a 2/2 Bears (3/2 with
    // the Stake) if it ever got to deal damage.
    let vampire = named_permanent(&mut state, &reg, "Markov Patrician", P1);

    state.step = Step::DeclareBlockers;
    state.active_player = P1;
    attacks_blocked_by(&mut state, vampire, P0, &[]);
    submit_declare_blockers(&mut state, P0, &[(creature, vampire)], &reg);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(vampire).unwrap().zone, Zone::Graveyard);

    // Now let combat damage happen. There is no Vampire left to deal any.
    mtg_engine::engine::advance_step(&mut state, &reg);
    while mtg_engine::sba::check_state_based_actions(&mut state, &reg) {}

    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "the blocker survives — the Vampire was destroyed before damage");
    assert_eq!(state.get_object(creature).unwrap().damage_marked, 0,
        "and took no damage at all");
}

/// "destroy that creature. **It can't be regenerated.**" — a regeneration
/// shield does not save the Vampire, which is why this goes through
/// `try_destroy_no_regen` rather than `try_destroy`.
#[test]
fn wooden_stakes_vampire_cannot_regenerate() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let stake_obj = named_permanent(&mut state, &reg, "Wooden Stake", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, stake_obj, creature);

    let vampire = named_permanent(&mut state, &reg, "Markov Patrician", P1);
    // The Vampire's controller has already paid for a regeneration shield.
    state.get_object_mut(vampire).unwrap().regeneration_shields = 1;

    // First establish the shield is live: an ordinary destroy is replaced by
    // regeneration (CR 701.15), so "it died anyway" below means something.
    {
        let mut probe = state.clone();
        let result = mtg_engine::destruction::try_destroy(&mut probe, vampire, &reg);
        assert_eq!(result, mtg_engine::destruction::DestroyResult::Regenerated,
            "the shield saves it from ordinary destruction");
        assert_eq!(probe.get_object(vampire).unwrap().zone, Zone::Battlefield);
    }

    state.step = Step::DeclareBlockers;
    state.active_player = P1;
    attacks_blocked_by(&mut state, vampire, P0, &[]);
    submit_declare_blockers(&mut state, P0, &[(creature, vampire)], &reg);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(vampire).unwrap().zone, Zone::Graveyard,
        "the same shield does not save it from the Stake — it can't be regenerated");
}

/// The other half of "blocks **or becomes blocked by** a Vampire": the equipped
/// creature attacks and a Vampire blocks it.
#[test]
fn wooden_stake_destroys_a_vampire_that_blocks_the_equipped_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let stake_obj = named_permanent(&mut state, &reg, "Wooden Stake", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
    state = equip(&state, &reg, stake_obj, creature);

    let vampire = named_permanent(&mut state, &reg, "Markov Patrician", P1);

    // This time P0's equipped creature is the attacker.
    state.step = Step::DeclareBlockers;
    state.active_player = P0;
    attacks_blocked_by(&mut state, creature, P1, &[]);
    submit_declare_blockers(&mut state, P1, &[(vampire, creature)], &reg);
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(vampire).unwrap().zone, Zone::Graveyard,
        "a Vampire that blocks the equipped creature is staked too");
}
