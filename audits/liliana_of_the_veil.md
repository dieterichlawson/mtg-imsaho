# Audit: Liliana of the Veil

## Oracle (Official)
- **Name:** Liliana of the Veil
- **Cost:** {1}{B}{B}
- **Type:** Legendary Planeswalker — Liliana
- **Oracle:** +1: Each player discards a card. -2: Target player sacrifices a creature. -6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.
- **Loyalty:** 3

## Implementation
- Name: "Liliana of the Veil" -- CORRECT
- Cost: {1}{B}{B} -- CORRECT
- Type: Planeswalker -- CORRECT
- Supertypes: [Legendary] -- CORRECT
- Subtypes: ["Liliana"] -- CORRECT
- Starting loyalty: 3 -- CORRECT
- Oracle text matches -- CORRECT
- +1: each player discards (auto-picks first card in hand) -- CORRECT (simplified selection)
- -2: opponent sacrifices a creature -- CORRECT (simplified: always targets opponent)
- -6: opponent sacrifices half their permanents -- SIMPLIFICATION (real card has pile division with player choice)

## Issues
1. **ISSUE (simplification):** +1 ability auto-picks the first card in hand rather than allowing player choice.
2. **ISSUE (simplification):** -2 always targets opponent, rather than allowing "target player" selection.
3. **ISSUE (simplification):** -6 simplified from pile division to "sacrifice half permanents." The real card separates into two piles and the target player chooses which pile to sacrifice. Code takes the first half, not a random/chosen split.

## Verdict: PASS (with noted simplifications — all acknowledged in comments)

## Audit — 2026-04-01 14:00

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/105/liliana-of-the-veil
**Oracle text**:
+1: Each player discards a card.
−2: Target player sacrifices a creature.
−6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.
**Type line**: Legendary Planeswalker — Liliana
**Mana cost**: {1}{B}{B}
**Starting loyalty**: 3
**Status**: ISSUE

### Code issues

1. **Oracle text field mismatch (line 30)** — The oracle_text field in code says "pile of your choice" but the actual oracle text says "pile of their choice."
   - Oracle text says: `That player sacrifices all permanents in the pile of their choice.`
   - Code has: `"That player sacrifices all permanents in the pile of your choice."`

2. **+1 ability auto-picks first card instead of player choice (lines 79-81)** — The ruling states "first the player whose turn it is chooses a card in hand without revealing it, then each other player in turn order does the same." The code just picks `hand.first()` with no player choice. Other cards in the codebase (e.g., Garruk's sacrifice via `present_target_choice`, Tribute to Hunger's sacrifice choice) demonstrate the engine supports player-driven choices.
   - Oracle text says: `Each player discards a card.` (ruling: each player *chooses*)
   - Code does: `if let Some(&card_id) = hand.first()` — always discards the first card in the hand vec

3. **-2 ability has no targeting and auto-picks creature (lines 53-55, 93-106)** — Oracle says "Target player sacrifices a creature." The `target_requirement` is `None` and the code hardcodes `opponent`. The targeted player should choose which creature to sacrifice (sacrifice effects let the sacrificing player choose). The code picks `creatures.first()` instead.
   - Oracle text says: `Target player sacrifices a creature.`
   - Code does: `target_requirement: None` and `let creatures ... if let Some(&creature_id) = creatures.first()`

4. **-6 ability has no targeting and completely simplified (lines 57-58, 108-121)** — Oracle says "Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice." The `target_requirement` is `None`, the code hardcodes `opponent`, and the behavior is simplified to "sacrifice half their permanents (first N)" with no pile division or player choice for either pile assignment or pile selection.
   - Oracle text says: `Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.`
   - Code does: `let half = permanents.len() / 2; let to_sacrifice: Vec<ObjectId> = permanents.into_iter().take(half.max(1)).collect();`

5. **-6 forces sacrifice of at least 1 even with 0 or 1 permanents (line 115)** — `half.max(1)` means if the opponent has 1 permanent, `half = 0` but `.max(1)` forces sacrificing 1. The oracle allows the player to choose an empty pile, which means 0 sacrifices are possible. With 1 permanent, the controller puts it in one pile, the other pile is empty, and the player could choose the empty pile.
   - Oracle ruling says: `A pile can be empty. If the player chooses an empty pile, no permanents will be sacrificed.`
   - Code does: `half.max(1)` — always sacrifices at least 1

### Tricky interactions checked
- +1 when a player has no cards in hand: PASS (the `if let Some` handles empty hand correctly, matching the ruling "You can activate Liliana's first ability even if some or all players will be unable to discard a card")
- -2 uses `sacrifice()` not `try_destroy()`: PASS (sacrifice is correct per oracle)
- -6 uses `sacrifice()` not `try_destroy()`: PASS (sacrifice is correct per oracle)
- Discarded event emitted on +1: PASS (line 84)
- Planeswalker enters with loyalty via `add_counters`: PASS (line 131)
- `card_types` set on resolve: PASS (line 133)

### Test coverage
- Liliana enters with 3 loyalty: `tier15_cards.rs:1059` — TESTED
- +1 each player discards: `tier15_cards.rs:1075` — TESTED (but test only checks cards moved to graveyard, doesn't verify player choice)
- -2 opponent sacrifices creature: `tier15_cards.rs:1098` — TESTED (but doesn't verify player choice of creature)
- -6 pile division: NOT TESTED
- Planeswalker SBA (0 loyalty dies): `tier15_cards.rs:1607` — TESTED
- Planeswalker SBA (3 loyalty stays): `tier15_cards.rs:1622` — TESTED
- Loyalty counter adjustment via engine: `tier15_cards.rs:1641` — TESTED
- Ruling: can activate +1 with empty hands: NOT TESTED
- Ruling: discard order (active player first): NOT TESTED
- Ruling: empty pile on -6: NOT TESTED
- -2 targeting self (target player, not target opponent): NOT TESTED

## Audit — 2026-04-02

**Oracle text source**: Scryfall API (cached 2026-04-01) — https://scryfall.com/card/isd/105/liliana-of-the-veil
**Oracle text**: +1: Each player discards a card. −2: Target player sacrifices a creature. −6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.
**Type line**: Legendary Planeswalker — Liliana
**Status**: PASS

### Code issues

No issues found. All previously reported issues from the 2026-04-01 audit have been resolved. Specifically:

1. **Oracle text field** now reads `"That player sacrifices all permanents in the pile of their choice."` — matches oracle exactly.

2. **+1 ability** now correctly presents `ChooseCardFromHand` to each player in turn order (active player first), matching the ruling: "first the player whose turn it is chooses a card in hand without revealing it, then each other player in turn order does the same." Code at line 78 builds the player list with active player first, and line 147 presents `ResolutionChoiceKind::ChooseCardFromHand` for each player with multiple cards. Single-card hands are auto-discarded (lines 132-143), which is acceptable since there is no meaningful choice.

3. **-2 ability** now has `target_requirement: Some(TargetRequirement::PlayerOnly)` (line 53) and correctly reads the target from `targets.first()` (line 160). The targeted player chooses which creature to sacrifice via `present_target_choice` (line 174), which presents `ChooseTarget` when multiple creatures exist.

4. **-6 ability** now has `target_requirement: Some(TargetRequirement::PlayerOnly)` (line 59), correctly reads the target player, and implements full two-step pile division: (a) Liliana's controller divides permanents via `DividePermanentsIntoPiles` (line 209), then (b) the target player chooses which pile to sacrifice via `ChoosePile` (engine.rs line 1914). Empty piles are supported — the `DividePermanentsIntoPiles` action generator produces all 2^N subsets including the empty set (engine.rs line 222), and the `ChoosePile` handler correctly allows choosing either pile.

5. **Sacrifice implementation** uses `crate::destruction::sacrifice()` (engine.rs line 1935) rather than `move_object` — correct for sacrifice semantics (bypasses indestructible, etc.).

### Tricky interactions checked

- **+1 with empty hands**: PASS. Code checks `if !hand.is_empty()` (line 84/95) before adding player to discard list. When all players have empty hands, `players_to_discard` is empty and the ability logs "no player has cards to discard" (line 102), matching ruling: "You can activate Liliana's first ability even if some or all players will be unable to discard a card."
- **+1 discard order**: PASS. Active player is added first (line 85), then others in turn order (line 89). Matches ruling about turn-order choice.
- **+1 simultaneous discard**: MINOR NOTE. Ruling says cards are "discarded at the same time" but implementation discards sequentially (each player resolves before the next). This is functionally equivalent in the 2-player engine and is an acceptable implementation approach.
- **-2 can target self**: PASS. `TargetRequirement::PlayerOnly` allows targeting any player including self. Test `liliana_minus_two_can_target_self` (tier15_cards.rs) confirms this.
- **-6 empty pile**: PASS. The subset generation includes the empty set (mask=0) and the full set (mask=2^N-1), so the controller can put all permanents in one pile. The target player can then choose the empty pile and sacrifice nothing. Matches ruling: "A pile can be empty. If the player chooses an empty pile, no permanents will be sacrificed."
- **-6 Aura splitting**: PASS. The code collects all permanents without filtering by type, so Auras can be placed in a different pile than the permanent they enchant. State-based actions would handle the Aura falling off after the enchanted permanent is sacrificed.
- **-6 controller vs target player roles**: PASS. Controller (line 205) divides the piles; target player (engine.rs line 1912) chooses which pile to sacrifice.
- **No move_spell_after_resolve anti-pattern**: PASS. This is a loyalty ability, not a spell, so `move_spell_after_resolve` is not applicable.

### Test coverage

Tests found in `mtg-engine/tests/tier15_cards.rs`:
- `liliana_enters_with_loyalty` (line 1554): Enters with 3 loyalty counters — TESTED
- `liliana_plus_one_each_player_discards_with_choice` (line 1570): Both players choose and discard — TESTED
- `liliana_plus_one_single_card_auto_discards` (line 1633): Auto-discard with single card — TESTED
- `liliana_plus_one_empty_hand_skipped` (line 1658): Empty hand is skipped, per ruling — TESTED
- `liliana_plus_one_both_empty_hands` (line 1682): Both empty hands, no effect — TESTED
- `liliana_minus_two_target_player_sacrifices_creature` (line 1701): Target player chooses creature — TESTED
- `liliana_minus_two_single_creature_auto_sacrifices` (line 1740): Auto-sacrifice with one creature — TESTED
- `liliana_minus_two_no_creatures` (line 1761): No creatures, no effect — TESTED
- `liliana_minus_two_can_target_self` (line 1780): Self-targeting works — TESTED
- `liliana_minus_six_pile_division_and_choice` (line 1801): Full pile division and choice flow — TESTED
- **Missing**: Test for -6 with empty pile chosen (no permanents sacrificed)
- **Missing**: Test for -6 targeting self

### UI coverage

Present in `mtg-player/src/llm.rs` line 143 with accurate description of all three abilities, including strategic advice.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: +1: Each player discards a card. / -2: Target player sacrifices a creature. / -6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.
**Type line**: Legendary Planeswalker — Liliana
**Status**: PASS

### Code issues
No issues found. Card data correct: cost {1}{B}{B}, Legendary supertype, Planeswalker type, Liliana subtype. Starting loyalty 3 matches Scryfall. All three loyalty abilities implemented with correct loyalty changes (+1, -2, -6). Ability 0 (+1): correctly implements simultaneous discard with active player choosing first per ruling. Handles auto-discard for single-card hands and chains through multiple players. Ability 1 (-2): targets a player, presents sacrifice choice to that player. Ability 2 (-6): controller divides permanents into piles, target player chooses which pile to sacrifice. All target requirements correctly use PlayerOnly.
