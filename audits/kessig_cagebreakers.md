## Audit — 2026-08-27 — CR 109.1: a token in a graveyard is not a card

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/189/kessig-cagebreakers?utm_source=api
**Oracle text**:
```
Whenever this creature attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard.
```
**Status**: ISSUE (fixed)

### Code issue
- Oracle text says: a **card** in a graveyard (`Whenever this creature attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard.`)
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/189/kessig-cagebreakers?utm_source=api
**Type line**: `Creature — Human Rogue` — {4}{G}, 3/4
**Oracle text**:
```
Whenever this creature attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "You count the number of creature cards in your graveyard **when the
  triggered ability resolves**": PASS
- Ruling: "Although the tokens are attacking, they were **never declared as
  attacking creatures** (for purposes of abilities that trigger whenever a
  creature attacks)." The tokens are inserted straight into `combat.attackers`
  rather than going through `declare_attackers`, so no Attacks trigger fires for
  them — including the Cagebreakers' own: PASS
- Ruling: "You declare which player or planeswalker each token is attacking as
  you put it onto the battlefield. It doesn't have to be the same player" — the
  defending player is read from combat state: PASS
- CR 109.1: "for each creature **card** in your graveyard", so tokens there are
  not counted: PASS
- The tokens enter tapped and are not summoning sick, so they deal combat damage
  this turn: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The token count and the attacking tokens: `cards_complex_creatures.rs`, `combat_rules.rs`
