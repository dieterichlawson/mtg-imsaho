# Audit: Markov Patrician

## Official Oracle
- **Name:** Markov Patrician
- **Cost:** {2}{B}
- **Type:** Creature — Vampire
- **Oracle:** Lifelink
- **P/T:** 3/1

## Implementation: `mtg-engine/src/cards/markov_patrician.rs`
- **Name:** Markov Patrician -- CORRECT
- **Cost:** {2}{B} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** Vampire -- CORRECT
- **Oracle:** Lifelink -- CORRECT
- **P/T:** 3/1 -- CORRECT
- **Keywords:** Lifelink -- CORRECT

## Verdict
**PASS** -- No issues found.

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Markov Patrician
- **Cost:** {2}{B}
- **Type:** Creature — Vampire
- **P/T:** 3/1
- **Oracle Text:** Lifelink (Damage dealt by this creature also causes you to gain that much life.)

### Card Data Checks
- [x] Name: "Markov Patrician" — correct
- [x] Cost: {2}{B} — correct
- [x] Types: Creature — correct
- [x] Subtypes: Vampire — correct
- [x] P/T: 3/1 — correct
- [x] Keywords: Lifelink — correct
- [x] Oracle text: "Lifelink" — correct

### Behavior Checks
- [x] No abilities beyond the keyword — correct (vanilla + keyword creature)
- [x] Lifelink handled by engine keyword system — correct

### Result: PASS
