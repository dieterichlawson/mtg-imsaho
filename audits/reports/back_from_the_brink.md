## Audit — 2026-04-02 20:33

**Oracle text source**: Scryfall API (cached 2026-04-01), https://scryfall.com/card/isd/44/back-from-the-brink
**Oracle text**: Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card. Activate only as a sorcery.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues
- X-cost creatures in graveyard are not handled per ruling. If a creature card with {X} in its mana cost (e.g., Mikaeus, the Lunarch) is in the graveyard, the generated ability includes `ManaSymbol::X` in its cost. The engine's X-cost handler (engine.rs:1719-1731) drains the player's entire mana pool for X. Per ruling: "If the exiled creature card has {X} in its mana cost, X is considered to be zero."
  - Oracle ruling says: `If the exiled creature card has {X} in its mana cost, X is considered to be zero.`
  - Code does: `activated_abilities()` at back_from_the_brink.rs:65 passes through `ManaSymbol::X` from the registry cost, causing the engine to treat X as variable rather than 0.
  - Fix: Filter out `ManaSymbol::X` from the creature's mana cost when building the ability cost.
- Exile of creature happens as effect, not as cost. The oracle text places "Exile a creature card from your graveyard" before the colon (cost), but the code exiles in `on_activate_ability` (back_from_the_brink.rs:110), which runs after cost payment. No Stifle-like effects exist in the card pool, so this is not functionally impactful currently, but is a rules-purity concern.

### Tricky interactions checked
- Sorcery-speed restriction: PASS. `sorcery_speed_only: true` correctly enforced.
- Token copy preserves creature characteristics (name, P/T, types, subtypes, keywords, card behavior): PASS. `create_token_copy` copies card_id so the token gets the same CardBehavior (state.rs:444).
- Creature must still be in graveyard at resolution: PASS. `on_activate_ability` verifies `o.zone == Zone::Graveyard` before proceeding (back_from_the_brink.rs:99-103).
- Only creature cards in controller's graveyard are eligible: PASS. `objects_in_zone(Zone::Graveyard, controller)` filters by owner (correct per rule 400.3), and creature detection checks `power.is_some()` or registry card types (back_from_the_brink.rs:54-59).
- Multiple creatures in graveyard generate separate abilities: PASS. Tested in `back_from_the_brink_ability_per_creature_in_graveyard`.

### Test coverage
- Basic token creation: `tier15_cards.rs:813` (back_from_the_brink_creates_token_copy)
- One ability per graveyard creature: `tier15_cards.rs:845` (back_from_the_brink_ability_per_creature_in_graveyard)
- No abilities without creatures: `tier15_cards.rs:883` (back_from_the_brink_no_abilities_without_creatures_in_graveyard)
- Ability uses creature's mana cost: `tier15_cards.rs:900` (back_from_the_brink_uses_creature_mana_cost)
- X-cost creature in graveyard (X=0 ruling): NOT TESTED
- Creature removed from graveyard before resolution: NOT TESTED (but code handles it via zone check)

---

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card. Activate only as a sorcery.
**Type line**: Enchantment
**Status**: PASS
### Code issues
No issues found.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.
