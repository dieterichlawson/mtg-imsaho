## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flash (You may cast this spell any time you could cast an instant.)
When this creature enters, untap all creatures you control.
**Type line**: Creature — Human Scout
**Status**: ISSUE

### Code issues

- ETB trigger resolution skipped if VBR leaves the battlefield before the trigger resolves (`mtg-engine/src/triggers.rs`, line 894–899)
  - Oracle text says: `"When this creature enters, untap all creatures you control."`
  - Code does: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { ... behavior.on_enter_battlefield(...) }` — if VBR is bounced or destroyed in response to its own ETB trigger (the engine has a full priority model: `state.priority_player` is set and players can cast instants / activate abilities before the top of stack resolves), the zone check returns `false` and the untap effect is silently skipped. Per MTG rules, this triggered ability does not require its source to remain on the battlefield to resolve; the effect "untap all creatures you control" is evaluated at resolution time using the controller stored in the trigger (`PendingTrigger::EnteredBattlefield { controller, .. }`), not the source's current state.

### Tricky interactions checked

- **Mana cost {2}{W}**: `ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::White)])` — PASS
- **Subtype Human, Scout**: `subtypes: vec!["Human".into(), "Scout".into()]` — PASS
- **P/T 1/4**: `power: Some(1), toughness: Some(4)` — PASS
- **Flash keyword present**: `keywords: vec![Keyword::Flash]` — PASS
- **ETB trigger kind is correct**: `TriggerKind::EntersBattlefield` declared in `triggered_abilities` — PASS
- **"all creatures you control" — not "another creature"**: the `on_enter_battlefield` filter does not exclude VBR herself (`o.id != object_id` is absent), so VBR is included — PASS (correct per oracle)
- **Opponent's creatures excluded**: filter `o.controller == controller` correctly limits untap to the controller's own creatures — PASS
- **Ruling: untapping an attacking creature doesn't remove it from combat**: attacking status is tracked separately in `state.combat.attackers` (a `HashMap<ObjectId, PlayerId>`); `on_enter_battlefield` only sets `tapped = false` on each object, leaving the combat map untouched — PASS
- **"all" means no targeting**: the code iterates all matching objects without presenting a target choice — PASS
- **Trigger fires for ETB into battlefield**: `collect_triggers` dispatches `GameEvent::EnteredBattlefield` and creates `PendingTrigger::EnteredBattlefield` for any registered card; VBR is registered, so the trigger is correctly collected — PASS
- **ETB resolution if source has left battlefield**: the engine guard at `triggers.rs:895` skips `on_enter_battlefield` if `o.zone != Battlefield`; the engine provides a real priority window (players can respond with instants before the top of stack resolves), so this skip is observable in-game — FAIL

### Test coverage

- Basic ETB untap (own creatures untapped, opponent's creature stays tapped): `mtg-engine/tests/tier3_cards.rs:143` (`village_bell_ringer_untaps_creatures`)
- Flash timing (casting at instant speed): NOT TESTED
- VBR leaves battlefield before ETB trigger resolves (source-absent resolution): NOT TESTED
- Attacking creature stays in combat after being untapped by VBR: NOT TESTED
- Ruling: untapping attacking creature doesn't remove it from combat: NOT TESTED
