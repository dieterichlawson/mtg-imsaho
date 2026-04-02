# Audit: Rotting Fensnake

## Official Oracle
- **Name:** Rotting Fensnake
- **Cost:** {3}{B}
- **Type:** Creature — Zombie Snake
- **Oracle Text:** (none — vanilla creature)
- **P/T:** 5/1

## Implementation Review
- **Name:** OK
- **Cost:** {3}{B} — OK
- **Type:** Creature, subtypes ["Zombie", "Snake"] — OK
- **Oracle Text:** Empty string — OK
- **P/T:** 5/1 — OK
- **Keywords:** None — OK

## Issues
None found.

## Verdict: PASS

---

# Audit: Rotting Fensnake (2026-04-02)

## Oracle Text (Scryfall)
- **Name:** Rotting Fensnake
- **Mana Cost:** {3}{B}
- **Type:** Creature — Zombie Snake
- **P/T:** 5/1
- **Oracle Text:** (none — vanilla creature)

## Card Data Verification
- **Name:** Correct ("Rotting Fensnake")
- **Cost:** Correct ({3}{B})
- **Type:** Correct (Creature)
- **Subtypes:** Correct (Zombie, Snake)
- **P/T:** Correct (5/1)
- **Oracle Text:** Correct (empty string)
- **Keywords:** Correct (none)

## Behavior Verification
- Vanilla creature with no abilities. No behavior methods implemented beyond `card_data`. Correct.

## Result: PASS
