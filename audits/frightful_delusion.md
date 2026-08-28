## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/57/frightful-delusion?utm_source=api
**Type line**: `Instant` — {2}{U}
**Oracle text**:
```
Counter target spell unless its controller pays {1}. That player discards a card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "You must target a spell in order to cast Frightful Delusion. You
  can't cast it without a legal target just to make a player discard a card":
  PASS
- CR 608.2g: the controller **may tap for the {1}** — having nothing floating is
  not the same as being unable to pay, which is the bug this card's tests were
  written for: PASS
- "That player discards a card" happens whether or not the spell was countered:
  PASS
- Countering uses `move_countered_spell` (CR 701.5a), not the resolving-spell
  cleanup path: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The pay-or-counter choice and the unconditional discard: `resolution_time_checks.rs:auto_counter_when_controller_has_no_floating_mana_but_has_lands`, `:player_offered_choice_when_controller_has_floating_mana`, `:claiming_to_pay_without_the_mana_does_not_save_the_spell`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/57/frightful-delusion?utm_source=api
**Type line**: `Instant` — {2}{U}
**Oracle text**:
```
Counter target spell unless its controller pays {1}. That player discards a card.
```

**Rulings fetched**:
- [2011-09-22] You must target a spell in order to cast Frightful Delusion. You can’t cast it without a legal target just to make a player discard a card.
- [2011-09-22] The player discards a card even if they pay {1}.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/57/frightful-delusion
**Oracle text**:
```
Counter target spell unless its controller pays {1}. That player discards a card.
```
**Type line**: `Instant` · **Mana cost**: `{2}{U}`
**Rulings** (2, both 2011-09-22, https://api.scryfall.com/cards/38c9ba98-90b4-4c28-9eef-a4fe0913b921/rulings):
1. "You must target a spell in order to cast Frightful Delusion. You can't cast it without a legal target just
   to make a player discard a card."
2. "The player discards a card even if they pay {1}."

**Status**: ISSUE (fixed) — one card's second sentence was living in the engine.

### Card data
| field | oracle | `frightful_delusion.rs` | |
|---|---|---|---|
| cost | `{2}{U}` | `Generic(2) + Blue` | ok |
| types | Instant | `vec![CardType::Instant]` | ok |
| oracle_text | as above | byte-identical | ok |
| targeting | "target spell" | `TargetRequirement::Spell` + `helpers::spell_target_is_legal` | ok |

### Code issues

**The engine's `PayOrNot` handler carried Frightful Delusion's discard.** Fixed.

`engine/actions/choices.rs` did two things when the choice was answered:

1. Settle the payment and, if unpaid, counter the spell. This half is general — the choice kind carries
   `spell_id` precisely so the engine can counter it, and CR 608.2g's "may they pay?" is engine business.
2. `// Controller discards a card — player chooses which.` — and then it did, unconditionally, for whatever
   card had raised the choice. That is the card's second sentence, and nothing else's.

Nothing is presently wrong, and I am not claiming it was: Frightful Delusion is the only card in the set that
raises a `PayOrNot`, so the handler only ever ran its own rider. But "counter unless its controller pays" is a
template, and the next card to use it would have inherited this discard on top of whatever it says itself.

The codebase already has the shape for this — `YesNo` dispatches to `behavior.on_yes_no_choice`, which is how
Murder of Crows runs its draw-and-discard. `PayOrNot` now dispatches the same way.

**A fragile re-read, tidied.** The handler read the payer twice:

```rust
let controller = state.get_object(*spell_id).map_or(PlayerId(0), |o| o.controller);   // before
...
crate::cards::helpers::counter_spell(state, *spell_id, registry);
let controller = state.get_object(*spell_id).map_or(PlayerId(0), |o| o.controller);   // after
```

CR 108.4: a card that is no longer a spell has no controller. I checked whether this was a live bug and it was
**not** — `move_object` resets `controller` to `owner` only when `from == Zone::Battlefield`, and a countered
spell leaves the stack, so the second read returns the same player. The hook takes `payer` as a parameter now,
so there is no second read to reason about.

### Rules check
- **Ruling 1**: `TargetRequirement::Spell` with an empty stack enumerates nothing, and `generate_cast_actions`
  produces no action. The discard is not a reason to cast it.
- **Ruling 2**: the discard is outside the paid/unpaid branch and ignores `paid`. Now stated as such in the
  card, with the ruling quoted.
- **CR 608.2g**: the choice is offered even with an empty pool, because paying may involve tapping;
  `pay_cost_with_sources` decides. Declining and failing to pay are the same outcome.
- **CR 701.5a**: the counter goes through `helpers::counter_spell` (shared pipeline, added in the Lost in the
  Mist audit), so a flashback spell is exiled rather than put in the graveyard.
- **"That player"** is the countered spell's controller, not the caster of Frightful Delusion.

### Changes made
- `mtg-engine/src/cards/mod.rs` — `CardBehavior::on_pay_decision(state, self_id, payer, paid, registry)`.
- `mtg-engine/src/engine/actions/choices.rs` — keeps the payment and the counter; dispatches the rest to the
  card; returns `ReturnNow` if the card left a choice pending (preserving the old control flow).
- `mtg-engine/src/cards/isd/frightful_delusion.rs` — implements `on_pay_decision` with the discard.
- `mtg-engine/tests/cards_morbid_and_ltb.rs` — three tests, below.

### Test coverage was missing three paths
- **Ruling 1** — not tested at all. Added `frightful_delusion_cannot_be_cast_just_for_the_discard`, with a
  control that puts a spell on the stack so "uncastable" is about the missing target and not about mana.
- **The discard when the player holds more than one card** — not tested. Both existing discard tests hand the
  player exactly one card, which takes the branch that discards without asking. The asking branch is precisely
  the code that moved out of the engine in this audit, so it was untested code being relocated.
- **An empty hand** — not tested: nothing discarded, and nothing left pending.

### Mutation checks (all discriminating)
1. Discard only when the payment was declined (violating ruling 2) → `frightful_delusion_discard_on_pay` FAILED.
2. Card picks the discard instead of asking (`hand.len() == 1` → `!hand.is_empty()`) →
   `frightful_delusion_lets_the_player_choose_which_card_to_discard` FAILED.
3. Engine never dispatches `on_pay_decision` → two tests FAILED across both files.
4. `TargetRequirement::Spell` enumeration made to offer a target with an empty stack →
   `frightful_delusion_cannot_be_cast_just_for_the_discard` FAILED.

### Tricky interactions checked
- Pays → spell survives, discard still happens: **pass** (`frightful_delusion_discard_on_pay`).
- Declines → countered, discard happens: **pass** (`frightful_delusion_counters_and_discards`).
- Empty pool → still asked, only "don't pay" offered: **pass**
  (`frightful_delusion_offers_the_choice_even_with_an_empty_pool`).
- Two cards in hand → the player chooses: **pass** (new).
- Empty hand → nothing pending: **pass** (new).
- Uncastable with nothing on the stack: **pass** (new).
- Target leaves the stack before resolution → `on_resolve`'s `zone == Stack` guard skips everything, including
  the discard. Correct: with the only target illegal the spell is countered by game rules (CR 608.2b) and
  neither sentence happens. Covered generally by `fizzle.rs`.

### Test coverage
- counters and discards on decline: `cards_removal_and_bounce.rs:76`
- discards even when paid (ruling 2): `cards_morbid_and_ltb.rs:1372`
- choice offered when the opponent has mana: `cards_morbid_and_ltb.rs:831`
- choice offered with an empty pool: `cards_morbid_and_ltb.rs:867`
- uncastable without a spell target (ruling 1): `cards_morbid_and_ltb.rs:903` (new)
- player chooses which card to discard: `cards_morbid_and_ltb.rs:936` (new)
- empty hand asks nothing: `cards_morbid_and_ltb.rs:981` (new)

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1404 passing.

