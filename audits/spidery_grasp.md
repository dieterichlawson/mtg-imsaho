# Audit: Spidery Grasp

## Oracle (Scryfall)
- **Name:** Spidery Grasp
- **Cost:** {2}{G}
- **Type:** Instant
- **Oracle:** Untap target creature. It gets +2/+4 and gains reach until end of turn.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/spidery_grasp.rs`
- **Name:** Spidery Grasp ✅
- **Cost:** {2}{G} ✅
- **Type:** Instant ✅
- **Target:** TargetRequirement::Creature ✅
- **on_resolve:** untaps target (sets tapped = false) ✅
- **P/T buff:** +2/+4 via until_end_of_turn_effects ✅
- **Keyword grant:** Reach via until_end_of_turn_keywords ✅
- **Spell cleanup:** move_spell_after_resolve ✅

## Verdict: PASS -- no issues found
