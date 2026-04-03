# Audit: Ghostly Possession

## Oracle Reference (Scryfall)
- Cost: {2}{W}
- Type: Enchantment -- Aura
- Oracle: "Enchant creature
  Enchanted creature has flying.
  Prevent all combat damage that would be dealt to and dealt by enchanted creature."

## Implementation: ghostly_possession.rs

## Issues Found

1. **MINOR: Oracle text missing "Enchant creature"** - The implementation oracle_text doesn't include "Enchant creature" as the first line. This is the enchant keyword ability that defines what the aura can attach to. However, the target_requirement is correctly set to Creature, so functionally this is fine.

Otherwise correct: cost ({2}{W}), type (Enchantment), subtype (Aura), flying grant via ContinuousEffect::GrantKeyword, combat damage prevention via ContinuousEffect::PreventCombatDamage with EffectScope::Attached.

## Verdict: PASS (1 minor oracle text omission)

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
```
Enchant creature
Enchanted creature has flying.
Prevent all combat damage that would be dealt to and dealt by enchanted creature.
```

### Findings
- Name, cost ({2}{W}), type (Enchantment -- Aura) all match.
- target_requirement correctly set to Creature (covers "Enchant creature").
- Grants Flying via ContinuousEffect::GrantKeyword to Attached scope -- correct.
- Prevents combat damage via ContinuousEffect::PreventCombatDamage to Attached scope -- correct.
- Resolves via resolve_aura helper -- correct.

### Verdict: PASS

---

## Audit — 2026-04-02 21:09

**Oracle text source**: Scryfall API (cached 2026-04-01), https://scryfall.com/card/isd/18/ghostly-possession
**Oracle text**: "Enchant creature\nEnchanted creature has flying.\nPrevent all combat damage that would be dealt to and dealt by enchanted creature."
**Type line**: Enchantment — Aura

**Status**: PASS

### Code issues

1. **MINOR (cosmetic): Oracle text missing "Enchant creature" prefix** — The implementation's `oracle_text` field is `"Enchanted creature has flying. Prevent all combat damage that would be dealt to and dealt by enchanted creature."` but the official oracle text starts with `"Enchant creature\n..."`. Some other auras in the codebase (Dead Weight, Wreath of Geists, Curiosity, Sensory Deprivation, Claustrophobia) include it; others (Pacifism, Holy Strength, Furor of the Bitten, etc.) omit it. This is a codebase-wide inconsistency, not specific to this card. Functionally irrelevant since `target_requirement()` returns `TargetRequirement::Creature` and `resolve_aura()` handles attachment correctly.

No functional issues found.

### Tricky interactions checked (min 3)

1. **Combat damage prevented in both directions**: `deal_damage_to_creature()` in `combat.rs` checks `has_damage_prevention()` for both `source` and `target`, so damage is prevented both TO and FROM the enchanted creature. `deal_damage_to_player()` also checks the source. Confirmed correct.

2. **Non-combat damage NOT prevented**: Spell damage (e.g., Geistflame) goes through `helpers::resolve_damage()` which does NOT check `PreventCombatDamage`. This is correct — Ghostly Possession only prevents combat damage, not all damage.

3. **Effect stops when aura is removed**: `has_continuous_effect()` iterates only battlefield objects and checks `attached_to` at runtime via `EffectScope::Attached`. If the aura leaves the battlefield, the effect no longer applies. Correct.

4. **Flying grant via continuous effect**: Uses `ContinuousEffect::GrantKeyword { keyword: Keyword::Flying, scope: EffectScope::Attached }`, checked by `has_keyword()` which calls `has_continuous_effect()`. Correctly applies only to the enchanted creature.

### Test coverage

- `innistrad_cards.rs::ghostly_possession_grants_flying` — Verifies the enchanted creature gains flying.
- `card_mechanics.rs::ghostly_possession_prevents_damage` — Verifies combat damage is prevented both TO and FROM the enchanted creature (checks `damage_marked == 0` for both attacker and blocker).
