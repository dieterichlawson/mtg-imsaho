## Audit — 2026-04-01

**Scryfall Oracle text**: Equipped creature can't be blocked by Vampires or Zombies.
Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target."
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
**Scryfall type line**: Artifact — Equipment
**Status**: ISSUE

- Mana cost {1}: correct
- Card types Artifact, subtypes Equipment: correct
- Block restriction for Vampires/Zombies: correct
- Equip {1}, sorcery speed: correct
- Damage ability deals 2, sacrifices torch: correct
- Uses NonCombatDamageDealt event: correct

Issues found:
1. **Damage source is wrong**: The Oracle text says "Blazing Torch deals 2 damage" — the source should be the Blazing Torch (the equipment), not the equipped creature. The implementation uses `object_id` (the creature) as the damage source (line 129-130). This matters for effects that care about damage sources (e.g., protection from artifacts).
2. **on_resolve places artifact on battlefield directly**: Line 44 uses `state.move_object(object_id, Zone::Battlefield)` instead of the standard artifact resolution. This should not call on_resolve at all for a permanent — permanents are placed on the battlefield by the engine. However, if the engine expects this pattern for artifacts, it may be fine.
3. **Equipment entering doesn't use move_spell_after_resolve**: The on_resolve moves to battlefield but doesn't call move_spell_after_resolve. This could be correct if the engine handles permanent resolution differently, but inconsistent with other cards.
