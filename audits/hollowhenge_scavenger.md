## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/188/hollowhenge-scavenger?utm_source=api
**Type line**: `Creature — Elemental` — {3}{G}{G}, 4/5
**Oracle text**:
```
Morbid — When this creature enters, if a creature died this turn, you gain 5 life.
```

**Status**: PASS

### Code issues
No issues found.

- Same morbid intervening-if shape as Woodland Sleuth, gated at dispatch and
  re-checked at resolution.
- Emits `LifeChanged` for the 5 life.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/188/hollowhenge-scavenger?utm_source=api
**Type line**: `Creature — Elemental` — {3}{G}{G}, 4/5
**Oracle text**:
```
Morbid — When this creature enters, if a creature died this turn, you gain 5 life.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Morbid — ... **if** a creature died this turn, you gain 5 life" is an
  intervening-if (CR 603.4), checked both when the trigger would go on the stack
  and again on resolution: PASS
- The Scavenger's own arrival cannot satisfy its condition — entering is not
  dying: PASS
- The life gain goes through `change_life`, so LifeChanged reaches every
  watcher: PASS
- CR 113.7a: killing the Scavenger in response to its own trigger does not stop
  the life gain: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The morbid condition and the life gain: `cards_morbid_and_ltb.rs`, `intervening_if.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/188/hollowhenge-scavenger?utm_source=api
**Type line**: `Creature — Elemental` — {3}{G}{G}, 4/5
**Oracle text**:
```
Morbid — When this creature enters, if a creature died this turn, you gain 5 life.
```

**Rulings fetched**: none published for this card.

**Status**: PASS


No rulings are cached for this card and none surfaced.

### Code issues
No issues found.

- Card data matches exactly: {3}{G}{G}, Creature — Elemental, 4/5, oracle text
  verbatim. Morbid is an ability word (CR 207.2c), so its absence from the
  `keywords` vector is right.
- Morbid is implemented as an intervening-if (CR 603.4) in both places the rule
  asks for: `should_trigger` via `helpers::morbid_should_trigger`, and again at
  the top of `on_enter_battlefield`.
- `controller_of` for "you", which is CR 608.2g's last-known controller.
- `state.change_life` emits `LifeChanged`, which is what the procedure asks for
  and what life-gain watchers read.

### What was untested
The card's entire effect. Its only coverage was `intervening_if.rs:206`, a
registry-wide sweep that asserts the trigger reaches the stack when a creature
died and stays off it otherwise. That is the trigger firing, not the life being
gained: nothing anywhere checked that the number is 5, that the *controller*
gets it, or that the opponent does not.

### Tricky interactions checked
- A creature died: the controller gains exactly 5: pass
- No creature died: nothing happens, and the Scavenger still enters: pass
- The opponent's life is untouched either way: pass
- "You" is the last known controller, not the owner (CR 608.2g / 400.7): pass
- Killing the Scavenger in response does not counter its trigger (CR 113.7a):
  pass
- The trigger does not reach the stack when no creature died (CR 603.4):
  pass (`intervening_if.rs:206`)

### Test coverage
- Intervening-if at trigger time, registry-wide sweep: `intervening_if.rs:206`
- **NEW** gains exactly 5, only when a creature died, and only for you:
  `cards_morbid_and_ltb.rs:60`
- **NEW** the life goes to the last known controller, not the owner, and the
  trigger survives the Scavenger's death: `cards_morbid_and_ltb.rs:88`

### On the resolution-time morbid check
It cannot be exercised. `creature_died_this_turn` only ever goes false at a
turn boundary, and the trigger resolves in the turn it fired, so the condition
is always still true by the time `on_enter_battlefield` runs.

I found this by mutation rather than by reading: making the resolution check
unconditional changed nothing, because `should_trigger` had already kept the
trigger off the stack. Removing *either* layer alone leaves the card behaving
correctly; only removing both makes the new test fail. So the second check is
defensive, not load-bearing — CR 603.4 does require the condition to be checked
again on resolution, so it is correct to have, but no test in this engine can
distinguish its presence from its absence, and I have not written one that
pretends to.

The first test's discriminating mutations are therefore the amount (5 → 3) and
both morbid layers removed together; the second test's is reading `controller`
off the object instead of last-known information.

