# Audit: Unruly Mob

## Scryfall Reference
- **Name:** Unruly Mob
- **Cost:** {1}{W}
- **Type:** Creature — Human
- **Oracle:** Whenever another creature you control dies, put a +1/+1 counter on this creature.
- **P/T:** 1/1

## Implementation: `mtg-engine/src/cards/unruly_mob.rs`
- Name: "Unruly Mob" -- MATCH
- Cost: {1}{W} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Human"] -- MATCH
- P/T: 1/1 -- MATCH
- Trigger: AnyCreatureDies -- MATCH
- on_any_creature_dies: Filters to dead_controller == controller (another creature YOU control) -- CORRECT
- Adds +1/+1 counter -- MATCH

## Verdict
**PASS** — Correctly implemented.

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Whenever another creature you control dies, put a +1/+1 counter on this creature.
**Type line**: Creature — Human
**Status**: PASS

### Card Data
- **Name:** Unruly Mob -- CORRECT
- **Mana Cost:** {1}{W} -- CORRECT
- **Type:** Creature — Human -- CORRECT
- **P/T:** 1/1 -- CORRECT

### Code issues
None. Trigger uses AnyCreatureDies, filters by dead_controller == controller (ensuring "you control"), and adds a PlusOnePlusOne counter. The "another" condition is implicitly satisfied because a dead creature is no longer on the battlefield when this triggers. All data and behavior match oracle.
