# Audit: Feral Ridgewolf

## Reference (Scryfall)
- **Name:** Feral Ridgewolf
- **Cost:** {2}{R}
- **Type:** Creature -- Wolf
- **Oracle:** Trample. {1}{R}: Feral Ridgewolf gets +2/+0 until end of turn.
- **P/T:** 1/2

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{R})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Wolf)
- Oracle text: CORRECT
- P/T: CORRECT (1/2)
- Keywords: CORRECT (Trample)
- Activated ability cost: CORRECT ({1}{R})
- requires_tap: CORRECT (false)
- +2/+0 until end of turn: CORRECT (power_mod: 2, toughness_mod: 0)

## Issues
None found.

---

## Audit 2026-04-02

### Oracle Text (Scryfall, cached 2026-04-01)
```
Trample
{1}{R}: This creature gets +2/+0 until end of turn.
```

### Implementation vs Oracle

| Field | Oracle | Implementation | Verdict |
|---|---|---|---|
| Name | Feral Ridgewolf | `"Feral Ridgewolf"` | CORRECT |
| Mana cost | {2}{R} | `Generic(2), Colored(Red)` | CORRECT |
| Type line | Creature -- Wolf | `CardType::Creature`, subtypes `["Wolf"]` | CORRECT |
| P/T | 1/2 | `power: Some(1), toughness: Some(2)` | CORRECT |
| Keywords | Trample | `vec![Keyword::Trample]` | CORRECT |
| Activated ability cost | {1}{R} | `Generic(1), Colored(Red)` | CORRECT |
| Activated ability effect | +2/+0 until end of turn | `power_mod: 2, toughness_mod: 0` via `UntilEndOfTurnEffect` | CORRECT |
| requires_tap | (none) | `false` | CORRECT |
| once_per_turn | (none) | `false` | CORRECT |
| sorcery_speed_only | (none) | `false` | CORRECT |
| Zone restriction | (implied battlefield) | checks `zone == Zone::Battlefield` | CORRECT |

### Oracle Text String Mismatch (cosmetic)
- **Oracle (Scryfall):** `{1}{R}: This creature gets +2/+0 until end of turn.`
- **Implementation:** `{1}{R}: Feral Ridgewolf gets +2/+0 until end of turn.`
- Scryfall now uses the modern "This creature" templating. The implementation uses the card's name. These are functionally identical under MTG rules but the stored `oracle_text` string does not match Scryfall verbatim.

### Tests
- `feral_ridgewolf_has_correct_stats` -- PASS
- `feral_ridgewolf_gets_plus_2_plus_0` -- PASS
- `feral_ridgewolf_can_activate_multiple_times` -- PASS

All 3 tests in `mtg-engine/tests/activated_abilities.rs` pass.

### Conclusion
Implementation is functionally correct. One cosmetic mismatch in the `oracle_text` field: Scryfall uses "This creature" while the implementation uses "Feral Ridgewolf".
