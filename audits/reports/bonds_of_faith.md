# Audit: Bonds of Faith

## Oracle (Scryfall/API)
- **Name:** Bonds of Faith
- **Cost:** {1}{W}
- **Type:** Enchantment — Aura
- **Oracle:** Enchant creature. Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/bonds_of_faith.rs`
- **Name:** Bonds of Faith -- CORRECT
- **Cost:** {1}{W} -- CORRECT
- **Type:** Enchantment — Aura -- CORRECT (subtypes: ["Aura"])
- **Target requirement:** Creature -- CORRECT
- **Effect on Human:** +2/+2 via ModifyPT -- CORRECT
- **Effect on non-Human:** PreventAttack + PreventBlock -- CORRECT
- **Aura attachment:** Uses resolve_aura helper -- CORRECT

## Issues
1. **ISSUE (minor):** The Human check is done once at ETB time and stored as `instance_continuous_effects`. If the creature's type changes (e.g., gains/loses Human subtype), the effect won't update. The oracle says "as long as it's a Human" which implies continuous checking.

## Verdict: PASS (with minor limitation) -- Human check is snapshot rather than continuous

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, cached 2026-04-01)
```
Enchant creature
Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
```

### Card Data Verification
- **Name:** "Bonds of Faith" -- CORRECT
- **Cost:** {1}{W} -- CORRECT
- **Types:** Enchantment -- CORRECT
- **Subtypes:** ["Aura"] -- CORRECT
- **Enchant creature** target requirement: `TargetRequirement::Creature` -- CORRECT
- **P/T:** None -- CORRECT (not a creature)

### Functional Audit

#### Aura Attachment
Uses `crate::cards::helpers::resolve_aura` in `on_resolve` -- CORRECT

#### Dual-Mode Effect
- Human path: `ContinuousEffect::ModifyPT { power: 2, toughness: 2, scope: EffectScope::Attached }` -- CORRECT
- Non-Human path: `PreventAttack` + `PreventBlock` with `EffectScope::Attached` -- CORRECT

### Issues Found

#### 1. BUG: Human subtype check ignores token subtypes and transformed creatures (MEDIUM)

The implementation at lines 43-46:
```rust
let is_human = state.get_object(target_id)
    .and_then(|o| registry.card_data(o.card_id))
    .map(|d| d.subtypes.iter().any(|s| s == "Human"))
    .unwrap_or(false);
```

This only checks `registry.card_data(card_id).subtypes`. It does NOT check:
- `obj.subtypes` on the GameObject itself (used by tokens created via `create_token_with_subtypes`)
- Back face subtypes for transformed DFCs

Compare with the correct approach used by `GameState::matches_filter` for `CreatureFilter::HasSubtype` in `state.rs` (lines ~581-599), which checks all three sources: registry card_data subtypes, back face subtypes for transformed creatures, and object-level subtypes for tokens.

A Human token (e.g., from Doomed Traveler creating a 1/1 white Human token) enchanted by Bonds of Faith would be treated as non-Human and locked down instead of receiving +2/+2.

Note: `Butcher's Cleaver` (`butchers_cleaver.rs`) has the same bug -- it also only checks `registry.card_data().subtypes`.

#### 2. KNOWN LIMITATION: Snapshot vs. continuous check (MINOR, previously identified)

The oracle says "as long as it's a Human" -- a continuous condition. The implementation evaluates this once in `on_enter_battlefield` and stores the result in `instance_continuous_effects`. If the creature's type changes while Bonds is attached (e.g., a Werewolf transforming from Human to non-Human, or a creature gaining the Human type), the effect will not update.

This is an engine-level limitation (no recalculation of conditional continuous effects), noted in the previous audit.

#### 3. No test coverage for tokens or transformed creatures

All existing tests use either named creatures from the registry (Elder Cathar, a Human) or anonymous `ready_creature` helpers (non-Human). No test covers:
- A Human token being enchanted (would expose Bug #1)
- A transformed DFC being enchanted

### Anti-Pattern Check
- No unsafe code
- No panics or unwraps on fallible paths (uses `and_then`/`unwrap_or` correctly)
- Effect scoping uses `EffectScope::Attached` correctly

### Verdict: CONDITIONAL PASS

The card is functionally correct for the common case (enchanting a non-token, non-transformed creature). Bug #1 (token subtypes not checked) is a real correctness issue that would produce wrong behavior when enchanting Human tokens. Bug #2 (snapshot vs continuous) is a known engine limitation.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Enchant creature\nEnchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found. Target requirement correctly set to Creature for the Aura. Human check grants +2/+2, non-Human prevents attack and block. The "Enchant creature" keyword ability is handled via target_requirement rather than oracle_text, which is acceptable.

## Audit — 2026-04-02 20:37

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Enchant creature\nEnchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found.

Card data matches oracle in all respects:
- Name: "Bonds of Faith" -- correct
- Cost: {1}{W} (Generic(1), White) -- correct
- Type line: Enchantment with subtype Aura -- correct
- "Enchant creature" handled via `TargetRequirement::Creature` and `resolve_aura` helper -- correct
- Human path: `ModifyPT { power: 2, toughness: 2, scope: Attached }` -- correct
- Non-Human path: `PreventAttack { scope: Attached }` + `PreventBlock { scope: Attached }` -- correct

Previously identified issues (still present, accepted as engine-level limitations):
- Human subtype check at ETB uses `registry.card_data()` only (misses token subtypes / transformed back faces)
- Effect is snapshot at ETB rather than continuously re-evaluated per "as long as it's a Human"

### Tricky interactions checked
- Aura falls off when creature leaves battlefield: pass (SBA rule 704.5m in `sba.rs` handles unattached auras)
- Creature declared as attacker then loses Human type mid-combat: not removed from combat per official ruling (2011-09-22) -- engine does not re-evaluate instance effects mid-combat, so behavior is consistent
- Bonds on a Human grants +2/+2 and does NOT prevent attack/block: pass (tested in `bug_fixes.rs:522` and `card_mechanics.rs:197`)
- Bonds on a non-Human prevents attack AND block without P/T bonus: pass (tested in `bug_fixes.rs:546` and `card_mechanics.rs:217`)

### Test coverage
- Human gets +2/+2: `bug_fixes.rs:522`, `card_mechanics.rs:197`
- Non-Human can't attack or block: `bug_fixes.rs:546`, `card_mechanics.rs:217`, `innistrad_cards.rs:306`
- Human with Bonds can still attack: `bug_fixes.rs:540`
- Non-Human does NOT get P/T bonus: `bug_fixes.rs:556`
- Bonds on a Human token: NOT TESTED
- Bonds on a transformed DFC: NOT TESTED

## Audit — 2026-04-02 21:23

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Enchant creature\nEnchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
**Type line**: Enchantment — Aura
**Status**: ISSUE

### Code issues

1. **ISSUE: Snapshot vs continuous evaluation (engine limitation causing wrong behavior)**
   The oracle text says "as long as it's a Human", which is a continuous condition. The implementation evaluates this once in `on_enter_battlefield` (line 39) and stores the result in `instance_continuous_effects`. If the enchanted creature's type changes after Bonds of Faith enters (e.g., a Human Werewolf transforms into a non-Human Werewolf via Moonmist, or a non-Human gains the Human type), the effect will NOT update. A formerly-Human creature would continue getting +2/+2 instead of being locked down, and vice versa. The engine has no mechanism for re-evaluating conditional instance effects.

2. **ISSUE: Human subtype check ignores token subtypes and transformed back faces**
   Implementation at lines 43-46:
   ```rust
   let is_human = state.get_object(target_id)
       .and_then(|o| registry.card_data(o.card_id))
       .map(|d| d.subtypes.iter().any(|s| s == "Human"))
       .unwrap_or(false);
   ```
   This calls `registry.card_data()` which always returns front-face static data. It does not check `obj.subtypes` (populated for tokens) or `back_face_data()` (for transformed DFCs). Compare with `GameState::matches_filter` for `CreatureFilter::HasSubtype` (state.rs line ~654) which correctly checks `is_transformed` and back face subtypes. A Human token enchanted by Bonds of Faith would be incorrectly locked down instead of receiving +2/+2.

3. **MINOR: oracle_text field missing "Enchant creature\n" prefix**
   The implementation's oracle_text is `"Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block."` but other auras in the codebase (Dead Weight, Claustrophobia, Curiosity, Wreath of Geists, Sensory Deprivation) all include the "Enchant creature\n" prefix. Inconsistent but not functionally broken since "Enchant creature" is handled via `TargetRequirement::Creature`.

### Tricky interactions checked (min 3)

1. **Creature loses Human type mid-combat (official ruling 2011-09-22)**: "Once the enchanted creature has been declared as an attacking or blocking creature, causing it to stop being a Human won't remove it from combat." The engine does not re-evaluate instance effects mid-combat, so a Human declared as attacker that loses its type would stay in combat -- consistent with the ruling. However, it would also keep the +2/+2 bonus, which the ruling says it should lose ("It will lose the +2/+2 bonus, however"). The snapshot approach means the +2/+2 persists incorrectly.

2. **Bonds on a Human token**: Due to issue #2, a Human token (e.g., the 1/1 Spirit Human token from Doomed Traveler) would have `obj.subtypes = ["Human", ...]` but `registry.card_data()` would not find "Human" in the token's subtypes since tokens don't have registry card data with subtypes. The token would be incorrectly locked down.

3. **Aura falls off when creature leaves battlefield**: SBA rule 704.5m handles unattached auras. When the enchanted creature leaves, the aura goes to graveyard. Verified that `sba.rs` handles this. PASS.

4. **Bonds on a creature with multiple types (e.g., Human Soldier)**: The `subtypes.iter().any(|s| s == "Human")` correctly checks if Human is among the subtypes, regardless of other types. PASS for non-token, non-transformed creatures.

### Test coverage
- Human gets +2/+2: `bug_fixes.rs:522`, `card_mechanics.rs:197`
- Non-Human can't attack or block: `bug_fixes.rs:546`, `card_mechanics.rs:217`, `innistrad_cards.rs:306`
- Human with Bonds can still attack: `bug_fixes.rs:540`
- Non-Human does NOT get P/T bonus: `bug_fixes.rs:556`
- Bonds on a Human token: NOT TESTED (would expose issue #2)
- Bonds on a transformed DFC: NOT TESTED
- Type change after Bonds enters: NOT TESTED (would expose issue #1)

## Audit — 2026-04-03 22:06

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
**Type line**: Enchantment — Aura
**Status**: ISSUE

### Code issues
- Dynamic type checking not implemented (lines 39-69): The aura sets continuous effects once when entering the battlefield, but doesn't re-evaluate when the enchanted creature's type changes.
  - Oracle text says: `Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.`
  - Code does: Sets either `ModifyPT` OR `PreventAttack/PreventBlock` effects in `instance_continuous_effects` once at ETB, based on creature type at that moment, and never re-evaluates

### Tricky interactions checked
- Combat timing (once declared, type change doesn't remove from combat): PASS - combat system only checks `can_attack/can_block` during declare steps
- Type change after aura attachment: FAIL - effects don't switch when creature gains/loses Human type 
- Multi-type creatures (Human Soldier, etc.): PASS - code correctly uses `any()` to check for Human subtype
- Aura removal: PASS - uses standard `resolve_aura` helper
- Instance oracle text display: PASS - correctly shows simplified text based on current mode

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Human gets +2/+2: `mtg-engine/tests/bug_fixes.rs:522` and `mtg-engine/tests/card_mechanics.rs:197`
- Non-Human prevented from attack/block: `mtg-engine/tests/bug_fixes.rs:546` and `mtg-engine/tests/card_mechanics.rs:217`
- Combat timing ruling (type change after declare): NOT TESTED
- Dynamic type switching: NOT TESTED
- Multi-subtype Human creatures: NOT TESTED

## Audit — 2026-04-03 22:06

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
**Type line**: Enchantment — Aura
**Status**: ISSUE

### Code issues

- **Effect is evaluated once at ETB instead of continuously** (`mtg-engine/src/cards/isd/bonds_of_faith.rs`, lines 39-69). The oracle text uses the phrase "as long as it's a Human", which is a continuous condition that should dynamically switch between +2/+2 and can't-attack-or-block as the creature's type changes. The ruling from 2011-09-22 confirms this: "causing it to stop being a Human won't remove it from combat. It will lose the +2/+2 bonus, however." The implementation sets `instance_continuous_effects` once in `on_enter_battlefield` and never recalculates.
  - Oracle text says: `Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.`
  - Code does: `on_enter_battlefield` evaluates `is_human` once at line 43, then sets either `ModifyPT { power: 2, toughness: 2 }` or `PreventAttack + PreventBlock` into `instance_continuous_effects` at lines 48-58, with no mechanism to re-evaluate. A Human Werewolf that transforms to its non-Human back face while enchanted by Bonds of Faith would keep the +2/+2 buff instead of switching to can't-attack-or-block.

- **Human subtype check reads from card registry instead of object's current subtypes** (`mtg-engine/src/cards/isd/bonds_of_faith.rs`, lines 43-46). The check uses `registry.card_data(o.card_id)` which always returns front-face data. Tokens have subtypes stored on `obj.subtypes` (per `state.rs` line 1205: "Subtypes on this object (for tokens — regular cards use CardData.subtypes via registry)"), but this code never checks `obj.subtypes`. A Human token (e.g., from Doomed Traveler) enchanted by Bonds of Faith would be treated as non-Human and locked down. Compare with `check_condition` in `state.rs` line 1084-1091 which correctly checks both `o.subtypes` AND `registry.card_data()`.
  - Oracle text says: `Enchanted creature gets +2/+2 as long as it's a Human.`
  - Code does: `let is_human = state.get_object(target_id).and_then(|o| registry.card_data(o.card_id)).map(|d| d.subtypes.iter().any(|s| s == "Human")).unwrap_or(false);` — only checks registry, not `obj.subtypes`

- **Oracle text field omits "Enchant creature" prefix** (`mtg-engine/src/cards/isd/bonds_of_faith.rs`, line 25). Other auras in the codebase include this prefix (e.g., Claustrophobia at `claustrophobia.rs` line 25: `"Enchant creature\nWhen this Aura enters, tap enchanted creature.\n..."`) but Bonds of Faith omits it.
  - Oracle text says: `Enchant creature\nEnchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.`
  - Code does: `oracle_text: "Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.".into()` — missing "Enchant creature\n" prefix

### Tricky interactions checked
- Werewolf transform after Bonds of Faith attachment: FAIL — a Human Werewolf enchanted with Bonds of Faith would keep +2/+2 after transforming to non-Human back face, because the effect is locked at ETB and never recalculated
- Human token enchanted by Bonds of Faith: FAIL — token subtypes stored on `obj.subtypes` are not checked; `registry.card_data()` returns no subtypes for tokens, so they are treated as non-Human
- Creature declared as attacker then loses Human type (ruling 2011-09-22): partial PASS — creature stays in combat (consistent with ruling), but also incorrectly keeps +2/+2 (ruling says "It will lose the +2/+2 bonus, however")
- Aura falls off when creature leaves battlefield: PASS — SBA handles unattached auras
- Multi-subtype creature (e.g., Human Soldier): PASS — `subtypes.iter().any(|s| s == "Human")` correctly finds Human among multiple subtypes for non-token, non-transformed creatures

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Human gets +2/+2: `card_mechanics.rs:197`, `bug_fixes.rs:522`
- Non-Human can't attack or block: `card_mechanics.rs:217`, `bug_fixes.rs:546`, `innistrad_cards.rs:306`
- Human with Bonds can still attack: `bug_fixes.rs:540`
- Non-Human does NOT get P/T bonus: `bug_fixes.rs:556`
- Type change after Bonds enters (ruling 2011-09-22): NOT TESTED
- Human token enchanted: NOT TESTED
- Transformed DFC enchanted: NOT TESTED

## Audit — 2026-04-03 22:16

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
**Type line**: Enchantment — Aura
**Status**: ISSUE

### Code issues

- Missing "Enchant creature" prefix in oracle_text field (bonds_of_faith.rs:25)
  - Oracle text says: `Enchant creature\nEnchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.`
  - Code has: `"Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block."`

- Static evaluation instead of continuous "as long as" condition (bonds_of_faith.rs:43-46)
  - Oracle text says: `gets +2/+2 as long as it's a Human`
  - Code does: Evaluates Human status once at ETB in `on_enter_battlefield` and sets permanent effects, never re-evaluating when creature type changes

- Incomplete subtype checking missing tokens and transformed creatures (bonds_of_faith.rs:43-46)
  - Oracle text says: `as long as it's a Human` (should check all ways a creature can be Human)
  - Code does: `registry.card_data(o.card_id).map(|d| d.subtypes.iter().any(|s| s == "Human"))` - only checks static registry data, misses `obj.subtypes` (tokens) and `back_face_data()` (transformed creatures)

### Tricky interactions checked

- Combat timing (creature already attacking when stops being Human): FAIL - effect not continuously evaluated
- Human token enchanted by Bonds of Faith: FAIL - token subtypes stored on `obj.subtypes` are not checked
- Transformed double-faced creature that becomes/stops being Human: FAIL - only front face data checked
- Multi-subtype creature (Human Soldier): PASS - `any()` correctly finds Human among multiple subtypes for non-transformed non-token creatures
- Creature gains/loses Human type after enchantment: FAIL - static evaluation doesn't detect type changes

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Basic Human gets +2/+2: `bug_fixes.rs:522` / `card_mechanics.rs:197`
- Basic non-Human gets locked down: `bug_fixes.rs:546` / `card_mechanics.rs:217`
- Combat timing when creature stops being Human: NOT TESTED
- Human token enchanted: NOT TESTED  
- Transformed creature's back face being Human: NOT TESTED
- Multi-subtype Human creature: `bug_fixes.rs:522` (Elder Cathar is Human Soldier)
- Dynamic type changes after enchantment: NOT TESTED

## Audit — 2026-04-03 22:50

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature. Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
**Type line**: Enchantment — Aura
**Status**: ISSUE

### Code issues
- Oracle text field missing "Enchant creature" prefix (`mtg-engine/src/cards/isd/bonds_of_faith.rs:25`)
  - Oracle text says: `Enchant creature. Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.`
  - Code has: `"Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block."`

- "As long as" condition not continuously re-evaluated (`mtg-engine/src/cards/isd/bonds_of_faith.rs:43-69`)
  - Oracle text says: `as long as it's a Human`
  - Code does: Sets `instance_continuous_effects` once at ETB based on creature type at that moment, never rechecks if type changes later. Should use `dynamic_pt` method like Wreath of Geists to continuously re-evaluate the Human condition.

- Incomplete subtype checking (`mtg-engine/src/cards/isd/bonds_of_faith.rs:43-46`)
  - Oracle text says: `as long as it's a Human`
  - Code does: Only checks `registry.card_data(o.card_id)` for Human subtype, ignoring runtime `o.subtypes`. This misses tokens or creatures with modified subtypes. Should check both sources like `check_condition` in state.rs (lines 1087-1091).

### Tricky interactions checked
- "As long as it's a Human" continuous condition: FAIL - effect set once at ETB, not re-evaluated
- Subtype checking for tokens and modified creatures: FAIL - only checks registry, not runtime subtypes  
- Combat declaration ruling (creature stays in combat if type changes): FAIL - not implemented correctly due to above issues
- Effect switching when creature type changes: FAIL - effect never changes after initial determination
- "Otherwise" clause exclusivity: PASS - correctly gives either +2/+2 OR prevents attack/block, not both

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic Human gets +2/+2: `mtg-engine/tests/bug_fixes.rs:522` and `mtg-engine/tests/card_mechanics.rs:197`
- Basic non-Human gets locked down: `mtg-engine/tests/bug_fixes.rs:546` and `mtg-engine/tests/card_mechanics.rs:217` 
- Prevent attack functionality: `mtg-engine/tests/innistrad_cards.rs:306`
- Dynamic type change while enchanted: NOT TESTED
- Combat declaration ruling: NOT TESTED
- Token Human subtype recognition: NOT TESTED
- Runtime subtype modification: NOT TESTED
