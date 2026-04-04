## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**:
Front face (Garruk Relentless):
When Garruk Relentless has two or fewer loyalty counters on him, transform him.
0: Garruk deals 3 damage to target creature. That creature deals damage equal to its power to him.
0: Create a 2/2 green Wolf creature token.

Back face (Garruk, the Veil-Cursed):
+1: Create a 1/1 black Wolf creature token with deathtouch.
−1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle.
−3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.

**Type line**: Legendary Planeswalker — Garruk (front) / Legendary Planeswalker — Garruk (back)
**Status**: ISSUE

### Code issues

- **`abilities_activated_this_turn` never cleared between turns** — `mtg-engine/src/engine.rs:1942`, `mtg-engine/src/engine.rs:3006-3061`
  - Oracle text says: Garruk's loyalty abilities are activatable once per turn, implying they reset each turn. The rulings confirm "You can't activate a loyalty ability of Garruk Relentless and **later that turn**..." — the restriction is per-turn, not permanent.
  - Code does: When a loyalty ability activates (`engine.rs:1942`), the sentinel `999` is inserted into `obj.abilities_activated_this_turn`. The legal-actions generator (`engine.rs:415`) skips loyalty abilities for any object where `abilities_activated_this_turn.contains(&999)` is true. The Cleanup step (`engine.rs:3006-3061`) clears damage, end-of-turn effects, regeneration shields, and mana pools, but never clears `abilities_activated_this_turn`. The Untap step resets `land_plays_remaining` and `summoning_sick` but also never clears `abilities_activated_this_turn`. There is no code path anywhere in the engine that calls `.clear()` on this field. After Garruk uses any loyalty ability on Turn N, all future turns still see `already_used = true` and the engine never generates any `ActivateLoyaltyAbility` action for Garruk again — permanently locking all three back-face abilities and both front-face abilities for the remainder of the game.

- **`is_legendary` not set in `on_resolve`** — `mtg-engine/src/cards/isd/garruk_relentless.rs:313-321`
  - Oracle text says: `"Legendary Planeswalker — Garruk"` — Garruk has the Legendary supertype, making the Legend Rule (CR 704.5k) apply.
  - Code does: `on_resolve` sets `obj.card_types = vec![CardType::Planeswalker]` but never sets `obj.is_legendary = true`. The Legend Rule SBA in `sba.rs:290` gates entirely on `obj.is_legendary`: `if obj.zone == Zone::Battlefield && obj.is_legendary { ... }`. Because `is_legendary` is never set to `true` for Garruk, two copies can coexist on the same player's battlefield indefinitely. Other legendary cards in the codebase (Geist of Saint Traft, Grimgrin, Mikaeus, Grimoire of the Dead) all explicitly set `obj.is_legendary = true` in their ETB handlers; Garruk's implementation omits this.

### Tricky interactions checked

- **State-triggered ability (CR 603.8) — trigger at ≤2 loyalty**: PASS. `sba.rs` correctly checks `!o.is_transformed && !o.state_trigger_on_stack && loyalty <= 2`, pushes `PendingTrigger::StateTriggered`, sets `state_trigger_on_stack = true`. On resolution, clears flag and calls `on_state_trigger` which sets `is_transformed = true`. Subsequent SBA passes fail the `!o.is_transformed` guard, preventing re-triggering. Correctly implements the ruling: "it can't retrigger while that ability is on the stack."
- **Loyalty counters preserved through transform**: PASS. `on_state_trigger` only sets `is_transformed = true` and changes `name`; it does not call `move_object` and does not clear `obj.counters`. Consistent with ruling: "You don't add or remove loyalty counters from Garruk Relentless when he transforms."
- **Once-per-turn loyalty rule across transform (same object)**: PASS (for the within-turn case). The sentinel `999` is stored on the same `GameObject` whose `ObjectId` is unchanged by transform. If a front-face ability is used, `abilities_activated_this_turn` gets `999`, and the back-face abilities are also blocked for that turn. This correctly implements: "You can't activate a loyalty ability of Garruk Relentless and later that turn after he transforms activate a loyalty ability of Garruk, the Veil-Cursed." However, this interacts with the never-cleared bug above — once any ability is used in any turn, ALL subsequent turns are also blocked, which is wrong.
- **Once-per-turn loyalty rule reset at start of new turn**: FAIL. See Issue 1 above — `abilities_activated_this_turn` is never cleared.
- **Legend Rule applies to two copies**: FAIL. See Issue 2 above — `is_legendary` never set.
- **Ability 0 (fight) — 3 damage to creature + creature's power to Garruk**: PASS. Code correctly marks `damage_marked += 3` on the target and emits `NonCombatDamageDealt`; then removes `target_power` loyalty counters from Garruk via `loyalty.saturating_sub(remove)` and emits a second `NonCombatDamageDealt` event. The creature detecting ≤2 loyalty after the fight causes the state trigger SBA to fire correctly.
- **Ability 0 (fight) — fight against a 0-power creature**: PASS. Code guards with `if target_power > 0` before removing loyalty; a 0-power creature deals no damage back.
- **Back face −1 — "If you do" conditional on tutor**: PASS. When no creatures are controlled (`creatures.is_empty()`), neither sacrifice nor search occurs. Only after a successful sacrifice does the library search proceed. Correctly models the "If you do" clause.
- **Back face −1 — mandatory sacrifice**: PASS. When exactly one creature is present, it is auto-sacrificed. When multiple exist, the choice is presented with `optional: false`, forcing the player to pick one.
- **Back face −3 — X snapshot at resolution**: PASS. `x` is computed from `objects_in_zone(Zone::Graveyard)` at ability resolution time, then fixed in `until_end_of_turn_effects` entries (`power_mod: x`). Correctly implements ruling: "The number of creature cards in your graveyard is counted when the third ability resolves. Once the ability resolves, the bonus doesn't change."
- **Back face −3 — snapshot of affected creatures**: PASS. The creature list is collected from the battlefield at resolution time; each collected creature gets its own `until_end_of_turn_effects` entry. Correctly implements ruling: "Only creatures you control when the third ability resolves will receive the bonus. Creatures that enter … later in the turn won't be affected."
- **Planeswalker dies at 0 loyalty**: PASS. `on_resolve` sets `obj.card_types = vec![CardType::Planeswalker]`, and the SBA in `sba.rs:216-245` checks `o.card_types.contains(&CardType::Planeswalker) && loyalty == 0`. Both front and back faces are covered since the `card_types` on the object is set once and persists through transform.
- **Mana cost / type / subtypes / starting loyalty**: PASS. `ManaCost` is `{3}{G}` (correct), `card_types: [Planeswalker]`, `supertypes: [Legendary]`, `subtypes: ["Garruk"]`, `starting_loyalty: Some(3)` all match oracle text.
- **Wolf token attributes (front 0-ability)**: PASS. `create_token_with_subtypes` called with `name="Wolf"`, `2, 2`, `colors=[Green]`, `card_types=[Creature]`, `keywords=[]`, `subtypes=["Wolf"]`. Matches oracle "2/2 green Wolf creature token."
- **Wolf token attributes (back +1 ability)**: PASS. `create_token_with_subtypes` called with `1, 1`, `colors=[Black]`, `keywords=[Deathtouch]`, `subtypes=["Wolf"]`. Matches oracle "1/1 black Wolf creature token with deathtouch."

### Test coverage

- State-triggered transform when loyalty ≤ 2: `tier15_cards.rs:2256` — TESTED
- Creating 2/2 Wolf token (front face ability 1): `tier15_cards.rs:2234` — TESTED
- Creating 1/1 black deathtouch Wolf (back face +1): `tier15_cards.rs:2281` — TESTED
- Back face −1 auto-sacrifice single creature + tutor: `tier15_cards.rs:2309` — TESTED
- Back face −1 presents sacrifice choice with multiple creatures: `tier15_cards.rs:2347` — TESTED
- Back face −1 shuffles library after tutor: `tier15_cards.rs:2391` — TESTED
- Back face −3 +X/+X trample (snapshot X and creature list): `tier15_cards.rs:2439` — TESTED
- Back face loyalty abilities shown when transformed: `tier15_cards.rs:2479` — TESTED
- Front face ability 0 (fight — 3 damage to creature, creature's power to Garruk): NOT TESTED
- Loyalty abilities reset each turn (once-per-turn clears at turn start): NOT TESTED
- Legend Rule applying to two copies of Garruk: NOT TESTED
- Loyalty ability once-per-turn blocked across transform (within same turn): NOT TESTED
- Garruk dies at 0 loyalty (SBA): NOT TESTED
- "If you do" — no sacrifice when no creatures controlled: NOT TESTED
