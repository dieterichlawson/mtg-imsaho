## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/10/divine-reckoning?utm_source=api
**Type line**: `Sorcery` — {2}{W}{W}
**Oracle text**:
```
Each player chooses a creature they control. Destroy the rest.
Flashback {5}{W}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**Each player** chooses a creature they control. Destroy the rest." — every
  player chooses, in turn order, and nothing is destroyed until all have chosen:
  PASS
- A player with no creatures chooses nothing and loses nothing: PASS
- The choices are collected through a chained `resolve_card_effect` that encodes
  who has already chosen, and the spell stays on the stack throughout — this is
  the card the CR 608.2m rule was written for, that reaching the graveyard is
  the *final* step of resolution: PASS
- `try_destroy_all`, so the rest die simultaneously and indestructible survives:
  PASS
- Flashback {5}{W}{W}: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The per-player choice chain and the simultaneous destruction: `spell_cleanup.rs`, `cards_flashback.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/10/divine-reckoning?utm_source=api
**Type line**: `Sorcery` — {2}{W}{W}
**Oracle text**:
```
Each player chooses a creature they control. Destroy the rest.
Flashback {5}{W}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Rulings fetched**:
- [2021-03-19] To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.
- [2021-03-19] "Flashback [cost]" means "You may cast this card from your graveyard by paying [cost] rather than paying its mana cost" and "If the flashback cost was paid, exile this card instead of putting it anywhere else any time it would leave the stack."
- [2021-03-19] You must still follow any timing restrictions and permissions, including those based on the card's type. For instance, you can cast a sorcery using flashback only when you could normally cast a sorcery.
- [2021-03-19] If a card with flashback is put into your graveyard during your turn, you can cast it if it's legal to do so before any other player can take any actions.
- [2021-03-19] A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.
- [2021-03-19] You can cast a spell using flashback even if it was somehow put into your graveyard without having been cast.
- [2011-09-22] Starting with the player whose turn it is, each player chooses a creature in turn order. Players will know the choice of each player who chose before them.

**Status**: ISSUE (fixed)

### Code issues

**The list of creatures a player picks from had no stable order.**

- Oracle text says: `Each player chooses a creature they control. Destroy the rest.`
- Code did:
  ```rust
  let options: Vec<Target> = state.objects.values()
      .filter(|o| o.zone == Zone::Battlefield
          && o.controller == player
          && state.is_creature(o.id, registry))
      .map(|o| Target::Object(o.id))
      .collect();
  ```

`state.objects` is a `HashMap`, so `.values()` yields in arbitrary order. The
player picks from this list by position, so the same game replays differently
run to run, and a recorded decision means something else on the way back. The
codebase already cares about this — `GameState::objects_in_zone` sorts by id
explicitly — but this card built its list by hand and missed it.

**The same bug in the shared helpers, which is where it actually matters.**

Having found it here, I swept for it. `cards/helpers.rs` builds *every* card's
target and choice list the same way: `creature_targets`,
`creature_targets_except`, `creature_choices_except`, `creatures_controlled_by`,
`any_targets` and `any_targets_except` all iterate `state.objects.values()`
unordered. So this was not a Divine Reckoning bug at all — it was every card
that offers a choice.

Added a `stable()` ordering applied in all six: objects by id, players after
objects and by id among themselves. One place, every caller.

Divine Reckoning's own list now goes through `objects_in_zone` (sorted, and
already filtered by controller), and the doomed list is sorted before
destruction — that one is cosmetic, since the destruction is simultaneous, but
it makes the log reproducible.

### Rulings checked

- **"Starting with the player whose turn it is, each player chooses a creature
  in turn order. Players will know the choice of each player who chose before
  them."** `on_resolve` rotates the player list to start at the active player
  and walks it one at a time, logging each choice as it is made — so a later
  chooser has the earlier choices in the log. This is genuinely sequential, not
  a simultaneous collection like Liliana's discard, and the difference is
  visible in the rulings for the two cards. PASS, tested.
- **Flashback rulings** (exiled afterwards, sorcery timing, cost computation)
  are the shared flashback machinery. The cost `{5}{W}{W}` is declared. PASS.

### Tricky interactions checked

- **"chooses" is not "targets"** (CR 115.1) — the options list applies no
  `can_be_targeted_by`, so a hexproof creature can and must be chosen. PASS.
- **"Destroy the rest" is one event** (CR 700.2c). `try_destroy_all`, with a
  comment naming the case that makes it matter: an Angelic Overseer and the last
  Human its controller has are both doomed, and the Overseer survives because
  that Human is still on the battlefield at the moment destruction happens.
  Destroying one at a time would get that wrong. PASS, and covered in
  `simultaneous_events.rs`.
- **A player with exactly one creature** keeps it with no prompt — the choice is
  determined. A player with none is skipped. PASS, tested.
- **The chain's intermediate state** round-trips through the `CardEffect` key
  rather than an engine enum variant, which is the deliberate design: the shape
  of a card's half-finished resolution is the card's business. PASS.
- **The spell does not clean itself up** — the comment says so, and the engine
  finishes the resolution once the choice chain empties (CR 608.2m). PASS.
- **Creatures entering or leaving mid-chain** — not reachable, since no player
  receives priority inside a resolution.

### Test coverage

- turn order, active player first, both players choosing: `cards_sacrifice_and_additional_costs.rs:225`.
- a single creature is kept without a prompt: `::divine_reckoning_with_one_creature_keeps_it`.
- the choice list is stably ordered and controller-filtered: `::divine_reckonings_choice_list_is_in_a_stable_order` (new).
- simultaneous destruction: `simultaneous_events.rs:100`.

