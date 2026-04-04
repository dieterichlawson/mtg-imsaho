## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues

- Extra Parallel Lives copies do not inherit post-creation token properties ("tapped", "tapped and attacking", dynamic P/T, combat assignment, delayed exile), because `create_token_with_subtypes` only returns the primary token's ID and silently discards the IDs of the extra copies it creates internally.
  - Oracle text says (ruling 2023-09-01): `"Everything that is specified by the effect creating the original token or tokens will also be true about the additional token or tokens created by Parallel Lives's replacement effect. For example, if an effect tells you to create a token 'tapped and attacking,' the additional tokens will also be tapped and attacking."`
  - Code does (`mtg-engine/src/state.rs` lines 337–348):
    ```rust
    // Create the primary token.
    let id = self.create_token_internal(name, owner, power, toughness,
        colors.clone(), card_types.clone(), keywords.clone(), subtypes.clone());

    // Create extra copies for Parallel Lives.
    for _ in 0..extra_copies {
        self.create_token_internal(name, owner, power, toughness,
            colors.clone(), card_types.clone(), keywords.clone(), subtypes.clone());
    }

    id
    ```
    Only `id` (the primary token) is returned. The IDs of extra copies are never exposed to callers. Any post-creation property set by the caller (e.g., `obj.tapped = true`, `combat.attackers.insert(token_id, ...)`, `state.end_of_combat_exiles.push(token_id)`, `token.card_state.insert("pt_source_counter", ...)`) is applied only to the primary token. This manifests as concrete wrong behavior in at least three cards:

  **Army of the Damned** (`mtg-engine/src/cards/isd/army_of_the_damned.rs` lines 44–54): Oracle says "Create thirteen tapped 2/2 black Zombie creature tokens." The code does `obj.tapped = true` on the returned primary `token_id` only. With Parallel Lives in play, the 13 extra copies are created untapped, contradicting the "tapped" specification.

  **Geist of Saint Traft** (`mtg-engine/src/cards/isd/geist_of_saint_traft.rs` lines 57–81): Oracle says create a 4/4 Angel "that's tapped and attacking. Exile that token at end of combat." The code sets `tapped = true`, adds to `combat.attackers`, and pushes to `end_of_combat_exiles` only on the returned primary `token_id`. With Parallel Lives, the extra Angel copy is untapped, not added to combat as an attacker, and never pushed to `end_of_combat_exiles` — so it stays on the battlefield indefinitely instead of being exiled at end of combat.

  **Kessig Cagebreakers** (`mtg-engine/src/cards/isd/kessig_cagebreakers.rs` lines 61–76): Oracle says create Wolf tokens "that's tapped and attacking." The code sets `tapped = true` and inserts into `combat.attackers` only on the returned primary `token_id` per loop iteration. With Parallel Lives, the extra Wolf copies are untapped and not added to combat.

  **Gutter Grime** (`mtg-engine/src/cards/isd/gutter_grime.rs` lines 63–76): Oracle says the token's P/T equals the number of slime counters on Gutter Grime. The code links the dynamic P/T by inserting `"pt_source_counter"` into `card_state` only on the returned primary `token_id`. With Parallel Lives, extra Ooze copies lack the `pt_source_counter` link and remain 0/0 with static P/T.

### Tricky interactions checked

- **Basic token doubling (1 Parallel Lives)**: The count/extra-copies math `(1 << 1) - 1 = 1` extra copy is correct; total = 2. PASS.
- **Exponential stacking (2+ Parallel Lives)**: Math `(1 << N) - 1` gives 4x for N=2, 8x for N=3, matching ruling [2023-09-01]. PASS in logic, but NOT TESTED with any automated test.
- **Only doubles controller's tokens**: Check `o.controller == owner` correctly excludes opponent's Parallel Lives from doubling tokens the opponent creates. PASS.
- **"Tapped and attacking" extra copies**: Extra copies from Parallel Lives are never set to tapped or added to combat as attackers (Geist of Saint Traft, Kessig Cagebreakers). FAIL — see issue above.
- **"Exile at end of combat" extra copies**: Extra Parallel Lives copies of Geist of Saint Traft's Angel token are never pushed to `end_of_combat_exiles`, so they are never exiled. FAIL — see issue above.
- **Dynamic P/T link on extra copies**: Gutter Grime's extra Ooze copies from Parallel Lives lack the `pt_source_counter` card_state link, so they are static 0/0 instead of tracking slime counters. FAIL — see issue above.
- **"Tapped" (non-attacking) extra copies**: Extra copies from Parallel Lives (Army of the Damned) are not tapped when the effect specifies "tapped". FAIL — see issue above.
- **All token creation paths route through `create_token_with_subtypes`**: Confirmed by source search — `create_token`, `create_token_with_subtypes`, and `create_token_copy` all feed through `create_token_with_subtypes`. No bypass paths exist. PASS.
- **Opponent's tokens not doubled**: Parallel Lives check uses controller identity; tokens created for the opponent are not doubled. PASS.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Basic doubling — 1 Parallel Lives creates 2 tokens: `tier14_cards.rs:81` (`parallel_lives_doubles_tokens`) — TESTED.
- No doubling without Parallel Lives: `tier14_cards.rs:105` (`no_parallel_lives_single_token`) — TESTED.
- Only doubles controller's tokens (not opponent's): `tier14_cards.rs:124` (`parallel_lives_only_doubles_for_controller`) — TESTED.
- Two Parallel Lives = 4x tokens (ruling [2023-09-01]): NOT TESTED.
- Extra copies are "tapped and attacking" when effect specifies it (ruling [2023-09-01]): NOT TESTED.
- Extra copies exiled at end of combat (Geist of Saint Traft + Parallel Lives interaction): NOT TESTED.
- Extra copies have correct dynamic P/T (Gutter Grime + Parallel Lives interaction): NOT TESTED.
- Extra copies are tapped when effect specifies "tapped" (Army of the Damned + Parallel Lives): NOT TESTED.
