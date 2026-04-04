# Audit: Ashmouth Hound

## Reference (Scryfall/API)
- **Name:** Ashmouth Hound
- **Mana Cost:** {1}{R}
- **Type:** Creature — Elemental Dog
- **Oracle:** Whenever Ashmouth Hound blocks or becomes blocked by a creature, Ashmouth Hound deals 1 damage to that creature.
- **P/T:** 2/1

## Implementation: `ashmouth_hound.rs`
- **Name:** Ashmouth Hound -- CORRECT
- **Mana Cost:** {1}{R} -- CORRECT
- **Type:** Creature — Elemental Dog -- CORRECT (subtypes: ["Elemental", "Dog"])
- **P/T:** 2/1 -- CORRECT
- **Triggered abilities:** Blocks + BecomesBlocked -- CORRECT
- **Effect:** Deals 1 damage via `deal_1_damage` helper -- CORRECT
- **NonCombatDamageDealt:** Correctly emits `NonCombatDamageDealt` event -- CORRECT
- **damaged_by tracking:** Pushes source into `damaged_by` -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature.
**Type line**: Creature — Elemental Dog
**Status**: ISSUE
### Code issues
1. **Oracle text wording mismatch (cosmetic)**: Oracle says `"Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature."` but code oracle_text field says `"Whenever Ashmouth Hound blocks or becomes blocked by a creature, Ashmouth Hound deals 1 damage to that creature."` The code uses the old self-referential template instead of the updated "this creature" template.
   - Code: `"Whenever Ashmouth Hound blocks or becomes blocked by a creature, Ashmouth Hound deals 1 damage to that creature."`
   - Oracle: `"Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature."`

Behavior is otherwise correct: two triggered abilities (Blocks and BecomesBlocked) each call deal_1_damage which marks 1 damage on the other creature. Stats (2/1), cost ({1}{R}), subtypes (Elemental, Dog) all match.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature." (was self-referential "Ashmouth Hound" wording). Doc comment updated. Behavior unchanged.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-01

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature.
**Type line**: Creature — Elemental Dog
**Mana cost**: {1}{R}
**P/T**: 2/1
**Ruling**: Ashmouth Hound's ability triggers once for each creature it blocks or becomes blocked by. (2011-09-22)
**Status**: PASS

### Code issues
No issues found.

All card data verified correct:
- Mana cost {1}{R}: `Generic(1), Colored(Red)` -- matches
- Card type Creature: `CardType::Creature` -- matches
- Subtypes Elemental Dog: `["Elemental", "Dog"]` -- matches
- P/T 2/1: `power: Some(2), toughness: Some(1)` -- matches
- Keywords: none in oracle, `vec![]` in code -- matches
- Oracle text field: matches current Scryfall oracle text exactly
- Triggered abilities: `TriggerKind::Blocks` and `TriggerKind::BecomesBlocked` declared, matching the two trigger conditions in the oracle text
- `on_blocks` deals 1 damage to the blocked attacker -- correct
- `on_becomes_blocked` deals 1 damage to the blocking creature -- correct
- Damage emitted as `NonCombatDamageDealt` (correct: this is triggered ability damage, not combat damage)
- `damaged_by` tracked on target creature -- correct
- Zone check ensures target is still on the battlefield before dealing damage -- correct

Ruling compliance: The trigger system iterates over each `(blocker_id, attacker_id)` pair in `BlockersDeclared` assignments. If Ashmouth Hound is blocked by multiple creatures, each pair generates a separate `BecomesBlockedTrigger`, so the ability correctly triggers once per blocker per the ruling.

Note: Doc comment on line 6 says "Elemental Hound" but subtypes in code correctly use "Dog". Non-functional.

### Tricky interactions checked
- Non-combat damage event type (NonCombatDamageDealt vs CombatDamageDealt): PASS -- correctly uses NonCombatDamageDealt
- damaged_by tracking for deathtouch/lifelink interactions: PASS -- source pushed to damaged_by
- Multiple blockers trigger separately (per ruling): PASS -- engine generates one BecomesBlockedTrigger per blocker
- Zone check before dealing damage (creature removed in response): PASS -- checks `obj.zone == Zone::Battlefield`
- Trigger fires as blocker (on_blocks) and as attacker (on_becomes_blocked): PASS -- both hooks implemented

### Test coverage
- Blocking trigger (Ashmouth Hound blocks an attacker): `tier12_cards.rs:169` (ashmouth_hound_deals_damage_on_block)
- Becomes-blocked trigger (Ashmouth Hound attacks and is blocked): NOT TESTED
- Multiple blockers trigger ruling: NOT TESTED
- Fizzle case (target removed before trigger resolves): NOT TESTED

## Audit — 2026-04-02 20:28

**Oracle text source**: Oracle cache (Scryfall API, cached 2026-04-01)
**Oracle text**: Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature.
**Type line**: Creature — Elemental Dog
**Status**: PASS

### Code issues
No issues found.

All card data matches Scryfall oracle:
- Name: "Ashmouth Hound" -- matches
- Mana cost: `Generic(1), Colored(Red)` = {1}{R} -- matches
- Card types: `Creature` -- matches
- Subtypes: `["Elemental", "Dog"]` -- matches oracle type line "Creature — Elemental Dog"
- P/T: `power: Some(2), toughness: Some(1)` = 2/1 -- matches
- Keywords: `vec![]` -- matches (no keywords in oracle)
- Oracle text field: exact match to Scryfall oracle text
- Triggered abilities: `TriggerKind::Blocks` and `TriggerKind::BecomesBlocked` -- correctly models both trigger conditions
- `on_blocks` calls `deal_1_damage(state, self_id, blocked_attacker)` -- deals 1 to the creature Hound blocks
- `on_becomes_blocked` calls `deal_1_damage(state, self_id, blocker_id)` -- deals 1 to the creature blocking Hound
- `deal_1_damage` marks 1 damage, pushes source to `damaged_by`, emits `NonCombatDamageDealt` event, checks zone -- all correct
- Doc comment on line 6 says "Elemental Hound" but code subtypes correctly use "Dog" -- cosmetic only

### Tricky interactions checked
- Non-combat damage event type: PASS -- uses `NonCombatDamageDealt`, not `CombatDamageDealt` (this is triggered ability damage, not combat damage)
- `damaged_by` tracking for death trigger interactions (e.g. Sengir Vampire): PASS -- source correctly pushed to `damaged_by` vec
- Multiple blockers trigger separately per ruling (2011-09-22): PASS -- engine iterates each `(blocker_id, attacker_id)` pair in `BlockersDeclared`, generating one `BecomesBlockedTrigger` per blocker
- Zone check before dealing damage (creature removed in response): PASS -- checks `obj.zone == Zone::Battlefield` before marking damage
- Both trigger directions implemented (blocker and attacker): PASS -- `on_blocks` and `on_becomes_blocked` both implemented and registered via `TriggeredAbilityDef`

### Test coverage
- Blocking trigger (Ashmouth Hound as blocker): `tier12_cards.rs:169` (ashmouth_hound_deals_damage_on_block)
- Becomes-blocked trigger (Ashmouth Hound as attacker): NOT TESTED
- Multiple blockers trigger ruling: NOT TESTED
- Fizzle case (target leaves battlefield before trigger resolves): NOT TESTED
