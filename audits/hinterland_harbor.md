## Audit — 2026-04-01

**Scryfall Oracle text**: Hinterland Harbor enters the battlefield tapped unless you control a Forest or an Island.
{T}: Add {G} or {U}.
**Scryfall type line**: Land
**Status**: PASS

- Card type Land (no mana cost): correct
- ETB: enters tapped unless controller has a Forest or Island (checks subtypes): correct
- Excludes self from the check (o.id != object_id): correct
- Mana abilities: {T}: Add {G} or {U} (two separate ManaAbilityDefs): correct
- Tests exist in innistrad_simple_cards.rs covering card data

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: Hinterland Harbor enters tapped unless you control a Forest or an Island. {T}: Add {G} or {U}.
**Scryfall type line**: Land
**Status**: PASS

No issues found. ETB check correctly excludes self, checks subtypes for Forest/Island. Mana abilities produce {G} or {U} with tap requirement. Oracle text in code uses older "enters the battlefield tapped" template vs Scryfall's current "enters tapped" but this is cosmetic.
