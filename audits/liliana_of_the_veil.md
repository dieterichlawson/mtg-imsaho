## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/105/liliana-of-the-veil?utm_source=api
**Type line**: `Legendary Planeswalker — Liliana` — {1}{B}{B}
**Oracle text**:
```
+1: Each player discards a card.
−2: Target player sacrifices a creature.
−6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "When Liliana's first ability resolves, first the player whose turn it
  is chooses a card in hand without revealing it, then each other player in turn
  order does the same. **Then all the chosen cards are discarded at the same
  time.**" The choices are queued and collected before anything leaves a hand,
  so a discard trigger (Murder of Crows is in this set) cannot fire while another
  player is still choosing: PASS
- Ruling: "You can activate Liliana's first ability even if some or all players
  will be unable to discard a card" — a player with an empty hand is skipped
  rather than blocking the ability: PASS
- "−2: **Target player** sacrifices a creature" — the targeted player chooses
  which, and sacrifice bypasses indestructible: PASS
- Ruling: "A pile can be empty. If the player chooses an empty pile, no
  permanents will be sacrificed": PASS
- Starting loyalty 3, and the −6 is not activatable below six: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The simultaneous discard: `cards_planeswalkers.rs`, `simultaneous_events.rs`
- The −2 sacrifice choice: `sacrifice_choice.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/105/liliana-of-the-veil?utm_source=api
**Type line**: `Legendary Planeswalker — Liliana` — {1}{B}{B}
**Oracle text**:
```
+1: Each player discards a card.
−2: Target player sacrifices a creature.
−6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.
```

**Rulings fetched**:
- [2022-09-09] You can activate Liliana’s first ability even if some or all players will be unable to discard a card.
- [2022-09-09] When Liliana’s first ability resolves, first the player whose turn it is chooses a card in hand without revealing it, then each other player in turn order does the same. Then all the chosen cards are discarded at the same time.
- [2022-09-09] When Liliana’s third ability resolves, you put each permanent the player controls into one of the two piles. For example, you could put a creature into one pile and an Aura enchanting that creature into the other pile.
- [2022-09-09] A pile can be empty. If the player chooses an empty pile, no permanents will be sacrificed.

**Status**: ISSUE (fixed)

### Code issues

The card itself is correct on all four rulings. Both findings are in the engine
code it runs through.

**1. The engine wrote Liliana's name into its own generic handlers.**

`engine/actions/choices.rs` — the shared resolution-choice handler — hardcoded
`"Liliana -6:"` in four places: two log lines, one prompt description, and one
sacrifice log. The pile-division handler is generic; the label was not. Any
second card that divided permanents into piles would have been logged as
Liliana. Three more cards had leaked the same way: `"Frightful Delusion: choose
a card to discard"`, `"Creeping Renaissance: chose {type}..."`, and `"Nevermore
names ..."`.

Each of those handlers already has the source object. The label now comes from
`state.obj_name(source_id)`.

There *is* a guard for this — `engine_knows_no_cards.rs` — and it missed all of
it, for two reasons, both fixed:

- `engine_sources()` read only the top level of `src/`, so the entire
  `src/engine/` subtree, where the engine now lives, was never scanned.
- It only looked for `get_id_by_name("Literal")`. A card name sitting in a log
  line or a prompt is the same dependency one step further along — the engine no
  longer *looks up* those cards, it still knows their names.

Added `engine_does_not_name_cards_in_the_text_it_shows`, checking every engine
string literal against the registry's names — plus, for legendary permanents,
the short name they actually get called by, since the offending string said
"Liliana", never "Liliana of the Veil". Mutation-checked against the original
line.

**2. Five production sites rebuilt the whole card registry mid-game.**

Found on Liliana's own `-2` path: `helpers::present_target_choice`'s
single-target fast path called `CardRegistry::with_all_cards()` — constructing
all 249 card behaviours inside a resolution — and then applied the effect
through *that* registry rather than the caller's. Four more sites did the same:
`engine/targeting.rs` in three functions, and Bloodline Keeper's Vampire count.

Besides the cost, it is a correctness hazard: a caller running with a registry
of its own (the fixtures in `player_protection.rs` register an extra card) had
it silently swapped for the default. The registry is now threaded through
`detect_modal_choice_mode`, `generate_cast_actions_with_targets`,
`build_cast_target_spec`, `present_target_choice`,
`present_optional_target_choice` and their callers.

Guarded: `test_suite_guards.rs::nothing_rebuilds_the_card_registry_at_run_time`.

### Rulings checked

- **"You can activate Liliana's first ability even if some or all players will
  be unable to discard a card."** The `+1` declares no target requirement and is
  offered whenever the loyalty cost is payable; nothing gates it on hand sizes.
  PASS.
- **"First the player whose turn it is chooses a card in hand without revealing
  it, then each other player in turn order does the same. Then all the chosen
  cards are discarded at the same time."** This is the whole point of the
  queue in `card_state`: `advance` asks one player at a time, starting with the
  active player and then the rest in turn order (CR 101.4), pushing each choice
  onto a `chosen` list with `discard_immediately: false`. Only once the queue
  empties does it loop over `chosen` and discard. So a discard-triggered ability
  cannot fire while another player is still choosing, and no player sees
  another's choice before making their own. PASS — and this is the part of the
  card most likely to be written wrong, so it is worth saying it is right.
- **"You put each permanent the player controls into one of the two piles."**
  `DividePermanentsIntoPiles` is handed every permanent the target controls, and
  the divider is Liliana's controller. Nothing keeps an Aura with the creature it
  enchants — they can be split, as the ruling's own example requires. PASS.
- **"A pile can be empty. If the player chooses an empty pile, no permanents
  will be sacrificed."** `ChosenSubset` may be empty, giving pile 1 empty and
  pile 2 everything; the `ChoosePile` handler then iterates an empty slice and
  sacrifices nothing. Both prompts render an empty pile as "empty" rather than
  omitting it, so the choice is actually offerable. PASS.

### Tricky interactions checked

- **Who chooses on the −2.** "Target player sacrifices a creature" — that player
  chooses which. `present_target_choice(state, self_id, target_player, ...)`
  passes the *target* as the chooser, not Liliana's controller. PASS.
- **−2 into an empty board.** A player with no creatures is still a legal target
  (the ability targets a player, not a creature); it resolves and does nothing.
  PASS.
- **−2 cannot be fizzled by sacrificing the creature in response**, because the
  creature is never targeted — only the player is. PASS.
- **A player's hand emptying mid-resolution.** `advance` re-reads each player's
  hand when it reaches them, and skips a player whose hand has emptied since the
  ability started. PASS.
- **A player with exactly one card** is not prompted, but their card is still
  held back and discarded with everyone else's, not immediately. PASS.
- **−6 into an empty board** logs and does nothing. PASS.
- Permanents that leave the battlefield between the division and the choice: the
  `ChoosePile` handler re-checks `zone == Battlefield` before sacrificing each.
  PASS.

### Recorded, not changed

The `+1`'s cross-player choice queue is stored in `card_state`, a
`HashMap<String, ObjectId>` — so list lengths and player ids are written as
`ObjectId(len as u64)` and `ObjectId(u64::from(pid.0))`. The card carries a
comment about an earlier encoding that packed the queue into one value and
silently read it as zero. It is a type being used as a container for things that
are not object ids.

The general fix would be an engine mechanism for "ask each player in APNAP order,
then apply together" (CR 101.4). Liliana is the only card in this set that needs
one, so extracting it now would give it a single caller and no second opinion on
its shape. Left as is, recorded here.

### Test coverage

- simultaneous discard across players, active player first: `cards_upkeep_triggers_and_curses.rs` / `apnap.rs`.
- −2 targeted player chooses their own creature: `cards_sacrifice_and_additional_costs.rs`.
- −6 pile division and empty-pile choice: `cards_removal_and_bounce.rs`.
- engine names no cards in player-facing text: `engine_knows_no_cards.rs::engine_does_not_name_cards_in_the_text_it_shows` (new, mutation-checked).
- no run-time registry rebuilds: `test_suite_guards.rs::nothing_rebuilds_the_card_registry_at_run_time` (new).

