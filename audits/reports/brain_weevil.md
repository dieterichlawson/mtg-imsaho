## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
Sacrifice this creature: Target player discards two cards. Activate only as a sorcery.
**Type line**: Creature — Insect
**Status**: PASS
### Code issues
No issues found.

## Audit — 2026-04-02 20:37

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
Sacrifice this creature: Target player discards two cards. Activate only as a sorcery.
**Type line**: Creature — Insect
**Status**: ISSUE

### Code issues
- When the target player has 3 or more cards in hand, Brain Weevil only forces 1 discard instead of 2. `brain_weevil.rs:64-75`
  - Oracle text says: `Target player discards two cards`
  - Code does: Sets up a single `ChooseCardFromHand` prompt for "choose a card to discard (1 of 2)" but never implements `on_discard_choice` to chain the second discard. After the first card is chosen and discarded by the engine (engine.rs:2009-2023), the engine calls `on_discard_choice` on the source card, but `BrainWeevil` uses the default no-op implementation, so the second discard never occurs.

### Tricky interactions checked
- Sacrifice as cost (cannot be sacrificed to another effect simultaneously): PASS -- uses `SacrificeCost::SacrificeThis` which is processed as a cost before the effect resolves
- Sorcery-speed restriction: PASS -- `sorcery_speed_only: true` in the ability definition
- Intimidate blocking restriction (artifact creatures and/or shared color): PASS -- combat.rs:626-644 correctly checks for `CardType::Artifact` or shared color
- Target player (not "opponent"): PASS -- `TargetRequirement::PlayerOnly` allows targeting any player, including self, which is correct per oracle text
- Discard with 0 cards in hand (no-op): PASS -- handled at line 50-52, returns early if hand is empty

### Test coverage
- Sacrifice + discard (2 cards in hand, auto-path): `tier8_cards.rs:93` (brain_weevil_forces_discard)
- Intimidate keyword present: `tier8_cards.rs:129` (brain_weevil_has_intimidate)
- Discard with 3+ cards in hand (choice path + second discard): NOT TESTED
- Discard with 0 cards in hand: NOT TESTED
- Sorcery-speed restriction enforcement: NOT TESTED

## Audit — 2026-04-03 22:06

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
Sacrifice this creature: Target player discards two cards. Activate only as a sorcery.
**Type line**: Creature — Insect
**Status**: ISSUE

### Code issues
- When the target player has 3 or more cards in hand, only 1 card is discarded instead of 2. (`mtg-engine/src/cards/isd/brain_weevil.rs:64-75`)
  - Oracle text says: `Target player discards two cards`
  - Code does: `on_activate_ability` creates a single `ChooseCardFromHand` prompt with description `"Brain Weevil: choose a card to discard (1 of 2)"` but `BrainWeevil` does not implement `on_discard_choice` to chain the second discard. After the engine processes the first chosen card (`engine.rs:2009-2023`), it calls `behavior.on_discard_choice(...)` on the source card, which falls through to the default no-op in `cards/mod.rs:421`. The second discard never occurs.

### Tricky interactions checked
- Sacrifice as cost (processed before effect): PASS — `SacrificeCost::SacrificeThis` at line 36 causes `crate::destruction::sacrifice` to be called at `engine.rs:1748` before `on_activate_ability` runs
- Sorcery-speed restriction: PASS — `sorcery_speed_only: true` at line 39; engine checks this at `engine.rs:360`
- Intimidate blocking restriction: PASS — `combat.rs:626-644` checks blocker for `CardType::Artifact` or shared color, consistent with oracle text "artifact creatures and/or creatures that share a color with it"
- Target player includes self: PASS — `TargetRequirement::PlayerOnly` at line 37 iterates all non-lost players (`engine.rs:883-888`), matching oracle "Target player" (not "target opponent")
- Discard with 0 cards (no-op): PASS — early return at line 50-52
- Discard with exactly 1-2 cards (auto-discard): PASS — lines 54-63 discard all cards and emit Discarded events
- Second discard with 3+ cards: FAIL — see code issue above
- Colors set correctly for Intimidate: PASS — colors derived from mana cost at `engine.rs:2654-2667`; {3}{B} yields [Black]
- Card data fields (cost, types, P/T, keywords): PASS — all match oracle text

### Test coverage
- Sacrifice + discard (2 cards in hand, auto-path): `tier8_cards.rs:93` (brain_weevil_forces_discard)
- Intimidate keyword present: `tier8_cards.rs:129` (brain_weevil_has_intimidate)
- Discard with 3+ cards in hand (choice path + second discard chaining): NOT TESTED
- Discard with 0 cards in hand: NOT TESTED
- Sorcery-speed restriction enforcement: NOT TESTED
- Target self (target own player): NOT TESTED
- Ruling (activate immediately after cast in main phase): NOT TESTED

## Audit — 2026-04-03 22:06

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
Sacrifice this creature: Target player discards two cards. Activate only as a sorcery.
**Type line**: Creature — Insect
**Status**: ISSUE

### Code issues
- When the target player has 3 or more cards in hand, Brain Weevil only forces 1 discard instead of 2. `brain_weevil.rs:64-75`
  - Oracle text says: `Target player discards two cards`
  - Code does: Sets up a single `ChooseCardFromHand` prompt for "choose a card to discard (1 of 2)" but never implements `on_discard_choice` to chain the second discard. After the first card is chosen and discarded by the engine, the engine calls `on_discard_choice` on the source card, but `BrainWeevil` uses the default no-op implementation (mod.rs:421), so the second discard never occurs.

### Tricky interactions checked
- Sacrifice as cost vs sacrifice as effect (timing): pass - uses `SacrificeCost::SacrificeThis` which is processed as a cost before the effect resolves
- Sorcery-speed restriction enforcement: pass - `sorcery_speed_only: true` enforced by engine.rs:360 check against `is_sorcery_speed` 
- Intimidate blocking rules (artifact or shared color): pass - combat.rs:626-644 correctly implements intimidate restrictions
- Target any player vs opponent only: pass - `TargetRequirement::PlayerOnly` allows targeting any player including self
- Discard with empty hand handling: pass - early return at lines 50-52 when hand is empty
- Immediate activation after ETB: pass - sorcery speed restriction allows activation during main phase when stack is empty

### Test coverage  
For each ruling and tricky interaction, list whether it is tested and where:
- Sacrifice + discard with ≤2 cards (auto-discard path): `tier8_cards.rs:93-126`
- Intimidate keyword presence: `tier8_cards.rs:129-135`  
- Discard with 3+ cards requiring choice (main bug): NOT TESTED
- Discard with 0 cards in hand: NOT TESTED
- Sorcery-speed restriction enforcement: NOT TESTED
- Immediate activation after casting: NOT TESTED
