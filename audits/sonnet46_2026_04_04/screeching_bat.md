## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying\nAt the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
**Back face oracle text**: At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
**Type line**: Creature — Bat (front) / Creature — Vampire (back)
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **"You may" is optional**: The `on_upkeep` method sets `awaiting_action` to a `YesNo` `ResolutionChoice`, giving the player a genuine yes/no decision. The player can decline and no transform occurs. PASS.

- **Can't-pay path suppresses choice**: If `can_pay(pool, &cost)` is false, `on_upkeep` returns without setting `awaiting_action`. Strictly, the oracle always triggers the ability, but since the player can never satisfy a cost they can't afford, the effective behavior is identical to presenting the choice. This is a known engine-wide optimization applied consistently across all cost-conditional upkeep abilities; functionally correct. PASS.

- **Upkeep trigger fires during controller's upkeep only**: `collect_triggers` queues upkeep triggers for ALL battlefield permanents regardless of whose upkeep it is. When it is not the controller's upkeep, `on_upkeep` checks `state.active_player != controller` and returns immediately without setting `awaiting_action`. Because `process_triggers` runs before any player receives priority (it runs at the top of `run_game_loop_inner`'s loop body), this spurious trigger resolves silently and is never visible to players. Functional behavior matches "at the beginning of YOUR upkeep." PASS.

- **Back-face (Stalking Vampire) upkeep trigger fires correctly**: `trigger_description` checks the front-face triggered abilities first. Since the front face declares `TriggerKind::Upkeep`, it returns a non-empty description even when `is_transformed = true`. This correctly produces an `UpkeepTrigger` for Stalking Vampire, which resolves via `on_upkeep`. PASS.

- **Transform cost {2}{B}{B}**: `Self::transform_cost()` returns `[Generic(2), Black, Black]`. Mana is consumed via `auto_pay` in `on_yes_no_choice` only when the player says yes. PASS.

- **Flying keyword removed on transform to Stalking Vampire**: `helpers::apply_transform` sets `obj.keywords = back.keywords.clone()` = `vec![]`. `has_keyword` checks `obj.keywords` first (now empty), then `behavior.back_face_data().keywords` (also empty). Result: no Flying when transformed. PASS.

- **Flying keyword restored on transform back to Screeching Bat**: `apply_transform` sets `obj.keywords = front.keywords.clone()` = `vec![Keyword::Flying]`. `has_keyword` finds Flying in `obj.keywords` immediately. PASS.

- **Subtypes updated on both transform directions**: `apply_transform` sets `obj.subtypes = back.subtypes.clone()` (= `["Vampire"]`) on front→back, and `obj.subtypes = front.subtypes.clone()` (= `["Bat"]`) on back→front. PASS.

- **P/T via `dynamic_pt`**: `dynamic_pt` returns `Some((5, 5))` when `obj.is_transformed` is true, and `None` when false. `effective_power` / `effective_toughness` use this override. Front face resolves to `obj.power`/`obj.toughness` = 2/2. Back face resolves to 5/5. PASS.

- **Object must be on battlefield for trigger and transform**: Both `on_upkeep` and `apply_transform` guard with `o.zone == Zone::Battlefield`. If the bat leaves the battlefield between trigger collection and resolution, neither proceeds. PASS.

- **Payment race (mana pool changes between `on_upkeep` and `on_yes_no_choice`)**: `on_yes_no_choice` calls `auto_pay` and handles the error case gracefully if mana is no longer available. PASS.

- **`dynamic_pt` not consulted for aura-attached objects**: The aura path in `continuous_pt_mods` calls `dynamic_pt(self, source.id)` where `source.id` is the aura's id, not the bat's id. Screeching Bat checks `obj.is_transformed` on the object passed in, and that object would be the aura (not transformed). This path is irrelevant; Screeching Bat is a creature, not an aura. PASS.

- **Back-face has no Flying per oracle**: `back_face_data()` declares `keywords: vec![]`. Oracle text for Stalking Vampire lists no keywords. PASS.

### Test coverage
For each ruling and tricky interaction:
- Front-face transforms to Stalking Vampire when player pays: `tier15_cards.rs:1128` TESTED
- No transform when player declines: `tier15_cards.rs:1163` TESTED
- No choice presented when player lacks mana: `tier15_cards.rs:1195` TESTED
- Stalking Vampire transforms back to Screeching Bat when player pays: `tier15_cards.rs:1210` TESTED
- Stalking Vampire does not have Flying: `tier15_cards.rs:1245` TESTED
- Screeching Bat regains Flying on transform back: `tier15_cards.rs:1282` TESTED
- Subtypes updated correctly on both transform directions: `tier15_cards.rs:1324` TESTED
- Trigger fires correctly during opponent's upkeep (silent, no effect): NOT TESTED
- Mana correctly consumed on yes: `tier15_cards.rs:1158` TESTED (asserts `mana_pool.total() == 0`)
- Mana not consumed on no: `tier15_cards.rs:1190` TESTED (asserts `mana_pool.total() == 4`)
