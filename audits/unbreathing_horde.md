# Audit: Unbreathing Horde

## Scryfall Reference
- **Name:** Unbreathing Horde
- **Cost:** {2}{B}
- **Type:** Creature — Zombie
- **Oracle:** This creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard. If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.
- **P/T:** 0/0

## Implementation: `mtg-engine/src/cards/unbreathing_horde.rs`
- Name: "Unbreathing Horde" -- MATCH
- Cost: {2}{B} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Zombie"] -- MATCH
- P/T: 0/0 -- MATCH
- Trigger: EntersBattlefield -- MATCH

### ETB Counter Logic
- Counts other Zombies on battlefield under controller (excludes self) -- MATCH
- Counts Zombie cards in controller's graveyard -- MATCH
- Adds +1/+1 counters equal to total -- MATCH

### ISSUE: Missing Damage Prevention
- Oracle: "If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it."
- The implementation does NOT implement this damage replacement effect. The code comment mentions indestructible as an approximation, but neither indestructible nor the damage prevention is actually implemented.
- **BUG**: The creature takes damage normally instead of preventing it and removing a counter. This changes the card's behavior significantly -- it should survive any single damage event (removing 1 counter regardless of damage amount), but instead it takes lethal damage as normal.

## Verdict
**FAIL** — Missing damage prevention replacement effect. ETB counters work correctly.

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: This creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard. If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.
**Scryfall type line**: Creature -- Zombie
**Status**: PASS

Previous damage prevention issue has been fixed. The implementation now includes `ContinuousEffect::PreventDamageRemoveCounter { scope: EffectScope::OnSelf }` in `continuous_effects`, which hooks into the engine's damage system to prevent damage and remove a +1/+1 counter.

Verified correct:
- Mana cost: {2}{B} -- matches
- Types: Creature -- matches
- Subtypes: Zombie -- matches
- P/T: 0/0 -- matches
- ETB: counts other Zombies on battlefield under controller + Zombie cards in graveyard, adds that many +1/+1 counters -- correct
- Damage prevention: `PreventDamageRemoveCounter` continuous effect on self -- correct
- `triggered_abilities`: EntersBattlefield -- correct
- Note: oracle text in code uses older templating ("Unbreathing Horde enters the battlefield" vs Scryfall's "This creature enters") -- cosmetic only, no functional impact
- No anti-patterns detected
- Tests found in `mtg-engine/tests/tier15_cards.rs` and `mtg-engine/tests/unbreathing_horde.rs`

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: This creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard. If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.
**Type line**: Creature — Zombie
**Status**: PASS

Card data correct: name, mana cost ({2}{B}), type (Creature), subtypes (Zombie), P/T (0/0).

ETB logic: counts other battlefield Zombies under controller (excludes self) + graveyard Zombies, adds that many +1/+1 counters. Correct.

Damage prevention: implemented via ContinuousEffect::PreventDamageRemoveCounter with EffectScope::OnSelf. Correct.

triggered_abilities declares EntersBattlefield. Correct.

Tests in unbreathing_horde.rs cover damage prevention with counter removal, still dealing damage to others, and ETB counter count. All correct. No anti-patterns found.
