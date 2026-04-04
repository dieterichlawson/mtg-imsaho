## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card. Activate only as a sorcery.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues

- Token copies are created with no colors (`Vec::new()`) in `state.rs`
  - Oracle text says: `Create a token that's a copy of that card.`
  - Code does: In `state.rs` line 425, `create_token_copy` passes `Vec::new()` for colors with a `// colors TODO` comment: `Vec::new(), // colors TODO`. The source object's `.colors` field is read for `name/power/toughness/card_id` but colors are explicitly discarded. The token copy of any colored creature (e.g., Walking Corpse {1}{B}, Goblin Piker {1}{R}, Savannah Lions {W}) is created without colors. A copy must share all characteristics with the original including color. A colorless token copy of a Black creature can be destroyed by Doom Blade ("Destroy target nonblack creature"), since `engine.rs:1396` tests `!obj.colors.contains(&Color::Black)` and the token has an empty colors vec. This is wrong: the original Black creature's token copy should also be Black and thus immune to Doom Blade.

### Tricky interactions checked

- **Sorcery-speed restriction** (`sorcery_speed_only: true`): pass — `ActivatedAbilityDef.sorcery_speed_only` is set `true` and `engine.rs` line 360 enforces it with `if ab.sorcery_speed_only && !is_sorcery_speed { continue; }`.
- **Exile as cost vs. effect timing**: The oracle text places "Exile a creature card from your graveyard" before the colon, making it a cost. In the implementation the exile happens inside `on_activate_ability` (effect), not during the cost-payment phase. Since this ability can only be activated at sorcery speed with an empty stack, no opponent can respond between activation and resolution, so there is no practical impact in this engine. Not flagged as an issue.
- **Token copy created from exiled source**: After `state.move_object(creature_id, Zone::Exile)`, the object still exists in `state.objects` with `zone = Exile`. `create_token_copy` reads it via `get_object(source_id)` which does not filter by zone, so name/power/toughness/card_id are read correctly from the exiled object. pass.
- **Token colors not copied** (reported above): fail — `Vec::new()` used for colors.
- **ETB triggers fire for the token copy**: `create_token_copy` sets `token.card_id = creature.card_id` (line 444–447 in `state.rs`). `create_token_internal` emits `EnteredBattlefield`. `triggers.rs` line 344 picks up that event and creates a `PendingTrigger::EnteredBattlefield` with the creature's `card_id`; on resolution it calls `on_enter_battlefield` on the creature's behavior. Matches oracle ruling "Any 'enters' abilities of the creature will trigger when the token enters." pass.
- **"as enters" replacement effects for token**: `create_token_internal` calls `apply_entering_copy_replacement(id)`, which handles Essence of the Wild–style replacement. For most creatures this is a no-op. Matches oracle ruling. pass.
- **Mana cost taken from registry (not object)**: `activated_abilities` reads `registry.card_data(creature.card_id).and_then(|d| d.cost.clone())`. This is the printed mana cost of the card, ignoring cost-reduction effects. Matches oracle ruling "abilities that reduce the cost to cast a creature spell won't apply." pass.
- **ability_index encoding (ObjectId → usize)**: `creature.id.0 as usize` is used as the `ability_index`. On 64-bit platforms (u64 == usize in size) this is lossless. The engine's monotonic ID counter starting at 1 means IDs are never reused, so the mapping is always unique. pass.
- **Parallel Lives doubling**: `create_token_copy` calls `create_token_with_subtypes`, which checks for Parallel Lives before creating tokens. Token doubling applies correctly. pass.
- **Multiple activations / same creature twice**: Once a creature is exiled by the first activation, `objects_in_zone(Zone::Graveyard, controller)` no longer returns it. Subsequent calls to `activated_abilities` will not generate the ability for that creature. No infinite token loop possible. pass.
- **Creature detection heuristic** (`o.power.is_some() || registry.card_data(...).contains(Creature)`): The `||` ensures creatures that were milled (never on the battlefield, possibly `power = None`) are still identified via the registry. Non-creature cards don't have power set and aren't in Creature card_types. pass.
- **X in mana cost is treated as zero**: `registry.card_data(creature.card_id).and_then(|d| d.cost.clone())` returns the cost with X symbols included. The engine's X-cost payment logic (engine.rs line 1719–1731) drains leftover mana as X. So if a creature with {X}{G}{G} in its cost is in the graveyard, the ability would treat the remaining mana as X. The oracle ruling says "X is considered to be zero." The engine does NOT force X = 0 for this specific case; it lets leftover mana fill X. This could allow the player to pay more than the minimum to create a token (which per the ruling should always be a 0/0 or base P/T token). However, the P/T of the token comes from `source.power`/`source.toughness` as stored on the graveyard object, not from X. So the extra mana spent on X is just wasted — the token doesn't get larger. Not a correctness issue for token P/T; the only question is whether the player is forced to pay exactly X=0 mana. In practice this is unlikely to matter in game, but it is a minor imprecision. Not flagged as a separate issue since it does not cause incorrect game outcomes.

### Test coverage

For each ruling and tricky interaction, whether it is tested and where:

- Basic ability creates token copy and exiles creature: `tier15_cards.rs` line 813 (`back_from_the_brink_creates_token_copy`) — TESTED
- One ability generated per creature in graveyard: `tier15_cards.rs` line 845 (`back_from_the_brink_ability_per_creature_in_graveyard`) — TESTED
- No abilities when graveyard has no creatures: `tier15_cards.rs` line 883 (`back_from_the_brink_no_abilities_without_creatures_in_graveyard`) — TESTED
- Ability cost equals creature's mana cost: `tier15_cards.rs` line 900 (`back_from_the_brink_uses_creature_mana_cost`) — TESTED
- `sorcery_speed_only` flag: `tier15_cards.rs` line 921 (assertion `ability.sorcery_speed_only`) — TESTED
- Token copy has correct name and is a token: `tier15_cards.rs` lines 838–841 — TESTED
- Token copy colors match original creature: NOT TESTED (the color issue is not checked in any test)
- ETB triggers fire for the token copy: NOT TESTED
- Parallel Lives doubles the token: NOT TESTED
- X-cost creature treated as X=0: NOT TESTED
- Multiple creatures in graveyard produce independent abilities: TESTED (line 845)
- Double-faced creature copies front face: NOT TESTED
