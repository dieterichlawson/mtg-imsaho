## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/221/demonmail-hauberk?utm_source=api
**Type line**: `Artifact — Equipment` — {4}
**Oracle text**:
```
Equipped creature gets +4/+2.
Equip—Sacrifice a creature.
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

### Deviation recorded, not changed
`legal_actions` filters out every (target, sacrifice) combo where the two are
the same object, and `activated_abilities` used to return nothing below two
creatures. Both are deliberate — `sacrifice_choice.rs`'s module doc explains
that the engine once auto-picked the sacrifice and fizzled the equip, and the
filter is there so a player cannot pick a fizzling combo by accident.

It is still a legal play the engine will not offer. Targets are chosen first
(CR 601.2c) and costs paid after (CR 601.2h); nothing stops you sacrificing the
creature you targeted, and the equip then fizzles (CR 608.2b). With Falkenrath
Noble out — "whenever this creature or another creature dies, target player
loses 1 life and you gain 1 life" — the fizzle is what you were buying.

Flagged rather than reversed: it spans three cards and several tests that
pin it on purpose, so it is the project's call, not the audit's.

- Ruling: "You can sacrifice the creature Demonmail Hauberk is equipping in
  order to equip it to another creature" — supported, as long as a second
  creature exists to be the new target: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The (target, sacrifice) enumeration: `sacrifice_choice.rs:hauberk_legal_actions_enumerate_target_sacrifice_combos`
- Explicit sacrifice attaches correctly: `sacrifice_choice.rs:hauberk_explicit_sacrifice_attaches_correctly`
