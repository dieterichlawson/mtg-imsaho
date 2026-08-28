## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/156/pitchburn-devils?utm_source=api
**Type line**: `Creature — Devil` — {4}{R}, 3/3
**Oracle text**:
```
When this creature dies, it deals 3 damage to any target.
```

**Status**: PASS

### Code issues
No issues found.

'it deals 3 damage to **any target**' — targeted and locked at trigger time; 'any target' includes planeswalkers, and the damage goes through the pipeline so CR 120.3c loyalty removal applies.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_source_independence.rs` (a dies trigger resolving after its source is gone).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/156/pitchburn-devils?utm_source=api
**Type line**: `Creature — Devil` — {4}{R}, 3/3
**Oracle text**:
```
When this creature dies, it deals 3 damage to any target.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "When this creature dies, **it** deals 3 damage to any target" — the source is
  the Devils, from the graveyard, using last known information (CR 608.2g): PASS
- The target is chosen when the death trigger goes on the stack (CR 603.3d), and
  "any target" includes a planeswalker: PASS
- Damage through the pipeline: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The death damage and planeswalker targeting: `damage_helper.rs:an_ability_that_picks_any_target_on_resolution_offers_a_planeswalker`, `cards_morbid_and_ltb.rs`

## Audit — 2026-08-28 17:44

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Pitchburn Devils"`, https://scryfall.com/card/isd/156/pitchburn-devils. Community knowledge via WebSearch (MTG Salvation, "Phage and Pitchburn Devils").
**Oracle text**:
```
When this creature dies, it deals 3 damage to any target.
```
**Type line**: Creature — Devil
**Mana cost**: {4}{R}   **P/T**: 3/3
**Rulings** (1, 2020-06-23): "If your life total is brought to 0 or less at the same time that
Pitchburn Devils is dealt lethal damage, you lose the game before the ability goes on the
stack."
**Status**: PASS (one rule-level test gap closed)

### Code issues
No issues found in `mtg-engine/src/cards/isd/pitchburn_devils.rs`.

`{4}{R}`, `Creature`, `subtypes: ["Devil"]`, 3/3, oracle text verbatim. The trigger is declared
`TriggerKind::SelfDies` with `target_requirement: Some(TargetRequirement::AnyTarget)` — a
*targeted* trigger, so the target is chosen as the ability goes on the stack (CR 603.3d) rather
than picked on resolution. `on_dies` reads `chosen_targets.first()` and hands the damage to
`apply_pending_effect` with `source_id: object_id`, so the source of the damage is the Devils
and `damaged_by` records it.

### Tricky interactions checked
- **"any target" is creature, player, or planeswalker (CR 115.4a)**: PASS, and tested — the
  planeswalker case is `damage_helper.rs`.
- **Target chosen when the trigger goes on the stack, not on resolution**: PASS. The declared
  `target_requirement` is what makes the engine prompt at push time; the community discussion
  turns on exactly this ("at the time your Devils' triggered ability goes on the stack, which
  is when you have to choose a target for it").
- **A creature that died in the same event is not an offerable target**: PASS by construction —
  the `AnyTarget` scan reads the battlefield, and a simultaneously-dead creature is already in a
  graveyard when targets are chosen.
- **The Devils is in the graveyard when its own ability resolves**: PASS (CR 113.7a). The card
  asks nothing about where it is.
- **The target becomes illegal in response**: PASS — the trigger is countered by game rules
  (CR 608.2b) and deals no damage at all.
- **APNAP with two death triggers**: the collector files by controller, active player first
  (CR 603.3b). Not specific to this card; not tested here.
- **The ruling about losing at 0 life simultaneously**: this is SBA ordering (CR 704.3) — the
  player loses before any trigger is put on the stack. **NOT TESTED**, and not modelled in a way
  a test could distinguish: the engine ends the game on the loss, so nothing observable
  separates "the trigger never went on the stack" from "the game ended first".

### Test coverage
- 3 damage to a chosen player: `cards_death_triggers_and_tokens.rs:294 pitchburn_devils_deals_3_on_death`
- 3 damage to a chosen creature: `cards_morbid_and_ltb.rs:1234 pitchburn_devils_choice_with_targets`
- a planeswalker is among the offered targets: `damage_helper.rs:82 an_ability_that_picks_any_target_on_resolution_offers_a_planeswalker`
- the requirement matches the printed "any target": `card_data_invariants.rs:1944` (registry sweep)
- the target is re-checked when the trigger resolves:
  `fizzle.rs:684 a_triggered_abilitys_target_is_rechecked_when_it_resolves` (NEW)
- the 0-life ruling: NOT TESTED (see above)

The new test is the first one in `fizzle.rs` for a *triggered* ability — the file covered spells
and one activated ability, and CR 608.2b applies to all three.

**The first version of it was vacuous and the mutation caught it.** It killed the target in
response, and damage aimed at a creature in a graveyard lands nowhere whether the trigger was
countered or not, so it passed with the re-check deleted. It now grants the target hexproof
instead: still on the battlefield, still somewhere the damage could have gone, no longer legal.
Disabling `triggers.rs`'s `if !any_legal` now fails it.

### Changes made
- `fizzle.rs`: one new test. No code change — the card is correct.
