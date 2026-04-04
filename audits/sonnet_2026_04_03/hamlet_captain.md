## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn.
**Type line**: Creature — Human Warrior
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"Other" exclusion**: Hamlet Captain correctly excludes itself from the buff using `o.id != self_id` filter. The ability affects "other Humans you control," not itself — pass.
- **Human subtype check for tokens**: Code checks both `o.subtypes.iter().any(|s| s == "Human")` and `registry.card_data(o.card_id).subtypes` to catch tokens that store subtypes on the object vs registry. This matches the pattern in `check_condition` — pass.
- **Until end of turn expiry**: `UntilEndOfTurnEffect` entries are cleared during cleanup step (`engine.rs:3021`: `state.until_end_of_turn_effects.clear()`), so +1/+1 buffs correctly expire — pass.
- **Trigger independence**: Once the ability resolves and creates `UntilEndOfTurnEffect` entries, the buff persists even if Hamlet Captain leaves the battlefield. This matches MTG rules for triggered abilities that create duration-based effects — pass.
- **Attack/Block trigger dispatch**: Engine correctly dispatches `AttacksTrigger` and `BlocksTrigger` from `AttackersDeclared`/`BlockersDeclared` events (triggers.rs:677-751, 752-847), both resolve to `on_attacks`/`on_blocks` methods calling `buff_humans()` — pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Attacks trigger buffing other Humans**: `tier12_cards.rs:hamlet_captain_buffs_humans_on_attack` 
- **Blocks trigger buffing other Humans**: `tier12_cards.rs:hamlet_captain_buffs_humans_on_block`
- **Self-exclusion ("other")**: `tier12_cards.rs:hamlet_captain_buffs_humans_on_attack:221-223`
- **Non-Human exclusion**: `tier12_cards.rs:hamlet_captain_buffs_humans_on_attack:217-219`
- **Until end of turn expiry**: NOT TESTED
- **Trigger independence when source leaves**: NOT TESTED
- **Token Human subtype recognition**: NOT TESTED