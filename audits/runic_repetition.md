# Audit: Runic Repetition

## Official Oracle
- **Name:** Runic Repetition
- **Cost:** {2}{U}
- **Type:** Sorcery
- **Oracle Text:** Return target exiled card with flashback you own to your hand.
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {2}{U} — OK
- **Type:** Sorcery — OK
- **Oracle Text:** "Return target exiled card you own with flashback to your hand." — minor wording order difference from official "Return target exiled card with flashback you own to your hand." — functionally identical — OK
- **P/T:** N/A — OK
- **on_resolve:** Auto-selects a card in exile owned by controller with flashback, moves to hand — functionally correct but no targeting

## Issues
1. **ISSUE: No targeting — auto-selects**: The card says "target exiled card with flashback you own" but the implementation auto-selects the first matching card rather than letting the player choose which exiled flashback card to return. The player should have a choice when multiple candidates exist.

## Verdict: FAIL
- **Targeting is bypassed** — player should choose which exiled flashback card to return
