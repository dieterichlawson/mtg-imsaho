## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature dies, create two 1/1 white Spirit creature tokens with flying.
**Type line**: Creature — Human Scout
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **SelfDies trigger dispatch**: `collect_triggers` in `triggers.rs` at line 401–415 creates a `PendingTrigger::SelfDies` for any registered card that dies. Mausoleum Guard is registered (`registry.get(dead_card_id).is_some()`), so the trigger is always pushed. `resolve_next_trigger` at line 901–904 calls `behavior.on_dies(state, dead_id, registry)` without a battlefield-presence check — correct, because the card is dead and not supposed to be on the battlefield. The trigger fires as expected: PASS
- **Controller lookup on dead object**: `on_dies` reads `state.get_object(object_id).map(|o| o.controller)` where `object_id` is the dead creature. After death via `destroy()` in `destruction.rs`, the object is moved to `Zone::Graveyard` but remains in `state.objects` with its controller intact. The lookup succeeds and returns the correct controller (not owner). PASS
- **Exact token count**: The loop `for _ in 0..2` runs exactly twice, creating two tokens — matching "create two" in oracle text. PASS
- **Token characteristics — 1/1 white Spirit with flying**: `create_token_with_subtypes("Spirit", controller, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying], vec!["Spirit".into()])` matches the oracle exactly: name/subtype "Spirit", 1/1, white, Creature type, Flying keyword. PASS
- **Parallel Lives interaction**: Each `create_token_with_subtypes` call internally checks for Parallel Lives and doubles that single creation. Two calls with one Parallel Lives on the battlefield yields 4 Spirit tokens — correct per Parallel Lives rules ("create twice that many of those tokens instead"). PASS
- **Tokens created under controller, not owner**: Code uses `o.controller` from the dead object, not `o.owner`. If the Guard was under an opponent's control when it died, the tokens correctly go to that controller. PASS
- **Card data — mana cost, P/T, subtypes**: `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::White)])` matches `{3}{W}`; `power: Some(2), toughness: Some(2)` matches 2/2; subtypes `["Human", "Scout"]` match type line "Human Scout". PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic death trigger creates exactly two 1/1 white Spirit tokens with flying: `tier3_cards.rs:109` (test `mausoleum_guard_creates_two_spirits_on_death`) — TESTED
- Tokens have flying keyword: `tier3_cards.rs:131` — TESTED
- Tokens are 1/1: `tier3_cards.rs:127–128` — TESTED
- Token color (white) and subtype ("Spirit"): NOT TESTED (test checks name and keywords but not `o.colors` or `o.subtypes`)
- Controller vs owner distinction (stolen creature scenario): NOT TESTED
- Parallel Lives doubling: NOT TESTED
