## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/232/sharpened-pitchfork?utm_source=api
**Type line**: `Artifact — Equipment` — {2}
**Oracle text**:
```
Equipped creature has first strike.
As long as equipped creature is a Human, it gets +1/+1.
Equip {1}
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/232/sharpened-pitchfork?utm_source=api
**Type line**: `Artifact — Equipment` — {2}
**Oracle text**:
```
Equipped creature has first strike.
As long as equipped creature is a Human, it gets +1/+1.
Equip {1}
```

**Rulings fetched**: none published for this card.

**Status**: PASS


No rulings are cached for this card and none surfaced.

### Code issues
No issues found. The third of the set's Human-conditional Equipment, and the
inverse shape of Butcher's Cleaver: an unconditional *keyword* plus a
conditional *P/T*. Card data matches exactly — {2}, Artifact — Equipment,
oracle text verbatim, `GrantKeyword { FirstStrike, scope: Attached }` with no
condition, `when(AttachedHasSubtype("Human"), ModifyPT { 1, 1, Attached })`,
Equip {1} at sorcery speed targeting a creature you control through the shared
helpers.

### What was untested
The same gap as Butcher's Cleaver, on the other keyword. Every test of this
card asserts `has_keyword(creature, FirstStrike)`; none of them fights. First
strike is not a property a creature merely has — it splits the combat damage
step (CR 510.4) — and whether the engine splits that step for a keyword granted
by *another permanent* is a different question from whether the keyword reads
back. The card's unconditional half had no behavioural test.

The new test equips a non-Human on purpose, so the +1/+1 is not in play and
cannot be what saves the attacker, and asserts the toughness is unchanged
before combat to make that explicit. It runs both arms: with the Pitchfork the
attacker kills its blocker and survives; without it the same two creatures
trade. The second arm is what makes the first attributable to the Pitchfork
rather than to the setup.

### Tricky interactions checked
- First strike granted regardless of the creature's type: pass
- First strike actually splits the damage step for the equipped creature: pass
- +1/+1 only while the equipped creature is a Human: pass
- The conditional bonus drops on transform while first strike survives it:
  pass — the shared helper asserts `base_kw` is present for both `is_human`
  arms, so the transform case covers it
- Bonus appears when a non-Human gains the Human subtype: pass
- A Human that also becomes a Vampire keeps the bonus: pass
- Both effects follow the Equipment on reattachment, and end when the equipped
  creature dies: pass
- Equip cost and autotap: pass

### Test coverage
- Both effects on a Human / conditional skipped on a non-Human:
  `equipment_human_conditional.rs:131`, `:143`
- Drops on transform (first strike survives) / appears on subtype gain:
  `:160`, `:179`
- Follows reattachment; ends on the creature's death: `:196`, `:222`
- Survives gaining a second subtype: `characteristics_card_sweep.rs:95`
- Equip {1} cost and autotap: `cards_equipment_costs.rs`,
  `equipment_autotap.rs:165`
- **NEW** the granted first strike wins the exchange in real combat:
  `keywords.rs:437`

Mutation-checked: granting a different keyword, and switching the grant's scope
from `Attached` to `OnSelf`, each fail the new test.

