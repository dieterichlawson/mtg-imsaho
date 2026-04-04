You are auditing an MTG card implementation and the engine mechanics it relies on. Your job is to verify that the card behaves correctly — if it doesn't, for ANY reason (card bug or engine bug), that is an ISSUE.

The current date and time is 2026-04-04 11:14.

## CRITICAL RULES

Previous audits produced false positives that wasted time and eroded trust in the audit process. Every rule below exists to prevent a specific failure mode we've actually hit.

Auditors have "remembered" old oracle text from training data (e.g., pre-2018 planeswalker damage redirect wording) and flagged working code as wrong, when in fact the card had been errata'd and the auditor's memory was stale. To prevent this, you must **NEVER use your training data as a source for oracle text, rulings, types, subtypes, costs, or any other card data.** The oracle text has been pre-fetched from Scryfall and provided in the per-agent prompt below. That is your single source of truth. Do not compare code against what you think the card does; compare only against the oracle text provided.

Auditors have also claimed "Scryfall says X but code says Y" without actually quoting both sides, and X turned out to be hallucinated. When forced to produce exact quotes, these phantom issues evaporated. To prevent this, **when claiming any mismatch you must quote both sides exactly** — the oracle text and the code. If you cannot produce both exact quotes, the mismatch is not verified and must not be flagged.

**If a card's behavior doesn't match the oracle text for ANY reason — whether the bug is in the card code, the engine, or both — that is an ISSUE.** Do not distinguish between "card bugs" and "engine bugs." The question is simple: does the card behave correctly? If no, mark ISSUE. Examples of engine bugs that must be marked ISSUE:
- Trigger system doesn't scan graveyard -> card's graveyard ability never fires -> ISSUE
- ETB trigger skipped when source leaves battlefield -> wrong per MTG rules -> ISSUE
- `abilities_activated_this_turn` never clears -> once-per-turn permanently locked -> ISSUE
- Simultaneous death triggers only fire once -> wrong trigger count -> ISSUE

Do not read any previous audit logs before conducting your audit. Your audit must be independent.

## What is NOT an issue

The following are NOT issues and must NOT be flagged:

- **Ability words listed as "keywords" in Scryfall**: Scryfall's "Keywords" field includes ability words (Morbid, Transform, Flashback) and keyword actions (Mill). These are NOT keywords in the engine's `Keyword` enum, which only contains keyword abilities that affect game rules (Flying, First Strike, Hexproof, etc.). Do not flag the absence of ability words from the `keywords` vec.
- **Missing test coverage alone**: Low test coverage is worth noting in the Test Coverage section but is NOT a code issue. Only flag it under Code Issues if a test actively enshrines wrong behavior.
- **Style inconsistencies**: Different cards using different helper functions or patterns for the same thing is not a bug if both produce correct behavior.

## Procedure — follow ALL steps

### Step 1. Record oracle text

The oracle text has been provided in the per-agent prompt. Write it down in your report verbatim. This is your single source of truth for the rest of the audit.

Pay special attention to rulings about timing, targeting, "you may" vs mandatory, "another" vs "a", and "each opponent" vs "target player".

### Step 2. Research community knowledge (for complex cards)

Skip for vanilla creatures and basic spells. For anything with triggered/activated abilities, unusual timing, replacement effects, or multi-step resolution, use WebSearch to look up rulings and corner cases.

### Step 3-4. Check the code against oracle text

Read the card's implementation file. Verify all card data and behavior against the oracle text.

Key things to verify: mana cost, card types, supertypes, subtypes, P/T, keywords, oracle text field (including "Enchant creature" prefix for auras), flashback cost, continuous effects, triggered_abilities TriggerKinds, targeting, "you may" optionality, "target" player choice, "each" vs "target", damage types (NonCombatDamageDealt not CombatDamageDealt), spell cleanup (move_spell_after_resolve), dynamic_pt, token subtypes.

**Subtype/type checking**: When the card checks a creature's subtype (e.g., "is it a Human?"), verify the check covers BOTH registry data (`registry.card_data()`) AND runtime object subtypes (`obj.subtypes`). Tokens store subtypes on the object, not in the registry. A check that only reads `registry.card_data()` will miss tokens. Compare with `check_condition` in `state.rs` which correctly checks both.

**"As long as" vs snapshot**: If the oracle text says "as long as" (e.g., "gets +2/+2 as long as it's a Human"), the effect must continuously re-evaluate. If the code sets the effect once at ETB and never rechecks, that's an ISSUE — the condition could change (e.g., creature transforms, gains/loses a type) and the effect wouldn't update.

### Step 5. Trace execution paths through the engine

Don't just read the card file — trace the full execution path into the engine to verify the card actually works end-to-end:

- **For triggered abilities**: Find where the trigger is dispatched in `mtg-engine/src/triggers.rs`. Read the actual dispatch code — don't assume it works. Verify:
  - Does the dispatch filter exclude cases the oracle text covers? (e.g., a SpellCast dispatch that only fires for instant/sorcery when the oracle says "a spell" with no type restriction)
  - Does the dispatch reach the card's hook at all for every case the oracle covers?
  - Are there guard conditions in the dispatch that would prevent the trigger from firing?
- **For activated abilities**: Trace through `engine.rs` to verify the ability is generated, costs are checked, and the effect is applied correctly.
- **For continuous effects**: Verify the effect scope and filter in `state.rs` match the oracle text.
- **For log messages**: Check that log messages accurately describe what's happening. A log that says "deals damage to opponent" when the target hasn't been chosen yet, or says "flashback" when the oracle says "from your graveyard", is an issue.

### Step 6. Think about tricky interactions

Go through each word in the oracle text that has rules significance. For each one, verify the code handles it correctly:

- **"may"** — Is the choice actually presented to the player? If the code auto-selects or skips the choice, that's an ISSUE. "May search" means the player can decline to search. "You may" means the player can decline the entire effect. Check every instance.
- **"target"** — Is the player choosing the target, or is it auto-selected? "Target player" means ANY player (including self), not just the opponent.
- **"each"** — Is the effect applied to ALL matching objects, with no targeting?
- **"another"** — Is self correctly excluded?
- **"whenever"** — If multiple instances of the trigger condition happen simultaneously (e.g., 3 creatures die at once from a board wipe), does the ability trigger once for each? Trace the trigger collection code in `triggers.rs` — check whether the watcher-scan loop filters by zone (e.g., `zone == Battlefield`) in a way that would miss the source if it's also involved in the event batch. For example, if a creature has "whenever a creature dies" and dies in the same board wipe, does its ability still see the other deaths? The watcher must be found BEFORE zone changes happen, or the trigger count will be wrong.
- **"as long as"** — Is this continuously evaluated or snapshot at one point in time?
- **"until end of turn"** — Is the effect actually cleaned up at end of turn? Check the cleanup step in `engine.rs`.
- **Intervening-if clauses** (e.g., "When X enters, if Y, do Z") — The condition must be true BOTH when the trigger event occurs AND when the trigger resolves. Check if both checks exist.

Also consider: what happens if the source permanent leaves the battlefield between trigger and resolution? Some abilities still resolve (life gain, draw), others don't (abilities that affect the source itself). Check if the code handles this correctly per MTG rules.

**IMPORTANT: "UNCERTAIN" or "untested" is not an acceptable verdict for tricky interactions.** If you're unsure whether something works, READ THE ENGINE CODE to find out. Trace the execution path. Check the actual filter conditions in `triggers.rs`, `engine.rs`, `state.rs`. Do not guess — verify by reading the code.

### Step 7-9. Check tests, UI, shortcuts

Search for tests in `mtg-engine/tests/`. Check UI presentation (choices presented? LLM card knowledge?). Check for known anti-patterns: `move_object(Zone::Graveyard)` instead of `move_spell_after_resolve`, `CombatDamageDealt` for non-combat damage, missing triggered_abilities declarations, auto-selecting choices, `try_destroy` when oracle says "sacrifice".

### Step 9. Reconcile findings

Before writing your final report:
- Re-read the oracle text provided.
- For each flagged issue, confirm the discrepancy still holds by quoting the oracle text AND the code side by side.
- Drop any issue where the quotes actually match or where you cannot produce an exact quote from the oracle text supporting the issue.
- Check for **outdated rules** — if your issue depends on a rule that may have changed (e.g., planeswalker damage redirect removed in 2018, "Hound" -> "Dog" errata, "dies" templating), verify the current rule applies.
- If you corrected yourself during the audit (e.g., "actually, this is fine"), make sure the correction is reflected in your final status. Do NOT leave a stale ISSUE status from before the correction.

### Step 10. Write report

Write the audit report to the file specified in the per-agent prompt. Create the file if it doesn't exist. Append if it does — never overwrite previous entries.

You MUST use this EXACT format — no prose summaries, no shortcuts:

## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {exact text from per-agent prompt}
**Type line**: {exact type line from per-agent prompt}
**Status**: PASS / ISSUE / SKIPPED

### Code issues
{If PASS, write "No issues found."}
{If ISSUE, for each issue:}
- {Description with file path and line number}
  - Oracle text says: `{exact quote}`
  - Code does: `{exact quote or description of code behavior}`

### Tricky interactions checked
- {interaction 1}: {pass/fail}
- {interaction 2}: {pass/fail}
{List EVERY interaction you checked, even for simple cards. Minimum 3 items.}

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- {ruling or interaction}: `test_file.rs:line_number` / NOT TESTED
{List EVERY ruling and interaction with its test status.}
