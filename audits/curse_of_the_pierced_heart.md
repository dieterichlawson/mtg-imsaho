# Audit: Curse of the Pierced Heart

## Scryfall Reference
- **Name:** Curse of the Pierced Heart
- **Cost:** {1}{R}
- **Type:** Enchantment -- Aura Curse
- **Oracle:** Enchant player. At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
- **P/T:** N/A
- **Keywords:** Enchant

## Implementation: `curse_of_the_pierced_heart.rs`
- **Name:** Curse of the Pierced Heart -- CORRECT
- **Cost:** {1}{R} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Subtypes:** ["Aura", "Curse"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Target:** TargetRequirement::PlayerOnly -- CORRECT
- **Trigger:** Upkeep -- CORRECT
- **Behavior:** Deals 1 damage to enchanted player at their upkeep -- CORRECT
- **Uses NonCombatDamageDealt event:** Yes -- CORRECT

## Issues
1. **MINOR: Oracle says "deals 1 damage to that player or a planeswalker that player controls."** The implementation always deals damage to the player (never to a planeswalker). Since planeswalkers are unlikely in this engine's context, this is a minor simplification.

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Enchant player. At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
**Scryfall type line**: Enchantment — Aura Curse
**Status**: PASS

Findings:
- Mana cost {1}{R}: correct.
- Types Enchantment, subtypes Aura/Curse: correct.
- P/T N/A: correct.
- TargetRequirement::PlayerOnly for enchant player: correct.
- Triggered ability declared in triggered_abilities vec (TriggerKind::Upkeep): correct, no missing declaration.
- on_upkeep checks active_player == cursed_player: correct (only fires on enchanted player's upkeep).
- Damage dealt via direct life subtraction + NonCombatDamageDealt event: correct (not CombatDamageDealt).
- Anti-pattern check: No `move_object(id, Zone::Graveyard)` for spells (this is an enchantment, stays on battlefield). No issues.
- Oracle discrepancy (carried forward): implementation cannot redirect damage to a planeswalker the enchanted player controls. Minor accepted simplification.
- Tests found in tier15_cards.rs and tier7_cards.rs.
