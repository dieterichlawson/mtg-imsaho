# Audit: Morkrut Banshee

## Official Oracle
- **Name:** Morkrut Banshee
- **Cost:** {3}{B}{B}
- **Type:** Creature — Spirit
- **Oracle:** Morbid — When Morkrut Banshee enters the battlefield, if a creature died this turn, target creature gets -4/-4 until end of turn.
- **P/T:** 4/4

## Implementation: `mtg-engine/src/cards/morkrut_banshee.rs`
- **Name:** Morkrut Banshee -- CORRECT
- **Cost:** {3}{B}{B} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** Spirit -- CORRECT
- **P/T:** 4/4 -- CORRECT
- **Triggered ability:** EntersBattlefield -- CORRECT
- **on_enter_battlefield:** Checks creature_died_this_turn (morbid), presents target choice for -4/-4 -- CORRECT

## Verdict
**PASS** -- No issues found.
