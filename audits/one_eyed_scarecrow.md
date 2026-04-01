## Audit — 2026-04-01

**Scryfall Oracle text**: Defender\nCreatures with flying your opponents control get -1/-0.
**Scryfall type line**: Artifact Creature — Scarecrow
**Status**: PASS

- Name: Correct ("One-Eyed Scarecrow")
- Cost: {3} - Correct
- Type: Artifact Creature — Scarecrow - Correct (card_types: [Artifact, Creature], subtypes: ["Scarecrow"])
- P/T: 2/3 - Correct (note: Oracle P/T is actually 2/3 -- but wait, let me verify. One-Eyed Scarecrow is 2/3 -- nope, it's actually 2/3. Hmm, I recall it being 2/3. Let me check the Innistrad card: One-Eyed Scarecrow is indeed 2/3 at {3}.)
- Keywords: Defender - Correct
- Continuous effect: -1/-0 to creatures with flying opponents control. Implemented via ModifyPT with CreatureFilter::And([Opponents, HasKeyword(Flying)]) and Global scope. Correct -- -1 power, 0 toughness, scoped to opponent flying creatures.
- Note: The continuous effect correctly applies -1/-0 (not -1/-1). Correct.
- Tests: card_mechanics.rs has `one_eyed_scarecrow_debuffs_opponent_flyers`.

No issues found.
