# Audit: Rally the Peasants

## Official Oracle
- **Name:** Rally the Peasants
- **Cost:** {2}{W}
- **Type:** Instant
- **Oracle Text:** Creatures you control get +2/+0 until end of turn.\nFlashback {2}{R}
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {2}{W} — OK
- **Type:** Instant — OK
- **Oracle Text:** "Creatures you control get +2/+0 until end of turn." — OK (flashback text missing from oracle_text field but flashback_cost is set)
- **Flashback Cost:** {2}{R} — OK
- **P/T:** N/A — OK
- **on_resolve:** Applies +2/+0 UntilEndOfTurnEffect to all creatures controller controls — OK

## Issues
1. **Minor: Oracle text missing flashback reminder**: The oracle_text field doesn't include "Flashback {2}{R}" but the flashback_cost field is correctly set. This is consistent with other cards in the engine.

## Verdict: PASS (minor oracle text omission, functionally correct)

---

# Audit: Rally the Peasants (2026-04-02)

## Oracle Text (Scryfall)
- **Name:** Rally the Peasants
- **Mana Cost:** {2}{W}
- **Type:** Instant
- **Oracle Text:** Creatures you control get +2/+0 until end of turn. Flashback {2}{R}

## Card Data Verification
- **Name:** Correct ("Rally the Peasants")
- **Cost:** Correct ({2}{W})
- **Type:** Correct (Instant)
- **Flashback Cost:** Correct ({2}{R})

## Behavior Verification
- **Effect:** Correct — collects all creatures the controller controls on the battlefield and applies `power_mod: 2, toughness_mod: 0` as an until-end-of-turn effect.
- **Scope:** Correct — only affects creatures you control at time of resolution (per ruling).
- **Cleanup:** Correct — calls `move_spell_after_resolve` to handle graveyard/exile.

## Result: PASS
