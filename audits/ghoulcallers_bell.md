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

---

## Audit — 2026-04-02 21:09
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: {T}: Each player mills a card.
**Type line**: Artifact
**Status**: PASS

### Code issues
None found. Implementation is correct:
- Name: "Ghoulcaller's Bell" -- matches oracle
- Cost: {1} (Generic(1)) -- matches oracle
- Type: Artifact -- matches oracle
- Oracle text in CardData: "{T}: Each player mills a card." -- matches Scryfall exactly
- Activated ability: requires_tap=true, cost=free, no targets, no sacrifice -- correct for {T} ability
- on_activate_ability: iterates all players via state.players, calls mill_cards(state, pid, 1) for each -- correct
- Zone check: ability only offered when on battlefield and untapped (line 34) -- correct
- once_per_turn: false -- correct, no such restriction on the card
- sorcery_speed_only: false -- correct, artifact tap abilities can be used at instant speed
- keywords: vec![] -- correct, "mill" is a keyword action, not a keyword ability

### Tricky interactions checked (min 3)
1. **Empty library**: mill_cards handles empty library gracefully -- it breaks out of the loop if library_order is empty (engine.rs:2760-2762). No crash or forced draw.
2. **Multiple players / iteration order**: The ability says "each player" which is simultaneous. Since mill involves no choices, the iteration order (player 0 then player 1) does not affect game correctness.
3. **Tap cost as limiting factor**: The ability correctly uses requires_tap=true with no mana cost. This means it can only be used once per turn cycle (until untapped), and cannot be used when the artifact is already tapped or has summoning sickness (artifacts without creature type don't have summoning sickness, which is correct).
4. **No targeting**: The ability has target_requirement: None, which is correct -- "each player" does not target. This means it cannot be countered by effects that counter targeted abilities, and is not affected by hexproof/shroud.

### Test coverage
- `ghoulcallers_bell_card_data`: Verifies card type (Artifact) and mana value (1). PASS.
- `ghoulcallers_bell_mills_both_players`: Sets up libraries for both players, activates the Bell, and verifies both players' top cards moved to graveyard. PASS.
- Missing: No test for activating with empty library (edge case), no test verifying the Bell cannot be activated when tapped.
