# Fix Card Issues

Fix audit issues for Magic: The Gathering card implementations. Given a card name (or list of cards), fix all issues found in the most recent audit.

## Arguments
- `$ARGUMENTS` — One or more card names to fix, comma-separated

## Principles

- **Do it right.** Do not take shortcuts. Do not simplify. Implement the MTG rules exactly as written.
- **Do not be afraid of large engine changes.** If the fix requires adding new fields to actions, new resolution choice kinds, new hooks on CardBehavior, new state fields, or new SBA checks — do it. The engine should serve the cards, not the other way around.
- **Player choice is mandatory.** If the oracle text says the player chooses, targets, sacrifices, or searches, that choice must be presented to the player. Auto-selecting is a shortcut. The only exceptions are: (1) when there's exactly one legal option, or (2) "target opponent" in a 2-player game (unambiguous). Note: "target player" is NOT auto-selectable — the caster can target themselves or their opponent.
- **Additional costs happen at cast time.** Sacrifice, exile, and other additional costs are paid when the spell is cast, not when it resolves. If the spell is countered, the costs are already paid.
- **Engine additions must be generic.** When you add a new mechanism to the engine (new resolution choice kind, new hook, new state field, etc.), design it as a generic facility that any card can use — not a one-off for the card you're fixing. No `if card_name == "Garruk"` in engine code. Use trait methods, enum variants, and data-driven patterns so future cards can reuse the same mechanism.

## Common pitfalls

These are patterns that have caused bugs before. Watch for them:

- **DFCs: check the active face, not the front face.** `registry.card_data()` returns front face data. For a transformed creature, you need `back_face_data()`. Check `o.is_transformed` and use the right source. Don't assume instance fields (like `o.subtypes`) are populated — test helpers often don't set them, so include a registry fallback.
- **Damage source isn't always the activating object.** Equipment abilities are activated by the creature but the damage source may be the equipment itself (Blazing Torch). Track the source ID carefully.
- **State-triggered abilities belong in SBA, not in event handlers.** If an ability should fire from any cause (e.g., "when X has 2 or fewer counters"), check it in state-based actions — not just after a specific event like loyalty ability activation.
- **Delayed triggers must survive the source leaving.** Store delayed trigger data at the game level (e.g., `state.end_of_combat_exiles`), not on the source permanent's `card_state`, which becomes inaccessible if the permanent leaves the battlefield.
- **Use TargetFilter on ability definitions, not just `is_valid_target`.** The `is_valid_target` method doesn't know which ability is being targeted. Put targeting restrictions (like `Another`, `YouControl`, `HasSubtype`) on the ability's `target_requirement` so the engine filters at action generation time.
- **Effects that stack must use counts, not booleans.** If an effect can come from multiple sources (e.g., multiple equipment), use `count_continuous_effect` and compute `2^n` or similar — don't use `has_effect` returning bool.
- **Cast-from-graveyard is not flashback.** Cards with `can_cast_from_graveyard()` use their normal mana cost and don't get exiled after resolving. The engine must distinguish this from flashback.

## Procedure (for each card)

### 1. Look up oracle text

Use the `/oracle-text` skill or run `python3 scripts/oracle_lookup.py lookup "Card Name"` to get the current oracle text and rulings. Read them carefully before touching any code.

### 2. Read the audit log

Read the most recent audit entry in `audits/{card_file_name}.md` to understand what issues were found. Note each issue with its file path and line number.

### 3. Read the implementation

Read the card's implementation file. Understand the current code before changing it.

### 4. Fix each issue

For each issue in the audit:
- Quote the oracle text that governs the behavior
- Identify what the code does wrong
- Implement the fix, making engine changes as needed
- Do not add workarounds or hacks — fix the root cause

### 5. Write tests

After fixing, write tests that specifically verify the fix. Tests should:
- Cover the exact scenario the audit flagged
- Test edge cases from rulings
- Verify mechanism, not just outcome (e.g., check that a choice is presented, not just that the right thing happens)
- For each ruling, write a test if one doesn't exist

### 6. Run tests

Run `cargo test` and verify all tests pass, not just the new ones.

### 7. Commit

Make one commit per card fix. Commit message should describe what was wrong and how it was fixed. Include the oracle text justification.

### 8. Update TODO

If the card was in TODO.md, mark it as done or remove it.

### 9. Push

Push to remote after each card is fixed and tests pass.
