## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/68/mirror-mad-phantasm?utm_source=api
**Type line**: `Creature — Spirit` — {3}{U}{U}, 5/1
**Oracle text**:
```
Flying
{1}{U}: This creature's owner shuffles it into their library. If that player does, they reveal cards from the top of that library until a card named Mirror-Mad Phantasm is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**This creature's owner** shuffles it into their library" — the owner, not
  the controller, so a stolen Phantasm goes to its owner's library: PASS
- "**If that player does**, they reveal cards ... until a card named Mirror-Mad
  Phantasm is revealed" — the reveal is conditional on the shuffle happening:
  PASS
- The revealed copy goes to the battlefield and *all other revealed cards* to
  the graveyard — usually the rest of the library, which is the card's whole
  point: PASS
- A library with no other copy mills itself entirely: PASS
- The ability resolves from the stack, so removing the Phantasm in response
  leaves nothing to shuffle: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The shuffle-and-dig, and being removed in response: `activated_no_stack.rs:mirror_mad_phantasm_shuffles_and_digs_itself_back_out_on_resolution`, `:mirror_mad_phantasm_source_removed_before_resolution`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/68/mirror-mad-phantasm?utm_source=api
**Type line**: `Creature — Spirit` — {3}{U}{U}, 5/1
**Oracle text**:
```
Flying
{1}{U}: This creature's owner shuffles it into their library. If that player does, they reveal cards from the top of that library until a card named Mirror-Mad Phantasm is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard.
```

**Rulings fetched**:
- [2011-09-22] You can only activate the ability if you control Mirror-Mad Phantasm, even if you don't own it.
- [2011-09-22] If no card named Mirror-Mad Phantasm is revealed (possibly because it was a card copying Mirror-Mad Phantasm or it was a token), all cards from that library will be put into their owner's graveyard.
- [2011-09-22] If another creature (such as Necrotic Ooze) gains Mirror-Mad Phantasm's activated ability, its owner reveals cards until they reveal a card named Mirror-Mad Phantasm.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/68/mirror-mad-phantasm
**Oracle text**:
```
Flying
{1}{U}: This creature's owner shuffles it into their library. If that player does, they reveal cards from the top of that library until a card named Mirror-Mad Phantasm is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard.
```
**Type line**: `Creature — Spirit` · **Mana cost**: `{3}{U}{U}` · **P/T**: 5/1 · **Keywords**: Flying
**Rulings** (3, all 2011-09-22, https://api.scryfall.com/cards/b20eea41-9daf-4ac1-8bad-bb4aa211bb53/rulings):
1. "You can only activate the ability if you control Mirror-Mad Phantasm, even if you don't own it."
2. "If no card named Mirror-Mad Phantasm is revealed (possibly because it was a card copying Mirror-Mad
   Phantasm or it was a token), all cards from that library will be put into their owner's graveyard."
3. "If another creature (such as Necrotic Ooze) gains Mirror-Mad Phantasm's activated ability, its owner
   reveals cards until they reveal a card named Mirror-Mad Phantasm."

**Status**: ISSUE (fixed) — a real bug: the reveal stopped on a token bearing the name.

### Card data
| field | oracle | `mirror_mad_phantasm.rs` | |
|---|---|---|---|
| cost | `{3}{U}{U}` | `Generic(3) + Blue + Blue` | ok |
| types / subtypes | Creature — Spirit | matching | ok |
| P/T | 5/1 | `Some(5)`/`Some(1)` | ok |
| keywords | Flying | `vec![Keyword::Flying]` | ok |
| oracle_text | as above | byte-identical | ok |
| ability cost | `{1}{U}` | `Generic(1) + Blue`, `requires_tap: false` | ok |

### Code issues

**1. The reveal stopped on a token with the right name.** Fixed.

- Oracle text says: `they reveal cards from the top of that library until a card named Mirror-Mad Phantasm is
  revealed`
- Code did: `if name == "Mirror-Mad Phantasm" {`

CR 109.1: a token is not a card. Ruling 2 names this case explicitly — "possibly because it was a card copying
Mirror-Mad Phantasm or it was a **token**" — and it is reachable in this set. Cackling Counterpart makes a token
copy of the Phantasm; the copy has the ability, and activating it shuffles the **token** into the library.
State-based actions do not run mid-resolution (CR 704.3), so the token is still sitting in the library while the
reveal walks past it. The reveal stopped on it and put it back onto the battlefield — where CR 704.5d would then
have left it alone, since it *is* on the battlefield. The library was never milled.

Fixed by `&& state.is_card(card_id)`. Correct behaviour: nothing stops the reveal, the whole library goes to the
graveyard, and the token ends up in the graveyard with it (and ceases to exist at the next SBA pass).

**2. A stale, self-deprecating doc comment.** Fixed.

It quoted a pre-errata wording ("Mirror-Mad Phantasm's owner shuffles it...") and then said
`/// Simplified: Shuffle into library, then mill until we find Mirror-Mad Phantasm (or run out).` The
implementation is not simplified — it is faithful — and labelling it that way sends a reader looking for a
shortcut that is not there. Rewritten to name the two words in the oracle text that carry the card: **owner**
(not controller) and **card** (not permanent).

### Rules check
- **Owner, not controller** (ruling 1): the code reads `o.owner` for whose library is shuffled and revealed, and
  the found card comes back under the owner's control. Correct, and now tested.
- **Ruling 3** (another creature gains the ability): not reachable — nothing in this set grants an activated
  ability to another creature. Recorded, not tested.
- **"If that player does"**: the early return when the Phantasm has left the battlefield. Already tested
  (`activated_no_stack.rs:214`).
- **CR 602.2a**: the ability uses the stack; nothing is shuffled or milled at activation. Already tested
  (`activated_no_stack.rs:186`).
- **The shuffle**: `library_order.push` then `shuffle` — `move_object` does not maintain `library_order`, so the
  push is required and is not a duplicate.
- **`reveal_top_card`** removes from `library_order` but leaves `obj.zone` alone; the subsequent `move_object`
  calls set the zone, so the two representations stay consistent.

### Deliberately not changed
`activated_abilities` opens with `o.zone == Zone::Battlefield`, which is redundant — every caller of
`activated_abilities` already scopes to battlefield permanents. **29 cards** carry this. I removed the
analogous *tapped* gate from ten cards during the Gavony Township audit, but that one earned it: it was written
per-card rather than per-ability, and it was strictly weaker than the engine's check, which also applies
summoning sickness. A bare zone gate has neither defect — it restates a caller's guarantee rather than
re-deciding a game rule. Churning 29 files for no rules content is not the same kind of change, so I left it,
and say so here rather than silently skipping it.

### Changes made
- `mtg-engine/src/cards/isd/mirror_mad_phantasm.rs` — the `is_card` check, and the rewritten doc comment.
- `mtg-engine/tests/token_is_not_a_card.rs` — `a_token_phantasm_is_not_the_card_the_reveal_is_looking_for`.
  That file already collects this family of bugs and explains the SBA window in its header.
- `mtg-engine/tests/cards_complex_creatures.rs` —
  `mirror_mad_phantasm_digs_through_its_owners_library_not_the_activators` for ruling 1. Both players are given
  libraries, so "the owner's" is a choice between two rather than the only one available.

### Mutation checks (all discriminating)
1. `is_card` check reverted → `a_token_phantasm_is_not_the_card_the_reveal_is_looking_for` FAILED.
2. `o.owner` → `o.controller` → `mirror_mad_phantasm_digs_through_its_owners_library_not_the_activators` FAILED.
3. Found card put into the graveyard instead of the battlefield →
   `mirror_mad_phantasm_shuffles_and_digs_itself_back_out_on_resolution` FAILED.
4. Revealed cards pushed back into the library instead of the graveyard →
   `mirror_mad_phantasm_mills_to_find_itself` FAILED.

### Tricky interactions checked
- Shuffles in and always finds itself: **pass** (`cards_complex_creatures.rs:2018`, 20 shuffles).
- Nothing happens until the ability resolves: **pass** (`activated_no_stack.rs:186`).
- Phantasm removed in response → no shuffle, no mill: **pass** (`activated_no_stack.rs:214`).
- A token copy is not what the reveal stops on: **pass** (new, and was failing).
- An opponent activating it digs through the owner's library and hands it back: **pass** (new).
- Another creature gaining the ability (ruling 3): **not reachable** in this set.

### Test coverage
- mills to find itself, across 20 shuffles: `cards_complex_creatures.rs:2018`
- ruling 1, owner vs activator: `cards_complex_creatures.rs:2064` (new)
- ruling 2, token bearing the name: `token_is_not_a_card.rs` (new)
- stack timing and "if that player does": `activated_no_stack.rs:186`, `activated_no_stack.rs:214`

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1414 passing.

