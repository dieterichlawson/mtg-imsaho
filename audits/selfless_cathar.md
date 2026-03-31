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
