# Audit: Grizzled Outcasts / Krallenhorde Wantons

## Oracle Reference (Scryfall)
**Front Face: Grizzled Outcasts**
- Cost: {4}{G}
- Type: Creature -- Human Werewolf
- P/T: 4/4
- Oracle: "At the beginning of each upkeep, if no spells were cast last turn, transform Grizzled Outcasts."

**Back Face: Krallenhorde Wantons**
- Type: Creature -- Werewolf
- P/T: 7/7
- Oracle: "At the beginning of each upkeep, if a player cast two or more spells last turn, transform Krallenhorde Wantons."

## Implementation: grizzled_outcasts.rs

## Issues Found

No issues found. Front and back face names, types, subtypes, P/T values, and oracle text all match. Werewolf transform logic is correct (same pattern as other werewolves).

## Verdict: PASS

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
**Front:**
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back (Krallenhorde Wantons):**
```
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

### Findings
- Name, cost ({4}{G}), type (Creature -- Human Werewolf), P/T (4/4 / 7/7) all match.
- Back face: Krallenhorde Wantons, Creature -- Werewolf, 7/7 -- correct.
- Transform logic: front transforms when no spells cast last turn (and not first turn); back transforms when any player cast 2+ spells -- correct.
- on_upkeep correctly checks battlefield zone, toggles is_transformed, updates name -- correct.
- dynamic_pt returns (7,7) when transformed -- correct.

### Verdict: PASS

---

## Audit — 2026-04-02 21:12
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text (front — Grizzled Outcasts)**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Type line (front)**: Creature — Human Werewolf
**Mana cost**: {4}{G}
**P/T (front)**: 4/4

**Oracle text (back — Krallenhorde Wantons)**:
```
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```
**Type line (back)**: Creature — Werewolf
**P/T (back)**: 7/7

**Status**: PASS

### Code issues
- **Minor (cosmetic only)**: Oracle text in implementation uses "transform Grizzled Outcasts" / "transform Krallenhorde Wantons" instead of the current oracle wording "transform this creature." This does not affect behavior.
- No functional issues found.

### Tricky interactions checked (min 3)
1. **First turn guard**: Front face does NOT transform on the first turn of the game — correctly enforced by `!state.is_first_turn` in `werewolf_should_transform`.
2. **Back face "a player" check**: The back-to-front condition checks whether ANY single player cast 2+ spells (`values().any(|&count| count >= 2)`), not total spells across all players. This matches the oracle text "if a player cast two or more spells."
3. **Multiple werewolves transform simultaneously**: Grizzled Outcasts transforms alongside other werewolves on the same upkeep trigger (tested in `multiple_werewolves_transform_on_same_upkeep`).
4. **Zone check**: `on_upkeep` returns early if the creature is not on the battlefield, preventing ghost transforms.

### Test coverage
- `grizzled_outcasts_transforms_to_7_7` — verifies transform to 7/7 and name becomes "Krallenhorde Wantons"
- `multiple_werewolves_transform_on_same_upkeep` — verifies Grizzled Outcasts transforms alongside other werewolves
- Shared werewolf tests (`werewolf_side_stays_if_one_spell_cast`, `human_side_stays_if_any_spell_cast`, `multiple_werewolves_transform_back_together`) cover the transform logic pattern used by this card
- No dedicated transform-back test for this specific card, but the logic is identical to other werewolves which are thoroughly tested
