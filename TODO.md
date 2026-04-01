# TODO

## Game state serialization
Add the ability to serialize a game state to a file and resume from it. This would let us set up specific board/hand/mana configurations to test particular interactions (e.g. Counterspell with 2 untapped Islands and an opponent's spell on the stack) without relying on RNG to produce the right conditions.

---

# Audit Bug List

## Open

- [ ] **Skaab Ruinator** — Casting from graveyard panics. The engine treats graveyard-zone casts as flashback and tries `data.flashback_cost.expect(...)`, but Skaab Ruinator uses `can_cast_from_graveyard()` not flashback. The `legal_actions` function handles this correctly but `apply_action` does not. Also missing graveyard eligibility check (3+ creature cards) for graveyard casts.

- [ ] **Harvest Pyre** — Exiles ALL graveyard cards instead of letting the player choose X. Oracle says "exile X cards from your graveyard" — player should choose how many. Needs engine support for choosing a number.

- [ ] **Inquisitor's Flail** — Oracle text field says "another source" but Scryfall says "another creature." Cosmetic only — behavior is correct.

- [ ] **Blazing Torch** — `is_valid_target` doesn't restrict equip ability to "creature you control." The equip and damage abilities share `is_valid_target`, so the equip ability allows targeting opponent's creatures. Needs ability-index-aware targeting or separate validation.

- [ ] **Garruk Relentless** — Back face -1 ability (sacrifice + library search) doesn't shuffle the library after searching. Oracle says "then shuffle."
