## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Return target creature card from your graveyard to the battlefield.
Flashback {3}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: ISSUE

### Code issues

- **Missing `target_requirement()` override — spell treated as untargeted**
  - File: `mtg-engine/src/cards/isd/unburial_rites.rs`, line 11–65
  - Oracle text says: `"Return target creature card from your graveyard to the battlefield."`
  - Code does: `UnburialRites` does not override `target_requirement()`. The `CardBehavior` trait default (see `cards/mod.rs` line 284–286) returns `TargetRequirement::None`. This means the engine generates `CastSpell { targets: vec![], ... }` for Unburial Rites (confirmed in `engine.rs` line 833–835: `TargetRequirement::None => { vec![Action::CastSpell { ..., targets: vec![], ... }] }`). The spell is treated as untargeted, so (a) the player never selects a target at cast time; (b) the spell can be legally announced even when there are no creature cards in the controller's graveyard; and (c) no target is stored on the stack object for legality checks or opponent awareness.

- **Target selected at resolution time, not at cast time — ignores `targets` parameter**
  - File: `mtg-engine/src/cards/isd/unburial_rites.rs`, lines 31–64; specifically the `_targets` parameter at line 31
  - Oracle text says: `"Return target creature card from your graveyard to the battlefield."` (target is chosen at cast time per CR 601.2c)
  - Code does: `fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry)` — the `_targets` parameter is explicitly marked unused (underscore prefix). The code then re-scans the graveyard at resolution time (lines 34–37): `let targets: Vec<Target> = state.objects.values().filter(|o| o.zone == Zone::Graveyard && o.owner == controller && o.power.is_some() && o.id != object_id).map(|o| Target::Object(o.id)).collect();` — meaning the choice is made at resolution, not at cast time. Consequence: if a new creature enters the graveyard between cast and resolution, it becomes eligible; if the originally-intended creature is exiled in response (e.g., by Purify the Grave), the spell selects a different creature and does not fizzle, contrary to CR 608.2b.

- **Spell can be cast with no legal targets**
  - File: `mtg-engine/src/cards/isd/unburial_rites.rs` (missing `target_requirement`) / `mtg-engine/src/engine.rs` line 833
  - Oracle text says: `"Return target creature card from your graveyard to the battlefield."` (CR 601.2c: you may not cast a spell that requires targets if there are no legal targets)
  - Code does: Because `target_requirement()` returns `TargetRequirement::None`, the engine always generates a cast action for Unburial Rites regardless of whether any creature cards exist in the controller's graveyard. Casting into an empty graveyard is illegal in MTG but permitted here, with the spell only "fizzling" at resolution (lines 39–41) rather than being blocked at cast time.

### Tricky interactions checked

- **Target chosen at cast vs. resolution time**: FAIL — the target is selected at resolution (re-scans graveyard), not when the spell is announced. Violates CR 601.2c.
- **Fizzle when target becomes invalid in response (e.g., Purify the Grave exiles the creature)**: FAIL — the code re-scans at resolution and picks any remaining graveyard creature instead of fizzling (as required by CR 608.2b).
- **Casting without legal targets**: FAIL — the engine generates a cast action even with an empty graveyard; only blocked at resolution, not at cast time.
- **"from your graveyard" filters to controller's graveyard only**: PASS — code filters `o.owner == controller` (line 35), correctly scoping to the spell controller's own graveyard.
- **Creature card detection via `o.power.is_some()`**: PASS in practice — consistent with how the engine populates objects from card data (non-creature cards have `power: None`; creature cards have `power: Some(n)`).
- **Flashback cost declared correctly**: PASS — `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::White)]))` matches oracle `{3}{W}`.
- **Flashback spell exiled after resolution**: PASS — `move_spell_after_resolve` is called (lines 41, 49) and uses the `cast_with_flashback` flag (set in `engine.rs` line 1637) to route to `Zone::Exile` vs `Zone::Graveyard`.
- **Flashback timing restriction (sorcery speed only)**: PASS — engine enforces sorcery-speed timing for sorcery types during graveyard flashback generation (engine.rs lines 692–706).
- **Mana cost {4}{B}**: PASS — `cost: Some(ManaCost::new(vec![ManaSymbol::Generic(4), ManaSymbol::Colored(Color::Black)]))` matches oracle.
- **"You may" optionality for flashback**: PASS — flashback is offered as an optional action; the player can decline by not selecting it.
- **Spell exiled after being countered via flashback**: PASS — `move_spell_after_resolve` is called on counter path via `stack.rs` lines 83–84 and checks `cast_with_flashback`.

### Test coverage

- **Target chosen at cast time**: NOT TESTED — `unburial_rites_returns_creature` in `flashback.rs` line 304 calls `cast_and_resolve(..., vec![])` (empty targets), which models the broken untargeted behavior and does not test that a target is actually declared at cast time.
- **Fizzle when target exiled in response**: NOT TESTED
- **Cannot cast without legal targets**: NOT TESTED
- **Basic case — creature returned to battlefield**: `flashback.rs:304` (`unburial_rites_returns_creature`) — tested, passes.
- **Multiple creatures in graveyard (player prompted to choose)**: NOT TESTED — the `AwaitingAction::ResolutionChoice` branch (lines 52–62) has no test.
- **Flashback version exiles spell after resolution**: NOT TESTED specifically for Unburial Rites (generic flashback exile tested for other cards in `flashback.rs:86`).
- **Flashback cost {3}{W}**: NOT TESTED specifically for Unburial Rites.
- **Sorcery timing restriction for flashback**: NOT TESTED specifically for Unburial Rites.
