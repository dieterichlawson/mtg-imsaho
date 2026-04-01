## Audit — 2026-04-01

**Scryfall Oracle text**: Hexproof\nWhenever a creature dies, put a +1/+1 counter on Lumberknot.
**Scryfall type line**: Creature — Treefolk
**Status**: PASS

- Name: Lumberknot -- correct
- Cost: {2}{G}{G} -- correct
- Type: Creature -- correct
- Subtypes: Treefolk -- correct
- P/T: 1/1 -- correct
- Keywords: Hexproof -- correct
- Triggered ability: any creature dies -> +1/+1 counter -- correctly implemented via on_any_creature_dies
- Checks that Lumberknot is on the battlefield before adding counter -- correct
- Tests exist in tier3_cards.rs

No issues found. Implementation matches Oracle text.

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: Hexproof. Whenever a creature dies, put a +1/+1 counter on Lumberknot.
**Scryfall type line**: Creature -- Treefolk
**Status**: PASS

No issues found. Note: "a creature" (not "another creature") means Lumberknot dying to lethal damage would not trigger itself (since it's no longer on the battlefield). The implementation checks zone == Battlefield before adding counter, which is correct.
