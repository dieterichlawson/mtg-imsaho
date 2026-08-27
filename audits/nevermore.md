## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/25/nevermore?utm_source=api
**Type line**: `Enchantment` — {1}{W}{W}
**Oracle text**:
```
As this enchantment enters, choose a nonland card name.
Spells with the chosen name can't be cast.
```

**Status**: ISSUE

### Code issues
See below.

- Oracle text says: `As this enchantment enters, choose a nonland card name.`
- Code did: declared `TriggerKind::EntersBattlefield` with
  `has_etb_handler() -> true`, so the choice was a triggered ability that went
  on the stack.
- CR 614.12: "**As** [this] enters, choose ..." is a replacement effect applied
  as the permanent enters, not a trigger. Measured before the fix: Nevermore
  resolved onto the battlefield, `awaiting_action` was `false` — **no name
  chosen** — and one trigger sat on the stack. That is a priority window in
  which Nevermore is on the battlefield naming nothing, long enough for an
  opponent to cast the very card it was about to name. For a card whose entire
  function is to pre-empt one card, that window is the card.
- Fixed: new `CardBehavior::chooses_as_it_enters` hook, called from the entering
  path in `move_object` beside the existing copy-guard arming — the one moment
  before any state-based action or priority. Nevermore declares it and no longer
  declares an `EntersBattlefield` trigger. After the fix the same probe shows the
  choice pending at entry with an empty stack.
- The hook is general, not a Nevermore special case: any card whose text begins
  "As this ... enters, choose" belongs on it. Evil Twin stays on the ETB path
  because its consequence is different — it needs its printed 0/0 to survive
  until the copy applies, which the engine already bridges with
  `entering_copy_source` armed at the same point.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`enters_tapped_replacement.rs::a_name_chosen_as_a_permanent_enters_is_chosen_before_anyone_has_priority` — **added by this audit**, asserting both halves: the choice is pending at entry, and nothing reaches the stack.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/25/nevermore?utm_source=api
**Type line**: `Enchantment` — {1}{W}{W}
**Oracle text**:
```
As this enchantment enters, choose a nonland card name.
Spells with the chosen name can't be cast.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**As this enchantment enters**, choose a nonland card name" is CR 614.12 — a
  choice made *as* it enters, not a triggered ability. It is declared through
  `chooses_as_it_enters`, so the engine asks during the entry event rather than
  afterwards: PASS
- Ruling: "**No one can cast spells or activate abilities** between the time a
  card is named and the time that Nevermore's ability starts to work" — a
  consequence of it being an as-enters choice rather than a trigger: PASS
- Ruling: "Spells with the chosen name that somehow happen to **already be on
  the stack** when Nevermore enters are **not affected**" — the restriction is
  on *casting*, checked when a spell would be cast: PASS
- Ruling: "Although the named card can't be cast, it **can still be put onto the
  battlefield** by a spell or ability" — Unburial Rites returning a named
  creature still works: PASS
- "a **nonland** card name": PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The as-enters choice and the cast restriction: `cards_rule_modifiers.rs`, `enters_tapped.rs`
