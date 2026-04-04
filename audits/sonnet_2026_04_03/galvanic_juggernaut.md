## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This creature attacks each combat if able. This creature doesn't untap during your untap step. Whenever another creature dies, untap this creature.
**Type line**: Artifact Creature — Juggernaut
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "Attacks each combat if able": PASS - Correctly implemented via ContinuousEffect::ForceAttack with EffectScope::OnSelf. Engine respects "if able" by checking summoning sickness, tapping state, and other attack prevention effects.
- "Another creature" restriction: PASS - The trigger system in triggers.rs:419 correctly filters out the dying creature itself with `o.id != dead_id` when collecting watchers for DeathWatch triggers.
- Multiple creatures dying simultaneously: PASS - Each dying creature generates separate DeathWatch triggers for all watchers, so if 3 creatures die at once, Galvanic Juggernaut's ability triggers 3 times.
- Self-death scenario: PASS - If Galvanic Juggernaut dies along with other creatures, it's correctly excluded from watching its own death due to the trigger filter, but still triggers for other creatures' deaths.
- Untap when not on battlefield: PASS - The trigger resolution in triggers.rs:908 verifies the watcher is still on battlefield before calling the ability. The ability itself also checks `obj.zone == Zone::Battlefield` before untapping.
- PreventUntap timing: PASS - ContinuousEffect::PreventUntap correctly prevents normal untapping during untap step, but triggered untapping bypasses this restriction as intended.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic untap when creature dies: `tier15_cards.rs:143` / TESTED
- "Another" restriction (doesn't trigger on self-death): NOT TESTED
- Multiple creature deaths simultaneously: NOT TESTED  
- Forced attack behavior: NOT TESTED
- Prevent untap during untap step: NOT TESTED
- Already untapped creature receiving untap trigger: NOT TESTED
- Galvanic Juggernaut dying while other creatures die: NOT TESTED

Sources:
- [Galvanic Juggernaut — Innistrad (ISD) #222 - Scryfall](https://scryfall.com/card/isd/222/galvanic-juggernaut)
- [Galvanic Juggernaut · Conspiracy (CNS) #200 - Scryfall](https://scryfall.com/card/cns/200/galvanic-juggernaut)
- [Galvanic Juggernaut (Innistrad Remastered)](https://aetherhub.com/Card/INR/Galvanic-Juggernaut/263)