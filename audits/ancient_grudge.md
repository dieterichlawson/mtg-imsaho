# Audit: Ancient Grudge

## Reference (Scryfall/API)
- **Name:** Ancient Grudge
- **Mana Cost:** {1}{R}
- **Type:** Instant
- **Oracle:** Destroy target artifact. Flashback {G}
- **P/T:** N/A

## Implementation: `ancient_grudge.rs`
- **Name:** Ancient Grudge -- CORRECT
- **Mana Cost:** {1}{R} -- CORRECT
- **Type:** Instant -- CORRECT
- **Flashback cost:** {G} -- CORRECT
- **Target:** PermanentWithFilter(HasCardType(Artifact)) -- CORRECT
- **Effect:** Destroy target artifact via `resolve_destroy` -- CORRECT (uses destruction pipeline)

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Destroy target artifact.\nFlashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Ancient Grudge", cost {1}{R}, type Instant, flashback_cost {G}. Target requirement correctly filters for artifacts on the battlefield. on_resolve delegates to resolve_destroy helper. All correct.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-01

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Destroy target artifact.
Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Mana cost**: {1}{R}
**Status**: PASS

### Code issues
No issues found.

- Mana cost: `Generic(1), Colored(Red)` = {1}{R} -- matches oracle "{1}{R}"
- Card types: `vec![CardType::Instant]` -- matches oracle "Instant"
- Supertypes: `vec![]` -- correct, oracle has none
- Subtypes: `vec![]` -- correct, oracle has none
- Power/toughness: `None` -- correct, instant has no P/T
- Keywords: `vec![]` -- correct, `Keyword` enum does not include Flashback; flashback is represented via the `flashback_cost` field (consistent with other flashback cards like Spider Spawning)
- Oracle text: `"Destroy target artifact.\nFlashback {G}"` -- matches oracle
- Flashback cost: `Some(ManaCost::new(vec![ManaSymbol::Colored(Color::Green)]))` = {G} -- matches oracle "Flashback {G}"
- `target_requirement`: `PermanentWithFilter(TargetFilter::HasCardType(vec![CardType::Artifact]))` -- correctly requires targeting an artifact permanent
- `is_valid_target`: checks `zone == Zone::Battlefield` and `card_types.contains(&CardType::Artifact)` -- correct
- `on_resolve`: delegates to `resolve_destroy` helper which calls `try_destroy` (handles indestructible correctly) and `move_spell_after_resolve` (handles flashback exile correctly) -- correct, no anti-patterns

### Tricky interactions checked
- Destroy vs indestructible: PASS -- uses `try_destroy` pipeline which respects indestructible
- Fizzle on invalid target: PASS -- engine handles fizzle when target leaves battlefield before resolution
- Flashback exile after resolution: PASS -- `move_spell_after_resolve` checks `cast_with_flashback` flag and exiles accordingly
- Flashback exile after countering: PASS -- handled by engine's stack resolution for countered flashback spells

### Test coverage
- Basic "destroy target artifact" effect: NOT TESTED (no dedicated Ancient Grudge tests exist)
- Flashback from graveyard: tested generically in `mtg-engine/tests/flashback.rs` (using Geistflame and other cards, but not Ancient Grudge specifically)
- Flashback exile after resolution: tested generically in `mtg-engine/tests/flashback.rs:86` (`flashback_spell_is_exiled_after_resolve`)
- Flashback exile when countered: tested generically in `mtg-engine/tests/flashback.rs:129` (`flashback_spell_countered_is_exiled`)
- Fizzle when target removed: NOT TESTED for this card specifically
- Target validation (only artifacts): NOT TESTED

## Audit — 2026-04-02 20:28

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Destroy target artifact.\nFlashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

All card data fields verified against oracle:
- Name: "Ancient Grudge" -- matches
- Mana cost: `Generic(1), Colored(Red)` = {1}{R} -- matches oracle "{1}{R}"
- Card types: `vec![CardType::Instant]` -- matches oracle "Instant"
- Supertypes: `vec![]` -- correct, none in oracle
- Subtypes: `vec![]` -- correct, none in oracle
- Power/toughness: `None` -- correct for instant
- Keywords: `vec![]` -- correct; `Keyword` enum has no Flashback variant; flashback modeled via `flashback_cost` field (consistent with all other flashback cards)
- Oracle text: `"Destroy target artifact.\nFlashback {G}"` -- matches (reminder text omitted per convention)
- Flashback cost: `Some(ManaCost::new(vec![ManaSymbol::Colored(Color::Green)]))` = {G} -- matches oracle "Flashback {G}"
- Target requirement: `PermanentWithFilter(TargetFilter::HasCardType(vec![CardType::Artifact]))` -- correctly targets artifact permanents
- `is_valid_target`: verifies `zone == Zone::Battlefield` and `card_types.contains(&CardType::Artifact)` -- correct
- `on_resolve`: delegates to `helpers::resolve_destroy` which calls `try_destroy` (respects indestructible) and `move_spell_after_resolve` (exiles flashback spells) -- correct

### Tricky interactions checked
- Flashback exile after resolution: PASS -- `move_spell_after_resolve` in `state.rs:1132` checks `cast_with_flashback` and exiles; per ruling "A spell cast using flashback will always be exiled afterward"
- Flashback exile when countered/fizzled: PASS -- `stack.rs:84` calls `move_spell_after_resolve` on fizzle, which exiles flashback spells; per ruling "whether it resolves, is countered, or leaves the stack in some other way"
- Mana value unchanged when cast via flashback: PASS -- engine determines mana value from `data.cost` (the printed mana cost {1}{R}), never from the flashback cost; per ruling "The mana value of the spell is determined only by its mana cost"
- Hexproof on target artifact: PASS -- engine calls `can_be_targeted` before `is_valid_target` in `PermanentWithFilter` path at `engine.rs:1069`
- Destroy vs indestructible: PASS -- `resolve_destroy` uses `try_destroy` pipeline which respects indestructible

### Test coverage
- Basic "destroy target artifact" effect: NOT TESTED (no dedicated Ancient Grudge tests)
- Flashback from graveyard: tested generically via other flashback cards in `mtg-engine/tests/flashback.rs`
- Flashback exile after resolution: tested generically in `mtg-engine/tests/flashback.rs` (`flashback_spell_is_exiled_after_resolve`)
- Flashback exile when countered: tested generically in `mtg-engine/tests/flashback.rs` (`flashback_spell_countered_is_exiled`)
- Target validation (only artifacts): NOT TESTED
- Fizzle when target removed: NOT TESTED for this card specifically
