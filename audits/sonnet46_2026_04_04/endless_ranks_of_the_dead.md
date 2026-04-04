## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of your upkeep, create X 2/2 black Zombie creature tokens, where X is half the number of Zombies you control, rounded down.
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"your upkeep" — trigger only fires during controller's upkeep**: The `on_upkeep` handler at line 44 checks `state.active_player != controller` and returns early if it's not the controller's turn. Correct. Engine note: `collect_triggers` in `triggers.rs` (lines 604–639) collects upkeep triggers for ALL permanents during ANY upkeep step, so the trigger is also placed on the stack during the opponent's upkeep. However, `process_triggers` resolves triggers synchronously before any player gets priority, and the early-return guard ensures no tokens are created. No game-observable difference. PASS.
- **Zombie count includes tokens (object.subtypes)**: The zombie filter at lines 51–56 checks BOTH `registry.card_data(o.card_id).map(|d| d.subtypes.iter().any(|s| s == "Zombie"))` (for regular cards) AND `o.subtypes.iter().any(|s| s == "Zombie")` (for tokens whose subtypes live on the object). This matches the pattern used by `check_condition` in `state.rs`. PASS.
- **Zombie count at resolution time, not trigger time**: The zombie count is computed inside `on_upkeep` (lines 48–57), which is called at trigger resolution. Not computed at trigger collection time. This satisfies the ruling: "The number of Zombies you control is counted when the ability resolves." PASS.
- **Multiple Endless Ranks — second trigger sees tokens from first**: When the first trigger resolves it creates tokens; when the second trigger resolves later, `on_upkeep` re-counts the battlefield (which now includes the new tokens). PASS.
- **Floor division ("rounded down")**: Rust integer division on `usize` truncates toward zero, which for non-negative values is floor division. `zombie_count / 2` correctly implements "half, rounded down." With 0 or 1 Zombies, result is 0 (no tokens) per the ruling. PASS.
- **Token stats — 2/2 black Zombie creature**: `create_token_with_subtypes` called with power=2, toughness=2, colors=[Black], card_types=[Creature], subtypes=["Zombie"], name="Zombie". Matches oracle text exactly. PASS.
- **Enchantment leaves battlefield mid-trigger**: Lines 40–43 check `o.zone == Zone::Battlefield` before proceeding. If Endless Ranks is no longer on the battlefield when the trigger resolves, `on_upkeep` returns early. This is correct for a "at the beginning of your upkeep" trigger where the source is not relevant to the effect itself; however, the engine also independently checks zone in `resolve_next_trigger` (line 955) before calling `on_upkeep`. PASS.
- **Parallel Lives interaction**: `create_token_with_subtypes` checks for Parallel Lives and doubles tokens appropriately (each copy doubles). Not in oracle text but correctly handled at engine level. PASS.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic token creation (5 Zombies → 2 tokens, floor division): `mtg-engine/tests/tier7_cards.rs:104` (`endless_ranks_creates_zombie_tokens`)
- Fewer than 2 Zombies → no tokens (ruling 2011-09-22): NOT TESTED
- Multiple Endless Ranks — tokens from first count for second (ruling 2011-09-22): NOT TESTED
- Trigger does not fire during opponent's upkeep: NOT TESTED
- Token counting includes Zombie tokens (object.subtypes path): Partially tested — the single test uses Zombie tokens, so it does exercise the `o.subtypes` path, but does not explicitly test a mix of registry-based Zombies and tokens. `mtg-engine/tests/tier7_cards.rs:104`
- Enchantment leaves battlefield before trigger resolves: NOT TESTED
