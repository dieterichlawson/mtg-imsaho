# Audit: Silent Departure

## Oracle (Scryfall)
- **Name:** Silent Departure
- **Cost:** {U}
- **Type:** Sorcery
- **Oracle:** Return target creature to its owner's hand. Flashback {4}{U}
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/silent_departure.rs`
- **Name:** Silent Departure ✅
- **Cost:** {U} ✅
- **Type:** Sorcery ✅
- **Oracle text:** matches ✅
- **Target:** TargetRequirement::Creature ✅
- **on_resolve:** returns target creature to hand, checks zone is battlefield ✅
- **Flashback:** {4}{U} ✅
- **Spell cleanup:** move_spell_after_resolve ✅

## Verdict: PASS -- no issues found
