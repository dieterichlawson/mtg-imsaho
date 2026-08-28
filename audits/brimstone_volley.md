## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/132/brimstone-volley?utm_source=api
**Type line**: `Instant` — {2}{R}
**Oracle text**:
```
Brimstone Volley deals 3 damage to any target.
Morbid — Brimstone Volley deals 5 damage instead if a creature died this turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Morbid — deals 5 damage **instead** if a creature died this turn" — one
  damage event of 5, not 3 plus 2, and the condition is read when the spell
  resolves rather than when it was cast: PASS
- "any target" — creature, player or planeswalker: PASS
- Damage goes through `deal_damage`, so protection, prevention and the
  planeswalker loyalty path (CR 120.3c) all apply: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both damage amounts and the morbid condition: `cards_burn_and_damage.rs`, `cards_morbid_and_ltb.rs`
- Planeswalker targeting: `damage_helper.rs:every_any_target_spell_can_point_at_a_planeswalker`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/132/brimstone-volley?utm_source=api
**Type line**: `Instant` — {2}{R}
**Oracle text**:
```
Brimstone Volley deals 3 damage to any target.
Morbid — Brimstone Volley deals 5 damage instead if a creature died this turn.
```

**Rulings fetched**: none published for this card.

**Status**: PASS

### Code issues

No issues found. The card needed no change.

### Card data

`{2}{R}` Instant, `TargetRequirement::AnyTarget` for "any target" — pinned by
`card_data_invariants.rs::any_target_in_the_text_means_any_target_in_the_requirement`
as well as by the cost and type-line invariants. Damage through
`helpers::resolve_damage`, which goes to `deal_damage` as noncombat damage, so
protection, `damaged_by` and the rest of the pipeline apply. No card-side
cleanup.

Morbid is not a keyword ability with its own machinery — it is a condition
inside the effect — and the card reads `state.creature_died_this_turn` in
`on_resolve`, which is where CR 608.2 says the value is worked out.

### Tricky interactions checked

- **When morbid is read.** As the spell *resolves*, not when it is cast.
  Brimstone Volley is an instant, so the window is real: cast it with nothing
  dead, kill something in response, and it deals 5. Untested until now — and
  both existing tests set `creature_died_this_turn` by hand, so neither ran a
  real death through the flag at all.
- **"this turn."** A death on the previous turn does not carry over; the flag
  is cleared as the turn advances. Untested until now.
- Any target — creature, player or planeswalker — is the shared requirement
  and is pinned pool-wide.
- Whose creature died does not matter ("a creature"), and a token dying counts;
  both fall out of the flag being set by any death.

### Test coverage

- 5 damage with morbid: `cards_morbid_and_ltb.rs::brimstone_volley_morbid_deals_5`
- 3 without: `cards_morbid_and_ltb.rs::brimstone_volley_no_morbid_deals_3`
- morbid read on resolution, through a real death:
  `cards_morbid_and_ltb.rs::brimstone_volley_reads_morbid_when_it_resolves` (new)
- a death on the previous turn does not count:
  `cards_morbid_and_ltb.rs::brimstone_volley_forgets_a_death_from_the_previous_turn` (new)

### Mutations run

- The card always deals 3: **fails** the two morbid tests.
- `creature_died_this_turn` is never cleared at end of turn: **fails** the
  previous-turn test, and only that one.
- `destruction::destroy` stops setting the flag: **fails** both new tests,
  which are the ones that kill a creature for real.
- (A first attempt at that last mutation edited the wrong one of the flag's
  three set-sites — `sba.rs`'s zero-toughness path and the batch loop in
  `destruction.rs` — and changed nothing, because `kill_by_damage` destroys
  through `destroy()`. Redone there. Worth recording: the morbid flag is set
  in three places, and no single test isolates which.)

Suite: 1549 passing, exit 0, `cargo check --workspace --all-targets` clean.
