## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature dies, create a 1/1 white Spirit creature token with flying.
**Type line**: Creature — Human Soldier
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "Dies" means graveyard, not exile: `GameEvent::CreatureDied` is emitted only when a creature is moved to `Zone::Graveyard` (via `destroy()` in destruction.rs and SBA zero-toughness handling in sba.rs). Direct exile via `move_object(Zone::Exile)` does not emit `CreatureDied`, so the trigger correctly does not fire when Doomed Traveler is exiled: pass
- Token color (white) set correctly: `on_dies` passes `vec![Color::White]` to `create_token_with_subtypes`: pass
- Token subtype "Spirit" set correctly: `on_dies` passes `vec!["Spirit".into()]` as the subtypes parameter: pass
- Token keyword Flying set correctly: `on_dies` passes `vec![Keyword::Flying]`: pass
- Controller capture after death: `on_dies` calls `state.get_object(object_id).map(|o| o.controller)` on the graveyard object; `move_object` preserves controller when leaving battlefield, so this correctly returns the pre-death controller: pass
- No battlefield presence check before `on_dies`: `resolve_next_trigger` (triggers.rs:901) does not require the dead object to still be on the battlefield before calling `on_dies`. This is correct — the creature is in the graveyard when the trigger resolves: pass
- Parallel Lives doubling: `create_token_with_subtypes` (state.rs:314) checks for Parallel Lives on the battlefield and creates extra token copies accordingly: pass
- SelfDies trigger dispatch in `collect_triggers`: on `GameEvent::CreatureDied`, the code at triggers.rs:402 checks `registry.get(dead_card_id).is_some()` and creates a `SelfDies` pending trigger. `trigger_description` finds the `SelfDies` entry in Doomed Traveler's `triggered_abilities` and returns "create a 1/1 white Spirit token with flying": pass
- Trigger resolution calls correct hook: `resolve_next_trigger` (triggers.rs:901) matches `PendingTrigger::SelfDies` and calls `behavior.on_dies(state, dead_id, registry)`: pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic dies trigger creates Spirit token: `mtg-engine/tests/tier3_cards.rs:79` (doomed_traveler_creates_spirit_on_death)
- Token P/T (1/1): `mtg-engine/tests/tier3_cards.rs:102-103`
- Token has Flying keyword: `mtg-engine/tests/tier3_cards.rs:104`
- Token is named "Spirit": `mtg-engine/tests/tier3_cards.rs:99`
- Token color (white): NOT TESTED (test does not assert `colors.contains(&Color::White)`)
- Token subtype "Spirit": NOT TESTED (test checks token name but not `subtypes` field)
- Dies vs. exile (trigger does not fire on exile): NOT TESTED
- Parallel Lives doubling of the Spirit token: NOT TESTED (covered separately in Parallel Lives tests)
- Controller-stealing interaction (token goes to controller, not owner): NOT TESTED
