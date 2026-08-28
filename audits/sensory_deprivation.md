## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/74/sensory-deprivation?utm_source=api
**Type line**: `Enchantment — Aura` — {U}
**Oracle text**:
```
Enchant creature
Enchanted creature gets -3/-0.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- -3/-0 only, so a 2/2 becomes 0/2 and survives — toughness is untouched: PASS
- Negative power deals no combat damage rather than negative damage: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The power reduction: `enchantments.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/74/sensory-deprivation?utm_source=api
**Type line**: `Enchantment — Aura` — {U}
**Oracle text**:
```
Enchant creature
Enchanted creature gets -3/-0.
```

**Rulings fetched**: none published for this card.

**Status**: PASS

### Code issues

No issues found. The card needed no change.

### Card data

`{U}` Enchantment — Aura, `TargetRequirement::Creature` for "Enchant
creature", one continuous effect
`ModifyPT { power: -3, toughness: 0, scope: Attached }` for "Enchanted
creature gets -3/-0". Cost, type line and subtypes pinned pool-wide by
`card_data_invariants.rs`; `Enchant` is one of the keywords the keyword
invariant does not model. `resolve_aura` for attachment, no `is_valid_target`
override, no card-side cleanup.

### Tricky interactions checked

- **Power below zero.** CR 208.3 allows negative power, and
  `effective_power` returns the raw sum without clamping — correct. The
  existing test put the Aura on a **3/3**, where -3 lands exactly on 0 and a
  clamped reading is indistinguishable from an unclamped one. A 2/2 separates
  them, at -1. Untested until now.
- **"-3/-0" cannot kill**, which is the contrast with Dead Weight: toughness
  is untouched, so nothing dies however far the power falls. Untested until
  now, and it shares the same test.
- **A negative-power attacker deals no damage, not negative damage**
  (CR 119.4): the defending player neither loses life nor gains any. Every
  combat site converts with `.max(0)` before `u32::try_from`, so there is no
  wrap either. Untested until now.
- Stacking against a +1/+1 counter or an anthem is the same addition the
  Curse of Death's Hold and Dead Weight audits pinned; the negative result
  above is the part unique to this card.
- The effect ends with the Aura, CR 704.5m, and fizzle: engine-level, covered
  for the Aura shape.

### Test coverage

- -3/-0 on a 3/3, toughness unchanged:
  `cards_vanilla_and_keywords.rs::sensory_deprivation_reduces_power`
- power goes to -1 and the creature lives:
  `cards_vanilla_and_keywords.rs::sensory_deprivation_takes_power_below_zero_without_killing` (new)
- a negative-power attacker deals nothing:
  `cards_vanilla_and_keywords.rs::sensory_deprivation_leaves_an_attacker_dealing_no_damage` (new)

### Mutations run

- `-3/-0` → `-3/-3`: **fails** the two power tests (the creature dies in one
  and the numbers move in the other).
- `effective_power` clamps its result at 0: **fails** the below-zero test and
  the no-damage test, and *passes* the pre-existing one — which is exactly the
  distinction a 3/3 cannot make.
- Combat drops the `.max(0)` before converting power to `u32`: **fails** the
  no-damage test, and only that one.

Suite: 1547 passing, exit 0, `cargo check --workspace --all-targets` clean.
