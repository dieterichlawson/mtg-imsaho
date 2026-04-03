# Audit: Diregraf Ghoul

## Reference (Scryfall)
- **Name:** Diregraf Ghoul
- **Cost:** {B}
- **Type:** Creature -- Zombie
- **Oracle:** Diregraf Ghoul enters the battlefield tapped.
- **P/T:** 2/2

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({B})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Zombie)
- Oracle text: CORRECT
- P/T: CORRECT (2/2)
- ETB tapped: CORRECT (sets obj.tapped = true in on_resolve)

## Issues
None found.

---

## Audit 2 (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
```
This creature enters tapped.
```

### Card Data Check
| Field        | Expected (Oracle)              | Implemented                                         | Status   |
|--------------|-------------------------------|-----------------------------------------------------|----------|
| Name         | Diregraf Ghoul               | `"Diregraf Ghoul"`                                  | CORRECT  |
| Cost         | {B}                          | `[ManaSymbol::Colored(Color::Black)]`               | CORRECT  |
| Type         | Creature -- Zombie           | `CardType::Creature`, subtypes `["Zombie"]`         | CORRECT  |
| P/T          | 2/2                          | `power: Some(2), toughness: Some(2)`                | CORRECT  |
| Oracle text  | "This creature enters tapped." | `"Diregraf Ghoul enters the battlefield tapped."`  | MISMATCH |

### Oracle Text Mismatch Detail
- **Scryfall (current):** `"This creature enters tapped."`
- **Implementation:** `"Diregraf Ghoul enters the battlefield tapped."`

The card received updated oracle text with the Bloomburrow template changes (2024). The old wording "enters the battlefield" was shortened to "enters", and self-referencing card names were replaced with "this creature". The implementation still uses the pre-Bloomburrow wording. This is a cosmetic-only mismatch; the functional behavior is identical.

### ETB Tapped Behavior
The `on_resolve` method (line 29-34) moves the object to the battlefield and immediately sets `obj.tapped = true`. This correctly implements the "enters tapped" replacement effect. The comment in the source code correctly notes this is a replacement effect, not a triggered ability.

### Test Coverage
- `diregraf_ghoul_enters_tapped` in `mtg-engine/tests/innistrad_cards.rs` -- verifies the card is on the battlefield and tapped after resolving. **PASSES.**
- Diregraf Ghoul also used as a Zombie fixture in tests for Ghoulcaller's Chant, Unbreathing Horde, and Elite Inquisitor.

### Issues
1. **COSMETIC -- Oracle text outdated:** The `oracle_text` field uses pre-Bloomburrow wording. Should be updated to `"This creature enters tapped."` to match current Scryfall oracle text.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: This creature enters tapped.
**Type line**: Creature — Zombie
**Status**: PASS

### Code issues
No issues found. Minor note: code oracle_text string uses older wording "Diregraf Ghoul enters the battlefield tapped." vs current oracle "This creature enters tapped." — no behavioral impact.

## Audit — 2026-04-02 20:54
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: This creature enters tapped.
**Type line**: Creature — Zombie
**Status**: PASS

### Code issues
None. The oracle_text field uses pre-Bloomburrow wording ("Diregraf Ghoul enters the battlefield tapped." vs "This creature enters tapped.") but this is cosmetic only and does not affect behavior.

### Tricky interactions checked (min 3)
1. **Enters tapped is a replacement effect, not a trigger.** The implementation correctly handles this via `on_resolve` (setting `tapped = true` immediately after `move_object`) rather than via `on_enter_battlefield`. The source comment explicitly notes this distinction. This means it cannot be responded to, which is correct.
2. **ETB event still fires after tapped is set.** `state.move_object()` emits `EnteredBattlefield` event, and `obj.tapped = true` is set after. Since triggers are processed after spell resolution, any ETB-watching triggers (e.g. Unruly Mob, Champion of the Parish) will correctly see the Ghoul on the battlefield in a tapped state.
3. **Interaction with Village Bell-Ringer.** If Village Bell-Ringer's ETB resolves after Diregraf Ghoul enters, it untaps all creatures you control -- including the Ghoul. The implementation handles this correctly since VBR's `on_enter_battlefield` iterates all tapped creatures and untaps them.
4. **Zone-change resets.** If Diregraf Ghoul leaves the battlefield and returns (e.g. via Fiend Hunter exile/return), `move_object` resets `tapped = false` on exit, and on re-entry the card's `on_resolve` would set it tapped again. This is correct per MTG rules -- the "enters tapped" ability applies each time.

### Test coverage
- `diregraf_ghoul_enters_tapped` in `mtg-engine/tests/innistrad_cards.rs`: verifies card is on battlefield and tapped after resolving. PASSES.
- Used as Zombie fixture in tests for: Elite Inquisitor (blocking), Ghoulcaller's Chant (graveyard return), Unbreathing Horde (counter counting).
