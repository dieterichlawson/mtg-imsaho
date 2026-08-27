## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/146/harvest-pyre?utm_source=api
**Type line**: `Instant` — {1}{R}
**Oracle text**:
```
As an additional cost to cast this spell, exile X cards from your graveyard.
Harvest Pyre deals X damage to target creature.
```

**Status**: PASS

### Code issues
No issues found.

- "As an additional cost to cast this spell, exile X cards from your graveyard.
  Harvest Pyre deals X damage to target creature." — X is fixed by the
  additional cost paid at cast time (CR 601.2f), not chosen again at
  resolution.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/146/harvest-pyre?utm_source=api
**Type line**: `Instant` — {1}{R}
**Oracle text**:
```
As an additional cost to cast this spell, exile X cards from your graveyard.
Harvest Pyre deals X damage to target creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**As an additional cost to cast this spell, exile X cards from your
  graveyard**" — paid on casting, so the cards are already in exile while the
  spell is on the stack and countering it does not give them back: PASS
- X is set by how many were exiled, not by an {X} in the mana cost — so there is
  no X-funding prompt: PASS
- "deals X damage to **target creature**", not any target: PASS
- CR 109.1: "X **cards** from your graveyard", so tokens are not payable: PASS
- Damage through `deal_damage`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The additional cost and the scaling damage: `cards_additional_costs.rs`, `cards_burn_and_damage.rs`
