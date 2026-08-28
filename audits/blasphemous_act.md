## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/130/blasphemous-act?utm_source=api
**Type line**: `Sorcery` — {8}{R}
**Oracle text**:
```
This spell costs {1} less to cast for each creature on the battlefield.
Blasphemous Act deals 13 damage to each creature.
```
**Status**: PASS

### Code issues
No issues found.

- 13 damage to each creature through `apply_pending_effect`, so the damage
  pipeline applies protection, prevention and replacements (Unbreathing Horde's
  among them).
- The creature list is snapshotted before any damage lands; nothing dies
  mid-resolution anyway, since state-based actions do not run until it finishes
  (CR 704.3).
- The cost reduction is declared in card data and handled by the cost pipeline.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/130/blasphemous-act?utm_source=api
**Type line**: `Sorcery` — {8}{R}
**Oracle text**:
```
This spell costs {1} less to cast for each creature on the battlefield.
Blasphemous Act deals 13 damage to each creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "Blasphemous Act's ability **can't reduce the total cost to cast the
  spell below {R}**." The reduction is `creature_count.min(8)`, so the {8}
  generic can go to zero but the coloured pip always remains: PASS
- "for each creature on the battlefield" — **all** creatures, not just yours:
  PASS
- Ruling: "The total cost is locked in before you pay that cost" — the count is
  taken during casting, so sacrificing a creature for mana afterwards does not
  raise the price back: PASS
- "deals 13 damage to **each** creature" — no targeting, so hexproof does not
  save anything, and the damage goes through `deal_damage` so protection,
  prevention and Unbreathing Horde's replacement all apply: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The cost reduction and the sweep: `cards_burn_and_damage.rs`, `inline_damage.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/130/blasphemous-act?utm_source=api
**Type line**: `Sorcery` — {8}{R}
**Oracle text**:
```
This spell costs {1} less to cast for each creature on the battlefield.
Blasphemous Act deals 13 damage to each creature.
```

**Rulings fetched**:
- [2020-11-10] The total cost to cast Blasphemous Act is locked in before you pay that cost. For example, if there are three creatures on the battlefield, including one you can sacrifice to add {C}, the total cost of Blasphemous Act is {5}{R}. Then you can sacrifice the creature when you activate mana abilities just before paying the cost.
- [2020-11-10] Blasphemous Act's ability can't reduce the total cost to cast the spell below {R}.
- [2020-11-10] Although players may respond to Blasphemous Act once it's been cast, once it's announced, they can't respond before the cost is calculated and paid.
- [2020-11-10] To determine the total cost of a spell, start with the mana cost or alternative cost you're paying, add any cost increases, then apply any cost reductions (such as that of Blasphemous Act). The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.

**Status**: PASS

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**:
```
This spell costs {1} less to cast for each creature on the battlefield.
Blasphemous Act deals 13 damage to each creature.
```
**Type line**: `Sorcery` — {8}{R}
**Status**: PASS (implementation); test gaps found and closed

### Rulings (all 2020-11-10)
1. "The total cost to cast Blasphemous Act is locked in before you pay that cost. For example, if there are three creatures on the battlefield, including one you can sacrifice to add {C}, the total cost of Blasphemous Act is {5}{R}. Then you can sacrifice the creature when you activate mana abilities just before paying the cost."
2. "Blasphemous Act's ability can't reduce the total cost to cast the spell below {R}."
3. "Although players may respond to Blasphemous Act once it's been cast, once it's announced, they can't respond before the cost is calculated and paid."
4. "To determine the total cost of a spell, start with the mana cost or alternative cost you're paying, add any cost increases, then apply any cost reductions (such as that of Blasphemous Act). The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was."

### Code issues

No issues in the card. `{8}{R}` as `Generic(8)` + `Colored(Red)`, `CardType::Sorcery`, no supertypes or subtypes, oracle text verbatim, no triggered or activated abilities to declare. The cost reduction is a `CardBehavior::modified_cost` — the general hook, consumed by `engine::costs::cost_to_cast` in CR 601.2f order — not engine special-casing. The damage goes through `PendingEffect::DealDamage` and `apply_pending_effect`, so it reaches `damage.rs` like all other damage and picks up protection, prevention and replacement per creature.

### Tricky interactions checked

- Ruling 2, floor at {R}: PASS. `reduction = creature_count.min(8)` then `8u32.saturating_sub(reduction)` leaves the `Colored(Red)` symbol untouched, and `costs::reduce` independently refuses to eat a coloured requirement.
- "each creature on the battlefield", both players': PASS. `all_objects_in_zone(Zone::Battlefield)` with no controller filter, in both the cost count and the damage loop.
- "each **creature**", not each permanent: PASS in both halves — but untested until this audit (see below).
- Ruling 4, mana value vs total cost: PASS. Both readers of a mana value in the pool (`mindshrieker.rs:66`, `heretics_punishment.rs:109`) go through `face_data(...).cost.mana_value()`, the printed cost. Nothing routes a mana value through `cost_to_cast`. Untested until this audit.
- Ruling 1 / ruling 3, cost locked in before payment: PASS by construction. Casting is one `Action::CastSpell`; the cost is determined and paid inside a single engine action, so nothing can change the board between the two. Sacrificing a creature for mana cannot raise the cost because there is no window in which to do it.
- Cost reductions composing with `ReduceCost` continuous effects: N/A in practice — `costs::spell_matches` only matches creature spells, and Blasphemous Act is a sorcery. The composition order (`modified_cost` supplies the base, `total_reduction` comes off it) is correct either way for generic reductions.
- Cost *increases*: none exist in this pool; `costs.rs` documents where a `ContinuousEffect::IncreaseCost` would be applied (between base and reductions, per ruling 4's ordering).
- Damage order: deterministic. `all_objects_in_zone` sorts by `ObjectId`, so the per-creature damage events are emitted in a stable order.
- Simultaneity: the 13 damage is dealt one creature at a time rather than as one simultaneous event. Not observable in this pool — state-based actions do not run mid-resolution, so nothing dies between the calls, and no card here counts "damage dealt at once". Restructuring `damage.rs` around a batch API for one card would be the tail wagging the dog; recorded rather than done.
- Self-inclusion: the Act is a sorcery on the stack, not a creature, so it neither reduces its own cost nor damages itself.

### Test gaps found (all three passed the whole workspace before this audit)

- `on_resolve` dropping `state.is_creature` — dealing 13 to every permanent — passed everything. No Blasphemous Act test had a non-creature permanent on the battlefield.
- `modified_cost` dropping `state.is_creature` — every permanent reducing the cost — passed everything, for the same reason.
- Mindshrieker's mana value routed through `effective_spell_cost` instead of the printed cost — ruling 4 — passed everything.

Fixed by putting a land, an Equipment and an enchantment on the board in the two card tests, and by adding `spell_costs.rs::a_cost_reduction_does_not_change_a_cards_mana_value`. `blasphemous_act_cost_reduction` was also rewritten to ask through `cost_to_cast` rather than calling the `modified_cost` hook directly, and to assert ruling 2 as a surviving `{R}` requirement rather than as a mana value of 1.

### Test coverage

- 13 damage to each creature, both players': `cards_spells_and_enchantments.rs:557` `blasphemous_act_deals_13_damage_to_all_creatures`
- "each **creature**" — a land and an Equipment take none: same test, added this audit
- Per-creature protection/replacement rather than once for the spell: `inline_damage.rs:186` `blasphemous_act_consults_each_creature_separately`
- Cost reduction through the real cost pipeline: `cards_spells_and_enchantments.rs:587` `blasphemous_act_cost_reduction`
- "for each **creature**" — non-creature permanents do not reduce: same test, added this audit
- Ruling 2, floor at {R}: same test (generic 0, `{R}` survives)
- Creatures anyone controls count: same test, and `blasphemous_act_castable_with_cost_reduction` (4 + 4 split)
- End to end, castable for {R}: `cards_spells_and_enchantments.rs:620` `blasphemous_act_castable_with_cost_reduction`
- Ruling 4, mana value unchanged by the reduction: `spell_costs.rs:267` `a_cost_reduction_does_not_change_a_cards_mana_value`, added this audit
- Rulings 1 and 3, cost locked in before payment: NOT TESTED — not observable (single-action casting, see above)

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 `amount: 13` -> `12` | `blasphemous_act_deals_13_damage_to_all_creatures`, `blasphemous_act_consults_each_creature_separately` FAILED | (unchanged) |
| M2 `on_resolve` drops `is_creature` | passed whole workspace | `blasphemous_act_deals_13_damage_to_all_creatures` FAILED |
| M3 `modified_cost` drops `is_creature` | passed whole workspace | `blasphemous_act_cost_reduction` FAILED |
| M4 `modified_cost` counts only P0's creatures | n/a | `blasphemous_act_cost_reduction`, `blasphemous_act_castable_with_cost_reduction` FAILED |
| M5 Mindshrieker's mana value via `effective_spell_cost` | passed whole workspace | `a_cost_reduction_does_not_change_a_cards_mana_value` FAILED |

Sources restored from `/tmp/ba.bak` and `/tmp/ms.bak` after each.

Not mutated, because the mutation is provably vacuous: `creature_count.min(8)` is redundant with the `8u32.saturating_sub` below it, and `if generic == 8 { return None }` is equivalent to returning the unchanged cost. Both are left as written — they document intent.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1469 passing (was 1468). `cargo check --workspace --all-targets` clean, zero warnings.
