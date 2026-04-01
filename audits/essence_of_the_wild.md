## Audit — 2026-04-01

**Scryfall Oracle text**: Creatures you control enter the battlefield as a copy of Essence of the Wild.
**Scryfall type line**: Creature — Avatar
**Status**: PASS

- Mana cost {3}{G}{G}{G}: correct.
- Type Creature, subtype Avatar: correct.
- Power/Toughness 6/6: correct.
- Replacement effect modeled as AnyCreatureEnters trigger: noted simplification, acceptable.
- Only affects creatures controlled by Essence's controller: correct.
- Does not affect Essence itself (self_id check): correct.
- Overrides power/toughness to 6/6, subtypes to Avatar, clears keywords: correct.
- Changes name to "Essence of the Wild": correct for a copy effect.
- TriggerKind::AnyCreatureEnters in triggered_abilities: correct.
- No dedicated tests found, but tested implicitly through the trigger system.

## Audit — 2026-04-01

**Scryfall Oracle text**: Creatures you control enter as a copy of this creature.
**Scryfall type line**: Creature — Avatar
**Status**: ISSUE

1. **No test coverage**: No test files found for this card. Missing tests for the copy replacement effect.
2. **TriggerKind mismatch**: Uses `TriggerKind::AnyCreatureEnters` and `on_any_creature_enters`, but this is a replacement effect, not a triggered ability. The engine treats it as a trigger which could cause timing differences (replacement effects happen before ETB triggers). File: `mtg-engine/src/cards/essence_of_the_wild.rs`, lines 34-39.
3. **Oracle text mismatch (cosmetic)**: Code says "Creatures you control enter the battlefield as a copy of Essence of the Wild." but Scryfall says "Creatures you control enter as a copy of this creature." (updated template). File: `mtg-engine/src/cards/essence_of_the_wild.rs`, line 29.
