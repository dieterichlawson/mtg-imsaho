## Audit — 2026-08-27 (Tier C — one behaviour hook: replacement effect)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/61/laboratory-maniac?utm_source=api
**Type line**: `Creature — Human Wizard` — {2}{U}, 2/2
**Oracle text**:
```
If you would draw a card while your library has no cards in it, you win the game instead.
```
**Status**: ISSUE

### Code issues
See the marked item below.

### What was checked
Card data was verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`). This card's
one hook is `replace_event`, so the audit centres on CR 614 — whether the effect
applies to the right events, exactly once, and modifies rather than replaces
where the oracle says "instead".

- The win itself is right: the draw is replaced rather than performed, and
  `has_drawn_from_empty` is cleared so the state-based action cannot kill the
  player before the replacement takes effect. That ordering is the whole card.
- **Issue (fixed):** it marked the opponent as lost with
  `LossReason::LifeReachedZero`.
  - Oracle text says: `If you would draw a card while your library has no cards in it, you win the game instead.`
  - Code did: `reason: crate::events::LossReason::LifeReachedZero`
  - The opponent may be on twenty life. They lose because their opponent won
    (CR 104.2a), and none of the three existing variants said so. Added
    `LossReason::OpponentWon` and used it. Marking the opponent lost at all is
    necessary — `.lost` gates targeting and the living-player count — so only
    the stated reason was wrong.

### Test coverage
`state_based_actions.rs` covers draw-from-empty; the win replacement itself is NOT TESTED
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/61/laboratory-maniac?utm_source=api
**Type line**: `Creature — Human Wizard` — {2}{U}, 2/2
**Oracle text**:
```
If you would draw a card while your library has no cards in it, you win the game instead.
```

**Status**: PASS

### Code issues
No issues found.

"If you would draw a card while your library has no cards in it, you win the
game instead." Implemented as a `replace_event` on
`ReplaceableEvent::DrawsFromEmptyLibrary`, returning `Replacement::Replaced` —
the draw does not happen, which is what "instead" means. It clears
`has_drawn_from_empty` first, so the state-based action that would kill the
player for the attempted draw does not fire before the win. Controller-scoped:
it returns `None` for any other player's draw.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`replacement_effects.rs` — the win replaces the draw; a second player's empty draw still loses.

## Audit — 2026-08-28 18:58

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Laboratory Maniac"`, https://scryfall.com/card/isd/61/laboratory-maniac
**Oracle text**:
```
If you would draw a card while your library has no cards in it, you win the game instead.
```
**Type line**: Creature — Human Wizard
**Mana cost**: {2}{U}   **P/T**: 2/2
**Rulings** (2, 2021-03-19): the Angel's Grace one ("you won't lose for having tried to draw...
The draw was still replaced") and the multi-Maniac turn-order one.
**Status**: PASS (one zone-scoping gap in the tests closed)

### Code issues
No issues found in `mtg-engine/src/cards/isd/laboratory_maniac.rs`.

`{2}{U}`, `Creature`, `subtypes: ["Human", "Wizard"]` — both — 2/2, oracle text verbatim, no
triggered abilities (it is a replacement effect, CR 614.1a, and is implemented as one through
`replace_event` on `DrawsFromEmptyLibrary`).

The three things it has to get right, it does:
- **"You"**: the controller check, so an opponent drawing from empty still loses.
- **Replacement, not trigger**: the win happens at the draw, not at a state-based check — with a
  Maniac out the game is already over when `draw_cards` returns, and there is no window in which
  the empty-draw flag is set and could be responded to.
- **The draw does not happen and neither does the loss**: `has_drawn_from_empty` is cleared, so
  SBA 704.5b cannot kill the winner for the draw that was replaced.

The battlefield scoping is the engine's: `replacement_zones` defaults to `[Battlefield]`, and the
Maniac does not override it.

### Tricky interactions checked
- **An opponent drawing from empty**: PASS — they lose as normal.
- **A Maniac in the graveyard or hand**: PASS — does nothing, which for this card means "does not
  win the game from the graveyard". This is the negative direction of `replacement_zones`; the
  positive one (Dearly Departed declaring the graveyard) is `replacement_effects.rs`.
- **The Angel's Grace ruling** ("if you can't win, you still don't lose — the draw was still
  replaced"): the flag-clear happens before the win, so the two halves are separate as the ruling
  requires. Unreachable in this pool — nothing here prevents winning — but the mechanism is
  tested via the flag mutation below.
- **Two Maniacs, both players drawing**: turn order per the second ruling. Not reachable as a
  simultaneous instruction in this pool (nothing makes both players draw at once).
- **The Maniac removed in response to the draw**: replacements apply as the event happens, so a
  Maniac killed at instant speed before the draw is simply not there — covered by the "no Lab
  Maniac" row.

### Test coverage
One table, `cards_rule_modifiers.rs:33 laboratory_maniac_replaces_the_empty_draw_loss_for_its_controller`,
now five rows:
- no Maniac: the drawer loses
- its controller draws from empty: the controller wins, the opponent loses
- the opponent draws: the Maniac does nothing for them
- a Maniac in the **graveyard** does not win the game from there (NEW)
- nor does one in **hand** (NEW)

Each row also asserts the *other* player's fate, so a row cannot pass by everyone losing.

Mutation-checked: overriding `replacement_zones` to include graveyard and hand fails the two new
rows; leaving `has_drawn_from_empty` set fails the win row (the winner would be killed by SBA
for the draw that was replaced — the mechanism behind the Angel's Grace ruling).

### Changes made
- `cards_rule_modifiers.rs`: the table gained the two wrong-zone rows. No code change.
