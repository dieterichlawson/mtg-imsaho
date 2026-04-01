## Audit — 2026-04-01

**Scryfall Oracle text**: First strike, vigilance\nProtection from Vampires, from Werewolves, and from Zombies
**Scryfall type line**: Creature — Human Soldier
**Status**: PASS

- Mana cost {W}{W}: correct.
- Type Creature, subtypes Human Soldier: correct.
- Power/Toughness 2/2: correct.
- Keywords: FirstStrike, Vigilance: correct.
- Protection from Vampires, Werewolves, Zombies via `ProtectionFromSubtype`: correct.
- Tests exist in `tier12_cards.rs` (`elite_inquisitor_keywords`, `elite_inquisitor_protection_prevents_damage`, `elite_inquisitor_cant_be_blocked_by_zombies`).
