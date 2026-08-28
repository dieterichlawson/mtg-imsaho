## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/219/cobbled-wings?utm_source=api
**Type line**: `Artifact — Equipment` — {2}
**Oracle text**:
```
Equipped creature has flying.
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
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

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Equip cost, bonus, and reattachment: `cards_equipment_costs.rs`, `equipment_autotap.rs`
- Detaching rather than dying: `cards_equipment_costs.rs:an_equipment_that_did_not_resolve_as_a_spell_still_detaches_rather_than_dying`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/219/cobbled-wings?utm_source=api
**Type line**: `Artifact — Equipment` — {2}
**Oracle text**:
```
Equipped creature has flying.
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
```

**Rulings fetched**: none published for this card.

**Status**: PASS

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/219/cobbled-wings
**Oracle text**:
```
Equipped creature has flying.
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
```
**Type line**: `Artifact — Equipment`
**Mana cost**: `{2}`
**Keywords**: Equip
**Rulings**: none published for this card.
**Status**: PASS (test coverage extended)

### Card data
| field | oracle | `cobbled_wings.rs` | |
|---|---|---|---|
| cost | `{2}` | `Generic(2)` | ok |
| types | Artifact | `vec![CardType::Artifact]` | ok |
| subtypes | Equipment | `vec!["Equipment"]` | ok |
| oracle_text | as above, reminder text included | byte-identical | ok |
| static ability | Equipped creature has flying | `GrantKeyword { keyword: Flying, scope: Attached }` | ok |
| equip cost | `Equip {1}` | `equip_for_generic(.., 1)` | ok |

Scryfall's `Keywords: Equip` has no counterpart in `types::Keyword`, and correctly so: equip is an activated
ability keyword (CR 702.6a), modelled here as an `ActivatedAbilityDef`, not a static keyword like flying.

### Code issues
No issues found. The card is three lines of declaration on top of the shared equip helpers.

### Rules check
- **CR 702.6a-b**: equip comes from `helpers::equip_for_generic` (sorcery speed, target creature you control),
  resolution from `resolve_equip`, target legality from `equip_target_is_legal`.
- **CR 301.5c / CR 704.5n**: the Equipment stays on the battlefield when its creature dies and becomes unattached
  — tested, and it is `state.is_equipment` reading the subtype rather than a per-object flag.
- **The grant is `EffectScope::Attached`**, read from the Equipment on the battlefield, so it follows the
  Equipment rather than being written onto the creature.

### Tricky interactions checked
- Equip cannot target an opponent's creature: **pass** (`cobbled_wings_equip_only_your_creatures`).
- Equip may target the creature already wearing it (CR 702.6a): **pass**
  (`curse_and_equip_scope.rs:70`).
- Moving the Wings moves the flying: **pass** (`equipment_can_be_moved_to_different_creature`).
- Creature dies → Wings stay, unattached: **pass** (`equipment_detaches_when_creature_dies`).
- Wings destroyed → creature loses flying: **pass** (new).
- **Granted flying actually restricts blocking (CR 509.1b)**: this was the gap. See below.

### Changes made
- `mtg-engine/tests/cards_equipment_costs.rs` — `cobbled_wings_flying_reaches_the_blocking_rules`.

The card's entire text is "Equipped creature has flying", and flying is not an end in itself: the whole of what
this card does is change who may block. Every existing test stopped at `state.has_keyword(creature, Flying)`
being true, which is equally true of an implementation whose granted keywords never reach the blocking rules.
The new test runs the consequence end to end — a ground creature wearing the Wings cannot be blocked by a
creature with neither flying nor reach, and can be blocked again once the Wings leave the battlefield.

### Mutation checks
1. `can_block_attacker` reading `obj.keywords` instead of `state.has_keyword` for the attacker →
   `cobbled_wings_flying_reaches_the_blocking_rules` FAILED. **Discriminating**, and it is the point of the test:
   nothing else in the suite noticed granted flying failing to reach combat.
2. `GrantKeyword { keyword: Reach }` instead of `Flying` → four tests FAILED including the new one.
   **Discriminating.**
3. Removing the battlefield-zone filter in `walk_effects` → **vacuous**, all 13 passed.
4. Removing `move_object`'s `attached_to = None` on leaving the battlefield → not run separately; mutation 3's
   result made the reason clear, so both were removed together in mutation 4, and *that* FAILED at the
   `has_keyword` assertion.

So the "flying goes with the Wings" half is guarded twice over — `walk_effects` skips a source that is not on
the battlefield, and `move_object` clears `attached_to` so `EffectScope::Attached` matches nothing — and either
guard alone is sufficient. Recorded in the test's own comment, because it makes the obvious single-line
mutations of that assertion vacuous and a future auditor should not have to rediscover it.

### Test coverage
- enters as an unattached Equipment: `cards_equipment_costs.rs:39`
- equip only your own creatures: `cards_equipment_costs.rs:53`
- grants flying: `cards_equipment_costs.rs:89` (`equipping_grants_the_printed_bonus` table)
- granted flying restricts blocking; lost when the Wings leave: `cards_equipment_costs.rs:120` (new)
- equip may re-target the wearer: `curse_and_equip_scope.rs:70`
- detaches when the creature dies: `cards_equipment_costs.rs:227`
- moves between creatures: `cards_equipment_costs.rs:245`
- cast-then-equip full flow: `cards_equipment_costs.rs:266`
- equip cost autotap: `equipment_autotap.rs:157`

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1391 passing.

