---
id: merged-temp-effect-zone-persist-02
status: new
card: multiple
created: 2026-04-15T04:57:48Z
kind: consolidated
source_tickets: charmbreaker_devils-02, kessig_wolf_run-01, manor_gargoyle-01, morkrut_banshee-02, past_in_flames-02, liliana_of_the_veil-02, trepanation_blade-03, merged-temp-effect-zone-persist-01
---

# TemporaryEffect and until-end-of-turn state persist across zone changes (CR 400.7)

## Description
Per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence. The engine reuses `ObjectId` across zone changes (incrementing `zone_change_count` in state.rs:567 instead of minting a new ID). Runtime effects keyed by `ObjectId` alone — `TemporaryEffect::ModifyPT`, `GrantKeyword`/`RemoveKeyword`, `GrantFlashback`, and the planeswalker `abilities_activated_this_turn` sentinel — all survive zone changes. `move_object` (state.rs:572-583) does not prune `until_end_of_turn` entries, and `effective_power`/`effective_toughness`/`has_keyword` match by ObjectId without consulting `zone_change_count`.

## Engine path
- state.rs:567 (zone_change_count increment — exists but unused at lookup)
- state.rs:572-583 (move_object cleanup — does not prune until_end_of_turn)
- state.rs:1072-1076 (effective_power — no zone_change_count check)
- state.rs:1253-1256 (has_keyword — no zone_change_count check)
- state.rs:209 (GrantFlashback stores ObjectId, no zone_change_count)

## Tests

### test_charmbreaker_devils_pump_does_not_persist_through_bounce
Source ticket: charmbreaker_devils-02
Implementation: (not yet written)
Scenario: Give Charmbreaker Devils +4/+0 via its second ability. Bounce and recast it in the same turn. Verify the replayed Devils has base P/T.

### test_kessig_wolf_run_pump_does_not_persist_through_bounce
Source ticket: kessig_wolf_run-01
Implementation: (not yet written)
Scenario: Activate Kessig Wolf Run giving a creature +3/+0 and trample. Bounce and replay it this turn. Verify the replayed creature lacks the pump and trample.

### test_manor_gargoyle_until_eot_effects_do_not_persist
Source ticket: manor_gargoyle-01
Implementation: (not yet written)
Scenario: Activate Manor Gargoyle's ability (removes Defender, grants Flying until EOT). Bounce and replay it in the same turn. Verify the replayed Gargoyle has Defender, no Flying, and retains Indestructible.

### test_morkrut_banshee_minus_4_does_not_persist_through_bounce
Source ticket: morkrut_banshee-02
Implementation: (not yet written)
Scenario: Morkrut Banshee ETBs with morbid, applying -4/-4 to a target. Bounce that target and replay it in the same turn. Verify the replayed creature has base P/T.

### test_past_in_flames_granted_flashback_expires_on_zone_change
Source ticket: past_in_flames-02
Implementation: (not yet written)
Scenario: Resolve Past in Flames granting flashback to all instants/sorceries in graveyard. Exile an instant (e.g., via Purify the Grave), then return it to graveyard the same turn. Verify that instant does NOT have flashback.

### test_liliana_relog_can_activate_loyalty_after_rebounce
Source ticket: liliana_of_the_veil-02
Implementation: (not yet written)
Scenario: Activate Liliana's +1 (marks her abilities_activated_this_turn sentinel). Bounce Liliana to hand and recast her. Verify the new Liliana can activate a loyalty ability.

### test_trepanation_blade_buff_does_not_persist_through_blink
Source ticket: trepanation_blade-03
Implementation: (not yet written)
Scenario: Equip Trepanation Blade to a 2/2 creature. Set up defender's library so 3 cards are milled. Trigger the blade's ability so the creature gets +3/+0 (effective power 5). Blink the creature (exile then return). Assert effective power is 2 (base), not 5 — the buff should not survive the zone change.

## Also closes

- merged-temp-effect-zone-persist-01

