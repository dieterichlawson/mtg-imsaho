## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature enters, you may have it deal 4 damage to target creature with flying.
**Type line**: Artifact Creature — Construct
**Status**: ISSUE

### Code issues

- **Targeting timing violates MTG rules** (mtg-engine/src/cards/isd/geistcatchers_rig.rs:40-60)
  - Oracle text says: `The target creature with flying is chosen when the ability triggers and goes on the stack. You choose whether or not Geistcatcher's Rig will deal 4 damage to it when then ability resolves.`
  - Code does: Both targeting and "may" decision happen during resolution via `ResolutionChoiceKind::ChooseTarget` with `optional: true`

### Tricky interactions checked

- **Target selection timing**: FAIL - targets chosen at resolution instead of when triggered
- **"You may" optionality**: PASS - player can decline via `ChosenTarget(None)`
- **Flying requirement**: PASS - correctly filters with `state.has_keyword(o.id, Keyword::Flying, registry)`
- **Self-targeting exclusion**: PASS - filters out `o.id != object_id`
- **Damage amount**: PASS - correctly deals 4 damage via `PendingEffect::DealDamage { amount: 4 }`
- **Invalid target handling**: FAIL - if no valid targets at resolution, no choice is presented (should be countered due to illegal targets per MTG rules)
- **Information timing**: FAIL - opponents don't know target until resolution (should know when ability triggers)

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- **Target selection timing**: NOT TESTED
- **"You may" choice**: NOT TESTED  
- **Flying requirement**: NOT TESTED
- **Self-targeting exclusion**: NOT TESTED
- **Damage amount**: NOT TESTED
- **Invalid target handling**: NOT TESTED
- **Information timing**: NOT TESTED