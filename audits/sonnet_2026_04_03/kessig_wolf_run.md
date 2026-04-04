## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Add {C}.
{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
**Type line**: Land
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **X=0 edge case**: PASS - Code correctly handles X=0 by giving +0/+0 and trample, verified in tests
- **Target requirement enforcement**: PASS - Code uses `TargetRequirement::Creature` which properly filters for creatures on battlefield
- **Until end of turn cleanup**: PASS - Both `until_end_of_turn_effects` and `until_end_of_turn_keywords` are cleared together in engine cleanup step
- **X value handling**: PASS - Engine sets `last_activated_x_value` properly, code reads it with `unwrap_or(0)` fallback
- **Effect application**: PASS - `effective_power()` includes until-end-of-turn power modifications, keyword system includes temporary keyword grants
- **Instant speed activation**: PASS - Activated ability has `sorcery_speed_only: false`, allowing instant-speed activation as expected

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **X=3 gives +3/+0 and trample**: `kessig_wolf_run.rs:62-96`
- **X=0 gives just trample (no power boost)**: `kessig_wolf_run.rs:98-127`
- **Cannot activate without both R and G mana**: `kessig_wolf_run.rs:40-59`
- **Can activate with just RG (X=0 minimum)**: `kessig_wolf_run.rs:18-38`
- **All mana is spent on activation**: `kessig_wolf_run.rs:93-95`
- **Trample keyword is properly granted**: `kessig_wolf_run.rs:88-91` and `kessig_wolf_run.rs:123-126`