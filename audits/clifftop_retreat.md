## Audit — 2026-04-01

**Scryfall Oracle text**: Clifftop Retreat enters tapped unless you control a Mountain or a Plains.
{T}: Add {R} or {W}.
**Scryfall type line**: Land
**Status**: PASS

No issues found. Correctly checks for Mountain or Plains subtypes on other permanents. Correctly enters tapped if no matching land. Mana abilities produce {R} or {W}. Good test coverage: tapped/untapped entry, mana production.
