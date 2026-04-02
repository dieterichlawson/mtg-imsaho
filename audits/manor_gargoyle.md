# Audit: Manor Gargoyle

## Oracle (Official)
- **Name:** Manor Gargoyle
- **Cost:** {5}
- **Type:** Artifact Creature — Gargoyle
- **Oracle:** Defender. Manor Gargoyle is indestructible as long as it has defender. {1}: Until end of turn, Manor Gargoyle loses defender and gains flying.
- **P/T:** 4/4

## Implementation
- Name: "Manor Gargoyle" -- CORRECT
- Cost: {5} -- CORRECT
- Types: [Artifact, Creature] -- CORRECT
- Subtypes: ["Gargoyle"] -- CORRECT
- P/T: 4/4 -- CORRECT
- Keywords: [Defender] -- CORRECT
- Oracle text matches -- CORRECT
- Conditional indestructible when it has defender via ConditionalKeyword -- CORRECT
- Activated ability {1}: loses defender, gains flying until end of turn -- CORRECT
- Uses until_end_of_turn_removed_keywords for removing defender -- CORRECT
- Uses UntilEndOfTurnKeyword for granting flying -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Manor Gargoyle
- **Cost:** {5}
- **Type:** Artifact Creature — Gargoyle
- **P/T:** 4/4
- **Oracle Text:** Defender / This creature has indestructible as long as it has defender. / {1}: Until end of turn, this creature loses defender and gains flying.

### Card Data Checks
- [x] Name: "Manor Gargoyle" — correct
- [x] Cost: {5} — correct
- [x] Types: Artifact, Creature — correct
- [x] Subtypes: Gargoyle — correct
- [x] P/T: 4/4 — correct
- [x] Keywords: Defender — correct
- [ ] Oracle text: minor mismatch (cosmetic)
  - **Oracle:** `"This creature has indestructible as long as it has defender."` / `"{1}: Until end of turn, this creature loses defender and gains flying."`
  - **Implementation:** `"Manor Gargoyle has indestructible as long as it has defender."` / `"{1}: Until end of turn, Manor Gargoyle loses defender and gains flying."`
  - Note: Scryfall uses modern "this creature" templating; implementation uses card name. Functionally equivalent.

### Behavior Checks
- [x] Defender keyword present — correct
- [x] Conditional indestructible (when has Defender) via `ContinuousEffect::ConditionalKeyword` — correct
- [x] Activated ability costs {1} — correct
- [x] Ability only available on battlefield — correct
- [x] Does not require tap — correct
- [x] Grants flying until end of turn — correct
- [x] Removes defender until end of turn — correct (which also removes indestructible via the conditional)

### Result: PASS
