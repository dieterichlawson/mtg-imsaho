## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Indestructible
Whenever this creature deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.
**Type line**: Artifact Creature — Construct
**Status**: ISSUE

### Code issues
- Incorrect battlefield check in trigger resolution (`mtg-engine/src/cards/isd/creepy_doll.rs:39-41`)
  - Oracle text says: `flip a coin. If you win the flip, destroy that creature.`
  - Code does: Checks if Creepy Doll is still on battlefield before resolving the ability, but the oracle text doesn't reference the source in its resolution, so it should resolve even if Creepy Doll has left the battlefield

### Tricky interactions checked
- Coin flip timing (resolves when ability resolves, not when triggers): pass
- Indestructible target interaction: pass (uses try_destroy which handles indestructible)
- Combat damage vs non-combat damage distinction: pass
- Trigger only on creature damage, not player damage: pass
- Lethal damage + coin flip interaction: pass (ability still triggers even if damage was lethal)
- Source leaves battlefield between trigger and resolution: fail (incorrectly prevents resolution)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Correct trigger kind (DealsCombatDamageToCreature): `tests/creepy_doll.rs:21-37`
- Indestructible keyword presence: `tests/creepy_doll.rs:39-46`
- Triggers on combat damage to creature: `tests/creepy_doll.rs:48-75`
- Does not trigger on combat damage to player: `tests/creepy_doll.rs:77-101`
- Destruction on coin flip win: `tests/creepy_doll.rs:103-128`
- Coin flip happens at resolution time: NOT TESTED
- Ability resolves when source has left battlefield: NOT TESTED
- Interaction with indestructible targets: NOT TESTED
- Interaction with regeneration: NOT TESTED