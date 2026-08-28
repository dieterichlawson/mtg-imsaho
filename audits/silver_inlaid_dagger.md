## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/233/silver-inlaid-dagger?utm_source=api
**Type line**: `Artifact — Equipment` — {1}
**Oracle text**:
```
Equipped creature gets +2/+0.
As long as equipped creature is a Human, it gets an additional +1/+0.
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
- "As long as equipped creature is a Human, it gets an **additional** +1/+0" —
  a second, conditional `ModifyPT` on top of the unconditional +2/+0, evaluated
  live: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Equip cost, bonus, and reattachment: `cards_equipment_costs.rs`, `equipment_autotap.rs`
- Detaching rather than dying: `cards_equipment_costs.rs:an_equipment_that_did_not_resolve_as_a_spell_still_detaches_rather_than_dying`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/233/silver-inlaid-dagger?utm_source=api
**Type line**: `Artifact — Equipment` — {1}
**Oracle text**:
```
Equipped creature gets +2/+0.
As long as equipped creature is a Human, it gets an additional +1/+0.
Equip {2}
```

**Rulings fetched**: none published for this card.

**Status**: PASS


No rulings are cached for this card and none surfaced.

### Code issues
No issues found in the card. This is one of the better-implemented cards in the
set.

- Card data matches exactly: {1}, Artifact — Equipment, oracle text verbatim.
- The Human bonus is modelled as
  `ContinuousEffect::when(EffectCondition::AttachedHasSubtype("Human"),
  ModifyPT { power: 1, .. })`, i.e. a live condition rather than a snapshot
  taken at equip time — which is right, and the file says why: a Human
  Werewolf that flips loses the +1/+0 the moment it does.
- `AttachedHasSubtype` resolves through `has_subtype`, which unions the
  runtime `obj.subtypes` with the active face, so a Human that Olivia
  Voldaren also made a Vampire keeps the bonus (subtypes are additive) and a
  transformed DFC reads its back face (CR 712.8d).
- Equip {2}, sorcery speed, `CreatureWithFilter(YouControl)`, and both
  `is_valid_target` and the resolution go through the shared equip helpers
  (CR 702.6b, and CR 301.5c via the `!is_creature` gate).

### Stale documentation fixed
`equipment_human_conditional.rs`'s module note told the reader the fix was to
express the bonus as `ContinuousEffect::ConditionalModifyPT` /
`ConditionalKeyword`. Neither variant exists: `types.rs:437` records that four
parallel `Conditional*` variants were folded into the single
`ContinuousEffect::when` wrapper. A reader following that note would go looking
for names that are not there, and the note is the first thing anyone touching
these three cards reads. Rewritten to name the current shape, with a line
saying what it used to be so the history is not lost.

### Tricky interactions checked
- +2/+0 on any creature, +3/+0 total on a Human: pass
- Bonus drops live when a Human Werewolf transforms: pass
- Bonus appears live when a non-Human gains the Human subtype: pass
- A Human that gains a second subtype stays a Human: pass
- The bonus follows the Equipment on reattachment: pass
- The equipped creature dies: the Equipment stays on the battlefield and
  detaches (CR 704.5n), buffs nothing, and recomputes the conditional against
  whatever it is equipped to next: pass
- Equip is sorcery-speed and targets a creature you control: pass
- An unattached Equipment's conditional resolves to nothing rather than
  panicking: pass, `attached_to` is an `Option` all the way through

### Test coverage
- Bonus on a Human / skipped on a non-Human:
  `equipment_human_conditional.rs:131`, `:143`
- Drops on transform / appears on subtype gain: `:160`, `:179`
- Follows reattachment: `:196`
- Survives gaining an extra subtype: `characteristics_card_sweep.rs:95`
- Equip cost and autotap: `cards_equipment_costs.rs:87`,
  `equipment_autotap.rs:91`
- **NEW** bonuses end when the equipped creature dies, and the conditional is
  recomputed for the next creature: `equipment_human_conditional.rs:222`

### On the new test, and one mutation that proved nothing
The general detach test (`cards_equipment_costs.rs:191`) checks the zones after
the equipped creature dies but never that the *bonus* stopped, and the
reattachment test moves a live Equipment, which is a different path. The new
case closes that, table-driven across all three Human-conditional Equipment.

It is mutation-checked by making the Equipment follow its creature to the
graveyard, which fails it. A second mutation — having the condition fall back
to `card_state["last_attached_to"]` when nothing is attached — changed nothing,
and the reason is worth recording rather than dressing up: that key is only
written when the *Equipment itself* leaves the battlefield, and on this path it
stays. So the mutation was vacuous, not the test insensitive. The stale-pointer
risk it was meant to probe is already covered by the reattachment test.

A third mutation, stopping the detach outright, hung the state-based-action
loop rather than failing — the SBA reports it took an action every pass and
never reaches a fixed point. Not pursued as a finding: it is an artefact of
breaking an invariant SBA relies on, not a reachable state.

