## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/183/gnaw-to-the-bone?utm_source=api
**Type line**: `Instant` — {2}{G}
**Oracle text**:
```
You gain 2 life for each creature card in your graveyard.
Flashback {2}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

- "for each creature **card** in your graveyard" — filters `!o.is_token`
  (CR 109.1) and excludes itself.
- Emits `LifeChanged`, and only when the gain is non-zero, so no event is
  reported for a life total that did not move.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/183/gnaw-to-the-bone?utm_source=api
**Type line**: `Instant` — {2}{G}
**Oracle text**:
```
You gain 2 life for each creature card in your graveyard.
Flashback {2}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.

"You gain 2 life for each creature card in your graveyard" — counted at
resolution, `is_card` filtered (CR 109.1), and the gain goes through
`state.change_life`, the single writer that emits `LifeChanged`. Instant, so it
can be cast in response to lethal damage; flashback {2}{G}.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_graveyard_interaction.rs` — the count and the life total; `life_events.rs` guards the single-writer rule.

## Audit — 2026-08-28 19:29

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Gnaw to the Bone"`, https://scryfall.com/card/isd/183/gnaw-to-the-bone
**Oracle text**:
```
You gain 2 life for each creature card in your graveyard.
Flashback {2}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: Instant
**Mana cost**: {2}{G}   **Keywords**: Flashback
**Rulings**: 6, all the generic flashback ones.
**Status**: PASS (the count's two qualifying words gained their rows)

### Code issues
No issues found in `mtg-engine/src/cards/isd/gnaw_to_the_bone.rs`.

`{2}{G}`, `CardType::Instant`, `flashback_cost: Some({2}{G})` — same as the front cost, which is
what this card prints — oracle text verbatim, no target requirement.

The count is the Spider Spawning / Boneyard Wurm shape: creature cards (`is_card`, `is_creature`)
in the controller's graveyard (keyed by owner, CR 404.3), counted at resolution, with the
belt-and-braces self-exclusion. Life goes through `change_life`, which emits `LifeChanged` —
verified at the source this audit.

**Flashback interaction worth noting**: cast via flashback, the spell is on the stack while
resolving, so it does not count itself — and the count is naturally one lower than a player
might eyeball, which is the card working, not a bug.

### Tricky interactions checked
- **2 per creature card**: PASS.
- **A land in yours, a creature card in theirs**: nothing. PASS, newly pinned.
- **Counted at resolution**: PASS — a creature dying in response raises it.
- **`LifeChanged` emitted**: PASS.
- **Zero creature cards**: zero life, no event (`change_life` returns on 0).
- **Flashback exile**: engine-side, pinned generically.

### Test coverage
- 3 creature cards = 6 life, with the land and opponent's-card exclusions:
  `flashback.rs:397 gnaw_to_the_bone_gains_life` (extended)
- flashback cost matches print: `card_data_invariants.rs:1907` (sweep)

Mutation-checked: counting any card, counting every graveyard, and 3 life each all fail the test.

### Changes made
- `flashback.rs`: the two exclusion rows. No code change.
