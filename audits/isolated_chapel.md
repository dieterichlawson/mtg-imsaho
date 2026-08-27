## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/242/isolated-chapel?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control a Plains or a Swamp.
{T}: Add {W} or {B}.
```

**Status**: PASS

### Code issues
No issues found.

- "enters tapped **unless** you control a [land type]" is a replacement effect
  (CR 614.1d), implemented through `replace_event` /
  `helpers::enters_tapped_unless` rather than as an ETB trigger.
- That distinction is not cosmetic, and `enters_tapped_replacement.rs` documents
  the three ways the trigger version was wrong: the land entered untapped and
  could be tapped for mana in response to its own trigger; the condition was read
  at resolution, so an opponent could destroy the enabling land in response; and
  a trigger opened a priority window even when nothing needed to happen.
- The condition reads the battlefield for a land of the right type, and the five
  lands do not satisfy each other.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`enters_tapped_replacement.rs` — all five lands, both directions, plus the already-tapped-before-priority and no-trigger-on-the-stack checks.
