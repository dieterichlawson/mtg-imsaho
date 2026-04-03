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

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Trample
{1}{R}: This creature gets +2/+0 until end of turn.
**Type line**: Creature — Wolf
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:58
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Trample
{1}{R}: This creature gets +2/+0 until end of turn.
**Type line**: Creature — Wolf
**Status**: PASS

### Code issues
- Cosmetic only: `oracle_text` field uses "Feral Ridgewolf" instead of "This creature" (modern Scryfall templating). Functionally equivalent; no gameplay impact.

### Tricky interactions checked (min 3)
1. **Multiple activations stack**: Two activations produce power_mod 2+2=4, so 1+4=5 total power. Confirmed by `feral_ridgewolf_can_activate_multiple_times` test.
2. **Trample is always present**: Keyword::Trample is in the static keywords vec, always active on battlefield. No conditional granting needed.
3. **Pump expires at end of turn**: `until_end_of_turn_effects` is cleared in the cleanup step (engine.rs line 3021), so +2/+0 correctly wears off.
4. **Ability only available on battlefield**: Zone check at line 33 ensures the activated ability is not offered when the card is in hand/graveyard/etc.
5. **Instant-speed activation**: `sorcery_speed_only: false` allows activation during combat (e.g., after blocks declared), which is correct for this card.

### Test coverage
- `feral_ridgewolf_has_correct_stats` — verifies P/T (1/2), Trample keyword, Wolf subtype
- `feral_ridgewolf_gets_plus_2_plus_0` — verifies single activation yields 3/2
- `feral_ridgewolf_can_activate_multiple_times` — verifies two activations yield 5/2

All 3 tests PASS.
