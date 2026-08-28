## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/67/mindshrieker?utm_source=api
**Type line**: `Creature — Spirit Bird` — {1}{U}, 1/1
**Oracle text**:
```
Flying
{2}: Target player mills a card. This creature gets +X/+X until end of turn, where X is the milled card's mana value.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- The mill goes through `mill_one`, so a creature card among the milled emits
  `CreatureCardMilled` (Undead Alchemist is in this set): PASS
- The +X/+X is applied only while Mindshrieker is still on the battlefield: PASS
- X is the milled card's mana value, read after the move: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The mill emits CreatureCardMilled: `token_is_not_a_card.rs`
- Pump from the milled card's mana value: `cards_activated_abilities.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/67/mindshrieker?utm_source=api
**Type line**: `Creature — Spirit Bird` — {1}{U}, 1/1
**Oracle text**:
```
Flying
{2}: Target player mills a card. This creature gets +X/+X until end of turn, where X is the milled card's mana value.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/67/mindshrieker
**Oracle text**:
```
Flying
{2}: Target player mills a card. This creature gets +X/+X until end of turn, where X is the milled card's mana value.
```
**Type line**: Creature — Spirit Bird
**Mana cost**: {1}{U} — **P/T**: 1/1 — **Keywords**: Flying, Mill
**Rulings**: none (Scryfall returns no rulings for this card)
**Status**: ISSUE (fixed)

### Card data
Matches the fetched text: `{1}{U}`, `card_types: [Creature]`,
`subtypes: ["Spirit", "Bird"]` (both), 1/1, `keywords: [Flying]`, oracle text
verbatim (the current "This creature gets…" errata wording). Scryfall also lists
"Mill" under keywords; that is the keyword *action* in the ability's text, not a
keyword ability, so `keywords` correctly carries only Flying.

The `zone == Battlefield` gate in `activated_abilities` is the redundant-but-kept
kind recorded in the Mirror-Mad Phantasm entry — it restates a caller's
guarantee, not a game rule. Left alone.

### Code issues

1. The card hand-rolled the mill because the helper could not answer it
   (`mindshrieker.rs:53-63`, `cards_flow.rs:91` — `mill_cards` now returns the ids).
   - Oracle text says: `Target player mills a card. This creature gets +X/+X until end of turn, where X is the milled card's mana value.`
   - The card did:
     `let Some(&milled_card_id) = state.get_player(*player_id).library_order.first() else { return; };`
     then `crate::engine::mill_one(...)`, then
     `state.log(LogLevel::Event, format!("p{} milled 1 card", player_id.0));`
   - That log line names no source. `mill_cards`' own doc comment is explicit
     about why the helper owns the logging: "Six cards used to log their
     *intended* count next to this function's real one … and the card's line,
     the one naming the source, was the one a reader would trust." This card
     had drifted back out of the pipeline because it needed to know *which*
     card went, and `mill_cards` returned only a count.
   - `mill_cards` now returns `Vec<ObjectId>` — the cards that went, in order.
     The nine callers that ignore it are unchanged; a caller wanting the count
     reads `.len()`. Mindshrieker uses it and gets the standard
     `"Mindshrieker: p1 milled 1 card"` line.

2. Heretic's Punishment had the same hand-rolled shape
   (`heretics_punishment.rs:92-115`, converted).
   - It drained `library_order` itself so it could read the three mana values
     before the move. Those characteristics are the card's and do not change on
     a library-to-graveyard move (CR 400.7 makes it a new object, not a
     different card), so reading them from the returned ids is the same answer.
     Now one `mill_cards` call and a `.max()`. Converted here rather than
     deferred: leaving the only other caller of the old shape hand-rolling a
     helper that now covers it is how the drift started.

3. Mindshrieker read the mana value around the characteristics layer
   (`mindshrieker.rs:65`, changed).
   - It did:
     `registry.get(cid).and_then(|b| b.card_data().cost.as_ref().map(ManaCost::mana_value))`
   - Heretic's Punishment, asking the identical question, does:
     `state.face_data(card_obj_id, registry).and_then(|d| d.cost.map(|c| c.mana_value()))`
   - Same answer today — a card outside the battlefield shows its front face
     either way — but `face_data` is the accessor the characteristics phase
     exists to be, and it follows a copy grantor where the raw registry read
     does not. Now both cards ask it the same way.

4. Three things the ability's text says had no test at all
   (`cards_activated_abilities.rs`, tests added). Each of these mutations
   passed the entire workspace beforehand:
   - **"until end of turn"** — replacing the `until_end_of_turn` entry with a
     direct write to `obj.power`/`obj.toughness` broke nothing. Added
     `mindshrieker_pump_wears_off_at_end_of_turn`, which crosses a turn boundary
     through `advance_to_next_turn` so it is the engine's cleanup step doing the
     removing, not the test.
   - **"This creature gets +X/+X"** — dropping the card's
     `zone == Zone::Battlefield` check on the pump broke nothing. Added
     `mindshrieker_that_left_the_battlefield_still_mills_but_pumps_nothing`: the
     ability goes on the stack, the Mindshrieker dies in response, and on
     resolution the mill still happens (that half targets a player) while
     nothing is pumped (CR 400.7).
   - **"mills a card"** — the existing test put exactly one card in the
     library, so milling two was indistinguishable from milling one. Added a
     second card underneath and an assertion that it stays; milling 2 now fails.
   - Plus `mindshrieker_pumps_nothing_when_the_library_is_empty`: nothing is
     milled, there is no mana value to read, and the ability finishes rather
     than stalling on a choice.

### Tricky interactions checked
- Milled creature card announces itself, so an opponent's Undead Alchemist
  fires: PASS — `token_is_not_a_card.rs:187`
  (`mindshrieker_milled_creature_triggers_undead_alchemist`). This is what the
  `mill_one` routing was for, and `mill_cards` calls `mill_one`.
- A land is milled: mana value 0, no pump. PASS — the `("Forest", 0)` row.
- Empty library: no mill, no pump, no loss (CR 701.13b — milling fewer than
  asked is not a loss). PASS — new test.
- Source leaves the battlefield before resolution: mill happens, pump does not.
  PASS — new test.
- Pump is until end of turn: PASS — new test.
- "Target **player**", not opponent: `PlayerOnly` is right; the caster is a
  legal target and can mill themselves. Not separately tested for this card;
  the requirement is shared and covered elsewhere.
- Target becomes untargetable (Witchbane Orb) before resolution: the ability
  path re-checks targets in `stack::resolve_top_of_stack` and fizzles. Generic,
  covered by `an_activated_abilitys_targets_are_rechecked_when_it_resolves`.
- Activating twice in a turn: `once_per_turn: false`, and two `ModifyPT`
  entries stack. Not tested; the field is declared correctly and the effect list
  is additive by construction.
- A DFC milled from the library: `face_data` gives the front face, which is the
  mana value a card in a graveyard has (CR 712.8a). Correct by the accessor
  rather than by this card.

### UI presentation
The ability's `description` reads
`"{2}: Target player mills a card. Mindshrieker gets +X/+X (X = mana value)"`.
Log lines are now `"Mindshrieker: p1 milled 1 card"` from the helper and
`"Mindshrieker gets +6/+6 (milled card's mana value)"` from the card — the
source is named in both, which it was not before.

### Test coverage
- Pump equals the milled card's mana value, for a 6-drop and for a land:
  `cards_activated_abilities.rs`
  (`mindshrieker_pumps_by_the_milled_cards_mana_value`) — **extended this audit**
  with a second library card so the mill count is asserted.
- Pump wears off at end of turn: same file
  (`mindshrieker_pump_wears_off_at_end_of_turn`) — **added this audit**.
- Source gone before resolution: same file
  (`mindshrieker_that_left_the_battlefield_still_mills_but_pumps_nothing`) —
  **added this audit**.
- Empty library: same file
  (`mindshrieker_pumps_nothing_when_the_library_is_empty`) — **added this audit**.
- Milled creature card is announced: `token_is_not_a_card.rs:187`.
- No rulings exist for this card, so there is no per-ruling row to fill.

### Mutations run
| mutation | result |
| --- | --- |
| pump written to `obj.power`/`obj.toughness` instead of `until_end_of_turn` | fails `mindshrieker_pump_wears_off_at_end_of_turn` (before: **nothing**) |
| drop the `zone == Battlefield` guard on the pump | fails `mindshrieker_that_left_the_battlefield_still_mills_but_pumps_nothing` (before: **nothing**) |
| mill 2 cards instead of 1 | fails `mindshrieker_pumps_by_the_milled_cards_mana_value` (before the extra library card: **nothing**) |

Suite after: 1445 passing, exit 0, zero warnings.

