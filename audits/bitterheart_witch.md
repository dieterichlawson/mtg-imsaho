## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/88/bitterheart-witch?utm_source=api
**Type line**: `Creature — Human Shaman` — {4}{B}, 1/2
**Oracle text**:
```
Deathtouch
When this creature dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.
```

**Status**: ISSUE

### Code issues
See below.


- The "target player" was asked for at resolution rather than when the trigger
  went on the stack.
  - Oracle text says: `put it onto the battlefield attached to target player`
  - Code did: `target_requirement: None` on the `SelfDies` trigger, and a
    hand-built player list presented after the search
    (`fn present_player_choice(...)`)
  - CR 603.3d: a triggered ability's targets are chosen as it is put on the
    stack. Asking at resolution meant an opponent responding to the trigger
    could not know whom it would hit, and CR 608.2b never re-checked the choice.
    It also made this card filter hexproof players itself, rather than the
    engine doing it once for everything that targets a player. The trigger now
    declares `TargetRequirement::PlayerOnly` and the handler reads the target it
    was given.

### Tricky interactions checked
- Ruling: "The Curse must be legally able to enchant the player. For example, if
  the player has protection from red, you couldn't put a red Curse onto the
  battlefield this way." CR 303.4h, applied when the Curse would enter — the
  target was chosen before the search, so this cannot be a choice filter: PASS
- The ward arriving between targeting and resolution still stops the Curse: PASS
- "you **may** search" — declining is offered and does nothing: PASS
- A search that finds nothing still shuffles; a search that is declined does not:
  PASS
- Targeting yourself is legal — "target player", not "target opponent": PASS
- Deathtouch, and the trigger is on death (`SelfDies`), so it works from the
  graveyard using last known information: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Targeting at trigger time: `cards_complex_creatures.rs:bitterheart_witch_targets_its_player_when_the_trigger_goes_on_the_stack`
- Finding and attaching, to an opponent and to yourself: `cards_complex_creatures.rs:bitterheart_witch_finds_curse_on_death`, `:bitterheart_witch_can_attach_curse_to_self`
- Declining: `cards_complex_creatures.rs:bitterheart_witch_decline_search`
- Protection and CR 303.4h: `player_protection.rs`
- Hexproof filtered by the engine: `hexproof_filter.rs:bug_bitterheart_witch_hexproof_not_filtered`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/88/bitterheart-witch?utm_source=api
**Type line**: `Creature — Human Shaman` — {4}{B}, 1/2
**Oracle text**:
```
Deathtouch
When this creature dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.
```

**Rulings fetched**:
- [2011-09-22] The Curse must be legally able to enchant the player. For example, if the player has protection from red, you couldn’t put a red Curse onto the battlefield this way.

**Status**: ISSUE (fixed)

### Code issues

**The search forced a find, and declining to search skipped the shuffle.**

- Oracle text says: `you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.`
- CR 701.19b: "If a player is instructed to search a hidden zone for cards with
  a stated quality ... that player isn't required to find some or all of those
  cards even if they're present."
- Code did:
  ```rust
  if curse_ids.len() == 1 {
      // Only one Curse — no choice to present; the player is already known.
      Self::attach_and_shuffle(state, self_id, curse_ids[0], registry);
  } else {
      ... ResolutionChoiceKind::ChooseTarget { ..., optional: false, ... }
  }
  ```

Two problems in one. With exactly one Curse in the library it was taken for the
player, and with several the choice was `optional: false`, so a player who
searched had to find something. Searching a hidden zone never forces a find.

And the shuffle only ever ran alongside a find (or a fruitless search), because
it lived inside `attach_and_shuffle`. "…then shuffle" belongs to the *search* —
a player who searched and found nothing has still shuffled.

Restructured so the shuffle happens when the search happens, and the Curse is
always offered as an optional choice. The shuffle now runs before the Curse
leaves the library rather than after; that is unobservable, since the result is
a uniformly random order either way, and it is the simplest way to make
declining still shuffle. `attach_and_shuffle` is now `attach_curse` and does
only what its name says.

Three existing tests relied on the auto-find and now answer the extra choice —
that is the new step being real, not a test being bent to fit.

### Rulings checked

- **"The Curse must be legally able to enchant the player. For example, if the
  player has protection from red, you couldn't put a red Curse onto the
  battlefield this way."** `attach_curse` checks
  `player_can_be_enchanted_by` before moving the Curse, and on failure leaves it
  in the library rather than putting it onto the battlefield unattached (CR
  303.4h). The check happens when the Curse would *enter*, not when the player
  was targeted, so protection arriving in between still stops it. PASS, and
  `player_protection.rs` tests both directions — a black Curse against
  protection from red goes through, and a red one arriving after the ward does
  not.

### Tricky interactions checked

- **"attached to target player" is targeted** — `TargetRequirement::PlayerOnly`,
  chosen as the trigger goes on the stack (CR 603.3d), so an opponent can
  respond knowing who it will hit, and CR 608.2b re-checks on resolution. The
  targeted player is stashed in `card_state` because the search chain runs
  inside one resolution and cannot ask again. PASS.
- **Targeting yourself is legal** — the card says "target player", not "target
  opponent". PASS, tested.
- **`card_state` survives the death.** The trigger fires as the Witch dies and
  the stash is written on the object after it has left the battlefield;
  `move_object` deliberately does not clear `card_state` on the way out, for
  exactly this kind of leave-the-battlefield trigger. PASS.
- **Declining the "you may" does not shuffle** — correct: no search happened.
  Distinct from searching and finding nothing, which does shuffle. Both paths
  now exist. PASS.
- **Deathtouch** is a printed keyword and is declared. PASS.
- **A Curse is found by subtype**, `has_subtype(id, "Curse")`, not by name — so
  it finds any of the set's six Curses. PASS.

### Recorded

The targeted player is stored as `card_state["curse_target"] = ObjectId(pid.0)`
— a player id wearing an object id's type, because `card_state` is a
`HashMap<String, ObjectId>`. Same shape as Liliana of the Veil's queue. It
works, and the alternative is a typed per-object scratchpad, which is a change
worth making once across the cards that need it rather than here.

### Test coverage

- search and decline the only Curse: `cards_complex_creatures.rs::bitterheart_witch_may_search_and_decline_the_only_curse` (new).
- finds a Curse and attaches it to the targeted player: `::bitterheart_witch_finds_curse_on_death`.
- may target yourself: `::bitterheart_witch_can_attach_curse_to_self`.
- declining the search: `::bitterheart_witch_decline_search`.
- the ruling, both directions: `player_protection.rs::bitterheart_witch_attaches_a_curse_of_another_color_to_the_same_player`, `::a_curse_does_not_enter_attached_to_a_player_it_cannot_enchant`.
- protected player not offered as a target: `player_protection.rs`, `hexproof_filter.rs`.

