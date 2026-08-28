## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/128/ashmouth-hound?utm_source=api
**Type line**: `Creature — Elemental Dog` — {1}{R}, 2/1
**Oracle text**:
```
Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "blocks **or becomes blocked by** a creature" — both directions, two declared
  triggers: PASS
- The damage is dealt to *that* creature, the one in the blocking relationship,
  not to every blocker: PASS
- Damage through `deal_damage`, so protection and prevention apply: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both directions: `combat_rules.rs`, `cards_complex_creatures.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/128/ashmouth-hound?utm_source=api
**Type line**: `Creature — Elemental Dog` — {1}{R}, 2/1
**Oracle text**:
```
Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature.
```

**Rulings fetched**:
- [2011-09-22] Ashmouth Hound’s ability triggers once for each creature it blocks or becomes blocked by.

**Status**: PASS

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/128/ashmouth-hound
**Oracle text**:
```
Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature.
```
**Type line**: `Creature — Elemental Dog` · **Mana cost**: `{1}{R}` · **P/T**: 2/1
**Ruling** (2011-09-22, https://api.scryfall.com/cards/900ff07e-e5d2-4fe6-ad1a-d0d7e1a272ea/rulings):
"Ashmouth Hound's ability triggers once for each creature it blocks or becomes blocked by."

**Status**: PASS (test coverage extended)

### Card data
| field | oracle | `ashmouth_hound.rs` | |
|---|---|---|---|
| cost | `{1}{R}` | `Generic(1) + Red` | ok |
| types | Creature | `vec![CardType::Creature]` | ok |
| subtypes | Elemental **Dog** | `vec!["Elemental", "Dog"]` | ok |
| P/T | 2/1 | `Some(2)`/`Some(1)` | ok |
| oracle_text | as above | byte-identical | ok |
| triggers | blocks; becomes blocked | `Blocks` + `BecomesBlocked`, both with hooks | ok |

The subtype is worth calling out because it is the exact case the audit procedure warns about: this card was
printed as an "Elemental Hound" and the Hound → Dog errata has since landed. The code already says Dog, and it
matches the type line I fetched today rather than the one on the physical card.

### Code issues
No issues found.

- Two `TriggeredAbilityDef`s for the two halves, each with a matching hook (`on_blocks`, `on_becomes_blocked`),
  so the declaration and the implementation agree.
- The damage is `DamageKind::NonCombat`, which is right: this is a triggered ability's damage, not combat
  damage. It goes through `damage::deal_damage`, so `damaged_by` is tracked and the correct event is emitted.

### Rules check
- **The ruling** — `triggers/collect/combat.rs::blockers_declared` iterates the `(blocker, attacker)`
  assignments, so an attacker blocked by two creatures produces two `BecomesBlocked` emissions with different
  `blocker_id`. Once per creature, as the ruling says.
- **"that creature"** is the specific blocker or blocked attacker carried on the trigger event, not a choice
  and not a target — the ability targets nothing, and `target_requirement: None` says so.
- **Non-combat damage** — the practical consequence is that Inquisitor's Flail ("If equipped creature would
  deal **combat** damage...") does not double it. Now asserted directly.
- **CR 509.1** — a creature can only block one attacker without an enabling effect, so the "blocks" half fires
  once; the "becomes blocked by" half is the one that can multiply.

### Changes made
Nothing in the card. `mtg-engine/tests/cards_spells_and_enchantments.rs` gained three tests. The card had
exactly one, covering the blocking half; `on_becomes_blocked` — a separate hook behind a separate trigger kind
— was never reached by any test.

- `ashmouth_hound_deals_damage_when_it_becomes_blocked`, which also asserts the Hound takes nothing from its
  own ability.
- `ashmouth_hound_triggers_once_per_creature_blocking_it` — the ruling. Two blockers take 1 each, which
  separates "once per creature" from both "one trigger that picks a blocker" and "1 damage split between them".
- `ashmouth_hounds_trigger_damage_is_not_combat_damage` — a Hound wearing Inquisitor's Flail still deals
  exactly 1 from the trigger. This doubles as a cross-check on the Flail audit's reading of "combat damage".

### Mutation checks (all discriminating)
1. `on_becomes_blocked` gutted → all three new tests FAILED, and the pre-existing blocking test did not. That is
   the measure of the gap: half the card had no coverage at all.
2. `BecomesBlocked` emitted once per attacker rather than once per blocker →
   `ashmouth_hound_triggers_once_per_creature_blocking_it` FAILED.
3. `DamageKind::NonCombat` → `DamageKind::Combat` → `ashmouth_hounds_trigger_damage_is_not_combat_damage`
   FAILED.

### Tricky interactions checked
- Blocks a creature → 1 damage to it: **pass** (`cards_spells_and_enchantments.rs:142`).
- Becomes blocked → 1 damage to the blocker: **pass** (new).
- Blocked by two creatures → 1 damage each: **pass** (new).
- The trigger's damage is not doubled by Inquisitor's Flail: **pass** (new).
- 2/1 body dies to its own blocker's combat damage in the normal case — not this ability's business, and the
  trigger's damage resolves before combat damage regardless.

### Test coverage
- deals 1 on blocking: `cards_spells_and_enchantments.rs:142`
- deals 1 on becoming blocked: `cards_spells_and_enchantments.rs:166` (new)
- the ruling, once per blocker: `cards_spells_and_enchantments.rs:190` (new)
- not combat damage: `cards_spells_and_enchantments.rs:215` (new)

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1419 passing.

