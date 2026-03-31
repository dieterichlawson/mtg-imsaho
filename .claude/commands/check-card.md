# Check Card Implementation

Thoroughly audit Magic: The Gathering card implementations for correctness, test coverage, and UI presentation.

## Arguments
- `$ARGUMENTS` — One or more card names to check, comma-separated (e.g., "Lightning Bolt" or "Lightning Bolt, Fiend Hunter, Doom Blade")

When multiple cards are given, run the full procedure for EACH card and compile a summary at the end.

## Procedure (repeat for each card)

### 1. Identify the card
- Find the card's implementation file in `mtg-engine/src/cards/`
- Assume the card is real — do NOT question whether a card exists. All cards in this codebase are real MTG cards.
- Note which set(s) the card appears in

### 2. Pull the correct Oracle text
- **IMPORTANT**: Do NOT use WebFetch for Scryfall — it will be blocked. Use the Bash tool to curl the API.
- **Primary method**: Use the Scryfall API (JSON, not HTML) via Bash:
  ```
  curl -s "https://api.scryfall.com/cards/named?fuzzy=card+name" | python3 -c "import json,sys; d=json.load(sys.stdin); print('Name:', d['name']); print('Cost:', d.get('mana_cost','')); print('Type:', d['type_line']); print('Oracle:', d.get('oracle_text','')); print('P/T:', d.get('power',''), '/', d.get('toughness',''))"
  ```
  Run this via the Bash tool. The API returns the CURRENT Oracle text with all errata applied.
- **Fallback**: If the API fails, use WebSearch for `{card name} scryfall` and try to fetch the page.
- **Last resort**: Use your MTG knowledge, but flag that you couldn't verify online. Oracle text changes over time (e.g., "Whenever a creature you control dies" vs "Whenever another creature dies" — these are DIFFERENT cards or DIFFERENT errata versions). Getting the exact current text matters.
- Record: name, mana cost, type line, Oracle text, power/toughness (if creature)
- **IMPORTANT**: Creature types have been updated over time. "Hound" became "Dog", "Bird" may have changed subtypes. Always use the API result.

### 3. Search for rulings and errata
- **IMPORTANT**: Scryfall is the AUTHORITATIVE source for current Oracle text, errata, and type lines. The Scryfall API always returns the latest Oracle text with all errata applied. NEVER use your training data over Scryfall — cards have been errata'd (e.g., "Hound" → "Dog", "sacrifice" → "destroy", type line changes, flashback cost corrections).
- **Primary method**: Use the Scryfall API for rulings:
  ```
  curl -s "https://api.scryfall.com/cards/named?fuzzy=card+name" -H "User-Agent: MTG/1.0" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('rulings_uri',''))"
  ```
  Then fetch the rulings URI to get judge rulings.
- **For DFCs (double-faced cards)**: Use the `card_faces` array from the Scryfall response:
  ```
  curl -s "https://api.scryfall.com/cards/named?fuzzy=card+name" -H "User-Agent: MTG/1.0" | python3 -c "import json,sys; d=json.load(sys.stdin); [print(f'Face {i}: Name={f[\"name\"]}, Oracle={f.get(\"oracle_text\",\"\")}, P/T={f.get(\"power\",\"\")} / {f.get(\"toughness\",\"\")}') for i,f in enumerate(d.get('card_faces',[]))]"
  ```
- **Fallback**: Search `{card name} mtg rulings` via WebSearch.
- Pay special attention to rulings about:
  - Timing (when effects happen)
  - Targeting (what can and can't be targeted)
  - Interactions with other cards
  - "You may" vs mandatory effects
  - "Another" vs "a" (self-exclusion)
  - "Each opponent" vs "target player"

### 4. Identify relevant rules
- Which comprehensive rules apply to this card?
- For creatures: summoning sickness, combat rules
- For instants/sorceries: stack rules, timing restrictions
- For auras: attachment rules, what happens when target leaves
- For triggered abilities: when they trigger, APNAP ordering, stack behavior
- For continuous effects: layer system implications
- For "enters the battlefield" / "dies": zone transition rules

### 5. Think about tricky interactions
This is the most important step. Consider:

**With the stack:**
- If this card has a triggered ability, what happens if the source is removed before the trigger resolves? (The trigger still resolves — it's independent on the stack.)
- Can an opponent respond to this card's ETB trigger? (Yes — triggers go on the stack.)

**With other cards in the pool:**
- Does this card's "destroy" effect work against indestructible? (No — use the destruction pipeline.)
- Does this card's damage trigger lifelink? (Only combat damage from a creature with lifelink triggers lifelink. Spell damage doesn't.)
- If this is an aura, what happens when the enchanted creature is bounced? (Aura falls off, goes to graveyard via SBA.)
- If this card has "whenever a creature dies" — does it trigger on tokens? (Yes — tokens do go to the graveyard before ceasing to exist.)

**Example tricky interactions:**
- **Fiend Hunter + sacrifice**: If you sacrifice Fiend Hunter with its ETB trigger on the stack, the LTB trigger goes on the stack ON TOP of the ETB trigger. LTB resolves first (returns nothing — nothing exiled yet). Then ETB resolves (exiles a creature permanently — because the LTB already resolved and found nothing to return). This is a known MTG rules interaction.
- **Falkenrath Noble + board wipe**: If all creatures die simultaneously, Noble triggers for each OTHER creature that died (not itself). Multiple triggers go on the stack.
- **Giant Growth + damage**: If a creature has 3 damage marked and you cast Giant Growth (+3/+3), the creature survives because toughness is now 6. SBAs only check after the spell resolves.
- **Flashback + fizzle**: A flashback spell that fizzles still goes to exile (not graveyard). The "exile instead of graveyard" replacement applies even on fizzle.
- **Bonds of Faith + Arcane Adaptation**: If a creature becomes a Human while Bonds of Faith is attached, the effect doesn't automatically update — it was set on ETB and the `instance_continuous_effects` are fixed.
- **Wreath of Geists + creature entering graveyard**: The +X/+X updates dynamically as creatures enter the graveyard. A creature enchanted by Wreath that fights and kills another creature should get bigger mid-combat (though SBAs only check after).
- **Geist-Honored Monk + its own tokens**: When the Monk ETBs and creates two Spirit tokens, its P/T should count itself plus the two tokens (= 3/3 minimum). The dynamic_pt must see the tokens that were just created.

### 6. Check the code
Read the card's implementation file. Verify:

**Card data:**
- [ ] Mana cost matches Oracle (correct colors, correct generic amount)
- [ ] Card types correct (Creature, Instant, Sorcery, Enchantment, Artifact)
- [ ] Supertypes correct (Legendary, Basic, Snow) if applicable
- [ ] Subtypes correct (creature types, "Aura" for auras)
- [ ] Power/toughness correct (if creature)
- [ ] Keywords correct and complete
- [ ] Oracle text field matches current Oracle text
- [ ] Flashback cost correct (if applicable)
- [ ] Continuous effects match the card's static abilities
- [ ] Triggered abilities declared with correct kinds and descriptions

**Behavior:**
- [ ] `on_resolve` implements the spell effect correctly
- [ ] Targeting: `target_requirement` and `is_valid_target` match Oracle text restrictions
- [ ] "You may" abilities are properly optional (not auto-applied)
- [ ] "Target" effects let the player choose (not auto-selected)
- [ ] "Each" effects apply to all matching (no targeting)
- [ ] ETB/dies/death-watch triggers fire at the right time
- [ ] Life gain/loss uses correct events (LifeChanged)
- [ ] Damage uses correct events (CombatDamageDealt for spell damage too — known quirk)
- [ ] Spell cleanup: uses `move_spell_after_resolve()` (not `move_object(Zone::Graveyard)`)
- [ ] Dynamic P/T: uses `dynamic_pt` trait method if P/T depends on game state

### 7. Check test coverage
Search for tests related to this card in `mtg-engine/tests/`. Check:

**Basic functionality:**
- [ ] Is there at least one test that verifies the card's main effect works?
- [ ] For creatures: is there a test that puts it on the battlefield and verifies stats?
- [ ] For spells: is there a test that casts and resolves it?
- [ ] For auras: is there a test that attaches it and verifies the buff/restriction?

**Tricky cases:**
- [ ] For targeted spells: is there a test for fizzle (target becomes illegal)?
- [ ] For "you may" effects: is there a test for declining?
- [ ] For ETB triggers: is there a test that the trigger works through the trigger system (not just direct function call)?
- [ ] For death triggers: is there a test with simultaneous death?
- [ ] For continuous effects: is there a test that the effect goes away when the source leaves?
- [ ] For auras: is there a test for the aura falling off when the creature dies?
- [ ] For flashback: is there a test for casting from graveyard AND being exiled after?

**Corner cases from step 5:**
- [ ] Are any of the tricky interactions from step 5 tested?
- [ ] If not, which ones SHOULD be tested? (List them.)

**Missing tests:**
- List any tests that should exist but don't, with a brief description of what they would verify.

### 8. Check UI presentation
Examine how this card's choices and effects appear to players.

**For the CLI player** (mtg-player/src/cli.rs):
- If this card involves choices (targeting, "you may", discard selection), how is the choice presented?
- Can the human player see enough information to make a good decision?
- Are targets shown with enough context (name, P/T, controller)?

**For the AI/LLM player** (mtg-player/src/llm.rs):
- If this card has a triggered ability, does it appear on the stack with a clear description?
  (Check: the `triggered_abilities` field should have a descriptive `description` string)
- If the AI needs to choose a target for a trigger, does the prompt include enough info?
- Is the card in the LLM's card knowledge section? (Search the system prompt in llm.rs)
- If not, does the AI have enough context from the game state to play the card correctly?

**For the game view** (mtg-engine/src/view.rs):
- If the card has continuous effects (auras, anthems), are the modified stats visible in the view?
  (Check: effective P/T should reflect the buff/debuff)
- If the card creates tokens, do the tokens appear correctly in the view?
- If the card has a triggered ability on the stack, does the StackItemView show a useful description?

**Common UI issues to check:**
- Auto-targeting: does the engine auto-pick a target without showing the player any choices? This is a shortcut — the player should always choose.
- Missing card knowledge: if the AI doesn't have this card in its system prompt, can it still figure out what to do from the action list and game state?
- Resolution choices: if the card uses `AwaitingAction::ResolutionChoice`, does the choice description make sense to a human reading the CLI or an AI reading the prompt?

### 9. Simplicity check
- Is the implementation as simple as it can be?
- Does it use the shared helpers where applicable?
  - **Spell resolution**: `resolve_aura()`, `resolve_damage()`, `resolve_destroy()`
  - **Target choice**: `present_target_choice()`, `present_optional_target_choice()`
  - **Target collection**: `creature_targets()`, `any_targets()`, `creature_targets_except()`, `creatures_controlled_by()`, `controller_of()`
- Does it use declarative data (continuous_effects, triggered_abilities) instead of imperative code where possible?
- Is there duplicated logic that could be extracted?

### 10. Shortcut check
- Is anything auto-targeted that should be player-chosen?
- Is anything mandatory that should be optional ("you may")?
- Is any effect missing (e.g., +2/+2 declared but not applied)?
- Does "another" properly exclude self?
- Does "you control" properly check controller?
- For auras: does the fallback case (target gone) properly go to graveyard?

**Known anti-patterns to check for:**
- `move_object(id, Zone::Graveyard)` instead of `move_spell_after_resolve(id)` — ALL spells must use the latter. This is a hard rule, not a suggestion. Failing to use it means flashback spells would go to graveyard instead of exile.
- `CardRegistry::with_all_cards()` created inside card methods — unnecessary since `is_valid_target` and `on_resolve` both receive the registry as a parameter. Use the parameter.
- Auto-targeting when only 1 target exists — for "you may" abilities the player should always get the choice to decline. For mandatory targeting with 1 option, auto-selection is acceptable.
- Auto-targeting "target player" — cards that say "target player" (as opposed to "target opponent") allow choosing ANY player including yourself. The player MUST be given a choice. Only auto-target when the Oracle says "target opponent" (since in 2-player there's only one opponent). Example: Bloodgift Demon says "target player draws a card and loses 1 life" — the controller should choose which player, not auto-target self.
- Using `obj.power` (base power) instead of `state.effective_power(id, registry)` — affects life gain (Swords), targeting (Smite), and any other power-dependent effect. Always use effective_power/effective_toughness.
- `EffectScope::Global` includes the source permanent. `EffectScope::GlobalOther` excludes it. Check which one the card needs:
  - "Creatures you control get +1/+1" → `Global` (includes self)
  - "Other Spirit creatures you control get +1/+0" → `GlobalOther` (excludes self)
- Hand-rolling target collection + choice presentation instead of using helpers. Cards should use `present_target_choice()` / `present_optional_target_choice()` with target helpers like `creature_targets()`, `any_targets()`, etc.
- `ChooseCardFromHand` should be used when the player needs to choose a card to discard, not auto-picking `hand.first()`.

**Engine limitation to flag (not a card bug):**
- If a card's behavior is limited by a trait signature or engine API (e.g., `is_valid_target` historically couldn't use effective_power because it had no registry parameter), flag it as an engine limitation rather than a card bug. The fix may require changing the trait, not just the card.

### 11. Verify test correctness
This is separate from test coverage (step 7). For EXISTING tests:
- [ ] Do the test assertions match the CURRENT Oracle text? (Tests may have been written against old/wrong Oracle text and never updated.)
- [ ] Does the test verify the mechanism, not just the outcome? (e.g., a spell fizzle test should check that SpellResolved is NOT emitted, not just that no damage was dealt.)
- [ ] If the card was recently fixed, were all related tests updated to match the fix?
- [ ] Are there tests that enshrine wrong behavior? (e.g., testing that a trigger does NOT fire when it should)

### 12. Final report
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

**UI presentation**:
- CLI: {how choices appear}
- AI: {does the AI have enough context?}
- Stack display: {trigger descriptions}

**Code quality**: {notes}
```
