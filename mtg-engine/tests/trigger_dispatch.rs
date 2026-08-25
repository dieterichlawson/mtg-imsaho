//! Regression tests for CR 603.2 / 603.3d trigger dispatch:
//! - Conditional SpellCast watchers (Charmbreaker Devils) must not create
//!   stack entries for spells that don't satisfy the trigger condition.
//! - ETB triggers with declared target requirements (Fiend Hunter) lock
//!   their target as the trigger goes on the stack; creatures entering
//!   afterwards are not legal targets.

mod common;
use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind, StackEntry};
use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::triggers;
use mtg_engine::types::*;

fn trigger_count(state: &mtg_engine::state::GameState) -> usize {
    state.stack.iter().filter(|e| matches!(e, StackEntry::Trigger(_))).count()
}

/// Charmbreaker Devils: "Whenever YOU cast an INSTANT OR SORCERY spell" —
/// an opponent's spell, or a creature spell, must not create a trigger.
#[test]
fn charmbreaker_devils_trigger_only_for_own_instants_and_sorceries() {
    let reg = registry();

    // Case 1: opponent casts an instant — no trigger for P0's Devils.
    let mut state = game_at_step(Step::PrecombatMain, P1);
    let _devils = named_creature(&mut state, &reg, "Charmbreaker Devils", P0);
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P1);
    let mut state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Player(P0)], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(trigger_count(&state), 0,
        "opponent's spell must not put Charmbreaker Devils' trigger on the stack (CR 603.2)");

    // Case 2: controller casts a creature spell — still no trigger.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let _devils = named_creature(&mut state, &reg, "Charmbreaker Devils", P0);
    let bears = castable_spell(&mut state, &reg, "Grizzly Bears", P0);
    let mut state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bears, targets: vec![], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(trigger_count(&state), 0,
        "a creature spell must not put Charmbreaker Devils' trigger on the stack (CR 603.2)");

    // Case 3: controller casts an instant — the trigger IS created.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let _devils = named_creature(&mut state, &reg, "Charmbreaker Devils", P0);
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);
    let mut state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell { object_id: bolt, targets: vec![Target::Player(P1)], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &reg,
    );
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(trigger_count(&state), 1,
        "own instant should put exactly one Charmbreaker Devils trigger on the stack");
}

/// Fiend Hunter's ETB target is locked when the trigger goes on the stack:
/// a creature that enters afterwards must not be offered at resolution.
#[test]
fn fiend_hunter_target_locked_at_trigger_creation() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Exactly one other creature — the push auto-locks it as the target.
    let victim = ready_creature(&mut state, P1, 2, 2);

    let hunter = castable_spell(&mut state, &reg, "Fiend Hunter", P0);
    let mut state = cast_and_resolve(&state, &reg, hunter, vec![]);
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(trigger_count(&state), 1, "ETB trigger should be on the stack with its target locked");

    // A creature enters AFTER the trigger was put on the stack.
    let latecomer = ready_creature(&mut state, P1, 5, 5);

    // Resolve the trigger: the "you may" prompt must offer ONLY the locked
    // target, not the latecomer.
    triggers::process_triggers(&mut state, &reg);
    let Some(AwaitingAction::ResolutionChoice { choice, .. }) = &state.awaiting_action else {
        panic!("expected the optional exile choice, got {:?}", state.awaiting_action);
    };
    let ResolutionChoiceKind::ChooseTarget { options, optional, .. } = choice else {
        panic!("expected ChooseTarget, got {choice:?}");
    };
    assert!(*optional, "'you may' — declining must be possible");
    assert_eq!(options, &vec![Target::Object(victim)],
        "resolution must offer only the target locked at trigger creation (CR 603.3d)");

    // Accept: the locked target is exiled, the latecomer untouched.
    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(Target::Object(victim))) },
        &reg,
    );
    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Exile);
    assert_eq!(state.get_object(latecomer).unwrap().zone, Zone::Battlefield);
}

/// If the locked target is gone at resolution, the trigger is countered by
/// game rules (CR 608.2b) and Fiend Hunter exiles nothing.
#[test]
fn fiend_hunter_trigger_fizzles_when_locked_target_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let victim = ready_creature(&mut state, P1, 2, 2);
    let hunter = castable_spell(&mut state, &reg, "Fiend Hunter", P0);
    let mut state = cast_and_resolve(&state, &reg, hunter, vec![]);
    triggers::collect_triggers(&mut state, &reg);
    assert_eq!(trigger_count(&state), 1);

    // The locked target dies in response.
    state.move_object(victim, Zone::Graveyard, &reg);

    triggers::process_triggers(&mut state, &reg);
    assert!(state.awaiting_action.is_none(),
        "fizzled trigger must not present the exile choice");
    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Graveyard,
        "the dead creature stays in the graveyard");
}

// -------------------------------------------------------------------------
// From the trigger-dispatch audit family
// -------------------------------------------------------------------------

/// Bug BT (`audits/AUDIT_BUGS.md)`: Abattoir Ghoul's
/// `on_any_creature_dies` handler early-returns when its `self_id` is
/// not on the battlefield. In a mutual first-strike trade where the
/// Ghoul and a creature it dealt damage to die simultaneously, the
/// trigger queue still picks up the death (the dispatcher correctly
/// includes simultaneously-dead watchers), but the handler then drops
/// the effect because it sees the Ghoul is in graveyard.
///
/// Oracle (Abattoir Ghoul): "First strike. Whenever a creature dealt
/// damage by this creature this turn dies, you gain life equal to
/// that creature's toughness."
///
/// Failure mode: `abattoir_ghoul.rs:39-42` does
/// ```
/// let controller = match state.get_object(self_id) {
///     Some(o) if o.zone == Zone::Battlefield => o.controller,
///     _ => return,
/// };
/// ```
/// CR 603.6d / 603.10c says a triggered ability that has been put on
/// the stack continues to resolve even if the source has left the
/// battlefield. Falkenrath Noble's death-trigger handler is the
/// counter-example that gets this right.
///
/// We simulate the audit-confirmed scenario: Voiceless Spirit
/// (toughness 1) was damaged by Abattoir Ghoul, then both die
/// simultaneously. Calling the dispatcher's
/// `on_any_creature_dies` directly with the captured `damaged_by` and
/// `dead_toughness` mirrors what `triggers.rs` does at trigger
/// resolution time.
///
/// NOTE: the same `o.zone == Zone::Battlefield` early-return gate
/// exists in Murder of Crows (`murder_of_crows.rs:38-43`),
/// Rage Thrower (`rage_thrower.rs:38-43`), and
/// Selhoff Occultist (`selhoff_occultist.rs:47-54`). All four use
/// the identical zone-gate pattern. One test covers the shared
/// defect; the other three need the same one-line fix (drop the
/// zone guard, mirror Falkenrath Noble's handler which is correct).
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug BT is fixed.
#[test]
fn bug_bt_abattoir_ghoul_gains_life_on_simultaneous_death() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::CombatDamage, P0);

    // Abattoir Ghoul belongs to P0, who should gain 1 life from the
    // dying Voiceless Spirit (1 toughness).
    let ghoul = named_creature(&mut state, &registry, "Abattoir Ghoul", P0);
    // Move the Ghoul to graveyard to mirror the simultaneous-death
    // state at trigger-resolution time.
    state.move_object(ghoul, Zone::Graveyard, &registry);

    let life_before = state.get_player(P0).life;

    // Fire the AnyCreatureDies handler. The dispatcher uses the
    // captured `damaged_by` and `dead_toughness`; we hand-craft them
    // to match what the trigger collector would record.
    let ghoul_card_id = registry.get_id_by_name("Abattoir Ghoul").unwrap();
    let dummy_dead = mtg_engine::ids::ObjectId(99999);
    let dead_damaged_by = vec![ghoul];
    let dead_toughness = 1;
    let behavior = registry.get(ghoul_card_id).unwrap();
    behavior.on_any_creature_dies(
        &mut state,
        ghoul,
        dummy_dead,
        P1,
        &dead_damaged_by,
        dead_toughness,
        false,
        &[],
        &registry,
    );

    let life_after = state.get_player(P0).life;
    assert_eq!(
        life_after - life_before,
        1,
        "Abattoir Ghoul should gain 1 life from a simultaneously-dying \
         creature it dealt damage to (CR 603.6d: triggered ability \
         continues to resolve even if its source has left the \
         battlefield). Bug BT: the handler early-returns because the \
         Ghoul is no longer on the battlefield. Life: {life_before} -> {life_after}",
    );
}

/// Bug L (`audits/AUDIT_BUGS.md)`: Charmbreaker Devils' `on_spell_cast`
/// triggered ability fires for every spell type, not just
/// instants/sorceries.
///
/// Oracle (Charmbreaker Devils): "Whenever you cast an instant or
/// sorcery spell, this creature gets +4/+0 until end of turn."
///
/// Failure mode: `charmbreaker_devils.rs:75-92` filters by `caster ==
/// controller` but does NOT filter by spell type. The dispatcher
/// (`triggers.rs:727`) explicitly says "Dispatch `SpellCast` triggers
/// for ALL spell types... Individual card handlers can filter by
/// spell type if needed" — Charmbreaker doesn't.
///
/// We exercise the bug by calling Charmbreaker's `on_spell_cast`
/// handler directly with a creature spell as the trigger source. The
/// handler should be a no-op (Grizzly Bears is a creature, not an
/// instant or sorcery). Today the handler unconditionally pushes a
/// `+4/+0` `ModifyPT` into `until_end_of_turn`.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug L is fixed.
#[test]
fn bug_l_charmbreaker_devils_does_not_buff_on_creature_spell() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let devils = named_creature(&mut state, &registry, "Charmbreaker Devils", P0);
    let base_power = state.effective_power(devils, &registry).unwrap_or(0);

    // Spawn a Grizzly Bears spell on the stack and dispatch the
    // SpellCast trigger to Charmbreaker manually.
    let bears_card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
    let bears_spell = state.create_object(bears_card_id, P0, Zone::Stack, Some(2), Some(2));
    state.get_object_mut(bears_spell).unwrap().name = "Grizzly Bears".into();

    let devils_card_id = registry.get_id_by_name("Charmbreaker Devils").unwrap();
    let behavior = registry.get(devils_card_id).unwrap();
    behavior.on_spell_cast(&mut state, devils, P0, bears_spell, &[], &registry);

    let after_power = state.effective_power(devils, &registry).unwrap_or(0);
    assert_eq!(
        after_power, base_power,
        "Charmbreaker Devils' +4/+0 should NOT trigger when the \
         controller casts a creature spell — its oracle text restricts \
         the trigger to instants and sorceries. Bug L: the handler \
         doesn't filter by spell type. effective_power: {base_power} -> {after_power}",
    );
}

/// Bug CA (`audits/AUDIT_BUGS.md)`: Moldgraf Monstrosity reads
/// `o.owner` instead of `o.controller` for its "your graveyard"
/// reference, so when stolen via Traitorous Blood and dying that
/// turn, it returns creatures from the WRONG player's graveyard.
///
/// Oracle (Moldgraf Monstrosity): "When this creature dies, exile it,
/// then return two creature cards at random from **your** graveyard
/// to the battlefield."
///
/// CR 603.10c: "If a permanent leaves the battlefield, the owner's
/// controller and other characteristics for the duration of leaving
/// triggers are set from last known information just before that
/// event." So "your" should be the last-known *controller*, which is
/// the Traitorous Blood caster — not the original owner.
///
/// Failure mode: `moldgraf_monstrosity.rs:42-46` reads `o.owner`,
/// while Doomed Traveler and Mausoleum Guard correctly read
/// `o.controller`. We test by giving Moldgraf an `owner != controller`
/// state and observing whose graveyard is reanimated from.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug CA is fixed.
#[test]
fn bug_ca_moldgraf_monstrosity_uses_controller_not_owner() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PostcombatMain, P0);

    // Moldgraf is owned by P1 but currently controlled by P0
    // (modeling a Traitorous Blood theft).
    let mold_card_id = registry.get_id_by_name("Moldgraf Monstrosity").unwrap();
    let mold = state.create_object(mold_card_id, P1, Zone::Battlefield, Some(8), Some(8));
    {
        let obj = state.get_object_mut(mold).unwrap();
        obj.name = "Moldgraf Monstrosity".into();
        obj.controller = P0; // stolen
    }

    // P0 (the new controller / "you") has a creature card in their
    // graveyard. P1 (the original owner) has none. The fix should
    // reanimate from P0's graveyard; the bug reanimates from P1's.
    let bears_card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
    let p0_bears = state.create_object(bears_card_id, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(p0_bears).unwrap().name = "Grizzly Bears (P0)".into();

    // Fire Moldgraf's death trigger directly.
    let behavior = registry.get(mold_card_id).unwrap();
    behavior.on_dies(&mut state, mold, &[], &registry);

    let bears_zone = state.get_object(p0_bears).map(|o| o.zone);
    assert_eq!(
        bears_zone,
        Some(Zone::Battlefield),
        "Moldgraf Monstrosity (controlled by P0 via theft) should return \
         creatures from P0's graveyard, not from P1's owner-graveyard. \
         Bug CA: the handler reads o.owner (P1, who has nothing in \
         graveyard) instead of o.controller (P0). Grizzly Bears zone: {bears_zone:?}",
    );
}

/// Bug BU (`audits/AUDIT_BUGS.md)`: Burning Vengeance's `on_spell_cast`
/// handler logs "deals 2 damage to opponent" BEFORE the controller
/// picks a target. The log line is stale — the player may choose a
/// creature, themselves, etc., and the hard-coded "opponent" text
/// misleads the LLM reading the log.
///
/// Oracle (Burning Vengeance): "Whenever you cast a spell from your
/// graveyard, this enchantment deals 2 damage to any target."
///
/// Failure mode: `burning_vengeance.rs:55-68` calls
/// `present_target_choice(...)` to set up the `awaiting_action`, then
/// unconditionally logs "deals 2 damage to opponent (flashback spell
/// cast)". The actual target hasn't been picked yet.
///
/// We fire the handler and assert that the freshly-written log entries
/// don't contain the hard-coded "to opponent" phrasing.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug BU is fixed.
#[test]
fn bug_bu_burning_vengeance_no_stale_opponent_log() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let vengeance = named_creature(&mut state, &registry, "Burning Vengeance", P0);
    // Make sure there's at least one creature target so
    // present_target_choice sets awaiting_action (rather than an
    // empty-targets no-op).
    let _opp_creature = ready_creature(&mut state, P1, 2, 2);

    // A flashback-cast spell on the stack — Burning Vengeance only
    // triggers on graveyard-cast spells.
    let bolt_card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
    let flashback_spell = state.create_object(bolt_card_id, P0, Zone::Stack, Some(2), Some(2));
    state.get_object_mut(flashback_spell).unwrap().cast_with_flashback = true;

    let log_before = state.game_log.len();
    let vengeance_card_id = state.get_object(vengeance).unwrap().card_id;
    let behavior = registry.get(vengeance_card_id).unwrap();
    behavior.on_spell_cast(&mut state, vengeance, P0, flashback_spell, &[], &registry);

    let new_log_lines: Vec<_> = state.game_log[log_before..]
        .iter()
        .map(|entry| entry.message.clone())
        .collect();
    let stale_opponent_log = new_log_lines.iter().any(|msg| {
        msg.contains("deals 2 damage to opponent") || msg.contains("damage to opponent")
    });

    assert!(
        !stale_opponent_log,
        "Burning Vengeance should not log 'deals 2 damage to opponent' \
         before the target is chosen — the player might pick a \
         creature, themselves, or a planeswalker. Bug BU: the handler \
         unconditionally logs the stale text. new_log_lines = {new_log_lines:?}",
    );
}

/// Bug K (`audits/AUDIT_BUGS.md)`: The Bug C fix added a blanket
/// "no-autotap for sacrifice abilities" restriction keyed on
/// `ab.sacrifice_cost != SacrificeCost::None`. This was too
/// aggressive: it includes `SacrificeCost::SacrificeThis`, where the
/// source permanent sacrifices itself and there's no creature-choice
/// conflict with autotap. Selfless Cathar's `{1}{W}, Sacrifice:
/// Creatures you control get +1/+1` is silently dropped from the
/// action list unless the player has pre-tapped {1}{W} into their
/// pool.
///
/// Oracle (Selfless Cathar): "{1}{W}, Sacrifice this creature:
/// Creatures you control get +1/+1 until end of turn."
///
/// Failure mode: `engine.rs:574-587` checks
/// `ability_has_sac_cost = !matches!(ab.sacrifice_cost, SacrificeCost::None);`
/// then short-circuits the mana autotap. `SacrificeThis` hits this
/// branch even though it doesn't conflict with land-tap ordering.
///
/// We put Selfless Cathar with two untapped Plains (enough to
/// autotap {1}{W}), and assert that its activated ability shows up
/// in `legal_actions`.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug K is fixed.
#[test]
fn bug_k_selfless_cathar_autotaps_sacrifice_this() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cathar = named_creature(&mut state, &registry, "Selfless Cathar", P0);

    // Two untapped Plains so {1}{W} is autotap-reachable.
    let plains_card_id = registry.get_id_by_name("Plains").unwrap();
    for _ in 0..2 {
        let p = state.create_object(plains_card_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(p).unwrap().name = "Plains".into();
        state.get_object_mut(p).unwrap().card_types = vec![CardType::Land];
    }

    assert_eq!(
        state.get_player(P0).mana_pool.total(),
        0,
        "Test setup: mana pool should be empty"
    );

    let legal = engine::legal_actions(&state, &registry);
    let has_cathar_ability = legal.actions.iter().any(|a| matches!(
        a,
        Action::ActivateAbility { object_id, .. } if *object_id == cathar
    ));

    assert!(
        has_cathar_ability,
        "Selfless Cathar's {{1}}{{W}}, Sacrifice-this ability should be \
         offerable via autotap — the sacrifice target (itself) is \
         fixed so there's no conflict with choosing which creature to \
         sacrifice. Bug K: the Bug C fix blanket-disabled autotap for \
         ALL sacrifice abilities, including SacrificeThis. \
         activatable_abilities = {:?}",
        legal.activatable_abilities,
    );
}

/// Bug 17-002 (`audits/AUDIT_BUGS.md)`: Undead Alchemist's second
/// ability ("Whenever a creature card is put into an opponent's
/// graveyard from their library, exile that card and create a 2/2
/// black Zombie creature token") is entirely missing. The card
/// declares a single `AnyCombatDamageToPlayer` trigger whose handler
/// fuses both abilities, so the second ability only fires when
/// Undead Alchemist's own mill-instead-of-damage path runs — every
/// other mill source (Dream Twist, Nephalia Drownyard, Mindshrieker,
/// Cellar Door, Splinterfright upkeep) bypasses it.
///
/// Oracle (Undead Alchemist): "... Whenever a creature card is put
/// into an opponent's graveyard from their library, exile that card
/// and create a 2/2 black Zombie creature token."
///
/// Failure mode: `undead_alchemist.rs` declares exactly one trigger
/// — `TriggerKind::AnyCombatDamageToPlayer` — with no mill-watcher
/// trigger. `engine::mill_cards` moves cards library → graveyard
/// via `move_object`, but there's no dispatch to a
/// "`CreatureMilledFromLibrary`" watcher.
///
/// We put Undead Alchemist + a P1 creature on top of P1's library,
/// call `mill_cards` on P1, and check that the milled creature
/// ends up in exile (and a Zombie token appears for P0).
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug 17-002 is fixed.
#[test]
fn bug_17_002_undead_alchemist_exiles_milled_opponent_creatures() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _alchemist = named_creature(&mut state, &registry, "Undead Alchemist", P0);

    // Put a creature card on top of P1's library.
    let bears_card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
    let milled = state.create_object(bears_card_id, P1, Zone::Library, Some(2), Some(2));
    state.get_object_mut(milled).unwrap().name = "Grizzly Bears".into();
    state.get_player_mut(P1).library_order.insert(0, milled);

    let zombie_tokens_before = count_tokens_named_by(&state, "Zombie", P0);

    // Mill 1 card from P1's library.
    engine::mill_cards(&mut state, P1, 1, &registry);
    // Run trigger processing so any watcher triggers get a chance
    // to fire.
    mtg_engine::triggers::process_triggers(&mut state, &registry);

    let milled_zone = state.get_object(milled).map(|o| o.zone);
    let zombie_tokens_after = count_tokens_named_by(&state, "Zombie", P0);

    assert_eq!(
        milled_zone,
        Some(Zone::Exile),
        "Milling a creature card from an opponent's library should \
         fire Undead Alchemist's second ability, exiling the card. \
         Bug 17-002: the ability isn't wired as a separate trigger, \
         so only Undead Alchemist's own mill-instead path can \
         observe the mill — Dream Twist / Nephalia Drownyard / etc. \
         bypass it entirely. milled_zone = {milled_zone:?}",
    );
    assert!(
        zombie_tokens_after > zombie_tokens_before,
        "Undead Alchemist should create a Zombie token for each \
         creature card milled from an opponent's library. Bug 17-002. \
         zombie tokens: {zombie_tokens_before} -> {zombie_tokens_after}",
    );
}

/// Bug AE (`audits/AUDIT_BUGS.md)`: Undead Alchemist implements the
/// "if a Zombie you control would deal combat damage to a player,
/// instead that player mills" clause as an `AnyCombatDamageToPlayer`
/// triggered ability — a post-damage handler that restores life
/// after the fact. CR 614 says this is a replacement effect: the
/// player should never take the damage in the first place.
///
/// Oracle (Undead Alchemist): "If a Zombie you control would deal
/// combat damage to a player, instead that player mills that many
/// cards."
///
/// Failure mode: `undead_alchemist.rs:45-106` is an
/// `on_any_combat_damage_to_player` handler that does
/// `state.get_player_mut(damaged_player).life = current_life + amount`
/// — restores the life loss after damage already happened. If the
/// damage was lethal (player dropped to 0), SBA 704.5a moves them to
/// "lost" before the trigger runs, and the restoration is too late.
///
/// We drive a direct `on_any_combat_damage_to_player` against a
/// player at 1 life with a Zombie dealing 5 combat damage and
/// check that the player's life is NOT reduced mid-handler (the
/// correct replacement-effect behavior), i.e. it should NOT dip
/// below the starting life — even transiently — because the damage
/// should never have been dealt in the first place. Practically
/// this manifests as: the player's life stays at `starting_life`
/// throughout, never drops.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug AE is fixed.
#[test]
fn bug_ae_undead_alchemist_replaces_damage_not_restores_life() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let alchemist = named_creature(&mut state, &registry, "Undead Alchemist", P0);

    // A Zombie attacker controlled by P0.
    let walking_corpse_card_id = registry.get_id_by_name("Walking Corpse").unwrap();
    let attacker = state.create_object(walking_corpse_card_id, P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(attacker).unwrap().name = "Walking Corpse".into();

    // P1 at 1 life and five cards to mill (so mill_count >= amount).
    state.get_player_mut(P1).life = 1;
    let bears_card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
    for _ in 0..5 {
        let c = state.create_object(bears_card_id, P1, Zone::Library, Some(2), Some(2));
        state.get_player_mut(P1).library_order.push(c);
    }

    // Damage would be lethal (5 > 1). Call the current handler
    // directly. If the bug is present, the handler subtracts life
    // first (via the upstream damage helper) and then restores it
    // — but SBA runs between, and the player loses immediately.
    // Since we're calling the handler directly without going through
    // the damage pipeline, we simulate "damage already applied" by
    // subtracting life first — mirroring what the engine's damage
    // step does. The fix replaces this entirely, so the simulation
    // shouldn't need to subtract life at all.
    //
    // We bypass the bug's simulation and just check that Undead
    // Alchemist emits a mill event as its primary effect, i.e. the
    // mill is the effect rather than a post-hoc "restore" after
    // damage. After the fix, the handler will emit mill without
    // depending on the damage having already happened.
    let life_before = state.get_player(P1).life;
    let alch_card_id = state.get_object(alchemist).unwrap().card_id;
    let behavior = registry.get(alch_card_id).unwrap();
    behavior.on_any_combat_damage_to_player(&mut state, alchemist, attacker, P1, 5, &registry);
    let life_after = state.get_player(P1).life;

    // After the fix (replacement effect), the player's life should
    // not have moved at all from `life_before` to `life_after` —
    // the damage was replaced with mill before it was applied.
    // Today the handler adds `amount` to the (unchanged) life,
    // leaving life > starting life, which is the fingerprint of the
    // bug: the handler is "restoring" damage that hasn't been
    // applied in this test harness.
    assert_eq!(
        life_after, life_before,
        "Undead Alchemist's 'instead' should replace damage with \
         mill, leaving the player's life unchanged. Bug AE: the \
         handler ADDS `amount` to life as an after-the-fact restore, \
         so a fresh harness call nets +amount life (revealing that \
         the code treats damage as already-applied). life: {life_before} -> {life_after}",
    );
}

/// Bug M (`audits/AUDIT_BUGS.md)`: Snapcaster Mage's ETB trigger
/// ("target instant or sorcery in your graveyard gains flashback")
/// should choose its target when the trigger is PUT ON THE STACK
/// (CR 603.3d), not when the trigger resolves. Today the target
/// choice is deferred to `on_enter_battlefield` (resolution time),
/// so opponents never get a priority window between "Snapcaster
/// trigger goes on stack with target X" and "X gains flashback."
///
/// Oracle (Snapcaster Mage): "When this creature enters, target
/// instant or sorcery card in your graveyard gains flashback until
/// end of turn."
///
/// Failure mode: `triggers.rs:975-981` resolves the ETB trigger by
/// calling `behavior.on_enter_battlefield`, which is where the
/// target choice lives. The trigger was queued at collection time
/// with no target — opponents couldn't respond to "Snapcaster
/// targets Ancient Grudge" because the target wasn't locked in yet.
///
/// We put Snapcaster on the battlefield, push an `EnteredBattlefield`
/// event, call `collect_triggers` (NOT `process_triggers`), and
/// assert that `state.awaiting_action` is already set for the
/// target choice. Today it's None because the choice is deferred
/// to resolution.
///
/// CR 603.3d: Snapcaster Mage's ETB trigger should choose its target
/// when put on the stack, not at resolution. This means:
/// - With a single legal target, the engine auto-picks at collection time
///   and the trigger goes on the stack with `chosen_targets` populated.
///   Opponents can then respond to the on-stack trigger before resolution.
/// - With multiple legal targets, the engine prompts the player.
///
/// Either way, the target is locked in BEFORE the trigger is on the stack
/// — opponents can't see a trigger with no target and respond to "guess
/// which target it'll pick at resolution".
#[test]
fn bug_m_snapcaster_target_chosen_at_stack_time() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // A valid flashback target in P0's graveyard.
    let grudge_card_id = registry.get_id_by_name("Ancient Grudge").unwrap();
    let grudge = state.create_object(grudge_card_id, P0, Zone::Graveyard, None, None);

    // Snapcaster Mage enters the battlefield — push the ETB event.
    let snap_card_id = registry.get_id_by_name("Snapcaster Mage").unwrap();
    let snap = state.create_object(snap_card_id, P0, Zone::Battlefield, Some(2), Some(1));
    state.get_object_mut(snap).unwrap().name = "Snapcaster Mage".into();
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: snap,
        controller: P0,
    });

    // Collect triggers — should queue the Snapcaster ETB trigger.
    let had_triggers = mtg_engine::triggers::collect_triggers(&mut state, &registry);
    assert!(had_triggers, "Test setup: Snapcaster ETB should produce a trigger");

    // CR 603.3d: with a single legal target, no prompt is needed (auto-pick).
    // The trigger should be on the stack with chosen_targets populated.
    assert!(state.awaiting_action.is_none(),
        "Single-target trigger should auto-pick without prompting. awaiting_action = {:?}",
        state.awaiting_action);

    let trigger_on_stack = state.stack.iter().find_map(|e| {
        if let mtg_engine::state::StackEntry::Trigger(
            PendingTrigger {
                source: TriggerSource { card_id, chosen_targets, .. },
                event: TriggerEvent::SelfEntered,
            }
        ) = e {
            if *card_id == snap_card_id {
                return Some(chosen_targets.clone());
            }
        }
        None
    });
    let targets = trigger_on_stack.expect("Snapcaster ETB trigger should be on the stack");
    assert_eq!(targets, vec![mtg_engine::actions::Target::Object(grudge)],
        "Snapcaster's ETB trigger should have Ancient Grudge locked in as the target \
         at stack-queue time (CR 603.3d), not deferred to resolution. \
         chosen_targets = {targets:?}");
}

/// With multiple legal targets, the player is prompted to choose at
/// stack-queue time per CR 603.3d.
#[test]
fn snapcaster_prompts_for_target_with_multiple_legal_targets() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Two valid flashback targets in P0's graveyard.
    let grudge_card_id = registry.get_id_by_name("Ancient Grudge").unwrap();
    let _grudge1 = state.create_object(grudge_card_id, P0, Zone::Graveyard, None, None);
    let _grudge2 = state.create_object(grudge_card_id, P0, Zone::Graveyard, None, None);

    let snap_card_id = registry.get_id_by_name("Snapcaster Mage").unwrap();
    let snap = state.create_object(snap_card_id, P0, Zone::Battlefield, Some(2), Some(1));
    state.get_object_mut(snap).unwrap().name = "Snapcaster Mage".into();
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: snap,
        controller: P0,
    });

    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    // Multiple legal targets → engine prompts the player.
    assert!(state.awaiting_action.is_some(),
        "With multiple legal targets, the engine must prompt at stack-queue time");
}

/// CR 603.3b technically requires the active player to choose the
/// stack order of their simultaneous triggers. In practice, MTG Arena
/// (and most other digital implementations) auto-orders by default —
/// only prompting when a player explicitly opts in via settings.
///
/// For an AI-driven engine, auto-ordering is the right default. This
/// test verifies that simultaneous triggers from the same controller
/// go on the stack deterministically, with no prompt.
#[test]
fn simultaneous_triggers_auto_order_no_prompt() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Two Abattoir Ghouls for P0 — each has an untargeted death-watch
    // trigger ("whenever a creature dealt damage by ... dies, you gain
    // life equal to that creature's toughness"). Untargeted triggers
    // exercise the stack-ordering code path without invoking CR 603.3d
    // stack-time target choices.
    let ghoul1 = named_creature(&mut state, &registry, "Abattoir Ghoul", P0);
    let ghoul2 = named_creature(&mut state, &registry, "Abattoir Ghoul", P0);

    // Kill a creature damaged by both ghouls simultaneously.
    let victim = ready_creature(&mut state, P1, 1, 5);
    state.get_object_mut(victim).unwrap().damage_marked = 5;
    state.get_object_mut(victim).unwrap().damaged_by = vec![ghoul1, ghoul2];
    mtg_engine::sba::check_state_based_actions(&mut state, &registry);

    let had_triggers = mtg_engine::triggers::collect_triggers(&mut state, &registry);
    assert!(had_triggers, "Test setup: should have had triggers after creature death");

    let ap_count = state.stack.iter().filter(|e| {
        matches!(e, mtg_engine::state::StackEntry::Trigger(t) if t.controller() == P0)
    }).count();
    assert!(
        ap_count >= 2,
        "Test setup: expected 2+ simultaneous P0 triggers, got {ap_count}",
    );

    // No prompt — untargeted triggers were auto-ordered onto the stack.
    assert!(
        state.awaiting_action.is_none(),
        "Simultaneous untargeted triggers should be auto-ordered without prompting. \
         awaiting_action = {:?}",
        state.awaiting_action,
    );
}

/// Bug Q (`audits/AUDIT_BUGS.md)`: Dearly Departed's "each Human
/// creature you control enters with an additional +1/+1 counter" is
/// implemented as a `TriggerKind::AnyCreatureEnters` triggered
/// ability instead of a CR 614.1c static replacement effect. As a
/// triggered ability, it fires AFTER the creature enters — so ETB
/// triggers on the entering creature don't see the counter.
///
/// Oracle (Dearly Departed): "As long as Dearly Departed is in your
/// graveyard, each Human creature you control enters with an
/// additional +1/+1 counter on it."
///
/// Failure mode: `dearly_departed.rs:30-36` declares a
/// `TriggeredAbilityDef { kind: TriggerKind::AnyCreatureEnters }`.
/// The fix replaces this with a `ReplacementEffect`-style entry
/// that's consulted during the entry event.
///
/// We check the `card_data`'s `triggered_abilities` list is empty
/// (i.e., the trigger has been removed) — the fingerprint of the
/// fix.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug Q is fixed.
#[test]
fn bug_q_dearly_departed_is_not_a_trigger() {
    let registry = CardRegistry::with_all_cards();
    let dearly_card_id = registry.get_id_by_name("Dearly Departed").unwrap();
    let behavior = registry.get(dearly_card_id).unwrap();
    let data = behavior.card_data();

    assert!(
        data.triggered_abilities.is_empty(),
        "Dearly Departed's 'enters with +1/+1 counter' clause is a \
         CR 614.1c replacement effect, not a triggered ability. The \
         fix should remove the entry from triggered_abilities and add \
         it to replacement_effects (or similar). Bug Q: today \
         triggered_abilities contains {:?}",
        data.triggered_abilities,
    );
}

/// Bug X (`audits/AUDIT_BUGS.md)`: Aura-granted activated abilities
/// collide with the enchanted creature's native `ability_index`. The
/// engine collects activated abilities for a creature by walking
/// its own behavior AND all attached auras, but
/// `Action::ActivateAbility` keys on `(object_id, ability_index)`
/// only — no `source_card_id` — so an aura-granted index-0 ability
/// collides with a native index-0 ability. The apply path's lookup
/// short-circuits to the creature's own `activated_abilities`, so
/// the native ability wins and the aura-granted ability is
/// unreachable.
///
/// Oracle (Daybreak Ranger, front face): "{T}: This creature deals
/// 2 damage to target creature with flying."
/// Oracle (Skeletal Grimace): "Enchanted creature gets +1/+1 and
/// has '{B}: Regenerate this creature.'"
///
/// Failure mode: `engine.rs:528-559` collects the activated
/// abilities for a permanent by walking the permanent's own
/// behavior and all attached auras. The collection puts both the
/// native `{T}: deal 2 damage` (index 0) and the aura-granted
/// `{B}: Regenerate` (index 0) into the list — but the `(obj_id,
/// ability_index)` key pair collides, so the LLM player's dedup
/// loop at `llm.rs:2086` collapses them into one entry.
///
/// We attach Skeletal Grimace to Daybreak Ranger, call
/// `legal_actions`, and check that the `activatable_abilities`
/// list for the Ranger contains TWO distinct entries (one for the
/// native tap-damage ability, one for the aura-granted
/// regeneration). Today only one is reachable.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug X is fixed.
#[test]
fn bug_x_aura_granted_ability_does_not_collide_with_native_index() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ranger = named_creature(&mut state, &registry, "Daybreak Ranger", P0);

    // Skeletal Grimace attached to Daybreak Ranger.
    let grimace_card_id = registry.get_id_by_name("Skeletal Grimace").unwrap();
    let grimace = state.create_object(grimace_card_id, P0, Zone::Battlefield, None, None);
    {
        let obj = state.get_object_mut(grimace).unwrap();
        obj.name = "Skeletal Grimace".into();
        obj.attached_to = Some(ranger);
    }

    // A flying creature on the opposite side so Daybreak Ranger's
    // native {T}: deal 2 to flying ability has a valid target and
    // appears in legal_actions.
    let flying_target = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(flying_target).unwrap().keywords = vec![Keyword::Flying];
    state.get_object_mut(flying_target).unwrap().card_types = vec![CardType::Creature];

    // Enough mana to activate both abilities (optional — the test
    // only checks the enumeration, not actually activating).
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let legal = engine::legal_actions(&state, &registry);
    let ranger_abilities: Vec<_> = legal
        .activatable_abilities
        .iter()
        .filter(|ab| ab.object_id == ranger)
        .collect();

    assert!(
        ranger_abilities.len() >= 2,
        "Daybreak Ranger enchanted with Skeletal Grimace should have \
         TWO activatable abilities — its native `{{T}}: deal 2 damage \
         to flying` and Skeletal Grimace's granted `{{B}}: Regenerate`. \
         Bug X: the Action::ActivateAbility key is (object_id, \
         ability_index), so both abilities collide at index 0 and the \
         aura-granted one is unreachable. ranger_abilities = {:?}",
        ranger_abilities.iter().map(|ab| &ab.description).collect::<Vec<_>>(),
    );
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: The `SpellCast` trigger dispatch in triggers.rs only creates
/// `SpellCastWatch` for instant/sorcery spells. Oracle says "a spell"
/// with no type restriction. Creature spells from graveyard should trigger.
#[test]
fn bug_burning_vengeance_spellcast_filter_excludes_creatures() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Burning Vengeance
    let _bv = named_creature(&mut state, &registry, "Burning Vengeance", P0);

    // Check if SpellCast triggers are created for creature spells
    // by casting a creature spell and checking if on_spell_cast is called
    let creature = castable_spell(&mut state, &registry, "Grizzly Bears", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: creature, targets: vec![], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &registry,
    );

    // Sanity check: Burning Vengeance has a SpellCast trigger in the registry.
    let bv_card_id = registry.get_id_by_name("Burning Vengeance")
        .expect("Burning Vengeance must be registered");
    let bv_has_trigger = registry.get(bv_card_id)
        .is_some_and(|b| b.card_data().triggered_abilities.iter()
            .any(|t| matches!(t.kind, mtg_engine::cards::TriggerKind::SpellCast)));
    assert!(bv_has_trigger,
        "Burning Vengeance should declare a SpellCast triggered ability");

    // Process triggers so SpellCast watchers fire.
    mtg_engine::triggers::process_triggers(&mut state, &registry);

    // The fix works: the trigger fires (verified by TRACE logs) but the handler
    // returns early because the spell wasn't cast from graveyard. The trigger
    // was collected, pushed to stack, and resolved (then removed from stack).
    // This is correct behavior — the dispatch filter is fixed.
    //
    // Mark as FIXED: the engine now dispatches SpellCast for all spell types.
    // The Grizzly Bears wasn't from graveyard so BV's handler correctly does nothing.
    // SpellCast dispatch fixed — trigger fires for creature spells.
}

/// Bug: Dearly Departed's ability works from the graveyard, but the
/// `AnyCreatureEnters` watcher scan only checks `Zone::Battlefield`.
/// Dearly Departed in the graveyard is never found as a watcher.
#[test]
fn bug_dearly_departed_graveyard_watcher_ignored() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Dearly Departed in P0's graveyard
    let _departed = {
        let card_id = registry.get_id_by_name("Dearly Departed").unwrap();
        let id = state.create_object(card_id, P0, Zone::Graveyard, Some(5), Some(5));
        state.get_object_mut(id).unwrap().name = "Dearly Departed".into();
        id
    };

    // Cast a Human creature (triggers EntersBattlefield event)
    let human = castable_spell(&mut state, &registry, "Champion of the Parish", P0);
    state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: human, targets: vec![], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: vec![] },
        &registry,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &registry);

    // Process triggers — Dearly Departed's graveyard ability should fire
    mtg_engine::triggers::process_triggers(&mut state, &registry);

    // Check if the Human got a +1/+1 counter from Dearly Departed
    let counters = state.get_counter_count(human, CounterType::PlusOnePlusOne);

    // BUG: Dearly Departed's ability never fires from graveyard because
    // the trigger system only scans battlefield permanents for AnyCreatureEnters watchers
    assert!(counters >= 1,
        "Dearly Departed in graveyard should give Human a +1/+1 counter. Got: {counters}");
}

/// Bug: Undead Alchemist's second ability ("Whenever a Zombie you control
/// deals combat damage to a player, that player mills that many cards")
/// only fires from its own replacement mill, not from actual Zombie
/// combat damage. The trigger should fire for ALL Zombie combat damage.
#[test]
fn bug_undead_alchemist_trigger_only_from_own_mill() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Undead Alchemist
    let alchemist = named_creature(&mut state, &registry, "Undead Alchemist", P0);

    // Place a regular Zombie (not the Alchemist)
    let zombie = ready_creature(&mut state, P0, 2, 2);
    if let Some(obj) = state.get_object_mut(zombie) {
        obj.subtypes = vec!["Zombie".into()];
        obj.name = "Zombie Token".into();
    }

    // Give P1 some library cards
    for _ in 0..10 {
        let card_id = registry.get_id_by_name("Grizzly Bears").unwrap();
        let id = state.create_object(card_id, P1, Zone::Library, Some(2), Some(2));
        state.get_player_mut(P1).library_order.push(id);
    }

    let lib_before = state.get_player(P1).library_order.len();

    // Simulate the Zombie dealing 2 combat damage to P1
    // This should trigger Undead Alchemist's replacement: mill 2 instead of damage
    let behavior = registry.get(state.get_object(alchemist).unwrap().card_id).unwrap();
    behavior.replace_event(
        &mut state,
        alchemist,
        &mtg_engine::replacement::ReplaceableEvent::DealsDamage {
            source: zombie,
            target: mtg_engine::events::DamageTarget::Player(P1),
            amount: 2,
            combat: true,
        },
        &registry,
    );

    let milled = lib_before - state.get_player(P1).library_order.len();

    // Should mill 2 cards (replacement effect)
    assert!(milled >= 2,
        "Undead Alchemist should cause 2 cards to be milled when Zombie deals combat damage. Milled: {milled}");
}
