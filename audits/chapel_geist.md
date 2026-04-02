# Audit: Chapel Geist

## Scryfall Reference
- **Name:** Chapel Geist
- **Cost:** {1}{W}{W}
- **Type:** Creature -- Spirit
- **Oracle:** Flying
- **P/T:** 2/3
- **Keywords:** Flying

## Implementation: `chapel_geist.rs`
- **Name:** Chapel Geist -- CORRECT
- **Cost:** {1}{W}{W} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** ["Spirit"] -- CORRECT
- **P/T:** 2/3 -- CORRECT
- **Keywords:** [Flying] -- CORRECT

## Issues
None

## Audit (2026-04-02)

### Oracle Text (Scryfall)
- **Name:** Chapel Geist
- **Mana Cost:** {1}{W}{W}
- **Type Line:** Creature — Spirit
- **P/T:** 2/3
- **Oracle Text:** Flying
- **Keywords:** Flying

### Implementation (`mtg-engine/src/cards/isd/chapel_geist.rs`)
- **Name:** `"Chapel Geist"` — correct
- **Mana Cost:** `Generic(1), Colored(White), Colored(White)` — correct ({1}{W}{W})
- **Card Types:** `[Creature]` — correct
- **Supertypes:** `[]` — correct (none expected)
- **Subtypes:** `["Spirit"]` — correct
- **Power/Toughness:** `2/3` — correct
- **Oracle Text:** `"Flying"` — correct
- **Keywords:** `[Flying]` — correct

### LLM Reference (`mtg-player/src/llm.rs`)
- `Chapel Geist ({1}{W}{W} 2/3 flying): Solid flying body.` — correct

### Tests
Chapel Geist is used in multiple test files as a test creature:
- `tier2_spells.rs` (Spirit subtype check)
- `tier5_cards.rs` (lord/warden buff tests with expected 2/3 base stats)
- `tier7_cards.rs`, `tier12_cards.rs`, `flashback.rs`

All test references consistent with 2/3 Spirit with flying.

### Result
**No issues found.** All card data matches oracle text exactly.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
**Type line**: Creature — Spirit
**Status**: PASS

### Code issues
No issues found. Vanilla 2/3 flyer with correct mana cost {1}{W}{W}, correct subtypes, and Flying keyword.
