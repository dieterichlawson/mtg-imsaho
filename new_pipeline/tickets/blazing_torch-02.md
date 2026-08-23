---
id: blazing_torch-02
status: new
card: Blazing Torch
audit_run_id: 2026-04-19-blazing_torch-audit
audit_model: sonnet
audit_tokens: 23515
audit_duration: 1971
---

## Audit Finding

**Oracle text:**
> Blazing Torch deals 2 damage to any target.

**Code:**
> .filter(|o| o.power.is_some() || o.card_types.contains(&CardType::Planeswalker))

**Description:**
The AnyTarget branch of generate_ability_targets (engine.rs:2057) filters battlefield permanents to creatures (power.is_some()) and planeswalkers (card_types.contains(&CardType::Planeswalker)). Since non-token permanents have empty card_types on the battlefield object, the planeswalker check always returns false for non-token planeswalkers, making them untargetable by Blazing Torch's damage ability. The PlayerOrPlaneswalker branch (engine.rs:2043-2045) was already patched with a registry fallback (registry.card_data(obj.card_id).is_some_and(|d| d.card_types.contains(&CardType::Planeswalker))), but the AnyTarget branch was not given the same treatment. Per oracle text, Blazing Torch's ability should be able to target any target including non-token planeswalkers.

**Engine path:** mtg-engine/src/engine.rs:2057

**Required check:** 8f

**Affected cards:**
- Heretic's Punishment

## Tests

### can_target_nontoken_planeswalker
Scenario: Blazing Torch is equipped to a creature; the controller activates the damage ability; a non-token planeswalker on the battlefield should appear as a valid target option.

