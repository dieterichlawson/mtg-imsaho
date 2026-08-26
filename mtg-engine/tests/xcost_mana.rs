//! Failing tests for bugs documented in `audits/AUDIT_BUGS.md`.
//! Each test is expected to FAIL until the corresponding bug is
//! fixed. Once the fix lands the test transitions from "proves the
//! bug exists" to "regression-protects against the bug coming back".
//!
//! This file covers the "X-cost / mana-cost handling" family.
//!
//! Bugs covered in this file:
//! - Bug I: X-cost flashback `compute_autotap` fails for
//!   `{X}{R}{R}{R}` etc. — Devil's Play flashback never appears in
//!   the action list even when the player has plenty of mana.
//! - Bug 17-001: `ExileCreaturesFromGraveyard` cost handler reads
//!   base `o.power` instead of `effective_power`, so Corpse Lunge
//!   deals 0 damage when fed a Boneyard Wurm whose effective power
//!   would otherwise decide the damage.
//! - Bug Y: pay-mana-during-resolution prompts only check the mana
//!   pool — Screeching Bat's upkeep transform prompt never appears
//!   because mana pools empty between phases.

mod common;
use common::*;

use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

/// Bug I (`audits/AUDIT_BUGS.md)`: Devil's Play can't be flashback-cast
/// via autotap because `compute_autotap` doesn't understand X-cost
/// spells on the flashback path.
///
/// Oracle (Devil's Play): "Devil's Play deals X damage to any target.
/// Flashback {X}{R}{R}{R}."
///
/// Failure mode: the normal-cast path at `engine.rs` has
/// explicit X handling ("strip X from cost, tap all mana sources").
/// The flashback path at `engine.rs` calls
/// `mana::compute_autotap(fb_cost, ...)` directly without stripping
/// X. `compute_autotap` early-returns `None` for X-cost, the caller
/// `continue`s, and no flashback action is emitted — even when the
/// player has the full `{X}{R}{R}{R}` available.
///
/// We seed P0 with Devil's Play in the graveyard and seven untapped
/// Mountains (enough for X=4 flashback). The legal-actions list
/// should include a flashback cast of Devil's Play. Today it
/// doesn't.
#[test]
fn bug_i_devils_play_flashback_appears_in_legal_actions() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Devil's Play in P0's graveyard so the flashback path can pick
    // it up.
    let devils_card_id = registry.get_id_by_name("Devil's Play").unwrap();
    let devils = state.create_object(devils_card_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(devils).unwrap().name = "Devil's Play".into();

    // Seven untapped Mountains (more than {X}{R}{R}{R} = 4 colored +
    // some generic needs).
    let mountain_card_id = registry.get_id_by_name("Mountain").unwrap();
    for _ in 0..7 {
        let m = state.create_object(mountain_card_id, P0, Zone::Battlefield, None, None);
        state.get_object_mut(m).unwrap().name = "Mountain".into();
    }

    // A target creature so the any-target enumeration has something
    // to aim at.
    let _target = ready_creature(&mut state, P1, 2, 2);

    let legal = engine::legal_actions(&state, &registry);
    let has_flashback_devils = legal.castable_spells.iter().any(|cs| {
        cs.is_flashback && cs.name == "Devil's Play"
    });

    assert!(
        has_flashback_devils,
        "Devil's Play flashback should show up in legal_actions when \
         the player has enough mana for {{X}}{{R}}{{R}}{{R}}. Bug I: \
         the flashback code path calls compute_autotap with the full \
         X-cost, compute_autotap doesn't know how to handle X, returns \
         None, and the flashback action is silently dropped. \
         castable_spells = {:?}",
        legal.castable_spells,
    );
}

/// Bug 17-001 (`audits/AUDIT_BUGS.md)`: Corpse Lunge's
/// `ExileCreaturesFromGraveyard` additional-cost handler reads base
/// `o.power` instead of `state.effective_power`. For a Boneyard
/// Wurm / Splinterfright / Sturmgeist / Geist-Honored Monk — whose
/// P/T is a CDA (CR 208.2, works in all zones) — the stored
/// `exiled_power` ends up at the base 0 instead of the live CDA value.
///
/// Oracle (Corpse Lunge): "As an additional cost to cast this spell,
/// exile a creature card from your graveyard. Corpse Lunge deals
/// damage equal to the exiled card's power to target creature."
/// Oracle (Boneyard Wurm): "Boneyard Wurm's power and toughness are
/// each equal to the number of creature cards in your graveyard."
///
/// Failure mode: `engine.rs` builds
/// `exile_candidates: Vec<(ObjectId, i32)>` from `o.power.unwrap_or(0)`.
/// For a graveyard containing only Boneyard Wurm (base power 0),
/// `exiled_power` is stored as 0 — so Corpse Lunge's `on_resolve`
/// reads 0 and deals 0 damage, even though the Wurm's effective
/// power at cost time is ≥1.
///
/// We put a Boneyard Wurm as the only creature in P0's graveyard
/// (effective power 1, since the Wurm counts itself), cast Corpse
/// Lunge at a P1 creature, and check the damage dealt.
#[test]
fn bug_17_001_corpse_lunge_reads_effective_power_of_cda_creature() {
    use mtg_engine::actions::Target;

    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Boneyard Wurm as the only creature in P0's graveyard.
    let wurm_card_id = registry.get_id_by_name("Boneyard Wurm").unwrap();
    let wurm = state.create_object(wurm_card_id, P0, Zone::Graveyard, Some(0), Some(0));
    state.get_object_mut(wurm).unwrap().name = "Boneyard Wurm".into();

    // Sanity: at cost time, Wurm's effective_power should be 1.
    let eff = state.effective_power(wurm, &registry).unwrap_or(0);
    assert_eq!(
        eff, 1,
        "Test setup: Boneyard Wurm should have effective_power=1 \
         when it's the only creature card in its controller's \
         graveyard"
    );

    // Target for Corpse Lunge: a P1 creature with enough toughness
    // that it doesn't die immediately, so we can read damage_marked.
    let victim = ready_creature(&mut state, P1, 3, 3);

    // Cast Corpse Lunge via the full engine path so the additional
    // cost handler fires. `cast_and_resolve` handles the
    // ChooseExileFromGraveyard prompt by picking the max-power subset —
    // with only one creature in the graveyard, that's the Wurm.
    let lunge = castable_spell(&mut state, &registry, "Corpse Lunge", P0);
    state = cast_and_resolve(&state, &registry, lunge, vec![Target::Object(victim)]);

    let damage = state.get_object(victim).map_or(0, |o| o.damage_marked);
    assert!(
        damage >= 1,
        "Corpse Lunge exiling a Boneyard Wurm with effective_power 1 \
         should deal 1 damage (per CR 208.2, CDAs 'work in all \
         zones'). Bug 17-001: the additional-cost handler reads base \
         o.power (0), stores exiled_power=0, and Corpse Lunge deals 0 \
         damage. damage_marked = {damage}",
    );
}

/// Bug Y (`audits/AUDIT_BUGS.md)`: pay-mana-during-resolution prompts
/// (Screeching Bat, Mentor of the Meek, Frightful Delusion) check the
/// controller's current `mana_pool` only. Mana pools empty between
/// phases and steps (CR 106.4), so the upkeep trigger never sees
/// mana even when the player has untapped lands. Result: Screeching
/// Bat's upkeep transform option is silently never offered.
///
/// Oracle (Screeching Bat): "{2}{B}{B}: Transform this creature."
/// Screeching Bat's upkeep trigger ("At the beginning of your upkeep,
/// you may pay {2}{B}{B}. If you do, transform this creature.") must
/// be *offered* whenever the player has enough untapped mana, not
/// only when the pool is already floating that much.
///
/// Failure mode: `screeching_bat.rs::on_upkeep` used to check
/// `crate::mana::can_pay(pool, &cost)` against the *current* pool.
/// Per CR 106.4, mana pools empty at the end of each step, so the
/// pool is empty on the upkeep trigger resolve and the `YesNo` "may
/// pay" prompt was never shown — even when the player controlled
/// four untapped Swamps.
///
/// Fix: `on_upkeep` now routes through
/// `engine::plan_autotap_for_cost` (autotap reachability) instead of
/// pool-floating. `on_yes_no_choice` runs the same plan and taps
/// sources before paying.
///
/// We drive the trigger handler directly with a Screeching Bat in
/// play, an empty pool, and four untapped Swamps, and assert that
/// the `YesNo` prompt is set up.
#[test]
fn bug_y_screeching_bat_transform_offered_with_untapped_lands() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_creature(&mut state, &registry, "Screeching Bat", P0);

    // Four untapped Swamps — {2}{B}{B} should be autotap-reachable.
    let swamp_card_id = registry.get_id_by_name("Swamp").unwrap();
    for _ in 0..4 {
        let s = state.create_object(swamp_card_id, P0, Zone::Battlefield, None, None);
        let obj = state.get_object_mut(s).unwrap();
        obj.name = "Swamp".into();
    }

    // Mana pool is empty (just entered upkeep) — the bug fires here.
    assert_eq!(
        state.get_player(P0).mana_pool.total(),
        0,
        "Test setup: mana pool should be empty at upkeep"
    );

    // Drive the trigger handler directly (simulating trigger resolution).
    let bat_card_id = state.get_object(bat).unwrap().card_id;
    let behavior = registry.get(bat_card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &[], &registry);

    // Expected: `YesNo` "may pay" prompt is now awaiting a response.
    match state.awaiting_action.as_ref() {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::YesNo { description, source_card },
            ..
        }) => {
            assert_eq!(*source_card, bat);
            assert!(
                description.contains("Swamp") || description.contains("tap"),
                "Prompt description should describe the tap plan so the \
                 player can weigh opportunity cost, got: {description:?}"
            );
        }
        other => panic!(
            "Screeching Bat's may-pay upkeep prompt should fire when the \
             player controls four untapped Swamps — Bug Y: the card checked \
             the current (empty) pool instead of routing through autotap. \
             awaiting_action={other:?}"
        ),
    }

    // And if the player says yes, the Bat should transform and the
    // Swamps should tap to pay the cost.
    behavior.on_yes_no_choice(&mut state, bat, true, &registry);
    assert!(
        state.get_object(bat).unwrap().is_transformed,
        "Screeching Bat should have transformed after saying yes"
    );
    let tapped_swamps = state.objects.values()
        .filter(|o| o.name == "Swamp" && o.tapped)
        .count();
    assert_eq!(tapped_swamps, 4, "all four Swamps should be tapped");
}
