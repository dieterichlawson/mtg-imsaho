## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: As an additional cost to cast this spell, exile two creature cards from your graveyard.
Trample
**Type line**: Creature — Zombie Giant
**Status**: ISSUE

### Code issues

- Engine auto-selects which creatures to exile rather than giving the player the choice (`mtg-engine/src/engine.rs:1574–1600`)
  - Oracle text says: `"As an additional cost to cast this spell, exile two creature cards from your graveyard."`
  - Code does: `exile_candidates.sort_by(|a, b| b.1.cmp(&a.1)); // Highest power first` then `let exile_candidates: Vec<_> = exile_candidates.into_iter().take(n).collect();` — the engine silently picks the two highest-power creatures from the graveyard and exiles them, with no player input. Under MTG rules, the casting player must choose which creature cards satisfy the additional cost. This denies player agency and can cause incorrect game outcomes when the player would prefer to keep specific high-power cards (e.g., for Boneyard Wurm / Lumberknot dynamic P/T, or other graveyard synergies).

### Tricky interactions checked

- **Card data (mana cost, types, subtypes, P/T, keywords)**: PASS — `{5}{U}`, Creature — Zombie Giant, 6/9, Trample all match oracle text.
- **`additional_cost` field**: PASS — `AdditionalCost::ExileCreaturesFromGraveyard(2)` correctly reflects the oracle requirement of exactly two creature cards.
- **Eligibility check (must have ≥ 2 creature cards in graveyard to cast)**: PASS — engine correctly gates the cast action on `creature_count >= n` at `engine.rs:553`.
- **Creature card identification (uses both `o.power.is_some()` and registry card type check)**: PASS — the filter at `engine.rs:545–552` and `1576–1581` checks `o.power.is_some()` (covers tokens and objects whose creature type is baked into the object) OR `registry.card_data(...).card_types.contains(&CardType::Creature)` (covers non-token cards whose power field may not be set on the object). Adequate coverage.
- **Player choice of which creatures to exile**: FAIL — see Code Issues above. The engine auto-selects by highest power rather than presenting a choice.
- **Spell resolution (moves to battlefield)**: PASS — `on_resolve` calls `state.move_object(object_id, Zone::Battlefield)`; after `on_resolve` the engine checks `obj.zone == Zone::Stack` before calling `move_spell_after_resolve`, so the already-moved permanent is not double-moved.
- **`move_spell_after_resolve` vs `move_object` for permanent**: PASS — permanent spells that self-move in `on_resolve` are correctly not double-moved by `stack.rs:107–111`.
- **Rooftop Storm interaction (free cast for Zombies)**: PASS — engine checks `data.subtypes.iter().any(|s| s == "Zombie")` at `engine.rs:615`; Skaab Goliath has "Zombie" subtype, so Rooftop Storm would apply if in play. Additional cost still needs to be paid (the alternative cost path only replaces mana cost, not additional costs, which is correct per MTG rules).
- **Exiled creatures correctly excluded from legal-action creature count**: PASS — the eligibility filter excludes `o.id == obj.id` (the spell being cast) but otherwise counts all `Zone::Graveyard` creatures. After exile, those cards are in `Zone::Exile` and would not be counted in a future cast attempt, which is correct.

### Test coverage

- **Two creature cards exiled and Goliath lands on battlefield with 6/9**: `tier11_cards.rs:81` — TESTED
- **Player gets to choose which two creatures are exiled**: NOT TESTED (test uses exactly two creatures, so there is no ambiguity to reveal the auto-selection bug)
- **Cast blocked when fewer than two creature cards in graveyard**: NOT TESTED
- **Trample keyword present on card data**: NOT TESTED (test checks P/T and zone but not keywords)
- **Rooftop Storm interaction**: NOT TESTED
