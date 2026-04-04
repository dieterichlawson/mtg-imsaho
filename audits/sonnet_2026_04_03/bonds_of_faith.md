## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
**Type line**: Enchantment — Aura
**Status**: ISSUE

### Code issues

- **Critical: "As long as" condition not continuously evaluated** (/Users/dlaw/mtg/mtg-engine/src/cards/isd/bonds_of_faith.rs:60-61)
  - Oracle text says: `Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.`
  - Code does: Sets `obj.instance_continuous_effects = Some(effects)` once at ETB based on a single subtype check, never re-evaluates the condition. If a Human creature transforms into a non-Human (werewolf transformation), the effect remains static instead of changing from +2/+2 to "can't attack or block".

- **Incomplete subtype checking for tokens** (/Users/dlaw/mtg/mtg-engine/src/cards/isd/bonds_of_faith.rs:43-46)
  - Oracle text says: `as long as it's a Human`
  - Code does: Only checks `registry.card_data(o.card_id).map(|d| d.subtypes.iter().any(|s| s == "Human"))` but doesn't check runtime `obj.subtypes`. Compare with `check_condition` in state.rs which correctly checks both registry and runtime subtypes. Human tokens would not be recognized properly.

- **Oracle text field incomplete** (/Users/dlaw/mtg/mtg-engine/src/cards/isd/bonds_of_faith.rs:25)
  - Oracle text says: `Enchant creature\nEnchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.`
  - Code does: `oracle_text: "Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.".into()` (missing the "Enchant creature" prefix)

### Tricky interactions checked
- **"As long as" continuous evaluation**: FAIL - Effect is set once at ETB, not continuously re-evaluated when creature type changes
- **Transformation during combat ruling**: FAIL - Code doesn't handle the case where a Human transforms mid-combat (should remain in combat but lose +2/+2 bonus per 2011-09-22 ruling)
- **Human token recognition**: FAIL - Only checks registry subtypes, misses tokens with Human as runtime subtype
- **Combat declaration timing**: NOT TESTED - Oracle ruling states creature already declared as attacker/blocker stays in combat when losing Human type
- **Effect switching between +2/+2 and "can't attack/block"**: FAIL - Static effect never switches between modes

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Basic Human gets +2/+2**: `mtg-engine/tests/bug_fixes.rs:522` / `mtg-engine/tests/card_mechanics.rs:197`
- **Non-Human gets locked down**: `mtg-engine/tests/bug_fixes.rs:546` / `mtg-engine/tests/card_mechanics.rs:217`
- **Werewolf transformation (Human to non-Human)**: NOT TESTED
- **Transform during combat remains in combat**: NOT TESTED
- **Human tokens properly recognized**: NOT TESTED
- **Effect re-evaluation when creature type changes**: NOT TESTED

Sources:
- [Magic Rules Tips: Bonds of Faith will give +2/+2 to a Human Werewolf](https://blogs.magicjudges.org/rulestips/2011/09/bonds-of-faith-will-give-22-to-a-human-werewolf/)