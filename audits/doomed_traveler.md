## Audit — 2026-04-01

**Scryfall Oracle text**: When Doomed Traveler dies, create a 1/1 white Spirit creature token with flying.
**Scryfall type line**: Creature — Human Soldier
**Status**: PASS

- Mana cost {W}: correct.
- Type Creature, subtypes Human Soldier: correct.
- Power/Toughness 1/1: correct.
- Dies trigger creates 1/1 white Spirit with flying: correct.
- Token has correct subtypes ["Spirit"], color [White], keywords [Flying]: correct.
- TriggerKind::SelfDies in triggered_abilities: correct.
- `on_dies` hook implemented: correct.
- Tests exist in `tier3_cards.rs` (`doomed_traveler_creates_spirit_on_death`).
