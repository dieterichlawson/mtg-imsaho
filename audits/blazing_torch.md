## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/216/blazing-torch?utm_source=api
**Type line**: `Artifact — Equipment` — {1}
**Oracle text**:
```
Equipped creature can't be blocked by Vampires or Zombies.
Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target."
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
- "{T}, Sacrifice Blazing Torch:" — the sacrifice is a cost, so it is paid on
  activation (CR 601.2h) and an opponent responding already sees the Torch in
  the graveyard. The Torch is not the object the ability is activated on, so the
  `ActivatedAbilityDef`'s `SacrificeCost` cannot express it; it is now the one
  card besides Moorland Haunt that uses `pay_activation_cost`.
- Ruling: "The source of the damage is Blazing Torch, not the equipped
  creature." The Torch is in the graveyard by resolution, so it is found through
  the `last_attached_to` the engine records on every zone change — last known
  information, CR 608.2g: PASS
- "Equipped creature can't be blocked by Vampires or Zombies" — a blocking
  restriction, not evasion, and not menace: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Damage to a creature and to a player, sourced from the Torch: `cards_equipment_and_artifacts.rs:blazing_torch_deals_damage_to_player`, `:blazing_torch_deals_its_damage_as_the_torch_not_the_creature`
