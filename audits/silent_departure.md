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

## Audit - 2026-04-02

### Oracle Text (Scryfall)
- **Name:** Silent Departure
- **Mana Cost:** {U}
- **Type:** Sorcery
- **Oracle Text:** Return target creature to its owner's hand. / Flashback {4}{U}

### Card Data Audit
- **Name:** Correct ("Silent Departure")
- **Cost:** Correct ({U})
- **Types:** Correct (Sorcery)
- **Oracle Text String:** MISMATCH
  - **Oracle:** "Return target creature to its owner's hand.\nFlashback {4}{U}"
  - **Code:** "Return target creature to its owner's hand."
  - The flashback line is missing from the oracle_text string (though `flashback_cost` is correctly set).
- **Flashback Cost:** Correct ({4}{U} = Generic(4) + Blue)

### Behavior Audit
- **Target requirement:** `TargetRequirement::Creature`. Correct.
- **Bounce effect:** Checks target is on battlefield, moves to Hand zone. Correct.
- **Spell cleanup:** Calls `move_spell_after_resolve`. Correct for flashback handling.
- **Sorcery timing:** Card type is Sorcery. Correct.

### Result
**ISSUE** -- Oracle text string missing flashback line. Code: `"Return target creature to its owner's hand."` vs oracle: `"Return target creature to its owner's hand.\nFlashback {4}{U}"`.
