## Audit — 2026-04-01

**Scryfall Oracle text**: (Front — Hanweir Watchkeep) Defender
At the beginning of each upkeep, if no spells were cast last turn, transform Hanweir Watchkeep.
(Back — Bane of Hanweir) Bane of Hanweir attacks each combat if able.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform Bane of Hanweir.
**Scryfall type line**: Creature — Human Warrior Werewolf // Creature — Werewolf
**Status**: PASS

- Mana cost {2}{R}: correct
- Front face 1/5: correct
- Front face subtypes Human Warrior Werewolf: correct
- Front face keyword Defender: correct
- Back face name "Bane of Hanweir": correct
- Back face 5/5: correct
- Back face subtypes Werewolf: correct
- Back face ForceAttack continuous effect: correct (attacks each combat if able)
- Werewolf transform logic: correct
- NOTE: When transforming to back face, the Defender keyword from the front face is no longer active (dynamic_pt overrides stats). The back face correctly does not have Defender.
- Tests exist in werewolf_cards.rs covering transform and defender/force-attack behavior
