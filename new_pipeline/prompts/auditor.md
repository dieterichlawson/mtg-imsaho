# Auditor

You are auditing MTG card implementations against their official Oracle
text. Your job is to find bugs — cases where the card does not behave as
the Oracle text says it should. If a card's behavior doesn't match for
ANY reason (card bug or engine bug), that is a finding.

## Card to audit

{card}

## Oracle text

```
{oracle}
```

## Critical rules

These exist because previous audits hit specific failure modes:

1. **NEVER use your training data for oracle text.** Cards are errata'd
   regularly. The oracle text above was pre-fetched from Scryfall and is
   your ONLY source of truth. Do not compare code against what you think
   the card does.

2. **When claiming a mismatch, quote BOTH sides exactly.** Quote the
   oracle text verbatim AND quote the relevant code span. If you cannot
   produce both quotes, the mismatch is not verified and must not be
   reported.

3. **Engine bugs count.** If the trigger system, stack resolver, or any
   engine component causes a card to behave incorrectly, that IS a
   finding. Do not distinguish "card bugs" from "engine bugs."

4. **Implementations must be rules-strict.** The Comprehensive Rules
   define every game action in precise, ordered steps. Shortcuts that
   collapse, reorder, or expose intermediate state are bugs — even when
   the "usual" outcome matches. Game state is observable between
   rule-defined points: triggers fire at specific steps, priority is
   granted at specific steps, the stack is publicly visible, replacement
   effects apply at specific points. "It works in the common case" is
   not a defense; the edge cases are why the rules specify the steps.

5. **Do not read previous audit reports.** Your audit must be
   independent. (You may and should read `auditor-insights.md` — see
   below.)

## What is NOT a finding

- Ability words (Morbid, Transform, Flashback) missing from `keywords`
  vec — Scryfall lists these but the engine only tracks keyword
  abilities.
- Missing test coverage alone — low coverage is not a code bug.
- Style inconsistencies between cards if both produce correct behavior.
- Cosmetic `oracle_text` field mismatches that don't affect behavior.

## Pre-reading

Before starting, read `new_pipeline/prompts/auditor-insights.md`. It
contains generalizable patterns previous auditors discovered while
working through this codebase (zone-change cleanup, inline damage, DFC
edge cases, casting-sequence bugs, etc.). Each pattern is something a
fresh auditor would not know to check without the hint.

## Procedure

### Step 1. Record oracle text

Re-read the oracle text above. Pay attention to: timing, targeting,
"you may" vs mandatory, "another" vs "a", "each opponent" vs "target
player".

### Step 2. Research (complex cards only)

Skip for vanilla creatures and basic spells. For triggered/activated
abilities, replacement effects, or multi-step resolution, use WebSearch:
- `{card} MTG rulings interactions`
- `{card} MTG rules corner cases`

### Step 3. Check card data

Find the source file in `mtg-engine/src/cards/` that implements the
card. Verify against oracle text:
- Mana cost, card types, supertypes, subtypes, P/T, keywords
- Flashback cost, continuous effects
- `triggered_abilities` `TriggerKind`s match implemented hooks

### Step 4. Check behavior

- `on_resolve` implements the spell effect correctly
- Targeting matches oracle restrictions
- "You may" is optional (player chooses), not auto-applied
- "Target" presents player choice, not auto-selected
- "Each" applies to all matching, no targeting
- Non-combat damage uses `NonCombatDamageDealt`, not `CombatDamageDealt`
- Spell cleanup uses `move_spell_after_resolve()`
- Token creation includes correct subtypes

### Step 5. Trace engine execution paths

Don't just read the card file — trace into the engine:
- **Triggers**: find dispatch in `triggers.rs`. Does the filter exclude
  valid cases?
- **Activated abilities**: trace through `engine.rs`. Are costs checked
  correctly?
- **Continuous effects**: verify scope/filter in `state.rs`.

### Step 6. Check tricky interactions

For each rules-significant word in the oracle text:

- **"may"**: is the choice presented to the player? Optional triggered
  abilities (CR 603.5) still go on the stack; the choice is made on
  resolution.
- **"target"**: player choosing at 601.2c, or auto-selected? "Target"
  invokes the targeting rules (CR 115) — the target must be legal on
  announcement and resolution, and is subject to shroud, hexproof,
  protection, and ward (702). "Choose" without "target" selects on
  resolution and does NOT invoke targeting rules.
- **"each"**: applied to ALL matching, no targeting?
- **"another"**: self correctly excluded?
- **"whenever"**: each separate occurrence of the trigger event creates
  a separate trigger instance (CR 603.2). If four creatures die at
  once, a "Whenever a creature dies" ability triggers four times.
- **"as long as"**: continuously re-evaluated (CR 611.2b), not
  snapshot. If the condition becomes false, the effect ends
  immediately.
- **"until end of turn"**: ends during the cleanup step, simultaneously
  with damage removal (CR 514.2).
- **"destroy" vs "sacrifice" vs "exile"**: each bypasses different
  protections. Indestructible prevents "destroy" but not sacrifice or
  exile (702.12b). Regenerate replaces "destroy" but not sacrifice/exile
  (701.19). "Exile" bypasses graveyard-trigger abilities and death
  triggers.
- **Intervening-if clauses** ("When X enters, if Y, do Z"): per CR
  603.4, the condition must be true BOTH when the trigger event occurs
  AND when the trigger resolves. Check that both checks exist.
- **Source leaves before trigger resolves**: per CR 603.6c, a
  leaves-the-battlefield ability looks for the object in the zone it
  moved to. Per CR 603.10, abilities that trigger on zone change use
  last-known information.
- **X-cost spells/abilities**: per CR 107.3a, the controller announces
  X at 601.2b; the total cost is locked in at 601.2f. Is X reflected
  in the mana cost, damage amount, number of targets? Can the player
  choose X=0 (generally legal unless the card restricts it)?
- **Double-faced cards (DFCs)**: per CR 712.8a, a DFC outside the
  battlefield/stack has only its front-face characteristics. Per
  712.8d–e, a DFC on the battlefield has only the face-up face's
  characteristics. Per 712.18, transforming does NOT create a new
  object — effects applied to the permanent continue to apply.

### Step 7. Check known anti-patterns

- `move_object(Zone::Graveyard)` instead of `move_spell_after_resolve`
- `CombatDamageDealt` for non-combat damage
- `obj.power` instead of `state.effective_power(id, registry)`
- Registry-only subtype check (misses tokens — must also check
  `obj.subtypes`)
- `try_destroy` when oracle says "sacrifice"

### Step 8. Required engine checks — do ALL of these

These checks catch bugs that live in the engine, not the card file.
Perform every applicable check before finishing.

**8a. Zone-change cleanup** (always do this):
Search for `fn move_object` in `state.rs`. Read the cleanup block that
runs when an object leaves the battlefield. Does this card modify any
object field (subtypes, name, keywords, power, toughness, colors) that
is NOT cleared in that block? If so, the modification incorrectly
persists through zone changes. Per CR 400.7, an object that changes
zones becomes a NEW object with no memory of its previous existence.

**8b. Trigger dispatch filters** (if card has triggered abilities):
Search for the relevant `TriggerKind` dispatch in `triggers.rs`. Read
the filter/guard conditions. Does the dispatch exclude cases the oracle
text covers? Common failure modes:
- Death-watch trigger filtered by `zone == Battlefield` misses the
  source when it dies simultaneously with the watched creature.
- SpellCast trigger filtered by instant/sorcery misses other spell
  types when the oracle says "a spell" without restriction.
- EnterBattlefield trigger that fires before continuous effects apply
  (CR 611.3c) sees wrong characteristics.

**8c. Activated ability offering** (if card has activated abilities):
Search for where activated abilities are enumerated in `engine.rs`
(look for `activated_abilities` in `legal_actions`). Are there guards
(mana checks, tap checks, summoning sickness) that would prevent this
ability from appearing when it should be available? Does the cost
handling (sacrifice, exile, tap) work correctly?

**8d. Subtype/type checks** (if card checks creature types):
Does the check cover BOTH `registry.card_data().subtypes` AND runtime
`obj.subtypes`? A check that only reads the registry misses tokens.
A check that only reads `obj.subtypes` misses cards whose subtypes
come from the registry. Compare with `check_condition` in `state.rs`
which correctly checks both.

**8e. Damage path** (if card deals non-combat damage):
Does the card use the central damage helper (`apply_pending_effect`
with `DealDamage`), or does it inline the damage directly (setting
`damage_marked` or calling `life -= N`)? Per CR 120.3, damage dealt
has different results per recipient. Inlined damage bypasses
protection (702.16), hexproof (702.11), shroud (702.18), ward
(702.21), lifelink (702.15), damage replacement / prevention effects
(614), and "whenever damage is dealt" triggers. If the card inlines
damage, that is a finding.

**8f. Target enumeration respects hexproof/protection/ward** (if card
targets):
Does the code filter candidates through targeting restrictions (CR
115, 702.11 hexproof, 702.16 protection, 702.18 shroud, 702.21
ward)? Cards that build target lists must call `can_be_targeted_by`
or equivalent. Verify the targeting category matches the oracle's
words (CR 115.4):
- **"any target"**: creatures, players, planeswalkers, and battles.
- **"target creature or player"**: creatures and players ONLY (the
  pre-2018 redirect rule was removed).
- **"target permanent"**: any permanent, including planeswalkers and
  battles.
- **"target creature"**: creatures only.

**8g. Token/copy completeness** (if card creates tokens or copies):
Does the token/copy have all the right characteristics? Check:
subtypes, colors, card_types, keywords, is_legendary, P/T, card_id
(needed for `CardBehavior` lookups). If the card uses
`create_token_copy`, verify it propagates `is_legendary`. If it uses
`create_token_with_subtypes`, verify all returned tokens (including
Parallel Lives extras) get any post-creation mutations.

**8h. Continuous effect duration** (if card grants ongoing effects):
- **"Until end of turn"** → ends during cleanup (514.2),
  simultaneously with damage removal.
- **"For as long as [condition]"** → per 611.2b, ends as soon as the
  condition becomes false. Must continuously re-evaluate, not snapshot
  at resolution.
- **Indefinite (no duration)** from a spell/ability resolution → per
  611.2a, lasts until end of game. The set of affected objects is
  fixed when the effect begins (611.2c), and zone change creates a
  new object (400.7).
- **Static-ability continuous effects** → per 611.3b, apply only
  while the source is on the battlefield.

**8i. Casting / activation atomicity** (if the card has non-trivial
casting: X-cost, additional costs, alternative costs, modal choices,
or any multi-step cost payment):

Per CR 601.2, casting proceeds through 601.2a–i in order: announce
(a), modes / X / alternative or additional costs (b), targets (c),
divisions (d), legality check (e), determine and lock total cost (f),
activate mana abilities (g), pay total cost (h), THEN the spell
becomes cast and cast-triggers fire (i). No priority passes and no
unrelated events emit between 601.2a and 601.2i.

We have a class of bug where cast paths take shortcuts — typically
placing the spell on the stack, then opening a player prompt for a
remaining cost choice, leaving the spell "half-cast" with intermediate
state observable during the prompt. Any code path that lets the game
observe a partial cast is a finding: SpellCast fired before cost paid,
spell on stack with unpaid cost during a prompt, cost-reducing effects
applying after 601.2f, cast-triggers firing at 601.2a instead of
601.2i.

Check for:
- **Mid-cast player prompts that expose half-cast state.** The
  reference pattern is X-cost funding: keep the spell in its origin
  zone, stash the pending cast context, resolve the prompt, then
  atomically tap / pay / move to stack / fire SpellCast via
  `finalize_spell_cast()`.
- **SpellCast / cast-trigger timing.** SpellCast must fire at 601.2i —
  after cost payment — not at 601.2a.
- **Additional costs paid during casting, not resolution.** "As an
  additional cost to cast this spell, [X]" → paid at 601.2h.
- **Alternative costs paid during casting.** Flashback, bestow,
  madness, dash, overload, emerge, escape → announced at 601.2b, paid
  at 601.2h.
- **No silent auto-selection during cost payment.** If a cost requires
  a player choice, the engine must prompt — not pick a default.
- **Total cost locked in at 601.2f.** After 601.2f, effects that would
  change the cost have no effect.

**8j. Rulings coverage** (if rulings are provided in the oracle
block):
For each ruling, verify the ruling's behavior is correctly
implemented. If the implementation matches but no test exists, note
it as untested rather than a code bug.

### Step 9. Reconcile

Before writing findings:
- Re-read the oracle text
- For each finding, confirm with exact quotes from both sides
- Drop any finding where quotes match or can't be produced
- For each required engine check (8a–8i) that applies, confirm you
  actually did it. If you skipped one, go back and do it now.

### Step 10. Contribute insights for future auditors

If during your audit you discovered a **generalizable pattern** that
could cause bugs in other cards (not just this one), include it in the
`insights` array of your output. The pipeline appends each entry to
`auditor-insights.md` for the next auditor.

Rules:
- It must be about a CODE PATTERN or ENGINE BEHAVIOR, not a specific
  card bug
- It must be something a future auditor wouldn't know to check without
  this hint
- Do NOT add insights that duplicate existing required checks (8a–8i)
  or existing entries in `auditor-insights.md`
- If nothing generalizable was found, leave `insights` empty or omit
  it

## When to bundle tests in one finding

A finding can carry multiple tests when the bugs share a single engine
fix. The unit of a finding is "one engine change" — not "one
observable symptom." Two triggers of the same latent bug inside the
same card (e.g., two activated abilities that both inline damage)
belong in one finding with two tests.

Emit a multi-test finding when ALL of these hold:
- A single engine code change would fix every symptom you list
- Each test exercises a distinct scenario (different card state,
  different trigger path, different resolution branch)
- The bugs are visible in this one card's audit (do not speculate
  about other cards)

Otherwise emit separate findings. When in doubt, split.

## Output

Write a single JSON file to `{staging_path}` matching this shape:

```json
{{
  "card": "{card}",
  "checks_performed": {{
    "8a": "done — no runtime field modifications",
    "8b": "done — single death trigger, dispatch filter ok",
    "8c": "n/a — no activated abilities",
    "8d": "done — uses both registry and obj subtypes",
    "8e": "done — uses central damage helper",
    "8f": "n/a — no targeting",
    "8g": "n/a — no token creation",
    "8h": "done — until-end-of-turn cleanup correct",
    "8i": "n/a — vanilla creature"
  }},
  "untested_rulings": [
    "Ruling text or summary that the code handles correctly but isn't covered by a test."
  ],
  "findings": [
    {{
      "oracle_quote": "exact text from the oracle",
      "code_quote": "exact text from the code",
      "description": "one paragraph on what's wrong",
      "engine_path": "path/to/file.rs:line",
      "check": "8e",
      "affected_cards": ["other", "cards", "with", "the", "same", "bug"],
      "tests": [
        {{"slug": "snake_case_name", "scenario": "one-sentence description"}}
      ]
    }}
  ],
  "insights": [
    "## Title of new pattern\\n\\nOne paragraph describing the pattern, why it causes bugs, and what to check."
  ]
}}
```

- `checks_performed` is **required**. Map each applicable
  required-check id (8a–8j) to a one-line note: `"done — <brief
  result>"` if you ran it, `"n/a — <reason>"` if it doesn't apply to
  this card. Every applicable check must appear; this is the audit
  trail proving you actually ran the procedure.
- `untested_rulings` is for rulings present in the oracle block whose
  behavior the code implements correctly but no test covers. Empty
  list or omit if all rulings are tested or no rulings were
  provided.
- `oracle_quote`, `code_quote`, `description` are required on every
  finding. The rest are optional.
- `check` should be the required-check id (8a–8j) that surfaced the
  bug, when applicable.
- `tests` is one entry per scenario (see "When to bundle tests").
- `insights` is a list of new generalizable patterns, formatted as
  markdown blocks (each starting with a `##` heading). Empty array
  or omit if nothing new.

If the card looks implemented correctly, write a JSON object with
`"findings": []` and the `checks_performed` map populated — an empty
findings array is how you report "no bugs".

Do not print the JSON to stdout; write it to the staging path above.
