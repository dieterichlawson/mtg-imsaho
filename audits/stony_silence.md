# Audit: Stony Silence

## Oracle (Scryfall)
- **Name:** Stony Silence
- **Cost:** {1}{W}
- **Type:** Enchantment
- **Oracle:** Activated abilities of artifacts can't be activated.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/stony_silence.rs`
- **Name:** Stony Silence ✅
- **Cost:** {1}{W} ✅
- **Type:** Enchantment ✅
- **Oracle text:** matches ✅

### Issue
- **NOT IMPLEMENTED:** The card's static ability (preventing activation of artifact abilities) is not enforced by the engine. The code comments explicitly document this as a known limitation. The card exists for deck building and oracle text purposes only.

## Verdict: ISSUE -- static ability not enforced (documented known limitation)

## Audit -- 2026-04-01 09:00

**Scryfall Oracle text**: Activated abilities of artifacts can't be activated.
**Scryfall type line**: Enchantment
**Status**: ISSUE

Findings:
1. **Mana cost {1}{W}**: Correct.
2. **Type (Enchantment)**: Correct. No subtypes, no supertypes. Correct.
3. **Oracle text**: Matches Scryfall exactly.
4. **Static ability not enforced**: The code comments (lines 7-11) explicitly document that the engine lacks an ability restriction system. The card is registered for deck building/oracle purposes only. The static ability has no effect in game.
5. **No anti-patterns detected**: Card has no on_resolve (enchantments enter battlefield via default), no triggered abilities, no damage. Clean implementation of a stub.
6. **Tests**: Found in `mtg-engine/tests/innistrad_simple_cards.rs`.

Issues:
- Static ability ("Activated abilities of artifacts can't be activated") is not enforced (documented known limitation).

## Audit — 2026-04-01 14:13

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/36/stony-silence)
**Oracle text**: Activated abilities of artifacts can't be activated.
**Type line**: Enchantment
**Mana cost**: {1}{W}
**Status**: ISSUE

Findings:
1. **Name**: "Stony Silence" -- correct.
2. **Mana cost {1}{W}**: Correct (Generic(1), White).
3. **Type (Enchantment)**: Correct. No subtypes, no supertypes -- correct.
4. **Oracle text**: Matches Scryfall exactly.
5. **Static ability not enforced**: The code comments (lines 7-11) explicitly document that the engine lacks an ability restriction system. The card is registered for deck building/oracle purposes only. The static ability has zero in-game effect.
6. **No anti-patterns**: No on_resolve (enchantments enter battlefield), no triggered abilities, no damage. Clean stub.
7. **Tests**: Found in `mtg-engine/tests/innistrad_simple_cards.rs`. Test only verifies card data (type, cost). No test for the static ability effect (as expected, since it's not implemented).

Issues:
- Static ability ("Activated abilities of artifacts can't be activated") is not enforced. Documented as known limitation in code comments. Rulings confirm this affects mana abilities and only artifacts on the battlefield.

## Audit — 2026-04-01 12:00

**Oracle text source**: Scryfall via WebSearch
**Oracle text**: Activated abilities of artifacts can't be activated.
**Type line**: Enchantment
**Status**: ISSUE

Mana cost {1}{W}: correct (Generic(1), White). Type Enchantment: correct. No subtypes or supertypes: correct. Oracle text string matches Scryfall exactly. No P/T: correct. No flashback: correct. No triggered abilities: correct.

Per Scryfall rulings: Activated abilities contain a colon; no abilities of artifacts can be activated including mana abilities; only affects artifacts on the battlefield; triggered abilities are unaffected.

Tests in `tests/innistrad_simple_cards.rs` (line 586): verifies card data (type is Enchantment). No test for the static ability effect.

Issues found:
1. **Static ability not enforced** (`/home/user/mtg-imsaho/mtg-engine/src/cards/stony_silence.rs`, lines 7-11):
   - Oracle text says: `Activated abilities of artifacts can't be activated.`
   - Code does: The card is registered with correct card data but the static ability has no in-game effect. The code comments document this as a known limitation: "the engine doesn't have an ability restriction system." No `continuous_effects` entry or other mechanism prevents artifact activated abilities from being used. This means artifacts on the battlefield can freely activate abilities while Stony Silence is in play.

## Audit — 2026-04-01 18:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Activated abilities of artifacts can't be activated.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues
Card data verified correct: name "Stony Silence", mana cost {1}{W} (Generic(1), White), type Enchantment, no subtypes, no supertypes, oracle text matches exactly.

The code comment in `stony_silence.rs` (lines 7-11) claims the engine does not enforce this ability, but this is a **stale comment**. The engine DOES implement Stony Silence's restriction in `engine.rs` (lines 253-273): it checks whether any "Stony Silence" is on the battlefield and skips artifact activated abilities during legal action generation. However:

1. **Mana abilities of artifacts are not blocked** (`/Users/dlaw/mtg/mtg-engine/src/engine.rs`, lines 229-246 vs 253-273):
   - Oracle ruling says: `No abilities of artifacts can be activated, including mana abilities.`
   - Code does: The Stony Silence check (lines 253-273) only applies to non-mana activated abilities. The mana abilities section (lines 229-246) does NOT check for Stony Silence, meaning artifact mana abilities (e.g., Sol Ring's `{T}: Add {C}{C}`) can still be activated while Stony Silence is on the battlefield. Per the Scryfall ruling, mana abilities should also be blocked.

2. **Stale code comment** (`/Users/dlaw/mtg/mtg-engine/src/cards/isd/stony_silence.rs`, lines 7-11):
   - Code says: `Known limitation: the engine doesn't have an ability restriction system. This card is registered for deck building and oracle text purposes, but its static ability is not enforced.`
   - Reality: The engine DOES enforce the restriction for non-mana activated abilities in `engine.rs` lines 253-273. The comment is outdated and misleading.

### Tricky interactions checked
- Non-mana activated abilities of artifacts blocked: pass (engine.rs lines 267-273)
- Mana abilities of artifacts blocked: ISSUE (not implemented, see #1)
- Only affects artifacts on the battlefield (ruling): pass (action generation only checks battlefield permanents)
- Triggered abilities unaffected (ruling): pass (only activated abilities are checked)
- Stony Silence checks by name, not card type: pass (hardcoded name check is functional)

### Test coverage
- Card data verification: `tests/innistrad_simple_cards.rs` (line 586)
- Artifact non-mana activated abilities blocked: NOT TESTED
- Artifact mana abilities blocked: NOT TESTED
- Non-artifact abilities unaffected: NOT TESTED
- Triggered abilities of artifacts unaffected: NOT TESTED

## Re-Audit — 2026-04-01 20:00

**Oracle text source**: Scryfall API (via oracle_lookup.py)
**Oracle text**: Activated abilities of artifacts can't be activated.
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found. All previously reported issues have been fixed:

1. **Static ability is now enforced by the engine** (engine.rs lines 229-282): The engine checks for any "Stony Silence" on the battlefield and blocks both mana abilities (lines 238-244) and non-mana activated abilities (lines 276-282) of artifacts. The artifact check examines both the card's registered types and the object's runtime types (`obj.card_types`).

2. **Code comment is now accurate** (stony_silence.rs lines 6-9): Previously said the ability was not enforced; now correctly states "Enforced by the engine in legal_actions(): when Stony Silence is on the battlefield, both mana abilities and non-mana activated abilities of artifacts are excluded from the legal action list."

3. **Previous audit false positive corrected**: The 2026-04-01 18:00 audit incorrectly claimed mana abilities were not blocked. The code at lines 238-244 clearly blocks mana abilities of artifacts when Stony Silence is active. The test `stony_silence_blocks_artifact_mana_abilities` confirms this.

All card data verified correct: name "Stony Silence", mana cost {1}{W} (Generic(1), White), type Enchantment, no subtypes, no supertypes, no P/T, oracle text matches exactly.

Per Scryfall rulings:
- "Activated abilities contain a colon": engine only blocks activated abilities (cost:effect pattern), not triggered abilities. Correct.
- "No abilities of artifacts can be activated, including mana abilities": both mana and non-mana abilities blocked. Correct.
- "Only affects artifacts on the battlefield": engine only generates actions for battlefield permanents. Correct.
- "Triggered abilities are unaffected": engine does not block triggered abilities. Correct.

### Tricky interactions checked
- Mana abilities of artifacts blocked: pass (engine.rs lines 238-244)
- Non-mana activated abilities of artifacts blocked: pass (engine.rs lines 276-282)
- Non-artifact mana abilities unaffected: pass (only skips when is_artifact is true)
- Only affects battlefield artifacts (ruling): pass (action generation only checks battlefield)
- Triggered abilities unaffected (ruling): pass (only activated abilities checked)
- Both players' artifacts affected: pass (stony_silence_active checks all battlefield objects regardless of controller)

### Test coverage
- Card data verification: `tests/innistrad_simple_cards.rs:586`
- Artifact mana abilities blocked: `tests/innistrad_simple_cards.rs:595`
- Non-artifact mana abilities unaffected: `tests/innistrad_simple_cards.rs:625`
- Artifact non-mana activated abilities blocked: NOT TESTED
- Triggered abilities of artifacts unaffected: NOT TESTED
- Opponent's artifacts also blocked: NOT TESTED

## Audit — 2026-04-01 21:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Activated abilities of artifacts can't be activated.
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found. Card data and engine enforcement both verified correct:

- Name "Stony Silence": correct
- Mana cost {1}{W}: correct (Generic(1), White)
- Type Enchantment: correct
- No subtypes, no supertypes: correct
- No P/T: correct (not a creature)
- Oracle text matches Scryfall exactly: correct
- No keywords, no flashback, no triggered abilities: correct

Engine enforcement (engine.rs lines 229-282):
- `stony_silence_active` check scans all battlefield objects regardless of controller (line 230-232): correct (symmetric effect)
- Mana abilities of artifacts blocked (lines 238-244): correct per ruling "No abilities of artifacts can be activated, including mana abilities"
- Non-mana activated abilities of artifacts blocked (lines 276-282): correct
- Artifact detection checks both registry card types and runtime `obj.card_types` (lines 240-243 and 277-280): correct (handles tokens and type-changing effects)
- Non-artifact permanents unaffected: correct (only skips when `is_artifact` is true)
- Only affects battlefield artifacts: correct (action generation only iterates `objects_in_zone(Zone::Battlefield, player)`)
- Triggered abilities unaffected: correct (engine only gates activated ability actions, not trigger processing)

Code comment in `stony_silence.rs` (lines 6-9) accurately describes the enforcement mechanism: correct.

### Tricky interactions checked
- Mana abilities of artifacts blocked: pass (engine.rs lines 238-244, confirmed by test)
- Non-mana activated abilities of artifacts blocked: pass (engine.rs lines 276-282)
- Non-artifact mana abilities unaffected: pass (confirmed by test)
- Only affects battlefield artifacts (ruling): pass
- Triggered abilities unaffected (ruling): pass
- Both players' artifacts affected: pass (stony_silence_active checks all objects)
- Detection by name ("Stony Silence"): acceptable simplification for this engine

### Test coverage
- Card data verification: `tests/innistrad_simple_cards.rs:586`
- Artifact mana abilities blocked: `tests/innistrad_simple_cards.rs:595`
- Non-artifact mana abilities unaffected: `tests/innistrad_simple_cards.rs:625`
- Artifact non-mana activated abilities blocked: NOT TESTED
- Triggered abilities of artifacts unaffected: NOT TESTED
- Opponent's artifacts also blocked: NOT TESTED

## Audit — 2026-04-01 22:00

**Oracle text source**: Scryfall API (via oracle_lookup.py cache)
**Oracle text**: Activated abilities of artifacts can't be activated.
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found. All card data and engine enforcement verified correct:

- Name "Stony Silence": correct
- Mana cost {1}{W}: correct (Generic(1), White)
- Type Enchantment: correct
- No subtypes, no supertypes: correct
- No P/T: correct (not a creature)
- Oracle text matches Scryfall exactly: correct
- No keywords, no flashback, no triggered abilities, no continuous_effects: correct

Engine enforcement (engine.rs lines 229-282):
- `stony_silence_active` check scans all battlefield objects for name "Stony Silence" regardless of controller (line 230-232): correct (symmetric effect, affects both players' artifacts)
- Mana abilities of artifacts blocked (lines 238-244): correct per ruling "No abilities of artifacts can be activated, including mana abilities"
- Non-mana activated abilities of artifacts blocked (lines 276-282): correct
- Artifact detection checks both registry card types and runtime `obj.card_types` (lines 240-243 and 277-280): correct (handles artifact creatures, tokens, and type-changing effects)
- Non-artifact permanents unaffected: correct (only skips when `is_artifact` is true)
- Only affects battlefield artifacts: correct per ruling (action generation only iterates `objects_in_zone(Zone::Battlefield, player)` for activated abilities)
- Triggered abilities unaffected: correct per ruling (engine only gates activated ability actions, not trigger processing)

Code comment in `stony_silence.rs` (lines 6-9) accurately describes the enforcement mechanism.

Per Stony Silence + artifact lands interaction (from judges' blog): artifact lands have their mana abilities blocked by Stony Silence because they are artifacts. The engine handles this correctly since the artifact type check (`d.card_types.contains(&CardType::Artifact)`) would match artifact lands.

### Tricky interactions checked
- Mana abilities of artifacts blocked: pass (engine.rs lines 238-244, confirmed by test)
- Non-mana activated abilities of artifacts blocked: pass (engine.rs lines 276-282)
- Non-artifact mana abilities unaffected: pass (confirmed by test)
- Only affects battlefield artifacts (ruling): pass
- Triggered abilities unaffected (ruling): pass
- Both players' artifacts affected: pass (stony_silence_active checks all objects)
- Artifact creatures' abilities also blocked: pass (artifact type check catches artifact creatures)
- Detection by name ("Stony Silence"): acceptable simplification for this engine

### Test coverage
- Card data verification: `tests/innistrad_simple_cards.rs:586`
- Artifact mana abilities blocked: `tests/innistrad_simple_cards.rs:595`
- Non-artifact mana abilities unaffected: `tests/innistrad_simple_cards.rs:625`
- Artifact non-mana activated abilities blocked: NOT TESTED
- Triggered abilities of artifacts unaffected: NOT TESTED
- Opponent's artifacts also blocked: NOT TESTED
- Artifact creatures' activated abilities blocked: NOT TESTED
- Not in LLM card knowledge: acceptable (static effect, AI reads oracle text)

## Audit — 2026-04-01 16:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Activated abilities of artifacts can't be activated.
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found.

Card data is correct: {1}{W} Enchantment, oracle text matches. The enforcement is entirely in the engine at engine.rs:234-287. The engine checks if any Stony Silence is on the battlefield (by name, regardless of controller), then skips both mana abilities (line 244) and non-mana activated abilities (line 281) for artifacts. The artifact check correctly looks at both registry card_data types and instance card_types (to handle cards whose types may have been modified). The check applies to both players since legal_actions is called for each player when they have priority. The effect only applies to battlefield artifacts (since legal_actions only generates actions for battlefield permanents), which is correct per the ruling: "Stony Silence's ability affects only artifacts on the battlefield."

### Tricky interactions checked
- Blocks mana abilities of artifacts: PASS - engine.rs:244
- Blocks non-mana activated abilities of artifacts: PASS - engine.rs:281
- Does not block non-artifact abilities: PASS - only skips when is_artifact is true
- Applies regardless of Stony Silence's controller: PASS - checks all objects by name
- Only affects battlefield artifacts (not cycling etc.): PASS - legal_actions only generates actions for battlefield objects
- Does not affect triggered abilities: PASS - only filters in activated/mana ability generation

### Test coverage
- Card data: `innistrad_simple_cards.rs:586`
- Blocks artifact mana abilities (Sol Ring): `innistrad_simple_cards.rs:595`
- Does not block non-artifact mana (Forest): `innistrad_simple_cards.rs:625`
- Blocks non-mana activated abilities of artifacts: NOT TESTED
- Artifact creatures' activated abilities blocked: NOT TESTED
- Multiple Stony Silences (redundant but should work): NOT TESTED

---

## Audit — 2026-04-02

### Oracle Text (Scryfall, cached 2026-04-01)
- **Name:** Stony Silence
- **Cost:** {1}{W}
- **Type:** Enchantment
- **Oracle Text:** "Activated abilities of artifacts can't be activated."

### Key Rulings
1. Activated abilities contain a colon ("[Cost]: [Effect]"), including keyword abilities like equip.
2. No abilities of artifacts can be activated, **including mana abilities**.
3. Only affects artifacts on the battlefield. Activated abilities in other zones (e.g., cycling) are unaffected. Triggered abilities are unaffected.

### Implementation Review (`mtg-engine/src/cards/isd/stony_silence.rs`)
- **Card data:** Correct. Name "Stony Silence", cost {1}{W}, type Enchantment, oracle text matches.
- **`continuous_effects` field is empty.** The restriction is not modeled as a continuous effect on the card itself. Instead, it is hard-coded in `engine.rs` `legal_actions()` (lines 256-309).

### Engine Enforcement (`mtg-engine/src/engine.rs`, lines 256-309)
- **Detection:** `stony_silence_active` is true if any object on the battlefield has `name == "Stony Silence"`. This is controller-agnostic (correct — the oracle text does not restrict by controller).
- **Mana abilities (lines 266-272):** When `stony_silence_active`, any object in `objects_in_zone(Battlefield, player)` whose card types include `Artifact` is skipped entirely. Mana abilities of artifacts are correctly blocked.
- **Non-mana activated abilities (lines 302-308):** Same artifact check; all activated abilities of artifacts are skipped. Correct.
- **Scope — both players:** `legal_actions()` computes actions for whichever player has priority. Since the `stony_silence_active` flag is global (not controller-filtered), it correctly restricts both players.
- **Non-artifact permanents unaffected:** The `continue` only fires when the object is an artifact. Non-artifact permanents (lands, creatures, enchantments) retain their activated abilities. Correct.

### Issues Found

**No rule-correctness bugs found.** The implementation is faithful to the oracle text and rulings.

#### Minor Design Notes (non-blocking)
1. **Hard-coded in engine rather than declared as a continuous effect.** The `continuous_effects: vec![]` field on the card is empty. If the engine ever refactors to use a continuous-effect system for restrictions, this card will need updating. However, the current hard-coded approach is functionally correct.
2. **Name-based detection.** The check uses `o.name == "Stony Silence"` rather than a card ID or effect flag. This is fragile if there were ever a different card with the same name or if a copy effect changed a permanent's name, but is acceptable for the current engine scope.

### Test Coverage (`mtg-engine/tests/innistrad_simple_cards.rs`)
| Scenario | Status | Location |
|---|---|---|
| Card data (type, CMC) | PASS | `stony_silence_card_data` (line 586) |
| Blocks artifact mana abilities (Sol Ring) | PASS | `stony_silence_blocks_artifact_mana_abilities` (line 595) |
| Does not block non-artifact mana (Forest) | PASS | `stony_silence_does_not_block_non_artifact_mana` (line 625) |
| Blocks non-mana activated abilities of artifacts | NOT TESTED | — |
| Opponent's artifacts blocked | NOT TESTED | — |
| Artifact creatures' activated abilities blocked | NOT TESTED | — |
| Multiple Stony Silences (redundant) | NOT TESTED | — |
| Artifacts in other zones (cycling) unaffected | NOT TESTED | — |

### Verdict
**PASS** — The implementation correctly prevents activation of all activated abilities (including mana abilities) of artifacts on the battlefield for both players. No mismatches between oracle text and engine behavior.
