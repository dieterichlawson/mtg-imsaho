## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Add {W}.
**Type line**: Creature — Human Monk
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Summoning sickness prevents {T} activation**: Correctly guarded by `!obj.summoning_sick` in `mana_abilities`. MTG rule 302.6 applies to all {T} abilities including mana abilities; confirmed code is correct. pass
- **Zone check — ability only available on battlefield**: `obj.zone == Zone::Battlefield` guard is present. pass
- **Tap state check — already-tapped Pilgrim not offered as an action**: `!obj.tapped` guard is present. Engine re-evaluates `mana_abilities` at execution time (line 1674 in engine.rs), so a race between action generation and execution cannot produce a double-tap. pass
- **Tap cost applied on activation**: Engine sets `tapped = true` and emits `GameEvent::Tapped` before adding mana (engine.rs lines 1676–1678). pass
- **White mana produced**: `produced: vec![(ManaType::White, 1)]` correctly produces exactly one White mana, not colorless or another color. pass
- **Summoning sickness cleared at correct time**: Cleared during `Step::Untap` for the active player's controlled creatures (engine.rs lines 2938–2946), matching MTG rules. pass
- **No unintended side effects**: `on_activate_mana_ability` is not overridden; default no-op is used. No milling, life-gain, or other side effects present, consistent with oracle text. pass
- **Deduplication of mana actions (multiple Pilgrims)**: Engine deduplicates by `(card_id, ability_index)` and stores the first encountered untapped object's `object_id` in the action. Since `mana_abilities` already filters out tapped/sick objects, only valid objects enter the dedup loop. Each priority window offers one Pilgrim tap at a time, but multiple taps are possible across successive priority windows. No incorrect behavior for Avacyn's Pilgrim. pass
- **ability_index Vec-index alignment**: Card returns a single-element Vec with `ability_index: 0`; engine looks up `abilities.get(0)` which correctly retrieves the only element. pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Card data (cost MV=1, P/T 1/1, subtypes Human+Monk): `innistrad_simple_cards.rs` — `avacyns_pilgrim_card_data`
- Tapping for White mana: `innistrad_simple_cards.rs` — `avacyns_pilgrim_taps_for_white`
- Summoning sickness prevents mana ability: `innistrad_simple_cards.rs` — `avacyns_pilgrim_cant_tap_with_summoning_sickness`
- Already-tapped Pilgrim cannot provide second mana action: NOT TESTED (specific to Pilgrim; general tap behavior tested elsewhere in engine)
- Summoning sickness cleared on Untap step: NOT TESTED for Pilgrim specifically; general sickness-clearing behavior exercised in other engine tests
