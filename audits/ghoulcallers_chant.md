## Audit — 2026-08-27 — CR 109.1: a token in a graveyard is not a card

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/101/ghoulcallers-chant?utm_source=api
**Oracle text**:
```
Choose one —
• Return target creature card from your graveyard to your hand.
• Return two target Zombie cards from your graveyard to your hand.
```
**Status**: ISSUE (fixed)

### Code issue
- Oracle text says: a **card** in a graveyard (`• Return target creature card from your graveyard to your hand.`)
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/101/ghoulcallers-chant?utm_source=api
**Type line**: `Sorcery` — {B}
**Oracle text**:
```
Choose one —
• Return target creature card from your graveyard to your hand.
• Return two target Zombie cards from your graveyard to your hand.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**Choose one —**" is modal, so the mode is chosen as the spell is cast
  (CR 601.2b) and the targets follow from it: PASS
- Mode 2 is "**two target** Zombie cards", so it needs two legal Zombie cards to
  be chosen — not "up to two": PASS
- "creature **card**" / "Zombie **cards**" — CR 109.1, and `is_valid_target`
  asks `state.is_card`: PASS
- "from **your** graveyard" on both modes: PASS
- With one of two targets illegal on resolution, the other is still returned
  (CR 608.2b): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both modes and the card filter: `cards_modal.rs`, `cards_graveyard_recursion.rs`
