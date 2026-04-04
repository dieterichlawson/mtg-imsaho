## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)

**Oracle text**: 
**Front Face:**
- Name: Gatstaf Shepherd
- Mana Cost: {1}{G}
- Type Line: Creature — Human Werewolf
- P/T: 2/2
- Oracle Text: At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
- Keywords: Transform, Intimidate

**Back Face:**
- Name: Gatstaf Howler  
- Type Line: Creature — Werewolf
- P/T: 3/3
- Oracle Text: Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.) At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.

**Type line**: Creature — Human Werewolf // Creature — Werewolf

**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Transform conditions**: PASS - Front face transforms when no spells cast last turn (excluding first turn), back face transforms when any single player cast 2+ spells last turn. Logic correctly distinguishes between total count (front) vs single player count (back).
- **Triggered abilities on both faces**: PASS - Front face declares TriggerKind::Upkeep, trigger system correctly generates upkeep triggers for both transformation states by checking front face first.
- **Intimidate keyword timing**: PASS - Intimidate only appears on back face keywords list, correctly gained/lost during transformation.
- **P/T changes during transform**: PASS - Dynamic PT correctly returns (3,3) when transformed, (2,2) when not transformed.
- **First turn exception**: PASS - Transform logic includes `!state.is_first_turn` check to prevent transformation on game's first turn.
- **Simultaneous werewolf transforms**: PASS - All werewolves use identical transformation logic, ensuring they transform together as intended by the rules.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Basic transform to werewolf side**: `werewolf_cards.rs:96-111` (gatstaf_shepherd_transforms_and_gains_intimidate)
- **Transform back to human side**: `werewolf_cards.rs:113-130` (gatstaf_shepherd_loses_intimidate_on_transform_back)  
- **Intimidate keyword presence/absence**: `werewolf_cards.rs:109,128` (verified in both transform tests)
- **P/T changes (2/2 -> 3/3)**: `werewolf_cards.rs:107-108` (verified effective power/toughness)
- **Multiple werewolves transform together**: `werewolf_cards.rs:624-638` (multiple_werewolves_transform_on_same_upkeep)
- **Transform back condition (2+ spells)**: `werewolf_cards.rs:651-657` (multiple_werewolves_transform_back_together)
- **First turn exception**: NOT TESTED for Gatstaf Shepherd specifically (tested for other werewolves)
- **Single vs total spell count distinction**: NOT TESTED for edge cases (e.g., P1 casts 1, P2 casts 1 should NOT transform werewolf back)

Sources:
- [Werewolf upkeep timing - MTG Salvation Forums](https://www.mtgsalvation.com/forums/magic-fundamentals/magic-rulings/magic-rulings-archives/296757-werewolf-upkeep-timing)
- [Werewolf transformation - MTG Salvation Forums](https://www.mtgsalvation.com/forums/magic-fundamentals/magic-rulings/magic-rulings-archives/308160-werewolf-transformation)
- [Transform in MTG: Rules, History, and Best Cards - Draftsim](https://draftsim.com/mtg-transform/)
- [Werewolf - MTG Wiki](https://mtg.fandom.com/wiki/Werewolf)