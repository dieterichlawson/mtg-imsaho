## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/73/selhoff-occultist?utm_source=api
**Type line**: `Creature — Human Rogue` — {2}{U}, 2/3
**Oracle text**:
```
Whenever this creature or another creature dies, target player mills a card.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **this creature or another** creature dies" — both kinds declared,
  same as Falkenrath Noble.
- "target player mills a card" — targeted, so the target is locked when the
  trigger goes on the stack (CR 603.3d).

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/73/selhoff-occultist?utm_source=api
**Type line**: `Creature — Human Rogue` — {2}{U}, 2/3
**Oracle text**:
```
Whenever this creature or another creature dies, target player mills a card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Whenever **this creature or another creature** dies" — declared as **two**
  trigger kinds, `SelfDies` *and* `AnyCreatureDies`, so it fires on its own death
  as well as on others'. Murder of Crows, whose text says "whenever **another**
  creature dies", declares only the second — the distinction is in the card data,
  not buried in a handler: PASS
- "**target player** mills a card" — targeted, so it can be pointed at yourself
  or an opponent: PASS
- The mill goes through the pipeline, so a creature card emits
  `CreatureCardMilled` and an opponent's Undead Alchemist sees it: PASS
- CR 113.7a: its own death does not counter the trigger: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both trigger kinds and the mill: `cards_morbid_and_ltb.rs`, `multi_target_and_mill.rs`
