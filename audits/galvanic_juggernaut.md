## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/222/galvanic-juggernaut?utm_source=api
**Type line**: `Artifact Creature — Juggernaut` — {4}, 5/5
**Oracle text**:
```
This creature attacks each combat if able.
This creature doesn't untap during your untap step.
Whenever another creature dies, untap this creature.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **another** creature dies, untap this creature" — `AnyCreatureDies`
  (self-excluded), and the untap is conditional on it being tapped and on the
  battlefield.
- The other two clauses are static: "attacks each combat if able" and "doesn't
  untap during your untap step".

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/222/galvanic-juggernaut?utm_source=api
**Type line**: `Artifact Creature — Juggernaut` — {4}, 5/5
**Oracle text**:
```
This creature attacks each combat if able.
This creature doesn't untap during your untap step.
Whenever another creature dies, untap this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Three abilities that only make sense together: it must attack, it does not
  untap normally, and another creature dying untaps it — so it attacks again
  only when something died: PASS
- "attacks each combat if able" is a requirement, and CR 508.1d means a tapped
  Juggernaut simply does not attack: PASS
- "**doesn't untap** during your untap step" is a `PreventUntap` continuous
  effect on itself: PASS
- "Whenever **another** creature dies, untap this creature" — `AnyCreatureDies`,
  and untapping does not remove it from combat: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The three abilities together: `combat_requirements.rs`, `cards_complex_creatures.rs`

## Audit — 2026-08-28 17:05

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Galvanic Juggernaut"`, https://scryfall.com/card/isd/222/galvanic-juggernaut
**Oracle text**:
```
This creature attacks each combat if able.
This creature doesn't untap during your untap step.
Whenever another creature dies, untap this creature.
```
**Type line**: Artifact Creature — Juggernaut
**Mana cost**: {4}   **P/T**: 5/5
**Rulings**: none on Scryfall for this card.
**Status**: PASS (test coverage rebuilt; one false comment corrected)

### Code issues
No issues found in `mtg-engine/src/cards/isd/galvanic_juggernaut.rs`.

Card data matches: `{4}`, `card_types: vec![CardType::Artifact, CardType::Creature]`,
`subtypes: vec!["Juggernaut"]`, 5/5. All three lines are declared:
`ContinuousEffect::ForceAttack { scope: OnSelf }`, `ContinuousEffect::PreventUntap { scope:
OnSelf }`, and `TriggeredAbilityDef { kind: TriggerKind::AnyCreatureDies, .. }` matching the
implemented `on_any_creature_dies` hook. `target_requirement: None` is right — "untap this
creature" targets nothing.

The hook untaps through `state.untap` (CR 701.20a, the one place that emits `Untapped`), and
guards on `zone == Battlefield` first.

### Tricky interactions checked
- **"another" creature**: PASS, and it is the engine's, not the card's. The death-watch
  collector filters `o.id != dead_id` (`triggers/collect/zones.rs:120`), so a creature never
  sees its own death as another's.
- **The Juggernaut dies alongside another creature**: PASS. CR 603.10a — the collector includes
  permanents that left in the same event batch, so the ability does trigger; it then resolves
  doing nothing, because the Juggernaut is not on the battlefield (CR 400.7 makes the graveyard
  card a new object). The `zone == Battlefield` guard is what makes that a no-op rather than a
  write to a dead object.
- **Its own two lines against each other**: PASS, and this is the card's whole design. "Doesn't
  untap during your untap step" is a restriction on the untap step alone (CR 302.6); an effect
  that untaps it works. `untaps_normally` is consulted only from the untap step.
- **A creature dies while the Juggernaut is untapped**: PASS — the ability triggers and
  resolves; untapping an untapped permanent does nothing. The card guards the log line only.
- **Tapped and "attacks each combat if able"**: PASS. A tapped creature cannot attack (CR
  508.1a), so the requirement asks nothing of it and it is neither eligible nor in `must_attack`.
- **Summoning sickness vs. the attack requirement**: PASS, via the shared
  `combat::eligible_attackers` — the prompt and the declare-attackers handler ask the same
  question.

### Test coverage
- untaps when another creature dies, *through the trigger system*:
  `cards_complex_creatures.rs:309 galvanic_juggernaut_untaps_when_another_creature_dies` (REWRITTEN)
- does not untap during its controller's untap step:
  `cards_complex_creatures.rs:334 galvanic_juggernaut_does_not_untap_during_the_untap_step` (NEW)
- untapped, it is forced to attack (`ForceAttack` scoped `OnSelf`):
  `combat_rules.rs:259 a_creature_that_forces_itself_to_attack_must_attack` (NEW)
- tapped, it is not: `combat_rules.rs:236 a_tapped_creature_is_not_forced_to_attack`
- "another" excludes the dying creature itself: engine-side, `trigger_dispatch.rs` /
  `triggers/collect/zones.rs` filter — NOT TESTED for this card specifically (the filter is
  one line shared by every death-watcher in the set)
- an effect may untap something under a "doesn't untap" restriction:
  `cards_vanilla_and_keywords.rs:417` (Claustrophobia, same `untaps_normally` mechanism)

All three abilities mutation-checked by deleting each in turn: retyping the trigger kind kills
only the death test, dropping `PreventUntap` kills the untap-step test, dropping `ForceAttack`
kills only the must-attack test.

### Changes made
- `cards_complex_creatures.rs`: the death test called `behavior.on_any_creature_dies` directly,
  so it exercised the hook body and nothing else — not the `TriggerKind` declaration, not the
  death-watch collector, not "another". It now kills a creature and runs `process_triggers`.
- `cards_complex_creatures.rs`: added the untap-step half.
- `combat_rules.rs`: the doc comment on `a_tapped_creature_is_not_forced_to_attack` said
  "Galvanic Juggernaut enters tapped", twice. It does not — there is no such line in the oracle
  text and no enters-tapped effect in `card_data`. Corrected, and the positive `OnSelf`
  force-attack case added beside it.
