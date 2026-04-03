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

## Audit — 2026-04-02 20:07

**Oracle text source**: Scryfall API (cached 2026-04-01) — https://scryfall.com/card/isd/105/liliana-of-the-veil, confirmed via live API query
**Oracle text**: +1: Each player discards a card.
−2: Target player sacrifices a creature.
−6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.
**Type line**: Legendary Planeswalker — Liliana
**Mana cost**: {1}{B}{B}
**Starting loyalty**: 3
**Status**: PASS

### Code issues
No issues found.

**Card data verification (all correct):**
- Mana cost: `Generic(1), Black, Black` matches `{1}{B}{B}`
- Card types: `[Planeswalker]` matches oracle type line
- Supertypes: `[Legendary]` matches oracle type line
- Subtypes: `["Liliana"]` matches oracle type line
- Power/toughness: `None` — correct for planeswalker
- Keywords: `[]` — correct, no keywords
- Starting loyalty: `Some(3)` matches Scryfall `loyalty: "3"`
- Triggered abilities: `[]` — correct, no triggered abilities
- Continuous effects: `[]` — correct, no static abilities

**Ability 0 (+1) verification:**
- `loyalty_change: 1` — correct
- `target_requirement: None` — correct ("each player" has no target)
- Implementation builds discard list with active player first (line 76-98), matching ruling: "first the player whose turn it is chooses a card in hand without revealing it, then each other player in turn order does the same"
- Presents `ChooseCardFromHand` when hand has 2+ cards (line 147-156)
- Auto-discards when hand has exactly 1 card (line 132-143) — correct optimization, no meaningful choice
- Skips players with empty hands (line 84-85, 95-96) — matching ruling: "You can activate Liliana's first ability even if some or all players will be unable to discard a card"
- Emits `Discarded` event (line 137-139) — correct

**Ability 1 (-2) verification:**
- `loyalty_change: -2` — correct
- `target_requirement: Some(TargetRequirement::PlayerOnly)` — correct for "target player"
- Target player read from `targets.first()` (line 160-163) — correct
- Sacrifice choice presented to `target_player` (line 174), not controller — correct per oracle ("Target player sacrifices")
- Uses `PendingEffect::SacrificeCreature` which calls `crate::destruction::sacrifice()` (engine.rs line 2424) — correct sacrifice semantics

**Ability 2 (-6) verification:**
- `loyalty_change: -6` — correct
- `target_requirement: Some(TargetRequirement::PlayerOnly)` — correct for "target player"
- Step 1: Controller divides via `DividePermanentsIntoPiles` (line 206-217) — correct, oracle says Liliana's controller separates
- Step 2: Target player chooses pile via `ChoosePile` (engine.rs line 2108-2120) — correct, oracle says "That player sacrifices all permanents in the pile of their choice"
- Empty piles supported: subset generation includes mask=0 (empty set) and mask=2^N-1 (full set) at engine.rs line 235 — matches ruling: "A pile can be empty"
- Sacrifice uses `crate::destruction::sacrifice()` (engine.rs line 2132) — correct

### Tricky interactions checked
- **+1 with all empty hands**: PASS — `players_to_discard` list is empty, logs message and returns (line 100-104). Matches ruling.
- **+1 discard order (active player first)**: PASS — Active player added at line 80-86, others at line 89-98. Matches ruling about turn-order choice.
- **+1 sequential vs simultaneous discard**: ACCEPTABLE — Ruling says "all the chosen cards are discarded at the same time" but implementation discards sequentially. In a 2-player engine with no cards that trigger on individual discards during resolution, this is functionally equivalent.
- **+1 if Liliana removed mid-resolution**: PASS — `chain_next_discard` reads `card_state` from `self_id`. Objects persist across zones, so the chaining works even if Liliana moves to graveyard between discards.
- **-2 uses sacrifice not destroy**: PASS — `PendingEffect::SacrificeCreature` correctly calls `sacrifice()`, bypassing indestructible as intended by the oracle.
- **-2 target player chooses creature**: PASS — `present_target_choice` (line 174) gives choice to `target_player`, not controller. The sacrificing player always chooses what to sacrifice per MTG rules.
- **-2 can target self**: PASS — `PlayerOnly` target requirement allows targeting any player including self.
- **-6 controller divides, target player chooses**: PASS — Division choice goes to `controller` (line 206), pile choice goes to `target_player` (engine.rs line 2109).
- **-6 empty pile (no sacrifices)**: PASS — Empty subset is a valid division. If target player chooses the empty pile, no permanents are sacrificed (engine.rs line 2129: iterates chosen_pile which is empty).
- **-6 Aura in different pile from enchanted permanent**: PASS — All permanents collected without type filtering (line 194-197). Auras can be split from their enchanted permanent. SBAs handle the Aura falling off after resolution.
- **-6 with zero permanents**: PASS — Returns early with no effect (line 199-203).
- **No move_spell_after_resolve anti-pattern**: N/A — Loyalty abilities are not spells on the stack in this engine's model.
- **No CombatDamageDealt anti-pattern**: N/A — No damage dealt by this card.
- **No dynamic_pt needed**: N/A — Planeswalker with no P/T.

### Test coverage
Tests in `mtg-engine/tests/tier15_cards.rs`:
- Enters with 3 loyalty: `liliana_enters_with_loyalty` (line 1814) — TESTED
- +1 each player chooses and discards: `liliana_plus_one_each_player_discards_with_choice` (line 1830) — TESTED
- +1 single card auto-discards: `liliana_plus_one_single_card_auto_discards` (line 1893) — TESTED
- +1 empty hand skipped (ruling): `liliana_plus_one_empty_hand_skipped` (line 1918) — TESTED
- +1 both empty hands: `liliana_plus_one_both_empty_hands` (line 1942) — TESTED
- -2 target player sacrifices creature (multiple creatures, choice): `liliana_minus_two_target_player_sacrifices_creature` (line 1961) — TESTED
- -2 single creature auto-sacrificed: `liliana_minus_two_single_creature_auto_sacrifices` (line 2000) — TESTED
- -2 no creatures: `liliana_minus_two_no_creatures` (line 2021) — TESTED
- -2 can target self: `liliana_minus_two_can_target_self` (line 2040) — TESTED
- -6 full pile division and choice: `liliana_minus_six_pile_division_and_choice` (line 2061) — TESTED
- -6 empty pile allowed (ruling): `liliana_minus_six_empty_pile_allowed` (line 2119) — TESTED
- -6 all in one pile: `liliana_minus_six_all_in_one_pile` (line 2149) — TESTED
- -6 no permanents: `liliana_minus_six_no_permanents` (line 2182) — TESTED
- -6 can target self: `liliana_minus_six_can_target_self` (line 2200) — TESTED
- Planeswalker SBA (0 loyalty dies): `planeswalker_with_zero_loyalty_dies` (line 2848) — TESTED
- Planeswalker SBA (positive loyalty survives): `planeswalker_with_loyalty_survives` (line 2864) — TESTED
- Loyalty abilities in legal actions: `loyalty_abilities_appear_in_legal_actions` (line 2882) — TESTED
- Loyalty counter adjustment: `loyalty_ability_adjusts_counters` (line 2915) — TESTED
- Ruling: simultaneous discard timing: NOT TESTED (sequential implementation is functionally equivalent in current card pool)
- Ruling: controller divides piles on -6 (not target player): Verified by test `liliana_minus_six_pile_division_and_choice` line 2086 asserting `*player == P0` — TESTED

### UI coverage
Present in `mtg-player/src/llm.rs` line 143 with accurate description of all three abilities and strategic advice.

## Audit — 2026-04-02 20:13

**Oracle text source**: Oracle cache (Scryfall API, cached 2026-04-01) — https://scryfall.com/card/isd/105/liliana-of-the-veil
**Oracle text**:
+1: Each player discards a card.
−2: Target player sacrifices a creature.
−6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.
**Type line**: Legendary Planeswalker — Liliana
**Mana cost**: {1}{B}{B}
**Starting loyalty**: 3
**Status**: PASS

### Code issues
No issues found.

All card data matches oracle text exactly:
- Mana cost: `Generic(1), Colored(Black), Colored(Black)` matches `{1}{B}{B}`
- Card types: `vec![CardType::Planeswalker]` matches type line
- Supertypes: `vec![Supertype::Legendary]` matches type line
- Subtypes: `vec!["Liliana".into()]` matches type line
- Starting loyalty: `Some(3)` matches Scryfall loyalty
- Oracle text field: matches oracle verbatim (including "their choice" wording)

All three abilities correctly implemented:
- +1: No target (`target_requirement: None`), each player discards with active player choosing first, auto-discard for single-card hands, skip for empty hands
- -2: `TargetRequirement::PlayerOnly`, target player chooses which creature to sacrifice via `present_target_choice`, uses `PendingEffect::SacrificeCreature` which calls `crate::destruction::sacrifice()`
- -6: `TargetRequirement::PlayerOnly`, controller divides via `DividePermanentsIntoPiles`, target player chooses pile via `ChoosePile`, uses `crate::destruction::sacrifice()` for each permanent in chosen pile

### Tricky interactions checked
- **+1 "each player" is not targeted**: PASS — `target_requirement: None` on ability_index 0 (line 47). Oracle says "Each player discards a card" which does not use the word "target."
- **+1 discard order matches ruling**: PASS — Active player added first (line 80-86), then others in turn order (line 89-98). Ruling: "first the player whose turn it is chooses a card in hand without revealing it, then each other player in turn order does the same."
- **+1 can activate with all empty hands**: PASS — When `players_to_discard` is empty, logs message and returns (line 100-104). Ruling: "You can activate Liliana's first ability even if some or all players will be unable to discard a card."
- **-2 "sacrifice" not "destroy"**: PASS — Uses `PendingEffect::SacrificeCreature` which calls `sacrifice()` (engine.rs line 2424), not `try_destroy()`. Sacrifice bypasses indestructible as intended.
- **-2 target player chooses, not controller**: PASS — `present_target_choice` passes `target_player` as the choosing player (line 174-176), not `controller`.
- **-6 controller divides, target player chooses**: PASS — `DividePermanentsIntoPiles` presented to `controller` (line 206), `ChoosePile` presented to `target_player` (engine.rs line 2109).
- **-6 empty pile allowed**: PASS — Subset generation includes mask=0 (empty) and mask=2^N-1 (all) at engine.rs line 235. If target player chooses empty pile, the for loop at engine.rs line 2129 iterates over an empty slice. Ruling: "A pile can be empty. If the player chooses an empty pile, no permanents will be sacrificed."
- **-6 all permanents included (including non-creatures)**: PASS — `objects_in_zone(Zone::Battlefield, target_player)` at line 194 collects all permanents without type filtering. Ruling: "you put each permanent the player controls into one of the two piles."

### Test coverage
All 14 Liliana-specific tests pass. Coverage by ruling/interaction:
- Enters with 3 loyalty: `tier15_cards.rs:1814` (liliana_enters_with_loyalty)
- +1 each player chooses and discards: `tier15_cards.rs:1830` (liliana_plus_one_each_player_discards_with_choice)
- +1 single card auto-discards: `tier15_cards.rs:1893` (liliana_plus_one_single_card_auto_discards)
- +1 empty hand skipped (ruling 1): `tier15_cards.rs:1918` (liliana_plus_one_empty_hand_skipped)
- +1 both empty hands: `tier15_cards.rs:1942` (liliana_plus_one_both_empty_hands)
- -2 target player chooses creature: `tier15_cards.rs:1961` (liliana_minus_two_target_player_sacrifices_creature)
- -2 single creature auto-sacrificed: `tier15_cards.rs:2000` (liliana_minus_two_single_creature_auto_sacrifices)
- -2 no creatures: `tier15_cards.rs:2021` (liliana_minus_two_no_creatures)
- -2 can target self: `tier15_cards.rs:2040` (liliana_minus_two_can_target_self)
- -6 full pile division and choice: `tier15_cards.rs:2061` (liliana_minus_six_pile_division_and_choice)
- -6 empty pile allowed (ruling 4): `tier15_cards.rs:2119` (liliana_minus_six_empty_pile_allowed)
- -6 all in one pile: `tier15_cards.rs:2149` (liliana_minus_six_all_in_one_pile)
- -6 no permanents: `tier15_cards.rs:2182` (liliana_minus_six_no_permanents)
- -6 can target self: `tier15_cards.rs:2200` (liliana_minus_six_can_target_self)
- Ruling 2 (simultaneous discard timing): NOT TESTED (sequential implementation is functionally equivalent in 2-player engine)

## Audit — 2026-04-02 20:20

**Oracle text source**: Oracle cache (Scryfall API, cached 2026-04-01) — https://scryfall.com/card/isd/105/liliana-of-the-veil
**Oracle text**: +1: Each player discards a card.
-2: Target player sacrifices a creature.
-6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.
**Type line**: Legendary Planeswalker — Liliana
**Mana cost**: {1}{B}{B}
**Starting loyalty**: 3
**Rulings**:
1. You can activate Liliana's first ability even if some or all players will be unable to discard a card.
2. When Liliana's first ability resolves, first the player whose turn it is chooses a card in hand without revealing it, then each other player in turn order does the same. Then all the chosen cards are discarded at the same time.
3. When Liliana's third ability resolves, you put each permanent the player controls into one of the two piles. For example, you could put a creature into one pile and an Aura enchanting that creature into the other pile.
4. A pile can be empty. If the player chooses an empty pile, no permanents will be sacrificed.
**Status**: PASS

### Code issues
No issues found.

**Card data verification (all match oracle exactly):**
- Name: `"Liliana of the Veil"` -- correct
- Mana cost: `Generic(1), Colored(Black), Colored(Black)` matches `{1}{B}{B}`
- Card types: `vec![CardType::Planeswalker]` -- correct
- Supertypes: `vec![Supertype::Legendary]` -- correct
- Subtypes: `vec!["Liliana".into()]` -- correct
- Power/toughness: `None` -- correct for planeswalker
- Keywords: `vec![]` -- correct, no keywords in oracle text
- Starting loyalty: `Some(3)` -- correct
- Oracle text field (line 28): matches oracle text verbatim, including "their choice" wording
- Triggered abilities: `vec![]` -- correct, no triggered abilities
- Continuous effects: `vec![]` -- correct, no static abilities
- Flashback cost: `None` -- correct

**Loyalty abilities verification:**
- Ability 0 (+1): `loyalty_change: 1`, `target_requirement: None` -- correct, "Each player" is not targeted
- Ability 1 (-2): `loyalty_change: -2`, `target_requirement: Some(TargetRequirement::PlayerOnly)` -- correct, "Target player"
- Ability 2 (-6): `loyalty_change: -6`, `target_requirement: Some(TargetRequirement::PlayerOnly)` -- correct, "target player"

**+1 behavior (lines 71-157):**
- Builds discard list with active player first (line 76-98), matching ruling 2
- Presents `ChooseCardFromHand` when hand has 2+ cards (line 147-156) -- correct
- Auto-discards when hand has exactly 1 card (line 132-143) -- acceptable, no meaningful choice
- Skips players with empty hands (line 84/95) -- matches ruling 1
- Emits `Discarded` event (line 137-139) -- correct
- Chains to next player via `on_discard_choice` callback (line 223-226) -- correct

**-2 behavior (lines 158-185):**
- Reads target player from `targets.first()` (line 160-163) -- correct
- Collects creatures via `creatures_controlled_by(state, target_player)` (line 165) -- correct
- Sacrifice choice presented to `target_player` via `present_target_choice` (line 174), not controller -- correct per oracle
- Uses `PendingEffect::SacrificeCreature` which calls `crate::destruction::sacrifice()` (engine.rs line 2424) -- correct sacrifice semantics

**-6 behavior (lines 186-218):**
- Reads target player from `targets.first()` (line 189-192) -- correct
- Collects ALL permanents of target player (line 194-197), not just creatures -- correct per oracle and ruling 3
- Step 1: Controller divides via `DividePermanentsIntoPiles` presented to `controller` (line 206) -- correct, oracle says Liliana's controller separates
- Step 2: Target player chooses pile via `ChoosePile` presented to `target_player` (engine.rs line 2108-2120) -- correct per oracle
- Sacrifice uses `crate::destruction::sacrifice()` (engine.rs line 2132) -- correct
- Empty piles supported: subset generation includes mask=0 at engine.rs line 235 -- matches ruling 4

### Tricky interactions checked
- **+1 with all empty hands**: PASS -- `players_to_discard` is empty, logs "no player has cards to discard" and returns (line 100-104). Matches ruling 1.
- **+1 discard order (active player first)**: PASS -- Active player added at line 80-86, others at line 89-98 in turn order. Matches ruling 2.
- **+1 sequential vs simultaneous discard**: ACCEPTABLE -- Ruling 2 says "all the chosen cards are discarded at the same time" but implementation discards sequentially after each player chooses. In a 2-player engine with no "whenever a player discards" triggers during resolution, this is functionally equivalent. Not a bug.
- **-2 uses sacrifice, not destroy**: PASS -- `PendingEffect::SacrificeCreature` calls `sacrifice()` not `try_destroy()`. Sacrifice bypasses indestructible, which is correct.
- **-2 target player chooses creature, not controller**: PASS -- `present_target_choice` passes `target_player` as the choosing player (line 174-176). The sacrificing player always chooses what to sacrifice per MTG rules.
- **-2 can target any player (including self)**: PASS -- `TargetRequirement::PlayerOnly` allows any player. Confirmed by test.
- **-6 controller divides, target player chooses pile**: PASS -- `DividePermanentsIntoPiles` presented to `controller` (line 206), `ChoosePile` presented to `target_player` (engine.rs line 2109). These are correctly different players when Liliana targets an opponent.
- **-6 empty pile allowed (ruling 4)**: PASS -- Subset generation includes the empty set (mask=0). If target player chooses the empty pile, the for loop at engine.rs line 2129 iterates over an empty slice, sacrificing nothing.
- **-6 includes all permanent types (ruling 3)**: PASS -- `objects_in_zone(Zone::Battlefield, target_player)` at line 194 collects all permanents without type filtering. Auras, artifacts, enchantments, creatures, planeswalkers, and lands are all included.
- **-6 with zero permanents**: PASS -- Returns early at line 199-203 with log message.
- **-6 sacrifice uses correct pipeline**: PASS -- engine.rs line 2132 calls `crate::destruction::sacrifice()`, not `move_object()` or `try_destroy()`.
- **No move_spell_after_resolve anti-pattern**: N/A -- Loyalty abilities are not spells on the stack in this engine.
- **No CombatDamageDealt anti-pattern**: N/A -- No damage dealt by this card.
- **No missing triggered_abilities**: PASS -- `triggered_abilities: vec![]` is correct; the card has no triggered abilities, only activated loyalty abilities.

### Test coverage
All Liliana-specific tests in `mtg-engine/tests/tier15_cards.rs`:
- Enters with 3 loyalty: `liliana_enters_with_loyalty` (line 1814) -- TESTED
- +1 each player chooses and discards: `liliana_plus_one_each_player_discards_with_choice` (line 1830) -- TESTED
- +1 single card auto-discards: `liliana_plus_one_single_card_auto_discards` (line 1893) -- TESTED
- +1 empty hand skipped (ruling 1): `liliana_plus_one_empty_hand_skipped` (line 1918) -- TESTED
- +1 both empty hands: `liliana_plus_one_both_empty_hands` (line 1942) -- TESTED
- -2 target player sacrifices creature (choice): `liliana_minus_two_target_player_sacrifices_creature` (line 1961) -- TESTED
- -2 single creature auto-sacrificed: `liliana_minus_two_single_creature_auto_sacrifices` (line 2000) -- TESTED
- -2 no creatures: `liliana_minus_two_no_creatures` (line 2021) -- TESTED
- -2 can target self: `liliana_minus_two_can_target_self` (line 2040) -- TESTED
- -6 full pile division and choice: `liliana_minus_six_pile_division_and_choice` (line 2061) -- TESTED
- -6 empty pile allowed (ruling 4): `liliana_minus_six_empty_pile_allowed` (line 2119) -- TESTED
- -6 all in one pile: `liliana_minus_six_all_in_one_pile` (line 2149) -- TESTED
- -6 no permanents: `liliana_minus_six_no_permanents` (line 2182) -- TESTED
- -6 can target self: `liliana_minus_six_can_target_self` (line 2200) -- TESTED
- Planeswalker SBA (0 loyalty dies): `planeswalker_with_zero_loyalty_dies` (line 2849) -- TESTED
- Planeswalker SBA (positive loyalty survives): `planeswalker_with_loyalty_survives` (line 2864) -- TESTED
- Loyalty abilities in legal actions: `loyalty_abilities_appear_in_legal_actions` (line 2882) -- TESTED
- Loyalty counter adjustment: `loyalty_ability_adjusts_counters` (line 2915) -- TESTED
- Ruling 2 (simultaneous discard timing): NOT TESTED (sequential implementation is functionally equivalent)
- Ruling 3 (Aura in different pile from enchanted permanent): NOT TESTED (but code correctly includes all permanents)

### UI coverage
Present in `mtg-player/src/llm.rs` line 143 with accurate description of all three abilities and strategic advice.

## Audit — 2026-04-03 07:08
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/105/liliana-of-the-veil), cached 2026-04-01
**Oracle text**: +1: Each player discards a card. / −2: Target player sacrifices a creature. / −6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.
**Type line**: Legendary Planeswalker — Liliana
**Status**: PASS

### Card data verification
- Name: "Liliana of the Veil" -- MATCHES
- Mana cost: {1}{B}{B} -- MATCHES
- Card types: [Planeswalker] -- MATCHES
- Supertypes: [Legendary] -- MATCHES
- Subtypes: ["Liliana"] -- MATCHES
- Starting loyalty: 3 -- MATCHES
- Oracle text in code matches Scryfall (minor dash normalization: U+002D vs U+2212) -- MATCHES

### Code issues
- **Dead card_state entry**: Line 117 stores `liliana_discard_remaining` as a comma-separated string parsed to u64, but this key is never read anywhere. The actual chaining logic uses the `liliana_discard_count` / `liliana_discard_N` encoding. This is harmless dead data but should be cleaned up.
- **Incomplete card_state cleanup**: When `chain_next_discard` finishes (count == 0), only `liliana_discard_count` is removed (line 242). The `liliana_discard_remaining` key is never cleaned up. Again harmless since it's never read, but untidy.
- **Sequential vs simultaneous discard**: Per ruling: "all the chosen cards are discarded at the same time." The implementation discards sequentially (player A chooses and discards, then player B chooses and discards). This means (a) later players can see what earlier players discarded, and (b) discard triggers fire sequentially rather than simultaneously. This is an engine-level limitation and functionally acceptable for 2-player games where each player discards one card, but technically deviates from the rules. Flagged as a known limitation, not a blocking issue.

### Tricky interactions checked (min 3)
1. **Ruling: "You can activate Liliana's first ability even if some or all players will be unable to discard a card."** -- Code correctly handles this: players with empty hands are skipped (line 84-86, 95-97), and if all hands are empty, the ability resolves with no effect (line 100-104). Test: `liliana_plus_one_empty_hand_skipped`, `liliana_plus_one_both_empty_hands`.
2. **Ruling: "A pile can be empty. If the player chooses an empty pile, no permanents will be sacrificed."** -- Engine generates all 2^N subsets including the empty set (engine.rs line 235), so empty piles are valid. The ChoosePile handler correctly iterates over the chosen pile and sacrifices only those permanents (engine.rs line 2129). If the empty pile is chosen, zero permanents are sacrificed. Test: `liliana_minus_six_empty_pile_allowed`.
3. **-2 targets "a player" (not "opponent")**: The ability correctly uses `TargetRequirement::PlayerOnly` (line 53) which allows targeting any player including self. The targeted player chooses which creature to sacrifice (not Liliana's controller). Tests: `liliana_minus_two_can_target_self`, `liliana_minus_two_target_player_sacrifices_creature`.
4. **-6 controller divides, target player chooses**: The pile division choice is presented to `controller` (line 206), and the subsequent ChoosePile is presented to `target_player` (engine.rs line 2108-2109). This correctly implements the two-step interaction. Test: `liliana_minus_six_pile_division_and_choice` (asserts P0 divides, P1 chooses).
5. **-6 includes ALL permanents, not just creatures**: Line 194 collects `objects_in_zone(Zone::Battlefield, target_player)` without filtering by type, correctly including all permanent types. Test: `liliana_minus_six_can_target_self` verifies Liliana herself is included in the permanent count.

### Test coverage
All 14 tests pass:
- `liliana_enters_with_loyalty` -- Enters with 3 loyalty counters
- `liliana_plus_one_each_player_discards_with_choice` -- Both players choose and discard
- `liliana_plus_one_single_card_auto_discards` -- Auto-discard when only 1 card
- `liliana_plus_one_empty_hand_skipped` -- Empty-hand player skipped
- `liliana_plus_one_both_empty_hands` -- Both empty hands, no effect
- `liliana_minus_two_target_player_sacrifices_creature` -- Target player chooses creature
- `liliana_minus_two_single_creature_auto_sacrifices` -- Auto-sacrifice single creature
- `liliana_minus_two_no_creatures` -- No creatures, no effect
- `liliana_minus_two_can_target_self` -- Can target self
- `liliana_minus_six_pile_division_and_choice` -- Full pile division flow
- `liliana_minus_six_empty_pile_allowed` -- Empty pile ruling
- `liliana_minus_six_all_in_one_pile` -- All permanents in one pile
- `liliana_minus_six_no_permanents` -- No permanents, no effect
- `liliana_minus_six_can_target_self` -- Can target self with -6
