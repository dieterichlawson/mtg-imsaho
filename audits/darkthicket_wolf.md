# Audit: Darkthicket Wolf

## Scryfall Reference
- **Name:** Darkthicket Wolf
- **Cost:** {1}{G}
- **Type:** Creature -- Wolf
- **Oracle:** {2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.
- **P/T:** 2/2
- **Keywords:** none

## Implementation: `darkthicket_wolf.rs`
- **Name:** Darkthicket Wolf -- CORRECT
- **Cost:** {1}{G} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** ["Wolf"] -- CORRECT
- **P/T:** 2/2 -- CORRECT
- **Keywords:** none -- CORRECT
- **Activated ability:** {2}{G}: +2/+2 until end of turn -- CORRECT
- **Once per turn:** true -- CORRECT

## Issues
None

---

## Audit 2026-04-02

### Oracle Text (Scryfall, cached 2026-04-01)
{2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.

### Card Data
| Field       | Oracle             | Implementation          | Status  |
|-------------|--------------------|-------------------------|---------|
| Name        | Darkthicket Wolf   | "Darkthicket Wolf"      | CORRECT |
| Mana cost   | {1}{G}             | Generic(1), Green       | CORRECT |
| Type        | Creature -- Wolf   | Creature, ["Wolf"]      | CORRECT |
| P/T         | 2/2                | 2/2                     | CORRECT |
| Keywords    | (none)             | []                      | CORRECT |

### Activated Ability
| Field            | Oracle                        | Implementation                    | Status  |
|------------------|-------------------------------|-----------------------------------|---------|
| Cost             | {2}{G}                        | Generic(2), Green                 | CORRECT |
| Effect           | +2/+2 until end of turn       | power_mod: 2, toughness_mod: 2    | CORRECT |
| Once per turn    | Activate only once each turn  | once_per_turn: true               | CORRECT |
| Requires tap     | (no tap symbol)               | requires_tap: false               | CORRECT |
| Zone restriction | (implicit: battlefield)       | Zone::Battlefield check           | CORRECT |

### Oracle Text String
- **Oracle (Scryfall):** "{2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn."
- **Implementation oracle_text:** "{2}{G}: Darkthicket Wolf gets +2/+2 until end of turn. Activate only once each turn."
- **Note:** Scryfall uses "This creature" (modern template); implementation uses card name. Functionally equivalent; cosmetic only.

### Tests
- `darkthicket_wolf_has_correct_stats` -- PASS
- `darkthicket_wolf_gets_plus_2_plus_2` -- PASS
- `darkthicket_wolf_once_per_turn` -- PASS

### Verdict
**PASS** -- Implementation is fully correct. One cosmetic note: the `oracle_text` field uses the card's name instead of "This creature" per the current Scryfall template, but this has no mechanical impact.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: {2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.
**Type line**: Creature — Wolf
**Status**: ISSUE

### Code issues
Minor oracle text mismatch: code uses `"{2}{G}: Darkthicket Wolf gets +2/+2 until end of turn. Activate only once each turn."` but current oracle uses `"This creature"` template instead of the card name. Behavior is fully correct: activated ability costs {2}{G}, grants +2/+2 via UntilEndOfTurnEffect, once_per_turn is true, does not require tap. P/T 2/2 and cost {1}{G} match. Subtypes ["Wolf"] match.
