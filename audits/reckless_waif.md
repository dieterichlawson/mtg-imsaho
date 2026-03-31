# Audit: Reckless Waif // Merciless Predator

## Official Oracle (Front Face)
- **Name:** Reckless Waif
- **Cost:** {R}
- **Type:** Creature — Human Rogue Werewolf
- **Oracle Text:** At the beginning of each upkeep, if no spells were cast last turn, transform Reckless Waif.
- **P/T:** 1/1

## Official Oracle (Back Face)
- **Name:** Merciless Predator
- **Cost:** None
- **Type:** Creature — Werewolf
- **Oracle Text:** At the beginning of each upkeep, if a player cast two or more spells last turn, transform Merciless Predator.
- **P/T:** 3/2

## Implementation Review
- **Front Face Name:** OK
- **Front Face Cost:** {R} — OK
- **Front Face Type:** Creature, subtypes ["Human", "Rogue", "Werewolf"] — OK
- **Front Face Oracle:** Matches — OK
- **Front Face P/T:** 1/1 — OK
- **Back Face Name:** "Merciless Predator" — OK
- **Back Face Type:** Creature, subtypes ["Werewolf"] — OK
- **Back Face Oracle:** Matches — OK
- **Back Face P/T:** 3/2 (via dynamic_pt when transformed) — OK
- **Transform Logic:** werewolf_should_transform checks spells_cast_last_turn, transforms in on_upkeep — OK
- **First turn protection:** !state.is_first_turn prevents transform on first turn — OK

## Issues
None found.

## Verdict: PASS
