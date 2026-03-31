# Audit: Purify the Grave

## Official Oracle
- **Name:** Purify the Grave
- **Cost:** {W}
- **Type:** Instant
- **Oracle Text:** Exile target card from a graveyard.\nFlashback {W}
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {W} — OK
- **Type:** Instant — OK
- **Oracle Text:** Matches — OK
- **Flashback Cost:** {W} — OK
- **P/T:** N/A — OK

## Issues
1. **ISSUE: No targeting — auto-selects from opponent's graveyard first**: The card says "target card from a graveyard" (any graveyard, player chooses). The implementation uses TargetRequirement::None and auto-selects the first card from the opponent's graveyard. This means:
   - The player has no choice of which card to exile
   - It always prioritizes opponent's graveyard over own graveyard
   - The card should target ANY card in ANY graveyard, not auto-select
   - Comment in code acknowledges this is "an approximation"

## Verdict: FAIL
- **Targeting is completely bypassed** — the player should choose which card from which graveyard to exile
