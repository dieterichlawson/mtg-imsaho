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

## Audit — 2026-04-02
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Untap target creature. It gets +2/+4 and gains reach until end of turn.
**Mana cost**: {2}{G}
**Type line**: Instant
**Status**: PASS
### Checks
- **Name**: "Spidery Grasp" -- CORRECT
- **Mana cost**: Generic(2) + Green -- CORRECT ({2}{G})
- **Type**: Instant -- CORRECT
- **Target requirement**: Creature -- CORRECT
- **on_resolve**: Untaps target (sets tapped=false), applies +2/+4 UntilEndOfTurnEffect, grants Reach UntilEndOfTurnKeyword -- CORRECT
- **Validity check**: Only applies if target is on battlefield -- CORRECT
### Code issues
None. Card data and behavior match oracle text.
