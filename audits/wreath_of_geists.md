## Audit — 2026-08-27 — CR 109.1: a token in a graveyard is not a card

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/211/wreath-of-geists?utm_source=api
**Oracle text**:
```
Enchant creature
Enchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.
```
**Status**: ISSUE (fixed)

### Code issue
- Oracle text says: a **card** in a graveyard (`Enchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.`)
- Code did: filtered the graveyard by creature-ness alone, with no card/token distinction.
- CR 109.1: a token is not a card. CR 111.7 removes a token from a graveyard as
  a state-based action, so between the moment it dies and the next SBA check it
  really is sitting there — the same window a dies-trigger sees. Measured
  directly on Boneyard Wurm: 2/2 with one creature card and one just-died token
  in the yard, 1/1 the instant SBAs ran. The oracle's answer is 1/1 throughout.
- Fixed: the graveyard filter now goes through `state.is_card`.

### How this was found
A sweep for cards whose oracle says "cards in a graveyard" against code that
never distinguishes tokens. Thirteen cards matched; five already guarded
(Gnaw to the Bone, Moorland Haunt, Past in Flames, Runechanter's Pike,
Splinterfright) and eight did not.

Splinterfright and Boneyard Wurm are the instructive pair — near-identical
text, adjacent in the set. `token_is_not_a_card.rs::cda_does_not_count_tokens_in_graveyard`
covered Splinterfright, which is why Splinterfright alone had the guard.

### Test coverage
`token_is_not_a_card.rs::a_token_in_a_graveyard_is_not_a_creature_card` —
**added by this audit**, covers Boneyard Wurm and Splinterfright together and
fails against the unfixed code.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/211/wreath-of-geists?utm_source=api
**Type line**: `Enchantment — Aura` — {G}
**Oracle text**:
```
Enchant creature
Enchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The value of X is **constantly updated** as creature cards are put
  into or removed from your graveyard." `dynamic_pt`, recomputed every time P/T
  is asked for: PASS
- "**your** graveyard" is the *Aura's controller's*, not the enchanted
  creature's — `dynamic_pt` is called with the Aura's own object id, so it reads
  the right player even on a stolen creature: PASS
- CR 109.1: "creature **cards**", so tokens are excluded: PASS
- The bonus ends when the Aura leaves: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The CDA-style bonus: `enchantments.rs`, `token_is_not_a_card.rs`
