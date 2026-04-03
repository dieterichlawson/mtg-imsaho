# Audit: Devil's Play

## Reference (Scryfall)
- **Name:** Devil's Play
- **Cost:** {X}{R}
- **Type:** Sorcery
- **Oracle:** Devil's Play deals X damage to any target. Flashback {X}{R}{R}{R}
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({X}{R})
- Type: CORRECT (Sorcery)
- Oracle text: CORRECT
- Flashback cost: CORRECT ({X}{R}{R}{R})
- Target requirement: CORRECT (AnyTarget)
- X damage via stored x_value: CORRECT
- P/T: CORRECT (N/A)

## Detailed Audit (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
```
Devil's Play deals X damage to any target.
Flashback {X}{R}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

### Mana Cost
- Oracle: {X}{R}
- Implementation: `ManaCost::new(vec![ManaSymbol::X, ManaSymbol::Colored(Color::Red)])`
- **CORRECT**

### Card Type
- Oracle: Sorcery
- Implementation: `vec![CardType::Sorcery]`
- **CORRECT**

### Targeting
- Oracle: "any target"
- Implementation: `TargetRequirement::AnyTarget`
- **CORRECT**

### X Cost Handling
- Implementation reads `x_value` from spell object via `state.get_object(object_id).and_then(|o| o.x_value).unwrap_or(0)`.
- Engine casting code computes and stores X based on mana paid minus colored requirements.
- **CORRECT**

### Damage
- Uses `resolve_damage` helper which:
  - Marks `damage_marked += amount` on creature targets
  - Pushes to `damaged_by` vector (tracks damage source)
  - Emits `NonCombatDamageDealt` event for both creature and player targets
  - Subtracts from player life total for player targets
- When X=0, damage is skipped entirely (correct per MTG rules: 0 damage is not dealt).
- **CORRECT**

### Flashback
- Flashback cost: `Some(ManaCost::new(vec![ManaSymbol::X, ManaSymbol::Colored(Color::Red), ManaSymbol::Colored(Color::Red), ManaSymbol::Colored(Color::Red)]))`
- Oracle: {X}{R}{R}{R}
- `flashback_cost` field used (engine convention; Flashback keyword not in keywords vec, consistent with all 28 other flashback cards).
- `move_spell_after_resolve` checks `cast_with_flashback` flag: exiles if true, moves to graveyard if false.
- **CORRECT**

### move_spell_after_resolve
- Called via `resolve_damage` when X > 0.
- Called directly in `on_resolve` when X = 0.
- Both paths covered.
- **CORRECT**

### Tests
- `devils_play_deals_x_damage`: Casts with 4 mana (3 colorless + 1 red), X=3, verifies opponent at 17 life. **PASSES**
- `devils_play_x_zero`: Casts with 1 red mana, X=0, verifies opponent at 20 life. **PASSES**
- No flashback-specific test for Devil's Play (flashback mechanics tested generically by engine).

## Issues
None found.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Devil's Play deals X damage to any target. / Flashback {X}{R}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:50
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/140/devils-play), cached 2026-04-01
**Oracle text**: Devil's Play deals X damage to any target.\nFlashback {X}{R}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found. All card data fields match oracle text exactly:
- Name: "Devil's Play" -- matches oracle
- Mana cost: {X}{R} -- matches oracle
- Type: Sorcery -- matches oracle
- Oracle text in implementation: `"Devil's Play deals X damage to any target.\nFlashback {X}{R}{R}{R}"` -- matches (reminder text correctly omitted)
- Flashback cost: {X}{R}{R}{R} via `flashback_cost: Some(...)` -- matches oracle
- Target: `TargetRequirement::AnyTarget` -- matches "any target"
- X=0 handling: skips damage entirely, which is correct per CR 120.8 (0 damage is not dealt)
- Post-resolve zone: `move_spell_after_resolve` correctly exiles on flashback, graveyards otherwise

### Tricky interactions checked (min 3)
1. **X with flashback cost**: Engine computes X as (total mana - non-X colored requirements). For flashback {X}{R}{R}{R}, this means X = total_mana - 3. Verified in `engine.rs:1513-1522` -- the `non_x_cost` is built by filtering out ManaSymbol::X, so for flashback the non-X cost is {R}{R}{R} (mana_value = 3). Correct.
2. **Mana value of spell on stack**: Per rulings, mana value is determined by the mana cost ({X}{R}), not the flashback cost. The engine stores `x_value` on the object but uses the original `cost` field for mana value calculations. This is correct -- even when cast with flashback for X=4 (paying 7 total), the mana value would be 5 (X+R where X=4).
3. **Fizzle with illegal targets**: `stack.rs:79-87` checks target legality at resolution. If the target (creature or player) becomes illegal, the spell fizzles and `move_spell_after_resolve` is called, which correctly exiles if cast with flashback. No damage is dealt on fizzle.
4. **X=0 does not trigger damage events**: When X=0, `on_resolve` skips `resolve_damage` and goes directly to `move_spell_after_resolve`. This is correct per CR 120.8: "If a source would deal 0 damage, it does not deal damage at all." No spurious `NonCombatDamageDealt` event is emitted.

### Test coverage
- `devils_play_deals_x_damage` (tier14_cards.rs:298): X=3 damage to player, verifies life total. PASS.
- `devils_play_x_zero` (tier14_cards.rs:318): X=0, verifies no damage dealt. PASS.
- Gap: No test for flashback casting (X computation with {R}{R}{R} base). Flashback mechanics are tested generically by the engine via other cards.
- Gap: No test for targeting a creature (only player targets tested).
