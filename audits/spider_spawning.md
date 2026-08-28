## Audit — 2026-08-27 — CR 109.1: a token in a graveyard is not a card

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/203/spider-spawning?utm_source=api
**Oracle text**:
```
Create a 1/2 green Spider creature token with reach for each creature card in your graveyard.
Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: ISSUE (fixed)

### Code issue
- Oracle text says: a **card** in a graveyard (`Create a 1/2 green Spider creature token with reach for each creature card in your graveyard.`)
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
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/203/spider-spawning?utm_source=api
**Type line**: `Sorcery` — {4}{G}
**Oracle text**:
```
Create a 1/2 green Spider creature token with reach for each creature card in your graveyard.
Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

Covered by the CR 109.1 entry above; token subtypes are set.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/203/spider-spawning?utm_source=api
**Type line**: `Sorcery` — {4}{G}
**Oracle text**:
```
Create a 1/2 green Spider creature token with reach for each creature card in your graveyard.
Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.

Ruling: "The number of creature cards in your graveyard is counted when
Spider Spawning resolves." Counted in `on_resolve`, not at cast. The filter is
`o.owner == controller && state.is_card(o.id) && state.is_creature(o.id, registry)`
— CR 109.1, a token in the graveyard is not a card. Tokens are 1/2 green
Spiders with reach, subtype supplied via `create_token_with_subtypes`.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs` — count and subtypes; the token exclusion is covered by the registry-wide graveyard sweep.

## Audit — 2026-08-28 19:14

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Spider Spawning"`, https://scryfall.com/card/isd/203/spider-spawning
**Oracle text**:
```
Create a 1/2 green Spider creature token with reach for each creature card in your graveyard.
Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: Sorcery
**Mana cost**: {4}{G}   **Keywords**: Flashback
**Rulings**: 7 — six generic flashback, and one specific: "The number of creature cards in your
graveyard is counted when Spider Spawning resolves."
**Status**: PASS (the count's two qualifying words gained their tests)

### Code issues
No issues found in `mtg-engine/src/cards/isd/spider_spawning.rs`.

`{4}{G}`, `CardType::Sorcery`, `flashback_cost: Some({6}{B})` — a different colour from the
card's own cost, the Forbidden Alchemy shape — oracle text verbatim, no target requirement.

The count is taken in `on_resolve` (the ruling), over `objects_in_zone(Graveyard, controller)`
(your graveyard, keyed by owner per CR 404.3), filtered by `is_card` (CR 109.1, no tokens) and
`is_creature` (creature cards only, read through the characteristics layer). The `o.id !=
object_id` guard is belt-and-braces — the spell is on the stack while resolving — but says the
right thing for free. Tokens carry the full definition: 1/2, green, Spider, reach, and CR 111.4
derives "Spider Token".

### Tricky interactions checked
- **Counted at resolution** (the ruling): PASS — the count lives in `on_resolve`.
- **"Creature card"**: a land in your graveyard does not count. PASS, newly pinned.
- **"Your graveyard"**: an opponent's creature card does not count. PASS, newly pinned.
- **Cast via its own flashback**: it is on the stack while resolving, so it does not count
  itself; and Splinterfright-style self-mill into a big Spawning is the deck this card is for.
- **Parallel Lives**: each token is its own creation event, so a doubler doubles each — the
  shared helper's job.
- **Zero creature cards**: zero tokens, and the spell finishes. Trivial path, unasserted;
  nothing observable beyond the absence.
- **The Spiders are Spiders**: targetable by Urgent Exorcism (`subtype.rs`), and blockers with
  reach.

### Test coverage
- one Spider per creature card, 1/2 with reach, a land and an opponent's creature card both
  excluded: `cards_evasion_and_graveyard_pt.rs:265 spider_spawning_creates_tokens` (extended —
  it was also one of the three vacuous token-name loops repaired at the Army of the Damned
  audit)
- flashback: the generic offering/exile tests

Mutation-checked: counting any card fails it (the land would be a fifth Spider); counting every
graveyard fails it (the opponent's card would be a sixth); 1/1 instead of 1/2 fails it.

### Changes made
- `cards_evasion_and_graveyard_pt.rs`: the land and opponent's-card exclusions. No code change.
