# Audit: Curse of the Pierced Heart

## Oracle Reference
- **Name:** Curse of the Pierced Heart
- **Mana Cost:** {1}{R}
- **Type:** Enchantment — Aura Curse
- **Oracle Text:** Enchant player / At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.

## Card Data Audit
- **Name:** Correct ("Curse of the Pierced Heart")
- **Mana Cost:** Correct (Generic(1), Red)
- **Type:** Correct (Enchantment)
- **Subtypes:** Correct ("Aura", "Curse")

## Behavior Audit
- **Enchant player:** `target_requirement` returns `TargetRequirement::PlayerOnly`. `on_resolve` uses `resolve_curse` to attach to player. Correct.
- **Triggered ability:** `on_upkeep` checks that the active player is the enchanted player. Correct.
- **Damage:** If no planeswalkers, deals 1 damage directly to player. If planeswalkers present, presents a choice to the curse controller to target the player or a planeswalker. Correct.
- **"This Aura deals 1 damage":** The damage source is `self_id` (the aura itself). Correct.

## Result: PASS
