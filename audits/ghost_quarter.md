## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/240/ghost-quarter?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
```

**Status**: ISSUE

### Code issues
See below.

- Its ruling is the plain statement of CR 608.2b for abilities, and
  `stack.rs`'s `StackEntry::Ability` arm had no legality check at all.
  - Ruling (2011-09-22) says: `If the targeted land is an illegal target by the
    time Ghost Quarter's ability resolves, it won't resolve and none of its
    effects will happen. The land's controller won't get to search for a basic
    land card.`
  - Code did: resolve against whatever it had targeted, however the board had
    changed
  - Fixed: the arm now substitutes `Target::Illegal` for a target that can no
    longer be targeted or no longer satisfies the card's `is_valid_target`, and
    fizzles when every target is illegal.

- The ability's effect lived in `on_activate_ability`, whose trait default *was*
  the CR 602.2a stack push, so the effect happened the instant the ability was
  activated and no opponent ever received priority.
  - CR 602.2a says: `the ability goes on the stack`
  - Code did: `fn on_activate_ability(&self, ...) { <the effect> }` — overriding
    the push away
  - Fixed set-wide: the hook is gone, the engine owns the push
    (`engine::actions::abilities::put_ability_on_stack`), and the effect moved to
    `resolve_activated_ability`. See
    `reports/ISD_AUDIT_CR6022a_ACTIVATED_ABILITIES.md`.

### Tricky interactions checked
- "Its controller **may** search" — the land's controller is offered the choice,
  and a player who declines does not shuffle: PASS
- The controller is offered the search even with no basic to find (declining is
  still their call): PASS
- The found land goes to the battlefield untapped, not to hand: PASS
- Sacrificing Ghost Quarter is a cost, paid on activation, so it is in the
  graveyard while the ability is on the stack: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- CR 602.2a (the ability waits on the stack): `activated_no_stack.rs:activating_through_the_engine_leaves_the_ability_on_the_stack`
- CR 608.2b (targets re-checked on resolution): `fizzle.rs:an_activated_abilitys_targets_are_rechecked_when_it_resolves`
- Guards: `test_suite_guards.rs:no_card_or_test_names_the_removed_activation_hook`, `test_suite_guards.rs:only_the_engine_puts_an_ability_on_the_stack`
- The "may" choice with no basics: `resolution_time_checks.rs:ghost_quarter_may_choice_offered_when_no_basics`
- Declining does not shuffle: `resolution_time_checks.rs:ghost_quarter_declining_the_search_does_not_shuffle`
- Searching does shuffle: `lands_and_mana.rs:ghost_quarter_shuffles_the_library_after_the_search`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/240/ghost-quarter?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
```

**Rulings fetched**:
- [2013-07-01] The target land's controller gets to search for a basic land card even if that land wasn't destroyed by Ghost Quarter's ability. This may happen because the land has indestructible or because it was regenerated.
- [2011-09-22] If the targeted land is an illegal target by the time Ghost Quarter's ability resolves, it won't resolve and none of its effects will happen. The land's controller won't get to search for a basic land card.
- [2006-05-01] If you target Ghost Quarter with its own ability, the ability won't resolve because its target is no longer on the battlefield. You won't get to search for a land card.

**Status**: ISSUE (1; shared with four other cards, all fixed)

### Code issues found and fixed

**One, shared with four other cards: a log line that announced a destruction
nobody had checked happened.**

- Ruling 2013-07-01 says: `The target land's controller gets to search for a
  basic land card even if that land wasn't destroyed by Ghost Quarter's
  ability. This may happen because the land has indestructible or because it
  was regenerated.`
- Code did:
  ```rust
  crate::destruction::try_destroy(state, *target_id, registry);
  state.log(crate::state::LogLevel::Event,
      format!("Ghost Quarter destroyed {target_name}"));
  ```

The *behaviour* was right — the search is offered either way, which is what the
ruling is about, and the code never made it conditional. The log was wrong:
`try_destroy` returns a `DestroyResult` saying whether the permanent died,
regenerated, or was indestructible, and the line was written without looking.
Against an indestructible land the game log said Ghost Quarter destroyed it.

Five cards did this: Ghost Quarter, Evil Twin, Into the Maw of Hell, Maw of the
Mire, and Witchbane Orb (over `try_destroy_all`). The pipeline already
announces what *happened* — `move_object` writes the death, `regenerate`
writes the regeneration — but neither names the source, which is why each card
wrote its own line. That is the same problem `mill_cards` had and the same
shape of fix: `destruction::try_destroy_by(state, id, source, registry)` names
the source and writes the line the result justifies. Witchbane Orb zips its
names against `try_destroy_all`'s per-permanent results.

New guard: `card_data_invariants.rs::no_card_announces_a_destruction_it_did_not_check`.

### A test-harness trap found on the way

My first version of the "the land survives" test granted indestructible with
`state.get_object_mut(victim).unwrap().keywords.push(Keyword::Indestructible)`
— and the Forest died anyway. That is not an engine bug: `has_keyword`
deliberately does *not* consult `obj.keywords` for a card with a registry
entry, because keywords have an effects layer and unioning the vector in would
resurrect a stale front-face keyword on a transformed DFC. Its comment says so.

The trap is that the same line *works* on an anonymous creature from
`ready_creature` (`CardId(9999)`, no registry entry, so the vector really is
its printed keywords) — which is how it is used correctly elsewhere in the
suite. On a real card it is silently ignored, and a test written that way
passes for the wrong reason. `common::grant_keyword` now does it through
`TemporaryEffect::GrantKeyword`, with the distinction written down.

(I checked the two tests I wrote earlier in this run that push keywords —
Tribute to Hunger's indestructible and hexproof cases — they use
`ready_creature`, so they are in the working half, and the indestructible one
was already mutation-checked against `try_destroy`.)

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | none | no `cost` OK |
| type | `Land` | `[CardType::Land]` OK |
| mana ability | `{T}: Add {C}.` | `ManaAbilityDef` producing one Colorless, `requires_tap` OK |
| ability | `{T}, Sacrifice this land: Destroy target land...` | `requires_tap` + `SacrificeCost::SacrificeThis`, `PermanentWithFilter(HasCardType([Land]))` OK |
| oracle text | verbatim match | OK |

### Tricky interactions checked

- **Ruling 2013-07-01, the search happens even if the land survives.**
  **Pass** — the destruction's result never gated the search. Was untested;
  now tested against both an indestructible land and a regenerating one, and
  making the search conditional on `Died` fails it.
- **Ruling 2011-09-22, an illegal target means none of the effects happen.**
  **Pass** — the engine's CR 608.2b re-check fizzles the ability before the
  handler runs.
- **Ruling 2006-05-01, targeting Ghost Quarter with its own ability.**
  **Pass** — the sacrifice is a cost, so the Quarter is in the graveyard by
  resolution and is no longer a legal target. Was untested; now is.
- **"Its controller"** is the *target's* controller, not the Quarter's.
  **Pass**.
- **"a basic land card"** — filtered on card types plus the Basic supertype,
  read from the face. **Pass.**
- **"put it onto the battlefield"** — untapped, not into hand. **Pass.**
- **"...then shuffle"**, and a player who declines does not shuffle.
  **Pass**, tested across twenty runs rather than against one forbidden order.
- **A controller with no basic lands is still offered the search**, because
  declining is their choice. **Pass**, handled by `search_library`.

### Test coverage

- destroys the land, finds a basic, and shuffles:
  `lands_and_mana.rs::ghost_quarter_shuffles_the_library_after_the_search`
- the ability goes on the stack rather than resolving on activation:
  same helper, asserted in its comments
- it can target a non-token land: `characteristics_targeting.rs:17`
- every basic in the library is offered, not just the first: `auto_pick.rs:596`
- **the search is offered even when the land survives**:
  `lands_and_mana.rs::ghost_quarter_offers_the_search_even_when_the_land_survives` (new),
  which also asserts the log does not claim a destruction that did not happen
- **targeting itself does nothing**:
  `lands_and_mana.rs::ghost_quarter_targeting_itself_does_nothing` (new)

Mutation-checked: logging "destroyed" regardless of the result fails the
survives test, and gating the search on `DestroyResult::Died` fails it too.
