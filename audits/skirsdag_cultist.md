## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/163/skirsdag-cultist?utm_source=api
**Type line**: `Creature — Human Shaman` — {2}{R}{R}, 2/2
**Oracle text**:
```
{R}, {T}, Sacrifice a creature: This creature deals 2 damage to any target.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Sacrifice a creature" is a cost, so it is paid on activation and the creature
  is gone while the ability is on the stack: PASS
- "any target" — creature, player or planeswalker: PASS
- The damage is non-combat and emits `NonCombatDamageDealt`: PASS
- The player chooses which creature to sacrifice (`legal_actions` enumerates one
  action per fodder choice): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Damage to a creature and to a player: `cards_sacrifice_and_additional_costs.rs:skirsdag_cultist_deals_2_damage_to_creature`, `:skirsdag_cultist_deals_2_damage_to_player`
- Explicit sacrifice choice: `sacrifice_choice.rs:skirsdag_cultist_explicit_sacrifice`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/163/skirsdag-cultist?utm_source=api
**Type line**: `Creature — Human Shaman` — {2}{R}{R}, 2/2
**Oracle text**:
```
{R}, {T}, Sacrifice a creature: This creature deals 2 damage to any target.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/163/skirsdag-cultist
**Oracle text**: {R}, {T}, Sacrifice a creature: This creature deals 2 damage to any target.
**Type line**: Creature — Human Shaman
**Mana cost**: {2}{R}{R} — **P/T**: 2/2
**Rulings**: none (Scryfall returns no rulings for this card)
**Status**: ISSUE (fixed)

### Card data
Matches the fetched text: `{2}{R}{R}`, `card_types: [Creature]`,
`subtypes: ["Human", "Shaman"]` (both), 2/2, oracle text verbatim in the
current "This creature deals" errata wording, no keywords. The ability is
`{R}` + `requires_tap: true` + `SacrificeCost::SacrificeCreature`, which is the
whole printed cost, with `target_requirement: AnyTarget` for "any target"
(CR 115.4a).

`SacrificeCreature` rather than `SacrificeAnotherCreature` is right: the card
says "a creature", not "another", so the Cultist is its own legal fodder.

### Code issues

1. **Nothing stopped a card dealing *combat* damage**
   (`test_suite_guards.rs`, new guard; `cards_sacrifice_and_additional_costs.rs`,
   assertion added).
   - Oracle text says: `This creature deals 2 damage to any target.`
   - Code says:
     `crate::damage::deal_damage(state, object_id, damage_target, 2, crate::damage::DamageKind::NonCombat, registry);`
     — correct.
   - Verified: changing that to `DamageKind::Combat` produced **zero failures**
     across the whole workspace.
   - This is not cosmetic. CR 510.1 makes combat damage the combat damage
     step's business; an ability's damage is never combat damage, whatever its
     source is. The kind decides whether `CombatDamageDealt` or
     `NonCombatDamageDealt` is emitted, and so whether every "whenever ~ deals
     combat damage" trigger in the set fires — Sturmgeist, Creepy Doll, Curse
     of Stalked Prey, Trepanation Blade.
   - The audit procedure's own anti-pattern list names this exact case
     ("`CombatDamageDealt` for non-combat damage") and nothing enforced it.
   - Added `a_card_never_deals_combat_damage`: no card file may pass
     `DamageKind::Combat`. All six card files that deal damage already pass
     `NonCombat`, so the guard passes today and keeps it that way. Also asserted
     the emitted event directly in `skirsdag_cultist_deals_2_damage_to_player`,
     since this card's audit is where it surfaced.

### Tricky interactions checked
- 2 damage, to a creature and to a player: PASS —
  `skirsdag_cultist_deals_2_damage_to_creature`,
  `skirsdag_cultist_deals_2_damage_to_player`. Dealing 3 fails four tests.
- "any target" reaches a player, not only creatures: PASS — narrowing to
  `TargetRequirement::Creature` fails three tests.
- The Cultist may sacrifice **itself** ("a creature", not "another"): PASS —
  `skirsdag_cultist_cannot_activate_without_creature` shows the ability is
  offered with the Cultist as the only creature; switching to
  `SacrificeAnotherCreature` fails it.
- Every (target, sacrifice) pair is enumerated so the player chooses both:
  PASS — `skirsdag_cultist_enumerates_every_target_sacrifice_pair`.
- The damage is **not** combat damage: PASS — new guard and new assertion.
- Lifelink applies to it (CR 702.15a — lifelink is not restricted to combat
  damage): PASS — `keywords.rs:338`
  (`butchers_cleaver_lifelink_applies_to_noncombat_damage_too`), which equips
  the Cultist with Butcher's Cleaver and checks the ability's 2 damage gains 2
  life.
- Target becomes illegal before resolution: `Target::Illegal => return`, and
  the ability path re-checks targets in `stack::resolve_top_of_stack`
  (CR 608.2b). Generic.
- `{T}` cost legality and summoning sickness: the engine's; the card does not
  re-decide them.
- Self-cleanup: none; this is a permanent.
- The `zone == Battlefield` gate in `activated_abilities` is the
  redundant-but-kept kind recorded in the Mirror-Mad Phantasm entry.

### UI presentation
Ability description: "{R}, {T}, Sacrifice a creature: Deal 2 damage to any
target". The damage pipeline logs the source and amount.

### Test coverage
- 2 damage to a creature / to a player: two tests in
  `cards_sacrifice_and_additional_costs.rs`.
- The damage is non-combat: same file
  (`skirsdag_cultist_deals_2_damage_to_player`) — **assertion added this audit**
  — and structurally by `test_suite_guards.rs`
  (`a_card_never_deals_combat_damage`) — **added this audit**.
- May sacrifice itself: `skirsdag_cultist_cannot_activate_without_creature`.
- Both choices enumerated: `sacrifice_choice.rs`
  (`skirsdag_cultist_enumerates_every_target_sacrifice_pair`).
- Lifelink on non-combat damage: `keywords.rs:338`.
- No rulings exist for this card, so there is no per-ruling row to fill.

### Mutations run
| mutation | result |
| --- | --- |
| `DamageKind::Combat` instead of `NonCombat` | fails the new guard and the new assertion (before: **nothing at all**) |
| deal 3 damage instead of 2 | fails four tests |
| `TargetRequirement::Creature` instead of `AnyTarget` | fails three tests |
| `SacrificeAnotherCreature` instead of `SacrificeCreature` | fails two tests |

Suite after: 1467 passing, exit 0, zero warnings.

