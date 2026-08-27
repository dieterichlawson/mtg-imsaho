## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/116/skeletal-grimace?utm_source=api
**Type line**: `Enchantment — Aura` — {1}{B}
**Oracle text**:
```
Enchant creature
Enchanted creature gets +1/+1 and has "{B}: Regenerate this creature."
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- The Aura grants an *activated ability* to the enchanted creature, so the
  ability is activated on the creature but dispatched to this card's behavior —
  which is exactly the `behavior_card_id` the engine resolves through the
  native → copy-grantor → attached walk. A card cannot work that out for itself,
  which is why the stack push is the engine's: PASS
- "{B}: Regenerate this creature" is a shield: it taps the creature, removes its
  damage and removes it from combat when it applies (CR 701.15): PASS
- Shields stack, and one is consumed per lethal event: PASS
- Regeneration does not save it from "destroy ... can't be regenerated", from
  exile, or from lethal -X/-X: PASS
- +1/+1 and the granted ability end together when the Aura leaves: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The granted regenerate ability and its limits: `cards_morbid_and_ltb.rs:skeletal_grimace_grants_regenerate_ability`, `:skeletal_grimace_regeneration_saves_from_lethal`, `:skeletal_grimace_regeneration_vs_deathtouch`, `:skeletal_grimace_regeneration_vs_doom_blade`
