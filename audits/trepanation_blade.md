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
