## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/210/woodland-sleuth?utm_source=api
**Type line**: `Creature — Human Scout` — {3}{G}, 2/3
**Oracle text**:
```
Morbid — When this creature enters, if a creature died this turn, return a creature card at random from your graveyard to your hand.
```

**Status**: PASS

### Code issues
No issues found.

- Morbid is an intervening-if on an ETB trigger (CR 603.4): the card overrides
  `should_trigger`, so with nothing dead this turn nothing reaches the stack, and
  the handler re-checks on resolution.
- "return a creature **card** at random" — at random, not chosen.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/210/woodland-sleuth?utm_source=api
**Type line**: `Creature — Human Scout` — {3}{G}, 2/3
**Oracle text**:
```
Morbid — When this creature enters, if a creature died this turn, return a creature card at random from your graveyard to your hand.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The creature card **isn't chosen at random until the ability
  resolves**." The candidate list is built and shuffled inside the trigger
  handler: PASS
- Ruling: "Woodland Sleuth could die in response to its own morbid ability. If
  this happens, the ability **could return Woodland Sleuth** to its owner's
  hand." There is no self-exclusion, and the handler reads the controller from
  whatever zone the Sleuth is in rather than requiring it on the battlefield:
  PASS
- "Morbid — ... **if** a creature died this turn" is an intervening-if
  (CR 603.4): PASS
- CR 109.1: "a creature **card** at random", so a token is not a candidate —
  the filter's `map_or` fallback used to admit exactly that and is now fixed:
  PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- A token is not returned: `token_is_not_a_card.rs:woodland_sleuth_does_not_return_a_token_from_the_graveyard`
- The morbid condition: `intervening_if.rs`, `cards_morbid_and_ltb.rs`
