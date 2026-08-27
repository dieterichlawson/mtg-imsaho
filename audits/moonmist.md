## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/195/moonmist?utm_source=api
**Type line**: `Instant` — {1}{G}
**Oracle text**:
```
Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves. (Only double-faced cards can be transformed.)
```
**Status**: PASS

### Code issues
No issues found.

- "Transform all Humans" — selects creatures with the Human subtype through
  `state.has_subtype` (so granted types count) that actually have a back face,
  since only a double-faced card can transform. Both directions: a back face
  that is still Human transforms too.
- The prevention half is a `PreventCombatDamageExcept` naming Werewolves and
  Wolves, verified in both directions by `moonmist.rs` including a control run
  without the prevention.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/195/moonmist?utm_source=api
**Type line**: `Instant` — {1}{G}
**Oracle text**:
```
Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves. (Only double-faced cards can be transformed.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "Moonmist causes **any double-faced Human** to transform, not just
  Werewolves." The filter is "has the Human subtype on its **active** face and
  has a back face", so it also flips a back face that is still Human — Thraben
  Militia is the case in this set: PASS
- "Transform **all** Humans" — a non-double-faced Human is unaffected (CR 701.28c,
  and the reminder text says as much): PASS
- Ruling: "Whether or not a creature is a Werewolf or a Wolf is checked **only as
  combat damage is dealt**" — the prevention is a live check at damage time, not
  a snapshot of the board when Moonmist resolved: PASS
- Ruling: "Moonmist will prevent combat damage dealt by a creature that isn't a
  Werewolf or a Wolf **even if that creature wasn't on the battlefield** when
  Moonmist resolved": PASS
- The prevention is combat damage only, so a Geistflame still gets through: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Flipping Humans in both directions and the damage prevention: `moonmist.rs`, `werewolf_cards.rs`
