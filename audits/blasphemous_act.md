# Audit: Blasphemous Act

## Oracle (Scryfall)
- **Name:** Blasphemous Act
- **Cost:** {8}{R}
- **Type:** Sorcery
- **Oracle:** This spell costs {1} less to cast for each creature on the battlefield. Blasphemous Act deals 13 damage to each creature.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/blasphemous_act.rs`
- **Name:** Blasphemous Act ✅
- **Cost:** {8}{R} ✅
- **Type:** Sorcery ✅
- **Oracle text:** matches ✅
- **Cost reduction:** `modified_cost` counts creatures, reduces generic by that count, capped at 8 ✅
- **Damage:** 13 to each creature ✅
- **NonCombatDamageDealt events:** emitted for each creature ✅
- **damaged_by tracking:** pushes `object_id` for each creature ✅
- **move_spell_after_resolve:** called ✅

## Verdict: PASS — no issues found
