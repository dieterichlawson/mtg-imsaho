## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature has flying.
Prevent all combat damage that would be dealt to and dealt by enchanted creature.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "Prevent all combat damage dealt TO enchanted creature": `deal_damage_to_creature` in `combat.rs:430` calls `has_damage_prevention(state, target, registry)`, so if the enchanted creature is the *target* of damage (e.g., a blocker receiving attacker damage), the damage is prevented. Pass.
- "Prevent all combat damage dealt BY enchanted creature": `deal_damage_to_creature` in `combat.rs:430` also calls `has_damage_prevention(state, source, registry)`, so if the enchanted creature is the *source* of damage to another creature, the damage is prevented. Pass.
- "Prevent all combat damage dealt BY enchanted creature (unblocked attacker to player)": `deal_damage_to_player` in `combat.rs:496` calls `has_damage_prevention(state, source, registry)`, so if the enchanted creature is the unblocked attacker dealing damage to the defending player, the damage is still prevented. Pass.
- Flying granted via continuous effect (not snapshot): `has_keyword` in `state.rs:987` calls `has_continuous_effect` which dynamically evaluates `EffectScope::Attached` every time — not cached at ETB. Pass.
- `EffectScope::Attached` evaluation: `effect_applies_to` in `state.rs:700` checks `source.attached_to == creature_id`, which is set by `resolve_aura`. Pass.
- Aura falls off when enchanted creature leaves battlefield: SBA rule 704.5m in `sba.rs:152` detects that `attached_to` references a target no longer on the battlefield and moves the aura to the graveyard. Pass.
- Aura goes to graveyard if target left battlefield before resolution: `resolve_aura` in `helpers.rs:18` checks the target is still on the battlefield before attaching; otherwise calls `move_spell_after_resolve`. Pass.
- "Enchant creature" targeting: `TargetRequirement::Creature` — can enchant any creature (no restriction to own or opponent's). This matches oracle text (which says "Enchant creature" with no qualifier). Pass.

### Test coverage
- Flying granted to enchanted creature: `innistrad_cards.rs:373` (`ghostly_possession_grants_flying`) — TESTED
- Combat damage prevented TO enchanted creature (blocker): `card_mechanics.rs:277` (`ghostly_possession_prevents_damage`) — TESTED
- Combat damage prevented FROM enchanted creature (attacker to blocker): `card_mechanics.rs:277` (`ghostly_possession_prevents_damage`) — TESTED
- Combat damage prevented FROM enchanted creature attacking unblocked (damage to player): NOT TESTED
- Aura falls off when enchanted creature leaves battlefield: NOT TESTED
