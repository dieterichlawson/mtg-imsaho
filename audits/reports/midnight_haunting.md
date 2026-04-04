# Audit: Midnight Haunting

## Reference (Scryfall/API)
- **Name:** Midnight Haunting
- **Mana Cost:** {2}{W}
- **Type:** Instant
- **Oracle:** Create two 1/1 white Spirit creature tokens with flying.

## Implementation: `midnight_haunting.rs`
- **Name:** Midnight Haunting -- CORRECT
- **Mana Cost:** {2}{W} -- CORRECT
- **Type:** Instant -- CORRECT
- **Token creation:** Two 1/1 white Spirit creature tokens with Flying -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Create two 1/1 white Spirit creature tokens with flying.
**Type line**: Instant
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Midnight Haunting", cost {2}{W}, type Instant. On resolve, creates two 1/1 white Spirit creature tokens with flying. Behavior is correct.
