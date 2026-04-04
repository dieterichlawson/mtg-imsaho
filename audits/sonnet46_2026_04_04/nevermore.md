## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: As this enchantment enters, choose a nonland card name.
Spells with the chosen name can't be cast.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues

- **Auto-selection of card name instead of player choice** (`mtg-engine/src/cards/isd/nevermore.rs:41-53`)
  - Oracle text says: `"As this enchantment enters, choose a nonland card name."`
  - Code does: Auto-selects the first nonland card from the *opponent's hand* (with a hardcoded fallback to `"Lightning Bolt"` if the opponent's hand has no nonland cards). The controller is never asked to choose, and only cards currently in the opponent's hand are candidates — not any nonland card name as the oracle requires.
  ```rust
  let chosen_name = state.objects.values()
      .filter(|o| o.zone == Zone::Hand && o.owner == opponent)
      .filter_map(|o| {
          registry.card_data(o.card_id).and_then(|d| {
              if !d.card_types.contains(&CardType::Land) {
                  Some(d.name)
              } else {
                  None
              }
          })
      })
      .next()
      .unwrap_or_else(|| "Lightning Bolt".into()); // Default if nothing found.
  ```

- **Nevermore ban not enforced for flashback casts** (`mtg-engine/src/engine.rs:665-747`)
  - Oracle text says: `"Spells with the chosen name can't be cast."`
  - Code does: The Nevermore ban is checked only in the "Cast spells from hand" section (`engine.rs:488-491`). The "Cast spells via flashback from graveyard" section (`engine.rs:665-747`) contains no Nevermore check at all, so a card named by Nevermore can still be legally cast via flashback. The relevant ban block appears only once:
  ```rust
  // Check Nevermore: spells with the banned name can't be cast.
  if nevermore_banned.iter().any(|n| *n == data.name) {
      continue;
  }
  ```
  This block is absent from the flashback loop (lines 665–747).

### Tricky interactions checked

- **"Choose" is player-driven, not auto-selected**: FAIL — engine auto-selects from opponent's hand with a hardcoded fallback, never presenting a choice to the controller.
- **Choice scope is any nonland card name**: FAIL — code is limited to cards currently in the opponent's hand; the oracle allows naming any nonland card name regardless of zone or visibility.
- **Ban applies to all casting methods (hand and flashback)**: FAIL — ban check is absent from the flashback-from-graveyard loop in `engine.rs`.
- **Ban applies to all players (not just opponent)**: PASS — `legal_actions` is called per player, so the check at line 488-491 applies to whoever calls it; the ban correctly prevents any player from casting the named spell from hand.
- **Named card can still be put onto the battlefield (not cast)**: PASS — the restriction is enforced only in `legal_actions` for casting; no restriction on ETB or other put-onto-battlefield effects.
- **Nevermore leaving the battlefield lifts the restriction**: PASS — the ban list (`nevermore_banned`) is built dynamically at the start of each `legal_actions` call by filtering objects currently on the battlefield; once Nevermore leaves, its entry is gone.
- **Spells already on stack when Nevermore enters are unaffected**: PASS — the ban only prevents new cast actions; spells already on the stack are unaffected.
- **ETB timing ("As" vs triggered ability)**: PASS (functionally) — although the oracle says "As" (a replacement/simultaneous effect), the engine implements this as an ETB trigger. However, `process_triggers` resolves all triggers before returning priority to any player, so no player can cast the named card in the window between Nevermore entering and the trigger resolving. The functional result is correct.
- **Hardcoded fallback to "Lightning Bolt"**: FAIL — if the opponent has no nonland cards in hand when Nevermore enters, the code names "Lightning Bolt" unconditionally. The oracle allows the controller to choose any nonland name in this situation; "Lightning Bolt" may or may not be the controller's intended choice.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Controller chooses any nonland card name (free choice): NOT TESTED — tests bypass ETB by manually setting `instance_oracle_text`
- Auto-selection from opponent's hand is wrong: NOT TESTED
- Hardcoded fallback to "Lightning Bolt": NOT TESTED
- Named spell cannot be cast from hand: `mtg-engine/tests/tier14_cards.rs:247` (`nevermore_prevents_named_spell`)
- Non-named spells are still castable: `mtg-engine/tests/tier14_cards.rs:271` (`nevermore_allows_other_spells`)
- Named spell cannot be cast via flashback: NOT TESTED
- Controller of Nevermore also cannot cast the named spell: NOT TESTED
- Nevermore leaving the battlefield lifts the restriction: NOT TESTED
- Spells already on stack when Nevermore enters are unaffected: NOT TESTED
- Named card can still be put onto the battlefield (not cast): NOT TESTED
