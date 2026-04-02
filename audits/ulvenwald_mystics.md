# Audit: Ulvenwald Mystics // Ulvenwald Primordials

## Scryfall Reference
### Front Face
- **Name:** Ulvenwald Mystics
- **Cost:** {2}{G}{G}
- **Type:** Creature — Human Shaman Werewolf
- **Oracle:** At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
- **P/T:** 3/3

### Back Face
- **Name:** Ulvenwald Primordials
- **Cost:** *(none)*
- **Type:** Creature — Werewolf
- **Oracle:** {G}: Regenerate this creature. / At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
- **P/T:** 5/5

## Implementation: `mtg-engine/src/cards/ulvenwald_mystics.rs`

### Front Face
- Name: "Ulvenwald Mystics" -- MATCH
- Cost: {2}{G}{G} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Human", "Shaman", "Werewolf"] -- MATCH
- P/T: 3/3 -- MATCH
- Trigger: Upkeep -- MATCH

### Back Face
- Name: "Ulvenwald Primordials" -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Werewolf"] -- MATCH
- P/T: 5/5 -- MATCH
- Activated ability: {G}: Regenerate (only when transformed) -- MATCH
- Regeneration shield implementation -- CORRECT

## Verdict
**PASS** — Werewolf with regenerate ability correctly implemented.

## Audit — 2026-04-01 15:12

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text (front)**: At the beginning of each upkeep, if no spells were cast last turn, transform Ulvenwald Mystics.
**Oracle text (back)**: {G}: Regenerate Ulvenwald Primordials.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform Ulvenwald Primordials.
**Type line (front)**: Creature — Human Shaman Werewolf
**Type line (back)**: Creature — Werewolf
**Ruling**: [2011-09-22] You can regenerate Ulvenwald Primordials in response to the triggered ability that would transform it. If you do, the regeneration shield will apply to Ulvenwald Mystics that turn.
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Werewolf transform timing (upkeep, no spells / 2+ spells): PASS
- First turn protection (no transform on first turn): PASS
- Regenerate ability only available on back face: PASS — `activated_abilities` checks `o.is_transformed`
- Regeneration shield implementation (increments `regeneration_shields`): PASS
- Regenerate cost is {G} (not tap): PASS — `requires_tap: false`
- Regeneration shield persists across transform: PASS — shield is on the object, not face-specific
- Back face missing Upkeep in triggered_abilities but trigger still fires: PASS (works due to `trigger_description` checking front face first)
- Dynamic P/T for back face (5/5): PASS
- Front face subtypes include all three (Human, Shaman, Werewolf): PASS

### Test coverage
- Transform and gain regenerate ability: `werewolf_cards.rs:ulvenwald_mystics_transforms_and_gains_regenerate` — TESTED
- Front face has no activated abilities: TESTED (in same test)
- Back face regenerate ability description: TESTED (in same test)
- Actually using regenerate ability (activating and gaining shield): NOT TESTED
- Regenerate shield persisting after transform back: NOT TESTED
- Transform back when 2+ spells cast: NOT DIRECTLY TESTED (covered by generic werewolf tests)
