## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature has intimidate. (It can't be blocked except by artifact creatures and/or creatures that share a color with it.)
**Type line**: Enchantment — Aura
**Status**: ISSUE

### Code issues

- Artifact creature tokens cannot block creatures with intimidate, violating the oracle rule "can't be blocked except by artifact creatures and/or creatures that share a color with it."
  - Oracle text says: `"It can't be blocked except by artifact creatures and/or creatures that share a color with it."`
  - Code does: `mtg-engine/src/combat.rs:632-634`:
    ```rust
    let is_artifact = registry.card_data(blocker.card_id)
        .map(|d| d.card_types.contains(&crate::types::CardType::Artifact))
        .unwrap_or(false);
    ```
    This only checks `registry.card_data(blocker.card_id)`. Tokens have `card_id: CardId(0)` (a sentinel not in the registry), so `registry.card_data` returns `None` and `unwrap_or(false)` yields `false`. The code then falls through to the color-sharing check, which also fails for colorless artifact tokens, causing the block to be incorrectly denied. In contrast, every other artifact check in the engine (e.g., `engine.rs:280-283` and `engine.rs:317-321`) correctly chains `|| obj.card_types.contains(&CardType::Artifact)` to cover tokens.

### Tricky interactions checked

- **Keyword grant via EffectScope::Attached**: `has_keyword` in `state.rs` calls `has_continuous_effect` which scans all battlefield permanents for `GrantKeyword { keyword: Intimidate, scope: Attached }`, then resolves `EffectScope::Attached` by checking `source.attached_to == Some(creature_id)`. This correctly and continuously grants intimidate only while the aura is attached. PASS.
- **Aura falls off if target illegal at resolution**: `resolve_aura` in `helpers.rs:18-31` checks that the target is still on the battlefield before attaching; if not, calls `move_spell_after_resolve` (goes to graveyard). PASS.
- **"As long as" continuous re-evaluation**: The continuous effect is not a snapshot — `has_keyword` is called dynamically each time a keyword check is needed (e.g., during blocking validation), so if the aura is removed the keyword is immediately lost. PASS.
- **Artifact creature (non-token) blocking**: `registry.card_data(blocker.card_id)` returns real data for registered artifact cards (e.g., One-Eyed Scarecrow), so non-token artifact creatures can correctly block intimidate attackers. PASS.
- **Artifact creature token blocking**: As described above, the artifact check omits the `|| blocker.card_types.contains(&CardType::Artifact)` fallback needed for tokens. FAIL.
- **Color-sharing check**: Uses `attacker.colors` and `blocker.colors` directly from the object, which are populated at game setup (for deck cards) and at token creation. PASS.
- **Intimidate granted via continuous effect is respected in combat**: `can_block_attacker` calls `state.has_keyword(attacker_id, Keyword::Intimidate, registry)`, which traverses continuous effects including aura grants, so a creature enchanted with Gruesome Deformity correctly triggers the intimidate blocking restriction. PASS.
- **Aura detachment when enchanted creature leaves**: `move_object` in `state.rs:479-487` clears `attached_to` on all objects leaving the battlefield, so if the enchanted creature dies, the aura's `attached_to` becomes None. The next `has_keyword` call will find no matching `EffectScope::Attached` source and return false. PASS.

### Test coverage

- Gruesome Deformity grants intimidate to the attached creature: `innistrad_cards.rs:291` TESTED
- Intimidate prevents non-matching-color creature from blocking: `keywords.rs:203` TESTED
- Intimidate allows same-color creature to block: `keywords.rs:203` TESTED
- Non-token artifact creature can block intimidate attacker: `keywords.rs:228` TESTED
- Artifact creature **token** can block intimidate attacker: NOT TESTED
- Aura falls off when target is gone at resolution: NOT TESTED
- Intimidate stops applying after aura is removed: NOT TESTED
