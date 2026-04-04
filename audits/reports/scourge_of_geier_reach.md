# Audit: Scourge of Geier Reach

## Official Oracle
- **Name:** Scourge of Geier Reach
- **Cost:** {3}{R}{R}
- **Type:** Creature — Elemental
- **Oracle Text:** Scourge of Geier Reach gets +1/+1 for each creature your opponents control.
- **P/T:** 3/3

## Implementation Review
- **Name:** OK
- **Cost:** {3}{R}{R} — OK
- **Type:** Creature, subtypes ["Elemental"] — OK
- **Oracle Text:** Matches — OK
- **P/T:** 3/3 base — OK
- **dynamic_pt:** Returns (3 + opponent_creatures, 3 + opponent_creatures) — ISSUE

## Issues
None found. Verified that the engine's effective_power/effective_toughness uses dynamic_pt as a REPLACEMENT for base P/T (not additive), so returning (3 + N, 3 + N) is correct.

## Verdict: PASS

---

## Audit 2 (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
- **Name:** Scourge of Geier Reach
- **Cost:** {3}{R}{R}
- **Type:** Creature — Elemental
- **Oracle Text:** This creature gets +1/+1 for each creature your opponents control.
- **P/T:** 3/3

### Implementation: `mtg-engine/src/cards/isd/scourge_of_geier_reach.rs`

| Field | Oracle | Implementation | Status |
|-------|--------|----------------|--------|
| Name | Scourge of Geier Reach | "Scourge of Geier Reach" | PASS |
| Cost | {3}{R}{R} | Generic(3), Red, Red | PASS |
| Type | Creature — Elemental | CardType::Creature, subtypes: ["Elemental"] | PASS |
| Base P/T | 3/3 | power: Some(3), toughness: Some(3) | PASS |
| Oracle text | This creature gets +1/+1 for each creature your opponents control. | Stored text uses card name instead of "This creature" | PASS (functionally equivalent) |

### Dynamic P/T Analysis
- `dynamic_pt()` returns `Some((3 + opponent_creatures, 3 + opponent_creatures))`.
- The engine's `effective_power`/`effective_toughness` uses `dynamic_pt` as a **replacement** for base P/T (not additive), so baking the base 3/3 into the return value is correct.
- Opponent creature counting: filters `zone == Battlefield && controller == opponent && power.is_some()` -- standard creature detection pattern in this codebase. Correct.
- Uses `state.opponent(controller)` which returns a single opponent; valid for this 2-player engine. In multiplayer, "your opponents" would need to count across all opponents, but that is out of scope.

### Test Coverage (`mtg-engine/tests/tier12_cards.rs`)
- `scourge_of_geier_reach_scales_with_opponent_creatures`: 0 opponents -> 3/3, 2 opponents -> 5/5. PASS.
- `scourge_of_geier_reach_ignores_own_creatures`: 2 friendly creatures -> still 3/3. PASS.
- All 25 tier12 tests pass.

### Issues
None.

### Verdict: PASS

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: This creature gets +1/+1 for each creature your opponents control.
**Type line**: Creature — Elemental
**Status**: PASS

### Code issues
No issues found. Card data correct: cost {3}{R}{R}, P/T 3/3, Elemental subtype. The oracle_text in code uses "Scourge of Geier Reach" instead of the updated oracle template "This creature" -- this is a minor cosmetic difference reflecting original print vs. current oracle templating, not a behavioral issue. `dynamic_pt` correctly counts opponent creatures (objects on battlefield with `power.is_some()` controlled by opponent) and returns `(3 + N, 3 + N)` which overrides the base 3/3. The `opponent()` function returns a single opponent, appropriate for the 2-player engine scope.
