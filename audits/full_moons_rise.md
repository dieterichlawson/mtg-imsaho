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

## Audit — 2026-04-02 21:03
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Werewolf creatures you control get +1/+0 and have trample.\nSacrifice this enchantment: Regenerate all Werewolf creatures you control.
**Type line**: Enchantment
**Status**: PASS

### Code issues
1. **Doc comment says "Werewolf and Wolf" but Oracle says only "Werewolf" (cosmetic).** Lines 8-9 of the file read `Werewolf and Wolf creatures you control get +1/+0 and have trample` and `Regenerate all Werewolf and Wolf creatures you control`. The actual oracle text says only "Werewolf creatures." The code logic correctly filters for `HasSubtype("Werewolf")` only, so no gameplay impact.
2. **Activated ability description string mentions "Wolf" (UI cosmetic).** Line 58: `"Sacrifice: Regenerate all Wolf and Werewolf creatures you control"` -- this is displayed to the player, but oracle text says only "Werewolf creatures."
3. **on_activate_ability uses inline subtype check instead of `state.matches_filter()` (latent fragility).** The regeneration effect (lines 74-88) manually checks `registry.card_data()` and `o.subtypes` for "Werewolf", but does NOT check `back_face_data()` for transformed DFCs. The engine's `matches_filter(HasSubtype(...))` correctly handles transformed creatures. Currently all Innistrad Werewolves have "Werewolf" on both faces, so this is not a gameplay bug with the current card pool, but it diverges from how the continuous effects resolve the same filter.
4. **All gameplay-affecting logic is correct.** The +1/+0 buff, trample grant, sacrifice cost, regeneration shield application, zone restriction (battlefield only), no mana/tap cost, and "affects all not targeted" semantics are all correctly implemented per oracle text and rulings.

### Tricky interactions checked (min 3)
1. **Sacrifice timing in combat (official ruling).** The ruling says you must sacrifice before combat damage to regenerate, losing the +1/+0 and trample. The implementation correctly uses `SacrificeCost::SacrificeThis` which removes the enchantment (and its continuous effects) before the regeneration shields are applied. Shields persist through the turn, so the regeneration works correctly when damage is later assigned.
2. **Transformed Werewolves.** The continuous effects use `HasSubtype("Werewolf")` which goes through `state.matches_filter()`, correctly checking back face subtypes for transformed DFCs. All current Innistrad Werewolves have "Werewolf" on their front face as well, so the inline check in `on_activate_ability` also works.
3. **Regeneration shield mechanics.** Verified in `destruction.rs`: regeneration shields are checked during `try_destroy()`, consuming one shield to tap the creature, remove damage, and remove it from combat -- matching MTG rules for regeneration.
4. **Non-Werewolf creatures excluded.** The filter only matches creatures with the "Werewolf" subtype. Creatures that are Wolves but not Werewolves are correctly excluded per oracle text.

### Test coverage
- `full_moons_rise_card_data` in `mtg-engine/tests/innistrad_simple_cards.rs` -- checks type and CMC only.
- **Missing tests:** No test for +1/+0 buff, trample grant, sacrifice/regeneration ability, or non-Werewolf exclusion.
