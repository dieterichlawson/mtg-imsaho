## Audit — 2026-04-02 21:03
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/182/gatstaf-shepherd-gatstaf-howler)
**Oracle text (front — Gatstaf Shepherd)**: At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Oracle text (back — Gatstaf Howler)**: Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line (front)**: Creature — Human Werewolf
**Type line (back)**: Creature — Werewolf
**Status**: PASS

### Code issues
No issues found.

- Front face: name, cost {1}{G}, subtypes [Human, Werewolf], P/T 2/2, no keywords — all correct.
- Back face: name "Gatstaf Howler", no cost, subtypes [Werewolf], P/T 3/3, keyword Intimidate — all correct.
- Transform condition (human->wolf): `total_spells == 0 && !is_first_turn` — matches oracle "if no spells were cast last turn."
- Transform condition (wolf->human): `any player cast >= 2` — matches oracle "if a player cast two or more spells last turn."
- `dynamic_pt` returns (3,3) when transformed, None otherwise (uses base 2/2) — correct.
- `on_upkeep` toggles `is_transformed`, updates name — correct.
- Trigger fires on both faces via front-face `TriggerKind::Upkeep` (engine checks front face triggers first regardless of transform state) — correct.

### Tricky interactions checked (min 3)
1. **Intimidate blocking enforcement**: `combat.rs::can_block_attacker` checks `has_keyword(attacker, Intimidate)` and correctly restricts blockers to artifact creatures or same-color creatures. `has_keyword` reads back face keywords when `is_transformed` is true, so Gatstaf Howler gets Intimidate. Colors are derived from front face mana cost (Green) and persist through transform — correct per DFC color rules.
2. **Subtype changes on transform**: `matches_filter` for `HasSubtype` uses back face subtypes when transformed. Gatstaf Howler has only "Werewolf" (no "Human"), so effects that care about Human/Werewolf subtypes work correctly for both faces.
3. **First turn guard**: Front-to-back transform is gated by `!state.is_first_turn`, preventing the no-spells condition from being trivially true on the first turn of the game (no "last turn" exists).
4. **Multiple werewolves transform together**: Each werewolf independently checks the same condition in `on_upkeep`, so all transform simultaneously on the same upkeep step. Verified by `multiple_werewolves_transform_on_same_upkeep` test.

### Test coverage
- `gatstaf_shepherd_transforms_and_gains_intimidate`: Verifies front-to-back transform, name change, P/T change (3/3), and Intimidate keyword present.
- `gatstaf_shepherd_loses_intimidate_on_transform_back`: Verifies back-to-front transform removes Intimidate.
- `multiple_werewolves_transform_on_same_upkeep`: Includes Gatstaf Shepherd among multiple werewolves transforming together.
- `multiple_werewolves_transform_back_together`: Includes Gatstaf Shepherd in multi-werewolf transform-back test.
