## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Exile target creature and all other creatures with the same name as that creature.
Flashback {5}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: ISSUE

### Code issues

- Engine does not re-check hexproof legality at resolution time (`mtg-engine/src/stack.rs:8-41`)
  - Oracle text says (via ruling 2025-01-24): `"If the target creature is an illegal target by the time Sever the Bloodline tries to resolve, the spell won't resolve. You won't exile any creatures at all."`
  - A creature is an illegal target for an opponent's spell if it has hexproof (CR 608.2b). If the targeted creature gains hexproof in response to Sever the Bloodline (e.g., via Ranger's Guile, which is in this engine's card set), the target becomes illegal at resolution and Sever should fizzle.
  - Code does: `is_target_legal` only checks zone (`_ => obj.zone == Zone::Battlefield || obj.zone == Zone::Stack`), not hexproof. The `can_be_targeted` hexproof check runs only during legal action generation at cast time, never at resolution. A target that gains hexproof after being targeted is still seen as legal at resolution, causing Sever to resolve and exile the now-hexproof creature when it should fizzle.
  - This is an engine-wide issue documented in `mtg-engine/tests/spell_fizzle.rs:192-226` (`bolt_target_gains_hexproof_before_resolution`), which confirms the engine does not re-check hexproof at resolution.

### Tricky interactions checked

- **"All other creatures with the same name" bypasses hexproof/protection**: pass — the code iterates all battlefield creatures by name without checking hexproof/protection for the non-targeted ones, matching the ruling "Other creatures with the same name will be exiled even if they have hexproof or protection."
- **Fizzle when target leaves the battlefield**: pass — `stack.rs` `is_target_legal` checks zone; if the target has left the battlefield, it returns `false`, the fizzle path runs, and `on_resolve` is never called.
- **Hexproof gained between cast and resolution**: FAIL — `is_target_legal` only checks zone, not hexproof. Sever resolves and exiles the creature even if it gained hexproof in response (e.g., via Ranger's Guile).
- **Double-faced card name (ruling 2017-03-14)**: pass — `move_object`/transform code updates `obj.name` to the active face's name on every transform (e.g., Reckless Waif → Merciless Predator sets `obj.name = "Merciless Predator"`). The name-search filter uses `o.name`, so only the current face is matched.
- **Only battlefield creatures exiled (ruling 2017-03-14)**: pass — filter is `o.zone == Zone::Battlefield`.
- **Flashback cost {5}{B}{B}**: pass — `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(5), ManaSymbol::Colored(Color::Black), ManaSymbol::Colored(Color::Black)]))` = 7 mana value, confirmed by test.
- **Flashback spell exiled after resolution**: pass — `move_spell_after_resolve` checks `cast_with_flashback` flag and routes to `Zone::Exile` for flashback casts.
- **Target creature included in the exile sweep**: pass — the code collects ALL battlefield creatures with the same name, which inherently includes the target.
- **Token name matching**: pass (internally consistent) — Spirit tokens from Midnight Haunting are named `"Spirit"` (not `"Spirit Token"` per the ruling), and Zombie tokens from Moan of the Unhallowed are named `"Zombie"`. Names are consistent within the engine, so name-matching for Sever works correctly across all same-type tokens. No card in the engine is named literally "Spirit" or "Zombie" that would cause a false match.
- **Mana cost {3}{B}**: pass — `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Black)])`.
- **`move_spell_after_resolve` vs `move_object(Graveyard)`**: pass — `on_resolve` correctly calls `state.move_spell_after_resolve(object_id)`, not `move_object(..., Zone::Graveyard)`.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Exile target + all same-named creatures: `mtg-engine/tests/tier12_cards.rs:522` (`sever_the_bloodline_exiles_all_with_same_name`) — TESTED
- Flashback cost exists and has correct mana value: `mtg-engine/tests/tier12_cards.rs:551` (`sever_the_bloodline_has_flashback`) — TESTED (mana value only; does not cast from graveyard)
- Flashback spell exiled after resolution: NOT TESTED specifically for Sever; generic mechanic tested in `mtg-engine/tests/flashback.rs:86` (`flashback_spell_is_exiled_after_resolve` using Geistflame)
- Fizzle when target is illegal at resolution (zone change): NOT TESTED for Sever; generic mechanic tested in `mtg-engine/tests/spell_fizzle.rs`
- Hexproof gained between cast and resolution causes fizzle: NOT TESTED for Sever; documented as wrong behavior in `mtg-engine/tests/spell_fizzle.rs:192` (`bolt_target_gains_hexproof_before_resolution`)
- Non-targeted same-name creatures bypass hexproof/protection: NOT TESTED
- Double-faced creature name matching (only current face): NOT TESTED
- Token name matching (all tokens of same type matched): `mtg-engine/tests/tier12_cards.rs:522` uses manually-set names; NOT TESTED with real token creation from Moan of the Unhallowed/Midnight Haunting
- Target leaves battlefield before resolution → fizzle: NOT TESTED for Sever specifically
- Only battlefield creatures exiled (not creatures in other zones): NOT TESTED explicitly
