# Audit: Vampire Interloper

## Scryfall Reference
- **Name:** Vampire Interloper
- **Cost:** {1}{B}
- **Type:** Creature — Vampire Scout
- **Oracle:** Flying / This creature can't block.
- **P/T:** 2/1

## Implementation: `mtg-engine/src/cards/vampire_interloper.rs`
- Name: "Vampire Interloper" -- MATCH
- Cost: {1}{B} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Vampire", "Scout"] -- MATCH
- P/T: 2/1 -- MATCH
- Keywords: [Flying] -- MATCH
- Continuous effects: [PreventBlock { scope: OnSelf }] -- MATCH ("can't block")

## Verdict
**PASS** — Correctly implemented with flying and can't-block restriction.

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Flying / This creature can't block.
**Type line**: Creature — Vampire Scout
**Status**: PASS

### Card Data
- **Name:** Vampire Interloper -- CORRECT
- **Mana Cost:** {1}{B} -- CORRECT
- **Type:** Creature — Vampire Scout -- CORRECT
- **P/T:** 2/1 -- CORRECT
- **Keywords:** Flying -- CORRECT
- **Continuous Effects:** PreventBlock (OnSelf) -- CORRECT

### Code issues
None. Flying keyword is set, can't-block restriction is implemented via ContinuousEffect::PreventBlock with EffectScope::OnSelf. All data and behavior match oracle.
