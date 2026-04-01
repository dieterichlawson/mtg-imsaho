# Audit: Ghoulcaller's Chant

## Oracle Reference (Scryfall)
- Cost: {B}
- Type: Sorcery
- Oracle: "Choose one --
  * Return target creature card from your graveyard to your hand.
  * Return two target Zombie creature cards from your graveyard to your hand."

NOTE: Current Scryfall oracle text says "Zombie cards" not "Zombie creature cards" for mode 2. However the original Innistrad printing says "Zombie creature cards". The current oracle errata simplified it.

## Implementation: ghoulcallers_chant.rs

## Issues Found

1. **ISSUE: Mode selection is automated, not player-chosen** - The implementation auto-selects mode 2 (return two Zombies) whenever there are 2+ Zombies in graveyard, and falls back to mode 1 otherwise. Per Oracle, the player chooses which mode. A player might want to return a single non-Zombie creature even when Zombies are available.

2. **BUG (from prior audit): Oracle text says "Zombie creature cards" but current errata says "Zombie cards"** - The engine filters for creature AND Zombie (lines 43-49), but updated oracle only requires Zombie subtype. Low severity since all Zombies in the set are creatures.

Otherwise correct: cost ({B}), type (Sorcery), oracle text structure matches.

## Verdict: ISSUES FOUND (2 issues)

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Choose one -- / Return target creature card from your graveyard to your hand. / Return two target Zombie cards from your graveyard to your hand.
**Scryfall type line**: Sorcery
**Status**: ISSUE

Mana cost {B}: correct. Type Sorcery: correct. Uses `move_spell_after_resolve`: correct (no graveyard anti-pattern). Modal targeting via `TargetRequirement::ModalChoice`: correct structure.

on_resolve: moves each targeted card from graveyard to hand, then calls `move_spell_after_resolve`: correct behavior.

Issues found:
1. **Mode 2 filters for "Zombie creature cards" but oracle says "Zombie cards"**: The `TargetRequirement` for mode 2 uses `GraveyardCreatureOfSubtype("Zombie")`, which requires the target to be both a creature card and have the Zombie subtype. The current Scryfall oracle text says "two target Zombie cards" (not "Zombie creature cards"). While all Zombie cards in Innistrad are creatures, this is technically a stricter filter than the oracle requires. Low severity since Zombie is a creature subtype and non-creature Zombie cards are extremely rare.

Tests present in `tests/ghoulcallers_chant.rs`, `tests/innistrad_simple_cards.rs`, and `tests/tier11_cards.rs`.
