## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/140/devils-play?utm_source=api
**Type line**: `Sorcery` — {X}{R}
**Oracle text**:
```
Devil's Play deals X damage to any target.
Flashback {X}{R}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.

- "{X}{R}: deals X damage to **any target**" — "any target" covers creature,
  player and planeswalker, and the damage goes through the pipeline so CR 120.3c
  loyalty removal applies to a planeswalker.
- X is the value announced on casting (CR 601.2b), covered by
  `x_cost_spells.rs`, which checks the announced X is the X the effect uses.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/140/devils-play?utm_source=api
**Type line**: `Sorcery` — {X}{R}
**Oracle text**:
```
Devil's Play deals X damage to any target.
Flashback {X}{R}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{X}{R}" and "Flashback {X}{R}{R}{R}" — X is announced on both, so the engine
  runs its X-funding prompt for the flashback cast as well: PASS
- "deals X damage to **any target**" — creature, player or planeswalker, and the
  planeswalker branch removes loyalty rather than marking damage (CR 120.3c) —
  this card was the one that wrote `damage_marked` on a planeswalker: PASS
- Damage through the pipeline: PASS
- Ruling: a spell cast with flashback is exiled afterwards whatever happens to
  it: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- X damage and the planeswalker path: `inline_damage.rs`, `damage_helper.rs`, `cards_flashback.rs`
