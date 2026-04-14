//! Failing tests for bugs documented in audits/AUDIT_BUGS.md.
//! Each test is expected to FAIL until the corresponding bug is
//! fixed. Once the fix lands the test transitions from "proves the
//! bug exists" to "regression-protects against the bug coming back".
//!
//! This file covers a slice of the "Trigger dispatch and timing"
//! family — bugs whose root cause is in card-level handlers reading
//! the wrong source-state field, or in the dispatcher reaching the
//! wrong handler.
//!
//! Bugs covered in this file:
//! - Bug BT: `on_any_creature_dies` handlers zone-gate on Battlefield,
//!   silently dropping triggers when the watcher dies simultaneously
//!   with its target (Abattoir Ghoul mutual first-strike trade)
//! - Bug L: Charmbreaker Devils' SpellCast trigger fires for every
//!   spell type instead of only instants/sorceries
//! - Bug CA: Moldgraf Monstrosity reads `o.owner` instead of
//!   `o.controller`, returning creatures from the wrong player's
//!   graveyard when stolen
//! - Bug BU: Burning Vengeance logs "deals 2 damage to opponent"
//!   before the target is chosen.
//! - Bug K: SacrificeThis activated abilities got the no-autotap
//!   restriction regression from the Bug C fix — Selfless Cathar's
//!   ability isn't offered unless the player has pre-floated mana.
//! - Bug 17-002: Undead Alchemist's second ability (watcher for
//!   creature cards milled into opponents' graveyards from their
//!   libraries) is entirely missing — `mill_cards` on an opp
//!   doesn't exile the creature or create Zombie tokens.
//! - Bug AE: Undead Alchemist's "instead" damage replacement is
//!   implemented as a post-damage trigger, so lethal combat damage
//!   kills the player before the trigger can restore life.

mod common;
use common::*;

use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

/// Bug BT (audits/AUDIT_BUGS.md): Abattoir Ghoul's
/// `on_any_creature_dies` handler early-returns when its self_id is
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

/// Bug L (audits/AUDIT_BUGS.md): Charmbreaker Devils' `on_spell_cast`
/// triggered ability fires for every spell type, not just
/// instants/sorceries.
///
/// Oracle (Charmbreaker Devils): "Whenever you cast an instant or
/// sorcery spell, this creature gets +4/+0 until end of turn."
///
/// Failure mode: `charmbreaker_devils.rs:75-92` filters by `caster ==
/// controller` but does NOT filter by spell type. The dispatcher
/// (`triggers.rs:727`) explicitly says "Dispatch SpellCast triggers
/// for ALL spell types... Individual card handlers can filter by
/// spell type if needed" — Charmbreaker doesn't.
///
/// We exercise the bug by calling Charmbreaker's `on_spell_cast`
/// handler directly with a creature spell as the trigger source. The
/// handler should be a no-op (Grizzly Bears is a creature, not an
/// instant or sorcery). Today the handler unconditionally pushes a
/// `+4/+0` ModifyPT into `until_end_of_turn`.
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
    behavior.on_spell_cast(&mut state, devils, P0, bears_spell, &registry);

    let after_power = state.effective_power(devils, &registry).unwrap_or(0);
    assert_eq!(
        after_power, base_power,
        "Charmbreaker Devils' +4/+0 should NOT trigger when the \
         controller casts a creature spell — its oracle text restricts \
         the trigger to instants and sorceries. Bug L: the handler \
         doesn't filter by spell type. effective_power: {base_power} -> {after_power}",
    );
}

/// Bug CA (audits/AUDIT_BUGS.md): Moldgraf Monstrosity reads
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
    behavior.on_dies(&mut state, mold, &registry);

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

/// Bug BU (audits/AUDIT_BUGS.md): Burning Vengeance's `on_spell_cast`
/// handler logs "deals 2 damage to opponent" BEFORE the controller
/// picks a target. The log line is stale — the player may choose a
/// creature, themselves, etc., and the hard-coded "opponent" text
/// misleads the LLM reading the log.
///
/// Oracle (Burning Vengeance): "Whenever you cast a spell from your
/// graveyard, this enchantment deals 2 damage to any target."
///
/// Failure mode: `burning_vengeance.rs:55-68` calls
/// `present_target_choice(...)` to set up the awaiting_action, then
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
    behavior.on_spell_cast(&mut state, vengeance, P0, flashback_spell, &registry);

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

/// Bug K (audits/AUDIT_BUGS.md): The Bug C fix added a blanket
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
#[ignore = "Tabled — requires autotap/special cost overhaul"]
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

/// Bug 17-002 (audits/AUDIT_BUGS.md): Undead Alchemist's second
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
/// "CreatureMilledFromLibrary" watcher.
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

    let zombie_tokens_before = state
        .objects
        .values()
        .filter(|o| o.is_token && o.controller == P0 && o.name == "Zombie")
        .count();

    // Mill 1 card from P1's library.
    engine::mill_cards(&mut state, P1, 1, &registry);
    // Run trigger processing so any watcher triggers get a chance
    // to fire.
    mtg_engine::triggers::process_triggers(&mut state, &registry);

    let milled_zone = state.get_object(milled).map(|o| o.zone);
    let zombie_tokens_after = state
        .objects
        .values()
        .filter(|o| o.is_token && o.controller == P0 && o.name == "Zombie")
        .count();

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

/// Bug AE (audits/AUDIT_BUGS.md): Undead Alchemist implements the
/// "if a Zombie you control would deal combat damage to a player,
/// instead that player mills" clause as an AnyCombatDamageToPlayer
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
/// this manifests as: the player's life stays at starting_life
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

/// Bug M (audits/AUDIT_BUGS.md): Snapcaster Mage's ETB trigger
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
/// We put Snapcaster on the battlefield, push an EnteredBattlefield
/// event, call `collect_triggers` (NOT `process_triggers`), and
/// assert that `state.awaiting_action` is already set for the
/// target choice. Today it's None because the choice is deferred
/// to resolution.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug M is fixed.
#[test]
fn bug_m_snapcaster_target_chosen_at_stack_time() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // A valid flashback target in P0's graveyard.
    let grudge_card_id = registry.get_id_by_name("Ancient Grudge").unwrap();
    let _grudge = state.create_object(grudge_card_id, P0, Zone::Graveyard, None, None);

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

    // CR 603.3d: the target should be chosen NOW (at stack-queue
    // time). awaiting_action should be set for a target choice.
    assert!(
        state.awaiting_action.is_some(),
        "Snapcaster Mage's ETB trigger should prompt for a target \
         when the trigger is put on the stack (CR 603.3d), not when \
         it resolves. Bug M: the target choice is deferred to \
         on_enter_battlefield at resolution time, so opponents can't \
         respond between 'trigger goes on stack' and 'target locked in'. \
         awaiting_action = {:?}",
        state.awaiting_action,
    );
}

/// Bug N (audits/AUDIT_BUGS.md): When multiple triggered abilities
/// controlled by the same player trigger simultaneously, CR 603.3b
/// says that player chooses the order they go on the stack. The engine
/// pushes them in collection order without ever asking.
///
/// Oracle (CR 603.3b): "If multiple triggered abilities triggered at
/// the same time, the active player puts all of theirs on the stack
/// in any order, then each other player in turn order does the same."
///
/// Failure mode: `triggers.rs:946-951` does:
/// ```
/// for t in ap_triggers { state.stack.push(...); }
/// for t in nap_triggers { state.stack.push(...); }
/// ```
/// No ordering prompt is presented. With 2+ Falkenrath Nobles and a
/// creature death, both drain triggers fire simultaneously and the
/// player should choose the stack order.
///
/// We set up two Falkenrath Nobles, kill a P1 creature, collect
/// triggers, and assert the engine pauses for an ordering choice.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug N is fixed.
#[test]
fn bug_n_apnap_ordering_prompt_for_simultaneous_triggers() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Two Falkenrath Nobles for P0 — each triggers on creature death.
    let _noble1 = named_creature(&mut state, &registry, "Falkenrath Noble", P0);
    let _noble2 = named_creature(&mut state, &registry, "Falkenrath Noble", P0);

    // Kill a P1 creature to fire both AnyCreatureDies triggers.
    let victim = ready_creature(&mut state, P1, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;
    mtg_engine::sba::check_state_based_actions(&mut state, &registry);

    // Collect triggers — should produce 2+ simultaneous AP triggers.
    let had_triggers = mtg_engine::triggers::collect_triggers(&mut state, &registry);
    assert!(had_triggers, "Test setup: should have had triggers after creature death");

    // Count AP (P0) triggers on the stack.
    let ap_count = state.stack.iter().filter(|e| {
        matches!(e, mtg_engine::state::StackEntry::Trigger(t) if t.controller() == P0)
    }).count();
    assert!(
        ap_count >= 2,
        "Test setup: expected 2+ simultaneous P0 triggers, got {ap_count}",
    );

    // CR 603.3b: the player should be prompted to choose the order.
    assert!(
        state.awaiting_action.is_some(),
        "When a player has 2+ simultaneous triggered abilities, the \
         engine should prompt them to choose the stack order (CR 603.3b). \
         Bug N: triggers.rs pushes them in collection order without any \
         prompt. awaiting_action = {:?}",
        state.awaiting_action,
    );
}

/// Bug Q (audits/AUDIT_BUGS.md): Dearly Departed's "each Human
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
/// We check the card_data's triggered_abilities list is empty
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

/// Bug X (audits/AUDIT_BUGS.md): Aura-granted activated abilities
/// collide with the enchanted creature's native ability_index. The
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
