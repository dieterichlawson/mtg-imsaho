# Audit: Stromkirk Patrol

## Oracle (Scryfall)
- **Name:** Stromkirk Patrol
- **Cost:** {4}{B}
- **Type:** Creature -- Vampire Soldier
- **Oracle:** Whenever Stromkirk Patrol deals combat damage to a player, put a +1/+1 counter on it.
- **P/T:** 4/3

## Implementation: `mtg-engine/src/cards/stromkirk_patrol.rs`
- **Name:** Stromkirk Patrol ✅
- **Cost:** {4}{B} ✅
- **Type:** Creature ✅
- **Subtypes:** Vampire, Soldier ✅
- **P/T:** 4/3 ✅
- **Triggered ability:** CombatDamageToPlayer ✅
- **on_combat_damage_to_player:** adds +1/+1 counter, checks zone ✅

## Verdict: PASS -- no issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
**Mana cost**: {4}{B}
**Type line**: Creature — Vampire Soldier
**P/T**: 4/3
**Status**: PASS
### Checks
- **Name**: "Stromkirk Patrol" -- CORRECT
- **Mana cost**: Generic(4) + Black -- CORRECT ({4}{B})
- **Type**: Creature with Vampire, Soldier subtypes -- CORRECT
- **P/T**: 4/3 -- CORRECT
- **Triggered ability**: CombatDamageToPlayer, adds +1/+1 counter -- CORRECT
- **on_combat_damage_to_player**: Checks zone is Battlefield, calls add_counters PlusOnePlusOne 1 -- CORRECT
### Code issues
None. Card data and behavior match oracle text.
