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
