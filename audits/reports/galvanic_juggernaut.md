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

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: This creature attacks each combat if able.\nThis creature doesn't untap during your untap step.\nWhenever another creature dies, untap this creature.
**Type line**: Artifact Creature — Juggernaut
**Status**: PASS

### Code issues
No issues found. The oracle_text field uses the card name instead of "This creature" (modern oracle templating), but this is cosmetic only. The "another creature" condition is correctly enforced by the trigger dispatch in triggers.rs which filters out dead_id == self_id. ForceAttack and PreventUntap continuous effects are correctly scoped to OnSelf.

## Audit — 2026-04-02 21:03

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: This creature attacks each combat if able.
This creature doesn't untap during your untap step.
Whenever another creature dies, untap this creature.
**Type line**: Artifact Creature — Juggernaut
**Status**: PASS

### Code issues
1. **Oracle text cosmetic mismatch**: Implementation uses "Galvanic Juggernaut" where current Scryfall oracle text uses "This creature" (modern template). Implementation: `"Galvanic Juggernaut attacks each combat if able.\nGalvanic Juggernaut doesn't untap during your untap step.\nWhenever another creature dies, untap Galvanic Juggernaut."` vs Scryfall: `"This creature attacks each combat if able.\nThis creature doesn't untap during your untap step.\nWhenever another creature dies, untap this creature."`. Cosmetic only -- no gameplay impact.
2. No behavioral issues found. All three abilities are correctly implemented:
   - `ContinuousEffect::ForceAttack { scope: EffectScope::OnSelf }` -- engine checks this in attack declaration (engine.rs lines 142-151 and 1835-1843).
   - `ContinuousEffect::PreventUntap { scope: EffectScope::OnSelf }` -- engine skips untapping permanents with this effect during untap step (engine.rs line 2913-2923).
   - `TriggerKind::AnyCreatureDies` with `on_any_creature_dies` handler -- correctly untaps when any other creature dies, with zone check ensuring it only fires while on the battlefield.

### Tricky interactions checked (min 3)
1. **"Another creature" -- Juggernaut's own death does not trigger it**: The engine's death-watch collector in triggers.rs filters `o.id != dead_id`, so the Juggernaut will not fire its own death trigger. Additionally, the handler checks `obj.zone == Zone::Battlefield`, which would also prevent it from triggering if somehow reached after leaving play.
2. **Untap trigger fires regardless of which player controlled the dying creature**: The `on_any_creature_dies` handler does not filter on `_dead_controller`. Whether your own creature or an opponent's creature dies, the Juggernaut untaps. This matches oracle text which has no controller restriction.
3. **Juggernaut already untapped when a creature dies**: The handler checks `obj.tapped` before setting `tapped = false`. If already untapped, the handler is a no-op. This is correct -- untapping an untapped permanent is a legal no-op in MTG rules; the optimization to skip it causes no behavioral difference.
4. **ForceAttack with PreventUntap interaction**: The Juggernaut attacks each combat if able, but doesn't untap normally. If it remains tapped (no creatures died), ForceAttack won't force it to attack because tapped creatures are not eligible attackers. The engine correctly handles this -- ForceAttack filters from `eligible` attackers, not all creatures.

### Test coverage
- `galvanic_juggernaut_untaps_when_creature_dies` (tier15_cards.rs): Taps Juggernaut, kills another creature, verifies Juggernaut untaps. PASS.
- No test for ForceAttack / PreventUntap behavior specific to this card (covered by engine-level tests for those effects).
- No test for own-death non-trigger (covered by engine-level filtering).
