# Audit: Abbey Griffin

## Reference (Scryfall/API)
- **Name:** Abbey Griffin
- **Mana Cost:** {3}{W}
- **Type:** Creature — Griffin
- **Oracle:** Flying, vigilance
- **P/T:** 2/2

## Implementation: `abbey_griffin.rs`
- **Name:** Abbey Griffin -- CORRECT
- **Mana Cost:** {3}{W} -- CORRECT
- **Type:** Creature — Griffin -- CORRECT
- **P/T:** 2/2 -- CORRECT
- **Keywords:** Flying, Vigilance -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying, vigilance
**Type line**: Creature — Griffin
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Abbey Griffin", cost {3}{W}, 2/2, type Creature — Griffin, keywords [Flying, Vigilance]. Vanilla creature with keywords only, no behavior needed beyond card_data.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.
