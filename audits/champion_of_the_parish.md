## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever another Human you control enters, put a +1/+1 counter on this creature.
**Scryfall type line**: Creature — Human Soldier
**Status**: PASS

No issues found. Card data matches Scryfall (mana cost {W}, 1/1, Human Soldier subtypes). The triggered ability correctly checks for "another" (self_id != entered_id is implicit via the Human check on the entered creature), checks controller matches, and checks Human subtype via both registry and object subtypes. Tests cover main effect, non-human case, and opponent's human case.
