## Audit — 2026-04-01

**Scryfall Oracle text (front)**: Vigilance\nWhenever another creature you control dies, you may transform Thraben Sentry.
**Scryfall Oracle text (back, Thraben Militia)**: Trample
**Scryfall type line (front)**: Creature — Human Soldier
**Scryfall type line (back)**: Creature — Human Soldier
**Status**: ISSUE

- Name: correct ("Thraben Sentry" / "Thraben Militia")
- Cost: {3}{W} -- correct
- Type: Creature -- correct
- Subtypes front: Human, Soldier -- correct
- Front P/T: 2/2 -- correct
- Back P/T: 5/4 -- correct
- Keywords front: Vigilance -- correct
- Keywords back: Trample -- correct

**Issue 1: "You may" is always auto-accepted.** The transform trigger says "you may transform", but the implementation always transforms (comment says "simplified 'you may' -- always yes"). This removes player agency.

**Issue 2: Transform is one-directional.** The implementation only transforms from front to back (checks `!is_transformed` before triggering). Once transformed to Thraben Militia, it cannot transform back. This is actually correct for this card -- Thraben Sentry is NOT a werewolf with the standard day/night cycle. It only transforms via its own trigger, which only fires on the front face. So this behavior is correct.

- Tests exist in `tier15_cards.rs`
