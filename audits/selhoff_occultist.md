# Audit: Selhoff Occultist

## Official Oracle
- **Name:** Selhoff Occultist
- **Cost:** {2}{U}
- **Type:** Creature — Human Rogue
- **Oracle Text:** Whenever Selhoff Occultist or another creature dies, target player mills a card.
- **P/T:** 2/3

## Implementation Review
- **Name:** OK
- **Cost:** {2}{U} — OK
- **Type:** Creature, subtypes ["Human", "Rogue"] — OK
- **Oracle Text:** Matches — OK
- **P/T:** 2/3 — OK
- **Triggered Abilities:** SelfDies + AnyCreatureDies — OK
- **on_dies:** Presents mill choice (target player) — OK
- **on_any_creature_dies:** Checks self is on battlefield, presents mill choice — OK
- **Mill count:** 1 card — OK

## Issues
None found.

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Text (Scryfall)
- **Name:** Selhoff Occultist
- **Mana Cost:** {2}{U}
- **Type:** Creature — Human Rogue
- **P/T:** 2/3
- **Oracle Text:** Whenever this creature or another creature dies, target player mills a card.

### Card Data Audit
- **Name:** Correct ("Selhoff Occultist")
- **Cost:** Correct ({2}{U})
- **Types:** Correct (Creature, subtypes Human + Rogue)
- **P/T:** Correct (2/3)
- **Oracle Text String:** MISMATCH
  - **Oracle:** "Whenever this creature or another creature dies, target player mills a card."
  - **Code:** "Whenever Selhoff Occultist or another creature dies, target player mills a card."
  - Functionally equivalent; modern oracle templates use "this creature" but older printings used the card name.

### Behavior Audit
- **Self-dies trigger:** `on_dies` calls `present_mill_choice`. Correct.
- **Other-creature-dies trigger:** `on_any_creature_dies` checks self is on battlefield, then calls `present_mill_choice`. Correct.
- **Target player mills a card:** Presents choice of all players, applies `PendingEffect::Mill { count: 1 }`. Correct.
- **Targeting:** Mandatory (`optional: false`). Correct.

### Result
**ISSUE** -- Oracle text string uses card name ("Selhoff Occultist") where current oracle uses "this creature".

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "Whenever this creature or another creature dies, target player mills a card." (was "Selhoff Occultist or another creature dies"). Doc comment updated. Behavior unchanged.
