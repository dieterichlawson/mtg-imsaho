---
id: geist_honored_monk-01
status: deduped
card: Geist-Honored Monk
card_file: mtg-engine/src/cards/isd/geist_honored_monk.rs
created: 2026-04-14T21:25:27Z
audit_run_id: 2026-04-14-geist_honored_monk-audit
audit_model: opus
audit_tokens: 13619
audit_duration: 281
deduped_into: merged-creature-type-proxy-01
---

## Audit Finding

**Oracle text:**
> Geist-Honored Monk's power and toughness are each equal to the number of creatures you control.

**Code:**
> `state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.controller == controller && o.power.is_some()).count()` (geist_honored_monk.rs:41-43)

**Description:**
The `dynamic_pt` method identifies creatures using `o.power.is_some()` instead of checking `o.card_types.contains(&CardType::Creature)`. Per CR 302.1, a creature is defined by having the creature card type, not by having power/toughness. This proxy fails for permanents that have P/T but are not creatures (e.g., uncrewed Vehicles have printed P/T but creature type only while crewed) and could theoretically miss creatures without P/T (though none currently exist in standard rules). While no cards in the Innistrad set trigger this mismatch, the code deviates from the CR-defined concept of "creature." This is an engine-wide pattern — `power.is_some()` is used as a creature proxy throughout engine.rs and state.rs — but the card's `dynamic_pt` implementation inherits the inaccuracy.

**Engine path:**
- geist_honored_monk.rs:42

**Required check:** 8d

**Affected cards:**
- Geist-Honored Monk
- Any card whose `dynamic_pt` or behavior uses `power.is_some()` to count/identify creatures (engine-wide pattern)
