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
