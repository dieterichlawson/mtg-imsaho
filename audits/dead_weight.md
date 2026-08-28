## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/96/dead-weight?utm_source=api
**Type line**: `Enchantment — Aura` — {B}
**Oracle text**:
```
Enchant creature
Enchanted creature gets -2/-2.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- -2/-2 kills a 2/2 by state-based action rather than by destruction, so
  indestructible does not save it (CR 704.5f): PASS
- The Aura goes to the graveyard with the creature it killed (CR 704.5m): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The debuff and the SBA death: `enchantments.rs`, `state_based_actions.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/96/dead-weight?utm_source=api
**Type line**: `Enchantment — Aura` — {B}
**Oracle text**:
```
Enchant creature
Enchanted creature gets -2/-2.
```

**Rulings fetched**: none published for this card.

**Status**: PASS

### Code issues

No issues found. The card needed no change.

### Card data

`{B}` Enchantment — Aura, `TargetRequirement::Creature` for "Enchant
creature", one continuous effect
`ModifyPT { power: -2, toughness: -2, scope: Attached }` for "Enchanted
creature gets -2/-2". Cost, type line and subtypes pinned pool-wide by
`card_data_invariants.rs`; `Enchant` is one of the keywords the keyword
invariant deliberately does not model. `resolve_aura` for attachment, no
`is_valid_target` override, no card-side cleanup, and correctly no LTB
trigger — `empty_triggers.rs` already fails the build if one appears, since
the card has no leaves-the-battlefield text.

### Tricky interactions checked

- **The two-step state-based action, which is the card's whole point**: the
  creature goes to the graveyard for 0 toughness (CR 704.5f) and the Aura,
  with nothing left to enchant, goes with it (CR 704.5m) — in the same pass.
  Pass, and only the first half was asserted. The suite tests 704.5m for this
  Aura elsewhere (`cards_morbid_and_ltb.rs::fiend_hunter_returns_a_new_object`),
  but there the creature is *exiled* by another card; the case where Dead
  Weight kills its own host was not covered.
- -2/-2 is a modifier applied on top of whatever the creature has: a 2/2 with
  a +1/+1 counter is a 3/3 and survives as a 1/1, with the Aura still on it.
  Pass, untested until now. This is the same 7a/7c/7d arithmetic the Curse of
  Death's Hold audit pinned from the other side.
- A creature killed this way *dies* — it goes to a graveyard, so death
  triggers fire. It was dealt no damage, so Abattoir Ghoul gains nothing from
  it; that pairing belongs to the Ghoul's tests, which already separate
  "damaged by" from "died".
- The Aura falling off a creature that leaves for any other reason, and
  fizzling when the target goes in response: engine-level, covered by
  `enchantments.rs` and `fizzle.rs` for the Aura shape.

### Test coverage

- -2/-2 kills a 2/2, **and the Aura follows it**:
  `cards_vanilla_and_keywords.rs::dead_weight_kills_small_creature` (extended)
- a +1/+1 counter saves the creature, and the Aura stays on it:
  `cards_vanilla_and_keywords.rs::dead_weight_does_not_kill_through_a_counter` (new)
- no LTB trigger is created for it: `empty_triggers.rs`
- CR 704.5m when the host is exiled instead:
  `cards_morbid_and_ltb.rs::fiend_hunter_returns_a_new_object`

### Mutations run

- `-2/-2` → `-1/-1`: **fails** both tests — the kill and the survival, from
  opposite directions.
- SBA 704.5m stops moving unattached Auras: **fails** the extended kill test
  on its new assertion, and only that one. The creature still dies, so the
  half that was already there passes — which is exactly why the missing
  assertion was worth adding.

Suite: 1543 passing, exit 0, `cargo check --workspace --all-targets` clean.
