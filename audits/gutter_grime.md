## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/186/gutter-grime?utm_source=api
**Type line**: `Enchantment` — {4}{G}
**Oracle text**:
```
Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever a **nontoken** creature **you control** dies" — both filters present.
  The token check reads the *captured* `dead_is_token` rather than the object,
  and the comment says why: SBA 704.5d has already removed the dead token from
  `state.objects` by the time the trigger resolves, so the object is not there
  to ask.
- The Ooze token's P/T is linked to the slime-counter count on this Gutter Grime
  rather than fixed at creation, so every Ooze grows as more creatures die.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/186/gutter-grime?utm_source=api
**Type line**: `Enchantment` — {4}{G}
**Oracle text**:
```
Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "If you control **more than one** Gutter Grime, each Ooze token
  **remembers which one created it**. The power and toughness of that Ooze will
  be equal to the number of slime counters on **that** Gutter Grime only." Each
  token stores its creator's id in `card_state`, and `effective_power` reads the
  counters of *that* object: PASS
- Ruling: "The power and toughness of the Ooze tokens will **constantly
  update**": it is read live, not snapshotted: PASS
- Ruling: "If Gutter Grime leaves the battlefield, the power and toughness of
  each Ooze token it created will become 0 ... put into its owner's graveyard
  the next time state-based actions are checked." A permanent's counters are
  cleared on a zone change, so the lookup yields 0 and the tokens die to SBA:
  PASS
- "Whenever a **nontoken** creature you control dies" — so the Oozes it makes do
  not feed it: PASS
- The counter goes on *then* the token is created, so the first Ooze is a 1/1
  rather than a 0/0: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The per-source counter link: `cards_complex_creatures.rs`, `state_based_actions.rs`
