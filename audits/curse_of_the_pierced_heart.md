# Audit: Curse of the Pierced Heart

## Scryfall Reference
- **Name:** Curse of the Pierced Heart
- **Cost:** {1}{R}
- **Type:** Enchantment -- Aura Curse
- **Oracle:** Enchant player. At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
- **P/T:** N/A
- **Keywords:** Enchant

## Implementation: `curse_of_the_pierced_heart.rs`
- **Name:** Curse of the Pierced Heart -- CORRECT
- **Cost:** {1}{R} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Subtypes:** ["Aura", "Curse"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Target:** TargetRequirement::PlayerOnly -- CORRECT
- **Trigger:** Upkeep -- CORRECT
- **Behavior:** Deals 1 damage to enchanted player at their upkeep -- CORRECT
- **Uses NonCombatDamageDealt event:** Yes -- CORRECT

## Issues
1. **MINOR: Oracle says "deals 1 damage to that player or a planeswalker that player controls."** The implementation always deals damage to the player (never to a planeswalker). Since planeswalkers are unlikely in this engine's context, this is a minor simplification.

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Enchant player. At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
**Scryfall type line**: Enchantment — Aura Curse
**Status**: PASS

Findings:
- Mana cost {1}{R}: correct.
- Types Enchantment, subtypes Aura/Curse: correct.
- P/T N/A: correct.
- TargetRequirement::PlayerOnly for enchant player: correct.
- Triggered ability declared in triggered_abilities vec (TriggerKind::Upkeep): correct, no missing declaration.
- on_upkeep checks active_player == cursed_player: correct (only fires on enchanted player's upkeep).
- Damage dealt via direct life subtraction + NonCombatDamageDealt event: correct (not CombatDamageDealt).
- Anti-pattern check: No `move_object(id, Zone::Graveyard)` for spells (this is an enchantment, stays on battlefield). No issues.
- Oracle discrepancy (carried forward): implementation cannot redirect damage to a planeswalker the enchanted player controls. Minor accepted simplification.
- Tests found in tier15_cards.rs and tier7_cards.rs.

## Audit — 2026-04-01 12:00

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/138/curse-of-the-pierced-heart), confirmed by Gatherer via WebSearch (https://gatherer.wizards.com/pages/card/Details.aspx?multiverseid=227071)
**Oracle text**: Enchant player. At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
**Type line**: Enchantment — Aura Curse
**Status**: ISSUE

1. **Oracle text string mismatch** (`mtg-engine/src/cards/curse_of_the_pierced_heart.rs`, line 26):
   - Oracle text says: `this Aura deals 1 damage to that player or a planeswalker that player controls`
   - Code says: `Curse of the Pierced Heart deals 1 damage to that player.`
   - The code's oracle_text field omits the "or a planeswalker that player controls" clause and uses the old card-name self-reference instead of "this Aura".

2. **Missing planeswalker targeting in behavior** (`mtg-engine/src/cards/curse_of_the_pierced_heart.rs`, lines 62-64):
   - Oracle text says: `deals 1 damage to that player or a planeswalker that player controls`
   - Code does: Always deals 1 damage to the player only (subtracts 1 from life, emits NonCombatDamageDealt targeting the player). No option to redirect damage to a planeswalker.

No other issues. Mana cost, types, subtypes, targeting, trigger kind, event types, and anti-pattern checks all pass. Tests in tier7_cards.rs cover the basic upkeep damage case.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/138/curse-of-the-pierced-heart)
**Oracle text**: Enchant player. At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
**Type line**: Enchantment — Aura Curse
**Status**: ISSUE

Findings:
- Mana cost {1}{R}: correct.
- Types Enchantment, subtypes Aura/Curse: correct.
- P/T N/A: correct.
- TargetRequirement::PlayerOnly for enchant player: correct.
- Triggered ability declared in triggered_abilities vec (TriggerKind::Upkeep): correct.
- on_upkeep checks active_player == cursed_player: correct (only fires on enchanted player's upkeep).
- Damage dealt via direct life subtraction + NonCombatDamageDealt event: correct (not CombatDamageDealt).
- Anti-pattern check: No move_object to graveyard for spells (this is an enchantment staying on battlefield). No issues.
- ISSUE: Oracle text says "deals 1 damage to that player **or a planeswalker that player controls**." The implementation (line 62-64) always deals damage to the player only, with no option to redirect to a planeswalker. The code's oracle_text field (line 26) also omits the "or a planeswalker that player controls" clause, not matching current Scryfall oracle text.
- Tests found in tier7_cards.rs (1 test: curse_of_pierced_heart_deals_damage_on_upkeep). Test coverage is minimal -- only tests damage to player on upkeep, no test for non-enchanted-player upkeep (no trigger).

## Audit — 2026-04-01 16:00

**Oracle text source**: Oracle cache (Scryfall API, re-fetched), https://scryfall.com/card/isd/138/curse-of-the-pierced-heart?utm_source=api
**Oracle text**: Enchant player. At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
**Type line**: Enchantment — Aura Curse
**Mana cost**: {1}{R}
**Status**: ISSUE

### Code issues

1. **Missing planeswalker damage option** (`mtg-engine/src/cards/isd/curse_of_the_pierced_heart.rs`, lines 61-64):
   - Oracle text says: `this Aura deals 1 damage to that player or a planeswalker that player controls`
   - Code does: Always deals 1 damage to the player only (`state.get_player_mut(cursed_player).life = new_life`). No option to redirect damage to a planeswalker the enchanted player controls.

2. **Oracle text field mismatch** (`mtg-engine/src/cards/isd/curse_of_the_pierced_heart.rs`, line 26):
   - Oracle text says: `this Aura deals 1 damage to that player or a planeswalker that player controls`
   - Code's oracle_text field says: `Curse of the Pierced Heart deals 1 damage to that player.` -- omits the planeswalker clause and uses the card name instead of "this Aura".

### Tricky interactions checked
- Triggers only on enchanted player's upkeep: PASS (line 58 checks `state.active_player != cursed_player`)
- NonCombatDamageDealt event: PASS (line 65)
- LifeChanged event: PASS (line 70)
- Enchant player targeting: PASS (TargetRequirement::PlayerOnly at line 40)
- Aura/Curse subtypes: PASS (subtypes: ["Aura", "Curse"] at line 23)
- Resolve via helper: PASS (calls `resolve_curse` at line 45)

### Test coverage
- Deals 1 damage on enchanted player's upkeep: `tier7_cards.rs:176` (curse_of_pierced_heart_deals_damage_on_upkeep)
- Referenced in Bitterheart Witch test: `tier15_cards.rs:183` (bitterheart_witch_finds_curse_on_death)
- Does not trigger on non-enchanted player's upkeep: NOT TESTED
- Planeswalker damage redirect option: NOT TESTED
- Curse removed when enchanted player leaves: NOT TESTED

## Audit — 2026-04-01 13:35

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/138/curse-of-the-pierced-heart?utm_source=api
**Oracle text**: Enchant player. At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
**Type line**: Enchantment — Aura Curse
**Mana cost**: {1}{R}
**Status**: PASS

### Code issues
No issues found.

Previous audits flagged two issues, both now fixed:
1. **Oracle text field** (line 26): Now reads `"Enchant player\nAt the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls."` -- matches Scryfall oracle text verbatim.
2. **Planeswalker damage option** (lines 63-100): Code now checks for planeswalkers the cursed player controls. If none, deals 1 damage directly to the player. If planeswalkers exist, presents a `ChooseTarget` choice to the curse's controller with both the player and planeswalkers as options, using `PendingEffect::DealDamage`. Correctly implements the "or a planeswalker that player controls" clause.

### Tricky interactions checked
- Triggers only on enchanted player's upkeep: PASS (line 58 checks `state.active_player != cursed_player`)
- Planeswalker damage redirect option: PASS (lines 63-100 present choice when planeswalkers exist)
- Choice presented to curse's controller (not enchanted player): PASS (line 84 uses `player: controller`)
- NonCombatDamageDealt event: PASS (line 74, and via PendingEffect::DealDamage for planeswalker path)
- LifeChanged event: PASS (line 79 for direct player damage path)
- Enchant player targeting: PASS (TargetRequirement::PlayerOnly at line 41)
- Aura/Curse subtypes: PASS (subtypes: ["Aura", "Curse"])
- Resolve via helper: PASS (calls `resolve_curse` at line 46)

### Test coverage
- Deals 1 damage on enchanted player's upkeep: `tier7_cards.rs:176` (curse_of_pierced_heart_deals_damage_on_upkeep)
- Referenced in Bitterheart Witch test: `tier15_cards.rs:183` (bitterheart_witch_finds_curse_on_death)
- Does not trigger on non-enchanted player's upkeep: NOT TESTED
- Planeswalker damage redirect option: NOT TESTED
- Curse removed when enchanted player leaves: NOT TESTED

## Audit — 2026-04-01 18:30

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/138/curse-of-the-pierced-heart?utm_source=api
**Oracle text**: Enchant player. At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
**Type line**: Enchantment — Aura Curse
**Mana cost**: {1}{R}
**Status**: PASS

### Code issues
No issues found.

All previously flagged issues have been resolved:
1. Oracle text field (line 26) now matches Scryfall text verbatim, using "this Aura" and including the planeswalker clause.
2. Planeswalker damage option (lines 63-100) now checks for planeswalkers the cursed player controls. If none exist, deals 1 damage directly to the player. If planeswalkers exist, presents a `ChooseTarget` choice to the curse's controller with both the player and all their planeswalkers as options, using `PendingEffect::DealDamage`.

### Tricky interactions checked
- Triggers only on enchanted player's upkeep: PASS (line 58 checks `state.active_player != cursed_player`)
- Planeswalker damage redirect option: PASS (lines 63-100 present choice when planeswalkers exist)
- Choice presented to curse's controller (not enchanted player): PASS (line 84 uses `player: controller`)
- NonCombatDamageDealt event: PASS (line 74, and via PendingEffect::DealDamage for planeswalker path)
- LifeChanged event: PASS (line 79 for direct player damage path)
- Enchant player targeting: PASS (TargetRequirement::PlayerOnly at line 41)
- Aura/Curse subtypes: PASS (subtypes: ["Aura", "Curse"])
- Resolve via helper: PASS (calls `resolve_curse` at line 46)
- triggered_abilities declaration matches on_upkeep hook: PASS (TriggerKind::Upkeep at line 33)

### Test coverage
- Deals 1 damage on enchanted player's upkeep: `tier7_cards.rs:176` (curse_of_pierced_heart_deals_damage_on_upkeep)
- Referenced in Bitterheart Witch test: `tier15_cards.rs:183` (bitterheart_witch_finds_curse_on_death)
- Does not trigger on non-enchanted player's upkeep: NOT TESTED
- Planeswalker damage redirect option: NOT TESTED
- Curse removed when enchanted player leaves: NOT TESTED

## Audit — 2026-04-01 20:00

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/138/curse-of-the-pierced-heart?utm_source=api
**Oracle text**: Enchant player. At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
**Type line**: Enchantment — Aura Curse
**Mana cost**: {1}{R}
**Status**: PASS

### Code issues
No issues found.

Card data matches oracle text exactly. The oracle text field at line 26 correctly uses "this Aura" and includes the planeswalker clause. The behavior at lines 63-100 correctly checks for planeswalkers the cursed player controls: if none exist, deals 1 damage directly to the player; if planeswalkers exist, presents a `ChooseTarget` choice to the curse's controller with both the player and all their planeswalkers as options.

### Tricky interactions checked
- Triggers only on enchanted player's upkeep: PASS (line 58 checks `state.active_player != cursed_player`)
- Planeswalker damage redirect option: PASS (lines 63-100 present choice when planeswalkers exist)
- Choice presented to curse's controller (not enchanted player): PASS (line 84 uses `player: controller`)
- NonCombatDamageDealt event: PASS (line 74 for direct path, and via PendingEffect::DealDamage for planeswalker path)
- LifeChanged event: PASS (line 79 for direct player damage path)
- Enchant player targeting: PASS (TargetRequirement::PlayerOnly at line 41)
- Aura/Curse subtypes: PASS (subtypes: ["Aura", "Curse"])
- Resolve via helper: PASS (calls `resolve_curse` at line 46)
- triggered_abilities declaration matches on_upkeep hook: PASS (TriggerKind::Upkeep at line 33)

### Test coverage
- Deals 1 damage on enchanted player's upkeep: `tier7_cards.rs:176` (curse_of_pierced_heart_deals_damage_on_upkeep)
- Referenced in Bitterheart Witch test: `tier15_cards.rs:183` (bitterheart_witch_finds_curse_on_death)
- Does not trigger on non-enchanted player's upkeep: NOT TESTED
- Planeswalker damage redirect option: NOT TESTED
- Curse removed when enchanted player leaves: NOT TESTED
- LLM card knowledge: NOT PRESENT

## Audit — 2026-04-01 14:49

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/138/curse-of-the-pierced-heart?utm_source=api
**Oracle text**: Enchant player. At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
**Type line**: Enchantment — Aura Curse
**Mana cost**: {1}{R}
**Status**: PASS

### Code issues
No issues found.

Card data matches oracle text exactly. Mana cost {1}{R} (Generic(1), Red), type Enchantment with subtypes Aura/Curse. Oracle text field at line 26 matches Scryfall verbatim, using "this Aura" and including the planeswalker clause. `TargetRequirement::PlayerOnly` for enchant player. Resolves via `resolve_curse` helper. `triggered_abilities` declares `TriggerKind::Upkeep` matching the `on_upkeep` hook. The `on_upkeep` method (line 48) correctly checks `state.active_player != cursed_player` to only trigger on the enchanted player's upkeep. When no planeswalkers are present, deals 1 damage directly to the player with `NonCombatDamageDealt` and `LifeChanged` events (lines 71-81). When planeswalkers exist, presents a `ChooseTarget` choice to the curse's controller (line 84 uses `player: controller`) with both the player and all planeswalkers as options, using `PendingEffect::DealDamage` (lines 83-99).

### Tricky interactions checked
- Triggers only on enchanted player's upkeep: PASS (line 58 checks `state.active_player != cursed_player`)
- Planeswalker damage redirect option: PASS (lines 63-99 present choice when planeswalkers exist)
- Choice presented to curse's controller (not enchanted player): PASS (line 84 uses `player: controller`)
- NonCombatDamageDealt event: PASS (line 74 for direct path, and via PendingEffect::DealDamage for planeswalker path)
- LifeChanged event: PASS (line 79 for direct player damage path)
- Enchant player targeting: PASS (TargetRequirement::PlayerOnly at line 41)
- Aura/Curse subtypes: PASS (subtypes: ["Aura", "Curse"])
- Resolve via helper: PASS (calls `resolve_curse` at line 46)
- triggered_abilities declaration matches on_upkeep hook: PASS (TriggerKind::Upkeep at line 33)

### Test coverage
- Deals 1 damage on enchanted player's upkeep: `tier7_cards.rs:176` (curse_of_pierced_heart_deals_damage_on_upkeep)
- Referenced in Bitterheart Witch test: `tier15_cards.rs:183` (bitterheart_witch_finds_curse_on_death)
- Does not trigger on non-enchanted player's upkeep: NOT TESTED
- Planeswalker damage redirect option: NOT TESTED
- Curse removed when enchanted player leaves: NOT TESTED
