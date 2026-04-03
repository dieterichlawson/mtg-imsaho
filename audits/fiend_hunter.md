# Audit: Fiend Hunter

## Reference (Scryfall)
- **Name:** Fiend Hunter
- **Cost:** {1}{W}{W}
- **Type:** Creature -- Human Cleric
- **Oracle:** When Fiend Hunter enters the battlefield, you may exile another target creature. When Fiend Hunter leaves the battlefield, return the exiled card to the battlefield under its owner's control.
- **P/T:** 1/3

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({1}{W}{W})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Human, Cleric)
- Oracle text: CORRECT
- P/T: CORRECT (1/3)
- ETB trigger: CORRECT (TriggerKind::EntersBattlefield)
- LTB trigger: CORRECT (TriggerKind::LeavesBattlefield)
- "You may" optional exile: CORRECT (uses present_optional_target_choice)
- "Another target creature": CORRECT (creature_targets_except excludes self)
- Returns exiled card on LTB: CORRECT (checks Zone::Exile, moves to Battlefield)

## Issues
None found.

## Audit — 2026-04-01 12:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: When this creature enters, you may exile another target creature.\nWhen this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
**Type line**: Creature — Human Cleric
**Status**: ISSUE

### Code issues

1. **LLM card knowledge is inaccurate** (`mtg-player/src/llm.rs:102`)
   - Oracle text says: `"you may exile another target creature"` (any creature, not just opponent's; and it's optional)
   - LLM knowledge says: `"When it enters, exiles an opponent's creature."` — this is wrong in two ways: (a) it says "opponent's creature" but the card can target any creature including your own, and (b) it omits the "you may" optional nature.

2. **Oracle text uses old templating** (`fiend_hunter.rs:27`)
   - Current oracle text: `"When this creature enters, you may exile another target creature. When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control."`
   - Code oracle_text: `"When Fiend Hunter enters the battlefield, you may exile another target creature. When Fiend Hunter leaves the battlefield, return the exiled card to the battlefield under its owner's control."`
   - This is a cosmetic difference from the 2023 templating update. Not a functional issue.

### Tricky interactions checked
- "Another" excludes self: PASS (`creature_targets_except(state, object_id)` excludes Fiend Hunter)
- "You may" is optional: PASS (uses `present_optional_target_choice`)
- Can target own creatures: PASS (`creature_targets_except` includes all creatures on battlefield)
- LTB returns exiled creature: PASS (line 57-65, checks `card_state["exiled_creature"]` and verifies zone is Exile)
- Oblivion Ring trick (LTB before ETB resolves): PASS (if LTB fires before ETB, `card_state` has no "exiled_creature" key, so LTB does nothing; then ETB exiles permanently)
- Token handling per ruling: PASS (tokens in exile are removed by SBA before LTB would typically fire in normal gameplay)
- ETB trigger kind: PASS (TriggerKind::EntersBattlefield)
- LTB trigger kind: PASS (TriggerKind::LeavesBattlefield)

### Test coverage
- ETB exiles a creature: `tier3_cards.rs:211` (fiend_hunter_exiles_on_etb)
- LTB returns exiled creature: `card_mechanics.rs:127` (fiend_hunter_returns_exiled_on_death)
- Can target own creatures: `card_fixes.rs:30` (fiend_hunter_can_target_own_creature)
- Presents choice with multiple targets: `card_fixes.rs:60` (fiend_hunter_presents_choice_with_multiple_targets)
- Declining to exile (choosing "no"): NOT TESTED
- Oblivion Ring trick (LTB before ETB): NOT TESTED
- Token exiled doesn't return: NOT TESTED

## Audit — 2026-04-02

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: When this creature enters, you may exile another target creature. When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
**Type line**: Creature — Human Cleric
**Mana cost**: {1}{W}{W}
**P/T**: 1/3
**Status**: MINOR ISSUES

### Card data verification
- Name: CORRECT
- Cost: CORRECT ({1}{W}{W})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Human, Cleric)
- P/T: CORRECT (1/3)
- ETB trigger: CORRECT (TriggerKind::EntersBattlefield in triggered_abilities)
- LTB trigger: CORRECT (TriggerKind::LeavesBattlefield in triggered_abilities)
- Two separate triggered abilities: CORRECT (both are independent entries in triggered_abilities vec; engine processes them as separate stack objects via PendingTrigger::EnteredBattlefield and PendingTrigger::LeftBattlefield in triggers.rs)
- "Another" self-exclusion: CORRECT (`creature_targets_except(state, object_id)`)
- "You may" optional: CORRECT (`present_optional_target_choice`)
- Can target any creature (not just opponent's): CORRECT

### Issues found

**Issue 1 (Minor): LTB does not return creature under owner's control**

Oracle text: "return the exiled card to the battlefield under its **owner's** control"

Code (`fiend_hunter.rs:56-66`):
```rust
fn on_leave_battlefield(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
    let exiled_id = state.get_object(object_id)
        .and_then(|o| o.card_state.get("exiled_creature").copied());
    if let Some(target_id) = exiled_id {
        if state.get_object(target_id).map(|o| o.zone == Zone::Exile).unwrap_or(false) {
            let name = state.get_object(target_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(target_id, Zone::Battlefield);
```

`move_object` does not reset `controller` to `owner`. If a stolen creature (controller != owner) was exiled by Fiend Hunter, it would return under the wrong player's control. In typical gameplay owner == controller, so this has no observable effect, but the oracle explicitly specifies "under its owner's control."

**Issue 2 (Testing gap): No test for the Oblivion Ring trick**

Official ruling (2018-12-07): "If Fiend Hunter leaves the battlefield before its first ability has resolved, its second ability will trigger and do nothing. Then its first ability will resolve and exile the target creature indefinitely."

The engine architecture supports this correctly — ETB and LTB are separate stack entries (PendingTrigger::EnteredBattlefield at triggers.rs:336, PendingTrigger::LeftBattlefield at triggers.rs:433). If LTB resolves first, `card_state` has no "exiled_creature" key yet, so LTB does nothing; then ETB resolves and exiles the target with no Fiend Hunter on the battlefield to later trigger an LTB return.

However, no test exercises this interaction. A test should verify:
1. Fiend Hunter ETB goes on the stack
2. Fiend Hunter is removed before ETB resolves
3. LTB resolves (does nothing — no stored exile)
4. ETB resolves (exiles target permanently)
5. Target remains in exile indefinitely

**Issue 3 (Cosmetic, previously noted): Oracle text uses older template wording**

Scryfall oracle: "When **this creature** enters, you may exile another target creature."
Code oracle_text: `"When Fiend Hunter enters the battlefield, you may exile another target creature."`

### LLM knowledge check (llm.rs:104)
Current text: "Fiend Hunter ({1}{W}{W} creature 1/3): When it enters, you may exile another target creature (any creature, not just opponent's). When it leaves, the exiled card returns."

This was corrected since the prior audit and is now accurate. It correctly notes the "you may" optional nature and that any creature can be targeted.

### Test coverage
- ETB exiles a creature: `tier3_cards.rs:211` — PASS
- LTB returns exiled creature: `card_mechanics.rs:127` — PASS
- Can target own creatures: `card_fixes.rs:30` — PASS
- Presents choice with multiple targets: `card_fixes.rs:60` — PASS
- Declining to exile (choosing "no"): NOT TESTED
- Oblivion Ring trick (LTB before ETB): NOT TESTED
- Token exiled doesn't return: NOT TESTED
- Return under owner's control (controller != owner): NOT TESTED

### Summary
The implementation is **functionally correct for standard gameplay**. The two separate triggered abilities are properly structured to enable the Oblivion Ring trick. Two minor issues: (1) the LTB does not explicitly reset the returned creature's controller to its owner, and (2) the critical permanent-exile interaction (Oblivion Ring trick) lacks test coverage despite being architecturally supported.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: When this creature enters, you may exile another target creature.
When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
**Type line**: Creature — Human Cleric
**Status**: PASS

### Code issues
No issues found. The two separate triggers (ETB and LTB) correctly model the classic "Oblivion Ring" template where removing Fiend Hunter before the first trigger resolves results in permanent exile. The ETB is correctly optional ("you may"). The LTB correctly returns the card under its owner's control.

## Audit — 2026-04-02 20:07

**Oracle text source**: Oracle cache (Scryfall API, cached 2026-04-01)
**Oracle text**: When this creature enters, you may exile another target creature. When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
**Type line**: Creature — Human Cleric
**Status**: PASS

### Code issues
No issues found.

All card data and behavior match the oracle text:
- Mana cost `{1}{W}{W}`: correct (line 17-21)
- Type `Creature`, subtypes `Human`, `Cleric`: correct (line 22-24)
- P/T `1/3`: correct (line 25-26)
- "you may" optional: correct — uses `present_optional_target_choice` (line 49)
- "another target creature": correct — uses `creature_targets_except(state, object_id)` which excludes self (line 47)
- "exile": correct — `ExileAndStore` calls `state.move_object(*id, Zone::Exile)` (engine.rs:2257)
- "return the exiled card to the battlefield": correct — `state.move_object(target_id, Zone::Battlefield)` (line 62)
- "under its owner's control": correct — `obj.controller = obj.owner` (line 65)
- Two triggered abilities declared: `EntersBattlefield` and `LeavesBattlefield` (lines 31-38)
- LTB correctly checks stored `card_state["exiled_creature"]` and verifies target is still in Exile zone before returning (lines 57-69)

Note: The `oracle_text` display string (line 27) uses older templating ("When Fiend Hunter enters the battlefield") vs current Scryfall text ("When this creature enters"). This is a cosmetic display difference with no gameplay impact.

### Tricky interactions checked
- Oblivion Ring trick (LTB before ETB resolves): PASS — If Fiend Hunter leaves before ETB resolves, `card_state` has no "exiled_creature" key yet, so LTB's `exiled_id` is `None` and does nothing. ETB then resolves and exiles the creature permanently with no future LTB to return it.
- "Another" self-exclusion: PASS — `creature_targets_except(state, object_id)` filters `o.id != exclude`
- "You may" is optional even with 1 target: PASS — `present_optional_target_choice` sets `optional: true`, so auto-apply is skipped (helpers.rs line 129: `if targets.len() == 1 && !optional` is false)
- Can target own creatures: PASS — `creature_targets_except` includes all creatures on battlefield regardless of controller
- Token exile handling: PASS — SBA rule 704.5d (sba.rs:308-312) removes tokens from exile, so the Exile zone check at line 60 fails and no return occurs
- Return under owner's control (not controller's): PASS — `obj.controller = obj.owner` at line 65

### Test coverage
- ETB exiles a creature: `tier3_cards.rs:211` (fiend_hunter_exiles_on_etb)
- LTB returns exiled creature on death: `card_mechanics.rs:127` (fiend_hunter_returns_exiled_on_death)
- Can target own creatures: `card_fixes.rs:30` (fiend_hunter_can_target_own_creature)
- Presents choice with multiple targets: `card_fixes.rs:60` (fiend_hunter_presents_choice_with_multiple_targets)
- Declining to exile ("you may" choosing no): NOT TESTED
- Oblivion Ring trick (LTB before ETB resolves): NOT TESTED
- Token exiled doesn't return: NOT TESTED
- Return under owner's control when controller differs from owner: NOT TESTED

## Audit — 2026-04-02 20:13

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: When this creature enters, you may exile another target creature. When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
**Type line**: Creature — Human Cleric
**Status**: PASS

### Code issues
No issues found.

All card data and behavior match the oracle text:
- Mana cost `{1}{W}{W}`: `ManaCost::new(vec![Generic(1), Colored(White), Colored(White)])` (line 17-21) -- correct
- Type `Creature`, subtypes `Human`, `Cleric`: `card_types: vec![CardType::Creature]`, `subtypes: vec!["Human", "Cleric"]` (lines 22-24) -- correct
- P/T `1/3`: `power: Some(1), toughness: Some(3)` (lines 25-26) -- correct
- "you may": `present_optional_target_choice` passes `optional: true`, so even with 1 target, player always gets a choice (helpers.rs:129 auto-apply requires `!optional`) -- correct
- "another target creature": `creature_targets_except(state, object_id)` filters `o.id != exclude` to exclude self (helpers.rs:176) -- correct
- "exile": `ExileAndStore` effect calls `state.move_object(*id, Zone::Exile)` (engine.rs:2257) and stores the exiled ID in `card_state["exiled_creature"]` (engine.rs:2260) -- correct
- "return the exiled card to the battlefield": `on_leave_battlefield` retrieves stored ID, checks target is still in Exile zone, then calls `state.move_object(target_id, Zone::Battlefield)` (line 62) -- correct
- "under its owner's control": `obj.controller = obj.owner` (line 65) -- correct
- Two separate triggered abilities: `EntersBattlefield` and `LeavesBattlefield` (lines 31-38), processed as independent stack objects -- correct

Note: The `oracle_text` display string (line 27) uses older templating ("When Fiend Hunter enters the battlefield") vs current Scryfall text ("When this creature enters"). This is a cosmetic display difference with no gameplay impact.

### Tricky interactions checked
- Oblivion Ring trick (LTB before ETB resolves): PASS -- If Fiend Hunter leaves before ETB resolves, `card_state` has no "exiled_creature" key yet, so `on_leave_battlefield` gets `None` from `card_state.get("exiled_creature")` and does nothing. ETB then resolves and exiles the creature permanently. The `card_state` is not cleared on zone change (state.rs:478-487 only clears tapped/summoning_sick/damage/counters), so the stored key persists but no future LTB can fire.
- "You may" is optional even with 1 target: PASS -- `present_optional_target_choice` always presents a choice to the player (never auto-applies).
- Can target own creatures: PASS -- `creature_targets_except` includes all creatures on the battlefield regardless of controller.
- Token exile handling: PASS -- If a token is exiled, SBA removes tokens in non-battlefield zones, so the Exile zone check at line 60 will fail and no return occurs. This matches the Scryfall ruling: "If a token is exiled this way, it won't return to the battlefield."
- Return under owner's control (not controller's): PASS -- `obj.controller = obj.owner` at line 65 correctly resets control.

### Test coverage
- ETB exiles a creature: `tier3_cards.rs:211` (fiend_hunter_exiles_on_etb)
- LTB returns exiled creature on death: `card_mechanics.rs:127` (fiend_hunter_returns_exiled_on_death)
- Can target own creatures: `card_fixes.rs:30` (fiend_hunter_can_target_own_creature)
- Presents choice with multiple targets: `card_fixes.rs:60` (fiend_hunter_presents_choice_with_multiple_targets)
- Declining to exile ("you may" choosing no): NOT TESTED
- Oblivion Ring trick (LTB before ETB resolves): NOT TESTED
- Token exiled doesn't return: NOT TESTED

## Audit — 2026-04-02 20:20

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: "When this creature enters, you may exile another target creature. When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control."
**Type line**: Creature — Human Cleric
**Mana cost**: {1}{W}{W}
**P/T**: 1/3
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Oblivion Ring trick (LTB before ETB resolves): PASS -- If Fiend Hunter leaves before ETB resolves, `card_state` has no `"exiled_creature"` key yet, so `on_leave_battlefield` reads `None` from `card_state.get("exiled_creature")` and does nothing. The ETB then resolves and exiles the target permanently with no future LTB to return it. (fiend_hunter.rs:57-58, engine.rs:2259-2260)
- "You may" is optional: PASS -- `present_optional_target_choice` (helpers.rs:148) is used, which passes `true` for the optional parameter, allowing the player to decline.
- "Another" self-exclusion: PASS -- `creature_targets_except(state, object_id)` at helpers.rs:174 filters `o.id != exclude`, correctly preventing Fiend Hunter from targeting itself.
- Can target own creatures: PASS -- `creature_targets_except` includes all battlefield creatures regardless of controller, matching oracle text which does not restrict to opponents.
- Return under owner's control: PASS -- `on_leave_battlefield` sets `obj.controller = obj.owner` (fiend_hunter.rs:65), correctly handling cases where the creature was stolen before being exiled.
- Exiled card moved elsewhere before LTB: PASS -- LTB checks `o.zone == Zone::Exile` (fiend_hunter.rs:60) before returning, so if the exiled card was moved out of exile by another effect, it won't be returned.
- Token exiled doesn't return: ENGINE-LEVEL -- Tokens ceasing to exist in exile is an engine-level concern, not card-level. The card correctly checks `zone == Zone::Exile`.

### Test coverage
- ETB exiles opponent creature: `tier3_cards.rs:211` (fiend_hunter_exiles_on_etb)
- LTB returns exiled creature on death: `card_mechanics.rs:127` (fiend_hunter_returns_exiled_on_death)
- Can target own creatures: `card_fixes.rs:30` (fiend_hunter_can_target_own_creature)
- Presents choice with multiple targets: `card_fixes.rs:60` (fiend_hunter_presents_choice_with_multiple_targets)
- Declining to exile ("you may" choosing no target): NOT TESTED
- Oblivion Ring trick (LTB before ETB resolves): NOT TESTED
- Token exiled doesn't return: NOT TESTED
