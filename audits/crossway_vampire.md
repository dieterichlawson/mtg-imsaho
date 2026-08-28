## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/135/crossway-vampire?utm_source=api
**Type line**: `Creature — Vampire` — {1}{R}{R}, 3/2
**Oracle text**:
```
When this creature enters, target creature can't block this turn.
```

**Status**: PASS

### Code issues
No issues found.

'target creature can't block this turn' — targeted, locked at trigger time, applied through the shared `CantBlockThisTurn` effect rather than a hand-rolled flag.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/135/crossway-vampire?utm_source=api
**Type line**: `Creature — Vampire` — {1}{R}{R}, 3/2
**Oracle text**:
```
When this creature enters, target creature can't block this turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "When this creature enters, target creature **can't block this turn**" — a
  blocking restriction until end of turn, not a tap: PASS
- Targeted at CR 603.3d time: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The block restriction: `combat_rules.rs`, `cards_complex_creatures.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/135/crossway-vampire?utm_source=api
**Type line**: `Creature — Vampire` — {1}{R}{R}, 3/2
**Oracle text**:
```
When this creature enters, target creature can't block this turn.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/135/crossway-vampire
**Oracle text**: When this creature enters, target creature can't block this turn.
**Type line**: Creature — Vampire
**Mana cost**: {1}{R}{R} — **P/T**: 3/2
**Rulings**: none (Scryfall returns no rulings for this card)
**Status**: ISSUE (fixed) — the card code is correct; nothing tested what it does, and its one targeted test was vacuous.

### Card data
Matches the fetched text: `{1}{R}{R}`, `card_types: [Creature]`,
`subtypes: ["Vampire"]`, 3/2, oracle text verbatim in the current "When this
creature enters" errata wording, no keywords. One `TriggeredAbilityDef` of kind
`EntersBattlefield` with `target_requirement: Some(Creature)`, matching the one
implemented hook, and `has_etb_handler()` returns true.

The trigger is **mandatory** — no "you may" — so the hook applies the effect to
the locked target directly rather than raising a decision, which is right.
`PendingEffect::CantBlockThisTurn` pushes `TemporaryEffect::CantBlock`, so
"this turn" is the engine's cleanup step rather than the card's business.

### Code issues

No issue in `crossway_vampire.rs`. Two in the tests.

1. **The card's entire effect was untested**
   (`combat_rules.rs`, test added).
   - Oracle text says: `target creature can't block this turn`
   - Verified: replacing the body of `on_enter_battlefield` with
     `let _ = (&effect, registry, state, target);` — the hook does nothing at
     all — produced zero failures across the whole workspace.
   - Its only coverage was `hexproof_filter.rs:98` (which targets are offered,
     and see finding 2) and `phantom_triggers.rs:44` (that it *has* an ETB
     trigger). Neither needs the effect to exist.
   - Added `crossway_vampire_stops_its_target_blocking_for_the_turn`: the
     targeted creature drops out of `combat::eligible_blockers`, the creature
     beside it does not, and the restriction is gone by the next turn —
     reached through `advance_to_next_turn`, so it is the engine's cleanup
     doing the removing.

2. **`bug_17_003_crossway_vampire_creature_targets_excludes_hexproof` was
   vacuous — and so was its Fiend Hunter twin**
   (`hexproof_filter.rs:98` and `:135`, both replaced).
   - Both did:
     `behavior.on_enter_battlefield(&mut state, id, &[], &registry);`
     — an **empty** `chosen_targets` — then asked whether a hexproof creature
     was among the options of `state.awaiting_action`.
   - Both cards declare a `target_requirement` now (CR 603.3d), so the hook
     opens `let Some(target) = chosen_targets.first() else { return };` and
     sets nothing pending. Each test's `_ => false` arm then made its negative
     assertion trivially true. They passed against an inert hook, and would
     have passed against a card that offered every hexproof creature in the
     game.
   - Their comments still describe the cards enumerating targets themselves via
     `creature_targets` / `creature_targets_except`, which neither has done
     since the target moved onto the `TriggeredAbilityDef`.
   - Both are now rows in
     `an_etb_trigger_does_not_offer_an_opponents_hexproof_creature`, which
     pushes `EnteredBattlefield`, runs `collect_triggers`, and reads either the
     locked target or the real prompt — the engine's own enumeration, which is
     the thing Bug 17-003 was ever about. Fiend Hunter is a different card's
     audit, but it is the same defect in the same file with the same one-line
     fix, so leaving it would have been flagging and moving on.

### Tricky interactions checked
- The targeted creature can't block: PASS — new test.
- Another creature under the same controller still can: PASS — new test's
  second assertion, so the effect is a target and not a sweep.
- "this turn": PASS — the restriction is gone after `advance_to_next_turn`.
- An opponent's hexproof creature is not offered: PASS —
  `an_etb_trigger_does_not_offer_an_opponents_hexproof_creature`, now driving
  the real enumeration for this card.
- "target creature" reaches any creature, including your own and the Vampire
  itself: `TargetRequirement::Creature` with no filter, which matches the
  printed text; the new test asserts both of the opponent's creatures are
  offered.
- Mandatory, not "you may": the hook calls `apply_pending_effect` directly
  rather than raising a choice. Structural.
- The Vampire leaves the battlefield before the trigger resolves: the effect
  targets a creature and does not read the source, so it still applies
  (CR 113.7a).
- Target becomes illegal between trigger and resolution: CR 608.2b, the
  engine's re-check, generic.
- Self-cleanup: none; this is a permanent.

### UI presentation
Trigger description: "target creature can't block this turn". The effect logs
"Crossway Vampire prevents {name} from blocking this turn" from
`engine/effects.rs`, naming the source.

### Test coverage
- The targeted creature can't block, its neighbour can, and it wears off:
  `combat_rules.rs` (`crossway_vampire_stops_its_target_blocking_for_the_turn`)
  — **added this audit**.
- Hexproof creature not offered: `hexproof_filter.rs`
  (`an_etb_trigger_does_not_offer_an_opponents_hexproof_creature`) —
  **moved there this audit**, from a test that could not fail.
- Has an ETB trigger at all: `phantom_triggers.rs:44`.
- The underlying `CantBlock` rule: `turn_structure.rs:528`.
- No rulings exist for this card, so there is no per-ruling row to fill.

### Mutations run
| mutation | result |
| --- | --- |
| `on_enter_battlefield` does nothing | fails the new test (before: **nothing at all**) |

Suite after: 1458 passing, exit 0, zero warnings. (1459 before; the two vacuous
tests became two rows in an existing one.)

