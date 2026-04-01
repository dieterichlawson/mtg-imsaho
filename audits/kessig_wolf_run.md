# Audit: Kessig Wolf Run

## Oracle (Official)
- **Name:** Kessig Wolf Run
- **Cost:** (none — Land)
- **Type:** Land
- **Oracle:** {T}: Add {C}. {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
- **P/T:** N/A

## Implementation
- Name: "Kessig Wolf Run" -- CORRECT
- Cost: None -- CORRECT
- Type: Land -- CORRECT
- Mana ability: {T} for {C} -- CORRECT
- Activated ability: simplified as {1}{R}{G},{T} for +1/+0 and trample (can be activated multiple times) -- SIMPLIFICATION noted in oracle_text and comments
- Grants trample via UntilEndOfTurnKeyword -- CORRECT
- Grants +1/+0 via UntilEndOfTurnEffect -- CORRECT

## Issues
1. **ISSUE (simplification):** X in the mana cost is simplified to 1. The engine doesn't support variable X costs for activated abilities. Noted in code.

## Verdict: PASS (with noted simplification)

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: {T}: Add {C}. {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
**Scryfall type line**: Land
**Status**: PASS

Findings:
- Cost: None (land): correct.
- Type Land: correct.
- P/T N/A: correct.
- Mana ability: {T} for {C}, requires_tap, checks battlefield and untapped: correct.
- Activated ability: cost includes ManaSymbol::X, ManaSymbol::Colored(Red), ManaSymbol::Colored(Green), requires_tap: correct.
- on_activate_ability reads `last_activated_x_value` for X: correct.
- Grants +X/+0 via UntilEndOfTurnEffect and trample via UntilEndOfTurnKeyword: correct.
- Target requirement: Creature: correct.
- Anti-pattern check: No spell resolution involved (land/activated ability). No `move_object(id, Zone::Graveyard)` misuse.
- No CombatDamageDealt misuse.
- No triggered_abilities declared, none needed: correct.
- sorcery_speed_only: false: correct (the ability can be activated at instant speed).
- Tests found in kessig_wolf_run.rs and tier14_cards.rs.
- Previous simplification note about X=1 appears to be FIXED -- the implementation now correctly uses ManaSymbol::X and reads last_activated_x_value.
