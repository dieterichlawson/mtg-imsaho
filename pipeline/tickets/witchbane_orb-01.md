---
id: witchbane_orb-01
status: new
card: Witchbane Orb
card_file: mtg-engine/src/cards/isd/witchbane_orb.rs
created: 2026-04-15T03:51:04Z
audit_run_id: 2026-04-14-witchbane_orb-audit
audit_model: opus
audit_tokens: 10258
audit_duration: 267
---

## Audit Finding

**Oracle text:**
> When this artifact enters, destroy all Curses attached to you.

**Code:**
> `registry.card_data(o.card_id).is_some_and(|d| d.subtypes.iter().any(|s| s == "Curse"))` (witchbane_orb.rs:52)

**Description:**
The curse detection in `on_enter_battlefield` only checks subtypes from the card registry (`registry.card_data()`). It does not check `obj.subtypes` on the game object itself. Per check 8d, a subtype check that only reads the registry misses tokens — tokens have `card_id = CardId(0)` (a sentinel not in the registry), so `registry.card_data(CardId(0))` returns `None` and `is_some_and(...)` returns `false`. Any token with the Curse subtype (e.g., created by a copy effect or future card) attached to the controller would be silently skipped. The correct dual-source pattern exists in `check_condition` (state.rs:1418-1423), which checks both `o.subtypes.iter().any(...)` and `registry.card_data(...)`.

**Engine path:**
- witchbane_orb.rs:47-55 (curse detection filter)
- state.rs:1418-1423 (correct dual-source pattern for comparison)

**Required check:** 8d

**Affected cards:**
- Witchbane Orb
- Any other card that checks subtypes via registry-only lookup

## Tests

### curse_token_not_destroyed
Source ticket: (new)
Implementation: (not yet written)
Scenario: Create a token with the Curse subtype attached to P0 (via `create_token_with_subtypes` with subtypes including "Curse" and `attached_to_player = Some(P0)`). Then trigger Witchbane Orb's ETB. Assert the curse token is destroyed. Currently it will NOT be destroyed due to the registry-only check.

