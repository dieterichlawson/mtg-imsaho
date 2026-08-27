## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/115/sever-the-bloodline?utm_source=api
**Type line**: `Sorcery` — {3}{B}
**Oracle text**:
```
Exile target creature and all other creatures with the same name as that creature.
Flashback {5}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.

**Ruling [2025-01-24]**: "Sever the Bloodline has only one target. Other
creatures with the same name will be exiled even if they have hexproof or
protection."

- This is the trap, and the code avoids it: the "all other creatures with the
  same name" sweep is a plain name match over the battlefield with no
  targetability filter, so a hexproofed same-named creature is still exiled.
  Routing it through the targeting helpers — the obvious shortcut — would have
  been wrong.
- The target itself is included in the sweep, matching "target creature **and**
  all other creatures with the same name".
- Name comes from `o.name`, which `apply_transform` keeps in step with the
  active face, so a transformed DFC is matched by the name it currently has
  (CR 712.8).

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/115/sever-the-bloodline?utm_source=api
**Type line**: `Sorcery` — {3}{B}
**Oracle text**:
```
Exile target creature and all other creatures with the same name as that creature.
Flashback {5}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "Sever the Bloodline has **only one target**. Other creatures with the
  same name will be exiled **even if they have hexproof or protection**." Only
  the first creature is a target; the sweep that follows filters on name alone,
  with no targetability check: PASS
- Ruling: "If the target creature is an illegal target by the time Sever the
  Bloodline tries to resolve, the spell won't resolve. You won't exile **any**
  creatures at all." The whole body is gated on the target, and the engine
  substitutes `Target::Illegal` for one that stopped being targetable — which
  fails the `Target::Object(..)` match, so nothing is exiled: PASS
- The name comparison reads the object's name, which on the battlefield is the
  *active* face's — `apply_transform` refreshes it — so a transformed Werewolf
  matches by its back face's name, which is what "the same name as that
  creature" means: PASS
- Ruling on token names: a token's name is its subtypes plus "Token", so two
  Wolf tokens share a name and are swept together: PASS
- **Exile**, not destroy, so indestructible does not save them and they do not
  reach a graveyard: PASS
- Flashback {5}{B}{B}: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The same-name sweep and the single target: `cards_removal.rs`, `cards_flashback.rs`
