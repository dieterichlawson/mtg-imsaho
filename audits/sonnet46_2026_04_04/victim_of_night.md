## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Destroy target non-Vampire, non-Werewolf, non-Zombie creature.
**Type line**: Instant
**Status**: ISSUE

### Code issues

- `is_valid_target` does not check `obj.subtypes` for the excluded subtypes — only `registry.card_data(obj.card_id).subtypes` is checked. Tokens are created with `card_id: CardId(0)` (a sentinel with no registry entry), so `registry.card_data(CardId(0))` returns `None`, causing the code to fall into the `else { true }` branch and treat all tokens as valid targets regardless of their creature type. This means Vampire tokens (e.g., from Bloodline Keeper's tap ability), Zombie tokens (e.g., from Moan of the Unhallowed, Army of the Damned, Endless Ranks of the Dead), and any other forbidden-subtype tokens are incorrectly presented as legal targets for Victim of Night.
  - Oracle text says: `Destroy target non-Vampire, non-Werewolf, non-Zombie creature.`
  - Code does:
    ```rust
    if let Some(data) = registry.card_data(obj.card_id) {
        !data.subtypes.iter().any(|s| s == "Vampire" || s == "Werewolf" || s == "Zombie")
    } else {
        true   // ← tokens have card_id: CardId(0), registry returns None, returns true = valid target
    }
    ```
    Tokens store their subtypes in `obj.subtypes`, not in the registry. The code never reads `obj.subtypes`. The correct pattern (illustrated in `bloodline_keeper.rs`) checks both: `o.subtypes.iter().any(|s| s == "Vampire")` first, then falls through to the registry check.

### Tricky interactions checked

- **Targeting Vampire token (e.g., from Bloodline Keeper)**: FAIL — token has `card_id: CardId(0)`, `registry.card_data(CardId(0))` returns `None`, `is_valid_target` returns `true` (incorrectly allows targeting)
- **Targeting Zombie token (e.g., from Moan of the Unhallowed)**: FAIL — same mechanism; Zombie token has `card_id: CardId(0)`, `is_valid_target` returns `true` (incorrectly allows targeting)
- **Targeting non-token Vampire (e.g., Markov Patrician)**: PASS — `registry.card_data()` finds `subtypes: ["Vampire"]`, returns `false` (correctly blocks targeting); confirmed by test `victim_of_night_cant_target_vampire`
- **Targeting non-token Zombie (e.g., Walking Corpse)**: PASS — `registry.card_data()` returns `subtypes: ["Zombie"]`, correctly blocked
- **Targeting non-token Werewolf DFC (e.g., Village Ironsmith / Ironfang)**: PASS — front-face `card_data()` includes `subtypes: ["Human", "Werewolf"]`, so `registry.card_data()` always finds "Werewolf" and blocks targeting; note that `on_upkeep` for VillageIronsmith does not call `apply_transform` and does not update `obj.subtypes`, but since registry data always returns the front face (which already lists Werewolf), targeting is still correctly blocked
- **Normal creature target (non-Vampire/Werewolf/Zombie)**: PASS — `registry.card_data()` returns subtypes without the excluded types, `is_valid_target` returns `true`; destroy pipeline via `resolve_destroy` → `try_destroy` correctly respects indestructible and regeneration
- **Indestructible creature targeted**: PASS — `resolve_destroy` calls `try_destroy`, which checks `has_keyword(Indestructible)` and skips destruction if true
- **Regenerating creature targeted**: PASS — `try_destroy` consumes a regeneration shield instead of destroying
- **Spell cleanup (flashback vs. graveyard)**: PASS — `resolve_destroy` calls `move_spell_after_resolve`, which correctly exiles if `cast_with_flashback` is set, otherwise moves to graveyard
- **Target leaves battlefield before resolution (fizzle)**: PASS — `resolve_destroy` checks `obj.zone == Zone::Battlefield` before calling `try_destroy`; spell still moves to graveyard via `move_spell_after_resolve`

### Test coverage

- Destroys a normal creature: `tier2_spells.rs:120` — TESTED (`victim_of_night_kills_normal_creature`)
- Cannot target non-token Vampire: `tier2_spells.rs:134` — TESTED (`victim_of_night_cant_target_vampire`)
- Cannot target Vampire token: NOT TESTED
- Cannot target Zombie token: NOT TESTED
- Cannot target non-token Zombie: NOT TESTED
- Cannot target non-token Werewolf DFC: NOT TESTED
- Target becomes indestructible before resolution (fizzle/no-effect): NOT TESTED
- Target leaves battlefield before resolution: NOT TESTED
