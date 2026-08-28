## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/235/trepanation-blade?utm_source=api
**Type line**: `Artifact — Equipment` — {3}
**Oracle text**:
```
Whenever equipped creature attacks, defending player reveals cards from the top of their library until they reveal a land card. The creature gets +1/+0 until end of turn for each card revealed this way. That player puts the revealed cards into their graveyard.
Equip {2}
```

**Status**: ISSUE

### Code issues
See below.


### Tricky interactions checked
- Equipment enters unattached and stays on the battlefield when what it equipped
  leaves (CR 704.5n), rather than going to the graveyard as an unattached Aura
  would (CR 704.5m): PASS — and this is the one that was wrong. Being an
  Equipment was a per-object `is_equipment` bool that eleven cards set in an
  `on_resolve` override which otherwise only repeated the trait default's "move
  a permanent to the battlefield". An Equipment that reached the battlefield any
  other way left the flag false and was then read as an Aura. Now derived from
  the Equipment subtype (CR 301.5) through the characteristics layer, and the
  eleven dead overrides are gone.
- "Equip only as a sorcery" — `sorcery_speed_only: true`: PASS
- "Attach to target creature **you control**" — `TargetFilter::YouControl` and
  the card's own `is_valid_target`: PASS
- The equip ability is offered on the Equipment, not duplicated onto the
  creature it is attached to: PASS
- The attach happens on resolution, not on activation (CR 602.2a): PASS
- "That player puts the revealed cards into their graveyard" is a mill
  (CR 701.13a), and it moved them by hand: `player.library_order.remove(0);
  state.move_object(card_id, Zone::Graveyard, registry);`. This is the one mill
  in the set that hits an *opponent's* library by default, which is exactly
  whose graveyard Undead Alchemist watches — "whenever a creature card is put
  into an opponent's graveyard from their library" — and it saw nothing.
  Fixed at the root: `move_object` now emits `CreatureCardMilled` for any
  library-to-graveyard move, so being a mill is a property of the zone change
  rather than of the caller having remembered a helper.
- Ruling: "The land card is counted when calculating the bonus, and it will be
  put into the graveyard with the other revealed cards" — the loop mills the
  land and then breaks, so it is counted: PASS
- Ruling: "If the equipped creature is attacking a planeswalker, the controller
  of the planeswalker is the defending player" — read from `AttackInfo`: PASS
- Both halves read the attack snapshot rather than the Blade's current
  `attached_to`, so killing the creature in response does not cancel the mill,
  and re-equipping does not move the buff onto a creature that never attacked:
  PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The mill emits CreatureCardMilled: `multi_target_and_mill.rs:trepanation_blade_emits_creature_card_milled`
- The attack snapshot: `trigger_snapshots.rs`, `trigger_source_independence.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/235/trepanation-blade?utm_source=api
**Type line**: `Artifact — Equipment` — {3}
**Oracle text**:
```
Whenever equipped creature attacks, defending player reveals cards from the top of their library until they reveal a land card. The creature gets +1/+0 until end of turn for each card revealed this way. That player puts the revealed cards into their graveyard.
Equip {2}
```

**Rulings fetched**:
- [2017-11-17] The land card is counted when calculating the bonus, and it will be put into the graveyard with the other revealed cards.
- [2017-11-17] If the equipped creature is attacking a planeswalker, the controller of the planeswalker is the defending player.

**Status**: PASS (stale wording and coverage fixed)

### Code issues

No behavioural bug. Two things fixed around it.

**The code quoted the card's printed wording, not its current oracle text.**

- Oracle text says: `The creature gets +1/+0 until end of turn for each card revealed this way. That player puts the revealed cards into their graveyard.`
- The comment in `on_attacks` said: `That player puts those cards into their graveyard. Equipped creature gets +1/+0 until end of turn for each card put into that player's graveyard this way.`

That is the original printed text. The card was errata'd: the bonus counts cards
**revealed**, not cards that reached the graveyard, and the two clauses swapped
order. Nothing in this set can separate the counts — no card replaces a mill —
so the behaviour is unaffected, but the count was also *named* `cards_milled`,
which encodes the old rule. Comment corrected and the variable renamed to
`cards_revealed`, so a future mill-replacement effect would be counted the way
the card says.

This is the failure mode the audit procedure warns about, sitting in the source
rather than in an auditor's head: the code was checked against remembered
wording once, and the memory was a decade stale.

**The ruling was not tested.**

`trepanation_blade_stops_on_land` asserts how many cards left the library and
never looks at the bonus — and it passes the *Blade* as the attacker, so any
buff would have landed on the Equipment rather than a creature. The number the
ruling is about had no coverage.

### Rulings checked

- **"The land card is counted when calculating the bonus, and it will be put
  into the graveyard with the other revealed cards."** The loop reveals, mills
  and counts the land before breaking, so a nonland-then-land library gives
  +2/+0 and both cards end up in the graveyard. PASS, now tested and
  mutation-checked by moving the increment after the break.
- **"If the equipped creature is attacking a planeswalker, the controller of the
  planeswalker is the defending player."** The handler takes
  `attack.defending_player` from the snapshot rather than deriving it, so it is
  whatever combat recorded. This engine does not model attacking a planeswalker,
  so the ruling is not reachable; the code would follow it if it were.

### Tricky interactions checked

- **The trigger reaches the Blade at all.** `Attacks` triggers are routed to
  objects attached to the attacker (`triggers/collect/combat.rs:34`), not just
  to the attacker itself. Verified rather than assumed. PASS.
- **The attack is read from the snapshot, not from the Blade's current
  `attached_to`.** So killing the equipped creature in response still mills, and
  re-equipping the Blade before the trigger resolves does not move the buff onto
  a creature that never attacked. The card carries a comment recording both
  failures. PASS, covered in `trigger_snapshots.rs`.
- **The buff is skipped when the attacker has gone** — there is nothing to put
  it on, while the mill (which names only the defending player) still happens.
  That is the CR 113.7a split done correctly. PASS.
- **A library with no land** is milled out and the loop stops on empty rather
  than spinning. PASS, now tested.
- **`is_land` reads `face_data`**, the front face, which is right for a card in
  a library (CR 712.8a). PASS.
- **Equip is sorcery-speed, targets a creature you control, and is not offered
  while the Equipment is itself a creature** (CR 301.5c). PASS.

### Test coverage

- the ruling — the land is counted and milled: `cards_equipment_and_artifacts.rs::trepanation_blades_bonus_counts_the_land_it_stopped_on` (new, mutation-checked).
- an all-nonland library empties instead of looping: `::trepanation_blade_stops_at_an_empty_library` (new).
- stops on the first land: `cards_shortcuts_taken.rs::trepanation_blade_stops_on_land`.
- the attack snapshot survives removal and re-equipping: `trigger_snapshots.rs:46`.
- mill goes through the shared pipeline: `multi_target_and_mill.rs:204`.

