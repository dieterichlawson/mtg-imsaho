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
