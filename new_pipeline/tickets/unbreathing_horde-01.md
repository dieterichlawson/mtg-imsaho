---
id: unbreathing_horde-01
status: fixed
card: Unbreathing Horde
audit_run_id: 2026-04-19-unbreathing_horde-audit
audit_model: sonnet
audit_tokens: 16267
audit_duration: 264
fixed_sha: 0b001c0
fixed_at: 2026-08-24T00:53:35Z
test_file: mtg-engine/tests/token_is_not_a_card.rs
fix_note: Oracle text saying "card" now excludes tokens (CR 109.1); added GameState::is_card.
---

## Audit Finding

**Oracle text:**
> each Zombie card in your graveyard

**Code:**
> let gy_count = u32::try_from(state.objects_in_zone(Zone::Graveyard, controller)
    .iter()
    .filter(|o| o.id != self_id && Self::is_zombie(o, registry))
    .count()).unwrap_or(u32::MAX);

**Description:**
The oracle text says 'each Zombie card in your graveyard.' Per CR 109.1, a 'card' is a physical game object — tokens are not cards, so Zombie tokens in the graveyard should not be counted. The graveyard filter calls `Self::is_zombie(o, registry)`, which returns `true` for any object whose `obj.subtypes` contains "Zombie" — including Zombie tokens (`card_id = 0`, not in registry, but `subtypes` populated by `create_token_with_subtypes`). The `is_token` field on `GameObject` is never checked. SBAs remove tokens from the graveyard before the next priority pass (CR 704.5e), but `entering_with_counters` is called during spell resolution, before SBAs run. A Zombie token that moved to the graveyard earlier in the same resolution window (e.g., sacrificed as a cost to a spell that reanimates the Horde, or killed by a trigger that resolved as part of the same batch) would be incorrectly counted, granting the Horde one extra +1/+1 counter per such token. The fix is to add `&& !o.is_token` to the graveyard filter. Note the battlefield count ('each other Zombie you control') correctly includes tokens because the oracle text there says 'Zombie' without the word 'card.'

**Engine path:** mtg-engine/src/cards/isd/unbreathing_horde.rs:67

**Required check:** 3

## Tests

### zombie_token_in_graveyard_not_counted
Scenario: A Zombie token dies during the same resolution window that the Horde enters the battlefield (before SBAs run); the token should not contribute to the Horde's entering counter count.

### zombie_card_in_graveyard_still_counted
Scenario: A real Zombie card (non-token) is in the graveyard when the Horde enters; it should contribute one counter, confirming the fix's !is_token guard does not accidentally exclude non-token Zombies.

