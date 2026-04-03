# Audit: Bonds of Faith

## Oracle (Scryfall/API)
- **Name:** Bonds of Faith
- **Cost:** {1}{W}
- **Type:** Enchantment — Aura
- **Oracle:** Enchant creature. Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/bonds_of_faith.rs`
- **Name:** Bonds of Faith -- CORRECT
- **Cost:** {1}{W} -- CORRECT
- **Type:** Enchantment — Aura -- CORRECT (subtypes: ["Aura"])
- **Target requirement:** Creature -- CORRECT
- **Effect on Human:** +2/+2 via ModifyPT -- CORRECT
- **Effect on non-Human:** PreventAttack + PreventBlock -- CORRECT
- **Aura attachment:** Uses resolve_aura helper -- CORRECT

## Issues
1. **ISSUE (minor):** The Human check is done once at ETB time and stored as `instance_continuous_effects`. If the creature's type changes (e.g., gains/loses Human subtype), the effect won't update. The oracle says "as long as it's a Human" which implies continuous checking.

## Verdict: PASS (with minor limitation) -- Human check is snapshot rather than continuous

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, cached 2026-04-01)
```
Enchant creature
Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
```

### Card Data Verification
- **Name:** "Bonds of Faith" -- CORRECT
- **Cost:** {1}{W} -- CORRECT
- **Types:** Enchantment -- CORRECT
- **Subtypes:** ["Aura"] -- CORRECT
- **Enchant creature** target requirement: `TargetRequirement::Creature` -- CORRECT
- **P/T:** None -- CORRECT (not a creature)

### Functional Audit

#### Aura Attachment
Uses `crate::cards::helpers::resolve_aura` in `on_resolve` -- CORRECT

#### Dual-Mode Effect
- Human path: `ContinuousEffect::ModifyPT { power: 2, toughness: 2, scope: EffectScope::Attached }` -- CORRECT
- Non-Human path: `PreventAttack` + `PreventBlock` with `EffectScope::Attached` -- CORRECT

### Issues Found

#### 1. BUG: Human subtype check ignores token subtypes and transformed creatures (MEDIUM)

The implementation at lines 43-46:
```rust
let is_human = state.get_object(target_id)
    .and_then(|o| registry.card_data(o.card_id))
    .map(|d| d.subtypes.iter().any(|s| s == "Human"))
    .unwrap_or(false);
```

This only checks `registry.card_data(card_id).subtypes`. It does NOT check:
- `obj.subtypes` on the GameObject itself (used by tokens created via `create_token_with_subtypes`)
- Back face subtypes for transformed DFCs

Compare with the correct approach used by `GameState::matches_filter` for `CreatureFilter::HasSubtype` in `state.rs` (lines ~581-599), which checks all three sources: registry card_data subtypes, back face subtypes for transformed creatures, and object-level subtypes for tokens.

A Human token (e.g., from Doomed Traveler creating a 1/1 white Human token) enchanted by Bonds of Faith would be treated as non-Human and locked down instead of receiving +2/+2.

Note: `Butcher's Cleaver` (`butchers_cleaver.rs`) has the same bug -- it also only checks `registry.card_data().subtypes`.

#### 2. KNOWN LIMITATION: Snapshot vs. continuous check (MINOR, previously identified)

The oracle says "as long as it's a Human" -- a continuous condition. The implementation evaluates this once in `on_enter_battlefield` and stores the result in `instance_continuous_effects`. If the creature's type changes while Bonds is attached (e.g., a Werewolf transforming from Human to non-Human, or a creature gaining the Human type), the effect will not update.

This is an engine-level limitation (no recalculation of conditional continuous effects), noted in the previous audit.

#### 3. No test coverage for tokens or transformed creatures

All existing tests use either named creatures from the registry (Elder Cathar, a Human) or anonymous `ready_creature` helpers (non-Human). No test covers:
- A Human token being enchanted (would expose Bug #1)
- A transformed DFC being enchanted

### Anti-Pattern Check
- No unsafe code
- No panics or unwraps on fallible paths (uses `and_then`/`unwrap_or` correctly)
- Effect scoping uses `EffectScope::Attached` correctly

### Verdict: CONDITIONAL PASS

The card is functionally correct for the common case (enchanting a non-token, non-transformed creature). Bug #1 (token subtypes not checked) is a real correctness issue that would produce wrong behavior when enchanting Human tokens. Bug #2 (snapshot vs continuous) is a known engine limitation.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Enchant creature\nEnchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found. Target requirement correctly set to Creature for the Aura. Human check grants +2/+2, non-Human prevents attack and block. The "Enchant creature" keyword ability is handled via target_requirement rather than oracle_text, which is acceptable.

## Audit — 2026-04-02 20:37

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Enchant creature\nEnchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found.

Card data matches oracle in all respects:
- Name: "Bonds of Faith" -- correct
- Cost: {1}{W} (Generic(1), White) -- correct
- Type line: Enchantment with subtype Aura -- correct
- "Enchant creature" handled via `TargetRequirement::Creature` and `resolve_aura` helper -- correct
- Human path: `ModifyPT { power: 2, toughness: 2, scope: Attached }` -- correct
- Non-Human path: `PreventAttack { scope: Attached }` + `PreventBlock { scope: Attached }` -- correct

Previously identified issues (still present, accepted as engine-level limitations):
- Human subtype check at ETB uses `registry.card_data()` only (misses token subtypes / transformed back faces)
- Effect is snapshot at ETB rather than continuously re-evaluated per "as long as it's a Human"

### Tricky interactions checked
- Aura falls off when creature leaves battlefield: pass (SBA rule 704.5m in `sba.rs` handles unattached auras)
- Creature declared as attacker then loses Human type mid-combat: not removed from combat per official ruling (2011-09-22) -- engine does not re-evaluate instance effects mid-combat, so behavior is consistent
- Bonds on a Human grants +2/+2 and does NOT prevent attack/block: pass (tested in `bug_fixes.rs:522` and `card_mechanics.rs:197`)
- Bonds on a non-Human prevents attack AND block without P/T bonus: pass (tested in `bug_fixes.rs:546` and `card_mechanics.rs:217`)

### Test coverage
- Human gets +2/+2: `bug_fixes.rs:522`, `card_mechanics.rs:197`
- Non-Human can't attack or block: `bug_fixes.rs:546`, `card_mechanics.rs:217`, `innistrad_cards.rs:306`
- Human with Bonds can still attack: `bug_fixes.rs:540`
- Non-Human does NOT get P/T bonus: `bug_fixes.rs:556`
- Bonds on a Human token: NOT TESTED
- Bonds on a transformed DFC: NOT TESTED
