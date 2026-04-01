## Audit — 2026-04-01

**Scryfall Oracle text**: Diregraf Ghoul enters the battlefield tapped.
**Scryfall type line**: Creature — Zombie
**Status**: PASS

- Mana cost {B}: correct.
- Type Creature: correct.
- Subtype Zombie: correct.
- Power/Toughness 2/2: correct.
- Enters tapped implemented via `on_resolve` setting `obj.tapped = true` after moving to battlefield: correct.
- No keywords: correct.
- Tests exist in `innistrad_cards.rs` (`diregraf_ghoul_enters_tapped`).
