## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Hexproof (This creature can't be the target of spells or abilities your opponents control.)
Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat.
**Type line**: Legendary Creature — Spirit Cleric
**Status**: ISSUE

### Code issues

- Extra tokens created by Parallel Lives doubling are not tapped, not attacking, and not added to `end_of_combat_exiles` — `mtg-engine/src/cards/isd/geist_of_saint_traft.rs` lines 57–81
  - Oracle text says: `"create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat."` and ruling says: `"If you create more than one Angel token (most likely due to Doubling Season), both are exiled at end of combat."`
  - Code does: `let token_id = state.create_token_with_subtypes(...)` returns only the primary token ID; `create_token_with_subtypes` internally creates `2^N - 1` extra copies via Parallel Lives but those extra IDs are discarded. The code then exclusively uses `token_id` for `obj.tapped = true`, `obj.summoning_sick = false`, `combat.attackers.insert(token_id, defender)`, and `state.end_of_combat_exiles.push(token_id)`. The extra tokens land on the battlefield untapped, non-attacking, and are never added to `end_of_combat_exiles`, leaving them permanently on the battlefield after combat.

### Tricky interactions checked

- **Exile fires even if Geist dies during combat**: PASS — exile uses `state.end_of_combat_exiles` drained by `combat::end_combat()`, which does not check Geist's zone; tested by `angel_exiled_even_if_geist_dies`.
- **Angel token does not trigger "whenever a creature attacks" abilities**: PASS — `on_attacks` inserts the token directly into `combat.attackers` without emitting `AttackersDeclared`, so no `AttacksTrigger` / `AttackWatch` scans see it; consistent with the 2020-08-07 ruling.
- **Hexproof enforcement**: PASS — `keywords: vec![Keyword::Hexproof]` is declared; `can_be_targeted()` in `engine.rs:759` correctly blocks opponent targeting.
- **Token enters tapped and attacking (primary token)**: PASS — `obj.tapped = true`, `obj.summoning_sick = false`, and `combat.attackers.insert(token_id, defender)` are all set on the primary token.
- **Token exiled at end of combat (primary token)**: PASS — `state.end_of_combat_exiles.push(token_id)` is drained in `combat::end_combat()` (`combat.rs:556`) which runs during `perform_turn_based_actions` at `Step::EndCombat`.
- **EndCombat trigger registered but `on_end_combat` is a no-op**: NOT AN ISSUE — Geist registers `TriggerKind::EndCombat` in `triggered_abilities` and the trigger fires via the normal step-started machinery, but `on_end_combat` has no override and the default is a no-op. The exile already happened via `end_of_combat_exiles` before the trigger resolves. No mechanical harm, but the registered trigger produces a spurious stack entry that resolves silently.
- **Attacker choice for Angel token (2-player game)**: PASS — engine `CombatState.attackers` maps `ObjectId -> PlayerId`, supporting only player targets. Auto-selecting the opponent is the only valid choice in a 2-player game. The ruling's "you choose which player or planeswalker" is vacuously satisfied.
- **Copy of Angel token NOT exiled**: PASS — `end_of_combat_exiles` only tracks IDs explicitly pushed during `on_attacks`; a copy made by some other effect (e.g., Cackling Counterpart) would never be pushed, so it would not be exiled — matching the ruling.
- **Parallel Lives doubling — extra tokens tapped and attacking**: FAIL — see Code Issues.
- **Parallel Lives doubling — extra tokens exiled at end of combat**: FAIL — see Code Issues.
- **Legendary rule**: PASS — `on_resolve` sets `obj.is_legendary = true`; SBA in `sba.rs:290` enforces the legend rule using this flag.
- **No priority window between attack trigger collection and resolution**: PASS (engine-level note) — `process_triggers` in `triggers.rs:1029` resolves all triggers synchronously with no priority window between collection and resolution; the `AttacksTrigger` resolver's battlefield check (`triggers.rs:981`) therefore cannot be reached in a state where Geist has left mid-resolution. No observable deviation.

### Test coverage

- Geist creates Angel on attack: `mtg-engine/tests/geist_of_saint_traft.rs:20` — TESTED
- Angel exiled at end of combat (primary token): `mtg-engine/tests/geist_of_saint_traft.rs:44` — TESTED
- Angel exiled even if Geist dies during combat: `mtg-engine/tests/geist_of_saint_traft.rs:70` — TESTED
- Hexproof prevents opponent targeting: NOT TESTED (in geist-specific tests; covered generically in other keyword tests)
- Parallel Lives doubling — extra tokens tapped and attacking: NOT TESTED
- Parallel Lives doubling — extra tokens exiled at end of combat: NOT TESTED
- Angel token does not trigger "whenever a creature attacks" abilities: NOT TESTED
- Copy of Angel token not exiled: NOT TESTED
