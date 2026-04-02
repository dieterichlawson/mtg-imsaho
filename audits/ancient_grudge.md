# Audit: Ancient Grudge

## Reference (Scryfall/API)
- **Name:** Ancient Grudge
- **Mana Cost:** {1}{R}
- **Type:** Instant
- **Oracle:** Destroy target artifact. Flashback {G}
- **P/T:** N/A

## Implementation: `ancient_grudge.rs`
- **Name:** Ancient Grudge -- CORRECT
- **Mana Cost:** {1}{R} -- CORRECT
- **Type:** Instant -- CORRECT
- **Flashback cost:** {G} -- CORRECT
- **Target:** PermanentWithFilter(HasCardType(Artifact)) -- CORRECT
- **Effect:** Destroy target artifact via `resolve_destroy` -- CORRECT (uses destruction pipeline)

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Destroy target artifact.\nFlashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Ancient Grudge", cost {1}{R}, type Instant, flashback_cost {G}. Target requirement correctly filters for artifacts on the battlefield. on_resolve delegates to resolve_destroy helper. All correct.
