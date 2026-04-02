# Audit: Full Moon's Rise

## Reference (Scryfall)
- **Name:** Full Moon's Rise
- **Cost:** {1}{G}
- **Type:** Enchantment
- **Oracle:** Werewolf creatures you control get +1/+0 and have trample. Sacrifice Full Moon's Rise: Regenerate all Werewolf creatures you control.
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({1}{G})
- Type: CORRECT (Enchantment)
- Oracle text: PARTIALLY INCORRECT -- the implementation oracle text says "Werewolf creatures" for the static ability but also says "Regenerate all Werewolf creatures" for the sacrifice ability, which matches. However, the code comment at the top says "Werewolf and Wolf creatures" which does not match Oracle.
- P/T: CORRECT (N/A)
- +1/+0 to Werewolf creatures: CORRECT (continuous effect ModifyPT with HasSubtype("Werewolf"))
- Trample to Werewolf creatures: CORRECT (GrantKeyword Trample with HasSubtype("Werewolf"))
- Sacrifice ability: CORRECT (SacrificeCost::SacrificeThis)
- Regeneration effect: The on_activate_ability only regenerates creatures with "Werewolf" subtype.

## Issues
**ISSUE: Code comment says "Werewolf and Wolf" but Oracle only says "Werewolf".** The doc comment at line 9 says "Werewolf and Wolf creatures" but the actual Scryfall oracle text only says "Werewolf creatures." The continuous_effects correctly only filter for HasSubtype("Werewolf"). The activated ability description also mentions "Wolf and Werewolf" but the actual filter in on_activate_ability only checks for "Werewolf" -- so the code behavior is correct, but the comments/descriptions are misleading.

---

## Full Audit (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
> Werewolf creatures you control get +1/+0 and have trample.
> Sacrifice this enchantment: Regenerate all Werewolf creatures you control.

### Official Ruling
> [2011-09-22] In order to regenerate Werewolves involved in combat, you must sacrifice Full Moon's Rise before combat damage is assigned. This means they will lose the +1/+0 and trample bonuses before combat damage assignment.

### Card Data Audit
| Field | Expected | Implemented | Status |
|-------|----------|-------------|--------|
| Name | Full Moon's Rise | "Full Moon's Rise" | CORRECT |
| Cost | {1}{G} | Generic(1), Green | CORRECT |
| Type | Enchantment | Enchantment | CORRECT |
| Supertypes | (none) | (none) | CORRECT |
| Subtypes | (none) | (none) | CORRECT |
| P/T | N/A | None/None | CORRECT |

### Ability Audit

**Static ability 1: +1/+0 to Werewolf creatures you control**
- Implementation: `ContinuousEffect::ModifyPT { power: 1, toughness: 0, scope: Global(And(You, HasSubtype("Werewolf"))) }`
- Status: **CORRECT**

**Static ability 2: Grant trample to Werewolf creatures you control**
- Implementation: `ContinuousEffect::GrantKeyword { keyword: Trample, scope: Global(And(You, HasSubtype("Werewolf"))) }`
- Status: **CORRECT**

**Activated ability: Sacrifice ~ : Regenerate all Werewolf creatures you control**
- Cost: `SacrificeCost::SacrificeThis`, no mana cost, no tap — **CORRECT**
- Zone restriction: battlefield only — **CORRECT**
- Effect: Iterates all battlefield creatures controlled by you with "Werewolf" subtype, adds `regeneration_shields += 1` to each — **CORRECT**
- Targets: None (affects all, not targeted) — **CORRECT**

### Issues Found

**1. Misleading doc comment and ability description (cosmetic, no gameplay impact)**
The doc comment on lines 8-9 reads:
```rust
/// Werewolf and Wolf creatures you control get +1/+0 and have trample.
/// Sacrifice Full Moon's Rise: Regenerate all Werewolf and Wolf creatures you control.
```
The activated ability description on line 58 reads:
```rust
"Sacrifice: Regenerate all Wolf and Werewolf creatures you control"
```
Oracle text says only **"Werewolf creatures"**, not "Werewolf and Wolf creatures." The actual code logic correctly filters only for `HasSubtype("Werewolf")`, so this is purely a comment/string mismatch with no gameplay impact. Wolf creatures without the Werewolf subtype are correctly excluded.

**2. No other issues found.**
The continuous effects have correct scope (your creatures with Werewolf subtype). The sacrifice ability correctly has no mana cost, is not tap-restricted, and applies regeneration shields to all qualifying creatures. The regeneration mechanic (shield-based) is correctly implemented.

### Test Coverage
- `full_moons_rise_card_data` in `mtg-engine/tests/innistrad_simple_cards.rs` — checks type and CMC only.
- **Missing tests:** No test for the +1/+0 buff applying to Werewolves, no test for trample grant, no test for the sacrifice/regeneration ability, no test confirming non-Werewolf creatures are excluded.

### Verdict
**PASS (with cosmetic nits).** The gameplay logic is correct. The doc comment and ability description string incorrectly mention "Wolf" creatures, but the actual filtering logic only targets "Werewolf" creatures as per oracle text. Test coverage is minimal and should be expanded.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Werewolf creatures you control get +1/+0 and have trample.\nSacrifice this enchantment: Regenerate all Werewolf creatures you control.
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found. The comment and activated ability description string mention "Wolf and Werewolf" but the actual filter logic in both continuous_effects and on_activate_ability correctly filters for "Werewolf" only, matching the oracle text. The description strings are cosmetic and do not affect behavior.
