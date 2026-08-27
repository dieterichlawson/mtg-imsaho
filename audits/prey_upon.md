## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/200/prey-upon?utm_source=api
**Type line**: `Sorcery` — {G}
**Oracle text**:
```
Target creature you control fights target creature you don't control. (Each deals damage equal to its power to the other.)
```

**Status**: PASS

### Code issues
No issues found.

**Ruling [2018-12-07]**: "If either target is an illegal target as Prey Upon
resolves, no creature will deal or be dealt damage."

- The opposite of Into the Maw of Hell's partial resolution, and the code gets
  it: `if a_on_bf && b_on_bf` — both or neither.
- With the new `Target::Illegal` substitution this also holds for a target that
  gained hexproof: the `(Target::Object(a), Target::Object(b))` pattern stops
  matching, so nothing happens. The all-or-nothing ruling falls out of the
  representation rather than needing its own check.
- "Target creature **you control** fights target creature **you don't control**"
  — two filtered slots, and the handler sorts them by controller rather than
  trusting the order they arrive in.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/200/prey-upon?utm_source=api
**Type line**: `Sorcery` — {G}
**Oracle text**:
```
Target creature you control fights target creature you don't control. (Each deals damage equal to its power to the other.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "If **either** target is an illegal target as Prey Upon resolves, **no
  creature will deal or be dealt damage**." This is the opposite of Into the Maw
  of Hell's partial resolution — a fight needs both creatures, so losing one
  cancels the whole thing rather than half of it. The card checks both before
  fighting: PASS
- "target creature **you control** fights target creature **you don't control**"
  — two different filters on the two slots: PASS
- The fight is simultaneous damage from each creature's power, through the damage
  pipeline: PASS
- A 0-power creature fights and deals nothing rather than being an illegal
  choice: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both targets required: `fizzle.rs:prey_upon_resolves_without_fighting_when_one_of_its_two_targets_is_gone`
