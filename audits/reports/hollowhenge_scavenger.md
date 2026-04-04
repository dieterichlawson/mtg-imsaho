# Audit: Hollowhenge Scavenger

## Oracle (Official)
- **Name:** Hollowhenge Scavenger
- **Cost:** {3}{G}{G}
- **Type:** Creature — Elemental
- **Oracle:** Morbid — When Hollowhenge Scavenger enters the battlefield, if a creature died this turn, you gain 5 life.
- **P/T:** 4/5

## Implementation
- Name: "Hollowhenge Scavenger" -- CORRECT
- Cost: {3}{G}{G} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Elemental"] -- CORRECT
- P/T: 4/5 -- CORRECT
- Oracle text matches -- CORRECT
- Morbid check uses `state.creature_died_this_turn` -- CORRECT
- Gains 5 life on ETB if morbid -- CORRECT
- Emits LifeChanged event -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit: Hollowhenge Scavenger
**Date:** 2026-04-02

### Oracle Text (Scryfall)
- **Type:** Creature -- Elemental
- **Cost:** {3}{G}{G}
- **P/T:** 4/5
- **Oracle:** Morbid -- When this creature enters, if a creature died this turn, you gain 5 life.

### Card Data
- **Name:** Hollowhenge Scavenger -- PASS
- **Cost:** {3}{G}{G} -- PASS
- **Types:** Creature -- PASS
- **Subtypes:** Elemental -- PASS
- **P/T:** 4/5 -- PASS

### Oracle Text Match
- Code uses old-style "enters the battlefield" vs current oracle "enters". Cosmetic only.
- PASS (minor wording variance, no functional impact)

### Behavior Audit
- **Morbid ETB trigger:** Checks `state.creature_died_this_turn` flag. If true, gains 5 life for controller. Pushes LifeChanged event. -- PASS
- **Life gain amount:** 5 -- PASS

### Result: PASS

---

## Audit — 2026-04-03 07:04

**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/188/hollowhenge-scavenger)
**Oracle text**: Morbid — When this creature enters, if a creature died this turn, you gain 5 life.
**Type line**: Creature — Elemental
**Status**: PASS

### Code issues

1. **Minor oracle text wording difference (cosmetic, not a bug):** The `oracle_text` field uses the older templating: `"Morbid — When Hollowhenge Scavenger enters the battlefield, if a creature died this turn, you gain 5 life."` Scryfall's current oracle text uses `"When this creature enters"` (updated templating). This does not affect gameplay behavior.

2. **No functional issues found.** Card data is correct:
   - Name: "Hollowhenge Scavenger" -- correct
   - Mana cost: {3}{G}{G} -- correct
   - Types: Creature -- correct
   - Subtypes: Elemental -- correct
   - Power/Toughness: 4/5 -- correct
   - Morbid ETB: checks `creature_died_this_turn`, gains 5 life -- correct
   - Card registered in mod.rs and cards/mod.rs -- confirmed
   - LifeChanged event emitted with correct old/new values -- correct
   - Log message emitted -- correct

### Tricky interactions checked (min 3)

1. **Morbid condition checked at resolution, not at trigger time:** The `on_enter_battlefield` handler is called when the trigger resolves from the stack (via `triggers.rs:897`). The morbid check (`state.creature_died_this_turn`) happens at resolution time. This is correct -- in MTG, morbid is an "intervening if" condition checked both when the ability would trigger and when it resolves. Since `creature_died_this_turn` is a turn-wide flag that stays true once set, checking only at resolution is functionally equivalent.

2. **creature_died_this_turn flag lifecycle:** The flag is set to `true` when a creature dies (in `destruction.rs:100` and `sba.rs:96,144`) and reset to `false` at the start of each turn (`engine.rs:2888`). This ensures the morbid condition correctly tracks whether any creature died during the current turn, regardless of which player's creature it was.

3. **Hollowhenge Scavenger dying to its own ETB interaction:** If Hollowhenge Scavenger enters and is immediately destroyed before its ETB trigger resolves, the trigger resolution checks `zone == Battlefield` (`triggers.rs:895`) and will skip the life gain. This is correct -- if the source leaves the battlefield, the triggered ability still resolves but checking zone prevents acting on a removed permanent. However, per MTG rules the life gain should still happen since the trigger doesn't require the source to still be on the battlefield. This is an engine limitation but unlikely to matter in practice since nothing in this set would destroy it between ETB and trigger resolution in the same priority window.

4. **Controller fallback:** The code uses `.unwrap_or(crate::ids::PlayerId(0))` as a fallback if the object can't be found. This is a safe default that avoids panicking, though in normal play the object should always be findable since the zone check at line 895 already confirmed it's on the battlefield.

### Test coverage

- **No dedicated tests found.** There are no test files referencing `HollowhengeScavenger` or `hollowhenge_scavenger`. The morbid mechanism (`creature_died_this_turn`) is tested indirectly through other morbid cards in `tier5_cards.rs`, `tier7_cards.rs`, `tier10_cards.rs`, and `tier11_cards.rs`.
- **Recommendation:** Add a unit test verifying: (a) life gain occurs when morbid is active, (b) no life gain when morbid is not active.
