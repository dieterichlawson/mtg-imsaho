## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Morbid — When this creature enters, if a creature died this turn, you gain 5 life.
**Type line**: Creature — Elemental
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Morbid timing (intervening if): pass — condition checked at resolution, consistent with other morbid implementations
- Mandatory vs optional life gain: pass — no "you may", effect is mandatory
- Controller identification: pass — gains life for controller of Hollowhenge Scavenger
- Source leaving battlefield: pass — ETB trigger resolves even if source leaves (life gain doesn't require source)
- Creature death tracking: pass — `creature_died_this_turn` flag properly set in destruction.rs and sba.rs, reset each turn
- Multiple creature deaths: pass — morbid only requires at least one death, multiple deaths still satisfy condition
- Turn boundary reset: pass — `creature_died_this_turn` correctly reset to false at start of new turn

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Morbid flag set on creature death: `card_mechanics.rs:28` 
- Morbid flag resets on new turn: `card_mechanics.rs:43`
- Various morbid ETB abilities: `card_mechanics.rs:88` (Somberwald Spider), `tier5_cards.rs:215` (Festerhide Boar)
- Sacrifice triggers morbid: `card_mechanics.rs:989`
- Hollowhenge Scavenger specific implementation: NOT TESTED