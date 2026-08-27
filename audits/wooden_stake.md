## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/237/wooden-stake?utm_source=api
**Type line**: `Artifact — Equipment` — {2}
**Oracle text**:
```
Equipped creature gets +1/+0.
Whenever equipped creature blocks or becomes blocked by a Vampire, destroy that creature. It can't be regenerated.
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
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
- "destroy that creature. **It can't be regenerated**" — the no-regenerate
  destruction path, not plain `try_destroy`: PASS
- "blocks **or becomes blocked by** a Vampire" — both directions: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Destroying a Vampire on block, and leaving a non-Vampire alone: `cards_equipment_costs.rs:wooden_stake_destroys_vampire_on_block`, `:wooden_stake_does_not_destroy_non_vampire`
