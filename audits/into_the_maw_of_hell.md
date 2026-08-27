## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/150/into-the-maw-of-hell?utm_source=api
**Type line**: `Sorcery` — {4}{R}{R}
**Oracle text**:
```
Destroy target land. Into the Maw of Hell deals 13 damage to target creature.
```

**Status**: ISSUE

### Code issues
See below.

**Ruling [2011-09-22]**: "If one of Into the Maw of Hell's targets is illegal by
the time it resolves, Into the Maw of Hell will still affect the remaining legal
target. If both targets are illegal at this time, Into the Maw of Hell won't
resolve."

- The card's own halves are right: each guards on its target still being on the
  battlefield, so a bounced land or a dead creature is skipped and the other
  half happens.
- The gap was one layer down, in the engine. `stack::resolve_spell` computed
  only whether **any** target was still legal and then handed the card the whole
  original list. A target that became illegal *without leaving the battlefield*
  — the ordinary case, a creature given hexproof in response — was still
  affected. Measured: with the creature hexproofed on the stack, it took the
  full 13.
- Fixed engine-side: an object target that can no longer be targeted by this
  caster is replaced with a new `Target::Illegal` before `on_resolve`. Substituted
  rather than removed, so positions hold — "the land is `targets[0]`" stays true
  — and the illegal one simply fails to match `Target::Object(..)`, which every
  card's existing pattern already handles.
- Deliberately scoped to *targeting restrictions* (hexproof CR 702.11b,
  protection CR 702.16b), which are properties of the target alone. Whether a
  target still satisfies its **requirement** cannot be asked per target at this
  point: `is_target_legal` unwraps only the first branch of a composite
  requirement, so Memory's Journey's graveyard-card targets would be judged
  against its `PlayerOnly` first slot. The broader version of this change did
  exactly that and broke four Memory's Journey tests, which is how the limit was
  found.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/150/into-the-maw-of-hell?utm_source=api
**Type line**: `Sorcery` — {4}{R}{R}
**Oracle text**:
```
Destroy target land. Into the Maw of Hell deals 13 damage to target creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "Into the Maw of Hell targets **both** the land and the creature. You
  can only cast it if you can choose a legal target for both": PASS
- Ruling: "If **one** of Into the Maw of Hell's targets is illegal by the time it
  resolves, Into the Maw of Hell will **still affect the remaining legal
  target**. If **both** targets are illegal at this time, Into the Maw of Hell
  won't resolve." This is the card the engine's CR 608.2b handling was written
  against: an illegal target is *substituted* with `Target::Illegal` rather than
  removed, so the positions hold — the land stays `targets[0]` — and the illegal
  one simply fails to match `Target::Object(..)`. Checking only "is any target
  legal", as the engine used to, meant a creature that gained hexproof in
  response still took all 13 damage: PASS
- 13 damage through `deal_damage`, so protection and prevention apply: PASS
- `try_destroy` on the land, so an indestructible land survives while the
  creature still takes its damage: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- One target illegal, the other still affected: `fizzle.rs:a_multi_target_spell_is_countered_only_when_every_target_is_illegal`, `:a_target_that_gained_hexproof_in_response_is_skipped_and_the_rest_resolve`
