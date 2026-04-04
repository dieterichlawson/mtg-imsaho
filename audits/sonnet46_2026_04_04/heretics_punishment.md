## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {3}{R}: Choose any target, then mill three cards. This enchantment deals damage to that permanent or player equal to the greatest mana value among the milled cards.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues

- `AnyTarget` engine implementation excludes planeswalkers as valid targets
  - Oracle text says: `Choose any target`
  - Code does: Both `valid_targets_for_req` (`engine.rs:1074–1090`) and `generate_ability_targets` (`engine.rs:1343–1358`) filter battlefield objects with `.filter(|o| o.power.is_some())`, which selects only creatures. Players are also added, but planeswalkers — which have no `power` field and are not players — are silently excluded. Per modern MTG rules, "any target" means any creature, planeswalker, or player. The engine's `PlayerOrPlaneswalker` path (line 1099) correctly adds planeswalkers, but the `AnyTarget` path never does. Garruk Relentless and Liliana of the Veil exist in this set and cannot be targeted by this ability.

### Tricky interactions checked

- **Fizzle when target becomes illegal at resolution**: PASS — `on_activate_ability` checks `Target::Object` zone == Battlefield (and `Target::Player` is always legal) before proceeding; returns early without milling if illegal. Matches ruling: "the entire ability won't resolve. No cards will be put into your graveyard."
- **Fewer than 3 cards in library**: PASS — code uses `std::cmp::min(3, player.library_order.len())`. Matches ruling: "all of them will be put into your graveyard."
- **All milled cards have MV 0 — no damage dealt**: PASS — code gates the entire damage block on `if max_mv > 0 { ... }`. Matches ruling: "If all three cards have a mana value of 0, no damage will be dealt."
- **MV computed before moving cards to graveyard**: PASS — `max_mv` is computed from the library-order slice before `drain` is called; `registry.card_data()` returns the front-face cost, consistent with the double-faced-card ruling.
- **Ordering — target chosen before milling**: PASS — the target is chosen at activation time (engine passes `targets` in), and the card handler only validates legality then mills; this is the correct MTG rule ordering.
- **Planeswalker targeting via `AnyTarget`**: FAIL — see Code Issues above. Both `valid_targets_for_req` (line 1076) and `generate_ability_targets` (line 1345) gate battlefield objects on `.filter(|o| o.power.is_some())`, which never matches a planeswalker.
- **Player damage path**: PASS — subtracts `max_mv` from life, emits `NonCombatDamageDealt` and `LifeChanged` events, consistent with the `resolve_damage` helper pattern used by other cards.
- **Creature damage path**: PASS — marks `damage_marked` and records `damaged_by`, emits `NonCombatDamageDealt` event.
- **Mana cost (cast)**: PASS — `{4}{R}` matches oracle.
- **Activated ability cost**: PASS — `{3}{R}` matches oracle.
- **No tap cost, no sorcery-speed restriction**: PASS — `requires_tap: false`, `sorcery_speed_only: false`.

### Test coverage

- Basic milling and player damage: `tier15_cards.rs:316` — TESTED
- `damaged_by` tracking when targeting a creature: `tier15_cards.rs:354` — TESTED
- Fizzle when target is no longer on battlefield: `tier15_cards.rs:383` — TESTED
- Fewer than 3 cards in library at resolution: NOT TESTED
- All milled cards have MV 0 (no damage): NOT TESTED
- Planeswalker as target: NOT TESTED
- Double-faced card mana value: NOT TESTED
- Target with hexproof (self vs. opponent): NOT TESTED
