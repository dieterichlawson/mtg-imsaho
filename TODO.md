# TODO

## Game state serialization
Add the ability to serialize a game state to a file and resume from it. This would let us set up specific board/hand/mana configurations to test particular interactions (e.g. Counterspell with 2 untapped Islands and an opponent's spell on the stack) without relying on RNG to produce the right conditions.

---

# Audit Bug List

## Open

- [ ] **Skaab Ruinator** — Casting from graveyard panics. The engine treats graveyard-zone casts as flashback and tries `data.flashback_cost.expect(...)`, but Skaab Ruinator uses `can_cast_from_graveyard()` not flashback. The `legal_actions` function handles this correctly but `apply_action` does not.

- [ ] **Harvest Pyre** — Exiles ALL graveyard cards instead of letting the player choose X. Oracle says "exile X cards from your graveyard" — player should choose how many. Needs a "choose a number" UI the engine doesn't have yet.

- [ ] **Inquisitor's Flail** — Oracle text field says "another source" but Scryfall says "another creature." Cosmetic only — behavior is correct.

