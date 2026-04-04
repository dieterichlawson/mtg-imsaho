## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
As long as you control a Human, this creature has hexproof and indestructible.
**Type line**: Creature — Angel
**Status**: ISSUE

### Code issues

- Sequential SBA processing allows a protecting Human to die before Angelic Overseer's indestructibility is evaluated, causing Angelic Overseer to be incorrectly destroyed.
  - Oracle text says: `"As long as you control a Human, this creature has hexproof and indestructible."` (and ruling: `"If you control a Human, and an effect tries to destroy each Human you control and Angelic Overseer simultaneously, Angelic Overseer won't be destroyed."`)
  - Code does: In `mtg-engine/src/sba.rs` lines 101–147, the `destroyed_ids` list is populated before any deaths occur, but the indestructibility check (`try_destroy` → `has_keyword(Indestructible)` → `check_condition(YouControlSubtype("Human"))`) is re-evaluated inside the sequential destruction loop, after earlier entries in the list have already been moved to the graveyard. Since `destroyed_ids` is populated by iterating `state.objects.values()` (a `HashMap` with non-deterministic iteration order), if a Human with lethal damage is visited before Angelic Overseer, the Human is moved to Zone::Graveyard first. When `try_destroy` is then called for Angelic Overseer, `check_condition` scans `Zone::Battlefield` for Humans, finds none, and returns `false`; Angelic Overseer is destroyed. Under MTG rule 704.3 all SBAs are performed simultaneously as a single event, so Angelic Overseer should retain indestructible for the entire batch because the Human was still on the battlefield when the SBA check was initiated.

### Tricky interactions checked

- "As long as" continuous re-evaluation: PASS — `has_keyword` calls `has_conditional_keyword` → `check_condition` every time it is invoked; the condition is not snapshotted at ETB.
- Human token recognition in `check_condition`: PASS — `check_condition(YouControlSubtype("Human"))` checks both `o.subtypes` (populated directly for tokens in `create_token_internal`) and `registry.card_data(o.card_id)` for regular cards (`state.rs` lines 1084–1093).
- Hexproof prevents opponent targeting: PASS — `can_be_targeted` in `engine.rs` line 759 calls `state.has_keyword(target_id, Keyword::Hexproof, registry)`, which routes through `has_conditional_keyword` and respects the Human condition.
- Indestructible prevents destroy effects: PASS — `try_destroy` in `destruction.rs` line 35 calls `state.has_keyword(id, Keyword::Indestructible, registry)` before moving the permanent, so a Doom Blade or similar destroy spell is correctly stopped while a Human is on the battlefield.
- Simultaneous SBA death (Human + Angelic Overseer both have lethal damage): FAIL — as described in Code Issues, HashMap iteration order determines whether the Human dies before or after Angelic Overseer's indestructibility is checked. If the Human is processed first, Angelic Overseer is incorrectly destroyed. This is non-deterministic.
- Lethal damage stays marked while indestructible (ruling 2): PASS — when `try_destroy` returns `DestroyResult::Indestructible` (line 115 of `sba.rs`), no state change occurs and the damage remains; if the Human later leaves, the next SBA pass finds lethal damage again and destroys Angelic Overseer.
- Hexproof from self-targeting (player can still target own hexproof creature): PASS — `can_be_targeted` at `engine.rs` line 763–764 only blocks targeting when `controller != caster`.
- Effect scope is OnSelf (no spillover to other creatures): PASS — both `ConditionalKeyword` effects use `EffectScope::OnSelf`; `effect_applies_to` for `OnSelf` checks `creature_id == source_id`.

### Test coverage

- Flying keyword always present: `tier12_cards.rs:563` (`angelic_overseer_has_flying`)
- Hexproof/indestructible granted when Human present: `tier12_cards.rs:574` (`angelic_overseer_hexproof_indestructible_with_human`)
- Hexproof/indestructible lost when Human leaves: `tier12_cards.rs:574` (same test, Human removed mid-test)
- Destroy effect blocked by indestructible: `tier12_cards.rs:605` (`angelic_overseer_survives_destroy_with_human`)
- Human token recognized as Human (triggers condition): NOT TESTED
- Simultaneous SBA death (Human + Angelic Overseer have lethal damage in same SBA pass): NOT TESTED
- Lethal damage remains marked while indestructible, Angelic Overseer dies after Human leaves: NOT TESTED
- Hexproof blocks opponent spell targeting: NOT TESTED
