## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/m21/199/rangers-guile?utm_source=api
**Type line**: `Instant` — {G}
**Oracle text**:
```
Target creature you control gets +1/+1 and gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Target creature **you control**" — `TargetFilter::YouControl` and the card's
  own `is_valid_target`: PASS
- Hexproof until end of turn is what makes a targeted removal spell already on
  the stack fizzle (CR 608.2b) — the interaction the ability exists for: PASS
- Granting hexproof to your own creature does not stop *your* spells targeting
  it: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The pump and the hexproof save: `cards_pump_spells.rs`, `fizzle.rs:a_target_that_gained_hexproof_in_response_is_skipped_and_the_rest_resolve`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/m21/199/rangers-guile?utm_source=api
**Type line**: `Instant` — {G}
**Oracle text**:
```
Target creature you control gets +1/+1 and gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/m21/199/rangers-guile
(the cached printing is M21; Oracle text is per-card, not per-printing, and the ISD printing shares it)
**Oracle text**:
```
Target creature you control gets +1/+1 and gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)
```
**Type line**: `Instant` · **Mana cost**: `{G}`
**Rulings**: none published for this card.
**Status**: ISSUE (fixed) — an engine re-check that never asked whether a "target creature" was still a creature.

### Card data
| field | oracle | `rangers_guile.rs` | |
|---|---|---|---|
| cost | `{G}` | `Colored(Green)` | ok |
| types | Instant | `vec![CardType::Instant]` | ok |
| oracle_text | as above, reminder text included | byte-identical | ok |
| targeting | "target creature you control" | `CreatureWithFilter(YouControl)` | ok |
| effect | +1/+1 and hexproof, until end of turn | two `TemporaryEffect`s on `until_end_of_turn` | ok |

### Code issues

**`stack::is_target_legal` did not re-check creature-ness for `CreatureWithFilter`.** Fixed.

CR 608.2b re-checks each target as the spell resolves. For `CreatureWithFilter` the re-check tested the zone and
re-ran the filter — but a filter carries the *rest* of the restriction ("you control", "power 4 or greater",
"isn't a Vampire"), never creature-ness. The enumeration does check it (`.filter(|o| state.is_creature(..))`);
the re-check did not. The two disagreed.

Seven cards papered over it with the same zone-and-`is_creature` preamble in their own `is_valid_target`. Four
of them — Reaper from the Abyss, Rebuke, Smite the Monstrous, Victim of Night — need `is_valid_target` for a
further restriction and keep it; their preamble is now redundant but harmless, and each is one audit away.
Ranger's Guile's guard was *only* the preamble:

```rust
o.zone == Zone::Battlefield && state.is_creature(o.id, registry) && o.controller == caster
```

which is `CreatureWithFilter(YouControl)` restated. With the re-check fixed, it is gone.

### On testing an engine change the card pool cannot reach
The first mutation of the new check was **vacuous**: removing it again failed nothing across the whole suite.
That is not surprising — nothing in this set turns a creature into a non-creature, so no card can stage the
state the rule governs.

Leaving an engine change unverified was not acceptable, so the state is built directly:
`ready_creature` makes an anonymous object that is a creature by virtue of carrying a P/T (CR 205.1b, and
`card_types_of` derives `Creature` from `o.power.is_some()`), and clearing the P/T makes it stop being one. The
test says in its own comment that it is synthetic and why. Re-run against it, the mutation FAILS.

### Rules check
- **CR 702.11b** — hexproof stops opponents only; the granting player can still target their own creature. Comes
  from `can_be_targeted_by`, which reads the target's controller.
- **"until end of turn"** — both effects go on `until_end_of_turn`, cleared in the cleanup step, so they expire
  together.
- **CR 608.2b** — with hexproof gained in response, an opponent's single-target removal has no legal target and
  is countered by game rules.

### Changes made
- `mtg-engine/src/stack.rs` — the `CreatureWithFilter` creature re-check.
- `mtg-engine/src/cards/isd/rangers_guile.rs` — `is_valid_target` removed, with a comment saying what applies
  the rule instead.
- `mtg-engine/tests/fizzle.rs` — two tests:
  - `a_target_creature_that_stopped_being_a_creature_is_no_longer_legal` (the engine rule, synthetic).
  - `rangers_guile_counters_removal_by_granting_hexproof` — the card doing the job it exists for. Worth noting
    that `fizzle.rs` already *simulated* this card by hand, pushing a raw `GrantKeyword` and commenting
    "Ranger's Guile is in this set", without ever casting it. Asserted through `resolved()`, because a
    single-target spell that resolved and found nothing to do leaves the same board.
- `mtg-engine/tests/cards_vanilla_and_keywords.rs` — `rangers_guile_wears_off_at_end_of_turn`, both halves.

### Mutation checks
1. Hexproof grant removed from the card → `rangers_guile_gives_hexproof_and_pump` and
   `rangers_guile_wears_off_at_end_of_turn` FAILED.
2. `until_end_of_turn.clear()` removed from the cleanup step → `rangers_guile_wears_off_at_end_of_turn` FAILED.
3. The new `CreatureWithFilter` re-check removed → **vacuous** against the card suite (0 failures), then
   discriminating against the synthetic test written for it. Both results recorded above.

### Tricky interactions checked
- +1/+1 and hexproof granted: **pass** (`cards_vanilla_and_keywords.rs:92`).
- Only your own creatures may be targeted: **pass** (`cards_vanilla_and_keywords.rs:434`).
- Cast in response to removal → the removal is countered: **pass** (new).
- Both halves expire at end of turn: **pass** (new).
- Hexproof does not stop its own controller targeting the creature: covered by `can_be_targeted_by`'s controller
  comparison, tested for granted hexproof under Mask of Avacyn.

### Test coverage
- +1/+1 and hexproof: `cards_vanilla_and_keywords.rs:92`
- targets only your own creatures: `cards_vanilla_and_keywords.rs:434`
- wears off at end of turn: `cards_vanilla_and_keywords.rs:449` (new)
- counters removal by granting hexproof: `fizzle.rs:445` (new)
- a target that stopped being a creature: `fizzle.rs:415` (new)

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1422 passing.

