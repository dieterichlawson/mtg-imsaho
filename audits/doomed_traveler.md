# Audit: Doomed Traveler

## Reference (Scryfall)
- **Name:** Doomed Traveler
- **Cost:** {W}
- **Type:** Creature -- Human Soldier
- **Oracle:** When Doomed Traveler dies, create a 1/1 white Spirit creature token with flying.
- **P/T:** 1/1

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({W})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Human, Soldier)
- Oracle text: CORRECT
- P/T: CORRECT (1/1)
- Dies trigger: CORRECT (TriggerKind::SelfDies)
- Token created: 1/1 white Spirit with flying: CORRECT
- Token subtypes: CORRECT (Spirit)

## Issues
None found.

## Audit 2026-04-02

### Oracle Text (Scryfall, cached 2026-04-01)
- **Name:** Doomed Traveler
- **Mana Cost:** {W}
- **Type Line:** Creature — Human Soldier
- **P/T:** 1/1
- **Oracle Text:** "When this creature dies, create a 1/1 white Spirit creature token with flying."

### Implementation File
`mtg-engine/src/cards/isd/doomed_traveler.rs`

### Card Data Check
- Name: CORRECT ("Doomed Traveler")
- Mana cost: CORRECT ({W} — single white mana)
- Card types: CORRECT (Creature)
- Supertypes: CORRECT (none)
- Subtypes: CORRECT (Human, Soldier)
- Power/Toughness: CORRECT (1/1)
- Keywords: CORRECT (none on the creature itself)

### Death Trigger Check
- `triggered_abilities`: Uses `TriggerKind::SelfDies` — CORRECT for "When this creature dies"
- `on_dies` implementation: Creates token via `create_token_with_subtypes("Spirit", controller, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying], vec!["Spirit".into()])` — CORRECT

### Token Verification
- Token name: CORRECT ("Spirit")
- Token P/T: CORRECT (1/1)
- Token color: CORRECT (White)
- Token types: CORRECT (Creature)
- Token subtypes: CORRECT (Spirit)
- Token keywords: CORRECT (Flying)
- Token controller: CORRECT (uses controller of dying creature, not owner)

### Test Coverage
- `tier3_cards::doomed_traveler_creates_spirit_on_death` — PASSES
  - Verifies creature moves to graveyard on lethal damage
  - Verifies exactly one Spirit token created on battlefield
  - Verifies token P/T is 1/1
  - Verifies token has Flying keyword
  - Note: Test does not assert token color (white), but implementation is correct

### Issues Found
None. Implementation matches oracle text exactly.
