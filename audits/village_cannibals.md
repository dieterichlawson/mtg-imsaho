# Audit: Village Cannibals

## Scryfall Reference
- **Name:** Village Cannibals
- **Cost:** {2}{B}
- **Type:** Creature — Human
- **Oracle:** Whenever another Human creature dies, put a +1/+1 counter on this creature.
- **P/T:** 2/2

## Implementation: `mtg-engine/src/cards/village_cannibals.rs`
- Name: "Village Cannibals" -- MATCH
- Cost: {2}{B} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Human"] -- MATCH
- P/T: 2/2 -- MATCH
- Trigger: AnyCreatureDies -- MATCH
- on_any_creature_dies: Checks if dead creature is Human (any controller) -- CORRECT (oracle says "another Human creature", not "another Human creature you control")
- Adds +1/+1 counter -- MATCH

## Verdict
**PASS** — Correctly triggers on any Human creature dying.

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Whenever another Human creature dies, put a +1/+1 counter on this creature.
**Mana cost**: {2}{B}
**Type line**: Creature — Human
**P/T**: 2/2
**Status**: ISSUE
### Code issues
1. **Oracle text string mismatch**: Oracle says `"put a +1/+1 counter on this creature"` but code has `"put a +1/+1 counter on Village Cannibals"`. The oracle template uses "this creature" rather than the card name.
### Behavior
Behavior is correct: on_any_creature_dies checks self is on battlefield, checks the dead creature had the "Human" subtype, and adds a +1/+1 counter. Trigger kind is AnyCreatureDies. Logic is sound.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "put a +1/+1 counter on this creature" (was "on Village Cannibals"). Doc comment updated. Behavior unchanged.
