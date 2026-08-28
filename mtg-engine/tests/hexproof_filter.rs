//! Regressions for bugs documented in `audits/AUDIT_BUGS.md`. Each of these
//! failed when it was written and passes now; they stay to protect against
//! the bug coming back.
//!
//! This file covers the "Hexproof, protection, and target-filter
//! gaps" family. Most of these bugs are call sites that build a
//! "valid targets" list without consulting the engine's
//! `can_be_targeted_by` / `player_has_hexproof` helpers, so illegal
//! targets leak into the player's prompt.
//!
//! Bugs covered in this file:
//! - Bug 17-003: shared `creature_targets` / `any_targets` helpers
//!   skip hexproof (Pitchburn Devils' on-death "any target" offers
//!   opp's Lumberknot)
//! - Bug E1-001: Grimgrin, Corpse-Born's attack trigger inline-builds
//!   targets without a hexproof filter
//! - Bug 0F-003: Falkenrath Noble's drain trigger lets the controller
//!   pick a player even if that player has hexproof from Witchbane Orb
//! - Bug H: Into the Maw of Hell's first-target enumeration drops the
//!   `PermanentWithFilter(land)` filter, so creatures appear as
//!   first-target options
//! - Bug AD: Unburial Rites' `GraveyardCreature` enumerator returns
//!   creature cards from any graveyard (not just the caster's)
//! - Bug 9F-001: Snapcaster Mage excludes graveyard cards that already
//!   have printed flashback
//! - Bug AW: Prey Upon's `TwoTargets` path ignores `YouControl` /
//!   `YouDontControl` filters, so the model can fight two of its own
//!   creatures

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::{AttackInfo, CardRegistry};
use mtg_engine::engine;
use mtg_engine::types::*;

/// Bug 17-003 (`audits/AUDIT_BUGS.md)`: the shared `creature_targets` /
/// `any_targets` helpers in `cards/helpers.rs` don't filter out hexproof
/// creatures the trigger's controller doesn't control.
///
/// Oracle (Pitchburn Devils): "When this creature dies, it deals 3
/// damage to any target."
/// Oracle (Lumberknot): "Hexproof (This creature can't be the target of
/// spells or abilities your opponents control.)"
///
/// Failure mode: Pitchburn Devils' `on_dies` calls
/// `cards::helpers::any_targets(state)`, which returns every creature
/// on the battlefield + every player without consulting
/// `engine::can_be_targeted_by`. So Pitchburn Devils' controller is
/// offered Lumberknot (an opponent's hexproof creature) as a damage
/// target, even though CR 702.11b forbids it.
#[test]
fn bug_17_003_pitchburn_devils_does_not_offer_opponent_hexproof_creature() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Lumberknot is a Treefolk with Hexproof in P1's hand. Put it in
    // play under P1.
    let lumberknot = named_permanent(&mut state, &registry, "Lumberknot", P1);

    // Pitchburn Devils on P0's side. Fire its death trigger directly.
    let pd = named_permanent(&mut state, &registry, "Pitchburn Devils", P0);
    let pd_card_id = state.get_object(pd).unwrap().card_id;
    let behavior = registry.get(pd_card_id).unwrap();
    behavior.on_dies(&mut state, pd, &[], &registry);

    let lumberknot_in_options = match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTarget { options, .. },
            ..
        }) => options
            .iter()
            .any(|t| matches!(t, Target::Object(id) if *id == lumberknot)),
        _ => false,
    };

    assert!(
        !lumberknot_in_options,
        "Pitchburn Devils' on-death 'any target' should NOT offer an \
         opponent's hexproof creature as a target. Bug 17-003: \
         cards::helpers::any_targets() returns every creature without \
         calling engine::can_be_targeted_by, so hexproof is silently \
         skipped. awaiting_action = {:?}",
        state.awaiting_action,
    );
}

/// Bug 17-003 — `creature_targets` path (`audits/AUDIT_BUGS.md)`:
/// Crossway Vampire's ETB "target creature can't block" uses the
/// `creature_targets` helper (not `any_targets`), which also lacks
/// a hexproof filter. This is a DIFFERENT helper from the one
/// Pitchburn Devils tests.
///
/// Oracle (Crossway Vampire): "When this creature enters, target
/// creature can't block this turn."
#[test]
fn bug_17_003_crossway_vampire_creature_targets_excludes_hexproof() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let lumberknot = named_permanent(&mut state, &registry, "Lumberknot", P1);
    // A second non-hexproof target so present_target_choice doesn't
    // auto-resolve the single mandatory target.
    let _bears = ready_creature(&mut state, P1, 2, 2);

    let cv = named_permanent(&mut state, &registry, "Crossway Vampire", P0);
    let cv_card_id = state.get_object(cv).unwrap().card_id;
    let behavior = registry.get(cv_card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, cv, &[], &registry);

    let lumberknot_in_options = match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTarget { options, .. }, ..
        }) => options.iter().any(|t| matches!(t, Target::Object(id) if *id == lumberknot)),
        _ => false,
    };
    assert!(
        !lumberknot_in_options,
        "Crossway Vampire's ETB 'target creature can't block' should \
         NOT offer an opponent's hexproof creature. Bug 17-003: \
         creature_targets() doesn't filter hexproof."
    );
}

/// Bug 17-003 — `creature_targets_except` path (`audits/AUDIT_BUGS.md)`:
/// Fiend Hunter's ETB "you may exile another target creature" uses
/// the `creature_targets_except` helper, which also lacks a hexproof
/// filter. This is the third distinct helper in this bug family.
///
/// Oracle (Fiend Hunter): "When this creature enters, you may exile
/// another target creature."
#[test]
fn bug_17_003_fiend_hunter_creature_targets_except_excludes_hexproof() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let lumberknot = named_permanent(&mut state, &registry, "Lumberknot", P1);
    let _bears = ready_creature(&mut state, P1, 2, 2);

    let fh = named_permanent(&mut state, &registry, "Fiend Hunter", P0);
    let fh_card_id = state.get_object(fh).unwrap().card_id;
    let behavior = registry.get(fh_card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, fh, &[], &registry);

    let lumberknot_in_options = match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTarget { options, .. }, ..
        }) => options.iter().any(|t| matches!(t, Target::Object(id) if *id == lumberknot)),
        _ => false,
    };
    assert!(
        !lumberknot_in_options,
        "Fiend Hunter's ETB 'exile another target creature' should NOT \
         offer an opponent's hexproof creature. Bug 17-003: \
         creature_targets_except() doesn't filter hexproof."
    );
}

/// Bug E1-001 (`audits/AUDIT_BUGS.md)`: Grimgrin, Corpse-Born's attack
/// trigger inline-enumerates the defender's creatures and only filters
/// protection — not hexproof.
///
/// Oracle (Grimgrin, Corpse-Born): "Whenever this creature attacks,
/// destroy target creature defending player controls, then put a +1/+1
/// counter on this creature."
/// Oracle (Lumberknot): "Hexproof".
///
/// Failure mode: `grimgrin_corpse_born.rs` builds the target
/// list as `objects_in_zone(Battlefield, defender) | filter(power.is_some) |
/// filter(!has_protection_from)`. There's no `has_keyword(Hexproof)`
/// check, so an opponent's Lumberknot is offered as a destroy target
/// even though hexproof should make it untargetable to Grimgrin's
/// controller.
#[test]
fn bug_e1_001_grimgrin_attack_trigger_excludes_opponent_hexproof_creature() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let grimgrin = named_permanent(&mut state, &registry, "Grimgrin, Corpse-Born", P0);
    let lumberknot = named_permanent(&mut state, &registry, "Lumberknot", P1);
    // Add a second non-hexproof creature on P1's side so the trigger has
    // multiple potential targets — otherwise present_target_choice's
    // single-mandatory-target shortcut auto-applies the effect and never
    // sets awaiting_action.
    let _bears = ready_creature(&mut state, P1, 2, 2);

    // Fire Grimgrin's attack trigger directly. The on_attacks handler
    // figures out the defender from `state.combat`, and falls back to
    // opponent if there's no combat state — which is exactly what we
    // want for this test.
    let grimgrin_card_id = state.get_object(grimgrin).unwrap().card_id;
    let behavior = registry.get(grimgrin_card_id).unwrap();
    behavior.on_attacks(&mut state, grimgrin, AttackInfo::new(grimgrin, P1), &[], &registry);

    let lumberknot_in_options = match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTarget { options, .. },
            ..
        }) => options
            .iter()
            .any(|t| matches!(t, Target::Object(id) if *id == lumberknot)),
        _ => false,
    };

    assert!(
        !lumberknot_in_options,
        "Grimgrin, Corpse-Born's attack trigger should NOT offer the \
         defending player's Lumberknot (hexproof) as a destroy target. \
         Bug E1-001: the inline target enumerator only filters protection, \
         not hexproof. awaiting_action = {:?}",
        state.awaiting_action,
    );
}

/// Bug 0F-003 (`audits/AUDIT_BUGS.md)`: Falkenrath Noble's death-trigger
/// drain enumerates `state.players` directly, ignoring Witchbane Orb's
/// player-hexproof.
///
/// Oracle (Falkenrath Noble): "Whenever this creature or another creature
/// dies, target player loses 1 life and you gain 1 life."
/// Oracle (Witchbane Orb): "You have hexproof. (You can't be the target
/// of spells or abilities your opponents control, ...)"
///
/// Failure mode, since fixed: `falkenrath_noble.rs` built the target list
/// itself with `state.players.iter().map(|p| Target::Player(p.id))` and no
/// filter on `state.player_has_hexproof`.
///
/// The note that used to sit here said the same defect remained in Bloodgift
/// Demon, Selhoff Occultist and Rage Thrower, and that "the other three cards
/// need the same one-line fix". None of the four hand-rolls a target list any
/// more: each declares a `TargetRequirement` and lets the engine choose, and
/// `stack.rs::is_target_legal` filters player hexproof there. Prose cannot
/// notice when it goes out of date, so the claim is now
/// `every_player_targeting_trigger_leaves_targeting_to_the_engine` below.
#[test]
fn bug_0f_003_falkenrath_noble_skips_player_with_witchbane_orb() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1 has Witchbane Orb in play, so P1 has player-hexproof.
    let _orb = named_permanent(&mut state, &registry, "Witchbane Orb", P1);

    // Falkenrath Noble on P0's side. Fire its self-death trigger.
    let noble = named_permanent(&mut state, &registry, "Falkenrath Noble", P0);
    let noble_card_id = state.get_object(noble).unwrap().card_id;
    let behavior = registry.get(noble_card_id).unwrap();
    behavior.on_dies(&mut state, noble, &[], &registry);

    // After firing, P0 should be the only legal "target player" — P1
    // is hexproof. Two failure modes both indicate the bug:
    //   1. options contains Target::Player(P1)
    //   2. options is missing entirely (the trigger fizzled because we
    //      didn't expect a hexproof world). The fix should leave P0 as
    //      a legal self-target.
    let p1_in_options = match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTarget { options, .. },
            ..
        }) => options
            .iter()
            .any(|t| matches!(t, Target::Player(p) if *p == P1)),
        _ => false,
    };

    assert!(
        !p1_in_options,
        "Falkenrath Noble's drain trigger should NOT offer P1 (who has \
         player-hexproof from Witchbane Orb) as a 'target player'. \
         Bug 0F-003: the trigger walks state.players directly without \
         consulting player_has_hexproof. awaiting_action = {:?}",
        state.awaiting_action,
    );
}

/// What keeps player-hexproof working is that these cards do not choose their
/// own targets: each declares a `TargetRequirement`, and the engine's
/// `is_target_legal` refuses a player with hexproof that the ability's
/// controller does not control (CR 702.11b).
///
/// A card that went back to enumerating `state.players` itself would slip past
/// that, which is how Bug 0F-003 happened in the first place.
#[test]
fn every_player_targeting_trigger_leaves_targeting_to_the_engine() {
    use mtg_engine::cards::TargetRequirement;
    let reg = registry();

    let mut checked = 0;
    for name in ["Falkenrath Noble", "Bloodgift Demon", "Selhoff Occultist", "Rage Thrower"] {
        let id = reg.get_id_by_name(name).unwrap_or_else(|| panic!("unknown {name}"));
        let data = reg.card_data(id).unwrap();
        assert!(!data.triggered_abilities.is_empty(), "{name} has a trigger");
        for ability in &data.triggered_abilities {
            assert!(
                matches!(ability.target_requirement,
                    Some(TargetRequirement::PlayerOnly | TargetRequirement::PlayerOrPlaneswalker)),
                "{name}'s \"{}\" trigger must declare a player target requirement so the \
                 engine picks the target and applies player hexproof; it declares {:?}",
                ability.description, ability.target_requirement);
            checked += 1;
        }
    }
    assert!(checked >= 6, "only {checked} player-targeting triggers — this has stopped covering the set");
}

/// The behavioural half for Bloodgift Demon, whose trigger is on an upkeep
/// rather than a death: with Witchbane Orb out, the opponent is not a legal
/// target, so the Demon's controller is the only one left and the engine takes
/// it without asking.
#[test]
fn bloodgift_demon_cannot_target_a_player_with_witchbane_orb() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    named_permanent(&mut state, &reg, "Witchbane Orb", P1);
    named_permanent(&mut state, &reg, "Bloodgift Demon", P0);
    stock_library(&mut state, &reg, P0, 1);
    let their_life = state.get_player(P1).life;

    state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::Upkeep });
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    assert!(state.awaiting_action.is_none(),
        "one legal target is not a choice: {:?}", state.awaiting_action);
    assert_eq!(state.get_player(P1).life, their_life,
        "the player with hexproof was never a candidate");
    assert_eq!(state.get_player(P0).life, 19,
        "so the Demon's controller took it themselves");
    assert_eq!(
        state.objects.values().filter(|o| o.zone == Zone::Hand && o.owner == P0).count(), 1,
        "and drew the card");
}

/// Bug H (`audits/AUDIT_BUGS.md)`: Into the Maw of Hell's first-target
/// enumeration drops the `PermanentWithFilter(HasCardType(Land))` filter,
/// so creatures appear as legal first-target options.
///
/// Oracle (Into the Maw of Hell): "Destroy target land. Into the Maw of
/// Hell deals 13 damage to target creature."
///
/// Failure mode: `engine.rs` and `engine.rs`
/// (`generate_cast_actions_with_targets` and `valid_targets_for_req`'s
/// `PermanentWithFilter(_)` arms) discard the inner filter — they
/// return every battlefield permanent and rely on the card's
/// `is_valid_target` to filter. Maw of Hell's `is_valid_target` accepts
/// "land OR creature" so it can serve both target slots, so creatures
/// pass the second filter and end up as legal first-target options.
///
/// Audit log evidence: Seat 7 cast Maw of Hell intending to remove
/// opponent's Makeshift Mauler; the prompt offered Mauler as a
/// first-target option, the model picked it, and the engine then dealt
/// 13 damage to the model's *own* Rakish Heir.
#[test]
fn bug_h_maw_of_hell_first_target_must_be_a_land() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 has TWO creatures and a land in play. We need at least two
    // creatures so that the second-target enumeration has options other
    // than the creature picked as first target — without that the
    // Cartesian product (skipping t1==t2) would have no pair where
    // targets[0] is a creature.
    let bears = ready_creature(&mut state, P0, 2, 2);
    state
        .get_object_mut(bears)
        .unwrap()
        .card_types = vec![CardType::Creature];
    let _bears2 = ready_creature(&mut state, P0, 2, 2);
    state
        .get_object_mut(_bears2)
        .unwrap()
        .card_types = vec![CardType::Creature];

    let _swamp = {
        let card_id = registry.get_id_by_name("Swamp").unwrap();
        let id = state.create_object(card_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(id).unwrap().name = "Swamp".into();
        id
    };

    let maw = castable_spell(&mut state, &registry, "Into the Maw of Hell", P0);

    let legal = engine::legal_actions(&state, &registry);
    let creature_as_first_target = legal.actions.iter().any(|a| matches!(
        a,
        Action::CastSpell { object_id, targets, .. }
            if *object_id == maw
                && targets.first()
                    == Some(&Target::Object(bears))
    ));

    assert!(
        !creature_as_first_target,
        "Into the Maw of Hell's first target must be a LAND — it should \
         not be possible to pick a creature as the first target. Bug H: \
         valid_targets_for_req drops the PermanentWithFilter inner filter, \
         so creatures appear in the first-target enumeration."
    );
}

/// Bug AD (`audits/AUDIT_BUGS.md)`: Unburial Rites' `GraveyardCreature`
/// enumerator returns creature cards from any graveyard, not just the
/// caster's.
///
/// Oracle (Unburial Rites): "Return target creature card from your
/// graveyard to the battlefield. Flashback {3}{W}."
///
/// Failure mode: `engine.rs` (the `GraveyardCreature` arm of
/// `valid_targets_for_req`) iterates every object whose `zone ==
/// Graveyard`, irrespective of `obj.owner`. Unburial Rites doesn't
/// override `is_valid_target` to add an owner filter, so opponent's
/// graveyard creatures are offered as legal targets — and the
/// resolution path then reanimates them under the caster's control.
///
/// Audit log evidence: Seat 1 cast Unburial Rites at lines 30188-30189
/// and the prompt offered Selfless Cathar / Avacynian Priest with no
/// owner labels.
#[test]
fn bug_ad_unburial_rites_only_targets_casters_graveyard() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a creature in P1's graveyard.
    let opp_creature_card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
    let opp_in_yard = state.create_object(
        opp_creature_card_id,
        P1,
        Zone::Graveyard,
        Some(2),
        Some(2),
    );
    state.get_object_mut(opp_in_yard).unwrap().name = "Grizzly Bears".into();

    // Cast Unburial Rites from P0's hand.
    let rites = castable_spell(&mut state, &registry, "Unburial Rites", P0);

    let legal = engine::legal_actions(&state, &registry);
    let opp_creature_as_target = legal.actions.iter().any(|a| matches!(
        a,
        Action::CastSpell { object_id, targets, .. }
            if *object_id == rites
                && targets.iter().any(|t| matches!(t, Target::Object(id) if *id == opp_in_yard))
    ));

    assert!(
        !opp_creature_as_target,
        "Unburial Rites should only be able to target creature cards in \
         the caster's graveyard. Bug AD: GraveyardCreature returns cards \
         from every graveyard, and Unburial Rites doesn't override \
         is_valid_target to add an owner filter, so opp's creature is \
         offered as a legal target."
    );
}

/// Bug 9F-001 (`audits/AUDIT_BUGS.md)`: Snapcaster Mage excludes graveyard
/// cards whose registry data has a `flashback_cost`, refusing to grant
/// flashback to cards that already have printed flashback.
///
/// Oracle (Snapcaster Mage): "When this creature enters, target instant
/// or sorcery card in your graveyard gains flashback until end of turn.
/// The flashback cost is equal to its mana cost."
///
/// Per Scryfall ruling: "If the targeted card already has flashback, it
/// has both the old flashback cost and the new one. The card's
/// controller may choose either cost to pay when casting it."
///
/// Failure mode: `snapcaster_mage.rs` filters with
/// `... && d.flashback_cost.is_none()`. Every printed-flashback
/// instant/sorcery (Ancient Grudge, Forbidden Alchemy, Devil's Play,
/// Burning Vengeance, etc.) is silently dropped from Snapcaster's
/// eligible-target list, even though the ruling explicitly supports
/// granting flashback to a card that already has it.
///
/// This test puts an Ancient Grudge in P0's graveyard and triggers
/// Snapcaster's ETB. Snapcaster should record a temporary
/// `GrantFlashback` effect on the Ancient Grudge object; today it
/// silently drops the trigger because the eligible list is empty.
#[test]
fn bug_9f_001_snapcaster_can_grant_flashback_to_card_with_printed_flashback() {
    use mtg_engine::state::TemporaryEffect;

    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put an Ancient Grudge (instant with flashback) into P0's graveyard.
    let grudge_card_id = registry.get_id_by_name("Ancient Grudge").unwrap();
    let grudge = state.create_object(grudge_card_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(grudge).unwrap().name = "Ancient Grudge".into();

    // Snapcaster Mage enters the battlefield for P0.
    let snap_card_id = registry.get_id_by_name("Snapcaster Mage").unwrap();
    let snap = state.create_object(snap_card_id, P0, Zone::Battlefield, Some(2), Some(1));
    state.get_object_mut(snap).unwrap().name = "Snapcaster Mage".into();
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: snap, controller: P0,
    });
    mtg_engine::triggers::collect_triggers(&mut state, &registry);
    mtg_engine::triggers::resolve_next_trigger(&mut state, &registry);

    let granted = state.until_end_of_turn.iter().any(|e| matches!(
        e,
        TemporaryEffect::GrantFlashback { target, .. } if *target == grudge
    ));

    assert!(
        granted,
        "Snapcaster Mage should be able to grant flashback to Ancient \
         Grudge (a graveyard instant that already has printed flashback). \
         Bug 9F-001: snapcaster_mage.rs filters by `d.flashback_cost.is_none()`, \
         so the only eligible card is excluded and the trigger silently \
         resolves with no effect. until_end_of_turn = {:?}",
        state.until_end_of_turn,
    );
}

/// Bug AW (`audits/AUDIT_BUGS.md)`: Prey Upon's `TwoTargets` path ignores
/// the inner `YouControl` / `YouDontControl` filters, so the model can
/// pick two of its own creatures and have them fight each other.
///
/// Oracle (Prey Upon): "Target creature you control fights target
/// creature you don't control."
///
/// Failure mode: `engine.rs::valid_targets_for_req` arm
/// `Creature | CreatureWithFilter(_)` (around line 1592) drops the
/// inner filter and returns every battlefield creature, deferring to
/// `behavior.is_valid_target` for refinement. Prey Upon doesn't
/// override `is_valid_target`, so the YouControl/YouDontControl
/// constraints are never checked. The Cartesian-product `TwoTargets`
/// enumeration in `generate_cast_actions_with_targets` then emits
/// `(your, your)`, `(your, opp)`, `(opp, your)`, `(opp, opp)` pairs
/// — none of which are filtered.
#[test]
fn bug_aw_prey_upon_rejects_two_of_your_own_creatures() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Two creatures on P0's side, none on P1's. The only "creature you
    // control" + "creature you don't control" pair is impossible — but
    // the buggy enumeration produces (P0_creature, P0_creature) pairs.
    let mine_a = ready_creature(&mut state, P0, 2, 2);
    state
        .get_object_mut(mine_a)
        .unwrap()
        .card_types = vec![CardType::Creature];
    let mine_b = ready_creature(&mut state, P0, 2, 2);
    state
        .get_object_mut(mine_b)
        .unwrap()
        .card_types = vec![CardType::Creature];

    let prey = castable_spell(&mut state, &registry, "Prey Upon", P0);

    let legal = engine::legal_actions(&state, &registry);
    let two_of_yours = legal.actions.iter().any(|a| matches!(
        a,
        Action::CastSpell { object_id, targets, .. }
            if *object_id == prey
                && targets.len() == 2
                && matches!(targets[0], Target::Object(id) if id == mine_a || id == mine_b)
                && matches!(targets[1], Target::Object(id) if id == mine_a || id == mine_b)
    ));

    assert!(
        !two_of_yours,
        "Prey Upon should not allow targeting two creatures the caster \
         controls — one target must be opponent's. Bug AW: \
         valid_targets_for_req's CreatureWithFilter arm drops the inner \
         YouControl/YouDontControl filter, so any creature pair is offered."
    );
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: When Bitterheart Witch dies and the player searches for a Curse,
/// the target player choice list doesn't filter out players with hexproof
/// (from Witchbane Orb). You shouldn't be able to attach a Curse to a
/// hexproof player.
#[test]
fn bug_bitterheart_witch_hexproof_not_filtered() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Witchbane Orb for P1 (grants hexproof to P1)
    let _orb = named_permanent(&mut state, &registry, "Witchbane Orb", P1);

    // Place a Curse in P0's library
    let curse_id = registry.get_id_by_name("Curse of the Pierced Heart").unwrap();
    let curse = state.create_object(curse_id, P0, Zone::Library, None, None);
    state.get_object_mut(curse).unwrap().name = "Curse of the Pierced Heart".into();
    state.get_player_mut(P0).library_order.push(curse);

    // Place and kill Bitterheart Witch
    let witch = named_permanent(&mut state, &registry, "Bitterheart Witch", P0);
    mtg_engine::destruction::sacrifice(&mut state, witch, &registry);
    mtg_engine::sba::check_state_based_actions(&mut state, &registry);
    mtg_engine::triggers::process_triggers(&mut state, &registry);

    // If there's a yes/no choice, accept it
    if let Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
        choice: mtg_engine::state::ResolutionChoiceKind::YesNo { .. }, ..
    }) = &state.awaiting_action {
        let action = Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::YesNoDecision(true),
        };
        state = engine::submit_action(&state, &action, &registry);
    }

    // Check if the player choice includes P1 (who has hexproof)
    let p1_targetable = state.awaiting_action.as_ref().is_some_and(|aa| {
        format!("{aa:?}").contains(&"Player(PlayerId(1))".to_string())
    });

    // BUG: P1 with hexproof (from Witchbane Orb) is still a valid target
    assert!(!p1_targetable,
        "Player with hexproof should not be targetable for Curse attachment");
}

/// Slayer of the Wicked and Geistcatcher's Rig each enumerated their ETB
/// target inline — walking the battlefield, filtering on subtype or keyword,
/// and offering the result. That path knows nothing about hexproof, so an
/// opponent's hexproof Zombie was offered as a legal target (CR 702.11b).
///
/// Both now declare a `target_requirement`, which routes the enumeration
/// through the engine, where hexproof is filtered once for every card. Found
/// by `card_data_invariants.rs`'s sweep over targeting triggers, which the
/// hand-listed four-card version of that test never reached.
#[test]
fn an_etb_trigger_does_not_offer_an_opponents_hexproof_creature() {
    let reg = registry();

    // (source card, a creature it could otherwise target, keyword it needs)
    let cases: &[(&str, &str, Option<Keyword>)] = &[
        ("Slayer of the Wicked", "Diregraf Ghoul", None),
        ("Geistcatcher's Rig", "Abbey Griffin", Some(Keyword::Flying)),
    ];

    for (source_name, victim_name, needs) in cases {
        // Control: without hexproof, the victim IS offered — otherwise this
        // test would pass just as well for a card that targets nothing.
        for hexproof in [false, true] {
            let mut state = game_at_step(Step::PrecombatMain, P0);
            let victim = named_permanent(&mut state, &reg, victim_name, P1);
            if let Some(kw) = needs {
                assert!(state.has_keyword(victim, *kw, &reg),
                    "test precondition: {victim_name} has {kw:?}");
            }
            if hexproof {
                state.until_end_of_turn.push(
                    mtg_engine::state::TemporaryEffect::GrantKeyword {
                        target: victim,
                        keyword: Keyword::Hexproof,
                    },
                );
            }

            let source = named_permanent(&mut state, &reg, source_name, P0);
            state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
                object: source,
                controller: P0,
            });
            mtg_engine::triggers::collect_triggers(&mut state, &reg);

            // With one legal target the engine auto-picks it as the trigger
            // goes on the stack rather than prompting (CR 603.3d), so read the
            // trigger's locked target — and the pending prompt when there is
            // more than one.
            let offered = state.stack.iter().any(|e| matches!(e,
                mtg_engine::state::StackEntry::Trigger(t)
                    if t.source.chosen_targets.contains(&Target::Object(victim))))
                || matches!(&state.awaiting_action,
                    Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
                        choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. }, ..
                    }) if options.contains(&Target::Object(victim)));
            assert_eq!(offered, !hexproof,
                "{source_name} should {} offer an opponent's {}{victim_name}",
                if hexproof { "not" } else { "" },
                if hexproof { "hexproof " } else { "" });
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// One rule, one place
// ─────────────────────────────────────────────────────────────────
//
// "May this player be targeted by this caster" used to be written out three
// times: `targeting::can_target_player`, again inline in `stack.rs`'s CR
// 608.2b re-check, and again in `helpers::any_targets` /
// `any_targets_except`. Only the callers of the first also filtered out a
// player who had lost, and they each did it themselves. The rule now lives in
// `can_target_player` alone; these two cases are what the divergence let
// through.

/// CR 104.3a: a player who has lost has left the game, so nothing may target
/// them. `helpers::any_targets` — the enumerator behind every "any target"
/// card — walked `state.players` and asked only about hexproof.
#[test]
fn a_player_who_has_lost_is_not_an_any_target() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let source = named_permanent(&mut state, &reg, "Witchbane Orb", P0);

    let before = mtg_engine::cards::helpers::any_targets(&state, source, P0, &reg);
    assert!(before.contains(&Target::Player(P1)),
        "test precondition: the opponent is an 'any target' while still in the game");

    state.get_player_mut(P1).lost = true;

    let after = mtg_engine::cards::helpers::any_targets(&state, source, P0, &reg);
    assert!(!after.contains(&Target::Player(P1)),
        "a player who has left the game is not there to be targeted");
    assert!(after.contains(&Target::Player(P0)),
        "and the caster is still a legal target for their own spell");
}

/// The same rule on the way down. CR 608.2b re-checks each target as the spell
/// resolves, and that check has to be the one that offered the target — a
/// hexproof player who was never offerable must not become legal just because
/// the re-check restated the rule for itself.
#[test]
fn the_resolution_recheck_uses_the_same_player_targeting_rule() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Witchbane Orb", P1);

    // Bump in the Night: "Target player loses 3 life." P0 casts it at P1, who
    // has hexproof — the engine would never offer this, so drive it directly.
    let bump = castable_spell(&mut state, &reg, "Bump in the Night", P0);
    let mut state = cast_onto_stack(&state, &reg, bump, vec![Target::Player(P1)]);
    let their_life = state.get_player(P1).life;
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, their_life,
        "the only target is illegal on resolution, so the spell is countered \
         by game rules (CR 608.2b) and no life is lost");
    assert_eq!(state.get_object(bump).unwrap().zone, Zone::Graveyard,
        "a countered spell still goes to the graveyard");
}

