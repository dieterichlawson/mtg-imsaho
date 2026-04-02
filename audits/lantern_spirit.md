# Audit: Lantern Spirit

## Oracle (Official)
- **Name:** Lantern Spirit
- **Cost:** {2}{U}
- **Type:** Creature — Spirit
- **Oracle:** Flying. {U}: Return Lantern Spirit to its owner's hand.
- **P/T:** 2/1

## Implementation
- Name: "Lantern Spirit" -- CORRECT
- Cost: {2}{U} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Spirit"] -- CORRECT
- P/T: 2/1 -- CORRECT
- Keywords: [Flying] -- CORRECT
- Activated ability: {U}, returns self to hand -- CORRECT
- No tap required for ability -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Lantern Spirit
- **Cost:** {2}{U}
- **Type:** Creature — Spirit
- **P/T:** 2/1
- **Oracle Text:** Flying / {U}: Return this creature to its owner's hand.

### Card Data Checks
- [x] Name: "Lantern Spirit" — correct
- [x] Cost: {2}{U} — correct
- [x] Types: Creature — correct
- [x] Subtypes: Spirit — correct
- [x] P/T: 2/1 — correct
- [x] Keywords: Flying — correct
- [ ] Oracle text: minor mismatch (cosmetic)
  - **Oracle:** `"{U}: Return this creature to its owner's hand."`
  - **Implementation:** `"{U}: Return Lantern Spirit to its owner's hand."`
  - Note: Scryfall uses modern "this creature" templating; implementation uses card name. Functionally equivalent.

### Behavior Checks
- [x] Flying keyword granted — correct
- [x] Activated ability costs {U} — correct
- [x] Ability only available on the battlefield — correct
- [x] Ability returns self to owner's hand via `state.move_object(object_id, Zone::Hand)` — correct
- [x] Does not require tap — correct

### Result: PASS
