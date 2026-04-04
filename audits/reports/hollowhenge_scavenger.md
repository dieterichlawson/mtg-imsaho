## Audit — 2026-04-03 22:21

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Morbid — When this creature enters, if a creature died this turn, you gain 5 life.
**Type line**: Creature — Elemental
**Status**: ISSUE

### Code issues

- **Engine limitation in trigger resolution (/Users/dlaw/mtg/mtg-engine/src/triggers.rs:895)**: The trigger resolution system checks `o.zone == Zone::Battlefield` before calling `on_enter_battlefield`, which prevents the life gain if Hollowhenge Scavenger leaves the battlefield before its ETB trigger resolves.
  - Oracle text says: `When this creature enters, if a creature died this turn, you gain 5 life.`
  - Code does: Checks if source is still on battlefield (`state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)`) and skips the ability if the source has left the battlefield. Per MTG rules, this triggered ability should still resolve and gain life even if the source leaves the battlefield before resolution, as long as the morbid condition is still met.

### Tricky interactions checked

- **Morbid timing (intervening if clause)**: The code checks `state.creature_died_this_turn` only at resolution time, not at trigger time. However, since this flag persists for the entire turn once set, this is functionally equivalent to the correct "intervening if" behavior. PASS
- **creature_died_this_turn flag lifecycle**: Flag is properly set when creatures die (destruction.rs:100, sba.rs:96,144) and reset at start of each turn (engine.rs:2888). This correctly tracks any creature death during the turn regardless of controller. PASS  
- **Hollowhenge Scavenger dying to opponent's faster ETB trigger**: If an opponent's creature also enters simultaneously and has an ETB that destroys Hollowhenge Scavenger before its trigger resolves, the zone check prevents life gain. This is incorrect behavior - the life gain should still happen. FAIL
- **Token creatures enabling morbid**: The `creature_died_this_turn` flag is set for both token and non-token creatures in sba.rs and destruction.rs, correctly enabling morbid for token deaths. PASS
- **Multiple creatures dying in one turn**: The flag remains true once set, so multiple deaths don't interfere with each other. PASS

### Test coverage

- **No dedicated tests found**: No test files reference `HollowhengeScavenger` or test the specific interaction of morbid ETB triggers. NOT TESTED
- **Morbid mechanism**: The `creature_died_this_turn` flag is tested indirectly through other morbid cards but not specifically for ETB triggers. PARTIALLY TESTED
- **ETB trigger resolution with source removal**: This edge case interaction is not tested. NOT TESTED

## Audit — 2026-04-03 22:21 (independent)

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Morbid — When this creature enters, if a creature died this turn, you gain 5 life.
**Type line**: Creature — Elemental
**Status**: ISSUE

### Code issues

- **Oracle text field uses outdated templating** (`hollowhenge_scavenger.rs:26`)
  - Oracle text says: `Morbid — When this creature enters, if a creature died this turn, you gain 5 life.`
  - Code does: `oracle_text: "Morbid — When Hollowhenge Scavenger enters the battlefield, if a creature died this turn, you gain 5 life.".into()`
  - The code uses pre-errata wording ("Hollowhenge Scavenger enters the battlefield") instead of the current oracle text ("this creature enters").

- **Intervening-if clause not enforced at trigger time** (`triggers.rs:349-363`, `hollowhenge_scavenger.rs:39`)
  - Oracle text says: `When this creature enters, if a creature died this turn, you gain 5 life.`
  - The "if a creature died this turn" is an intervening-if clause per MTG rules (CR 603.4). It must be checked both when the trigger event occurs (to determine if the ability triggers at all) and again at resolution. The engine at `triggers.rs:351` unconditionally puts the ETB trigger on the stack (`if registry.get(card_id).is_some()`), and the morbid condition is only checked at resolution in `hollowhenge_scavenger.rs:39` (`if state.creature_died_this_turn`). This means: if no creature has died when the Scavenger enters, the trigger still goes on the stack; if a creature then dies before the trigger resolves, the player incorrectly gains 5 life.

- **ETB trigger skipped if source leaves battlefield before resolution** (`triggers.rs:895`)
  - Oracle text says: `When this creature enters, if a creature died this turn, you gain 5 life.`
  - Code does: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` — skips the trigger entirely if the Scavenger has left the battlefield. Per MTG rules (CR 113.7a), a triggered ability on the stack exists independently of its source. If the Scavenger is destroyed or bounced after its ETB trigger goes on the stack but before it resolves, the trigger should still resolve and gain 5 life (if morbid is met). The engine incorrectly prevents this.

### Tricky interactions checked

- **Intervening-if clause (morbid false at ETB, true at resolution)**: FAIL — trigger incorrectly goes on the stack and resolves, gaining 5 life when it shouldn't trigger at all.
- **Source leaves battlefield before trigger resolves**: FAIL — trigger is incorrectly suppressed when source is no longer on the battlefield.
- **creature_died_this_turn flag lifecycle**: PASS — flag is set in `sba.rs:96`, `sba.rs:144`, `destruction.rs:100` when creatures die, and reset at turn start in `engine.rs:2888`.
- **Token creature deaths enabling morbid**: PASS — `creature_died_this_turn` is set for all creature deaths including tokens (SBA and destruction paths handle all creatures uniformly).
- **Life gain amount**: PASS — code correctly adds 5 life per oracle text.
- **Controller identification**: PASS — `state.get_object(object_id).map(|o| o.controller)` correctly identifies the controller who gains life.
- **Mana cost**: PASS — `{3}{G}{G}` matches oracle `{3}{G}{G}`.
- **P/T**: PASS — 4/5 matches oracle.
- **Subtypes**: PASS — `"Elemental"` matches oracle type line `Creature — Elemental`.

### Test coverage

- Morbid ETB trigger (basic case): NOT TESTED — no tests reference Hollowhenge Scavenger.
- Intervening-if clause (morbid false at trigger time): NOT TESTED
- Source leaves battlefield before ETB trigger resolves: NOT TESTED
- Token death enabling morbid: NOT TESTED (for this card specifically)
- Life gain amount (5 life): NOT TESTED

## Audit — 2026-04-03 22:50 (independent)

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Morbid — When this creature enters, if a creature died this turn, you gain 5 life.
**Type line**: Creature — Elemental
**Status**: ISSUE

### Code issues
- Oracle text field uses outdated templating (`hollowhenge_scavenger.rs:26`)
  - Oracle text says: `Morbid — When this creature enters, if a creature died this turn, you gain 5 life.`
  - Code does: `oracle_text: "Morbid — When Hollowhenge Scavenger enters the battlefield, if a creature died this turn, you gain 5 life.".into()`

### Tricky interactions checked
- Morbid intervening-if clause timing (must be true at trigger and resolution): UNCERTAIN - engine pattern may be functionally equivalent
- Life gain applies to controller of Hollowhenge Scavenger: PASS
- Creature death tracking via `creature_died_this_turn` flag: PASS - set in sba.rs:96,144 and destruction.rs:100, reset in engine.rs:2888
- Token creature deaths count for morbid: PASS - flag applies to all creature deaths
- Morbid flag resets at start of each turn: PASS
- ETB trigger resolution when source leaves battlefield: ISSUE - triggers.rs:895 incorrectly requires source on battlefield
- Basic card data (mana cost, P/T, types): PASS - {3}{G}{G}, 4/5, Creature — Elemental match oracle
- Life gain amount and target: PASS - correctly gains 5 life for controller

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Morbid intervening-if clause timing: `mtg-engine/tests/card_mechanics.rs:28` / TESTED (general mechanism)
- Life gain applies to controller: NOT TESTED (specific to this card)
- Creature death tracking via `creature_died_this_turn` flag: `mtg-engine/tests/card_mechanics.rs:28` / TESTED
- Token creature deaths count for morbid: `mtg-engine/tests/card_mechanics.rs:991` / TESTED (sacrifice case)
- Morbid flag resets at start of each turn: `mtg-engine/tests/card_mechanics.rs:43` / TESTED
- ETB trigger resolution when source leaves battlefield: NOT TESTED
- Basic card properties (cost, P/T, types): NOT TESTED
- Hollowhenge Scavenger specific morbid life gain: NOT TESTED
