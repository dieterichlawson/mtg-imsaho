## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"nontoken" filtering**: PASS - Code correctly checks `was_token` and excludes token deaths (line 53-56)
- **"you control" filtering**: PASS - Code correctly checks `dead_controller != controller` (lines 49-51)
- **"whenever" multiple deaths**: PASS - Each CreatureDied event generates separate triggers, correctly handling simultaneous deaths
- **Dynamic P/T tracking**: PASS - Tokens use `pt_source_counter` system to continuously track slime counters on their creator
- **Multiple Gutter Grimes**: PASS - Each token links to specific creator via `self_id`, not all Gutter Grimes
- **Gutter Grime leaves battlefield**: PASS - When source object not found, dynamic P/T returns 0, causing tokens to die
- **Trigger sequence**: PASS - Counter is added first (line 58), then token is created (lines 63-69), matching "put...then create"
- **Counter type mapping**: PASS - ObjectId(1) correctly maps to CounterType::Slime in state.rs effective_power/toughness functions

### Test coverage
Comprehensive test coverage exists in mtg-engine/tests/gutter_grime.rs:
- **Basic trigger and token creation**: `gutter_grime_creates_dynamic_pt_ooze` / TESTED
- **Dynamic P/T growth with additional counters**: `gutter_grime_ooze_tokens_grow_with_more_counters` / TESTED  
- **Token death filtering**: `gutter_grime_ignores_token_deaths` / TESTED
- **Controller filtering**: `gutter_grime_ignores_opponent_deaths` / TESTED
- **Source removal causing tokens to become 0/0**: `gutter_grime_ooze_tokens_become_zero_without_source` / TESTED
- **Multiple Gutter Grimes with distinct token tracking**: NOT TESTED
- **Simultaneous creature deaths creating multiple triggers**: NOT TESTED