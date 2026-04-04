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

## Audit — 2026-04-02

**Oracle source**: Scryfall  
**Card**: Morkrut Banshee  
**Type**: Creature — Spirit | **Cost**: {3}{B}{B} | **P/T**: 4/4  
**Oracle text**: "Morbid — When this creature enters, if a creature died this turn, target creature gets -4/-4 until end of turn."

### Checks
- Name: "Morkrut Banshee" -- PASS
- Cost: {3}{B}{B} -- PASS
- Type: Creature -- PASS
- Subtypes: Spirit -- PASS
- P/T: 4/4 -- PASS
- Morbid condition: Checks `creature_died_this_turn` before triggering -- PASS
- Trigger: ETB with EntersBattlefield trigger kind -- PASS
- Effect: -4/-4 until end of turn via PendingEffect::DebuffUntilEOT -- PASS
- Targeting: Targets any creature (including self per rulings), mandatory -- PASS

**Verdict: PASS**
