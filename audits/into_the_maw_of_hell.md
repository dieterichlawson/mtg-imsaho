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
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/150/into-the-maw-of-hell?utm_source=api
**Type line**: `Sorcery` — {4}{R}{R}
**Oracle text**:
```
Destroy target land. Into the Maw of Hell deals 13 damage to target creature.
```

**Rulings fetched**:
- [2011-09-22] Into the Maw of Hell targets both the land and the creature. You can only cast it if you can choose a legal target for both.
- [2011-09-22] If one of Into the Maw of Hell’s targets is illegal by the time it resolves, Into the Maw of Hell will still affect the remaining legal target. If both targets are illegal at this time, Into the Maw of Hell won’t resolve.

**Status**: ISSUE


Two rulings:
1. "Into the Maw of Hell targets both the land and the creature. You can only
   cast it if you can choose a legal target for both."
2. "If one of Into the Maw of Hell's targets is illegal by the time it
   resolves, Into the Maw of Hell will still affect the remaining legal target.
   If both targets are illegal at this time, Into the Maw of Hell won't
   resolve."

### Code issues
The card file is correct. The bug is in the engine, and ruling 2 is what it
breaks.

**A two-target spell judged every target against its *first* slot's
requirement.**

`stack.rs::is_target_legal` is asked one target at a time and has no way to
know which slot a target came from, so it unwrapped `TwoTargets(a, b)` to `a`
and tested everything against that:

```rust
let inner_req = match target_req {
    TargetRequirement::UpToTargets(_, inner) | TargetRequirement::TwoTargets(inner, _) => inner.as_ref(),
    other => other,
};
```

For this card the first slot is `PermanentWithFilter(HasCardType(Land))`, so
the *creature* target was tested against "is it a land", failed, and could
never count as legal. `any_legal` therefore rested entirely on the land. When
the land became an illegal target — hexproof in response, and Ranger's Guile is
in this set — `any_legal` was false and the whole spell was countered by game
rules, creature and all. Ruling 2 says the opposite in as many words.

I found this by writing the test for the half of ruling 2 that had none. The
existing fizzle test covers only the creature becoming illegal, which is the
arm the old code got right by luck: the land satisfies the land filter, so
`any_legal` held.

Fixed by giving `TwoTargets` the same treatment `ModalChoice` already had two
lines above — legal under *either* slot. Which slot a target belongs to was
settled when the spell was cast (CR 601.2c); what CR 608.2b re-checks is
whether the target is still there and still targetable, and the hexproof and
protection checks inside each branch still apply.

**This was known and worked around rather than fixed.** A comment further down
the same function reads: "Whether a target still satisfies its *requirement*
cannot be asked per target here: `is_target_legal` unwraps only the first
branch of a composite requirement, so Memory's Journey's graveyard cards would
be judged against its `PlayerOnly` first slot." The author scoped the *target
substitution* around the problem but left `any_legal` computing through it.

### Blast radius
Five cards use `TwoTargets`. Three had heterogeneous slots and were affected:
- **Into the Maw of Hell** — land + creature. Fixed and tested here.
- **Memory's Journey** — `PlayerOnly` + graveyard cards. The case the comment
  names.
- **Lost in the Mist** — `Spell` + land.

Two are unaffected in outcome: Ghoulcaller's Chant targets two graveyard
Zombies with the same requirement, and Prey Upon's two slots differ only by
`YouControl` / `YouDontControl` — its opponent-creature target also never
counted, but a fight needs both targets anyway, so the result was right by
accident.

The fix applies to all of them. Their own tests belong to their own audits;
I have not written them here.

### Tricky interactions checked
- Both halves resolve on a plain cast: pass
- The land is destroyed through the destruction pipeline, so an indestructible
  land survives and the creature still burns: pass
- The 13 damage is non-combat and records its source in `damaged_by`: pass
- Ruling 1 — not castable without a legal target for both (CR 601.2c): pass
- Ruling 2, creature illegal → land still destroyed: pass (pre-existing)
- Ruling 2, land illegal → creature still burns: **failed before the fix**
- Ruling 2, both illegal → nothing happens, spell to the graveyard: pass
- The first slot offers only lands: pass (`hexproof_filter.rs:390`)
- The slots are offered in oracle order, land then creature: pass
  (`characteristics_targeting.rs:143`)

### Test coverage
- Slot ordering and pairing: `characteristics_targeting.rs:143`
- First slot is lands only: `hexproof_filter.rs:390`
- Creature illegal, land still destroyed: `fizzle.rs:270`
- **NEW** both halves on a plain cast, damage kind and `damaged_by`:
  `cards_lands_and_mana_sources.rs:361`
- **NEW** indestructible land survives, creature still burns:
  `cards_lands_and_mana_sources.rs:390`
- **NEW** ruling 1, castable only with both: `cards_lands_and_mana_sources.rs:411`
- **NEW** ruling 2's other two cases: `fizzle.rs:296`

### An empty section claiming coverage
`cards_lands_and_mana_sources.rs` had a banner comment for this card with
nothing under it, while the file's own list of covered cards named it. All the
card's real coverage was about targeting; its plain effect — destroy the land,
deal the 13 — was asserted nowhere in the suite. The section now has it.

### On `is_valid_target` accepting either a land or a creature
Deliberate, not sloppy: the function has no slot to key on, the per-slot
requirement does the work at announcement, and
`characteristics_targeting.rs:134` already documents this. Left alone.

