## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/62/lantern-spirit?utm_source=api
**Type line**: `Creature — Spirit` — {2}{U}, 2/1
**Oracle text**:
```
Flying
{U}: Return this creature to its owner's hand.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{U}: Return **this creature** to **its owner's** hand" — the owner, so a
  stolen Lantern Spirit returns to its owner: PASS
- The return happens on resolution, so the Spirit can be removed in response and
  the ability then does nothing: PASS
- Returning it to hand while it is attacking removes it from combat: PASS
- Flying: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The self-bounce: `activated_abilities.rs:lantern_spirit_returns_itself_to_hand`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/62/lantern-spirit?utm_source=api
**Type line**: `Creature — Spirit` — {2}{U}, 2/1
**Oracle text**:
```
Flying
{U}: Return this creature to its owner's hand.
```

**Rulings fetched**:
- [2011-09-22] Only Lantern Spirit’s controller may activate its ability.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**:
```
Flying
{U}: Return this creature to its owner's hand.
```
**Type line**: `Creature — Spirit` — {2}{U}, 2/1, Flying
**Status**: ISSUE (fixed) — a real gameplay bug

### Ruling (2011-09-22)
"Only Lantern Spirit's controller may activate its ability." — CR 602.1a, and the engine's, not this card's: `legal_actions` enumerates only permanents the player with priority controls.

### Code issues

- `mtg-engine/src/cards/isd/lantern_spirit.rs:49` — the ability returned the card from wherever it happened to be.
  - Oracle text says: `{U}: Return **this creature** to its owner's hand.`
  - Code did: `state.move_object(object_id, Zone::Hand, registry);`
  - Kill the Spirit in response to its own ability and the ability resolved anyway, lifting the card **out of the graveyard** and into its owner's hand. For {U}, that is a free rescue from any removal spell. CR 400.7: a permanent that has left the battlefield is a new object, and "this creature" has nothing left to return.
  - Confirmed by running it before writing anything: activating, moving the Spirit to the graveyard in response, and resolving left it in `Zone::Hand`.
  - Fixed with `helpers::still_on_battlefield`, the documented way to ask — the same guard Tree of Redemption uses, and the mirror of the one Moldgraf Monstrosity already has for its own zone change. The ability still resolves: it has no targets, so CR 608.2b cannot counter it; it does as much as it can, which is nothing (CR 608.2).

Everything else is right: `{2}{U}`, Creature — Spirit, 2/1, `Keyword::Flying`, oracle text verbatim, ability `{U}` with no tap and no restriction.

### Tricky interactions checked

- Killed in response to its own ability: FAIL before this audit, PASS now.
- "to its **owner's** hand" with the Spirit stolen: PASS — and automatically, see below.
- Repeatable at instant speed: PASS, no restriction flags, pinned to the text by the cost invariant.
- The card is a fresh object in hand, so nothing about the old permanent follows it: PASS, `move_object` resets the object-level grants on a zone change (CR 400.7).
- Printed flying: PASS, and pinned by the keyword invariant.

### Test coverage

- Returns itself to hand: `activated_abilities.rs:66` `lantern_spirit_returns_itself_to_hand`
- Returns nothing once it is already dead: `activated_abilities.rs:80` `lantern_spirit_returns_nothing_if_it_is_already_dead`, added this audit
- Goes to its owner's hand rather than the thief's: `activated_abilities.rs:104` `lantern_spirit_returns_to_its_owners_hand_not_the_thiefs`, added this audit
- Cost and restriction flags match the text: `card_data_invariants.rs:1706`
- Flying is printed: `card_data_invariants.rs:1790`

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 remove the CR 400.7 guard | n/a (was the bug) | `lantern_spirit_returns_nothing_if_it_is_already_dead` FAILED |
| M2 return to the *controller's* hand | passed | passed — **unfalsifiable**, see below |
| M3 drop printed `Keyword::Flying` | n/a | `keywords_say_what_scryfall_says` FAILED |

M2 is recorded as unfalsifiable rather than as covered ground. Routing the return through `move_object_under_control` with the controller still lands the card in its owner's hand, because hands are keyed by owner (`objects_in_zone`, CR 108.4) and `move_object` resets the controller on the way out of the battlefield. No change to this card can put it in the wrong hand, so the second test pins the engine's zone model rather than the card's code — worth having, but not evidence about Lantern Spirit.

Source restored from `/tmp/ls2.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1497 passing (was 1495). `cargo check --workspace --all-targets` clean, zero warnings.
