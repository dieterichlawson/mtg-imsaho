## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Destroy target Spirit or enchantment.
**Type line**: Instant
**Status**: ISSUE

### Code issues

- `is_valid_target` only checks `registry.card_data(obj.card_id)` for subtypes/card_types, missing Spirit tokens
  - Oracle text says: `Destroy target Spirit or enchantment.`
  - Code does (`mtg-engine/src/cards/isd/urgent_exorcism.rs` lines 40–45):
    ```rust
    registry.card_data(obj.card_id)
        .map(|d| {
            d.card_types.contains(&CardType::Enchantment)
                || d.subtypes.contains(&"Spirit".to_string())
        })
        .unwrap_or(false)
    ```
    Spirit tokens have `card_id = CardId(0)` (the sentinel for tokens). `registry.card_data(CardId(0))` returns `None`, so the entire expression evaluates to `false`. As a result, Spirit tokens (e.g., those created by Midnight Haunting, Doomed Traveler, Mausoleum Guard) cannot be legally targeted by Urgent Exorcism, even though the oracle text says "target Spirit or enchantment" with no restriction excluding tokens.

    By contrast, `state.rs` `matches_filter` (line 666–672) correctly checks BOTH `registry.card_data(creature.card_id)` AND `creature.subtypes` directly to handle tokens:
    ```rust
    if registry.card_data(creature.card_id)
        .map(|d| d.subtypes.iter().any(|s| s == subtype))
        .unwrap_or(false) {
        return true;
    }
    // ...
    creature.subtypes.iter().any(|s| s == subtype)
    ```
    `is_valid_target` in `urgent_exorcism.rs` does not perform the second check against `obj.subtypes`.

    Spirit tokens store their subtype correctly: `create_token_with_subtypes` in `midnight_haunting.rs` (line 32) passes `subtypes: vec!["Spirit".into()]`, which is stored on `obj.subtypes`. The `is_valid_target` code never reads `obj.subtypes`, so this information is ignored.

### Tricky interactions checked

- Targeting a non-token Spirit (e.g., Chapel Geist): PASS — `registry.card_data()` returns `Some(d)` for real cards; `d.subtypes.contains("Spirit")` is true.
- Targeting a Spirit token (e.g., from Midnight Haunting): FAIL — `registry.card_data(CardId(0))` returns `None`; `is_valid_target` returns `false`; token cannot be targeted.
- Targeting a non-token enchantment (e.g., Pacifism): PASS — `registry.card_data()` returns `Some(d)` for real cards; `d.card_types.contains(Enchantment)` is true.
- `move_spell_after_resolve` used (not raw `move_object`): PASS — `resolve_destroy` at helpers.rs line 101 calls `state.move_spell_after_resolve(spell_id)`, correctly exiling flashback-cast copies.
- Indestructible/regeneration handling: PASS — `resolve_destroy` calls `crate::destruction::try_destroy`, which goes through the destruction pipeline.
- Target validity at resolution (fizzle if target leaves): PASS — `resolve_destroy` re-checks `obj.zone == Zone::Battlefield` before destroying.
- `matches_target_filter` pre-filter for `SubtypeOrCardType`: `_ => true` (engine.rs line 1398), so the token would pass the pre-filter. The failure is solely in `is_valid_target`.

### Test coverage

- Urgent Exorcism destroys a non-token Spirit: `tier2_spells.rs:317` (tests Chapel Geist)
- Urgent Exorcism targeting a Spirit token: NOT TESTED
- Urgent Exorcism destroys an enchantment: NOT TESTED
- Urgent Exorcism cannot target a non-Spirit, non-enchantment creature: NOT TESTED
- Flashback-cast Urgent Exorcism is exiled instead of going to graveyard: NOT TESTED (card has no flashback, but `move_spell_after_resolve` handles it generically — not applicable here)
