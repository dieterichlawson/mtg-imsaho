# Audit: Galvanic Juggernaut

## Reference (Scryfall)
- **Name:** Galvanic Juggernaut
- **Cost:** {4}
- **Type:** Artifact Creature -- Juggernaut
- **Oracle:** Galvanic Juggernaut attacks each combat if able. Galvanic Juggernaut doesn't untap during your untap step. Whenever another creature dies, untap Galvanic Juggernaut.
- **P/T:** 5/5

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({4})
- Type: CORRECT (Artifact, Creature)
- Subtypes: CORRECT (Juggernaut)
- Oracle text: CORRECT
- P/T: CORRECT (5/5)
- Attacks each combat if able: CORRECT (ForceAttack, scope: OnSelf)
- Doesn't untap during untap step: CORRECT (PreventUntap, scope: OnSelf)
- Whenever another creature dies, untap: CORRECT (TriggerKind::AnyCreatureDies, on_any_creature_dies sets tapped=false)

## Issues
None found.

## Audit 2 (2026-04-02)

### Oracle Text (Scryfall)
Galvanic Juggernaut attacks each combat if able.
Galvanic Juggernaut doesn't untap during your untap step.
Whenever another creature dies, untap Galvanic Juggernaut.

### Implementation Review
- **Name:** CORRECT ("Galvanic Juggernaut")
- **Cost:** CORRECT ({4} -- `Generic(4)`)
- **Types:** CORRECT (Artifact, Creature)
- **Subtypes:** CORRECT ("Juggernaut")
- **P/T:** CORRECT (5/5)
- **Oracle text string:** CORRECT (matches verbatim)
- **Attacks each combat if able:** CORRECT (`ContinuousEffect::ForceAttack { scope: EffectScope::OnSelf }`)
- **Doesn't untap during untap step:** CORRECT (`ContinuousEffect::PreventUntap { scope: EffectScope::OnSelf }`)
- **Triggered ability -- "Whenever another creature dies, untap":**
  - Trigger kind: CORRECT (`TriggerKind::AnyCreatureDies`)
  - "Another creature" filtering: CORRECT -- engine's death-watch collector in `triggers.rs` line 402 filters `o.id != dead_id`, ensuring the dying creature itself is not a watcher. If Galvanic Juggernaut dies, it will not trigger itself.
  - Effect: CORRECT -- `on_any_creature_dies` checks `zone == Battlefield && tapped`, then sets `tapped = false`. The tapped check is an optimization (untapping an untapped permanent is a no-op in MTG rules) and does not affect correctness.

### Test Coverage
- `galvanic_juggernaut_untaps_when_creature_dies` in `tier15_cards.rs`: Tests basic case -- tap Juggernaut, another creature dies, Juggernaut untaps. PASSES.
- Missing test: Juggernaut's own death should not trigger it. Handled correctly by engine-level filtering, but no explicit test exists.

### Verdict
PASS -- no issues found. Implementation matches oracle text.
