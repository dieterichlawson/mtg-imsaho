# Audit: Purify the Grave

## Official Oracle
- **Name:** Purify the Grave
- **Cost:** {W}
- **Type:** Instant
- **Oracle Text:** Exile target card from a graveyard.\nFlashback {W}
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {W} — OK
- **Type:** Instant — OK
- **Oracle Text:** Matches — OK
- **Flashback Cost:** {W} — OK
- **P/T:** N/A — OK

## Issues
1. **ISSUE: No targeting — auto-selects from opponent's graveyard first**: The card says "target card from a graveyard" (any graveyard, player chooses). The implementation uses TargetRequirement::None and auto-selects the first card from the opponent's graveyard. This means:
   - The player has no choice of which card to exile
   - It always prioritizes opponent's graveyard over own graveyard
   - The card should target ANY card in ANY graveyard, not auto-select
   - Comment in code acknowledges this is "an approximation"

## Verdict: FAIL
- **Targeting is completely bypassed** — the player should choose which card from which graveyard to exile

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Exile target card from a graveyard. Flashback {W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Scryfall type line**: Instant
**Status**: PASS

Previous targeting issue has been fixed. The implementation now uses `TargetRequirement::GraveyardCard` to allow the player to choose a target card from any graveyard.

Verified correct:
- Mana cost: {W} -- matches
- Type: Instant -- matches
- Oracle text: matches
- Flashback cost: {W} -- matches
- `on_resolve`: exiles target card, then calls `move_spell_after_resolve(object_id)` -- correct
- No anti-patterns detected: uses `move_spell_after_resolve` (not `move_object` to graveyard)
- Tests found in `mtg-engine/tests/tier11_cards.rs`

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Exile target card from a graveyard. Flashback {W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

Card data is correct: name, mana cost ({W}), type (Instant), flashback cost ({W}).

on_resolve correctly exiles the target card and calls move_spell_after_resolve(object_id).

Targeting uses TargetRequirement::GraveyardCard which allows targeting any card in any graveyard, matching "target card from a graveyard."

Tests in tier11_cards.rs cover core functionality (exile from graveyard, flashback cost exists). No anti-patterns found.

---

## Audit (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
- **Name:** Purify the Grave
- **Mana Cost:** {W}
- **Type:** Instant
- **Oracle Text:** Exile target card from a graveyard. / Flashback {W}
- **Keywords:** Flashback

### Implementation: `mtg-engine/src/cards/isd/purify_the_grave.rs`

### Checklist

| Field | Oracle | Implementation | Verdict |
|---|---|---|---|
| Name | Purify the Grave | `"Purify the Grave"` | MATCH |
| Mana cost | {W} | `ManaCost::new(vec![ManaSymbol::Colored(Color::White)])` | MATCH |
| Type | Instant | `vec![CardType::Instant]` | MATCH |
| Oracle text | "Exile target card from a graveyard.\nFlashback {W}" | `"Exile target card from a graveyard.\nFlashback {W}"` | MATCH |
| Targeting | "target card from a graveyard" | `TargetRequirement::GraveyardCard` (any card in any graveyard) | MATCH |
| Effect | Exile target card | `state.move_object(*target_id, Zone::Exile)` | MATCH |
| Flashback cost | {W} | `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Colored(Color::White)]))` | MATCH |
| Keywords | Flashback | `keywords: vec![]` | OK — no `Flashback` variant in `Keyword` enum; flashback modeled via `flashback_cost` field |
| Spell cleanup | Flashback → exile, otherwise → graveyard | `move_spell_after_resolve` checks `cast_with_flashback` flag | MATCH |

### Tests (`mtg-engine/tests/tier11_cards.rs`)
- `purify_the_grave_exiles_card_from_graveyard` — puts a card in opponent's graveyard, casts Purify, verifies card moved to Exile zone. PASS.
- `purify_the_grave_has_flashback` — verifies `flashback_cost` is `Some`. PASS.

### Issues Found
None. All fields match oracle text. Targeting, exile mechanic, and flashback cost are correctly implemented.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Exile target card from a graveyard. / Flashback {W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found. Card data correct: cost {W}, Instant type, flashback cost {W}. Target requirement is `GraveyardCard` which correctly targets any card in any graveyard. Resolution moves the target to Exile zone. `move_spell_after_resolve` handles flashback exile. Simple and correct implementation.
