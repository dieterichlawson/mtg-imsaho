# Audit: Moon Heron

## Official Oracle
- **Name:** Moon Heron
- **Cost:** {3}{U}
- **Type:** Creature — Spirit Bird
- **Oracle:** Flying
- **P/T:** 3/2

## Implementation: `mtg-engine/src/cards/moon_heron.rs`
- **Name:** Moon Heron -- CORRECT
- **Cost:** {3}{U} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** Spirit, Bird -- CORRECT
- **P/T:** 3/2 -- CORRECT
- **Keywords:** Flying -- CORRECT

## Verdict
**PASS** -- No issues found.

## Audit — 2026-04-02

**Oracle source**: Scryfall  
**Card**: Moon Heron  
**Type**: Creature — Spirit Bird | **Cost**: {3}{U} | **P/T**: 3/2  
**Oracle text**: "Flying"

### Checks
- Name: "Moon Heron" -- PASS
- Cost: {3}{U} -- PASS
- Types: Creature -- PASS
- Subtypes: Spirit, Bird -- PASS
- P/T: 3/2 -- PASS
- Keywords: Flying -- PASS
- Behavior: Vanilla creature with flying, no special behavior needed -- PASS

**Verdict: PASS**
