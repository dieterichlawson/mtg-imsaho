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

## Procedure (repeat for each card)

### 1. Identify the card
- Find the card's implementation file in `mtg-engine/src/cards/`
- Assume the card is real — do NOT question whether a card exists
- Note which set(s) the card appears in

### 2. Obtain the CURRENT Oracle text — THIS IS MANDATORY

You **MUST** obtain the current Oracle text from an authoritative source before proceeding. Do not skip this step. Do not rely on your training data. Oracle text changes over time due to errata, and auditing against stale text produces false positives.

**Try these approaches in order until one works:**

**Approach 1: Scryfall API via curl**
```
curl -s "https://api.scryfall.com/cards/named?fuzzy=card+name" -H "User-Agent: MTG/1.0" | python3 -c "import json,sys; d=json.load(sys.stdin); print('Name:', d['name']); print('Cost:', d.get('mana_cost','')); print('Type:', d['type_line']); print('Oracle:', d.get('oracle_text','')); print('P/T:', d.get('power',''), '/', d.get('toughness',''))"
```

For DFCs (double-faced cards), also check card_faces:
```
curl -s "https://api.scryfall.com/cards/named?fuzzy=card+name" -H "User-Agent: MTG/1.0" | python3 -c "import json,sys; d=json.load(sys.stdin); [print(f'Face {i}: Name={f[\"name\"]}, Type={f.get(\"type_line\",\"\")}, Oracle={f.get(\"oracle_text\",\"\")}, P/T={f.get(\"power\",\"\")} / {f.get(\"toughness\",\"\")}') for i,f in enumerate(d.get('card_faces',[]))]"
```

**Approach 2: If curl fails (403, empty response, proxy block), use WebSearch**
Search for: `{card name} scryfall oracle text`
The Scryfall card page in search results usually shows the full oracle text in the snippet.

**Approach 3: If Scryfall search doesn't show full text, try other sources**
- Search: `{card name} MTG oracle text gatherer`
- Search: `{card name} MTG card text type line`
- Check Gatherer, MTG Wiki, EDHREC, MTGAssist, or any other source that shows current Oracle text
- Cross-reference multiple sources if needed to confirm accuracy

**You are not done until you have the oracle text.** If after all attempts you truly cannot find it, state this explicitly in the audit log — do NOT fall back to your training data and pretend it's authoritative.

Record: name, mana cost, type line (including ALL creature subtypes), Oracle text, power/toughness, and which source you got it from.

### 3. Obtain rulings

**Approach 1: Scryfall rulings API**
```
curl -s "https://api.scryfall.com/cards/named?fuzzy=card+name" -H "User-Agent: MTG/1.0" | python3 -c "import json,sys; d=json.load(sys.stdin); uri=d.get('rulings_uri',''); print(uri)"
```
Then fetch the rulings URI:
```
curl -s "RULINGS_URI" -H "User-Agent: MTG/1.0" | python3 -c "import json,sys; d=json.load(sys.stdin); [print(f'{r[\"published_at\"]}: {r[\"comment\"]}') for r in d.get('data',[])]"
```

**Approach 2: If API fails, use WebSearch**
- Search: `{card name} MTG rulings`
- Search: `{card name} scryfall rulings`
- Check Gatherer rulings, MTGAssist rulings, or MTG Salvation forums

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

**Card data (compare EXACTLY against Scryfall):**
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

### 12. Write audit log
**IMPORTANT**: After completing your audit, append your findings to the audit log file:

For each card audited, append to `audits/{card_file_name}.md` (create if it doesn't exist):
```markdown
## Audit — {YYYY-MM-DD HH:MM}

**Oracle text source**: {e.g., "Scryfall card page via WebSearch", "Gatherer via WebSearch", "Scryfall API via curl"}
**Oracle text**: {exact text from external source}
**Type line**: {exact type line from external source}
**Status**: PASS / ISSUE / SKIPPED (if oracle text could not be obtained)

{If ISSUE, describe each issue with file path and line number}
{If PASS, write "No issues found."}
```

**If you do not have an external source citation, do NOT write the audit entry. Mark as SKIPPED instead.**

Use the current date/time. Append — never overwrite previous audit entries.

### 13. Final report
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
