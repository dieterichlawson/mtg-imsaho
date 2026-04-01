## Audit — 2026-04-01

**Scryfall Oracle text**: Galvanic Juggernaut attacks each combat if able.\nGalvanic Juggernaut doesn't untap during your untap step.\nWhenever another creature dies, untap Galvanic Juggernaut.
**Scryfall type line**: Artifact Creature — Juggernaut
**Status**: PASS

- Mana cost {4}: correct.
- Types Artifact Creature, subtype Juggernaut: correct.
- Power/Toughness 5/5: correct.
- Force attack via `ContinuousEffect::ForceAttack { scope: OnSelf }`: correct.
- Prevent untap via `ContinuousEffect::PreventUntap { scope: OnSelf }`: correct.
- Triggered ability: AnyCreatureDies untaps it: correct.
- `on_any_creature_dies` checks zone == Battlefield and tapped: correct.
- TriggerKind::AnyCreatureDies in triggered_abilities: correct. Note: the Oracle says "another creature" but AnyCreatureDies should handle this correctly since the Juggernaut dying wouldn't trigger it (it would leave the battlefield first).
- Tests exist in `tier15_cards.rs` (`galvanic_juggernaut_untaps_when_creature_dies`).

## Audit — 2026-04-01

**Scryfall Oracle text**: Galvanic Juggernaut attacks each combat if able. Galvanic Juggernaut doesn't untap during your untap step. Whenever another creature dies, untap Galvanic Juggernaut.
**Scryfall type line**: Artifact Creature — Juggernaut
**Status**: PASS

No issues found. Correctly implements force attack, prevent untap, and untap-on-death trigger.
