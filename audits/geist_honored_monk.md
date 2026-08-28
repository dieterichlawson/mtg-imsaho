## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/khc/25/geist-honored-monk?utm_source=api
**Type line**: `Creature — Human Monk` — {3}{W}{W}, */*
**Oracle text**:
```
Vigilance
Geist-Honored Monk's power and toughness are each equal to the number of creatures you control.
When this creature enters, create two 1/1 white Spirit creature tokens with flying.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "power and toughness are each equal to the number of creatures you control" is
  a characteristic-defining ability — `dynamic_pt`, recomputed every time rather
  than snapshotted, and it counts itself: PASS
- The two Spirit tokens it makes are creatures you control, so they raise its own
  P/T: PASS
- The tokens carry colour, subtype and flying via
  `create_token_with_subtypes`: PASS
- Vigilance: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The CDA and the tokens: `cards_complex_creatures.rs`, `token_is_not_a_card.rs:cda_does_not_count_tokens_in_graveyard`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/khc/25/geist-honored-monk?utm_source=api
**Type line**: `Creature — Human Monk` — {3}{W}{W}, */*
**Oracle text**:
```
Vigilance
Geist-Honored Monk's power and toughness are each equal to the number of creatures you control.
When this creature enters, create two 1/1 white Spirit creature tokens with flying.
```

**Rulings fetched**:
- [2011-09-22] The ability that defines Geist-Honored Monk's power and toughness works in all zones, not just the battlefield.
- [2011-09-22] As long as Geist-Honored Monk is on the battlefield, its second ability will count itself.

**Status**: PASS


Two rulings:
1. "The ability that defines Geist-Honored Monk's power and toughness works in
   all zones, not just the battlefield."
2. "As long as Geist-Honored Monk is on the battlefield, its second ability
   will count itself."

(The cached printing is `khc`; oracle text, type line and P/T are the same as
the ISD printing.)

### Code issues
No issues found.

- Card data matches: {3}{W}{W}, Creature — Human Monk, Vigilance, ETB trigger
  declared, oracle text exact. `power: Some(0)` is the codebase's sentinel for
  a `*/*` creature — `effective_power` uses it to decide the card consults its
  own `dynamic_pt` at all.
- `dynamic_pt` counts `objects_in_zone(Battlefield, controller)` filtered by
  `is_creature`, so it counts itself while on the battlefield (ruling 2) and
  ignores the opponent's board.
- The tokens go through `create_token_with_subtypes` with the Spirit subtype
  and Flying, so they are the 1/1 white flying Spirits the text asks for, and
  CR 111.4 names them "Spirit Token" from the subtype.
- `controller_of` for the ETB's "you"; nothing re-derived from the source.

Ruling 1 holds, and it is worth saying why rather than just that it does.
`effective_power` consults `dynamic_pt` whenever the object has a base P/T,
with no zone test, so the ability runs in a graveyard or a hand as well as on
the battlefield (CR 604.3). And `dynamic_pt` reads the object's `controller`,
which CR 400.7 resets to the owner when a permanent leaves the battlefield —
so off the battlefield it counts the creatures the *owner* controls, which is
what CR 109.5 means by "you" for a card with no controller. The two rules line
up by accident of the same field rather than by design, which is exactly why
it now has a test.

### Tricky interactions checked
- Counts itself on the battlefield (ruling 2): pass
- Works in a graveyard (ruling 1 / CR 604.3): pass
- Off the battlefield, "you" resolves to the owner (CR 109.5 / 400.7): pass
- An opponent's creatures do not count: pass
- Recomputed as the battlefield changes, not snapshotted (CR 604.3): pass
- Tokens are 1/1, white, Spirit, and have flying: pass
- Exactly two tokens: pass
- The Monk enters at power 1 (itself) and becomes 3/3 only once the trigger
  resolves: pass, implied by the ETB test's final 3/3

### Test coverage
- Main effect, 3/3 with two Spirits: `cards_evasion_and_graveyard_pt.rs:61`
- Dynamic-P/T display in the harness: `harness_display.rs:32`
- Spirit tokens are targetable as Spirits: `subtype.rs:445`
- `dynamic_pt` is used rather than a hand-rolled P/T: `test_suite_guards.rs:979`
- **NEW** the tokens are 1/1 white flying Spirits:
  `cards_evasion_and_graveyard_pt.rs:80`
- **NEW** "creatures you control" excludes the opponent's, and the count keeps
  up as the board changes: `cards_evasion_and_graveyard_pt.rs:108`
- **NEW** the defining ability works outside the battlefield (ruling 1):
  `cards_evasion_and_graveyard_pt.rs:139`

### What was untested before
The existing test counted three creatures and read the Monk at 3/3. Two
colourless vanilla 1/1s would have satisfied it, so nothing pinned the tokens'
colour, subtype, or flying — the subtype in particular matters, since Urgent
Exorcism can destroy target Spirit. Neither ruling had a test of its own:
ruling 2 was implied by the 3/3 but ruling 1 was not exercised at all, and it
depends on two engine facts (no zone gate on `dynamic_pt`, and `controller`
falling back to owner) that no test held in place.

