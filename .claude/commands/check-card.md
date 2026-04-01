# Check Card Implementation

Thoroughly audit Magic: The Gathering card implementations for correctness, test coverage, and UI presentation.

## Arguments
- `$ARGUMENTS` — One or more card names to check, comma-separated (e.g., "Lightning Bolt" or "Lightning Bolt, Fiend Hunter, Doom Blade")

When multiple cards are given, run the full procedure for EACH card and compile a summary at the end.

## CRITICAL: Batching over agents

If you need to split work across multiple agents (e.g., auditing many cards in parallel), you **MUST** include the **full text of this skill prompt** in each agent's instructions — not a summary or abbreviation. Every agent must receive the complete procedure, checklists, anti-patterns, and rules verbatim. Summarizing or abbreviating the prompt will result in shallow, incomplete audits.

## CRITICAL RULES
- **DO NOT read any previous audit logs before conducting your audit.** Your audit must be independent. You will write your results to the log AFTER completing your audit.
- **NEVER USE YOUR TRAINING DATA AS A SOURCE FOR ORACLE TEXT, RULINGS, TYPES, SUBTYPES, COSTS, OR ANY OTHER CARD DATA.** Your memory of Magic cards is unreliable. Cards are errata'd regularly (e.g., "Hound" → "Dog", planeswalker damage redirect removal, "dies" templating changes, "mill" keyword addition). You MUST fetch the oracle text from an external source (Scryfall, Gatherer, etc.) for EVERY card you audit. There are ZERO exceptions to this rule.
- **Do NOT compare code against what you think the card does.** Compare ONLY against text you fetched from an external source during this audit session. If you did not fetch it, you do not have it.
- **Do NOT mark a card as ISSUE based on your memory of the oracle text.** If you couldn't fetch the oracle text from any source, say so explicitly and mark the card as SKIPPED — do NOT guess or fall back to memory.
- **Every audit entry MUST cite its source** (e.g., "Source: Scryfall via WebSearch" or "Source: Gatherer via WebSearch"). If there is no source citation, the audit is invalid.

### Why these rules exist
Previous audits produced false positives because:
1. **Hallucinated oracle text**: Auditors "remembered" old oracle text (e.g., pre-2018 planeswalker damage redirect wording) and flagged working code as wrong. The card had been errata'd but the auditor's training data was stale.
2. **Fabricated mismatches**: Auditors claimed "Scryfall says X but code says Y" without actually quoting both — and X was hallucinated. When forced to produce exact quotes, these phantom issues evaporate.
3. **Self-contradictions**: Auditors found an issue, then later in the same audit realized it was fine, but forgot to update the status from ISSUE to PASS.

The write-it-down-verbatim rule, the side-by-side quoting rule, and the reconciliation step exist specifically to prevent these failure modes. Follow them exactly.

## Procedure (repeat for each card)

### 1. Identify the card
- Find the card's implementation file in `mtg-engine/src/cards/`
- Assume the card is real — do NOT question whether a card exists
- Note which set(s) the card appears in

### 2. Obtain the CURRENT Oracle text — THIS IS MANDATORY

You **MUST** obtain the current Oracle text from an authoritative source before proceeding. Do not skip this step. Do not rely on your training data. Oracle text changes over time due to errata, and auditing against stale text produces false positives.

**Try these approaches in order until one works:**

**Approach 1: Local oracle cache (ALWAYS try this first)**
```bash
python3 scripts/oracle_lookup.py lookup "Card Name"
```
If the card is in the cache, this is your source of truth. The cache contains verified oracle text with source citations. **Write down the oracle text from the cache output** — this is your single source of truth for the rest of the audit.

**Approach 2: If not cached, use the oracle-text skill to fetch and cache it**
Run the `/oracle-text "Card Name"` skill, which will fetch from WebSearch, cache the result, and return the oracle text. If running inside a subagent without access to the skill, do the following manually:

1. Use WebSearch: `{card name} scryfall oracle text` (restrict to scryfall.com)
2. If that doesn't show full text: `{card name} MTG oracle text gatherer`
3. If still not found: `{card name} MTG card text type line` (any source)

**After fetching, immediately cache it:**
```bash
python3 scripts/oracle_lookup.py add-card "Card Name" \
  --mana-cost "{1}{R}" \
  --type-line "Enchantment — Aura Curse" \
  --oracle-text "The exact oracle text here..." \
  --source "Scryfall via WebSearch" \
  --source-url "https://scryfall.com/card/set/number/card-name"
```
For creatures add `--power` and `--toughness`. For DFCs, also run `add-back-face`.

**You are not done until you have the oracle text.** If after all attempts you truly cannot find it, state this explicitly in the audit log — do NOT fall back to your training data and pretend it's authoritative.

**Write down the oracle text verbatim** (from cache output or from what you just fetched). Copy-paste it exactly — do not paraphrase, summarize, or reword. This written record is your single source of truth for the rest of the audit. All comparisons in later steps MUST reference this written-down text, not your memory. (Why: previous auditors unconsciously drifted from fetched text back to training-data memories mid-audit. Writing it down anchors you to the real text.)

Record: name, mana cost, type line (including ALL creature subtypes), Oracle text (verbatim), power/toughness, and which source you got it from.

### 3. Obtain rulings

**Approach 1: Check local cache first**
If you ran `python3 scripts/oracle_lookup.py lookup "Card Name"` in step 2 and rulings were shown, you already have them. Use those.

**Approach 2: If no cached rulings, fetch via WebSearch**
- Search: `{card name} scryfall rulings` (restrict to scryfall.com)
- Search: `{card name} MTG rulings gatherer`
- Check Gatherer rulings, MTGAssist rulings, or MTG Salvation forums

**After fetching, cache EACH ruling with its source URL:**
```bash
python3 scripts/oracle_lookup.py add-ruling "Card Name" \
  --date "2011-09-22" \
  --text "The exact ruling text..." \
  --source "Scryfall rulings via WebSearch" \
  --source-url "https://scryfall.com/card/set/number/card-name"
```
**Every cached ruling MUST have a `--source-url`.** This is mandatory — rulings without source links are unverifiable and useless.

Pay special attention to rulings about timing, targeting, "you may" vs mandatory, "another" vs "a", and "each opponent" vs "target player".

### 4. Research community knowledge (for complex cards)

Skip for vanilla creatures and basic spells. For anything with triggered abilities, activated abilities, unusual timing, replacement effects, or multi-step resolution:

- **Search for known interactions**: Use WebSearch:
  - `{card name} MTG rulings interactions`
  - `{card name} MTG rules corner cases`
- **Check MTG rules forums**: Reddit (r/mtgrules), MTG Salvation wiki, Judges' Corner
- **For cards with similar mechanics**, search for the mechanic:
  - Equipment: "MTG equipment rules when creature dies"
  - DFCs: "MTG transform rules trigger timing"
  - Curses: "MTG curse rules enchant player"
  - Death triggers: "MTG death trigger timing simultaneous"
  - Replacement effects: "MTG replacement effect ordering"
- **Record surprising findings** for step 6.

### 5. Identify relevant comprehensive rules
- Creatures: summoning sickness, combat rules
- Instants/sorceries: stack rules, timing restrictions
- Auras: attachment rules, what happens when target leaves
- Equipment: enters unattached, detaches on creature death, stays on battlefield
- Triggered abilities: when they trigger, APNAP ordering, stack behavior
- Continuous effects: layer system implications
- ETB/dies: zone transition rules, last-known information
- Transform/DFC: which face's characteristics are active, transform vs ETB

### 6. Think about tricky interactions
This is the most important step. Consider:

**With the stack:**
- If this card has a triggered ability, what happens if the source is removed before the trigger resolves?
- Can an opponent respond to this card's ETB trigger?

**With other cards in the pool:**
- Does "destroy" work against indestructible? (No — use try_destroy pipeline)
- Does "sacrifice" bypass indestructible? (Yes — use sacrifice pipeline)
- Does "destroy... it can't be regenerated" need try_destroy_no_regen?
- Does damage trigger lifelink? (Only combat damage from lifelink creature)
- Do tokens have creature types? (Yes, if specified at creation)
- Does "whenever a creature dies" trigger on tokens? (Yes)

**Semantic precision:**
- "Destroy" vs "sacrifice" vs "exile" — each has different rules interactions
- "Target" vs "choose" — targeting can be responded to, choosing can't
- "You may" vs no "may" — optional vs mandatory
- "Another" vs "a" — self-exclusion
- "Each" vs "target" — no targeting for "each"
- "Combat damage" vs "damage" — combat damage is a subset
- "Mana cost" vs "mana value" — cost includes colors, value is just the number

### 7. Check the code
Read the card's implementation file. Verify:

**IMPORTANT: When claiming ANY mismatch between oracle text and code, you MUST quote both sides exactly:**
- "Oracle text says: `{exact quote from your written-down oracle text}`"
- "Code says: `{exact quote from the code}`"
If you cannot produce both exact quotes, the mismatch is not verified and MUST NOT be flagged.

**Card data (compare EXACTLY against your written-down oracle text from step 2):**
- [ ] Mana cost matches (correct colors and generic amount)
- [ ] Card types correct (Creature, Instant, Sorcery, Enchantment, Artifact, Land, Planeswalker)
- [ ] Supertypes correct (Legendary, Basic) — Scryfall type_line is authoritative
- [ ] Subtypes correct — ALL creature subtypes from Scryfall (e.g., "Vampire Noble" needs both)
- [ ] Power/toughness correct
- [ ] Keywords correct and COMPLETE (don't miss any)
- [ ] Oracle text field reasonably matches
- [ ] Flashback cost correct (if applicable)
- [ ] Continuous effects match static abilities
- [ ] Triggered abilities declared with correct TriggerKinds matching implemented hooks

**Behavior:**
- [ ] `on_resolve` implements the spell effect correctly
- [ ] Targeting: `target_requirement` and `is_valid_target` match Oracle text restrictions
- [ ] "You may" abilities are properly optional (not auto-applied)
- [ ] "Target" effects present player choice (not auto-selected)
- [ ] "Each" effects apply to all matching (no targeting)
- [ ] ETB/dies/death-watch triggers fire at the right time
- [ ] Life gain/loss emits LifeChanged event
- [ ] Non-combat damage emits NonCombatDamageDealt (NOT CombatDamageDealt)
- [ ] Non-combat damage tracks damaged_by on target creatures
- [ ] Spell cleanup uses `move_spell_after_resolve()` (not `move_object(Zone::Graveyard)`)
- [ ] Dynamic P/T uses `dynamic_pt` trait method
- [ ] Token creation includes correct subtypes via `create_token_with_subtypes`
- [ ] triggered_abilities TriggerKind entries match EVERY implemented hook (on_blocks needs Blocks, on_upkeep needs Upkeep, etc.)

### 8. Check test coverage
Search for tests in `mtg-engine/tests/`. Check:
- [ ] At least one test for the card's main effect
- [ ] For targeted spells: fizzle test?
- [ ] For "you may": declining test?
- [ ] For triggers: test through trigger system?
- [ ] For flashback: cast from graveyard + exiled after?

### 9. Check UI presentation
- If the card involves choices, are they presented to the player?
- Does the triggered ability description make sense on the stack?
- Is the card in the LLM card knowledge section?

### 10. Shortcut check
**Known anti-patterns:**
- `move_object(id, Zone::Graveyard)` instead of `move_spell_after_resolve(id)`
- `CombatDamageDealt` for non-combat damage (should be `NonCombatDamageDealt`)
- Auto-targeting "target player" without choice (must present choice)
- `obj.power` instead of `state.effective_power(id, registry)`
- `EffectScope::Global` when `GlobalOther` is needed (or vice versa)
- Missing token subtypes (tokens need subtypes via `create_token_with_subtypes`)
- Missing triggered_abilities declaration for an implemented hook
- `try_destroy` when Oracle says "sacrifice" (or vice versa)
- Human/subtype check only via registry, not also checking `obj.subtypes` (misses tokens)

### 11. Verify test correctness
For EXISTING tests:
- [ ] Assertions match CURRENT Oracle text from Scryfall?
- [ ] Tests verify mechanism, not just outcome?
- [ ] Any tests that enshrine wrong behavior?

### 12. Reconcile findings before writing

Before writing anything, review every issue you flagged:
- Re-read your written-down oracle text from step 2.
- For each flagged issue, confirm the discrepancy still holds by quoting the oracle text AND the code side by side.
- Drop any issue where the quotes actually match or where you cannot produce an exact quote from the oracle text supporting the issue.
- Check for **outdated rules** — if your issue depends on a rule that may have changed (e.g., planeswalker damage redirect removed in 2018, "Hound" → "Dog" errata, "dies" templating), verify the current rule applies.
- If you corrected yourself during the audit (e.g., "actually, this is fine"), make sure the correction is reflected in your final status. Do NOT leave a stale ISSUE status from before the correction.

### 13. Write audit log
**IMPORTANT**: After completing your audit, append your findings to the audit log file:

For each card audited, append to `audits/{card_file_name}.md` (create if it doesn't exist):
```markdown
## Audit — {YYYY-MM-DD HH:MM}

**Oracle text source**: {e.g., "Scryfall card page via WebSearch", "Gatherer via WebSearch", "Scryfall API via curl"}
**Oracle text**: {exact text from external source}
**Type line**: {exact type line from external source}
**Status**: PASS / ISSUE / SKIPPED (if oracle text could not be obtained)

{If ISSUE, for each issue provide:}
{  - Description with file path and line number}
{  - Oracle text says: `{exact quote from written-down oracle text}`}
{  - Code does: `{exact quote or description of code behavior}`}
{If PASS, write "No issues found."}
```

**If you do not have an external source citation, do NOT write the audit entry. Mark as SKIPPED instead.**

Use the current date/time. Append — never overwrite previous audit entries.

### 14. Final report
Output a structured report:

```
## Card: {name}
**Set**: {set name}
**Oracle text**: {current Oracle text from Scryfall}
**Status**: CORRECT / NEEDS FIX / CRITICAL

**Code issues**:
- {issue 1}
- {issue 2}

**Tricky interactions checked**:
- {interaction 1}: {pass/fail}

**Test coverage**:
- Existing tests: {list}
- Missing tests: {list with descriptions}

**Code quality**: {notes}
```
