# Audit: Manor Skeleton

## Oracle (Official)
- **Name:** Manor Skeleton
- **Cost:** {1}{B}
- **Type:** Creature — Skeleton
- **Oracle:** Haste. {1}{B}: Regenerate Manor Skeleton.
- **P/T:** 1/1

## Implementation
- Name: "Manor Skeleton" -- CORRECT
- Cost: {1}{B} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Skeleton"] -- CORRECT
- P/T: 1/1 -- CORRECT
- Keywords: [Haste] -- CORRECT
- Oracle text matches -- CORRECT
- Activated ability: {1}{B} regenerate -- CORRECT
- Uses regeneration_shields mechanism -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Manor Skeleton
- **Cost:** {1}{B}
- **Type:** Creature — Skeleton
- **P/T:** 1/1
- **Oracle Text:** Haste / {1}{B}: Regenerate this creature.

### Card Data Checks
- [x] Name: "Manor Skeleton" — correct
- [x] Cost: {1}{B} — correct
- [x] Types: Creature — correct
- [x] Subtypes: Skeleton — correct
- [x] P/T: 1/1 — correct
- [x] Keywords: Haste — correct
- [ ] Oracle text: minor mismatch (cosmetic)
  - **Oracle:** `"{1}{B}: Regenerate this creature."`
  - **Implementation:** `"{1}{B}: Regenerate Manor Skeleton."`
  - Note: Scryfall uses modern "this creature" templating; implementation uses card name. Functionally equivalent.

### Behavior Checks
- [x] Haste keyword present — correct
- [x] Activated ability costs {1}{B} — correct
- [x] Ability only available on battlefield — correct
- [x] Does not require tap — correct
- [x] Adds regeneration shield via `obj.regeneration_shields += 1` — correct

### Result: PASS
