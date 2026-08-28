## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/227/inquisitors-flail?utm_source=api
**Type line**: `Artifact — Equipment` — {2}
**Oracle text**:
```
If equipped creature would deal combat damage, it deals double that damage instead.
If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.
Equip {2}
```

**Status**: ISSUE

### Code issues
See below.


### Tricky interactions checked
- Equipment enters unattached and stays on the battlefield when what it equipped
  leaves (CR 704.5n), rather than going to the graveyard as an unattached Aura
  would (CR 704.5m): PASS — and this is the one that was wrong. Being an
  Equipment was a per-object `is_equipment` bool that eleven cards set in an
  `on_resolve` override which otherwise only repeated the trait default's "move
  a permanent to the battlefield". An Equipment that reached the battlefield any
  other way left the flag false and was then read as an Aura. Now derived from
  the Equipment subtype (CR 301.5) through the characteristics layer, and the
  eleven dead overrides are gone.
- "Equip only as a sorcery" — `sorcery_speed_only: true`: PASS
- "Attach to target creature **you control**" — `TargetFilter::YouControl` and
  the card's own `is_valid_target`: PASS
- The equip ability is offered on the Equipment, not duplicated onto the
  creature it is attached to: PASS
- The attach happens on resolution, not on activation (CR 602.2a): PASS
- Ruling: "If a creature is equipped with a second Inquisitor's Flail, combat
  damage dealt by and dealt to that creature will be multiplied by four" — each
  Flail is its own `DoubleCombatDamage` continuous effect, so they compose: PASS
- "If **another creature** would deal combat damage to equipped creature" — the
  doubling of incoming damage is from another creature, not from any source: PASS
- Ruling: "you'll divide the original amount and then double the results" — the
  doubling is a replacement on the damage event, after assignment: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Equip cost, bonus, and reattachment: `cards_equipment_costs.rs`, `equipment_autotap.rs`
- Detaching rather than dying: `cards_equipment_costs.rs:an_equipment_that_did_not_resolve_as_a_spell_still_detaches_rather_than_dying`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/227/inquisitors-flail?utm_source=api
**Type line**: `Artifact — Equipment` — {2}
**Oracle text**:
```
If equipped creature would deal combat damage, it deals double that damage instead.
If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.
Equip {2}
```

**Rulings fetched**:
- [2011-09-22] If a creature is equipped with a second Inquisitor’s Flail, combat damage dealt by and dealt to that creature will be multiplied by four. A third Inquisitor’s Flail would multiply the combat damage by eight, and so on.
- [2011-09-22] If you divide the combat damage dealt by the equipped creature, perhaps because the creature has trample or is dealing combat damage to multiple creatures, you’ll divide the original amount and then double the results. For example, if a 5/5 creature with trample is blocked by a 2/2 creature, you can assign 2 damage to the blocker and 3 damage to the defending player. These amounts are then doubled to 4 and 6 damage, respectively. You can’t double the damage to 10 first and then assign 2 to the creature and 8 to the player.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/227/inquisitors-flail
**Oracle text**:
```
If equipped creature would deal combat damage, it deals double that damage instead.
If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.
Equip {2}
```
**Type line**: `Artifact — Equipment`
**Mana cost**: `{2}`
**Keywords**: Equip
**Rulings** (2, both 2011-09-22, https://api.scryfall.com/cards/3014f59d-9012-473a-8bb1-8085c6e91632/rulings):
1. "If a creature is equipped with a second Inquisitor's Flail, combat damage dealt by and dealt to that creature
   will be multiplied by four. A third Inquisitor's Flail would multiply the combat damage by eight, and so on."
2. "If you divide the combat damage dealt by the equipped creature, perhaps because the creature has trample or is
   dealing combat damage to multiple creatures, you'll divide the original amount and then double the results. For
   example, if a 5/5 creature with trample is blocked by a 2/2 creature, you can assign 2 damage to the blocker and
   3 damage to the defending player. These amounts are then doubled to 4 and 6 damage, respectively. You can't
   double the damage to 10 first and then assign 2 to the creature and 8 to the player."

**Status**: ISSUE (fixed) — one doc-comment misquote; both rulings under-tested.

### Card data
| field | oracle | `inquisitors_flail.rs` | |
|---|---|---|---|
| cost | `{2}` | `Generic(2)` | ok |
| types | Artifact | `vec![CardType::Artifact]` | ok |
| subtypes | Equipment | `vec!["Equipment"]` | ok |
| P/T | — | none | ok |
| oracle_text | as above | byte-identical, "another creature" included | ok |
| equip cost | `Equip {2}` | `Generic(2)` | ok |

### Code issues

**1. The doc comment misquoted the card's second clause.** Fixed.
- Oracle text says: `If another creature would deal combat damage to equipped creature`
- Code said: `/// If another source would deal combat damage to equipped creature, it deals double`

"Source" is strictly wider than "creature". Nothing in this set deals *combat* damage from a noncreature
source — only attacking and blocking creatures deal combat damage at all — so the two readings never diverge
here and the implementation is not wrong. But the comment is the place a future reader checks the card's wording,
and it should quote the card. Now it does, with a note saying why the distinction is inert in this pool.

**2. Ruling 1 was half-tested.** The ruling says "combat damage dealt by **and dealt to** that creature will be
multiplied by four." `two_flails_quadruple_damage` checked only damage dealt. Each Flail contributes a clause in
each direction, so the receiving side is a separate claim; it now has its own assertion (a 2-power blocker deals
8 to a doubly-equipped creature, not 4).

**3. Ruling 2 was untested — the ordering gap.** This is the one that mattered. Nothing checked that the doubling
happens *after* combat damage assignment. Added the ruling's own example verbatim, because it hands over numbers
that separate the two orders: 4 to the blocker and 6 to the player if you divide first, 2 and 8 if you double
first. Mutation 1 below shows the old suite passed clean under the forbidden order.

**4. Two "Inquisitor's Flail" sections in one test file.** Consolidated. The earlier one was an older version of
the second's first test; its one distinct claim (the Flail replaces damage, it does not pump power) is now
`the_flail_does_not_change_power_or_toughness`.

### Rules check
- **Both halves are one `ContinuousEffect::DoubleCombatDamage { scope: Attached }`**, read from the source side
  and the target side in `damage.rs:88-89`. That is the general hook, not a card special case.
- **Ruling 1 arithmetic**: `combat_damage_multiplier` uses `state.count_effect`, which walks battlefield objects
  and counts each applying source, then `1u32 << count`. Two Flails give 4, three give 8, matching the ruling's
  "and so on".
- **Ruling 2 ordering**: `combat.rs` assigns from `state.effective_power` and computes lethal from undoubled
  numbers; `damage.rs` applies the multiplier inside `deal_damage`. Division first, doubling after.
- **"Combat damage"**: guarded by `kind == DamageKind::Combat`, so `fight` is untouched (CR 701.13a — fight
  damage is not combat damage).
- **CR 616.1**: two doublings are two replacement effects on one event; order is the affected player's choice and
  irrelevant here because multiplication commutes.
- **Equip**: `sorcery_speed_only: true`, `CreatureWithFilter(YouControl)`, resolution through `resolve_equip`,
  which re-checks target legality (CR 608.2b) and CR 301.5c.

### Changes made
- `mtg-engine/src/cards/isd/inquisitors_flail.rs` — doc comment now quotes the card.
- `mtg-engine/tests/cards_equipment_and_artifacts.rs` — ruling 2's trample example; ruling 1's receiving half;
  the not-a-pump guard; the duplicate section folded away.
- `mtg-engine/src/cards/helpers.rs` — **`equip_ability` / `equip_for_generic`**, the larger change this card led
  to. Eleven Equipment cards each declared the identical `ActivatedAbilityDef` for equip, differing only in cost,
  so eleven cards had to agree by hand on `sorcery_speed_only`, `once_per_turn`, the target requirement, and the
  not-a-creature gate. `resolve_equip` and `equip_target_is_legal` were already shared; this was the missing
  third piece. The helper renders "Equip {2}" from the cost rather than taking it as a string a card could get
  out of step with. Net -88 lines across the eleven.
- `mtg-engine/tests/card_data_invariants.rs` — the refactor tripped
  `no_card_defines_is_valid_target_without_taking_a_target`, which decides an `is_valid_target` is dead by
  scanning the card file for a target requirement; with equip in a helper it called ten live overrides dead.
  Fixed the guard rather than exempting the cards: it now follows one hop into `helpers.rs`, growing the set of
  "targeting helpers" to a fixpoint so `equip_for_generic` counts through its call to `equip_ability`.

### Mutation checks (all discriminating)
1. **Double before assigning** (multiplier moved onto `attacker_power` in `combat.rs`, removed from the source
   side in `damage.rs`) → only `damage_is_divided_first_and_doubled_afterwards` FAILED, `left: 2 right: 4`.
   Every pre-existing test passed. This is the ruling-2 gap, demonstrated.
2. `1u32 << count` → `if count > 0 { 2 } else { 1 }` → `two_flails_quadruple_damage_in_both_directions` FAILED,
   `left: 6 right: 12`.
3. Dropped the target-side multiplier (the second clause) → `the_equipped_creature_takes_double_combat_damage`
   and the new receiving half both FAILED.
4. Modelled the card as `ModifyPT { power: 3, toughness: 0 }` → six tests FAILED including
   `the_flail_does_not_change_power_or_toughness`.
5. `sorcery_speed_only: false` in the new helper → `equip_is_not_offered_at_instant_speed` FAILED.
6. Dropped the not-a-creature gate in the new helper → four tests FAILED across two files
   (`blazing_torch_grants_damage_ability`, `opponents_equipment_grants_no_activatable_ability`, and two more).

### Tricky interactions checked
- Fight damage is not combat damage: **pass** (`fight_damage_is_not_combat_damage_and_is_not_doubled`).
- Flail on the battlefield attached to nothing doubles nothing: **pass** (control row in
  `the_equipped_creature_deals_double_combat_damage`).
- Attacker and blocker both wearing a Flail: ×4, source clause and target clause both applying — covered by the
  arithmetic, since `deal_damage_to_object` multiplies by both sides independently.
- Damage to a player is doubled but takes only the source's multiplier (a player cannot wear Equipment):
  `deal_damage_to_player` multiplies by the source only. **pass**.
- The card does not touch P/T: **pass** (`the_flail_does_not_change_power_or_toughness`).

### Test coverage
- deals double, to a player and to a blocker: `cards_equipment_and_artifacts.rs:483`
- takes double: `cards_equipment_and_artifacts.rs:504`
- ruling 1, both directions: `cards_equipment_and_artifacts.rs:527`
- ruling 2, divide then double: `cards_equipment_and_artifacts.rs:568`
- not a P/T change: `cards_equipment_and_artifacts.rs:590`
- fight damage undoubled: `cards_equipment_and_artifacts.rs:604`
- equip attaches / fizzles / sorcery speed: `cards_equipment_and_artifacts.rs` and `combat_rules.rs`, shared
  across all eleven Equipment now that the ability is declared once.

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1390 passing.

