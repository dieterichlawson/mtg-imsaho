# Audit: Angel of Flight Alabaster

## Reference (Scryfall/API)
- **Name:** Angel of Flight Alabaster
- **Mana Cost:** {4}{W}
- **Type:** Creature — Angel
- **Oracle:** Flying. At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
- **P/T:** 4/4

## Implementation: `angel_of_flight_alabaster.rs`
- **Name:** Angel of Flight Alabaster -- CORRECT
- **Mana Cost:** {4}{W} -- CORRECT
- **Type:** Creature — Angel -- CORRECT
- **P/T:** 4/4 -- CORRECT
- **Keywords:** Flying -- CORRECT
- **Triggered ability:** Upkeep trigger, returns Spirit from graveyard to hand -- CORRECT
- **Target filtering:** Checks both registry subtypes and object subtypes for "Spirit" -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying\nAt the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
**Type line**: Creature — Angel
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Angel of Flight Alabaster", cost {4}{W}, 4/4, type Creature — Angel, keywords [Flying]. Triggered ability on Upkeep correctly filters Spirit cards in owner's graveyard and presents choice via present_target_choice with ReturnToHand effect. Only triggers for active_player == controller. All correct.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-01

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying\nAt the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
**Type line**: Creature — Angel
**Mana cost**: {4}{W}
**P/T**: 4/4
**Ruling [2011-09-22]**: The Spirit card must already be in your graveyard when the ability triggers at the beginning of your upkeep. If there is no Spirit card in your graveyard when your upkeep begins, the ability will be removed from the stack with no effect.
**Status**: PASS

### Code issues
No issues found.

Card data verified against oracle text:
- Name: "Angel of Flight Alabaster" -- matches
- Mana cost: `Generic(4)` + `Colored(Color::White)` = {4}{W} -- matches
- Type: `CardType::Creature` -- matches
- Supertypes: none -- matches (no supertypes in oracle type line)
- Subtypes: `["Angel"]` -- matches
- P/T: 4/4 -- matches
- Keywords: `[Keyword::Flying]` -- matches
- Oracle text field: matches
- Triggered ability: `TriggerKind::Upkeep` declared, `on_upkeep` implemented -- matches

Behavior verified:
- `on_upkeep` checks Angel is on battlefield and active player is controller (correct for "your upkeep")
- Collects Spirit cards from controller's graveyard using both `registry.card_data().subtypes` and `o.subtypes` (correctly handles both regular cards and tokens)
- Uses `present_target_choice` with `optional: false` (correct -- oracle says "return target Spirit card", mandatory, no "you may")
- Effect is `ReturnToHand` which moves the object to hand zone (correct)
- If no Spirits in graveyard, `present_target_choice` returns early due to empty targets (consistent with ruling)

### Tricky interactions checked
- Mandatory targeting (no "you may"): PASS -- `optional: false` in `present_target_choice`
- No Spirits in graveyard (ruling): PASS -- helper returns early when `targets.is_empty()`
- Only triggers on controller's upkeep, not opponent's: PASS -- checks `state.active_player != controller`
- Spirit subtype check covers tokens: PASS -- checks both registry subtypes and object subtypes
- Multiple Spirits in graveyard (choice presented): PASS -- `present_target_choice` presents choice when targets > 1 and not optional

### Test coverage
- Main effect (return Spirit on upkeep): `mtg-engine/tests/tier7_cards.rs:225` (angel_of_flight_alabaster_returns_spirit)
- Fizzle (target removed before resolution): NOT TESTED
- No Spirits in graveyard (ruling): NOT TESTED
- Multiple Spirits (choice among targets): NOT TESTED
- Non-Spirit not returned: NOT TESTED

## Audit — 2026-04-02 20:28

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Flying
At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
**Type line**: Creature — Angel
**Status**: PASS

### Code issues
No issues found.

All card data matches oracle exactly:
- Name: "Angel of Flight Alabaster" — correct
- Mana cost: {4}{W} (Generic(4) + Colored(White)) — correct
- Type: Creature — Angel — correct
- P/T: 4/4 — correct
- Keywords: Flying — correct
- Oracle text string: matches verbatim
- Triggered ability: TriggerKind::Upkeep with on_upkeep handler — correct
- Mandatory targeting (no "you may"): optional=false — correct
- Controller's upkeep only: active_player != controller guard — correct
- Spirit filtering: checks both registry card_data subtypes and runtime object subtypes — correct
- Empty graveyard: present_target_choice returns early on empty targets — consistent with ruling [2011-09-22]

### Tricky interactions checked
- Mandatory targeting (no "you may"): PASS — `optional: false` passed to `present_target_choice`
- No Spirits in graveyard (ruling [2011-09-22]): PASS — `present_target_choice` returns early when `targets.is_empty()`
- Only triggers on controller's upkeep: PASS — `state.active_player != controller` check at line 43
- Multiple Spirits in graveyard: PASS — `present_target_choice` presents a choice when targets.len() > 1
- Angel must be on battlefield: PASS — checked at line 39-41 (`o.zone == Zone::Battlefield`)

### Test coverage
- Return Spirit on upkeep (single target auto-applied): `tier7_cards.rs:225` (angel_of_flight_alabaster_returns_spirit)
- No Spirits in graveyard (ruling): NOT TESTED
- Multiple Spirits (player choice): NOT TESTED
- Non-Spirit card not returned: NOT TESTED
