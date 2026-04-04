## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: If equipped creature would deal combat damage, it deals double that damage instead.
If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.
Equip {2}
**Type line**: Artifact — Equipment
**Status**: ISSUE

### Code issues
- "Another creature" restriction not enforced (`/Users/dlaw/mtg/mtg-engine/src/combat.rs:453-454`)
  - Oracle text says: `If another creature would deal combat damage to equipped creature`
  - Code does: Applies damage multiplier from both source and target creatures without checking if they are the same creature. If an equipped creature could deal combat damage to itself, damage would be quadrupled (source multiplier × target multiplier) instead of doubled as per oracle text.

### Tricky interactions checked
- Multiple flails stacking multiplicatively: pass (2 flails = ×4, 3 flails = ×8 per ruling)
- Trample damage assignment then doubling: pass (assigns 2+3, then doubles to 4+6 per ruling)
- Combat vs non-combat damage distinction: pass (only affects CombatDamageDealt, not NonCombatDamageDealt)
- Equip targeting restrictions: pass (targets creatures you control only)
- Both directions of damage doubling: pass (source damage dealt and target damage received)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Equipped creature deals double damage to player: `inquisitors_flail.rs:21-41`
- Equipped creature deals double damage to creature: `inquisitors_flail.rs:44-64` 
- Equipped creature takes double damage from attacker: `inquisitors_flail.rs:67-87`
- No doubling without equipment attached: `inquisitors_flail.rs:90-110`
- Multiple flails stack multiplicatively: `inquisitors_flail.rs:113-136`
- "Another creature" restriction: NOT TESTED
- Trample damage assignment then doubling: NOT TESTED
- Non-combat damage unaffected: NOT TESTED