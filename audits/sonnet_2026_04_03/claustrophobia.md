## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
When this Aura enters, tap enchanted creature.
Enchanted creature doesn't untap during its controller's untap step.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- ETB trigger timing: PASS — Uses TriggerKind::EntersBattlefield which correctly creates a triggered ability that uses the stack
- Attachment scope: PASS — Uses EffectScope::Attached which correctly applies PreventUntap to the creature the aura is attached to
- Target validation on resolution: PASS — Uses resolve_aura helper which correctly handles cases where target is no longer valid
- Continuous effect persistence: PASS — PreventUntap effect is checked during each untap step as long as Claustrophobia remains on battlefield
- Tapping already tapped creatures: PASS — Sets tapped=true regardless of current state, which is correct
- Multiple untap step prevention: PASS — Effect says "doesn't untap during its controller's untap step" (not "next untap step"), correctly implemented as ongoing continuous effect

### Test coverage  
For each ruling and tricky interaction, list whether it is tested and where:
- ETB trigger taps creature: `mtg-engine/tests/innistrad_cards.rs:323-339`
- Aura attaches correctly: `mtg-engine/tests/innistrad_cards.rs:323-339`
- Prevents untapping during untap step: `mtg-engine/tests/card_mechanics.rs:490-545`
- Normal creatures still untap: `mtg-engine/tests/card_mechanics.rs:490-545`
- Can target tapped or untapped creatures: NOT TESTED (but ruling states this works)
- Other untap effects still work: NOT TESTED
- Aura goes to graveyard if target becomes invalid: NOT TESTED