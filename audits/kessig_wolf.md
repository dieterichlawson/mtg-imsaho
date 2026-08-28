## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/151/kessig-wolf?utm_source=api
**Type line**: `Creature — Wolf` — {2}{R}, 3/1
**Oracle text**:
```
{1}{R}: This creature gains first strike until end of turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{1}{R}: This creature gains first strike until end of turn" — no activation
  limit, so it stacks harmlessly with itself: PASS
- Granted mid-combat after first-strike damage has been dealt does not give it a
  second damage step (CR 510.4): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The keyword grant: `activated_abilities.rs:a_pump_ability_changes_the_creature_it_is_activated_on`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/151/kessig-wolf?utm_source=api
**Type line**: `Creature — Wolf` — {2}{R}, 3/1
**Oracle text**:
```
{1}{R}: This creature gains first strike until end of turn.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**: `{1}{R}: This creature gains first strike until end of turn.`
**Type line**: `Creature — Wolf` — {2}{R}, 3/1
**Status**: ISSUE (fixed) — one test gap, shared with two other cards; the card is correct

### Rulings
None on Scryfall.

### Code issues

No issues in the card. `{2}{R}`, Creature — Wolf, 3/1, no printed keywords (first strike is granted, not printed — Scryfall lists none either, which `keywords_say_what_scryfall_says` now checks), oracle text verbatim, one `ActivatedAbilityDef` at `{1}{R}` with no tap, no target and no restriction, and the grant as a `TemporaryEffect::GrantKeyword` in `until_end_of_turn` — the documented way, rather than writing `obj.keywords`, which the engine ignores for a registry card.

The gap was the duration. **Reimplementing "gains first strike until end of turn" as a permanent `instance_continuous_effects` grant passed the entire workspace**: every test of this card looks only at the turn the ability was activated, so a first strike that never went away was indistinguishable from one that did.

The same held for the two other cards of this shape — Feral Ridgewolf's and Darkthicket Wolf's "+N/+N until end of turn" were equally undetectable as permanent grants. Closed with one table over all three, in `activated_abilities.rs::a_pump_ability_wears_off_at_end_of_turn`, advancing real turns rather than clearing `until_end_of_turn` by hand — a test that cleared it itself would pass with the engine's cleanup deleted. Mindshrieker already had this test for its own card, with the same reasoning written out; nothing else did.

### Tricky interactions checked

- The grant goes to this creature: PASS, tested.
- It is first strike and not some other keyword: PASS, tested.
- "until end of turn": PASS. Untested until this audit.
- Repeatable — no "once each turn" printed, so two activations are legal (and idempotent for a keyword): PASS by the absence of the flag, which the cost invariant now pins to the text.
- Instant speed: PASS, `sorcery_speed_only: false`, pinned by the same invariant.
- First strike's *effect* in combat (a separate first-strike damage step): engine-general, covered by the combat tests rather than per card.
- Redundant `zone == Battlefield` gate in `activated_abilities`: present, left alone — one of the 29 recorded in the Mirror-Mad Phantasm entry.

### Test coverage

- One activation grants first strike: `activated_abilities.rs:26` `a_pump_ability_changes_the_creature_it_is_activated_on` (table row)
- The grant wears off at end of turn: `activated_abilities.rs:101` `a_pump_ability_wears_off_at_end_of_turn`, added this audit (three cards)
- The declared cost is `{1}{R}` with no restriction flags: `card_data_invariants.rs:1706` (added during the Darkthicket Wolf audit)
- It declares no printed keywords: `card_data_invariants.rs:1790` (added during the Feral Ridgewolf audit)
- Used as the non-Human in Avacynian Priest's targeting test: `activated_abilities.rs:191`

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 grant Trample instead of FirstStrike | `a_pump_ability_changes_the_creature_it_is_activated_on` FAILED | (unchanged) |
| M2 grant to `ObjectId(0)` | same test FAILED | (unchanged) |
| M3 permanent `OnSelf` grant instead of until-end-of-turn | passed whole workspace | `a_pump_ability_wears_off_at_end_of_turn` FAILED |
| M4 the same for Darkthicket Wolf's +2/+2 | passed whole workspace | same test FAILED |

M3's first attempt did not compile — I reached for a `state.permanent_grants` field that does not exist. Recorded because a mutation that fails to compile proves nothing; it was redone through `instance_continuous_effects`, which is a real way to write the card wrongly, and only then counted.

Source restored from `/tmp/kw.bak` and `/tmp/dw3.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1491 passing (was 1490). `cargo check --workspace --all-targets` clean, zero warnings.
