## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/52/deranged-assistant?utm_source=api
**Type line**: `Creature — Human Wizard` — {1}{U}, 1/1
**Oracle text**:
```
{T}, Mill a card: Add {C}. (To mill a card, put the top card of your library into your graveyard.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{T}, **Mill a card**: Add {C}" — a mana ability with a side effect, which is
  why it is declared `has_side_effects`: PASS
- The mill goes through the pipeline, so a creature card among it emits
  `CreatureCardMilled`: PASS
- A mana ability does not use the stack (CR 605.1a), so the mill happens
  immediately: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The mana and the mill: `cards_lands_and_mana_sources.rs`, `mana_ability_offers.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/52/deranged-assistant?utm_source=api
**Type line**: `Creature — Human Wizard` — {1}{U}, 1/1
**Oracle text**:
```
{T}, Mill a card: Add {C}. (To mill a card, put the top card of your library into your graveyard.)
```

**Rulings fetched**:
- [2025-01-24] Once Deranged Assistant’s ability has been activated, it can’t be reversed for any reason. If you activate it while casting a spell and discover you can’t produce enough mana to pay that spell’s costs, the spell is reversed. The spell returns to whatever zone you were casting it from. You may reverse other mana abilities you activated while casting the spell, but Deranged Assistant’s ability can’t be reversed. You’ll still have any mana the ability produced, and the milled card will still be in your graveyard.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**: `{T}, Mill a card: Add {C}. (To mill a card, put the top card of your library into your graveyard.)`
**Type line**: `Creature — Human Wizard` — {1}{U}, 1/1
**Status**: ISSUE (fixed) — engine-level, found through this card

### Ruling (2025-01-24)
"Once Deranged Assistant's ability has been activated, it can't be reversed for any reason. If you activate it while casting a spell and discover you can't produce enough mana to pay that spell's costs, the spell is reversed. The spell returns to whatever zone you were casting it from. You may reverse other mana abilities you activated while casting the spell, but Deranged Assistant's ability can't be reversed. You'll still have any mana the ability produced, and the milled card will still be in your graveyard."

### Rules research

CR 701.17b, obtained by WebSearch (mtg.wiki, mtg.fandom.com and yawgatog.com are all blocked by this environment's egress proxy; two independent searches returned the same text): "A player can't mill a number of cards greater than the number of cards in their library. If given the choice to do so, they can't choose to take that action. If instructed to do so, they mill as many as possible. Similarly, the player can't pay a cost that includes milling a number of cards greater than the number of cards in their library."

This is the rule the card's own gate rests on, and it confirms it: milling one card with an empty library is 1 > 0, so the cost cannot be paid and the ability cannot be activated. The gate is right.

### Code issues

- `mtg-engine/src/engine/mana_sources.rs:164` — the tap-cost gate was applied where the action is *offered* but not where the tap *happens*.
  - `available_mana_abilities`'s own doc says: `Every caller that needs a permanent's mana abilities goes through here rather than calling CardBehavior::mana_abilities directly, so the cost-legality gate is applied in exactly one place`
  - `activate_mana_source` did: `let abilities = behavior.mana_abilities(state, source_id);`
  - `activate_mana_source` is every path that taps a permanent for mana — the `ActivateManaAbility` action, `CastSpell` tap plans, `pay_cost_with_sources`, `funding.rs`, the mid-resolution may-pay plans in `choices.rs`. None of them re-checked zone, tapped, or summoning sickness. Calling it on a summoning-sick Deranged Assistant tapped it, produced {C} and milled a card, against CR 302.6.
  - It matters beyond a hand-built action: a tap plan is computed in full before any of it runs, and the state moves underneath it. Two Assistants over a one-card library are both offered; after the first mills that card the second's cost is unpayable (CR 701.17b), and the card's own list still offered it. Fixed by looking the ability up through `available_mana_abilities`, which is what the doc already claimed.

Nothing wrong in the card. `{1}{U}`, Human Wizard, 1/1, oracle text verbatim, `ManaType::Colorless` for `{C}`, `requires_tap: true`, `has_side_effects: true`, the mill through `engine::mill_cards` (so `CreatureCardMilled` fires for an opponent's Undead Alchemist and the log names the source). Scryfall lists `Mill` under keywords; it is a keyword *action*, not a keyword ability, and nothing in `Keyword` corresponds to it — correctly absent from `keywords`.

### Tricky interactions checked

- CR 701.17b, empty library: PASS. `mana_abilities` returns nothing when `library_order.is_empty()`, which is exactly the right gate for milling one.
- The condition lives in the card, the tap cost in the engine: PASS, and now true at execution as well as at enumeration.
- Mill is a cost, not an effect: PASS in substance. A mana ability uses no stack (CR 605.3a), so cost-then-effect is not separately observable; the ruling's point — that the mill is irreversible and the mana stays even if the spell is reversed — holds because `activate_mana_source` commits both before any spell-level rollback.
- Milling your own library, not an opponent's: PASS, `controller_of`.
- The milled card reaching other watchers: PASS. `mill_cards` is the shared pipeline, so a creature card milled here emits `CreatureCardMilled` for Undead Alchemist and appears to Skaab Ruinator, Moldgraf Monstrosity and the flashback cards.
- Summoning sickness on the turn it arrives (CR 302.6): PASS, via `can_pay_tap_cost` — this is what the fix restores at execution.
- Auto-tap opportunity cost: PASS. `has_side_effects: true` puts it in `ManaSourceKind::HasSideEffects`, the last tier, which also outranks the colour-demand tiebreak. Documented in `mtg-player/src/llm.rs:144`; untested until this audit.

### Test coverage

- Taps for {C} through `legal_actions` and `submit_action`: `cards_lands_and_mana_sources.rs:242` `deranged_assistant_taps_for_colorless`
- The mill half of the cost — card to graveyard, library empty, Assistant tapped: same test, added this audit
- Ability withdrawn once the library is empty: `tap_cost_legality.rs:104` `a_mana_ability_keeps_its_own_conditions`, and at the action level in `deranged_assistant_taps_for_colorless` (added this audit)
- CR 701.17b, a second Assistant over a one-card library: `cards_lands_and_mana_sources.rs:270` `a_second_deranged_assistant_cannot_mill_an_empty_library`, added this audit
- The tap-cost gate at execution (summoning-sick, tapped, off-battlefield): `tap_cost_legality.rs:305` `activating_a_mana_ability_re_checks_the_tap_cost`, added this audit
- Auto-tap prefers a land: `cards_lands_and_mana_sources.rs:300` `autotap_taps_a_land_before_it_mills_you`, added this audit
- Auto-tap prefers another creature's mana even against colour demand: `cards_lands_and_mana_sources.rs:334` `autotap_would_rather_lose_a_colour_than_mill_a_card`, added this audit
- The ruling's irreversibility on a failed cast: NOT TESTED — the engine has no partial-cast rollback to reverse, so there is no state in which the claim differs

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 drop the empty-library gate | `a_mana_ability_keeps_its_own_conditions` FAILED | + `a_second_deranged_assistant_...` FAILED |
| M2 mill 0 instead of 1 | passed whole workspace | `deranged_assistant_taps_for_colorless`, `a_second_deranged_assistant_...` FAILED |
| M3 produce {U} instead of {C} | `deranged_assistant_taps_for_colorless` FAILED | (unchanged) |
| M4 mill the opponent | passed whole workspace | `deranged_assistant_taps_for_colorless`, `a_second_deranged_assistant_...` FAILED |
| M5 `requires_tap: false` | `deranged_assistant_taps_for_colorless` FAILED | + `activating_a_mana_ability_re_checks_the_tap_cost` FAILED |
| M6 `has_side_effects: false` | passed whole workspace | `autotap_would_rather_lose_a_colour_than_mill_a_card` FAILED |
| M7 `activate_mana_source` back to `behavior.mana_abilities` | n/a (was the bug) | `activating_a_mana_ability_re_checks_the_tap_cost` FAILED |

M2, M4 and M6 each passed the entire workspace before this audit: the mill was never checked at all, and the auto-tap tier was never checked at all.

Note on M6: a first attempt (`autotap_taps_a_land_before_it_mills_you`) did **not** catch it — demoting the Assistant from `HasSideEffects` to `Creature` still leaves it behind a basic land. The discriminating case needs another *creature* mana source and a colour demand pulling the other way, which is what the second test sets up. Recorded because the first version looked like a passing test of the same thing and was not.

Sources restored from `/tmp/da.bak` and `/tmp/ms_src.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1473 passing (was 1469). `cargo check --workspace --all-targets` clean, zero warnings.
