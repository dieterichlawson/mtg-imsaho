# Audit: Selfless Cathar

## Official Oracle
- **Name:** Selfless Cathar
- **Cost:** {W}
- **Type:** Creature — Human Cleric (Oracle updated from original "Human")
- **Oracle Text:** {1}{W}, Sacrifice Selfless Cathar: Creatures you control get +1/+1 until end of turn.
- **P/T:** 1/1

## Implementation Review
- **Name:** OK
- **Cost:** {W} — OK
- **Type:** Creature, subtypes ["Human", "Cleric"] — OK (matches current Oracle)
- **Oracle Text:** Matches — OK
- **P/T:** 1/1 — OK
- **Activated Ability:** {1}{W}, SacrificeThis, gives +1/+1 until EOT to all your creatures — OK
- **on_activate_ability:** Applies UntilEndOfTurnEffect to all creatures controller controls — OK

## Issues
None found. (Note: doc comment says "Human Soldier" but code correctly has "Human Cleric" matching current Oracle.)

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Text (Scryfall)
- **Name:** Selfless Cathar
- **Mana Cost:** {W}
- **Type:** Creature — Human Cleric
- **P/T:** 1/1
- **Oracle Text:** {1}{W}, Sacrifice this creature: Creatures you control get +1/+1 until end of turn.

### Card Data Audit
- **Name:** Correct ("Selfless Cathar")
- **Cost:** Correct ({W})
- **Types:** Correct (Creature, subtypes Human + Cleric)
- **P/T:** Correct (1/1)
- **Oracle Text String:** Correct (uses "Sacrifice Selfless Cathar" vs oracle "Sacrifice this creature" -- minor wording variant, acceptable)
- **Doc comment:** Says "Human Soldier" but should be "Human Cleric". Cosmetic only; subtypes in code are correct.

### Behavior Audit
- **Activated ability cost:** {1}{W} + sacrifice this. Correct.
- **Effect:** Iterates creatures controller controls on battlefield, pushes +1/+1 UntilEndOfTurnEffect for each. Correct.
- **Only affects creatures at resolution time:** Yes, collects IDs at resolution. Correct per ruling.
- **No tap required:** Correct.
- **Instant speed:** `sorcery_speed_only: false`. Correct.

### Result
**PASS**
