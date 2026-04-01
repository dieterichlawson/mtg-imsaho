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
