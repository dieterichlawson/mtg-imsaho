## Audit — 2026-04-01

**Scryfall Oracle text**: Defender, protection from Zombies
**Scryfall type line**: Creature — Plant
**Status**: PASS

- Mana cost {1}{G}{G}: correct
- 3/4 stats: correct
- Subtype Plant: correct
- Keyword Defender: correct
- Protection from Zombies via ProtectionFromSubtype continuous effect: correct
- Tests exist in card_mechanics.rs covering protection from Zombie damage

## Audit — 2026-04-01

**Scryfall Oracle text**: Defender, protection from Zombies
**Scryfall type line**: Creature — Plant
**Status**: PASS

No issues found. Mana cost {1}{G}{G} correct. P/T 3/4 correct. Subtype [Plant] correct. Keywords [Defender] present. Protection from Zombies via ContinuousEffect::ProtectionFromSubtype correct. Tests exist (card_mechanics.rs).
