## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Equipped creature can't be blocked by Vampires or Zombies.
Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target."
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
**Type line**: Artifact — Equipment
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Cross-controller equipment scenario (ruling): pass - The code correctly finds the torch via attachment relationship and only allows the creature's controller to activate the ability, but sacrifice requires controlling the torch, making the ability unactivatable as intended
- Damage source attribution (ruling): pass - Code explicitly uses `damage_source = torch_id` and logs "The source of the damage is Blazing Torch, not the equipped creature" comment
- Equipment attachment scope: pass - `EffectScope::Attached` correctly checks `source.attached_to == creature_id` 
- Subtype checking for tokens: pass - `matches_filter` checks both `registry.card_data().subtypes` AND `creature.subtypes` to handle tokens
- Blocking restriction logic: pass - Uses `CreatureFilter::Not(Or([HasSubtype("Vampire"), HasSubtype("Zombie")]))` which correctly prevents vampires and zombies from blocking
- Equipment equip targeting: pass - `TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)` correctly restricts to own creatures
- Damage type classification: pass - Uses `NonCombatDamageDealt` correctly for activated ability damage
- Sacrifice mechanics: pass - Uses `crate::destruction::sacrifice(state, torch, registry)` which properly moves torch to graveyard with appropriate events

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic equip ability: `mtg-engine/tests/tier9_cards.rs:501`
- Equipment grants damage ability: `mtg-engine/tests/tier9_cards.rs:394`
- Damage deals to player: `mtg-engine/tests/tier9_cards.rs:412`
- Damage deals to creature: `mtg-engine/tests/tier9_cards.rs:444`
- Damage source attribution: `mtg-engine/tests/tier9_cards.rs:470`
- Equip targets only own creatures: `mtg-engine/tests/tier9_cards.rs:528`
- Cross-controller equipment interaction: NOT TESTED
- Blocking restriction against vampires/zombies: NOT TESTED