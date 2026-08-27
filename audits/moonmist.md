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
