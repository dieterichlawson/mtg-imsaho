# Audit: Butcher's Cleaver

## Oracle (Scryfall/API)
- **Name:** Butcher's Cleaver
- **Cost:** {3}
- **Type:** Artifact — Equipment
- **Oracle:** Equipped creature gets +3/+0. As long as equipped creature is a Human, it has lifelink. Equip {3}
- **P/T:** N/A

## Implementation: `butchers_cleaver.rs`
- **Name:** Butcher's Cleaver -- CORRECT
- **Cost:** {3} -- CORRECT
- **Type:** Artifact — Equipment -- CORRECT (subtypes: ["Equipment"])
- **Static P/T bonus:** +3/+0 via ModifyPT with Attached scope -- CORRECT
- **Conditional lifelink:** Grants Lifelink keyword if creature is Human via `update_effects` -- CORRECT
- **Equip cost:** {3}, sorcery speed -- CORRECT
- **Target validation:** Only your own creatures -- CORRECT

## Issues
1. **ISSUE (minor):** The Human check in `update_effects` only checks registry subtypes, not object subtypes. Token creatures that have Human subtype only on the object (not in registry) would not get lifelink. Other cards (e.g., Avacynian Priest) check both sources.
2. **ISSUE (minor):** Like Bonds of Faith, the Human check is done once when equipping. If the creature gains/loses Human subtype later, the lifelink status won't update. The oracle says "as long as" which implies continuous checking.

## Verdict: PASS (with minor limitations) -- Human check is slightly incomplete and not continuously updated

---

# Re-Audit: Butcher's Cleaver (2026-04-02)

## Oracle Text (Scryfall, cached 2026-04-01)

> Equipped creature gets +3/+0.
> As long as equipped creature is a Human, it has lifelink.
> Equip {3}

- **Name:** Butcher's Cleaver
- **Cost:** {3}
- **Type:** Artifact -- Equipment
- **Keywords:** Equip

## Implementation Review: `mtg-engine/src/cards/isd/butchers_cleaver.rs`

### Card Data -- CORRECT
- Name: "Butcher's Cleaver" -- matches oracle
- Cost: Generic(3) -- matches {3}
- Card types: [Artifact] -- correct
- Subtypes: ["Equipment"] -- correct
- Oracle text string matches oracle verbatim -- correct
- P/T: None -- correct (not a creature)

### Equip Ability -- CORRECT
- Cost: Generic(3) -- matches Equip {3}
- `sorcery_speed_only: true` -- correct (equip is sorcery-speed by default)
- `requires_tap: false` -- correct
- Target: `CreatureWithFilter(YouControl)` -- correct
- Only available on the battlefield -- correct

### Static +3/+0 Bonus -- CORRECT
- `continuous_effects` in card_data includes `ModifyPT { power: 3, toughness: 0, scope: Attached }` -- matches "Equipped creature gets +3/+0"

### Conditional Lifelink for Humans -- PARTIALLY CORRECT
- `update_effects` checks if the equipped creature is a Human and, if so, adds `GrantKeyword { keyword: Keyword::Lifelink, scope: Attached }` -- matches the oracle intent.

### Target Validation (`is_valid_target`) -- CORRECT
- Checks battlefield zone, has power (is a creature), controller matches caster -- correct for Equip targeting.

### Equip Resolution (`on_activate_ability`) -- CORRECT
- Sets `attached_to` on the equipment object -- correct
- Calls `update_effects` to apply continuous effects -- correct

### Enter Battlefield (`on_resolve`) -- CORRECT
- Moves to battlefield and sets `is_equipment = true` -- correct

## Issues Found

### 1. BUG (minor): Human subtype check only queries registry, not object subtypes

**Oracle text:** "As long as equipped creature is a Human, it has lifelink."

**Code (lines 15-18):**
```rust
let is_human = state.get_object(creature_id)
    .and_then(|o| registry.card_data(o.card_id))
    .map(|d| d.subtypes.iter().any(|s| s == "Human"))
    .unwrap_or(false);
```

The check only looks at `registry.card_data(o.card_id).subtypes` (the static card definition). It does not check `obj.subtypes` (the runtime object subtypes). Creatures that gain the Human subtype at runtime (e.g., via Olivia Voldaren adding Vampire, or changelings represented as tokens) would be missed.

Compare with Wooden Stake (`wooden_stake.rs` lines 86-94), which correctly checks both:
```rust
let is_vampire = state.get_object(other_creature)
    .and_then(|o| registry.card_data(o.card_id))
    .map(|d| d.subtypes.iter().any(|s| s == "Vampire"))
    .unwrap_or(false);
// Also check instance subtypes on the game object (for tokens, etc.).
let is_vampire = is_vampire || state.get_object(other_creature)
    .map(|o| o.subtypes.iter().any(|s| s == "Vampire"))
    .unwrap_or(false);
```

### 2. BUG (moderate): Continuous effect is set once at equip time, not continuously recalculated

**Oracle text:** "**As long as** equipped creature is a Human, it has lifelink."

The phrase "as long as" indicates a continuous condition that should be checked at all times. However, `update_effects` is only called inside `on_activate_ability` (when the equip ability resolves). If the equipped creature gains or loses the Human subtype after equipping (e.g., via a type-changing effect), the lifelink grant will not update.

This is a known engine-level limitation shared with other equipment cards (Silver-Inlaid Dagger, Sharpened Pitchfork) that use the same `instance_continuous_effects` pattern.

### 3. No issue: Missing test for token/runtime Human

The existing tests (`tier9_equipment.rs` lines 253-295) cover:
- Card data correctness
- Non-Human creature gets +3/+0 but no lifelink
- Human creature (Champion of the Parish) gets +3/+0 and lifelink

No test covers a creature that gains Human subtype at runtime, which would expose issue #1.

## Community Rulings

No official rulings are published for Butcher's Cleaver. The card's behavior is straightforward: the Human check is a continuous condition on the equipped creature's current subtypes.

## Test Coverage

| Test | File | Status |
|------|------|--------|
| `butchers_cleaver_has_correct_data` | tier9_equipment.rs:253 | Covers card data |
| `butchers_cleaver_non_human_gets_power_no_lifelink` | tier9_equipment.rs:264 | Covers non-Human equip |
| `butchers_cleaver_human_gets_power_and_lifelink` | tier9_equipment.rs:281 | Covers Human equip |

Missing test: runtime subtype change after equip.

## Verdict: PASS (with minor bugs)

The core functionality is correct: the card data matches oracle text, equip works at sorcery speed for {3}, +3/+0 is applied to equipped creatures, and Humans get lifelink. The two bugs (registry-only subtype check and one-time effect calculation) are minor edge cases that affect uncommon interactions. The registry-only check should be fixed to also check `obj.subtypes` for consistency with other subtype-checking cards like Wooden Stake.
