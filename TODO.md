# TODO

## Game state serialization
Add the ability to serialize a game state to a file and resume from it. This would let us set up specific board/hand/mana configurations to test particular interactions (e.g. Counterspell with 2 untapped Islands and an opponent's spell on the stack) without relying on RNG to produce the right conditions.

---

# Audit Bug List

## Open

- [ ] **Skaab Ruinator** — Casting from graveyard panics. The engine treats graveyard-zone casts as flashback and tries `data.flashback_cost.expect(...)`, but Skaab Ruinator uses `can_cast_from_graveyard()` not flashback. The `legal_actions` function handles this correctly but `apply_action` does not.

- [ ] **Harvest Pyre** — Exiles ALL graveyard cards instead of letting the player choose X. Oracle says "exile X cards from your graveyard" — player should choose how many. Needs a "choose a number" UI the engine doesn't have yet.

- [ ] **Inquisitor's Flail** — Oracle text field says "another source" but Scryfall says "another creature." Cosmetic only — behavior is correct.

## Fixed (2026-04-01)

- [x] Blazing Torch — wrong damage source (torch, not creature)
- [x] Civilized Scholar — auto-selects discard instead of player choice
- [x] Curse of the Pierced Heart — missing planeswalker damage option
- [x] Daybreak Ranger — Nightfall Predator fight restricted to opponent only
- [x] Garruk Relentless — transform not state-triggered, missing damage event, auto-selects target
- [x] Geist of Saint Traft — Angel not exiled if Geist dies
- [x] Ghoulcaller's Chant — oracle text says "Zombie creature cards" not "Zombie cards"
- [x] Harvest Pyre — additional cost at resolve instead of cast time
- [x] Inquisitor's Flail — multiple Flails don't stack (x2 not x4)
- [x] Kruin Outlaw — MinimumBlockers instead of menace keyword
- [x] Memory's Journey — can't cast with 0 card targets
- [x] Moonmist — (passed re-audit)
- [x] Olivia Voldaren — self-targeting not excluded, engine ignores CreatureWithFilter
- [x] Skaab Ruinator — additional cost at resolve, no eligibility check
- [x] Stony Silence — mana abilities of artifacts not blocked
