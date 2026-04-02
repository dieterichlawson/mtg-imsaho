# Audit: Ghoulcaller's Bell

## Oracle Reference (Scryfall)
- Cost: {1}
- Type: Artifact
- Oracle: "{T}: Each player mills a card."

## Implementation: ghoulcallers_bell.rs

## Issues Found

No issues found. Name, cost ({1}), type (Artifact), oracle text, and activated ability all match. The tap ability correctly mills 1 card from each player using crate::engine::mill_cards.

## Verdict: PASS

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
```
{T}: Each player mills a card.
```

### Findings
- Name, cost ({1}), type (Artifact) all match.
- Oracle text in code matches Scryfall oracle.
- Activated ability: {T}, no mana cost, no sacrifice -- correct.
- On resolution: iterates all players, mills 1 card each -- correct.

### Verdict: PASS
