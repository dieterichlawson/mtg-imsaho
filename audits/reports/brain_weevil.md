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
