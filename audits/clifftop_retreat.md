## Audit — 2026-04-01

**Scryfall Oracle text**: Clifftop Retreat enters the battlefield tapped unless you control a Mountain or a Plains.
{T}: Add {R} or {W}.
**Scryfall type line**: Land
**Status**: PASS

### Findings

1. **Card data correct**: Name, no mana cost (land), type (Land), no subtypes, no P/T.

2. **ETB tapped logic correct**: Checks if controller has a Mountain or Plains (by subtype), correctly excludes self from the check (line 21: `o.id != object_id`).

3. **Mana abilities correct**: Produces {R} or {W}, requires tap.

4. **Tests**: No dedicated tests found.
