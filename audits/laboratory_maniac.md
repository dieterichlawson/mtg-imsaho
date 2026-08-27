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
