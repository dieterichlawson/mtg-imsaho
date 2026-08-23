---
id: snapcaster_mage-02
status: fixed
card: Snapcaster Mage
audit_run_id: 2026-04-19-snapcaster_mage-audit
audit_model: sonnet
audit_tokens: 34984
audit_duration: 719
fixed_sha: 8f754da4380b632f90aa42b773f2c5f872a1fa27
fixed_at: 2026-08-23T23:10:08Z
test_file: mtg-engine/tests/flashback_multiple_instances.rs
fix_note: cluster fix: every available flashback cost is offered as its own castable option (CR 702.33); no-mana-cost cards no longer get a free one (702.33a)
---

## Audit Finding

**Oracle text:**
> If a card has multiple instances of flashback, you may choose any of its flashback costs to pay.

**Code:**
> let fb_cost = match dynamic_fb {
    Some(ref c) => c,
    None => match &data.flashback_cost {
        Some(c) => c,
        None => if cast_from_gy {
            match &data.cost {
                Some(c) => c,
                None => continue,
            }
        } else {
            continue,
        },
    },
};

**Description:**
The flashback action generator in `engine.rs` resolves which cost to use via a strict priority match: dynamic flashback (from `GrantFlashback` in `until_end_of_turn`) always wins over intrinsic flashback (`data.flashback_cost`). Only a single `CastSpell` action is generated per graveyard card. When Snapcaster targets a card that also has printed flashback — such as Past in Flames ({3}{R} mana cost, {4}{R}{R}{R} intrinsic flashback), or any card whose intrinsic flashback cost differs from its mana cost — the intrinsic flashback option is silently discarded. The player cannot choose the intrinsic cost even when it is cheaper or differently-colored than Snapcaster's grant. Per the ruling "If a card has multiple instances of flashback, you may choose any of its flashback costs to pay," both the dynamic cost (equal to mana cost, per Snapcaster's oracle) and the intrinsic flashback cost must each produce an independent `CastSpell` action. The fix requires generating one action per available flashback source instead of short-circuiting after the first match.

**Engine path:** mtg-engine/src/engine.rs:1231

**Required check:** 8j

**Affected cards:**
- Past in Flames

## Tests

### snapcaster_targets_card_with_intrinsic_flashback_both_costs_available
Scenario: A card with both an intrinsic flashback cost and dynamic flashback from Snapcaster is in the graveyard; the player should see two distinct CastSpell actions (one per flashback cost), but only the dynamic cost is currently offered.

### snapcaster_dynamic_does_not_suppress_cheaper_intrinsic_flashback
Scenario: A card whose intrinsic flashback is cheaper than its mana cost is targeted by Snapcaster; the player has mana to pay the intrinsic cost but not the dynamic cost — the card should still be castable via the intrinsic flashback but currently is not.

