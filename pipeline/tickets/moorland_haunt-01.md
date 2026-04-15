---
id: moorland_haunt-01
status: closed-duplicate
card: Moorland Haunt
card_file: mtg-engine/src/cards/isd/moorland_haunt.rs
created: 2026-04-14T21:31:23Z
audit_run_id: 2026-04-14-moorland_haunt-audit
audit_model: opus
audit_tokens: 13727
audit_duration: 355
duplicate_of: merged-creature-type-proxy-01
---

## Audit Finding

**Oracle text:**
> {W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying.

**Code:**
> `let has_creature_in_graveyard = state.objects_in_zone(Zone::Graveyard, controller).iter().any(|o| o.power.is_some() && !o.is_token);` (moorland_haunt.rs:53-55)
> Same pattern at moorland_haunt.rs:82: `.filter(|o| o.power.is_some() && !o.is_token)`

**Description:**
The oracle text says "Exile a creature card from your graveyard." A "creature card" is defined by having the Creature card type, not by having power/toughness. The code uses `power.is_some()` as a proxy for "is a creature card." This proxy would incorrectly include non-creature cards that have printed power/toughness (e.g., Vehicle artifact cards have P/T but are not creature cards in the graveyard) and would theoretically exclude any creature without power (none exist currently but the check is semantically wrong). The `GameObject` struct has a `card_types: Vec<CardType>` field (state.rs:1553) that is available on graveyard objects and is not cleared by zone-change cleanup. The correct check is `o.card_types.contains(&CardType::Creature) && !o.is_token`. The `check_condition` function in state.rs:1414 demonstrates the correct pattern of checking both `obj.subtypes` and `registry.card_data().subtypes` for type-based conditions.

**Engine path:**
- mtg-engine/src/cards/isd/moorland_haunt.rs:53-55 (ability availability check)
- mtg-engine/src/cards/isd/moorland_haunt.rs:80-84 (exile candidate enumeration)

**Required check:** 8d

**Affected cards:**
- Moorland Haunt
- Any other card that uses `power.is_some()` as a proxy for "is a creature card" in a non-battlefield zone
