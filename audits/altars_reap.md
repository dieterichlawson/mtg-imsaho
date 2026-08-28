## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/86/altars-reap?utm_source=api
**Type line**: `Instant` — {1}{B}
**Oracle text**:
```
As an additional cost to cast this spell, sacrifice a creature.
Draw two cards.
```
**Status**: PASS

### Code issues
No issues found.

Draws two. The sacrifice is an additional cost paid at cast time (CR 601.2f), so it happens even if the spell is later countered — correctly not part of resolution.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/86/altars-reap?utm_source=api
**Type line**: `Instant` — {1}{B}
**Oracle text**:
```
As an additional cost to cast this spell, sacrifice a creature.
Draw two cards.
```

**Status**: PASS

### Code issues
No issues found.

Both rulings hold structurally. "You must sacrifice exactly one creature" —
`AdditionalCost::SacrificeCreature` is a fixed one, and `legal_actions` will not
offer the cast with no creature to sacrifice. "Players can only respond once
this spell has been cast and all its costs have been paid" — the sacrifice is
paid in `pay_additional_cost` during the cast, before the spell is on the stack
and before anyone gets priority; `on_resolve` only draws.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_sacrifice_and_additional_costs.rs` — the creature is gone before the spell can be responded to.

## Audit — 2026-08-28 19:37

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: As an additional cost to cast this spell, sacrifice a creature.
Draw two cards.
**Type line**: Instant
**Status**: ISSUE

### Code issues
Card data (`mtg-engine/src/cards/isd/altars_reap.rs`) matches oracle: {1}{B} instant, `additional_cost: Some(AdditionalCost::SacrificeCreature)`, on_resolve draws two. The issue was in the engine's cost-payment path, found while testing the rulings:

- **A submitted cast with an unpayable/illegal additional cost was accepted** (`mtg-engine/src/engine/costs.rs`, `pay_additional_cost` and its callers).
  - Oracle text says: `As an additional cost to cast this spell, sacrifice a creature.` (CR 601.2h: costs must actually be paid; a cast whose costs cannot be paid is rewound.)
  - Code did: `pay_additional_cost` took the submitted `sacrifice`/`exile_ids` entirely on trust. Three concrete holes: (1) a cast of Altar's Reap submitted with no creature on the battlefield went on the stack with the cost unpaid; (2) a named `sacrifice` id was never checked to be a creature the caster controls on the battlefield — an opponent's creature could be "sacrificed"; (3) explicit `exile_ids` (Corpse Lunge et al.) were never checked to be creature cards in the caster's own graveyard.
  - Fix: new `additional_cost_is_payable` in `mtg-engine/src/engine/costs.rs`, called from `cast_spell` (`mtg-engine/src/engine/actions/cast.rs`) before any payment; a failing check refuses the cast entirely (spell stays in hand, no mana paid). Committed as bcf5987.

### Tricky interactions checked
- Ruling: "You must sacrifice exactly one creature to cast this spell; you can't cast it without sacrificing a creature": now enforced — refused with none available or none legal. PASS (after fix)
- Ruling: "No player may take actions between the time you sacrifice a creature and the time the spell is cast" (cost paid at cast, not at resolution): sacrifice happens during `cast_spell`, creature is in the graveyard while the spell is on the stack; an opponent's removal in response cannot save it. PASS
- Named sacrifice must be the caster's own battlefield creature: PASS (after fix)
- Offer side (`legal_actions`) already required a creature before offering the cast; only the submit path was open. Both halves now validate.
- Draw two on resolution goes through `crate::engine::draw_cards` (empty-library rule applies). PASS

### Test coverage
- Main effect (sacrifice + draw two): `mtg-engine/tests/cards_sacrifice_and_additional_costs.rs` `altars_reap_sacrifices_and_draws_two`
- Ruling 1 (can't cast without a creature): `cards_sacrifice_and_additional_costs.rs` `altars_reap_cannot_be_cast_without_a_creature` — spell stays in Hand, no mana paid
- Ruling 2 (cost paid at cast time): `cards_sacrifice_and_additional_costs.rs` `altars_reap_sacrifice_happens_with_the_cast_not_the_resolution` — creature in Graveyard while spell still on Stack
- Opponent's creature as named sacrifice refused: `mtg-engine/tests/submitted_targets.rs` `a_sacrifice_cost_cannot_be_paid_with_an_opponents_creature`
- Opponent's graveyard as exile source refused: `submitted_targets.rs` `an_exile_cost_cannot_be_paid_from_an_opponents_graveyard`

Mutation checks (each mutation applied alone, suite section rerun, then reverted):
- Removing the named-sacrifice ownership/zone check → `a_sacrifice_cost_cannot_be_paid_with_an_opponents_creature` FAILS (and only it). Bites.
- Removing the explicit-exile-ids validation → `an_exile_cost_cannot_be_paid_from_an_opponents_graveyard` FAILS. Bites.
- Removing the no-creature-available refusal → `altars_reap_cannot_be_cast_without_a_creature` FAILS. Bites.
