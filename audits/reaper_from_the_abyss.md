# Audit: Reaper from the Abyss

## Official Oracle
- **Name:** Reaper from the Abyss
- **Cost:** {3}{B}{B}{B}
- **Type:** Creature — Demon
- **Oracle Text:** Flying\nMorbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature.
- **P/T:** 6/6

## Implementation Review
- **Name:** OK
- **Cost:** {3}{B}{B}{B} — OK
- **Type:** Creature, subtypes ["Demon"] — OK
- **Oracle Text:** Matches — OK
- **P/T:** 6/6 — OK
- **Keywords:** Flying — OK
- **Triggered Abilities:** EndStep trigger — OK
- **on_end_step:** Checks creature_died_this_turn (morbid), filters non-Demon creatures, presents target choice, PendingEffect::DestroyCreature — OK
- **Non-Demon filter:** Checks both registry and instance subtypes — OK

## Issues
None found.

## Verdict: PASS

---

# Audit: Reaper from the Abyss (2026-04-02)

## Oracle Text (Scryfall)
- **Name:** Reaper from the Abyss
- **Mana Cost:** {3}{B}{B}{B}
- **Type:** Creature — Demon
- **P/T:** 6/6
- **Oracle Text:** Flying / Morbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature.

## Card Data Verification
- **Name:** Correct ("Reaper from the Abyss")
- **Cost:** Correct ({3}{B}{B}{B})
- **Type:** Correct (Creature)
- **Subtypes:** Correct (Demon)
- **P/T:** Correct (6/6)
- **Keywords:** Correct (Flying)

## Behavior Verification
- **Trigger:** Correct — `on_end_step` fires at the beginning of each end step.
- **Morbid check:** Correct — checks `state.creature_died_this_turn` and returns early if false.
- **Target filtering:** Correct — filters to battlefield creatures that are not Demons (checks both card_data subtypes and object subtypes). Excludes self.
- **Effect:** Correct — uses `PendingEffect::DestroyCreature` to destroy the selected target.

## Result: PASS
