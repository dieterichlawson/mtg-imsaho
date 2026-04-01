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

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Exile target card from a graveyard. Flashback {W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Scryfall type line**: Instant
**Status**: PASS

Previous targeting issue has been fixed. The implementation now uses `TargetRequirement::GraveyardCard` to allow the player to choose a target card from any graveyard.

Verified correct:
- Mana cost: {W} -- matches
- Type: Instant -- matches
- Oracle text: matches
- Flashback cost: {W} -- matches
- `on_resolve`: exiles target card, then calls `move_spell_after_resolve(object_id)` -- correct
- No anti-patterns detected: uses `move_spell_after_resolve` (not `move_object` to graveyard)
- Tests found in `mtg-engine/tests/tier11_cards.rs`
