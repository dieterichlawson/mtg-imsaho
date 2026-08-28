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
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/189/kessig-cagebreakers?utm_source=api
**Type line**: `Creature — Human Rogue` — {4}{G}, 3/4
**Oracle text**:
```
Whenever this creature attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard.
```

**Rulings fetched**:
- [2011-09-22] You count the number of creature cards in your graveyard when the triggered ability resolves.
- [2011-09-22] You declare which player or planeswalker each token is attacking as you put it onto the battlefield. It doesn't have to be the same player or planeswalker Kessig Cagebreakers is attacking.
- [2011-09-22] Although the tokens are attacking, they were never declared as attacking creatures (for purposes of abilities that trigger whenever a creature attacks, for example).

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**: `Whenever this creature attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard.`
**Type line**: `Creature — Human Rogue` — {4}{G}, 3/4
**Status**: ISSUE (fixed)

### Rulings (all 2011-09-22)
1. "The number of creature cards in your graveyard is counted when the ability resolves."
2. "You choose which player or planeswalker each token is attacking as it's created. They don't all have to be attacking the same player or planeswalker Kessig Cagebreakers is attacking."
3. "Although the tokens are attacking, they were never declared as attacking creatures."

### Code issues

- `mtg-engine/src/cards/isd/kessig_cagebreakers.rs:49-52` — the trigger ignored the `AttackInfo` it is handed and re-derived the defender from combat state, with an invented fallback.
  - Oracle text says: `create a 2/2 green Wolf creature token that's tapped and attacking`
  - Code did: `let defending_player = state.combat.as_ref().and_then(|c| c.attackers.get(&self_id).copied()).unwrap_or_else(|| state.opponent(controller));`
  - Two failures. The `attackers` lookup misses when the Cagebreakers has already left combat before the trigger resolves — a case `kessig_cagebreakers_counts_itself_among_the_dead` really does exercise. The `state.opponent(controller)` fallback then answers with "the next player", which is only the right player in a two-player game with no planeswalkers. Fixed to `attack.defending_player`, the same fix the Geist of Saint Traft audit made to the other "tapped and attacking" token maker.

- `mtg-engine/tests/cards_complex_creatures.rs:241` — vacuous assertion.
  - Test did: `for wolf in state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.name == "Wolf") { assert!(wolf.tapped, ...) }`
  - CR 111.4 names a token after its subtypes plus "Token", so the tokens are `"Wolf Token"`. The filter matched nothing and the loop body never ran; setting `obj.tapped = false` in the card left the whole workspace green.

### Card data

Verified line by line against the fetched oracle text: `{4}{G}` (`Generic(4)` + `Colored(Green)`), `CardType::Creature`, subtypes `["Human", "Rogue"]`, power 3, toughness 4, no keywords, `TriggerKind::Attacks` matching the implemented `on_attacks` hook, `target_requirement: None` (the card targets nothing). All match.

### Tricky interactions checked

- Ruling 1, count at resolution not at declaration: PASS. `on_attacks` is the trigger's *resolution* hook — the count is read there, not when the trigger goes on the stack. Covered by `trigger_source_independence.rs:397`, which kills the Cagebreakers in response and confirms it counts itself.
- "for each creature **card**" excludes tokens (CR 111.4 / 109.1): PASS, `state.is_card(o.id)` guards it. Not directly testable — a token in a graveyard is removed by SBA before anything can count it — so the test covers the neighbouring cases instead (a non-creature card, and an opponent's creature card).
- "in **your** graveyard": PASS, `objects_in_zone(Zone::Graveyard, controller)`. Now tested — an opponent's creature card in their graveyard is not counted.
- Ruling 3, tokens never declared as attackers: PASS by construction. The tokens are inserted straight into `combat.attackers` and no `AttackersDeclared` event is emitted for them, so nothing that watches attack declarations (Curse of Stalked Prey, Bloodcrazed Neonate) sees them. This is the same mechanism `common::joins_the_attack` exists for.
- Tokens are not summoning-sick-blocked: PASS. They are put onto the battlefield attacking rather than declared, so summoning sickness never applies (CR 302.6 governs declaring attackers); `summoning_sick = false` is set for the same reason Geist's Angel does.
- Zero creature cards in graveyard: PASS, early return, no tokens and no log line.

### Documented, not implemented

Ruling 2 — "You choose which player or planeswalker each token is attacking as it's created. They don't all have to be attacking the same player" — is not implemented: every token attacks the Cagebreakers' own defender. This is the identical gap recorded in the Geist of Saint Traft audit and needs the same missing piece (a player choice presented while a trigger resolves, for each token created). Deferred there and deferred here, so the two land together. With the fix above, the tokens at least attack the *right* default player rather than an invented one.

### Test coverage

- Main effect, N tokens for N creature cards: `cards_complex_creatures.rs:219` `kessig_cagebreakers_creates_wolf_tokens_on_attack`
- Token characteristics (2/2, green, creature, Wolf, tapped, attacking): same test, added this audit
- "creature **card**" — non-creature card in graveyard not counted: same test, added this audit
- "**your** graveyard" — opponent's creature card not counted: same test, added this audit
- Ruling 1, count at resolution with the source dead: `trigger_source_independence.rs:397` `kessig_cagebreakers_counts_itself_among_the_dead`
- Defender comes from the trigger, three-player: `cards_complex_creatures.rs:266` `kessig_wolves_attack_the_cagebreakers_defender_and_not_just_the_next_player`, added this audit
- Ruling 2, per-token defender choice: NOT TESTED (not implemented — see above)

### Mutation checking

Four mutations, each run against the whole `cards_complex_creatures` binary. Before this audit's test changes, M1–M3 all passed the entire workspace — three untested gaps.

| Mutation | Before | After |
| --- | --- | --- |
| M1 `obj.tapped = true` -> `false` | passed (vacuous filter) | `kessig_cagebreakers_creates_wolf_tokens_on_attack` FAILED |
| M2 `Color::Green` -> `Color::Black` | passed | `kessig_cagebreakers_creates_wolf_tokens_on_attack` FAILED |
| M3 drop `state.is_creature` from the count | passed | `kessig_cagebreakers_creates_wolf_tokens_on_attack` FAILED |
| M4 `attack.defending_player` -> `state.opponent(controller)` | n/a (was the bug) | `kessig_wolves_attack_the_cagebreakers_defender_and_not_just_the_next_player` FAILED |

Source restored from `/tmp/kc2.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1468 passing (was 1467). `cargo check --workspace --all-targets` clean, zero warnings.
