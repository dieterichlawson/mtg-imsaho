## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/175/darkthicket-wolf?utm_source=api
**Type line**: `Creature — Wolf` — {1}{G}, 2/2
**Oracle text**:
```
{2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Activate only once each turn" — `once_per_turn: true`, tracked per object and
  reset at untap: PASS
- The pump is until end of turn, not permanent: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- One activation pumps: `activated_abilities.rs:a_pump_ability_changes_the_creature_it_is_activated_on`
- The once-per-turn restriction: `activated_abilities.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/175/darkthicket-wolf?utm_source=api
**Type line**: `Creature — Wolf` — {1}{G}, 2/2
**Oracle text**:
```
{2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**: `{2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.`
**Type line**: `Creature — Wolf` — {1}{G}, 2/2
**Status**: ISSUE (fixed) — test gaps; the card is correct

### Rulings
None on Scryfall.

### Code issues

No issues in the card. `{1}{G}`, Creature — Wolf, 2/2, oracle text verbatim, one `ActivatedAbilityDef` at `{2}{G}` with `once_per_turn: true`, no tap and no target, and the pump as a `TemporaryEffect::ModifyPT` in `until_end_of_turn` rather than a write to the object's P/T.

The gaps were in the tests, and two of them were the same shape — a declared *cost* or *restriction* that no test ever exercised:

- Charging `{3}` instead of `{2}{G}` passed the whole workspace. A colour requirement is only ever exercised by a test that happens to pay for it in the right colours, and every Darkthicket Wolf test does `add_mana(Colorless 2, Green 1)`, which pays either cost.
- Setting `sorcery_speed_only: true` passed the whole workspace. Nothing activated the ability outside a main phase.

Both are classes rather than cases, so both are closed pool-wide: `card_data_invariants.rs::activated_ability_costs_are_the_costs_the_oracle_text_prints` parses the cost half of every "cost: effect" line (CR 602.1) out of the oracle text — which is itself already pinned to Scryfall — and compares it with the `ActivatedAbilityDef` the engine charges: the mana, `{T}`, and the two printed restrictions, "Activate only once each turn" and "Activate only as a sorcery". It is deliberately restricted to cards printing exactly one activated ability and declaring exactly one; pairing lines to abilities on a card with several means guessing, and a guess would be a test that passes for the wrong reason. Nineteen cards, with a coverage floor so it cannot quietly stop covering anything.

### Tricky interactions checked

- "Activate only once each turn": PASS, `once_per_turn: true`, and the restriction is per *turn* rather than per game — `activated_abilities.rs:126` proves it by advancing real turns rather than by clearing `abilities_activated_this_turn` itself.
- "until end of turn": PASS, through `until_end_of_turn` so the engine's cleanup removes it, not the card.
- CR 602.2a, the ability uses the stack: PASS, `activated_no_stack.rs:337` shows the +2/+2 has not happened while the opponent still has priority.
- Available at instant speed: PASS. Untested until this audit.
- The cost's green pip: PASS. Untested until this audit.
- The Wolf leaving the battlefield in response: the ability still resolves and pushes a `ModifyPT` at an object that is gone, which does nothing (CR 400.7). Not separately tested; nothing distinguishes it from the effect simply not applying.
- Redundant `zone == Battlefield` gate in `activated_abilities`: present, and left alone — one of the 29 recorded in the Mirror-Mad Phantasm entry. `legal_actions` enumerates only battlefield permanents.

### Test coverage

- One activation makes it a 4/4: `activated_abilities.rs:26` `a_pump_ability_changes_the_creature_it_is_activated_on` (table row)
- "Activate only once each turn", blocked this turn and offered the next: `activated_abilities.rs:126` `a_once_per_turn_ability_is_blocked_this_turn_and_offered_the_next`
- The ability goes on the stack and resolves separately: `activated_no_stack.rs:337` `activating_through_the_engine_leaves_the_ability_on_the_stack`
- Castable/payable through the mana filter machinery: `mana_filters.rs:175`
- Available during combat, not sorcery-speed: `activated_abilities.rs:120` `a_pump_ability_with_no_speed_restriction_is_offered_during_combat`, added this audit
- The declared cost is `{2}{G}`, and the printed restrictions match the flags: `card_data_invariants.rs:1706`, added this audit (19 cards)

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 `once_per_turn: false` | `a_once_per_turn_ability_is_blocked_this_turn_and_offered_the_next` FAILED | + the new invariant FAILED |
| M2 `+2/+2` -> `+3/+3` | 2 tests FAILED | (unchanged) |
| M3 cost `{2}{G}` -> `{3}` | passed whole workspace | `activated_ability_costs_are_the_costs_the_oracle_text_prints` FAILED |
| M4 `sorcery_speed_only: true` | passed whole workspace | `a_pump_ability_with_no_speed_restriction_is_offered_during_combat` and the new invariant FAILED |
| M5 `requires_tap: true` | n/a | the new invariant FAILED |

Source restored from `/tmp/dw.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1484 passing (was 1482). `cargo check --workspace --all-targets` clean, zero warnings.
