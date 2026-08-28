## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/15/fiend-hunter?utm_source=api
**Type line**: `Creature — Human Cleric` — {1}{W}{W}, 1/3
**Oracle text**:
```
When this creature enters, you may exile another target creature.
When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- CR 603.3d: "exile **another target** creature" is targeted, so the target is
  locked when the ETB trigger goes on the stack; only the "you may" decision
  remains at resolution, and the card offers exactly that locked target rather
  than a fresh pick: PASS
- "**another**" — it cannot exile itself: PASS
- "return the exiled card to the battlefield **under its owner's control**"
  (CR 110.2), and that is true when `EnteredBattlefield` fires rather than
  corrected afterwards: PASS
- The LTB trigger only returns a card still in exile, so a second effect that
  moved it in the meantime is respected (CR 608.2): PASS
- Exiling a token means it never comes back: PASS
- Declining the "you may" exiles nothing, and the LTB trigger then returns
  nothing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The exile/return pair and the locked target: `cards_complex_creatures.rs`, `trigger_target_recheck.rs`
- Returning under the owner's control: `control_change.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/15/fiend-hunter?utm_source=api
**Type line**: `Creature — Human Cleric` — {1}{W}{W}, 1/3
**Oracle text**:
```
When this creature enters, you may exile another target creature.
When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
```

**Rulings fetched**:
- [2018-12-07] If Fiend Hunter leaves the battlefield before its first ability has resolved, its second ability will trigger and do nothing. Then its first ability will resolve and exile the target creature indefinitely. This is different from abilities on other cards that exile a permanent "until" something happens.
- [2018-12-07] Once the exiled creature returns, it's considered a new object with no relation to the object that it was. Auras attached to the exiled creature will be put into their owners' graveyards. Equipment attached to the exiled creature will become unattached and remain on the battlefield. Any counters on the exiled creature will cease to exist.
- [2018-12-07] If a token is exiled this way, it won't return to the battlefield.
- [2018-12-07] In a multiplayer game, if you lose the game, the creature exiled with Fiend Hunter remains exiled indefinitely. This is also different from abilities on other cards that exile a permanent "until" something happens.

**Status**: PASS

### Code issues

No issues found. The card is correct on all four rulings; three of them had no test, which is fixed below.

### Checked against each ruling

- `If Fiend Hunter leaves the battlefield before its first ability has resolved, its second ability will trigger and do nothing. Then its first ability will resolve and exile the target creature indefinitely.` — PASS, and the mechanism is right, not just the outcome. The leave trigger has no `should_trigger` gate, so it fires whether or not anything was exiled; `on_leave_battlefield` reads `card_state["exiled_creature"]`, which is written only by `resolve_card_effect`, so at that point it is absent and the trigger does nothing. Verified the leave trigger goes on the stack **above** the enters trigger and resolves first.
- `Once the exiled creature returns, it's considered a new object with no relation to the object that it was. Auras attached to the exiled creature will be put into their owners' graveyards. Equipment attached to the exiled creature will become unattached and remain on the battlefield. Any counters on the exiled creature will cease to exist.` — PASS. All three fall out of general machinery rather than card code: SBA 704.5m for the Aura, 704.5n for the Equipment, and `move_object`'s leave-the-battlefield reset for the counters.
- `If a token is exiled this way, it won't return to the battlefield.` — PASS. CR 704.5d removes the token from `state.objects` entirely at the next SBA pass, so `on_leave_battlefield`'s `get_object` finds nothing. Worth noting the code's guard is `zone == Zone::Exile` rather than an `is_card` check — the right answer arrives via 704.5d rather than via the word "card" in the oracle text. Since the Hunter's leave trigger can only ever run after an SBA pass (SBAs run before any player receives priority, CR 117.5), the two are not distinguishable in play.
- `In a multiplayer game, if you lose the game, the creature exiled with Fiend Hunter remains exiled indefinitely.` — not applicable: this is a two-player engine, and a player losing ends the game.

### Checked and correct

- Cost `{1}{W}{W}`, `Creature — Human Cleric`, 1/3, oracle text verbatim.
- `another target creature` is `TargetRequirement::CreatureWithFilter(TargetFilter::Another)` on the trigger itself, so the target is locked as the trigger goes on the stack (CR 603.3d) and re-checked before resolution — not re-picked from the current battlefield at resolution.
- `you may` is a real decision, presented separately from the target choice. Confirmed the prompt sequence directly: with several legal targets the player is asked for the target first (mandatory, `optional: false`) and then whether to exile it (`optional: true`); with exactly one legal target the target prompt is skipped, since there is no choice to make.
- "Another" is not "another opponent's" — your own creature is offered, and that is a real play.
- The return is `move_object_under_control(target_id, Zone::Battlefield, owner, ...)`, so the creature is under its owner's control at the moment `EnteredBattlefield` fires rather than being corrected afterwards (CR 110.2).
- `card_state["exiled_creature"]` is cleared when the Hunter next enters the battlefield (`card_state.clear()` in `move_object`), so a blinked Hunter does not remember a creature it no longer exiled.
- The leave trigger checks the remembered creature is still in exile before returning it.

### Tricky interactions checked

- Hunter killed in response to its own enters trigger: creature exiled permanently. PASS.
- Exiled token: ceases to exist, does not return. PASS.
- Aura / Equipment / counters on the exiled creature: PASS.
- Creature stolen before being exiled returns to its **owner**: PASS.
- Leave trigger fires even with nothing exiled: PASS.
- "Another" excludes the Hunter itself: PASS.

### Test coverage

- exiles on enter and returns on death: `cards_morbid_and_ltb.rs:144`
- offers every creature but itself, and exiles nothing while the choice is pending: `cards_morbid_and_ltb.rs:916`
- leave trigger fires even when nothing was exiled: `empty_triggers.rs:94`
- returns under the owner's control after a steal: `enters_under_control.rs:74`
- copied Hunter's ETB is the Hunter's: `copy_effects.rs:87`
- exiled token does not return: `cards_morbid_and_ltb.rs` `fiend_hunter_does_not_return_an_exiled_token` (NEW, mutation-checked)
- killed in response, exiled for good: `cards_morbid_and_ltb.rs` `fiend_hunter_killed_in_response_exiles_the_creature_for_good` (NEW, mutation-checked against a bottom-up stack)
- returns as a new object (Aura, Equipment, counters): `cards_morbid_and_ltb.rs` `fiend_hunter_returns_a_new_object` (NEW, mutation-checked)
- multiplayer loss leaves it exiled: NOT TESTED — two-player engine, not reachable.

