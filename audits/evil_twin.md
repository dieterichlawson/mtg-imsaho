## Audit — 2026-04-01

**Scryfall Oracle text**: You may have Evil Twin enter the battlefield as a copy of any creature on the battlefield, except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature."
**Scryfall type line**: Creature — Shapeshifter
**Status**: ISSUE

- Mana cost {2}{U}{B}: correct.
- Type Creature, subtype Shapeshifter: correct.
- Power/Toughness 0/0: correct.
- Activated ability cost {U}{B}, tap: correct.
- Destroy ability uses `try_destroy`: correct pipeline for "destroy".
- Name matching check in `on_activate_ability`: correct.

**Issue 1 — "You may" is not respected.** The Oracle says "You may have Evil Twin enter..." making the copy optional. The implementation always copies a creature automatically (preferring opponent's creatures) without giving the player a choice. If no creatures are on the battlefield, it stays as a 0/0 and dies to SBA, which is correct, but the player should be able to choose NOT to copy even when creatures exist.

**Issue 2 — Player doesn't choose which creature to copy.** The implementation auto-selects (preferring opponent's creatures by `max_by_key`), but the Oracle says "any creature on the battlefield" — the player should choose.

- Tests exist in `tier15_cards.rs` (`evil_twin_copies_creature_on_etb`).
