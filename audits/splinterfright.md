## Audit — 2026-08-27 — CR 603.2 trigger scope

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/205/splinterfright?utm_source=api
**Type line**: `Creature — Elemental` — {2}{G}, */*
**Oracle text**:
```
Trample
Splinterfright's power and toughness are each equal to the number of creature cards in your graveyard.
At the beginning of your upkeep, mill two cards. (Put the top two cards of your library into your graveyard.)
```

**Status**: ISSUE (fixed) — duplication, not a rules defect

### Code issue
- Oracle text says the trigger happens at **your** upkeep / **your** end step.
- Code did: declared `step_trigger_scope` → `TriggerScope::Your`, which is
  correct and sufficient, and then re-derived the same thing inside the handler
  as `state.active_player != controller`.
- The engine's gate is not taken on trust: `your_upkeep_scope.rs` sweeps the
  registry for every card with a controller-scoped step trigger and checks both
  directions — fires on the controller's step, silent on the opponent's. The
  handler check was provably dead.
- Fixed: removed, with a comment saying where the scoping actually lives.


### What else was checked
- Card data verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`): cost, types,
  subtypes, supertypes, P/T, oracle text, keywords on both faces, flashback
  cost, and trigger kinds against the oracle phrasing.
- Step 9 anti-patterns: clean after this change.

### Test coverage
`your_upkeep_scope.rs::a_your_step_trigger_fires_on_its_controllers_step_and_no_one_elses`
covers this card by registry sweep, in both directions.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/205/splinterfright?utm_source=api
**Type line**: `Creature — Elemental` — {2}{G}, */*
**Oracle text**:
```
Trample
Splinterfright's power and toughness are each equal to the number of creature cards in your graveyard.
At the beginning of your upkeep, mill two cards. (Put the top two cards of your library into your graveyard.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The ability that defines Splinterfright's power and toughness works
  **in all zones**, not just the battlefield. **If Splinterfright is in your
  graveyard, it will count itself.**" `dynamic_pt` has no self-exclusion and no
  battlefield gate, and Splinterfright's own card data carries the `Some(0)`
  P/T sentinel that marks a characteristic-defining creature — so it is counted
  by `is_creature` when it is in the graveyard: PASS
- CR 112.8: a card in a graveyard is controlled by its **owner**, so the count
  reads `obj.owner` rather than a `controller` left stale by a steal effect:
  PASS
- CR 109.1: "creature **cards** in your graveyard", so tokens are excluded: PASS
- Ruling: "If Splinterfright's controller has only one card in their library
  when its triggered ability resolves, they put that card into their graveyard"
  — `mill_cards` stops at an empty library: PASS
- The self-mill grows it, since a milled creature card is another creature card
  in the graveyard: PASS
- "At the beginning of **your** upkeep": PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The CDA and the token exclusion: `token_is_not_a_card.rs:a_token_in_a_graveyard_is_not_a_creature_card`, `:cda_does_not_count_tokens_in_graveyard`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/205/splinterfright?utm_source=api
**Type line**: `Creature — Elemental` — {2}{G}, */*
**Oracle text**:
```
Trample
Splinterfright's power and toughness are each equal to the number of creature cards in your graveyard.
At the beginning of your upkeep, mill two cards. (Put the top two cards of your library into your graveyard.)
```

**Rulings fetched**:
- [2025-01-24] The ability that defines Splinterfright’s power and toughness works in all zones, not just the battlefield. If Splinterfright is in your graveyard, it will count itself.
- [2025-01-24] If Splinterfright’s controller has only one card in their library when its triggered ability resolves, they put that card into their graveyard.
- [2025-01-24] The ability that defines Splinterfright’s power and toughness works in all zones, not just the battlefield. If Splinterfright is in your graveyard, it will count itself.
- [2025-01-24] If Splinterfright’s controller has only one card in their library when its triggered ability resolves, they put that card into their graveyard.

**Status**: ISSUE (1, fixed; one test was enshrining it)

**Note on the rulings**: Scryfall returns four, which are the same two
duplicated. Both are addressed below.

### Code issues found and fixed

**One: "your graveyard" was read as the owner's, not the controller's.**

- Oracle text says: `Splinterfright's power and toughness are each equal to the
  number of creature cards in your graveyard.`
- Code did:
  ```rust
  // CR 112.8: a card in a graveyard is controlled by its owner, and
  // `objects_in_zone` filters graveyards by owner — so reading a stale
  // `controller` left over from a steal effect would count the
  // opponent's graveyard instead of this card's owner's.
  let owner = state.get_object(object_id)?.owner;
  ```

CR 109.5: "The words 'you' and 'your' on an object refer to the object's
controller, its would-be controller..., or its owner (if it has no
controller)" — and, for a static ability, that is *the current controller of
the object it's on*. A characteristic-defining ability is a static ability
(CR 604.3), so a stolen Splinterfright is the size of the **thief's**
graveyard. Traitorous Blood is in this set.

Confirmed before changing anything: with one creature card in its owner's
graveyard and three in an opponent's, taking control of Splinterfright left it
1/1 rather than 3/3.

The comment's premise was wrong twice over. A steal on the battlefield does not
leave a "stale" controller — it sets a real one. And the field it was avoiding
is right in *both* zones, because CR 108.4 gives a card off the battlefield no
controller and `move_object` resets `controller` to `owner` on the way out. So
one read of `controller` covers the battlefield case and the graveyard case,
where reading `owner` was only ever right in one of them.

`objects_in_zone(Zone::Graveyard, player)` filters by owner, which is correct:
"your graveyard" is the graveyard whose cards you own, and the player it is
asked about is the one "you" resolves to.

**A test enshrined the wrong behaviour.**
`enters_under_control.rs::splinterfright_counts_its_owners_graveyard` set up a
steal, called it "an Act-of-Treason style steal leaves a stale controller
behind", and asserted the size *must not change*. Replaced with two tests that
assert the rule in both directions.

**Stale CR citation.** `112.8` appeared in the card and in that test. Rule 112
in the current Comprehensive Rules is *Spells*; the rule about a card off the
battlefield having no controller is CR 108.4, which is what `state.rs` itself
cites. Both corrected. (Reaper from the Abyss had the same pre-renumbering
number in this audit run; these were the last two.)

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{2}{G}` | `Generic(2), Colored(Green)` OK |
| type | `Creature - Elemental` | `Creature`, `["Elemental"]` OK |
| P/T | `*/*` | `Some(0)/Some(0)` as the CDA sentinel, with `dynamic_pt` defining it OK |
| keywords | Trample, Mill | `vec![Keyword::Trample]` OK - "mill" is a keyword *action* (CR 701.13), not a keyword ability, and no card in the set declares it |
| oracle text | verbatim, reminder text included | OK |
| trigger | "At the beginning of your upkeep" | `TriggerKind::Upkeep` with `TriggerScope::Your` OK, and cross-checked against the printed text by `your_upkeep_scope.rs::a_step_triggers_scope_is_the_one_its_oracle_text_states` |

### Tricky interactions checked

- **Ruling: "The ability... works in all zones, not just the battlefield. If
  Splinterfright is in your graveyard, it will count itself."** **Pass** —
  `effective_power` has no zone gate, and a Splinterfright in the graveyard is
  one of the creature cards its own CDA counts. Was untested; now is, including
  that its controller has been reset to its owner by then.
- **Ruling: "If Splinterfright's controller has only one card in their library
  when its triggered ability resolves, they put that card into their
  graveyard."** **Pass** — `mill_cards` mills what is there and says so.
  Untested; now tested, along with the empty-library case. Removing the
  emptiness guard from `mill_cards` makes both new tests panic on an
  out-of-bounds index, which is what they are really protecting.
- **Milling an empty library is not a loss** (CR 701.13b — the loss comes from
  *drawing* from an empty library). **Pass**, now asserted.
- **"creature cards"** — a token in the graveyard is not a card (CR 109.1) and
  does not count. **Pass**, tested in `token_is_not_a_card.rs`.
- **A token copy of Splinterfright** has the CDA and counts its controller's
  graveyard. **Pass**, tested in `token_copy.rs`.
- **The mill happens after Splinterfright is destroyed in response**
  (CR 113.7a). **Pass**, tested in `trigger_source_independence.rs`.
- **Trample.** Declared as a keyword; the engine implements it.
- **Only on your upkeep, not each.** **Pass** — `TriggerScope::Your`, and the
  text-derived invariant added earlier in this run checks that against the
  printed "your".

### Test coverage

- mills 2 on upkeep:
  `cards_upkeep_triggers_and_curses.rs::splinterfright_mills_on_upkeep`
- the mill still happens if it is destroyed in response:
  `trigger_source_independence.rs::splinterfright_mills_after_dying`
- tokens in the graveyard do not count: `token_is_not_a_card.rs:101`
- a token copy has the CDA: `token_copy.rs:48`
- **"your graveyard" follows the controller**:
  `enters_under_control.rs::splinterfright_is_the_size_of_its_controllers_graveyard`
  (rewritten from a test asserting the opposite)
- **the ability works in the graveyard and counts itself**:
  `enters_under_control.rs::splinterfright_in_a_graveyard_counts_itself` (new)
- **a one-card library, and an empty one**:
  `cards_upkeep_triggers_and_curses.rs::splinterfright_mills_what_is_left_of_a_short_library`,
  `::splinterfright_against_an_empty_library_does_nothing` (new)

Mutation-checked: reading `owner` again fails the controller test, and removing
`mill_cards`'s empty-library guard makes both short-library tests panic.
