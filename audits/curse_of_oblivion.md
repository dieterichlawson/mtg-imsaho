## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/95/curse-of-oblivion?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {3}{B}
**Oracle text**:
```
Enchant player
At the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard.
```

**Status**: ISSUE

### Code issues
See below.


- It counted tokens as cards.
  - Oracle text says: `that player exiles two cards from their graveyard`
  - Code did: `state.objects_in_zone(Zone::Graveyard, cursed_player).iter().map(|o| Target::Object(o.id))` — no `is_card`
  - CR 109.1: a token is not a card, and CR 704.5e leaves one in a graveyard
    until the next state-based-action pass, so the choice list built
    mid-resolution could offer one. Both the first choice and the chained second
    now ask `state.is_card`.

### Tricky interactions checked
- "At the beginning of **enchanted player's** upkeep" — CR 603.2: the trigger
  event is that player's upkeep beginning, so `TriggerScope::AttachedPlayer`
  keeps it off the stack during anyone else's: PASS
- CR 113.7a: destroying the Curse in response does not counter its trigger, and
  `attached_player` still knows whom it cursed: PASS
- "**that player** exiles" — the cursed player chooses which cards, not the
  Curse's controller: PASS
- Two or fewer cards are exiled outright with no choice to present: PASS
- Enchant **player**, so `TargetRequirement::PlayerOnly`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The upkeep scope and the choice chain: `curse_and_equip_scope.rs`, `cards_auras.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/95/curse-of-oblivion?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {3}{B}
**Oracle text**:
```
Enchant player
At the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard.
```

**Rulings fetched**:
- [2011-09-22] If the enchanted player has only one card in their graveyard, they exile that card.

**Status**: PASS (coverage added)

### Code issues

No behavioural issues found. The card is correct, including the branch that had
no test at all.

One tidy: `resolve_card_effect` took its registry as `_registry` and then used
it, which reads as "unused" to anyone skimming. Renamed.

### Rulings checked

- **"If the enchanted player has only one card in their graveyard, they exile
  that card."** The `<= 2` branch exiles whatever is there rather than requiring
  two, so one card is exiled and none is left behind (CR 608.2 — do as much as
  possible). PASS. The existing test used *two* cards, which does not actually
  exercise the ruling; a one-card test is added.

### Tricky interactions checked

- **The two-exile countdown.** With three or more cards the effect chains two
  prompts, carrying the remaining count in the `PendingEffect::CardEffect` key:
  `on_upkeep` seeds `"1"`, the first resolution exiles one and re-prompts with
  `"0"`, the second exiles and stops. Exactly two. This is the one place an
  off-by-one would exile one card or three, and it had no coverage — the only
  test took the `<= 2` auto path. Now tested end to end through
  `submit_action`, and mutation-checked by seeding the counter at `"0"`, which
  makes it exile one.
- **The cursed player chooses, not the Curse's controller.** The prompt is
  raised with `player: cursed_player`. Asserted explicitly in the new test,
  since a curse whose controller picked would be a meaningfully different card.
  PASS.
- **"At the beginning of *enchanted player's* upkeep"** — `TriggerScope::AttachedPlayer`
  (CR 603.2), so it does nothing on the controller's own upkeep. PASS, and
  covered by the shared `a_curse_does_nothing_on_its_controllers_upkeep` test.
- **The Curse destroyed in response** does not stop the exile: the handler never
  looks at the Curse's zone, and `attached_player` falls back to
  `last_attached_to_player` (CR 113.7a, 608.2g). PASS.
- **"exiles two **cards**"** — `is_card` filters out a token still sitting in the
  graveyard before the next state-based check (CR 109.1). PASS.
- **An empty graveyard** exits before prompting rather than offering an empty
  choice. PASS, now tested.
- **A graveyard that empties between the two prompts** — not reachable, since no
  player receives priority inside a resolution, but the second prompt is guarded
  by an `is_empty` check anyway. PASS.
- **Whose graveyard.** The chained prompt re-derives the player from the exiled
  card's `owner`. A card in a graveyard is always in its owner's graveyard, so
  this is the cursed player. PASS.
- **Enchant player** is `TargetRequirement::PlayerOnly` and resolution goes
  through the shared `resolve_curse`. PASS.

### Test coverage

- the ruling, one card in the graveyard: `cards_upkeep_triggers_and_curses.rs::curse_of_oblivion_exiles_the_only_card_when_there_is_just_one` (new).
- empty graveyard, no prompt: `::curse_of_oblivion_does_nothing_with_an_empty_graveyard` (new).
- three or more cards, exactly two exiled, cursed player choosing: `::curse_of_oblivion_lets_the_cursed_player_choose_exactly_two_of_several` (new, mutation-checked).
- two cards auto-exiled: `::curse_of_oblivion_exiles_from_graveyard`.
- does nothing on the controller's upkeep: `::a_curse_does_nothing_on_its_controllers_upkeep`.
- curse scope invariants: `curse_and_equip_scope.rs:22`.

