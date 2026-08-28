## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/236/witchbane-orb?utm_source=api
**Type line**: `Artifact` — {4}
**Oracle text**:
```
When this artifact enters, destroy all Curses attached to you.
You have hexproof. (You can't be the target of spells or abilities your opponents control, including Aura spells.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "destroy **all** Curses attached to you" — every one, via `try_destroy_all`,
  and only those attached to the Orb's controller: PASS
- "You have hexproof" — a *player* hexproof, so opponents cannot target you
  with spells or abilities, including Aura spells (Curses): PASS
- The ETB destruction and the static hexproof are separate: an existing Curse
  is destroyed, and future ones cannot be attached: PASS
- Your own spells can still target you: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The Curse sweep and player hexproof: `hexproof_filter.rs`, `player_protection.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/236/witchbane-orb?utm_source=api
**Type line**: `Artifact` — {4}
**Oracle text**:
```
When this artifact enters, destroy all Curses attached to you.
You have hexproof. (You can't be the target of spells or abilities your opponents control, including Aura spells.)
```

**Rulings fetched**: none published for this card.

**Status**: PASS


No rulings are cached for this card and none surfaced; the oracle text above is
the whole of it.

### Code issues
No issues found in the card. The card file is correct on every point: {4}
Artifact, oracle text exact, the ETB declared untargeted (it is "all Curses",
not "target Curse"), `controller_of` for the trigger's "you", the Curse filter
scoped to `attached_to_player == Some(controller)` for "attached to **you**",
`try_destroy_all` for one simultaneous event (CR 700.2c), and per-Curse logging
that reports what actually happened rather than announcing every one destroyed.

The engine side was correct too: `player_has_hexproof` counts only Orbs on the
battlefield (CR 113.6), and `can_target_player` exempts the caster themself,
which is what CR 702.11b's "spells or abilities your **opponents** control"
means.

### Engine issue found and fixed
"May this caster target this player" was written out three separate times:

1. `targeting::can_target_player` — the canonical one, used by six enumeration
   sites, each of which *also* filtered `!p.lost` itself.
2. `stack.rs`'s CR 608.2b re-check — restated inline, no `lost` check.
3. `helpers::any_targets` and `any_targets_except` — restated inline again, no
   `lost` check.

Witchbane Orb is the only card in the pool that grants a player hexproof, so
each divergent copy is a way for its one static ability to be skipped, and the
missing `lost` check meant a player who had left the game stayed a legal "any
target" and stayed legal through every resolution re-check.

The rule now lives in `can_target_player` alone, with the `lost` check folded
in (CR 104.3a), and all nine sites call it. The six `!p.lost` filters in
`targeting.rs` are gone as redundant, and the two hand-rolled copies are
replaced by calls.

This matters most at (2): CR 608.2b says a target is re-checked on resolution,
which only means anything if the re-check applies the same rule that offered
the target. A rule stated twice is a rule that can disagree with itself.

### Tricky interactions checked
- ETB destroys the Curses attached to you: pass
- ETB leaves a Curse attached to the opponent alone, even one you control: pass
- ETB destroys a Curse attached to you that an *opponent* controls: pass
- "Destroy" respects indestructible: pass
- Hexproof only while the Orb is on the battlefield (CR 113.6): pass
- You can still target yourself (CR 702.11b): pass
- An opponent cannot target you: pass
- A player who has lost is not a legal target (CR 104.3a): pass, after the fix
- The resolution re-check applies the same rule as the offer (CR 608.2b): pass,
  after the fix

### Test coverage
- Player has hexproof with the Orb: `cards_lands_and_mana_sources.rs:551`
- Opponent cannot target you: `cards_lands_and_mana_sources.rs:565`
- You can still target yourself: `cards_lands_and_mana_sources.rs:591`
- A player-targeting trigger skips the hexproof player:
  `hexproof_filter.rs:250`, `hexproof_filter.rs:319`
- A Curse cannot be attached to a hexproof player: `hexproof_filter.rs:587`
- **NEW** ETB destroys the Curses on you and leaves the others:
  `cards_lands_and_mana_sources.rs:485`
- **NEW** "destroy" cannot move an indestructible Curse:
  `cards_lands_and_mana_sources.rs:513`
- **NEW** hexproof stops when the Orb leaves the battlefield:
  `cards_lands_and_mana_sources.rs:531`
- **NEW** a player who has lost is not an "any target": `hexproof_filter.rs:625`
- **NEW** the resolution re-check uses the same rule: `hexproof_filter.rs:646`

### What was untested before
Every existing Witchbane Orb test was about hexproof. The entire triggered
half of the card — "destroy all Curses attached to you", which is what the
card is *for* against a Curse deck — had no test at all, and neither did the
CR 113.6 restriction that makes the static ability stop when the Orb dies.

