## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Geistflame deals 1 damage to any target.
Flashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: ISSUE

### Code issues

- Engine's `AnyTarget` implementation excludes planeswalkers as valid targets (`mtg-engine/src/engine.rs` lines 836–864, 1074–1090, 1343–1358; `mtg-engine/src/cards/mod.rs` line 244–245)
  - Oracle text says: `"Geistflame deals 1 damage to any target."`
  - Code does: `TargetRequirement::AnyTarget` is described in its own comment as "Target any creature or player" and the engine's three implementations of `AnyTarget` target generation all filter objects to `o.power.is_some()` (i.e., creatures only) plus players — planeswalkers (e.g., Liliana of the Veil, Garruk Relentless which exist in the card pool) are never included. Per MTG rules, "any target" encompasses creatures, players, AND planeswalkers. Geistflame cannot target a planeswalker even if one is on the battlefield.

- `resolve_damage` helper does not remove loyalty counters when dealing damage to a planeswalker (`mtg-engine/src/cards/helpers.rs` lines 52–62)
  - Oracle text says: `"Geistflame deals 1 damage to any target."` — damage to a planeswalker causes it to lose that many loyalty counters (MTG CR 120.3).
  - Code does: `obj.damage_marked += amount;` — marks `damage_marked` on the object rather than subtracting from loyalty counters. There is no SBA or other mechanism that converts `damage_marked` to loyalty counter loss for planeswalkers. The only way a card correctly handles planeswalker damage in this engine is if it does so explicitly (as Stensia Bloodhall does at `mtg-engine/src/cards/isd/stensia_bloodhall.rs` lines 90–95). This issue is secondary to the targeting issue above but would cause incorrect behavior if a planeswalker were somehow targeted.

### Tricky interactions checked

- **Flashback exile on resolution**: `move_spell_after_resolve` checks `obj.cast_with_flashback` and routes to `Zone::Exile` if true. Flag set at cast time (engine.rs line 1637). PASS
- **Flashback exile when countered**: Counterspell calls `move_spell_after_resolve` on the countered spell (counterspell.rs line 50), which correctly exiles it if `cast_with_flashback` is set. PASS
- **Flashback exile when fizzled (all targets illegal)**: `resolve_spell` in stack.rs line 84 calls `move_spell_after_resolve` when all targets are illegal, correctly exiling flashback spells. PASS
- **Timing of flashback cast**: Geistflame is an Instant; engine checks `data.card_types.contains(&CardType::Instant)` → `can_cast_timing = true` (engine.rs lines 692–698), so flashback is offered at any time. PASS
- **"You may" flashback optionality**: Flashback generates a `CastSpell` action; the player can choose not to take it. Optionality is correctly preserved. PASS
- **AnyTarget includes planeswalkers**: The engine's `AnyTarget` handler filters objects to `o.power.is_some()` (creatures only) plus players. Planeswalkers are never generated as targets. Geistflame cannot target planeswalkers. FAIL
- **Damage amount**: `resolve_damage` called with `amount = 1`. Oracle says 1 damage. PASS
- **Normal cast to graveyard**: `cast_with_flashback` not set for hand casts → `move_spell_after_resolve` sends to `Zone::Graveyard`. PASS
- **Target legality check at resolution**: stack.rs checks that at least one target is still legal (on battlefield or stack). If the target has left the battlefield, the spell fizzles and still exiles (if flashback). PASS
- **Flashback cost**: Code `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Red)])` = {3}{R}. Oracle says Flashback {3}{R}. PASS
- **Mana cost**: Code `ManaCost::new(vec![ManaSymbol::Colored(Color::Red)])` = {R}. Oracle says {R}. PASS
- **damage_marked vs loyalty counter loss for planeswalkers**: `resolve_damage` marks `damage_marked`, does not remove loyalty counters. There is no general SBA converting `damage_marked` to loyalty counter loss for planeswalkers. FAIL (secondary to targeting issue)

### Test coverage

- Flashback offered from graveyard when mana available: `flashback.rs:23` TESTED
- Flashback not offered from hand: `flashback.rs:44` TESTED
- Flashback not offered without sufficient mana: `flashback.rs:64` TESTED
- Flashback spell exiled after resolution: `flashback.rs:86` TESTED
- Normal cast goes to graveyard: `flashback.rs:110` TESTED
- Flashback spell exiled when countered: `flashback.rs:128` TESTED
- Flashback spell exiled when fizzled (target died): `fizzle.rs:137` TESTED
- Fizzled flashback spell does not emit SpellResolved: `fizzle.rs:176` TESTED
- Geistflame deals 1 damage to creature: `tier2_spells.rs:33` TESTED
- Geistflame targeting a planeswalker: NOT TESTED
- resolve_damage correctly handles planeswalker loyalty counter loss: NOT TESTED
