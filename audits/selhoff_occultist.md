# Audit: Selhoff Occultist

## Official Oracle
- **Name:** Selhoff Occultist
- **Cost:** {2}{U}
- **Type:** Creature — Human Rogue
- **Oracle Text:** Whenever Selhoff Occultist or another creature dies, target player mills a card.
- **P/T:** 2/3

## Implementation Review
- **Name:** OK
- **Cost:** {2}{U} — OK
- **Type:** Creature, subtypes ["Human", "Rogue"] — OK
- **Oracle Text:** Matches — OK
- **P/T:** 2/3 — OK
- **Triggered Abilities:** SelfDies + AnyCreatureDies — OK
- **on_dies:** Presents mill choice (target player) — OK
- **on_any_creature_dies:** Checks self is on battlefield, presents mill choice — OK
- **Mill count:** 1 card — OK

## Issues
None found.

## Verdict: PASS
