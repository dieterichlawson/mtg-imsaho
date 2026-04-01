## Audit — 2026-04-01

**Scryfall Oracle text**: Equipped creature has flying.
Equip {1}
**Scryfall type line**: Artifact — Equipment
**Status**: PASS

### Findings

1. **Card data correct**: Name, cost ({2}), types (Artifact), subtype (Equipment), no P/T all match.

2. **Continuous effect correct**: Grants Flying to attached creature via `EffectScope::Attached`.

3. **Equip ability correct**: Cost {1}, sorcery speed only, targets creature.

4. **Equip target validation correct**: Only allows targeting creatures you control (line 52-53).

5. **on_resolve sets is_equipment flag**: Correct.

6. **Tests**: No dedicated tests found.
