## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/53/dissipate?utm_source=api
**Type line**: `Instant` — {1}{U}{U}
**Oracle text**:
```
Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
```

**Status**: PASS

### Code issues
No issues found.

**Ruling [2004-10-04]**: "If the spell is not countered (because the spell it
targets can't be countered), then it does not get exiled." And: "The card does
not go to the graveyard before being exiled."

- The exile is conditional on the counter actually happening, and the countered
  spell goes straight to exile rather than to the graveyard and then out.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/53/dissipate?utm_source=api
**Type line**: `Instant` — {1}{U}{U}
**Oracle text**:
```
Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The card does **not** go to the graveyard before being exiled." The
  spell is taken off the stack and moved straight to exile in one step: PASS
- This is the one card that legitimately does *not* use `move_countered_spell`.
  That helper sends a countered spell where it would normally go — graveyard, or
  exile for flashback — and Dissipate's whole text is that the destination is
  different: "exile it **instead of** putting it into its owner's graveyard". A
  countered flashback spell would be exiled either way, so the two never
  disagree: PASS
- Ruling: "If the spell is **not** countered (because the spell it targets can't
  be countered), then it does **not** get exiled." No card in this set is
  uncounterable, so the case is unreachable here; the exile is gated on the
  target still being on the stack, which is the same guard: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Countering and exiling: `cards_counterspells.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/53/dissipate?utm_source=api
**Type line**: `Instant` — {1}{U}{U}
**Oracle text**:
```
Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
```

**Rulings fetched**:
- [2004-10-04] The card does not go to the graveyard before being exiled.
- [2004-10-04] If the spell is not countered (because the spell it targets can't be countered), then it does not get exiled.

**Status**: PASS

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/53/dissipate
**Oracle text**:
```
Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
```
**Type line**: `Instant` · **Mana cost**: `{1}{U}{U}`
**Rulings** (2, both 2004-10-04, https://api.scryfall.com/cards/5d778082-bcdb-423a-b16f-57ac0d4dace7/rulings):
1. "The card does not go to the graveyard before being exiled."
2. "If the spell is not countered (because the spell it targets can't be countered), then it does not get
   exiled."

**Status**: PASS (one test added; both rulings' testability recorded)

### Card data
| field | oracle | `dissipate.rs` | |
|---|---|---|---|
| cost | `{1}{U}{U}` | `Generic(1) + Blue + Blue` | ok |
| types | Instant | `vec![CardType::Instant]` | ok |
| oracle_text | as above | byte-identical | ok |
| targeting | "target spell" | `TargetRequirement::Spell` + `helpers::spell_target_is_legal` | ok |
| effect | counter, exile instead of graveyard | `helpers::counter_spell_exiling` | ok |

### Code issues
No issues found. The card is three lines on top of the shared countering pipeline extracted during the Lost in
the Mist audit — which is where Dissipate's own drift (`stack.retain` then a bare `move_object(Exile)`) was
fixed. CR 614.1a replacing CR 701.5a's default destination is what `counter_spell_exiling` exists to express.

### Both rulings have limits, and mutation found them
This is the substance of the audit, so it is written out rather than summarised as "covered".

**Ruling 1 — "The card does not go to the graveyard before being exiled."** Structurally true:
`counter_spell_exiling` removes the stack entry and then calls `move_object(spell_id, Zone::Exile, ..)` in one
step, with no graveyard in between. But it is **not observable**. There is no generic zone-change event; the
graveyard-adjacent ones are `LeftBattlefield` (battlefield only) and `CreatureCardMilled` (library → graveyard
only), and nothing in the set triggers on a card arriving in a graveyard. Mutation 3 inserted a graveyard
stopover before the exile and **nothing failed**. Recorded as structurally satisfied, not claimed as tested.

**Ruling 2 — "If the spell is not countered ..., then it does not get exiled."** The ruling's premise is
Dissipate *resolving* against a spell that can't be countered. Nothing in this set can't be countered, and with
a single target the only way the counter does not happen is the spell not resolving at all — `on_resolve` is
never called, so `counter_spell_exiling`'s own "is it still on the stack?" guard is never reached. Mutation 1
made that guard exile regardless, and **nothing failed**.

My first version of the new test claimed to cover ruling 2. It did not, and the mutation is what showed it: the
Bears staying put is because Dissipate never resolved, not because any rider checked anything. The test's
comment now states plainly what it does and does not show.

### Changes made
- `mtg-engine/tests/fizzle.rs` — `a_fizzling_dissipate_goes_to_the_graveyard_and_exiles_nothing`. What it holds
  is the half a careless rider could get wrong: "exile it instead" is about the *countered* spell, so a
  Dissipate that fizzles must land in its owner's graveyard like any other spell rather than exiling itself.

### Mutation checks
1. `counter_spell_exiling` exiling even when nothing was on the stack to counter → **vacuous** (unreachable; see
   ruling 2 above).
2. Dissipate calling `counter_spell` instead of `counter_spell_exiling` → `dissipate_counters_and_exiles`
   FAILED. **Discriminating.**
3. A graveyard stopover inserted before the exile → **vacuous** (unobservable; see ruling 1 above).
4. A fizzling spell exiled instead of put into its owner's graveyard →
   `a_fizzling_dissipate_goes_to_the_graveyard_and_exiles_nothing` FAILED along with three other fizzle tests.
   **Discriminating.**

### Tricky interactions checked
- Counters a spell and exiles it rather than binning it: **pass** (`cards_removal_and_bounce.rs:53`).
- Target already off the stack → Dissipate is countered by game rules, goes to *its* graveyard, exiles nothing:
  **pass** (new).
- A flashback spell countered by Dissipate: exiled either way — `move_countered_spell` already exiles a
  flashback spell (CR 702.33a) — so the two paths agree and no test can separate them. Recorded, not claimed.
- Card code no longer touches `state.stack`: enforced by `no_card_removes_a_spell_from_the_stack_itself`.

### Test coverage
- counters and exiles: `cards_removal_and_bounce.rs:53`
- fizzling Dissipate goes to the graveyard, exiles nothing: `fizzle.rs:256` (new)
- countering goes through the shared pipeline: `card_data_invariants.rs` guard
- ruling 1: structurally satisfied, **not observable** — see above
- ruling 2: **unreachable** in this set — see above

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1425 passing.

