## Audit — 2026-04-02 20:33

**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/131/bloodcrazed-neonate)
**Oracle text**: This creature attacks each combat if able.\nWhenever this creature deals combat damage to a player, put a +1/+1 counter on it.
**Type line**: Creature — Vampire
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- ForceAttack respects summoning sickness (creature not in eligible_attackers if sick): pass
- ForceAttack respects Defender keyword (engine.rs:1834 skips Defender): pass
- ForceAttack respects tapped creatures (eligible_attackers filters out tapped): pass
- Combat damage trigger checks zone before adding counter (won't add counter if creature left battlefield): pass
- Counter is +1/+1 (not +2/+2 like Falkenrath Marauders) and only 1 counter per trigger: pass

### Test coverage
- ForceAttack continuous effect present: `tier6_cards.rs:266` (bloodcrazed_neonate_forced_to_attack)
- +1/+1 counter on combat damage to player: NOT TESTED
- Interaction with removal (creature dies before trigger resolves): NOT TESTED
