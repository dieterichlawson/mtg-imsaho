# Audit: Caravan Vigil

## Scryfall Reference
- **Name:** Caravan Vigil
- **Cost:** {G}
- **Type:** Sorcery
- **Oracle:** Search your library for a basic land card, reveal it, put it into your hand, then shuffle. Morbid -- You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.
- **P/T:** N/A
- **Keywords:** Morbid

## Implementation: `caravan_vigil.rs`
- **Name:** Caravan Vigil -- CORRECT
- **Cost:** {G} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Subtypes:** none -- CORRECT
- **P/T:** N/A -- CORRECT
- **Keywords:** [] -- CORRECT (Morbid is an ability word, not a keyword mechanic)
- **Behavior:** Searches for basic land, puts in hand; Morbid puts on battlefield -- CORRECT
- **Oracle text note:** Implementation oracle text says "then shuffle your library" vs Scryfall "then shuffle" (minor wording modernization, not functional)

## Issues
None
