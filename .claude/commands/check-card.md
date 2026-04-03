# Check Card Implementation

Thoroughly audit Magic: The Gathering card implementations for correctness, test coverage, and UI presentation.

## Arguments
- `$ARGUMENTS` — What to audit. Accepts any of:
  - Card names, comma-separated: `"Lightning Bolt, Fiend Hunter, Doom Blade"`
  - A set code: `"isd"` or `"innistrad"` (audits all cards in the set)
  - `"all"` (audits every implemented card)
Use `/audit-status` to view audit history and status tables instead.

## Audit mode

### Resolving the card list

1. **If a set code** (e.g., `"isd"`): List all `.rs` files in `mtg-engine/src/cards/{set}/` (excluding `mod.rs`). Convert filenames to card names by looking up each file's struct name or `card_data().name`.
2. **If `"all"`**: List all `.rs` files in `mtg-engine/src/cards/` recursively (excluding `mod.rs`, `helpers.rs`).
3. **If card names**: Use as-is.

### Dispatching audits — one agent per card, max 10 at a time

Each card MUST be audited by its own dedicated subagent with fresh context. Never give multiple cards to a single agent.

1. Take the next batch of up to 10 cards from the list.
2. Launch one Agent per card, all in parallel (up to 10 simultaneous agents). Each agent gets the prompt below.
3. Wait for all agents in the batch to complete.
4. Commit and push: `git add audits/ && git commit -m "Audit batch: Card A, Card B, ..." && git push`
5. Collect results from each agent and note PASS/ISSUE/SKIPPED.
6. If more cards remain, go to step 1 with the next batch.
7. After all batches, output a summary table.

### Agent prompt (for each card)

Launch each agent with the prompt below. Replace `{CARD_NAME}`, `{CARD_FILE}`, `{AUDIT_FILE}`, and `{NOW}` (use the current date and time in YYYY-MM-DD HH:MM format, e.g. "2026-04-02 14:30") with the appropriate values. Include the prompt EXACTLY as written — do NOT summarize or abbreviate it.

```
You are auditing the MTG card: {CARD_NAME}

The current date and time is {NOW}.

The card implementation is at `mtg-engine/src/cards/{CARD_FILE}`.

## CRITICAL RULES

Previous audits produced false positives that wasted time and eroded trust in the audit process. Every rule below exists to prevent a specific failure mode we've actually hit.

Auditors have "remembered" old oracle text from training data (e.g., pre-2018 planeswalker damage redirect wording) and flagged working code as wrong, when in fact the card had been errata'd and the auditor's memory was stale. To prevent this, you must **NEVER use your training data as a source for oracle text, rulings, types, subtypes, costs, or any other card data.** Cards are errata'd regularly ("Hound" to "Dog", "dies" templating changes, "mill" keyword addition). You must fetch the oracle text from an external source for every card you audit — there are zero exceptions. Do not compare code against what you think the card does; compare only against text you fetched during this audit session. If you did not fetch it, you do not have it. If you couldn't fetch oracle text from any source, mark the card as SKIPPED — do not guess or fall back to memory.

Auditors have also claimed "Scryfall says X but code says Y" without actually quoting both sides, and X turned out to be hallucinated. When forced to produce exact quotes, these phantom issues evaporated. To prevent this, **when claiming any mismatch you must quote both sides exactly** — the oracle text and the code. If you cannot produce both exact quotes, the mismatch is not verified and must not be flagged.

Finally, auditors have read old audit logs and been biased by prior findings instead of auditing independently. To prevent this, **do not read any previous audit logs before conducting your audit.** Your audit must be independent. You will write your results to the log after completing your audit.

## Procedure — follow ALL steps

### Step 1. Obtain oracle text

Run `python3 scripts/oracle_lookup.py lookup "{CARD_NAME}"`. If not cached, run `python3 scripts/oracle_lookup.py fetch "{CARD_NAME}"`.

You are not done until you have the oracle text. If you truly cannot find it, mark as SKIPPED.

**Write down the oracle text verbatim.** Copy-paste it exactly — do not paraphrase. This is your single source of truth for the rest of the audit.

Record: name, mana cost, type line (including ALL subtypes), oracle text (verbatim), power/toughness, rulings, and source.

Pay special attention to rulings about timing, targeting, "you may" vs mandatory, "another" vs "a", and "each opponent" vs "target player".

### Step 2. Research community knowledge (for complex cards)

Skip for vanilla creatures and basic spells. For anything with triggered/activated abilities, unusual timing, replacement effects, or multi-step resolution:

- WebSearch: `{CARD_NAME} MTG rulings interactions`
- WebSearch: `{CARD_NAME} MTG rules corner cases`
- Check mechanic-specific rules if relevant (equipment, DFCs, curses, death triggers, etc.)

### Step 3-4. Check the code against oracle text

Read the implementation. Verify all card data and behavior against your written-down oracle text. Use the checklists in `.claude/commands/check-card-procedure.md` for reference if needed.

Key things to verify: mana cost, card types, supertypes, subtypes, P/T, keywords, oracle text field, flashback cost, continuous effects, triggered_abilities TriggerKinds, targeting, "you may" optionality, "target" player choice, "each" vs "target", damage types (NonCombatDamageDealt not CombatDamageDealt), spell cleanup (move_spell_after_resolve), dynamic_pt, token subtypes.

### Step 5. Think about tricky interactions

Consider stack interactions, semantic precision ("destroy" vs "sacrifice", "target" vs "choose", "you may" vs mandatory, "another" vs "a", "each" vs "target", "combat damage" vs "damage"), and interactions with other cards (indestructible, tokens, lifelink).

### Step 6-8. Check tests, UI, shortcuts

Search for tests in `mtg-engine/tests/`. Check UI presentation (choices presented? LLM card knowledge?). Check for known anti-patterns: `move_object(Zone::Graveyard)` instead of `move_spell_after_resolve`, `CombatDamageDealt` for non-combat damage, missing triggered_abilities declarations, auto-selecting choices, `try_destroy` when oracle says "sacrifice".

### Step 9. Reconcile findings

Before writing your final report:
- Re-read your written-down oracle text.
- For each flagged issue, confirm the discrepancy still holds by quoting the oracle text AND the code side by side.
- Drop any issue where the quotes actually match or where you cannot produce an exact quote from the oracle text supporting the issue.
- Check for **outdated rules** — if your issue depends on a rule that may have changed (e.g., planeswalker damage redirect removed in 2018, "Hound" → "Dog" errata, "dies" templating), verify the current rule applies.
- If you corrected yourself during the audit (e.g., "actually, this is fine"), make sure the correction is reflected in your final status. Do NOT leave a stale ISSUE status from before the correction.

### Step 10. Write report

Append to `audits/{AUDIT_FILE}` (create if needed). Never overwrite previous entries.

If you do not have an external source citation for the oracle text, mark as SKIPPED.

You MUST use this EXACT format — no prose summaries, no shortcuts:

## Audit — {YYYY-MM-DD HH:MM}

**Oracle text source**: {e.g., "Oracle cache (Scryfall API)"}
**Oracle text**: {exact text from external source}
**Type line**: {exact type line from external source}
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
```

### After all audits complete

Output a summary table:

```
## Audit Summary — {date}

| Card | Status | Key Issues |
|------|--------|------------|
| ... | PASS / ISSUE | ... |

**Total**: N PASS, N ISSUE, N SKIPPED
```
