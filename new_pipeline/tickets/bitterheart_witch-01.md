---
id: bitterheart_witch-01
status: fixed
card: Bitterheart Witch
audit_run_id: 2026-04-19-bitterheart_witch-audit
audit_model: sonnet
audit_tokens: 45598
audit_duration: 898
fixed_sha: d0afe9e
fixed_at: 2026-08-24T01:39:16Z
test_file: mtg-engine/tests/player_protection.rs
fix_note: Adds player protection from a color and the enchant-legality check (CR 702.16b / 303.4h); Bitterheart Witch filters and re-checks.
---

## Audit Finding

**Oracle text:**
> The Curse must be legally able to enchant the player. For example, if the player has protection from red, you couldn't put a red Curse onto the battlefield this way.

**Code:**
> .filter(|&pid| !state.player_has_hexproof(pid, registry) || pid == controller)

**Description:**
When the player chooses who to attach the found Curse to, both `present_player_choice` (bitterheart_witch.rs:17) and the `ChooseCurseThenAttach` handler (engine.rs:3907) filter valid player targets using only `player_has_hexproof`. The oracle ruling requires that the Curse must be legally able to enchant the chosen player — meaning a player with protection from the Curse's color (e.g., protection from red when the Curse is Curse of Stalked Prey or Curse of the Pierced Heart) must be excluded from valid targets. No such check exists anywhere in the target-building or attachment code. The `AttachCurseToPlayer` handler (engine.rs:3924–3938) also does not verify legality before calling `move_object` and setting `obj.attached_to_player`. The engine has no `player_has_protection_from_color` function, so this class of restriction is entirely absent. A player with protection from red would incorrectly appear as a valid target for a red Curse.

**Engine path:** mtg-engine/src/cards/isd/bitterheart_witch.rs:17

**Required check:** 8j

**Affected cards:**
- Bitterheart Witch

## Tests

### bitterheart_witch_respects_player_protection_from_color
Scenario: Witch dies with a red Curse (e.g. Curse of Stalked Prey) in library; opponent has protection from red; only the controller (not the protected opponent) should appear as a valid attachment target

