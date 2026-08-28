## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/140/devils-play?utm_source=api
**Type line**: `Sorcery` — {X}{R}
**Oracle text**:
```
Devil's Play deals X damage to any target.
Flashback {X}{R}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.

- "{X}{R}: deals X damage to **any target**" — "any target" covers creature,
  player and planeswalker, and the damage goes through the pipeline so CR 120.3c
  loyalty removal applies to a planeswalker.
- X is the value announced on casting (CR 601.2b), covered by
  `x_cost_spells.rs`, which checks the announced X is the X the effect uses.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/140/devils-play?utm_source=api
**Type line**: `Sorcery` — {X}{R}
**Oracle text**:
```
Devil's Play deals X damage to any target.
Flashback {X}{R}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{X}{R}" and "Flashback {X}{R}{R}{R}" — X is announced on both, so the engine
  runs its X-funding prompt for the flashback cast as well: PASS
- "deals X damage to **any target**" — creature, player or planeswalker, and the
  planeswalker branch removes loyalty rather than marking damage (CR 120.3c) —
  this card was the one that wrote `damage_marked` on a planeswalker: PASS
- Damage through the pipeline: PASS
- Ruling: a spell cast with flashback is exiled afterwards whatever happens to
  it: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- X damage and the planeswalker path: `inline_damage.rs`, `damage_helper.rs`, `cards_flashback.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/140/devils-play?utm_source=api
**Type line**: `Sorcery` — {X}{R}
**Oracle text**:
```
Devil's Play deals X damage to any target.
Flashback {X}{R}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Rulings fetched**:
- [2021-03-19] If a card with flashback is put into your graveyard during your turn, you can cast it if it's legal to do so before any other player can take any actions.
- [2021-03-19] "Flashback [cost]" means "You may cast this card from your graveyard by paying [cost] rather than paying its mana cost" and "If the flashback cost was paid, exile this card instead of putting it anywhere else any time it would leave the stack."
- [2021-03-19] You must still follow any timing restrictions and permissions, including those based on the card's type. For instance, you can cast a sorcery using flashback only when you could normally cast a sorcery.
- [2021-03-19] To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.
- [2021-03-19] A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.
- [2021-03-19] You can cast a spell using flashback even if it was somehow put into your graveyard without having been cast.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**:
```
Devil's Play deals X damage to any target.
Flashback {X}{R}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: `Sorcery` — {X}{R}, Flashback {X}{R}{R}{R}
**Status**: ISSUE (fixed)

### Rulings (all 2021-03-19)
Six, all of them the standard flashback rulings — nothing specific to how X or the damage works.

### Code issues

- `mtg-engine/tests/damage_helper.rs:41` — the "any target" sweep could not notice a card leaving it.
  - `every_any_target_spell_can_point_at_a_planeswalker` derives its subjects from the registry by looking for `TargetRequirement::AnyTarget`, and asserts each offers a planeswalker (CR 115.4a). Narrowing Devil's Play to `TargetRequirement::Creature` **passed the whole suite**: the card dropped out of the sweep rather than failing it, and the floor of three that guards the sweep was still met by the others.
  - A sweep derived from the declaration it is meant to check cannot notice the declaration going away. Closed with the other half, in `card_data_invariants.rs`: a spell whose oracle text says "any target" declares it, and one that declares it says so. The text side is already pinned to Scryfall by `oracle_text_says_what_scryfall_says`, so the two cannot drift together.

- `mtg-engine/src/cards/isd/devils_play.rs:7` — a doc comment that outlived its subject.
  - It said X was `computed as the total mana the player had minus the colored requirement ({R}) when cast normally`, under the heading `Simplified: Since the engine doesn't yet support choosing X at cast time`. The engine does: `ChooseXFunding` puts the choice to the player, taps what they name, and records `x_value` — which is the only thing this card reads, and which `x_cost_funding_flow.rs` tests end to end. The note described a limitation that no longer exists, which is what an auditor reads before the code.

- `mtg-engine/src/cards/isd/devils_play.rs:44` — `if x > 0 { ... } else { }` around the damage.
  - Dead code, and the same one removed from Harvest Pyre two cards ago: CR 120.8 lives in `damage::deal_damage`. The empty `else` was what made it visible.

The card is otherwise right: `{X}{R}`, Sorcery, oracle text verbatim, flashback `{X}{R}{R}{R}`, `AnyTarget` for "any target", and damage through `helpers::resolve_damage` into the one pipeline.

### Tricky interactions checked

- X is announced by the player, not derived: PASS, and tested end to end in `x_cost_funding_flow.rs`.
- The X announced is the X that resolves: PASS, `x_cost_spells.rs:126`.
- The flashback cost's `{X}` uses the same machinery: PASS — the card reads `x_value` and never looks at how the spell was cast.
- "any target" reaches a planeswalker: PASS, `damage_helper.rs:41` — and now cannot silently stop being checked.
- Damage to a planeswalker removes loyalty rather than marking damage: PASS, `damage_helper.rs:113` (Stensia Bloodhall covers the ability path; `test_suite_guards.rs` names Devil's Play as the card that once wrote `damage_marked` on a planeswalker by hand).
- X = 0: the spell resolves and deals nothing, by CR 120.8 in the pipeline. Covered by `damage_pipeline.rs::zero_damage_is_not_damage`, added earlier in this run.
- Mana value is 2 regardless of X: not this card's code — `mana_value` reads the printed cost, pinned by `a_cost_reduction_does_not_change_a_cards_mana_value`.

### Test coverage

- The funding prompt, the tap plan, and `x_value` being set: `x_cost_funding_flow.rs`, six tests
- The announced X is what resolves: `x_cost_spells.rs:126` `the_announced_x_is_the_x_the_spell_resolves_with`
- Damage equals X: `devils_play_deals_as_much_damage_as_x_was_paid_for`
- "any target" offers a planeswalker: `damage_helper.rs:41`
- The requirement matches the printed wording: `card_data_invariants.rs:1946` `any_target_in_the_text_means_any_target_in_the_requirement`, added this audit
- Flashback cost is `{X}{R}{R}{R}`: `card_data_invariants.rs` (added during the Sever the Bloodline audit)

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 deal `x + 1` | 2 tests FAILED | (unchanged) |
| M2 `AnyTarget` -> `Creature` | passed whole workspace | `any_target_in_the_text_means_any_target_in_the_requirement` FAILED |
| M3 drop the `x > 0` guard | passed (dead code) | still passes; the rule lives in `deal_damage`, pinned by `zero_damage_is_not_damage` |

M2 is the finding, and the reason it survived is worth naming: the test that exists for this exact rule could not fail, because it chose its subjects by the same declaration it was checking. That is a shape to watch for — a sweep is only as good as the thing it enumerates from.

Source restored from `/tmp/dp.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1495 passing (was 1494). `cargo check --workspace --all-targets` clean, zero warnings.
