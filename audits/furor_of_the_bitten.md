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

## Audit — 2026-04-02 21:03
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Enchant creature\nEnchanted creature gets +2/+2 and attacks each combat if able.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
- `oracle_text` field is `"Enchanted creature gets +2/+2 and attacks each combat if able."` but should be `"Enchant creature\nEnchanted creature gets +2/+2 and attacks each combat if able."` per Scryfall. Other auras in the codebase (Dead Weight, Curiosity, Wreath of Geists, Sensory Deprivation, Claustrophobia) include the "Enchant creature\n" prefix. This is a cosmetic/display issue only -- targeting is correctly handled via `TargetRequirement::Creature` and `resolve_aura`. No functional impact.

### Tricky interactions checked (min 3)
1. **Tapped creature cannot be forced to attack**: Engine (engine.rs ~line 1827) skips tapped creatures when collecting forced attackers. Correctly implements the ruling.
2. **Summoning sick creature cannot be forced to attack**: Engine (engine.rs ~line 1827) skips summoning sick creatures. Correctly implements the ruling.
3. **Creature with Defender cannot be forced to attack**: Engine (engine.rs ~line 1834) checks for Defender keyword and skips. Correct.
4. **Aura fizzles when target leaves battlefield**: `resolve_aura` helper checks target is still on battlefield before attaching; otherwise aura goes to graveyard. Correct.
5. **Vigilance interaction**: Forced attackers with Vigilance are not tapped when forced to attack (engine.rs ~line 1864). Correct.

### Test coverage
- `innistrad_cards::furor_of_the_bitten_gives_plus_two` -- +2/+2 stat boost via cast-and-resolve on a 1/1 creature (verifies 3/3). PASS.
- `bug_fixes::furor_of_the_bitten_gives_plus_two_and_forces_attack` -- verifies both +2/+2 and presence of ForceAttack continuous effect. PASS.
- `card_mechanics::furor_forces_attack` -- verifies creature is auto-added as attacker even when player declares zero attackers. PASS.
