## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This creature enters tapped.
**Type line**: Creature — Zombie
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "Enters tapped" is a replacement effect, not a triggered ability: The code comment at line 8 correctly notes this distinction. The card uses no `triggered_abilities` and implements the effect directly in `on_resolve`. PASS.
- ETB event fires before `tapped = true` is set within `on_resolve`: `move_object` emits `EnteredBattlefield` at line 511 of `state.rs`, while `obj.tapped = true` is set afterward in `on_resolve` (line 32 of `diregraf_ghoul.rs`). However, `collect_triggers` runs after `on_resolve` returns, so by the time any ETB triggers are collected and see the state, the creature IS already tapped. Functionally correct. PASS.
- Summoning sickness: `move_object` in `state.rs` (line 490–492) sets `summoning_sick = true` when any object enters the battlefield. Diregraf Ghoul correctly gains summoning sickness. PASS.
- Tapped state reset on leaving battlefield: `move_object` (line 479–487) clears `tapped`, `summoning_sick`, `damage_marked`, etc. when an object leaves the battlefield. PASS.
- Mana cost {B}: implemented as `ManaSymbol::Colored(Color::Black)`. PASS.
- P/T 2/2: `power: Some(2), toughness: Some(2)`. PASS.
- Subtype "Zombie": `subtypes: vec!["Zombie".into()]`. PASS.
- No keywords: `keywords: vec![]`. PASS.

### Test coverage
- Creature enters the battlefield tapped after resolution: `innistrad_cards.rs:141` (`diregraf_ghoul_enters_tapped`) — TESTED.
- Tapping is a replacement effect (never enters untapped): NOT TESTED (only post-resolution state is verified, not the precise moment of entry).
- Summoning sickness on entry: NOT TESTED for this specific card (general engine behavior is tested elsewhere in `summoning_sickness.rs`).
- Tapped state cleared when leaving battlefield: NOT TESTED for this specific card (general engine behavior tested in `state.rs:1550`).
