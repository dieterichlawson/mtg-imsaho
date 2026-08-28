## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/43/armored-skaab?utm_source=api
**Type line**: `Creature — Zombie Warrior` — {2}{U}, 1/4
**Oracle text**:
```
When this creature enters, mill four cards.
```

**Status**: PASS

### Code issues
No issues found.

'mill four cards' goes through `engine::mill_cards`, so the milled cards emit the events that mill-watchers in this set (Undead Alchemist, Selhoff Occultist) rely on.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/43/armored-skaab?utm_source=api
**Type line**: `Creature — Zombie Warrior` — {2}{U}, 1/4
**Oracle text**:
```
When this creature enters, mill four cards.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "When this creature enters, **mill four cards**" — its controller's own
  library, through the mill pipeline so creature cards emit
  `CreatureCardMilled`: PASS
- A library with fewer than four cards mills what it has: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The self-mill: `multi_target_and_mill.rs`, `cards_complex_creatures.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/43/armored-skaab?utm_source=api
**Type line**: `Creature — Zombie Warrior` — {2}{U}, 1/4
**Oracle text**:
```
When this creature enters, mill four cards.
```

**Rulings fetched**:
- [2011-09-22] If you have fewer than four cards in your library when Armored Skaab enters, you'll put all of them into your graveyard.

**Status**: PASS

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/43/armored-skaab
**Oracle text**: When this creature enters, mill four cards.
**Type line**: Creature — Zombie Warrior
**Mana cost**: {2}{U} — **P/T**: 1/4 — **Keywords**: Mill
**Rulings** (1, 2011-09-22): "If you have fewer than four cards in your library when Armored Skaab enters, you'll put all of them into your graveyard."

**Status**: PASS

### Code issues
No issues found.

### Card data
Matches the fetched text: `{2}{U}`, `card_types: [Creature]`,
`subtypes: ["Zombie", "Warrior"]` (both), 1/4, oracle text verbatim in the
current "mill four cards" keyword-action wording, and one `TriggeredAbilityDef`
of kind `EntersBattlefield` with `target_requirement: None` — right, the
ability targets nothing. `has_etb_handler()` returns true, so the trigger
actually reaches the stack.

`keywords` is empty. Scryfall lists "Mill" under keywords, but that is the
keyword *action* in the rules text, not a keyword ability, so there is nothing
for the field to carry — the same call made for Mindshrieker earlier in this run.

### Behaviour
The whole card is `mill_cards(state, controller, 4, "Armored Skaab", registry)`
from the ETB hook, reading `helpers::controller_of` so "you" survives the source
leaving the battlefield (CR 608.2g).

### Tricky interactions checked
- Mills exactly **four**: PASS — milling 3 fails
  `an_etb_trigger_collected_for_real_survives_its_source_dying`, which stocks a
  ten-card library and asserts the difference is 4.
- **You** mill, not the opponent: PASS — milling
  `state.opponent(controller)` fails the same test.
- The trigger still resolves after the Skaab dies (CR 113.7a): PASS — that test
  exists precisely to run cast → resolve → collect → kill → process.
- **The ruling** (fewer than four cards → all of them): the behaviour is
  `mill_cards`', whose doc cites CR 701.13b, and it is held down by four
  separate cards — `curse_of_bloody_tome_mills_the_last_card_and_says_so`,
  `nephalia_drownyard_mills_as_many_as_it_can_and_no_one_loses`,
  `splinterfright_mills_what_is_left_of_a_short_library`, and
  `heretics_punishment_mills_a_short_library_and_still_deals_damage`. Verified:
  making a short library mill nothing at all fails all four. A fifth per-card
  copy would be duplication rather than coverage, so none was added — recorded
  here so the absence is a decision and not an oversight.
- Milling an empty library is not a loss (CR 701.13b): covered by
  `cards_upkeep_triggers_and_curses.rs:96`, same rule, same helper.
- A milled creature card announces itself, so an opponent's Undead Alchemist
  sees it: structural — `CreatureCardMilled` is emitted by `move_object` on any
  library-to-graveyard move, which
  `move_object_emits_creature_card_milled_for_any_library_to_graveyard_move`
  pins down, and `mill_cards` goes through `mill_one` → `move_object`.
- Self-cleanup: none; this is a permanent.
- Vanilla body otherwise: no keywords, no activated abilities, no static
  effects.

### UI presentation
Trigger description: "mill four cards". The log line comes from `mill_cards`
and names the source: "Armored Skaab: p0 milled 4 cards", with a
"(of 4 — library ran out)" suffix when short. No choices.

### Test coverage
- Mills four, from the controller's library, and does so even after the source
  dies: `trigger_source_independence.rs:679`
  (`an_etb_trigger_collected_for_real_survives_its_source_dying`). Housed under
  CR 113.7a rather than under the card, but it is a full cast → resolve →
  collect → process run and both mutations above fail it.
- The ruling (short library): covered generically by four other cards' tests of
  the same `mill_cards` behaviour — see above. NOT TESTED per card, deliberately.
- The mill is announced: `multi_target_and_mill.rs:227`.

### Mutations run
| mutation | result |
| --- | --- |
| mill 3 instead of 4 | fails `an_etb_trigger_collected_for_real_survives_its_source_dying` |
| mill the opponent instead of the controller | fails the same test |
| `mill_cards`: a short library mills nothing | fails four other cards' short-library tests, confirming the ruling's rule is held down |

Suite: 1460 passing, exit 0, zero warnings. No changes were needed.

