# Audit: Armored Skaab

## Reference (Scryfall/API)
- **Name:** Armored Skaab
- **Mana Cost:** {2}{U}
- **Type:** Creature — Zombie Warrior
- **Oracle:** When Armored Skaab enters the battlefield, mill four cards.
- **P/T:** 1/4

## Implementation: `armored_skaab.rs`
- **Name:** Armored Skaab -- CORRECT
- **Mana Cost:** {2}{U} -- CORRECT
- **Type:** Creature — Zombie Warrior -- CORRECT
- **Subtypes:** ["Zombie", "Warrior"] -- CORRECT
- **P/T:** 1/4 -- CORRECT
- **Triggered ability:** EntersBattlefield, mills 4 cards -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: When this creature enters, mill four cards.
**Type line**: Creature — Zombie Warrior
**Status**: ISSUE
### Code issues
1. **Oracle text wording mismatch (cosmetic)**: Oracle says `"When this creature enters, mill four cards."` but code oracle_text field says `"When Armored Skaab enters the battlefield, mill four cards."` The code uses the old ETB template instead of the updated "this creature enters" template.
   - Code: `"When Armored Skaab enters the battlefield, mill four cards."`
   - Oracle: `"When this creature enters, mill four cards."`

Behavior is otherwise correct: triggered ability on EntersBattlefield calls mill_cards(state, controller, 4). Stats (1/4), cost ({2}{U}), types (Creature — Zombie Warrior) all match.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "When this creature enters, mill four cards." (was "When Armored Skaab enters the battlefield, mill four cards."). Doc comment updated. Behavior unchanged.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-01

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/43/armored-skaab
**Oracle text**: When this creature enters, mill four cards.
**Type line**: Creature — Zombie Warrior
**Mana cost**: {2}{U}
**P/T**: 1/4
**Keywords**: Mill
**Ruling**: [2011-09-22] If you have fewer than four cards in your library when Armored Skaab enters, you'll put all of them into your graveyard.
**Status**: PASS

### Code issues
No issues found.

All card data fields match oracle text exactly:
- Name: "Armored Skaab" -- matches
- Mana cost: Generic(2) + Blue -- matches {2}{U}
- Card types: Creature -- matches
- Supertypes: none -- matches
- Subtypes: ["Zombie", "Warrior"] -- matches "Zombie Warrior"
- P/T: 1/4 -- matches
- Oracle text field: "When this creature enters, mill four cards." -- matches
- Triggered ability: TriggerKind::EntersBattlefield declared, on_enter_battlefield implemented -- correct
- Mill keyword: "Mill" is a keyword action, not a keyword ability; correctly implemented via `mill_cards()` engine function rather than in the `keywords` vec

### Tricky interactions checked
- ETB trigger through trigger system (not direct execution): pass -- on_enter_battlefield called from triggers.rs resolve_pending_triggers
- Fewer than 4 cards in library (ruling): pass -- mill_cards breaks on empty library, mills as many as available
- No targeting (mill is self-mill, not targeted): pass -- mills controller's library, no targeting needed

### Test coverage
- Basic ETB mill effect: NOT TESTED
- Fewer than 4 cards in library (ruling): NOT TESTED
- No tests exist for this card in mtg-engine/tests/

## Audit — 2026-04-02 20:28

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/43/armored-skaab
**Oracle text**: When this creature enters, mill four cards.
**Type line**: Creature — Zombie Warrior
**Status**: PASS

### Code issues
No issues found.

All card data fields verified against Scryfall oracle:
- Name: "Armored Skaab" -- matches
- Mana cost: Generic(2) + Blue -- matches {2}{U}
- Card types: [Creature] -- matches
- Supertypes: [] -- correct (none on oracle)
- Subtypes: ["Zombie", "Warrior"] -- matches
- P/T: 1/4 -- matches
- Oracle text field: "When this creature enters, mill four cards." -- exact match
- Triggered ability: TriggerKind::EntersBattlefield registered, on_enter_battlefield calls mill_cards(state, controller, 4) -- correct
- Keywords vec: empty -- correct (Mill is a keyword action, not a keyword ability; implemented via engine function)

### Tricky interactions checked
- ETB mills controller (not opponent): pass -- uses controller lookup from the entering object, mills self
- Fewer than 4 cards in library (ruling [2011-09-22]): pass -- mill_cards breaks when library is empty, mills as many as available
- No targeting required: pass -- "mill four cards" affects the controller's own library without targeting
- Trigger goes through trigger system (not hardcoded): pass -- TriggeredAbilityDef registered, engine dispatches via resolve_pending_triggers

### Test coverage
- mill_cards engine function (basic behavior): `flashback.rs:166` (mill_cards_moves_to_graveyard)
- Armored Skaab ETB trigger specifically: NOT TESTED
- Fewer than 4 cards in library: NOT TESTED
