# Audit: Sever the Bloodline

## Official Oracle
- **Name:** Sever the Bloodline
- **Cost:** {3}{B}
- **Type:** Sorcery
- **Oracle Text:** Exile target creature and all other creatures with the same name as that creature.\nFlashback {5}{B}{B}
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {3}{B} — OK
- **Type:** Sorcery — OK
- **Oracle Text:** "Exile target creature and all other creatures with the same name.\nFlashback {5}{B}{B}" — close match (official says "same name as that creature") — OK
- **Flashback Cost:** {5}{B}{B} — OK
- **P/T:** N/A — OK
- **Target:** TargetRequirement::Creature — OK
- **on_resolve:** Gets name of target, finds all creatures with same name, exiles all — OK

## Issues
None found.

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Text (Scryfall)
- **Name:** Sever the Bloodline
- **Mana Cost:** {3}{B}
- **Type:** Sorcery
- **Oracle Text:** Exile target creature and all other creatures with the same name as that creature. / Flashback {5}{B}{B}

### Card Data Audit
- **Name:** Correct ("Sever the Bloodline")
- **Cost:** Correct ({3}{B})
- **Types:** Correct (Sorcery)
- **Oracle Text String:** MISMATCH
  - **Oracle:** "Exile target creature and all other creatures with the same name as that creature."
  - **Code:** "Exile target creature and all other creatures with the same name."
  - Missing "as that creature" suffix.
- **Flashback Cost:** Correct ({5}{B}{B})

### Behavior Audit
- **Target requirement:** `TargetRequirement::Creature`. Correct.
- **Exile target + same-name creatures:** Gets target's name, filters all battlefield creatures with matching name, exiles all. Correct.
- **Only battlefield creatures:** Filters `o.zone == Zone::Battlefield`. Correct per ruling.
- **Flashback:** `flashback_cost` set to {5}{B}{B}. Correct.
- **Spell cleanup:** Calls `move_spell_after_resolve`. Correct.

### Result
**ISSUE** -- Oracle text string missing "as that creature" at the end. Code: `"Exile target creature and all other creatures with the same name."` vs oracle: `"Exile target creature and all other creatures with the same name as that creature."`.
