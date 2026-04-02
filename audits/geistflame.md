# Audit: Geistflame

## Oracle Reference
- **Name:** Geistflame
- **Mana Cost:** {R}
- **Type:** Instant
- **Oracle Text:** Geistflame deals 1 damage to any target. / Flashback {3}{R}
- **Keywords:** Flashback

## Card Data Audit
- **Name:** Correct ("Geistflame")
- **Mana Cost:** Correct (Red)
- **Type:** Correct (Instant)
- **Flashback Cost:** Correct (Generic(3), Red)

## Behavior Audit
- **Targeting:** `TargetRequirement::AnyTarget`. Correct.
- **Damage:** `resolve_damage` with amount 1. Correct.
- **Flashback:** `flashback_cost` is set to {3}{R}. Correct.

## Result: PASS
