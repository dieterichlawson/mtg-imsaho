# TODO

## Game state serialization
Add the ability to serialize a game state to a file and resume from it. This would let us set up specific board/hand/mana configurations to test particular interactions (e.g. Counterspell with 2 untapped Islands and an opponent's spell on the stack) without relying on RNG to produce the right conditions.

---

# Audit Bug List

## Open

- [ ] **Garruk Relentless** — Back face -1 ability auto-selects weakest creature to sacrifice and first creature found in library. Per MTG rules, both the sacrifice target and the library search result should be player choices.

- [ ] **Moonmist** — Transform filter uses `!o.is_transformed` which skips back-face Humans. Thraben Militia (back face of Thraben Sentry) has Human subtype but gets skipped. Oracle says "Transform all Humans" — any creature with Human subtype should transform regardless of current face.
