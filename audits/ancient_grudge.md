## Audit — 2026-04-01

**Scryfall Oracle text**: Destroy target artifact.
Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Scryfall type line**: Instant
**Status**: PASS

- Mana cost {1}{R}: correct
- Card type Instant: correct
- Flashback {G}: correct
- Target requirement: PermanentWithFilter(HasCardType(Artifact)): correct
- is_valid_target checks for Artifact card type on battlefield: correct
- on_resolve uses resolve_destroy helper (which uses try_destroy pipeline): correct
- Uses move_spell_after_resolve (via helper): correct
