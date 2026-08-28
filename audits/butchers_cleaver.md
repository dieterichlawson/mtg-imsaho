## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/217/butchers-cleaver?utm_source=api
**Type line**: `Artifact — Equipment` — {3}
**Oracle text**:
```
Equipped creature gets +3/+0.
As long as equipped creature is a Human, it has lifelink.
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
- "As long as equipped creature **is a Human**, it has lifelink" — a live
  `ContinuousEffect::when(AttachedHasSubtype("Human"))`, so lifelink drops the
  moment a Human Werewolf transforms rather than being snapshotted at equip
  time: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The unconditional and Human-conditional halves, and following a reattachment: `equipment_human_conditional.rs`
- Detaching rather than dying: `cards_equipment_costs.rs:an_equipment_that_did_not_resolve_as_a_spell_still_detaches_rather_than_dying`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/217/butchers-cleaver?utm_source=api
**Type line**: `Artifact — Equipment` — {3}
**Oracle text**:
```
Equipped creature gets +3/+0.
As long as equipped creature is a Human, it has lifelink.
Equip {3}
```

**Rulings fetched**: none published for this card.

**Status**: PASS


No rulings are cached for this card and none surfaced.

### Code issues
No issues found. The card is structurally identical to Silver-Inlaid Dagger and
correct on every point: {3}, Artifact — Equipment, oracle text verbatim, an
unconditional `ModifyPT { power: 3, toughness: 0, scope: Attached }`, and
lifelink under `ContinuousEffect::when(AttachedHasSubtype("Human"), ..)` so it
tracks the creature's current type rather than a snapshot. Equip {3} is
sorcery-speed, targets a creature you control, and goes through the shared
equip helpers (CR 702.6b, CR 301.5c).

The doc comment's claim that this is "the same pattern Bonds of Faith uses" is
still true — checked, `bonds_of_faith.rs` uses `ContinuousEffect::when` with
`AttachedHasSubtype("Human")`.

### What was untested
Every Butcher's Cleaver test asserted `has_keyword(creature, Lifelink)`. Not
one of them dealt any damage. That the keyword is granted and that the damage
path honours a keyword granted *by another permanent* are two different claims,
and only the second is what the card promises — the same gap I closed on the
Manor Gargoyle flying assertion. The one other test naming the Cleaver
(`cards_morbid_and_ltb.rs:299`) uses it as a generic Equipment prop for Fiend
Hunter and never touches lifelink.

So the card's headline ability had no behavioural test at all, in either
direction.

### Tricky interactions checked
- +3/+0 applies whatever the creature's type: pass
- Lifelink gains life on combat damage while equipped to a Human: pass
- No life gained while equipped to a non-Human: pass
- Lifelink applies to non-combat damage too (CR 702.15a): pass
- Lifelink drops live when the equipped creature transforms:
  pass (`equipment_human_conditional.rs:160`)
- Lifelink appears live when a non-Human gains the Human subtype:
  pass (`:179`)
- A Human that also becomes a Vampire keeps it: pass
  (`characteristics_card_sweep.rs:95`)
- The bonus follows the Equipment on reattachment and ends when the equipped
  creature dies: pass (`equipment_human_conditional.rs:196`, `:222`)
- Equip cost and autotap: pass (`cards_equipment_costs.rs:86`,
  `equipment_autotap.rs:153`)

### Test coverage
- Keyword granted / not granted, and both directions of the conditional:
  `equipment_human_conditional.rs:131`, `:143`, `:160`, `:179`
- Follows reattachment; ends on the creature's death: `:196`, `:222`
- Survives gaining a second subtype: `characteristics_card_sweep.rs:95`
- Equip cost: `cards_equipment_costs.rs:86`, `equipment_autotap.rs:153`
- Unattaches rather than dying with its creature: `cards_morbid_and_ltb.rs:299`
- **NEW** lifelink actually gains life on combat damage, and only for a Human:
  `keywords.rs:301`
- **NEW** lifelink applies to non-combat damage (CR 702.15a), via Skirsdag
  Cultist's activated ability: `keywords.rs:330`

Mutation-checked: pointing the condition at a subtype the creature does not
have, and switching the grant's scope from `Attached` to `OnSelf`, each fail
both new tests.

