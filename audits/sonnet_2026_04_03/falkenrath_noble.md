## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
**Type line**: Creature — Vampire Noble
**Status**: ISSUE

### Code issues
- Incorrect targeting implementation at mtg-engine/src/cards/isd/falkenrath_noble.rs:60-61
  - Oracle text says: `target player loses 1 life`
  - Code does: `let opponent = state.opponent(controller);` - auto-targets opponent without player choice

### Tricky interactions checked
- "this creature or another creature dies": pass - correctly has both TriggerKind::SelfDies and TriggerKind::AnyCreatureDies
- "target player" choice: fail - should present targeting choice but auto-targets opponent
- Simultaneous death ruling ("Noble and another creature die at same time"): uncertain - trigger collection logic filters by zone == Battlefield which may prevent Noble from seeing other deaths if moved to graveyard first
- "you gain 1 life" vs "target player loses 1 life" resolution order: pass - code applies both effects in sequence
- Life total changes properly tracked: pass - generates LifeChanged events correctly

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Noble triggers on own creature death: `mtg-engine/tests/bug_fixes.rs:426`
- Noble triggers on opponent creature death: `mtg-engine/tests/bug_fixes.rs:401`
- Noble triggers on self death: `mtg-engine/tests/bug_fixes.rs:449`
- Basic drain effect (1 life loss/gain): `mtg-engine/tests/tier3_cards.rs:283`
- APNAP trigger ordering with Noble: `mtg-engine/tests/apnap.rs:94`
- Targeting choice presentation: NOT TESTED
- Simultaneous death ruling (Noble + another die together): NOT TESTED
- Player choice to target any player (not just opponent): NOT TESTED