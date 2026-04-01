## Audit — 2026-04-01

**Scryfall Oracle text**: Flying
As long as Dearly Departed is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.
**Scryfall type line**: Creature — Spirit
**Status**: PASS

No issues found. Mana cost {4}{W}{W}, 5/5, Spirit subtype, Flying keyword all correct. The graveyard ability correctly checks self is in Zone::Graveyard and entered creature is Human. Checks both registry and object subtypes for Human. Test coverage exists.
