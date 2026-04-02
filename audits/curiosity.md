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
