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

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: When this creature dies, create a 1/1 white Spirit creature token with flying.
**Type line**: Creature — Human Soldier
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:54
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: When this creature dies, create a 1/1 white Spirit creature token with flying.
**Type line**: Creature — Human Soldier
**Status**: PASS

### Code issues
None. All card data fields match the oracle reference exactly:
- Name: "Doomed Traveler" -- correct
- Mana cost: {W} (single white) -- correct
- Card types: Creature -- correct
- Supertypes: none -- correct
- Subtypes: Human, Soldier -- correct
- Power/Toughness: 1/1 -- correct
- Keywords: none -- correct
- Triggered ability: `TriggerKind::SelfDies` -- correct for "When this creature dies"
- `on_dies` creates token via `create_token_with_subtypes("Spirit", controller, 1, 1, [White], [Creature], [Flying], ["Spirit"])` -- correct
- Oracle text in implementation says "When Doomed Traveler dies" vs Scryfall's "When this creature dies" -- semantically identical (Scryfall updated self-referential text); behavior is correct

### Tricky interactions checked (min 3)
1. **Controller vs Owner**: `on_dies` uses `o.controller`, not `o.owner`. If Doomed Traveler is stolen (e.g., Traitorous Blood) and then dies, the Spirit token correctly goes to the last controller, not the original owner.
2. **Exile instead of graveyard**: "Dies" means battlefield-to-graveyard only. The `CreatureDied` event is only emitted from SBA when the creature moves to Graveyard, so exile effects (e.g., Fiend Hunter) correctly do NOT trigger the ability.
3. **Parallel Lives doubling**: Token creation goes through `create_token_with_subtypes`, which checks for Parallel Lives and doubles tokens accordingly. Two Spirits would be created if Parallel Lives is present.
4. **Simultaneous death with other dies-triggers**: Doomed Traveler dying alongside other creatures with dies triggers (e.g., Falkenrath Noble, Elder Cathar) -- all triggers are queued and resolved in APNAP order per the trigger system.

### Test coverage
- `tier3_cards::doomed_traveler_creates_spirit_on_death` -- PASSES
  - Verifies creature moves to graveyard on lethal damage
  - Verifies exactly one Spirit token created on battlefield
  - Verifies token P/T is 1/1
  - Verifies token has Flying keyword
- Also used as a Human creature fixture in `card_mechanics` tests for Elder Cathar and Village Cannibals interactions
