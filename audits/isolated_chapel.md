## Audit — 2026-04-01

**Scryfall Oracle text**: Isolated Chapel enters the battlefield tapped unless you control a Plains or a Swamp.
{T}: Add {W} or {B}.
**Scryfall type line**: Land
**Status**: PASS

- Card type Land (no mana cost): correct
- ETB: enters tapped unless controller has a Plains or Swamp (checks subtypes): correct
- Excludes self from the check (o.id != object_id): correct
- Mana abilities: {T}: Add {W} or {B} (two separate ManaAbilityDefs): correct
- Tests exist in innistrad_simple_cards.rs covering card data

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: Isolated Chapel enters tapped unless you control a Plains or a Swamp. {T}: Add {W} or {B}.
**Scryfall type line**: Land
**Status**: PASS

No issues found. Implementation mirrors Hinterland Harbor pattern correctly.
