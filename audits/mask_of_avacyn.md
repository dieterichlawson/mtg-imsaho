## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/229/mask-of-avacyn?utm_source=api
**Type line**: `Artifact — Equipment` — {2}
**Oracle text**:
```
Equipped creature gets +1/+2 and has hexproof. (It can't be the target of spells or abilities your opponents control.)
Equip {3}
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
- Ruling: "If Mask of Avacyn somehow becomes attached to a creature an opponent
  controls, that creature can't be the target of spells or abilities you
  control." Hexproof is granted to the attached creature and read relative to
  that creature's controller, not the Mask's: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Equip cost, bonus, and reattachment: `cards_equipment_costs.rs`, `equipment_autotap.rs`
- Detaching rather than dying: `cards_equipment_costs.rs:an_equipment_that_did_not_resolve_as_a_spell_still_detaches_rather_than_dying`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/229/mask-of-avacyn?utm_source=api
**Type line**: `Artifact — Equipment` — {2}
**Oracle text**:
```
Equipped creature gets +1/+2 and has hexproof. (It can't be the target of spells or abilities your opponents control.)
Equip {3}
```

**Rulings fetched**:
- [2011-09-22] If Mask of Avacyn somehow becomes attached to a creature an opponent controls, that creature can't be the target of spells or abilities you control.

**Status**: PASS

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/229/mask-of-avacyn
**Oracle text**:
```
Equipped creature gets +1/+2 and has hexproof. (It can't be the target of spells or abilities your opponents control.)
Equip {3}
```
**Type line**: `Artifact — Equipment` · **Mana cost**: `{2}` · **Keywords**: Equip
**Ruling** (2011-09-22, https://api.scryfall.com/cards/4ff1acce-bed4-452c-8416-06726004f2e8/rulings):
"If Mask of Avacyn somehow becomes attached to a creature an opponent controls, that creature can't be the
target of spells or abilities you control."

**Status**: PASS (test coverage extended)

### Card data
| field | oracle | `mask_of_avacyn.rs` | |
|---|---|---|---|
| cost | `{2}` | `Generic(2)` | ok |
| types / subtypes | Artifact — Equipment | matching | ok |
| oracle_text | as above, reminder text included | byte-identical | ok |
| static | +1/+2 | `ModifyPT { power: 1, toughness: 2, scope: Attached }` | ok |
| static | hexproof | `GrantKeyword { keyword: Hexproof, scope: Attached }` | ok |
| equip cost | `Equip {3}` | `equip_for_generic(.., 3)` | ok |

### Code issues
No issues found. The card is two continuous effects on top of the shared equip helpers.

### Rules check
- **CR 702.11b** — hexproof is "can't be the target of spells or abilities **your opponents** control", where
  "your" is the creature's controller. `engine::can_be_targeted_by` reads the controller off the *target*
  (`state.get_object(target_id).controller`) and compares it to the caster, which is what makes the ruling come
  out right without the card doing anything.
- **`EffectScope::Attached`** resolves through `effect_applies_to`, which tests only `source.attached_to`. No
  controller condition, so the grant follows the Equipment across a control change of the creature — again what
  the ruling requires.
- **CR 702.6b / 301.5c / 704.5n** — equip, resolution and detach-on-death all come from the shared helpers
  audited under Cobbled Wings and Inquisitor's Flail.

### Changes made
Nothing in the card. `mtg-engine/tests/cards_equipment_costs.rs` gained
`mask_of_avacyn_turns_against_you_when_the_creature_changes_hands`.

The Mask had no test under its own heading — only a row in the shared `equipping_grants_the_printed_bonus`
table, which asserts `has_keyword(.., Hexproof)` is true and stops there. Its ruling was untested.

Equip can only ever point at a creature you control, so the ruling's "somehow" has to be a control change
afterwards. `state.change_control` is used directly rather than casting Traitorous Blood, so the test is about
the Mask rather than about that card's untap, trample and haste riders; the ruling itself is agnostic about how
the attachment came about.

The test asserts both directions before and after the change — P1 cannot target it while P0 controls both, P0
cannot target it once P1 has it, and in each case the other player can. Without the "can" halves it would be
passed by an engine that never offered the creature to anyone.

### Mutation checks (all discriminating)
1. `EffectScope::Attached` narrowed to also require `source_controller == the creature's controller` — the
   plausible "equipment only helps its own controller's creatures" mistake →
   `mask_of_avacyn_turns_against_you_when_the_creature_changes_hands` FAILED.
2. `can_be_targeted_by`'s `controller != caster` → `controller == caster` → same test FAILED.
3. Hexproof grant removed from the card → that test and `equipping_grants_the_printed_bonus` both FAILED.

### Tricky interactions checked
- +1/+2 and hexproof granted on equip: **pass** (`cards_equipment_costs.rs:84` table row).
- Hexproof stops an opponent targeting, not its own controller: **pass** (new, both halves).
- The ruling — the Mask's controller loses the ability to target after a control change: **pass** (new).
- Equipment stays attached and keeps granting across a control change: **pass** (new).
- Hexproof reaching the targeting rules at all is covered generally by `hexproof_filter.rs`, which uses
  Lumberknot's *printed* hexproof. Granted and printed hexproof converge on the same `state.has_keyword` call
  in `can_be_targeted_by`, so this card adds no separate discrimination there — recorded rather than claimed.
- Equip cost autotap: `equipment_autotap.rs:173`.

### Test coverage
- +1/+2 and hexproof: `cards_equipment_costs.rs:84`
- the ruling, both directions across a control change: `cards_equipment_costs.rs:160` (new)
- equip cost autotap: `equipment_autotap.rs:173`, `equipment_autotap.rs:450`
- equip only your creatures / sorcery speed / detach on death: shared equipment tests

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1412 passing.

