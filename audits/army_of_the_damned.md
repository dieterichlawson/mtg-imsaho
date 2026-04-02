# Audit: Army of the Damned

## Reference (Scryfall/API)
- **Name:** Army of the Damned
- **Mana Cost:** {5}{B}{B}{B}
- **Type:** Sorcery
- **Oracle:** Create thirteen tapped 2/2 black Zombie creature tokens. Flashback {7}{B}{B}{B}
- **P/T:** N/A

## Implementation: `army_of_the_damned.rs`
- **Name:** Army of the Damned -- CORRECT
- **Mana Cost:** {5}{B}{B}{B} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Flashback:** {7}{B}{B}{B} -- CORRECT
- **Effect:** Creates 13 tokens, each 2/2 black Zombie creature, enters tapped -- CORRECT
- **Token subtypes:** ["Zombie"] -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Create thirteen tapped 2/2 black Zombie creature tokens.\nFlashback {7}{B}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: ISSUE
### Code issues
1. **Oracle text wording mismatch (cosmetic)**: Oracle says `"Create thirteen tapped 2/2 black Zombie creature tokens."` but code oracle_text field says `"Create thirteen 2/2 black Zombie creature tokens. They enter the battlefield tapped."` The code uses an older template; the current oracle consolidates "tapped" into the create clause.
   - Code: `"Create thirteen 2/2 black Zombie creature tokens. They enter the battlefield tapped."`
   - Oracle: `"Create thirteen tapped 2/2 black Zombie creature tokens."`

Behavior is otherwise correct: creates 13 tokens with correct stats (2/2 black Zombie creature), sets each tapped, flashback_cost is {7}{B}{B}{B}. All functional behavior matches.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "Create thirteen tapped 2/2 black Zombie creature tokens." (was "Create thirteen 2/2 black Zombie creature tokens. They enter the battlefield tapped."). Doc comment updated. Behavior unchanged.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.
