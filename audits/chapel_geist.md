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

## Audit — 2026-04-02 20:41

**Oracle text source**: Scryfall API (cached 2026-04-01), via `python3 scripts/oracle_lookup.py lookup "Chapel Geist"`
**Oracle text**: Flying
**Type line**: Creature — Spirit
**Status**: PASS

### Code issues
None. All card data fields match oracle text exactly:
- Name: `"Chapel Geist"` matches oracle `Chapel Geist`
- Cost: `Generic(1), Colored(White), Colored(White)` matches oracle `{1}{W}{W}`
- Types: `[Creature]` with subtypes `["Spirit"]` matches oracle `Creature — Spirit`
- P/T: `Some(2)/Some(3)` matches oracle `2/3`
- Oracle text: `"Flying"` matches oracle `Flying`
- Keywords: `[Keyword::Flying]` matches oracle keyword `Flying`
- No extra behavior methods (resolve, on_*, activated_abilities) -- correct for vanilla flyer
- No flashback, continuous effects, triggered abilities, or additional costs -- correct

### Tricky interactions checked (min 3)
1. **Flying evasion**: Chapel Geist used in `tier5_cards.rs::orchard_spirit_blocked_by_flyer` to verify a creature with Flying can block Orchard Spirit (which can't be blocked except by creatures with flying or reach). Confirms Flying keyword is functional.
2. **Spirit lord buffs**: In `tier5_cards.rs::battleground_geist_buffs_other_spirits`, Chapel Geist (2/3 base) becomes 3/3 with Battleground Geist's +1/+0 to other Spirits. In `gallows_warden_buffs_other_spirits`, Chapel Geist becomes 2/4 with Warden's +0/+1. Confirms Spirit subtype is recognized by lords.
3. **Token copy fidelity**: In `tier12_cards.rs::cackling_counterpart_creates_token_copy`, a token copy of Chapel Geist is created and verified to have power 2, toughness 3, confirming card data is correctly propagated to copies.
4. **Spirit lord doesn't buff opponents**: In `tier5_cards.rs::spirit_lord_doesnt_buff_opponent`, an opponent's Chapel Geist is NOT buffed by a friendly Battleground Geist (remains 2/3), confirming controller-scoped lord effects.

### Test coverage
Chapel Geist appears in 5 test files as a test creature:
- `tier2_spells.rs`: Spirit subtype targeted by Urgent Exorcism
- `tier5_cards.rs`: Lord buff tests (Battleground Geist, Gallows Warden), opponent non-buff, flying block interaction
- `tier7_cards.rs`: Angel of Flight Alabaster Spirit graveyard return
- `tier12_cards.rs`: Cackling Counterpart token copy
- `flashback.rs`: Rolling Temblor interaction with flyers

No dedicated unit test needed -- vanilla creature with keyword only. All test references consistent with 2/3 Spirit with Flying.
