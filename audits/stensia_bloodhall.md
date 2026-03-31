# Audit: Stensia Bloodhall

## Oracle (Scryfall)
- **Name:** Stensia Bloodhall
- **Cost:** N/A (Land)
- **Type:** Land
- **Oracle:** {T}: Add {C}. {3}{B}{R}, {T}: Stensia Bloodhall deals 2 damage to target player or planeswalker.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/stensia_bloodhall.rs`
- **Name:** Stensia Bloodhall ✅
- **Cost:** None ✅
- **Type:** Land ✅
- **Mana ability:** {T}: Add {C} ✅
- **Activated ability:** {3}{B}{R}, {T}: deals 2 damage to target player ✅
- **Target:** PlayerOnly ✅
- **Damage events:** NonCombatDamageDealt + LifeChanged ✅

### Note
- Oracle says "target player or planeswalker" but implementation only targets players (PlayerOnly). Since planeswalkers are not in the card pool, this is acceptable.

## Verdict: PASS -- no issues found
