# Audit: Blazing Torch

## Oracle (Scryfall)
- **Name:** Blazing Torch
- **Cost:** {1}
- **Type:** Artifact — Equipment
- **Oracle:** Equipped creature can't be blocked by Vampires or Zombies. Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target." Equip {1}
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/blazing_torch.rs`
- **Name:** Blazing Torch ✅
- **Cost:** {1} ✅
- **Type:** Artifact ✅
- **Subtypes:** Equipment ✅
- **Block restriction:** Vampires and Zombies cannot block ✅
- **Equip cost:** {1}, sorcery speed ✅
- **Damage ability:** {T}, Sacrifice, deal 2 to any target ✅
- **NonCombatDamageDealt events:** emitted ✅
- **Sacrifice:** calls `crate::destruction::sacrifice` ✅

## BUG: Missing `damaged_by` tracking for creature damage
In `on_activate_ability` ability_index 1, when dealing damage to a creature target (line 126-128), the code does:
```rust
obj.damage_marked += 2;
```
But does NOT do:
```rust
obj.damaged_by.push(...);
```
Every other card that deals non-combat damage to creatures (Balefire Dragon, Blasphemous Act, Daybreak Ranger, Olivia Voldaren, Ashmouth Hound, helpers.rs) pushes to `damaged_by`. This omission means damage source tracking is broken for Blazing Torch — affecting interactions like Abattoir Ghoul's life gain on lethal damage.

## BUG: Damage source should be the Torch, not the creature
The oracle text says "Blazing Torch deals 2 damage" — the damage source should be the Torch object, not the equipped creature. The comment on line 122 acknowledges this ("Source is the torch (flavor), but we use creature ID") but uses `object_id` (the creature) as source. Since the torch is sacrificed before damage, using the creature as source is a pragmatic choice, but the `NonCombatDamageDealt` event's source is technically wrong per oracle text. This matters for effects that care about what dealt the damage (e.g., protection from creatures would incorrectly prevent this damage if the source is the creature, but shouldn't since the Torch is an artifact).

## Verdict: FAIL — 2 issues found
1. **Missing `damaged_by.push()`** when dealing damage to creatures (real bug)
2. **Wrong damage source** — uses creature ID instead of torch ID (acknowledged in comment, but technically incorrect)
