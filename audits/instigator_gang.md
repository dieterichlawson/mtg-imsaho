## Audit — 2026-04-01

**Scryfall Oracle text**: (Front — Instigator Gang) Attacking creatures you control get +1/+0.
At the beginning of each upkeep, if no spells were cast last turn, transform Instigator Gang.
(Back — Wildblood Pack) Trample
Attacking creatures you control get +3/+0.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform Wildblood Pack.
**Scryfall type line**: Creature — Human Werewolf // Creature — Werewolf
**Status**: PASS

- Mana cost {3}{R}: correct
- Front face 2/3: correct
- Front face subtypes Human Werewolf: correct
- Front face: attacking creatures get +1/+0: correct (via on_any_creature_attacks with bonus=1)
- Back face name "Wildblood Pack": correct
- Back face 5/5: correct
- Back face keyword Trample: correct
- Back face: attacking creatures get +3/+0: correct (bonus=3 when transformed)
- Werewolf transform logic: correct
- Tests exist in werewolf_cards.rs
