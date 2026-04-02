# Audit: Curse of the Bloody Tome

## Oracle Text (Scryfall)

> Enchant player
> At the beginning of enchanted player's upkeep, that player mills two cards.

- **Mana Cost:** {2}{U}
- **Type Line:** Enchantment -- Aura Curse
- **Keywords:** Enchant, Mill

## Implementation File

`mtg-engine/src/cards/isd/curse_of_the_bloody_tome.rs`

## Checklist

### Card Data
- [x] **Name:** Correct (`"Curse of the Bloody Tome"`)
- [x] **Mana cost:** `Generic(2), Colored(Blue)` matches `{2}{U}`
- [x] **Card types:** `[Enchantment]` -- correct
- [x] **Subtypes:** `["Aura", "Curse"]` -- correct
- [x] **Oracle text:** Matches Scryfall verbatim
- [x] **Power/toughness:** `None` -- correct (not a creature)

### Trigger Setup
- [x] **TriggerKind:** `TriggerKind::Upkeep` -- correct; the oracle says "At the beginning of enchanted player's upkeep"
- [x] **Trigger description:** `"enchanted player mills two cards"` -- reasonable summary

### Target Requirement
- [x] **`target_requirement`:** `TargetRequirement::PlayerOnly` -- correct for "Enchant player"

### on_resolve
- [x] Delegates to `crate::cards::helpers::resolve_curse(state, object_id, targets)` which moves the curse to the battlefield and sets `attached_to_player` -- correct pattern for Aura Curses

### on_upkeep Logic
- [x] **Zone check:** Only fires if the curse is on the battlefield (`o.zone == Zone::Battlefield`) -- correct
- [x] **Attached player check:** Reads `attached_to_player` and returns early if `None` -- correct
- [x] **Active player gate:** `state.active_player != cursed_player` returns early -- correctly ensures the trigger only fires on the enchanted player's own upkeep
- [x] **Mill amount:** `mill_cards(state, cursed_player, 2)` -- matches oracle ("mills two cards")
- [x] **Mill implementation:** `engine::mill_cards` moves top N cards from library to graveyard, handles empty library gracefully (breaks loop) -- consistent with ruling: "If the enchanted player has only one card in their library, they put that card into their graveyard."

### Anti-Patterns Check
- [x] No hardcoded player IDs
- [x] No incorrect zone checks
- [x] No missing early returns
- [x] Logging present after mill

### Keywords
- [ ] **Missing keywords:** The `keywords` field is `vec![]` but Scryfall lists `Enchant` and `Mill` as keywords. This is a minor metadata omission -- the functionality is correctly implemented through `target_requirement` (for Enchant) and `mill_cards` (for Mill). This is consistent with how other cards in the codebase handle these keywords (they are implicit in the behavior rather than listed in the keywords vec).

### Tests
- [x] **Test exists:** `mtg-engine/tests/tier7_cards.rs::curse_of_bloody_tome_mills_on_upkeep` -- sets up curse attached to P1, triggers upkeep on P1's turn, asserts 2 cards milled to graveyard

### LLM Integration
- [x] No special-casing found in `mtg-player/src/llm.rs` -- none needed

## Issues Found

**No functional issues.** The implementation correctly matches the oracle text.

**Minor (cosmetic):** The `keywords` vec is empty while Scryfall lists `Enchant` and `Mill`. This has no gameplay impact as both behaviors are properly implemented through code. This pattern is consistent across the codebase.

## Verdict

**PASS** -- Implementation is correct and complete.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Enchant player\nAt the beginning of enchanted player's upkeep, that player mills two cards.
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
No issues found. Card data matches: name, cost {2}{U}, subtypes Aura Curse, oracle text. Upkeep trigger correctly checks active_player == cursed_player. Calls mill_cards(state, cursed_player, 2) which correctly implements milling two cards.
