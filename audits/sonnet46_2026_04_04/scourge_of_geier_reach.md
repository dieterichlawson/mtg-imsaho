## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This creature gets +1/+1 for each creature your opponents control.
**Type line**: Creature — Elemental
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Dynamic P/T re-evaluation ("as long as" semantics)**: The card's `dynamic_pt` method is called every time `effective_power` / `effective_toughness` is invoked in `state.rs` (lines 868 and 912), so the bonus updates continuously whenever board state changes. PASS.
- **Double-counting from `continuous_pt_mods`**: `continuous_pt_mods` (state.rs:726) calls `dynamic_pt` only for sources that are auras attached to the target creature (`source.attached_to == Some(creature_id)`). Since Scourge is not an aura, its `dynamic_pt` is never invoked from `continuous_pt_mods`; the base is set once in `effective_power` / `effective_toughness` and external modifiers are added separately. No double-counting. PASS.
- **"your opponents" in a 2-player engine**: `state.opponent(controller)` (state.rs:622) returns `next_player`, which correctly gives the single other player in a 2-player game. The engine is documented as a 2-player engine. In a multiplayer context this would under-count, but there is no multiplayer support. PASS.
- **Opponent creature identification via `o.power.is_some()`**: The filter `o.power.is_some()` is the established engine pattern for identifying creature-type objects (also used identically in `geist_honored_monk.rs:40`). Non-creature permanents in this engine (lands, enchantments, planeswalkers, equipment) all have `power: None` in their `CardData`, so the proxy is reliable. PASS.
- **Self-exclusion**: The filter `o.controller == opponent` (where opponent ≠ controller) correctly excludes the Scourge itself from the count when it is controlled by its owner. PASS.
- **Mana cost**: Oracle `{3}{R}{R}` matches `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Red), ManaSymbol::Colored(Color::Red)])`. PASS.
- **Base P/T**: Oracle 3/3 matches card data `power: Some(3), toughness: Some(3)`. The `dynamic_pt` return value replaces these base stats (it returns the total `3 + N`), which is the correct engine contract. PASS.
- **No keywords, no triggered abilities, no continuous_effects**: Oracle text contains none; card data declares none. PASS.

### Test coverage
- **Basic scaling with opponent creatures**: `tier12_cards.rs:20` (`scourge_of_geier_reach_scales_with_opponent_creatures`) — asserts 3/3 with no opponents and 5/5 with 2 opponent creatures. TESTED.
- **Ignores own creatures**: `tier12_cards.rs:41` (`scourge_of_geier_reach_ignores_own_creatures`) — asserts P/T stays 3/3 when friendly creatures are added. TESTED.
- **Scourge does not count itself**: NOT TESTED explicitly (covered implicitly by the base-case test showing 3/3 with just Scourge on board, but there is no explicit test placing the Scourge under opponent control or verifying it is excluded when it is the only creature).
- **Interaction with auras / external P/T modifiers while dynamic_pt is active**: NOT TESTED (e.g., Wreath of Geists on the Scourge simultaneously with opponent creatures).
- **P/T updates after an opponent creature leaves the battlefield**: NOT TESTED (only tests static snapshots; no test removes an opponent creature and re-checks P/T).
