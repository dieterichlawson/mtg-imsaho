## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/102/ghoulraiser?utm_source=api
**Type line**: `Creature — Zombie` — {1}{B}{B}, 2/2
**Oracle text**:
```
When this creature enters, return a Zombie card at random from your graveyard to your hand.
```

**Status**: PASS

### Code issues
No issues found.

- "return a **Zombie card** at random from your graveyard" — reads subtypes from
  the card's face, which is the right source for a graveyard card: continuous
  effects that grant types apply to permanents, not to cards in a graveyard.
  A Zombie *token* in the graveyard has no registry face, so it is excluded and
  CR 109.1 is satisfied without a separate check.
- "Zombie card", not "Zombie creature card" — the filter is deliberately not
  restricted to creatures, and a comment says so.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
