# Card Audit Procedure

This file contains the full audit procedure for a single MTG card. Read it completely before starting.

## CRITICAL RULES

Previous audits produced false positives that wasted time and eroded trust in the audit process. Every rule below exists to prevent a specific failure mode we've actually hit.

Auditors have "remembered" old oracle text from training data (e.g., pre-2018 planeswalker damage redirect wording) and flagged working code as wrong, when in fact the card had been errata'd and the auditor's memory was stale. To prevent this, you must **NEVER use your training data as a source for oracle text, rulings, types, subtypes, costs, or any other card data.** Cards are errata'd regularly ("Hound" to "Dog", "dies" templating changes, "mill" keyword addition). You must fetch the oracle text from an external source for every card you audit — there are zero exceptions. Do not compare code against what you think the card does; compare only against text you fetched during this audit session. If you did not fetch it, you do not have it. If you couldn't fetch oracle text from any source, mark the card as SKIPPED — do not guess or fall back to memory.

Auditors have also claimed "Scryfall says X but code says Y" without actually quoting both sides, and X turned out to be hallucinated. When forced to produce exact quotes, these phantom issues evaporated. To prevent this, **when claiming any mismatch you must quote both sides exactly** — the oracle text and the code. If you cannot produce both exact quotes, the mismatch is not verified and must not be flagged.

Finally, auditors have read old audit logs and been biased by prior findings instead of auditing independently. To prevent this, **do not read any previous audit logs before conducting your audit.** Your audit must be independent. You will write your results to the log after completing your audit.

## Procedure

### 1. Identify the card
- Find the card's implementation file in `mtg-engine/src/cards/`
- Assume the card is real — do NOT question whether a card exists
- Note which set(s) the card appears in

### 2. Obtain the CURRENT Oracle text and rulings — THIS IS MANDATORY

You **MUST** obtain the current Oracle text from an authoritative source before proceeding. Do not skip this step. Do not rely on your training data. Oracle text changes over time due to errata, and auditing against stale text produces false positives.

Run `python3 scripts/oracle_lookup.py lookup "Card Name"`. If not cached, run `python3 scripts/oracle_lookup.py fetch "Card Name"`. If that fails, fall back to WebSearch for `{card name} scryfall oracle text`.

**You are not done until you have the oracle text.** If after all attempts you truly cannot find it, state this explicitly in the audit log — do NOT fall back to your training data and pretend it's authoritative.

**Write down the oracle text verbatim** (from cache output or from what you just fetched). Copy-paste it exactly — do not paraphrase, summarize, or reword. This written record is your single source of truth for the rest of the audit. All comparisons in later steps MUST reference this written-down text, not your memory. (Why: previous auditors unconsciously drifted from fetched text back to training-data memories mid-audit. Writing it down anchors you to the real text.)

Record: name, mana cost, type line (including ALL creature subtypes), Oracle text (verbatim), power/toughness, rulings, and which source you got it from.

Pay special attention to rulings about timing, targeting, "you may" vs mandatory, "another" vs "a", and "each opponent" vs "target player".

### 3. Research community knowledge (for complex cards)

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
- **Record surprising findings** for step 5.

### 4. Identify relevant comprehensive rules
- Creatures: summoning sickness, combat rules
- Instants/sorceries: stack rules, timing restrictions
- Auras: attachment rules, what happens when target leaves
- Equipment: enters unattached, detaches on creature death, stays on battlefield
- Triggered abilities: when they trigger, APNAP ordering, stack behavior
- Continuous effects: layer system implications
- ETB/dies: zone transition rules, last-known information
- Transform/DFC: which face's characteristics are active, transform vs ETB

### 5. Think about tricky interactions
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

### 6. Check the code
Read the card's implementation file. Verify:

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
- [ ] Card code does NOT clean up its own spell at all — no `move_object(id, Zone::Graveyard)` and no `move_spell_after_resolve(id)`. The engine owns it (`stack::resolve_spell`, and `engine::finish_spell_resolution_if_idle` once a choice chain completes); a card that moves itself and then presents another choice has left the stack mid-resolution, against CR 608.2m. Countering a *different* spell uses `move_countered_spell` (CR 701.5a).
- [ ] Dynamic P/T uses `dynamic_pt` trait method
- [ ] Token creation includes correct subtypes via `create_token_with_subtypes`
- [ ] triggered_abilities TriggerKind entries match EVERY implemented hook (on_blocks needs Blocks, on_upkeep needs Upkeep, etc.)

### 7. Check test coverage
Search for tests in `mtg-engine/tests/`. Check:
- [ ] At least one test for the card's main effect
- [ ] For targeted spells: fizzle test?
- [ ] For "you may": declining test?
- [ ] For triggers: test through trigger system?
- [ ] For flashback: cast from graveyard + exiled after?
- [ ] Is there a test for each ruling?
- [ ] Is there a test for each tricky interaction?
- [ ] Do tests verify mechanism, not just outcome?
- [ ] Do any tests enshrine wrong behavior?

### 8. Check UI presentation
- If the card involves choices, are they presented to the player?
- Does the triggered ability description make sense on the stack?
- Does the description of the cards make sense in the logs and on the stack?
- Is the card in the LLM card knowledge section?

### 9. Shortcut check

Often implementations have been found to take shortcuts and not implement things correctly. All implementations should 'do the right thing' and implement the MTG rules exactly. There should be no kludges or simplifications.

**Known anti-patterns:**
- Any self-cleanup in card code: `move_object(id, Zone::Graveyard)` OR `move_spell_after_resolve(id)` — both are the engine's job, and `test_suite_guards.rs::no_card_moves_a_spell_off_the_stack_itself` fails the build on the latter
- `CombatDamageDealt` for non-combat damage (should be `NonCombatDamageDealt`)
- `obj.power` instead of `state.effective_power(id, registry)`
- `EffectScope::Global` when `GlobalOther` is needed (or vice versa)
- Missing token subtypes (tokens need subtypes via `create_token_with_subtypes`)
- Missing triggered_abilities declaration for an implemented hook
- `try_destroy` when Oracle says "sacrifice" (or vice versa)
- Human/subtype check only via registry, not also checking `obj.subtypes` (misses tokens)
- Using menace for an effect that requires 2 blockers on a creature when the card does not mention menace.

### 10. Reconcile findings

Before writing your final report, review every issue you flagged:
- Re-read your written-down oracle text from step 2.
- For each flagged issue, confirm the discrepancy still holds by quoting the oracle text AND the code side by side.
- Drop any issue where the quotes actually match or where you cannot produce an exact quote from the oracle text supporting the issue.
- Check for **outdated rules** — if your issue depends on a rule that may have changed (e.g., planeswalker damage redirect removed in 2018, "Hound" → "Dog" errata, "dies" templating), verify the current rule applies.
- If you corrected yourself during the audit (e.g., "actually, this is fine"), make sure the correction is reflected in your final status. Do NOT leave a stale ISSUE status from before the correction.

### 11. Write report

Write a single report that serves as both the audit log and the skill output. Append it to `audits/{card_file_name}.md` (create if it doesn't exist), and also output it to the user.

Use the current date/time. Append — never overwrite previous audit entries.

**If you do not have an external source citation, do NOT write the audit entry. Mark as SKIPPED instead.**

```markdown
## Audit — {YYYY-MM-DD HH:MM}

**Oracle text source**: {e.g., "Oracle cache (Scryfall API)", "Scryfall via WebSearch"}
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

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- {ruling or interaction}: `test_file.rs:line_number` / NOT TESTED
```
