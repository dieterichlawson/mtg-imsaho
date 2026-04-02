# Audit: Geistcatcher's Rig

## Oracle Reference
- **Name:** Geistcatcher's Rig
- **Mana Cost:** {6}
- **Type:** Artifact Creature — Construct
- **P/T:** 4/5
- **Oracle Text:** When this creature enters, you may have it deal 4 damage to target creature with flying.

## Card Data Audit
- **Name:** Correct ("Geistcatcher's Rig")
- **Mana Cost:** Correct (Generic(6))
- **Type:** Correct (Artifact, Creature)
- **Subtypes:** Correct ("Construct")
- **P/T:** Correct (4/5)

## Behavior Audit
- **ETB trigger:** `on_enter_battlefield` finds all creatures with flying on the battlefield (excluding itself) and presents a choice. Correct.
- **"You may":** The ability is optional. Code sets `optional: true` in the choice. Correct.
- **Damage amount:** `PendingEffect::DealDamage { amount: 4 }`. Correct.
- **Target restriction:** Filters creatures with `has_keyword(o.id, Keyword::Flying, registry)`. Correct -- targets creature with flying only.
- **Oracle text wording:** Code uses "enters the battlefield" while current oracle uses "When this creature enters". This is a cosmetic oracle text string difference only; behavior is correct.

## Result: PASS
