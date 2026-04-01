## Audit — 2026-04-01

**Scryfall Oracle text**: Trample\nMorbid — Festerhide Boar enters the battlefield with two +1/+1 counters on it if a creature died this turn.
**Scryfall type line**: Creature — Boar
**Status**: ISSUE

- Mana cost {3}{G}: correct.
- Type Creature, subtype Boar: correct.
- Power/Toughness 3/3: correct.
- Keywords: Trample: correct.
- Morbid check via `state.creature_died_this_turn`: correct.
- Adds 2 +1/+1 counters when morbid: correct.

**Issue — Oracle text wording difference.** The actual Oracle text says "Festerhide Boar enters the battlefield with two +1/+1 counters on it if a creature died this turn." This is a static replacement effect (enters WITH counters), not a triggered ability. The implementation uses `TriggerKind::EntersBattlefield` and `on_enter_battlefield` which functions as a triggered ability. In practice the difference is minimal (the counters are applied immediately on ETB), but technically this should not use the stack as a triggered ability. The oracle_text field in the code uses "When Festerhide Boar enters the battlefield" phrasing which is incorrect for a replacement/static ability.

- Tests exist in `tier5_cards.rs` (`festerhide_boar_morbid`, `festerhide_boar_no_morbid`).
