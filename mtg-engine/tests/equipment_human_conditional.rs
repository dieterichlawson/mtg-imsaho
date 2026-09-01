//! Regression tests for the equipment "as long as Human" snapshot bug
//! (audit finding "Bug B").
//!
//! Background: Silver-Inlaid Dagger says "Equipped creature gets +2/+0. As
//! long as equipped creature is a Human, it gets an additional +1/+0." and
//! Butcher's Cleaver says "+3/+0; as long as equipped creature is a Human,
//! it has lifelink." Both bonuses are *continuous* — they should follow the
//! current type of the creature, not be frozen at equip time.
//!
//! The previous implementations took a snapshot of `is_human` in their
//! `update_effects` helper at the moment of equipping, then wrote a fixed
//! `instance_continuous_effects` vector. So if a Human Werewolf transformed
//! into its non-Human back face (via Moonmist or via the no-spells-last-turn
//! upkeep trigger), the +1/+0 / lifelink stayed put. The fix is to express
//! the bonus as a condition wrapped around the effect —
//! `ContinuousEffect::when(EffectCondition::AttachedHasSubtype("Human"), ..)`,
//! the same shape Bonds of Faith uses — which is re-evaluated every time we
//! compute effective P/T or check keywords.
//!
//! (This note used to name `ContinuousEffect::ConditionalModifyPT` and
//! `ConditionalKeyword`. Those were four parallel variants that have since
//! been folded into the single `when` wrapper, so the names no longer exist.)
//!
//! These tests pin the fix in both directions: bonus appears when the
//! creature is Human, drops when it isn't, and updates live when the
//! type changes.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::state::GameState;
use mtg_engine::types::*;



/// Equip the equipment to the creature, paying mana from the pool.
fn equip(state: &GameState, reg: &CardRegistry, equipment_id: ObjectId, creature_id: ObjectId, equip_cost: i32) -> GameState {
    let mut s = state.clone();
    s.get_player_mut(P0).mana_pool.add(ManaType::Colorless, u32::try_from(equip_cost).unwrap_or(0));
    let legal = engine::legal_actions(&s, reg);
    let action = legal.actions.iter()
        .find(|a| matches!(a, Action::ActivateAbility { object_id, targets, .. }
            if *object_id == equipment_id && targets == &[Target::Object(creature_id)]))
        .cloned()
        .expect("equip action should be available");
    resolve_activated(engine::submit_action(&s, &action, reg), &reg)
}

// ════════════════════════════════════════════════════════════════════
// All three cards share the same rule shape: some unconditional
// modifier (P/T and/or keyword) plus an extra modifier that applies
// only while the equipped creature is a Human. The tests below are
// table-driven over HUMAN_CONDITIONAL — one row per card, four
// behavioral tests (Human / non-Human / subtype stripped / subtype
// added) plus a reattachment test.
// ════════════════════════════════════════════════════════════════════

struct HumanConditional {
    equipment: &'static str,
    equip_cost: i32,
    /// Power/toughness bonus applied regardless of subtype.
    base_p: i32,
    base_t: i32,
    /// Keyword granted regardless of subtype (if any).
    base_kw: Option<Keyword>,
    /// Additional power/toughness bonus when the creature is a Human.
    human_p: i32,
    human_t: i32,
    /// Keyword granted only when the creature is a Human (if any).
    human_kw: Option<Keyword>,
}

const HUMAN_CONDITIONAL: &[HumanConditional] = &[
    HumanConditional {
        equipment: "Silver-Inlaid Dagger", equip_cost: 2,
        base_p: 2, base_t: 0, base_kw: None,
        human_p: 1, human_t: 0, human_kw: None,
    },
    HumanConditional {
        equipment: "Butcher's Cleaver", equip_cost: 3,
        base_p: 3, base_t: 0, base_kw: None,
        human_p: 0, human_t: 0, human_kw: Some(Keyword::Lifelink),
    },
    HumanConditional {
        equipment: "Sharpened Pitchfork", equip_cost: 1,
        base_p: 0, base_t: 0, base_kw: Some(Keyword::FirstStrike),
        human_p: 1, human_t: 1, human_kw: None,
    },
];

/// Avacyn's Pilgrim is a 1/1 Human.
const PILGRIM_BASE: (i32, i32) = (1, 1);
/// Villagers of Estwald is a 2/3 Human Werewolf; its back face,
/// Howlpack of Estwald, is a 4/6 Werewolf — no longer a Human.
const VILLAGER_BASE: (i32, i32) = (2, 3);
const HOWLPACK_BASE: (i32, i32) = (4, 6);
/// Walking Corpse is a 2/2 Zombie (not a Human).
const ZOMBIE_BASE: (i32, i32) = (2, 2);

/// Assert the equipped creature's effective P/T and keywords match
/// what `case` promises for the given `is_human` status.
fn assert_equipped_matches(
    state: &GameState,
    reg: &CardRegistry,
    creature: ObjectId,
    base: (i32, i32),
    case: &HumanConditional,
    is_human: bool,
) {
    let (bp, bt) = base;
    let human_p = if is_human { case.human_p } else { 0 };
    let human_t = if is_human { case.human_t } else { 0 };
    assert_eq!(state.effective_power(creature, reg), Some(bp + case.base_p + human_p),
        "{}: power wrong (is_human={is_human})", case.equipment);
    assert_eq!(state.effective_toughness(creature, reg), Some(bt + case.base_t + human_t),
        "{}: toughness wrong (is_human={is_human})", case.equipment);
    if let Some(k) = case.base_kw {
        assert!(state.has_keyword(creature, k, reg),
            "{}: unconditional {k:?} should be granted", case.equipment);
    }
    if let Some(k) = case.human_kw {
        assert_eq!(state.has_keyword(creature, k, reg), is_human,
            "{}: conditional {k:?} should match is_human={is_human}", case.equipment);
    }
}

#[test]
fn human_equipment_bonuses_apply_when_attached_to_human() {
    let reg = registry();
    for case in HUMAN_CONDITIONAL {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let pilgrim = named_permanent(&mut state, &reg, "Avacyn's Pilgrim", P0);
        let gear = named_permanent(&mut state, &reg, case.equipment, P0);
        let state = equip(&state, &reg, gear, pilgrim, case.equip_cost);
        assert_equipped_matches(&state, &reg, pilgrim, PILGRIM_BASE, case, true);
    }
}

#[test]
fn human_equipment_bonuses_skip_conditional_on_non_human() {
    let reg = registry();
    for case in HUMAN_CONDITIONAL {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let zombie = named_permanent(&mut state, &reg, "Walking Corpse", P0);
        let gear = named_permanent(&mut state, &reg, case.equipment, P0);
        let state = equip(&state, &reg, gear, zombie, case.equip_cost);
        assert_equipped_matches(&state, &reg, zombie, ZOMBIE_BASE, case, false);
    }
}

/// Regression: the conditional bonus must follow the creature's current
/// subtype, not a snapshot taken at equip time. Uses the real lever from this
/// file's opening note — a Human Werewolf transforming into its non-Human back
/// face — rather than overwriting `obj.subtypes`, which models nothing the
/// engine does: outside transform, subtypes are only ever added to.
#[test]
fn human_equipment_conditional_bonus_drops_when_creature_transforms() {
    let reg = registry();
    for case in HUMAN_CONDITIONAL {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let villager = named_permanent(&mut state, &reg, "Villagers of Estwald", P0);
        let gear = named_permanent(&mut state, &reg, case.equipment, P0);
        let mut state = equip(&state, &reg, gear, villager, case.equip_cost);
        assert_equipped_matches(&state, &reg, villager, VILLAGER_BASE, case, true);

        mtg_engine::cards::helpers::apply_transform(&mut state, villager, &reg);
        assert!(!state.has_subtype(villager, "Human", &reg),
            "test precondition: Howlpack of Estwald is a Werewolf, not a Human");
        assert_equipped_matches(&state, &reg, villager, HOWLPACK_BASE, case, false);
    }
}

/// Inverse regression: bonus appears the moment a non-Human gains the
/// Human subtype.
#[test]
fn human_equipment_conditional_bonus_appears_when_subtype_added() {
    let reg = registry();
    for case in HUMAN_CONDITIONAL {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let zombie = named_permanent(&mut state, &reg, "Walking Corpse", P0);
        let gear = named_permanent(&mut state, &reg, case.equipment, P0);
        let mut state = equip(&state, &reg, gear, zombie, case.equip_cost);
        assert_equipped_matches(&state, &reg, zombie, ZOMBIE_BASE, case, false);

        state.get_object_mut(zombie).unwrap().subtypes = vec!["Zombie".into(), "Human".into()];
        assert_equipped_matches(&state, &reg, zombie, ZOMBIE_BASE, case, true);
    }
}

/// Reattachment: the bonus should follow the equipment, not stay on the
/// previously-equipped creature.
#[test]
fn human_equipment_bonuses_follow_reattachment() {
    let reg = registry();
    for case in HUMAN_CONDITIONAL {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let pilgrim = named_permanent(&mut state, &reg, "Avacyn's Pilgrim", P0);
        let zombie = named_permanent(&mut state, &reg, "Walking Corpse", P0);
        let gear = named_permanent(&mut state, &reg, case.equipment, P0);

        // Attach to Human — full bonus on pilgrim, nothing on zombie.
        let state = equip(&state, &reg, gear, pilgrim, case.equip_cost);
        assert_equipped_matches(&state, &reg, pilgrim, PILGRIM_BASE, case, true);
        assert_eq!(state.effective_power(zombie, &reg), Some(ZOMBIE_BASE.0),
            "{}: unequipped zombie should have base stats only", case.equipment);

        // Reattach to zombie — bonus moves with the equipment.
        let state = equip(&state, &reg, gear, zombie, case.equip_cost);
        assert_eq!(state.effective_power(pilgrim, &reg), Some(PILGRIM_BASE.0),
            "{}: pilgrim should lose all bonuses when unequipped", case.equipment);
        if let Some(k) = case.base_kw {
            assert!(!state.has_keyword(pilgrim, k, &reg),
                "{}: unconditional {k:?} should follow the equipment, not stay", case.equipment);
        }
        assert_equipped_matches(&state, &reg, zombie, ZOMBIE_BASE, case, false);
    }
}

/// CR 704.5n: when the equipped creature dies the Equipment stays on the
/// battlefield, unattached. The general detach test checks the zones; it does
/// not check that the bonus stopped, and neither does anything else here — the
/// reattachment test moves a live Equipment, which is a different path.
///
/// Then equip the survivor, so the conditional is recomputed against the new
/// creature rather than against whatever it last saw.
#[test]
fn human_equipment_bonuses_end_when_the_equipped_creature_dies() {
    let reg = registry();
    for case in HUMAN_CONDITIONAL {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let pilgrim = named_permanent(&mut state, &reg, "Avacyn's Pilgrim", P0);
        let zombie = named_permanent(&mut state, &reg, "Walking Corpse", P0);
        let gear = named_permanent(&mut state, &reg, case.equipment, P0);

        let mut state = equip(&state, &reg, gear, pilgrim, case.equip_cost);
        assert_equipped_matches(&state, &reg, pilgrim, PILGRIM_BASE, case, true);

        // Kill the Human it is attached to. Lethal is measured against the
        // toughness it has *while equipped* — Sharpened Pitchfork's Human
        // bonus is +1/+1, so the base 1/1 is not the number here.
        let lethal = state.effective_toughness(pilgrim, &reg).expect("a creature has toughness");
        state.get_object_mut(pilgrim).unwrap().damage_marked = u32::try_from(lethal).unwrap_or(0);
        check_state_based_actions(&mut state, &reg);
        assert_eq!(state.get_object(pilgrim).unwrap().zone, Zone::Graveyard,
            "{}: test precondition — the equipped creature died", case.equipment);

        assert_eq!(state.get_object(gear).unwrap().zone, Zone::Battlefield,
            "{}: an Equipment does not follow its creature to the graveyard", case.equipment);
        assert_eq!(state.get_object(gear).unwrap().attached_to, None,
            "{}: it detaches instead (CR 704.5n)", case.equipment);

        // Nothing it used to buff, and nothing it never buffed, is affected.
        assert_eq!(state.effective_power(zombie, &reg), Some(ZOMBIE_BASE.0),
            "{}: an unattached Equipment buffs nothing", case.equipment);
        if let Some(k) = case.base_kw {
            assert!(!state.has_keyword(zombie, k, &reg),
                "{}: nor grants its keyword to anything", case.equipment);
        }

        // Equipping the survivor recomputes the conditional for *it* — the
        // zombie is not a Human, so only the unconditional half applies.
        let state = equip(&state, &reg, gear, zombie, case.equip_cost);
        assert_equipped_matches(&state, &reg, zombie, ZOMBIE_BASE, case, false);
    }
}

/// Mutation-motivated (#44 backlog, `equip_target_is_legal`): equip attaches
/// only to a creature, on the battlefield, that the equipping player
/// controls (CR 702.6a) — each leg of that conjunction gets a case, killing
/// the `&& -> ||` survivor that let any one leg suffice.
#[test]
fn equip_targets_must_be_own_battlefield_creatures() {
    use mtg_engine::cards::helpers::equip_target_is_legal;
    use mtg_engine::actions::Target;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mine = ready_creature(&mut state, P0, 2, 2);
    let theirs = ready_creature(&mut state, P1, 2, 2);
    let land = named_permanent(&mut state, &reg, "Forest", P0);
    let in_hand = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);

    assert!(equip_target_is_legal(&state, P0, &Target::Object(mine), &reg),
        "your own battlefield creature is a legal equip target");
    assert!(!equip_target_is_legal(&state, P0, &Target::Object(theirs), &reg),
        "an opponent's creature is not (\"creature you control\")");
    assert!(!equip_target_is_legal(&state, P0, &Target::Object(land), &reg),
        "a land is not a creature");
    assert!(!equip_target_is_legal(&state, P0, &Target::Object(in_hand), &reg),
        "a creature card in hand is not on the battlefield");
    assert!(!equip_target_is_legal(&state, P0, &Target::Player(P0), &reg),
        "a player is never equipped");
}
