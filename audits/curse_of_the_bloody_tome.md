## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/50/curse-of-the-bloody-tome?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {2}{U}
**Oracle text**:
```
Enchant player
At the beginning of enchanted player's upkeep, that player mills two cards.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "At the beginning of **enchanted player's** upkeep" — CR 603.2: the trigger
  event is that player's upkeep beginning, so `TriggerScope::AttachedPlayer`
  keeps it off the stack during anyone else's: PASS
- CR 113.7a: destroying the Curse in response does not counter its trigger, and
  `attached_player` still knows whom it cursed: PASS
- Enchant **player**, so `TargetRequirement::PlayerOnly` and the Curse subtype:
  PASS
- Ruling: "If the enchanted player has only one card in their library, they put
  that card into their graveyard" — `mill_cards` stops at an empty library
  rather than making the player lose: PASS
- The mill goes through the pipeline, so a creature card among the two emits
  `CreatureCardMilled`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The upkeep mill: `cards_auras.rs`, `curse_and_equip_scope.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/50/curse-of-the-bloody-tome?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {2}{U}
**Oracle text**:
```
Enchant player
At the beginning of enchanted player's upkeep, that player mills two cards.
```

**Rulings fetched**:
- [2011-09-22] If the enchanted player has only one card in their library, they put that card into their graveyard.

**Status**: ISSUE (fixed)

### Code issues

One found, and it was six cards wide.

1. **The card logged a mill count it never verified.** `curse_of_the_bloody_tome.rs:63` (before the fix)
   - Oracle text says: `At the beginning of enchanted player's upkeep, that player mills two cards.`
   - Code did: `crate::engine::mill_cards(state, cursed_player, 2, registry);` followed by `state.log(..., format!("Curse of the Bloody Tome: p{} milled 2 cards", cursed_player.0));`
   - `mill_cards` had already logged the **real** count. With a one-card library the log read "p1 milled 1 card" and then "Curse of the Bloody Tome: p1 milled 2 cards" — two lines disagreeing, and the card's line, the one naming the source, is the one a reader trusts. That short-library case is precisely what this card's only ruling is about.

**Set-wide follow-up.** Six of the nine cards that mill did the same thing: Curse of the Bloody Tome, Splinterfright, Armored Skaab, Deranged Assistant, Selhoff Occultist and Undead Alchemist all logged their intended count next to the real one. `mill_cards` now takes the source name, logs one line, says when the library ran out ("milled 1 card (of 2 — library ran out)"), and returns how many actually went. The six duplicate lines are gone.

This is the same argument as moving the transform log into `apply_transform` earlier in this audit: the function that performs the action is the one that knows what it did, and a caller restating it from its own intentions will eventually be wrong.

### Checked against the ruling

- `If the enchanted player has only one card in their library, they put that card into their graveyard.` — PASS. `mill_cards` stops when `library_order` is empty and mills what it found (CR 701.13b), so a short library is milled out rather than failing. Milling is not drawing: the player does not lose for having an empty library — that only happens on a draw from one (CR 104.3c). Now tested, including that the player is still in the game afterwards.

### Checked and correct

- Cost `{2}{U}`, `Enchantment — Aura Curse`, subtypes `["Aura", "Curse"]`, oracle text verbatim.
- `target_requirement: PlayerOnly` implements `Enchant player`.
- `step_trigger_scope` returns `TriggerScope::AttachedPlayer`, so the trigger goes on the stack during the **enchanted** player's upkeep and not its controller's (CR 603.2).
- "**that player** mills two cards" — the mill is applied to `attached_player`, the cursed player, not to the Curse's controller. Note this card has no "you" in its text at all, so unlike Curse of the Pierced Heart there is no controller to read and nothing for the CR 608.2g class of bug to catch on.
- `attached_player` falls back to `last_attached_to_player`, so destroying the Curse in response does not counter its trigger (CR 113.7a) and it still knows whom it cursed.
- Scryfall lists `Mill` among the card's keywords; that is the keyword *action* (CR 701.13), not a keyword ability, so no `keywords` entry is wanted — the same reason `Enchant` has none.
- The mill goes through `mill_cards` → `mill_one` → `move_object`, which emits `CreatureCardMilled` for watchers rather than moving cards by hand.

### Tricky interactions checked

- One card in library: milled, no loss. PASS.
- Empty library: nothing milled, no loss, and the log says so rather than claiming two.
- Curse destroyed in response to its own trigger: still mills. PASS (`trigger_source_independence.rs:134`).
- Trigger fires only on the enchanted player's upkeep: PASS (`curse_and_equip_scope.rs`).
- Milled creature cards reach graveyard-watching cards (Splinterfright, Boneyard Wurm): holds via `move_object`'s event.

### Test coverage

- mills two on the enchanted player's upkeep: `cards_upkeep_triggers_and_curses.rs:380`
- destroyed in response, still mills: `trigger_source_independence.rs:134`
- fires only on the enchanted player's upkeep: `curse_and_equip_scope.rs:23`
- short library mills what is there, nobody loses, and the log is truthful: `cards_upkeep_triggers_and_curses.rs` `curse_of_bloody_tome_mills_the_last_card_and_says_so` (NEW, mutation-checked)

