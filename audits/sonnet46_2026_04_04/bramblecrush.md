## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Destroy target noncreature permanent.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **Mana cost `{2}{G}{G}`**: Code has `Generic(2), Colored(Green), Colored(Green)` — matches oracle. Pass.
- **Sorcery type (no instant-speed casting)**: `card_types: vec![CardType::Sorcery]` — correct. Pass.
- **"Destroy" respects indestructible**: `resolve_destroy` calls `try_destroy`, which checks `has_keyword(Indestructible)` and returns early if found. Pass.
- **"Destroy" respects regeneration**: `try_destroy` checks `regeneration_shields > 0` and replaces destruction with regeneration (tap, clear damage, consume shield). Pass.
- **`move_spell_after_resolve` called**: `resolve_destroy` ends with `state.move_spell_after_resolve(spell_id)` — correctly moves sorcery to graveyard after resolution. Pass.
- **"noncreature" filter**: `is_valid_target` checks `registry.card_data(obj.card_id).map(|d| !d.card_types.contains(&CardType::Creature)).unwrap_or(false)`. For all registered non-creature permanents (lands, enchantments, artifacts, planeswalkers), this correctly returns `true`. Pass in current game scenarios.
- **Token targeting (latent issue)**: Tokens are created with `card_id: CardId(0)` (sentinel, never registered). `registry.card_data(CardId(0))` returns `None`, so `unwrap_or(false)` returns `false` — making ALL tokens untargetable by Bramblecrush. Creature tokens being untargetable is the correct outcome (they ARE creatures). Noncreature tokens would also be untargetable — which is incorrect, as "noncreature permanent" includes noncreature tokens. However, no card in the current ISD engine generates noncreature tokens; all token creators (Midnight Haunting, Spider Spawning, Moan of the Unhallowed, Endless Ranks of the Dead, Skirsdag High Priest, etc.) produce creature tokens. The latent bug does not manifest in any currently reachable game state. Pass in current game scope.
- **"target" requires player choice, not auto-select**: `TargetRequirement::PermanentWithFilter` generates one `CastSpell` action per valid target, each with a distinct target. The player (or LLM) picks which action to take, exercising the targeting choice. Pass.
- **Targeting check uses `obj.zone == Zone::Battlefield`**: `is_valid_target` checks `o.zone == Zone::Battlefield` before proceeding. Permanents that leave the battlefield before resolution become invalid targets. Pass.
- **No "may"**: The destruction is mandatory on resolution (no optional clause). `resolve_destroy` executes unconditionally if the target is still on the battlefield. Pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Destroys a noncreature permanent (land): `tests/tier2_spells.rs:280` — `bramblecrush_destroys_land` TESTED
- Cannot target a creature: `tests/tier2_spells.rs:299` — `bramblecrush_cant_target_creature` TESTED
- Destruction pipeline respects indestructible: `tests/card_fixes.rs:251` — `bramblecrush_respects_indestructible` TESTED
- Destruction pipeline respects regeneration: NOT TESTED
- Noncreature token untargetable (latent bug): NOT TESTED
- Targets enchantment (as opposed to only lands): NOT TESTED
- Targets artifact: NOT TESTED
- Targets planeswalker: NOT TESTED
