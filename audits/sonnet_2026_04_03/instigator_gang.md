## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: 
Front face - Instigator Gang {3}{R} Creature — Human Werewolf 2/3: Attacking creatures you control get +1/+0. At the beginning of each upkeep, if no spells were cast last turn, transform this creature.

Back face - Wildblood Pack Creature — Werewolf 5/5: Trample. Attacking creatures you control get +3/+0. At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.

**Type line**: Front: Creature — Human Werewolf, Back: Creature — Werewolf
**Status**: ISSUE

### Code issues

- **Missing upkeep trigger on back face** (mtg-engine/src/cards/isd/instigator_gang.rs:62-73)
  - Oracle text says: `At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.`
  - Code does: Back face has empty `triggered_abilities: vec![]` - no upkeep trigger defined, so Wildblood Pack can never trigger its transformation back to Instigator Gang

- **Oracle text wording mismatch** (mtg-engine/src/cards/isd/instigator_gang.rs:35 and 63)
  - Oracle text says: `transform this creature`
  - Code does: Front face says `transform Instigator Gang`, back face says `transform Wildblood Pack`

### Tricky interactions checked

- **Spell counting mechanism**: PASS - correctly uses `spells_cast_last_turn` map with proper logic for front face (== 0) and back face (.any(|&count| count >= 2))
- **Transform timing on first turn**: PASS - front face transformation correctly checks `!state.is_first_turn`
- **Attack bonus application**: PASS - correctly checks controller match and applies +1/+0 for front face, +3/+0 for back face
- **Self-targeting for attack bonus**: PASS - Instigator Gang correctly buffs itself when attacking
- **Continuous effects cleanup**: PASS - uses `until_end_of_turn_effects` which are properly cleaned up
- **Transform state tracking**: PASS - correctly uses `is_transformed` flag and `apply_transform` helper

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- **Basic transformation from front to back**: `werewolf_cards.rs:453` (instigator_gang_transforms_and_gains_trample)
- **Attack bonus +1/+0 for front face**: `werewolf_cards.rs:469` (instigator_gang_buffs_itself_when_attacking)
- **Attack bonus +3/+0 for back face**: `werewolf_cards.rs:528` (wildblood_pack_buffs_itself_plus_3)
- **Buffs other attackers you control**: `werewolf_cards.rs:489` (instigator_gang_buffs_other_attackers_you_control)
- **Does not buff opponent's creatures**: `werewolf_cards.rs:510` (instigator_gang_does_not_buff_opponent_attackers)
- **Trample keyword on back face**: `werewolf_cards.rs:464` (checked in transform test)
- **Transform back from Wildblood Pack to Instigator Gang**: NOT TESTED - this interaction is broken due to missing back face upkeep trigger
- **First turn no-transform rule**: NOT TESTED for Instigator Gang specifically (tested for other werewolves)
- **Spell count threshold (2+ spells for back-transform)**: NOT TESTED for Instigator Gang specifically