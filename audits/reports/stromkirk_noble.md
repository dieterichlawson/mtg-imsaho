# Audit: Stromkirk Noble

## Oracle (Scryfall)
- **Name:** Stromkirk Noble
- **Cost:** {R}
- **Type:** Creature -- Vampire Noble
- **Oracle:** Stromkirk Noble can't be blocked by Humans. Whenever Stromkirk Noble deals combat damage to a player, put a +1/+1 counter on it.
- **P/T:** 1/1

## Implementation: `mtg-engine/src/cards/stromkirk_noble.rs`
- **Name:** Stromkirk Noble ✅
- **Cost:** {R} ✅
- **Type:** Creature ✅
- **Subtypes:** Vampire, Noble ✅
- **P/T:** 1/1 ✅
- **Block restriction:** BlockRestriction with Not(HasSubtype("Human")) on OnSelf ✅
- **Triggered ability:** CombatDamageToPlayer ✅
- **on_combat_damage_to_player:** adds +1/+1 counter, checks zone ✅

## Verdict: PASS -- no issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: This creature can't be blocked by Humans.\nWhenever this creature deals combat damage to a player, put a +1/+1 counter on it.
**Mana cost**: {R}
**Type line**: Creature — Vampire Noble
**P/T**: 1/1
**Status**: PASS
### Checks
- **Name**: "Stromkirk Noble" -- CORRECT
- **Mana cost**: Red -- CORRECT ({R})
- **Type**: Creature with Vampire, Noble subtypes -- CORRECT
- **P/T**: 1/1 -- CORRECT
- **Block restriction**: BlockRestriction with Not(HasSubtype("Human")) on self -- CORRECT (can't be blocked by Humans)
- **Triggered ability**: CombatDamageToPlayer, adds +1/+1 counter -- CORRECT
- **on_combat_damage_to_player**: Checks zone is Battlefield, calls add_counters PlusOnePlusOne 1 -- CORRECT
### Code issues
None. Card data and behavior match oracle text.
