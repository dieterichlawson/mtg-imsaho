mod common;

use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;
use common::{game_at_step, named_creature, P0, P1};

/// Bug BR (audits/AUDIT_BUGS.md): Olivia Voldaren's first ability
/// applies damage by directly writing `obj.damage_marked += 1`
/// (olivia_voldaren.rs:112) instead of routing through the central
/// `PendingEffect::DealDamage` handler (engine.rs:3024). The central
/// handler checks `has_protection_from` (engine.rs:3054) before applying
/// damage. By inlining the damage, Olivia bypasses the protection check.
///
/// Oracle (Elite Inquisitor): "Protection from Vampires, from Werewolves,
/// and from Zombies."
/// Oracle (Olivia Voldaren): "{1}{R}: Olivia Voldaren deals 1 damage to
/// another target creature."
///
/// Olivia Voldaren is a Vampire. Elite Inquisitor has protection from
/// Vampires. Per CR 702.16b, protection prevents all damage that would
/// be dealt by sources with the specified quality. So Olivia's 1 damage
/// should be prevented entirely — Elite Inquisitor should take 0 damage.
///
/// Failure mode: olivia_voldaren.rs:107-113 writes `obj.damage_marked += 1`
/// without calling `has_protection_from`. The central damage handler at
/// engine.rs:3054 does check protection, but Olivia never uses it.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing once the bug is fixed.
#[test]
#[ignore] // Pipeline bug — awaiting fix
fn bug_br_olivia_damage_blocked_by_protection_from_vampires() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Olivia Voldaren on P0's side — she is a Vampire.
    let olivia = named_creature(&mut state, &registry, "Olivia Voldaren", P0);

    // Elite Inquisitor on P1's side — has protection from Vampires.
    let inquisitor = named_creature(&mut state, &registry, "Elite Inquisitor", P1);

    // Sanity: Elite Inquisitor starts with 0 damage.
    assert_eq!(
        state.get_object(inquisitor).unwrap().damage_marked, 0,
        "Sanity: Elite Inquisitor should start with no damage"
    );

    // Fire Olivia's first ability targeting Elite Inquisitor.
    let olivia_card_id = state.get_object(olivia).unwrap().card_id;
    let behavior = registry.get(olivia_card_id).unwrap();
    behavior.on_activate_ability(
        &mut state,
        olivia,
        0,
        &[Target::Object(inquisitor)],
        &registry,
    );

    // CR 702.16b: Protection prevents all damage from sources with the
    // specified quality. Olivia is a Vampire, Elite Inquisitor has
    // protection from Vampires, so the 1 damage should be prevented.
    let damage = state.get_object(inquisitor).unwrap().damage_marked;
    assert_eq!(
        damage, 0,
        "Elite Inquisitor has protection from Vampires (CR 702.16b). \
         Olivia Voldaren is a Vampire, so her 1 damage should be \
         prevented entirely. Bug BR: olivia_voldaren.rs inlines \
         `obj.damage_marked += 1` instead of using the central damage \
         handler, which bypasses the protection check. damage_marked = {}",
        damage
    );
}
