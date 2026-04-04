## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {1}{R}: This creature gains first strike until end of turn.
**Type line**: Creature — Wolf
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Activated ability cost matches oracle `{1}{R}`: `ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Red)])` — PASS
- `requires_tap: false` — no tap symbol in oracle cost — PASS
- `once_per_turn: false` — oracle text has no "once per turn" restriction; ability can be activated multiple times per turn — PASS
- `sorcery_speed_only: false` — oracle text has no sorcery-speed restriction; can be activated at instant speed (e.g., during combat) — PASS
- "This creature" self-targeting — ability passes `object_id` directly to `UntilEndOfTurnKeyword { target: object_id }`, no targeting required from the player — PASS
- "Until end of turn" cleanup — `state.until_end_of_turn_keywords.clear()` is called in the cleanup step (`engine.rs:3022`); `has_keyword()` reads this vec at `state.rs:1036-1040` — PASS
- First strike is read via `has_keyword()` during combat damage (`combat.rs:139, 184, 220`) which correctly checks `until_end_of_turn_keywords` — PASS
- Card stats: cost {2}{R} → `Generic(2) + Colored(Red)`, P/T 3/1, subtype "Wolf", no native keywords — all match oracle — PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Card stats (P/T 3/1, subtype Wolf): `activated_abilities.rs:83` (`kessig_wolf_has_correct_stats`) — TESTED
- Activated ability grants first strike: `activated_abilities.rs:93` (`kessig_wolf_gains_first_strike`) — TESTED
- First strike expires at end of turn: NOT TESTED
- Ability can be activated at instant speed (e.g., during combat): NOT TESTED
- Ability can be activated multiple times in a turn: NOT TESTED
- First strike from this ability applies correctly in combat damage steps: NOT TESTED
