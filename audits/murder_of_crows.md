## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/70/murder-of-crows?utm_source=api
**Type line**: `Creature — Bird` — {3}{U}{U}, 4/4
**Oracle text**:
```
Flying
Whenever another creature dies, you may draw a card. If you do, discard a card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Whenever **another** creature dies" — including an opponent's, and including
  tokens: PASS
- "you **may** draw a card. **If you do**, discard a card" — the discard is
  conditional on the draw, so declining costs nothing and an empty library that
  drew nothing does not force a discard: PASS
- The discard goes through `discard_card`, so it announces itself to discard
  watchers (Civilized Scholar's transform is in this set): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The may-draw and the linked discard: `cards_discard_and_hand.rs`, `simultaneous_events.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/70/murder-of-crows?utm_source=api
**Type line**: `Creature — Bird` — {3}{U}{U}, 4/4
**Oracle text**:
```
Flying
Whenever another creature dies, you may draw a card. If you do, discard a card.
```

**Rulings fetched**:
- [2018-03-16] If another creature dies at the same time as Murder of Crows, its last ability triggers.
- [2018-03-16] You can't do anything in between drawing a card and discarding a card, including casting or cycling the card you drew.

**Status**: PASS

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/70/murder-of-crows
**Oracle text**:
```
Flying
Whenever another creature dies, you may draw a card. If you do, discard a card.
```
**Type line**: `Creature — Bird`
**Mana cost**: `{3}{U}{U}` · **P/T**: 4/4 · **Keywords**: Flying
**Rulings** (2, both 2018-03-16, https://api.scryfall.com/cards/f914f7e4-06fc-4943-8597-b7f834938c00/rulings):
1. "If another creature dies at the same time as Murder of Crows, its last ability triggers."
2. "You can't do anything in between drawing a card and discarding a card, including casting or cycling the
   card you drew."

**Status**: PASS (test coverage extended)

### Card data
| field | oracle | `murder_of_crows.rs` | |
|---|---|---|---|
| cost | `{3}{U}{U}` | `Generic(3) + Blue + Blue` | ok |
| types | Creature | `vec![CardType::Creature]` | ok |
| subtypes | Bird | `vec!["Bird"]` | ok |
| P/T | 4/4 | `Some(4)`/`Some(4)` | ok |
| keywords | Flying | `vec![Keyword::Flying]` | ok |
| oracle_text | as above | byte-identical | ok |
| trigger | "whenever another creature dies" | `TriggerKind::AnyCreatureDies` | ok — see below |

### Code issues
No issues found.

The one thing worth checking closely is **"another"**, since the card declares `AnyCreatureDies` and does not
test `dead_id != self_id` itself. It does not have to: `triggers/collect/zones.rs` filters the watcher list with
`o.id != dead_id`, so `AnyCreatureDies` *means* another creature. The self-exclusion is in the general hook, not
in the card, which is where it belongs — eleven cards share that trigger kind.

### Rules check
- **Ruling 1 (simultaneous death)**: the same collector includes watchers in `simultaneously_dead` — objects
  that left the battlefield in the same event batch — because they were still on the battlefield when the
  deaths happened (CR 603.10a). The Crows dying alongside another creature still see it die. The ability's
  controller then comes from `helpers::controller_of`, i.e. last known information (CR 608.2g), which is what
  makes that case resolve at all.
- **Ruling 2 (nothing happens in between)**: the draw and the discard are one resolution, suspended on
  `AwaitingAction::ResolutionChoice`. `legal_actions_while_awaiting` replaces the ordinary priority-based
  options entirely while one is pending, so no player receives priority between the two.
- **"You may"**: a `YesNo` choice presented to the controller; the draw happens only on "yes".
- **"If you do"**: keyed on `draw_cards(..) == 0`, i.e. on whether a card was actually drawn, not on whether
  the ability resolved. An empty library means no draw and so no discard.
- **CR 120.3 / empty library**: `draw_cards` applies the `DrawsFromEmptyLibrary` replacement (Laboratory
  Maniac) and otherwise leaves the loss to SBAs. Not this card's business, and it does not shortcut it.

### Changes made
Nothing in the card. `mtg-engine/tests/cards_death_triggers_and_tokens.rs` gained five tests plus a shared
setup helper. The existing coverage asserted only that the yes/no prompt appeared and that the draw had not
happened yet — the entire second half of the card was untested.

- `murder_of_crows_draws_and_then_discards_when_you_accept`
- `murder_of_crows_does_nothing_when_you_decline` — without it, an implementation that ignored the answer and
  always drew would pass the accepting test.
- `murder_of_crows_discards_nothing_when_the_draw_found_no_card` — "if you do". The card's own comment records
  this having been wrong once ("checking the hand instead made a player with cards already in hand discard one
  they had never drawn"), and there was no test holding the fix in place.
- `murder_of_crows_gives_nobody_priority_between_the_draw_and_the_discard` — ruling 2, then answers the discard
  so the multi-card path is finished rather than just observed.
- `murder_of_crows_does_not_trigger_on_its_own_death` — "another".

### Mutation checks
1. Discard keyed on the hand rather than on the draw (the recorded old bug) → **vacuous on the first version of
   the empty-library test**; discriminating after it was corrected. See below.
2. `on_yes_no_choice` ignoring `yes` → `murder_of_crows_does_nothing_when_you_decline` FAILED.
3. Dropping `o.id != dead_id` from the death-watch collector → four tests FAILED, including
   `murder_of_crows_does_not_trigger_on_its_own_death`.
4. Skipping the discard entirely → `murder_of_crows_draws_and_then_discards_when_you_accept` FAILED.

**The vacuous one, and what it showed.** My first empty-library test set up two cards in hand and asserted the
hand still held two and the graveyard was empty. Both are true under the mutation — because with two cards in
hand a discard is a *choice*, and a version that discards regardless of the draw does not discard immediately;
it presents a prompt. Hand and graveyard look identical to the correct outcome while that prompt sits pending.
The test now also asserts `awaiting_action.is_none()`, i.e. that the ability is over rather than waiting on a
card to throw away, and it fails the mutation.

### Tricky interactions checked
- Crows die alongside another creature → the draw is still offered: **pass**
  (`trigger_source_independence.rs:581`).
- Crows die alone → nothing offered: **pass** (new).
- Empty library → no draw, no discard, nothing pending: **pass** (new).
- No priority window between draw and discard: **pass** (new).
- Hand of exactly one card after the draw → discarded without a redundant prompt: **pass**, exercised by the
  accepting test, which starts from an empty hand.

### Test coverage
- yes/no prompt appears, draw deferred: `cards_death_triggers_and_tokens.rs:644`
- draws then discards on "yes": `cards_death_triggers_and_tokens.rs:722` (new)
- declining: `cards_death_triggers_and_tokens.rs:740` (new)
- "if you do" with an empty library: `cards_death_triggers_and_tokens.rs:762` (new)
- ruling 2, no priority in between: `cards_death_triggers_and_tokens.rs:786` (new)
- "another" — no trigger on its own death: `cards_death_triggers_and_tokens.rs:822` (new)
- ruling 1, simultaneous death: `trigger_source_independence.rs:581`

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1401 passing.

