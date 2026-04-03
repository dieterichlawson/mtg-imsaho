# Audit: Curse of Oblivion

## Scryfall Reference
- **Name:** Curse of Oblivion
- **Cost:** {3}{B}
- **Type:** Enchantment -- Aura Curse
- **Oracle:** Enchant player. At the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard.
- **P/T:** N/A
- **Keywords:** Enchant

## Implementation: `curse_of_oblivion.rs`
- **Name:** Curse of Oblivion -- CORRECT
- **Cost:** {3}{B} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Subtypes:** ["Aura", "Curse"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Target:** TargetRequirement::PlayerOnly -- CORRECT
- **Trigger:** Upkeep -- CORRECT
- **Behavior:** Cursed player exiles two cards from graveyard at their upkeep -- CORRECT
- **Choice mechanism:** Presents choice if >2 cards, auto-exiles if <=2 -- CORRECT

## Issues
None

---

## Re-Audit (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
> Enchant player
> At the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard.

**Ruling [2011-09-22]:** If the enchanted player has only one card in their graveyard, they exile that card.

### Card Data Verification
All fields verified correct: name, mana cost ({3}{B}), types (Enchantment -- Aura Curse), oracle text, `TargetRequirement::PlayerOnly`, `TriggerKind::Upkeep`.

### Trigger Logic Verification
- Correctly gates on `state.active_player != cursed_player` (fires only on enchanted player's upkeep).
- Empty graveyard: returns early (correct).
- 1-2 cards in graveyard: auto-exiles all (correct per ruling).
- 3+ cards: presents sequential player choices via `ExileCurseOfOblivion { remaining: 1 }` (correct).

### Issue 1 (Minor): Hardcoded ObjectId(0) for second exile choice source

In `mtg-engine/src/engine.rs` line 2088, the second exile prompt uses a dummy source ID:

```rust
source: crate::ids::ObjectId(0), // curse source
```

The first choice (in `curse_of_oblivion.rs:86`) correctly uses `source: self_id`, but the curse's object ID is not stored in `PendingEffect::ExileCurseOfOblivion`, so it is lost when the engine presents the second choice. Could matter if source tracking is used for UI or interactions.

**Severity:** Low.

### Issue 2 (Minor): Test coverage gap

Test `curse_of_oblivion_exiles_from_graveyard` in `mtg-engine/tests/tier7_cards.rs` only covers the auto-exile path (<=2 cards). No test covers the >2 cards player-choice path.

### LLM Player
No special-case handling in `mtg-player/src/llm.rs`. The generic `ResolutionChoice`/`ChooseTarget` path should handle this card.

### Verdict
**PASS** -- Implementation correctly matches oracle text and ruling. Two minor issues noted but no functional correctness bugs.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Enchant player\nAt the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard.
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
No issues found. Card data matches: name, cost {3}{B}, subtypes Aura Curse, oracle text. Upkeep trigger correctly checks active_player == cursed_player. Handles edge cases: empty graveyard (returns early), 1-2 cards (exiles all without choice), 3+ cards (presents player choice). Per ruling, if only one card in graveyard, it is exiled — the <= 2 branch handles this correctly.

## Audit — 2026-04-02 20:45
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Enchant player
At the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard.
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
None. All card data fields match oracle text exactly. Mana cost {3}{B}, card type Enchantment, subtypes Aura and Curse, oracle text verbatim match. `TargetRequirement::PlayerOnly` correct for "Enchant player". `TriggerKind::Upkeep` declared. `resolve_curse` helper correctly attaches to target player. No anti-patterns found (no `move_object(Zone::Graveyard)` misuse, no incorrect damage types).

### Tricky interactions checked (min 3)
1. **Empty graveyard**: `on_upkeep` returns early if no cards in graveyard — correct, no crash or spurious choice prompt.
2. **Exactly one card in graveyard**: Falls into the `<= 2` branch, exiles that single card automatically — matches ruling [2011-09-22] which states "they exile that card."
3. **Curse on non-active player's upkeep**: Guard `state.active_player != cursed_player` prevents the trigger from firing on the wrong player's upkeep — correct.
4. **Second exile choice with graveyard emptied**: In the `ExileCurseOfOblivion` handler in engine.rs, after exiling the first chosen card, if `remaining > 0` but graveyard is now empty, the `if !gy_cards.is_empty()` check prevents presenting an impossible choice — correct.

### Test coverage
One test in `mtg-engine/tests/tier7_cards.rs`: `curse_of_oblivion_exiles_from_graveyard` covers the auto-exile path (exactly 2 cards in graveyard). No test for the >2 cards player-choice path. No test for empty graveyard or single-card edge case.
