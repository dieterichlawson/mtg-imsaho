## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This creature attacks each combat if able.
Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
**Type line**: Creature — Vampire
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Forced attack when tapped/summoning sickness: PASS - ForceAttack only applies when creature is able to attack, correctly excludes tapped creatures and creatures with summoning sickness
- Combat damage trigger when creature dies in combat: PASS - Trigger fires correctly, but counter addition fails gracefully when creature no longer on battlefield (correct per MTG rules)  
- Trigger only fires on combat damage to players: PASS - Uses TriggerKind::CombatDamageToPlayer, correctly excludes damage to creatures/planeswalkers and non-combat damage
- Multiple instances of combat damage in same turn: PASS - Each instance correctly triggers separately and adds separate counters
- Forced attack vs cannot attack effects: PASS - Engine correctly handles "can't" trumping "must" - forced attack only applies when able

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Forced attack continuous effect: `mtg-engine/tests/tier6_cards.rs:266` - Tests that ForceAttack effect is present
- Combat damage trigger adding counters: NOT TESTED
- Interaction between forced attack and combat damage trigger: NOT TESTED
- Graceful failure when creature dies during combat: NOT TESTED
- Multiple combat damage instances: NOT TESTED

Sources:
- [Bloodcrazed Neonate MTG - Innistrad #131 (English) | Magic: The Gathering](https://gatherer.wizards.com/ISD/en-us/131/bloodcrazed-neonate)
- [Bloodcrazed Neonate · Innistrad (ISD) #131 - Scryfall](https://scryfall.com/card/isd/131/bloodcrazed-neonate)
- [Bloodcrazed Neonate rulings - MTG Assist](https://www.mtgassist.com/cards/Innistrad/Bloodcrazed-Neonate/rulings)
- [What "attacks if able" really means. - Magic Rules Tips](https://blogs.magicjudges.org/rulestips/2014/07/what-attacks-if-able-really-means/)
- [must attack if able vs can't attack — MTG Q&A](https://tappedout.net/mtg-questions/must-attack-if-able-vs-cant-attack/)