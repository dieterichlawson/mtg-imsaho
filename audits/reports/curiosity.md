# Audit: Curiosity

## Oracle Text (Scryfall, cached 2026-04-01)
> Enchant creature
> Whenever enchanted creature deals damage to an opponent, you may draw a card.

## Card Data
- **Mana Cost:** {U}
- **Type:** Enchantment -- Aura
- **Keywords:** Enchant

## Implementation File
`mtg-engine/src/cards/isd/curiosity.rs`

---

## Issues Found

### Issue 1 (Medium): "You may" optional draw is not optional

**Oracle text:** "you **may** draw a card"

**Code (line 65):**
```rust
// "You may draw a card" — auto-draw in 2-player (always beneficial).
crate::engine::draw_cards(state, controller, 1);
```

The implementation always draws a card unconditionally. The oracle text says "you may draw a card", making the draw optional. While drawing is almost always beneficial, there are edge cases where a player would decline (e.g., near-empty library to avoid decking, or in response to an opponent's Notion Thief effect). The code comment acknowledges this is a deliberate simplification ("auto-draw in 2-player (always beneficial)"), but it is technically a rules deviation.

**Severity: Medium** -- Incorrect in corner cases (self-mill / decking).

### Issue 2 (Low): LLM hint omits "you may"

**Oracle text:** "you **may** draw a card"

**LLM hint (`mtg-player/src/llm.rs`, line 136):**
```
- Curiosity ({U} aura): Whenever enchanted creature deals damage to an opponent, draw a card. Put on evasive creatures!
```

The hint says "draw a card" rather than "you may draw a card", which is consistent with the forced-draw implementation but misrepresents the oracle text.

**Severity: Low** -- Cosmetic / hint accuracy.

---

## Verified Correct

1. **Trigger kind:** Uses `TriggerKind::AnyDamageToPlayer`, which fires on both combat and non-combat damage. This matches the oracle text ("deals damage", not "deals combat damage") and the ruling: "Any damage dealt by the enchanted creature to an opponent will cause Curiosity to trigger, not just combat damage."

2. **Enchanted creature check:** The handler correctly verifies `source_id == attached_to` (line 56), ensuring only the enchanted creature's damage triggers the ability.

3. **Opponent check:** The handler checks `damaged_player == controller` and returns if true (line 61), correctly ensuring it only triggers on damage to an opponent, not the controller.

4. **Aura mechanics:** Uses `resolve_aura` helper for attachment, `TargetRequirement::Creature` for targeting. Correct.

5. **Card data:** Mana cost ({U}), types (Enchantment, Aura subtype), oracle text all match Scryfall.

6. **Draw count:** Draws exactly 1 card per trigger, matching the ruling: "You draw one card each time the enchanted creature deals damage to an opponent, no matter how much damage it deals."

## Test Coverage

One test exists in `mtg-engine/tests/tier6_cards.rs`:
- `curiosity_draw_on_enchanted_creature_combat_damage` -- verifies draw on combat damage to opponent.

**Missing test coverage:**
- Non-combat damage trigger (e.g., a "ping" ability dealing damage to an opponent)
- No trigger when enchanted creature damages its own controller
- No trigger when a different creature deals damage

---

## Summary

| # | Issue | Severity |
|---|-------|----------|
| 1 | "You may" draw is forced, not optional | Medium |
| 2 | LLM hint omits "you may" | Low |

Overall the implementation is functionally solid for typical gameplay. The main deviation is the non-optional draw, which matters in self-decking scenarios.

## Audit — 2026-04-02

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Enchant creature
Whenever enchanted creature deals damage to an opponent, you may draw a card.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues

The major issue from the prior audit (forced draw) has been fixed:

1. **"You may" draw is now optional: FIXED.** Lines 66-73 present a `YesNo` choice to the player via `AwaitingAction::ResolutionChoice` with description "Curiosity: draw a card?". The `on_yes_no_choice` handler (lines 76-84) only draws if `yes` is chosen. This correctly implements the "you may" clause.

2. **Card data correct.** Cost `{U}` (line 17), types Enchantment with Aura subtype (lines 19-21), oracle text matches (line 24).

3. **Trigger correctly scoped.** `TriggerKind::AnyDamageToPlayer` fires on both combat and non-combat damage (matching oracle "deals damage", not "deals combat damage"). Ruling confirms: "Any damage dealt by the enchanted creature to an opponent will cause Curiosity to trigger, not just combat damage."

4. **Enchanted creature check correct.** `source_id != attached_to` guard at line 57.

5. **Opponent check correct.** `damaged_player == controller` guard at line 62 ensures it only triggers on opponents, not the controller.

### Tricky interactions checked
- Curiosity on an opponent's creature: the controller check uses `aura.controller` (line 61), so it correctly checks opponents of Curiosity's controller, not the creature's controller.
- Draw count: exactly 1 card per trigger (line 82), matching ruling.
- Self-decking: now handled correctly since player can decline the draw.

### Test coverage
- `curiosity_draw_on_enchanted_creature_combat_damage` -- combat damage trigger.
- Missing: non-combat damage trigger, no-trigger on own-controller damage, no-trigger on different creature's damage.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Enchant creature\nWhenever enchanted creature deals damage to an opponent, you may draw a card.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found. Card data (name, cost {U}, types, subtypes, oracle text) all match. Trigger on AnyDamageToPlayer correctly checks that the source is the enchanted creature and that the damaged player is an opponent (not the controller). The "may" is properly implemented as a YesNo choice. Draw is handled correctly.

## Audit — 2026-04-02 20:45

**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/49/curiosity, cached 2026-04-01)
**Oracle text**: Enchant creature
Whenever enchanted creature deals damage to an opponent, you may draw a card.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues

No issues found. All prior issues have been resolved:

1. **Card data matches oracle exactly.** Name "Curiosity", cost `{U}` (line 17: `ManaSymbol::Colored(Color::Blue)`), types `Enchantment` with subtype `Aura` (lines 19-21), oracle text on line 24 matches Scryfall verbatim.

2. **"You may" draw is correctly optional.** Lines 66-73 present a `YesNo` choice via `AwaitingAction::ResolutionChoice`. The `on_yes_no_choice` handler (lines 76-84) only draws when `yes == true`. This correctly implements the optional "you may" clause.

3. **Trigger type is correct.** Uses `TriggerKind::AnyDamageToPlayer`, which the engine dispatches for both `CombatDamageDealt` and `NonCombatDamageDealt` events (verified in `triggers.rs` lines 542-559 and 566-591). Matches oracle "deals damage" (not "deals combat damage") and ruling: "Any damage dealt by the enchanted creature to an opponent will cause Curiosity to trigger, not just combat damage."

4. **Enchanted creature guard correct.** Line 57: `if source_id != attached_to { return; }` ensures only the enchanted creature's damage triggers the ability.

5. **Opponent guard correct.** Line 61-62: `let controller = aura.controller; if damaged_player == controller { return; }` correctly uses the aura's controller (not the creature's controller) per ruling: "'You' refers to the controller of Curiosity, which may be different from the controller of the enchanted creature."

6. **Aura resolution correct.** Uses `resolve_aura` helper (line 43) with `TargetRequirement::Creature` (line 39).

7. **LLM hint correct.** `mtg-player/src/llm.rs` line 136 now includes "you may draw a card", matching oracle text.

### Tricky interactions checked (min 3)

1. **Curiosity enchanting an opponent's creature**: The controller check at line 61 uses `aura.controller`, not the creature's controller. Per ruling, if you control Curiosity on an opponent's creature, it triggers when that creature damages one of YOUR opponents (not you). The `damaged_player == controller` check correctly prevents triggering when the creature damages the aura's controller.

2. **Non-combat damage triggers Curiosity**: The engine's trigger system (verified in `triggers.rs`) dispatches `AnyDamageToPlayer` for `NonCombatDamageDealt` events as well as `CombatDamageDealt`, so a ping ability on the enchanted creature would correctly trigger Curiosity. Matches ruling: "Any damage dealt by the enchanted creature to an opponent will cause Curiosity to trigger, not just combat damage."

3. **Draw count per trigger**: The implementation draws exactly 1 card (line 82: `draw_cards(state, controller, 1)`), regardless of damage amount. Matches ruling: "You draw one card each time the enchanted creature deals damage to an opponent, no matter how much damage it deals."

4. **Self-decking / declining draw**: Since the draw is optional (YesNo choice), a player can decline to avoid decking themselves when their library is nearly empty. Verified by the `curiosity_decline_draw` test.

5. **Planeswalker/battle damage**: The trigger system only fires `AnyDamageToPlayer` for `DamageTarget::Player`, never for planeswalker or battle damage. Matches ruling: "Curiosity doesn't trigger if the enchanted creature deals damage to a planeswalker or to a battle."

### Test coverage

Two tests in `mtg-engine/tests/tier6_cards.rs`:
- `curiosity_draw_on_enchanted_creature_combat_damage` -- combat damage to opponent, player accepts draw. Passes.
- `curiosity_decline_draw` -- combat damage to opponent, player declines draw. Passes.

**Missing test coverage (minor):**
- Non-combat damage trigger scenario
- No-trigger when enchanted creature damages its own controller
- No-trigger when a different creature deals damage
