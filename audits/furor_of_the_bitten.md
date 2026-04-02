# Audit: Furor of the Bitten

## Reference (Scryfall)
- **Name:** Furor of the Bitten
- **Cost:** {R}
- **Type:** Enchantment -- Aura
- **Oracle:** Enchant creature. Enchanted creature gets +2/+2 and attacks each combat if able.
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({R})
- Type: CORRECT (Enchantment)
- Subtypes: CORRECT (Aura)
- Oracle text: CORRECT
- P/T: CORRECT (N/A)
- +2/+2 to enchanted creature: CORRECT (ModifyPT power:2, toughness:2, scope: Attached)
- Attacks each combat if able: CORRECT (ForceAttack, scope: Attached)
- Target requirement: CORRECT (Creature)
- Resolves as aura: CORRECT (resolve_aura)

## Issues
None found.

## Audit — 2026-04-02

### Oracle Text (Scryfall, cached 2026-04-01)

```
Enchant creature
Enchanted creature gets +2/+2 and attacks each combat if able.
```

Ruling (2020-06-23): If the enchanted creature can't attack for any reason (such as being tapped or having come under that player's control that turn), then it doesn't attack. If there's a cost associated with having it attack, the player isn't forced to pay that cost, so it doesn't have to attack in that case either.

### Implementation Review

**File:** `mtg-engine/src/cards/isd/furor_of_the_bitten.rs`

| Aspect | Oracle | Implementation | Status |
|---|---|---|---|
| Name | Furor of the Bitten | `"Furor of the Bitten"` | PASS |
| Mana cost | {R} | `ManaSymbol::Colored(Color::Red)` | PASS |
| Type line | Enchantment — Aura | `CardType::Enchantment`, subtypes `["Aura"]` | PASS |
| Enchant creature | "Enchant creature" | `TargetRequirement::Creature` + `resolve_aura` helper | PASS |
| +2/+2 | "gets +2/+2" | `ModifyPT { power: 2, toughness: 2, scope: Attached }` | PASS |
| Forced attack | "attacks each combat if able" | `ForceAttack { scope: Attached }` | PASS |
| "If able" condition | Creature doesn't attack if tapped, summoning sick, or has Defender | Engine checks tapped, summoning_sick, Defender before forcing (engine.rs:1630-1638) | PASS |
| Oracle text field | "Enchant creature\nEnchanted creature gets +2/+2 and attacks each combat if able." | `"Enchanted creature gets +2/+2 and attacks each combat if able."` (missing "Enchant creature\n" prefix) | MINOR — matches dominant codebase convention (most auras omit it), but Curiosity includes it |

### Forced Attack — "If Able" Cost Exemption

The Scryfall ruling says the player is not forced to pay costs to attack. The engine implementation does not currently model attack costs, so this ruling is not testable but also not violated.

### Test Coverage

- `innistrad_cards::furor_of_the_bitten_gives_plus_two` — verifies +2/+2 via cast-and-resolve. PASS.
- `card_mechanics::furor_forces_attack` — verifies forced attack on enchanted creature. PASS.
- `bug_fixes::furor_of_the_bitten_gives_plus_two_and_forces_attack` — verifies both +2/+2 and ForceAttack effect. PASS.

### Verdict

**PASS** — Implementation is correct. The card faithfully implements all oracle text abilities. The only minor note is the `oracle_text` field omits the "Enchant creature" line, which is consistent with most other auras in the codebase but inconsistent with Curiosity. No functional impact.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Enchant creature\nEnchanted creature gets +2/+2 and attacks each combat if able.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found.
