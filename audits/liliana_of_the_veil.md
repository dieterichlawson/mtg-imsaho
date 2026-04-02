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
