# Audit: Wooden Stake

## Scryfall Reference
- **Name:** Wooden Stake
- **Cost:** {2}
- **Type:** Artifact — Equipment
- **Oracle:** Equipped creature gets +1/+0. Whenever equipped creature blocks or becomes blocked by a Vampire, destroy that creature. It can't be regenerated. Equip {1}
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/wooden_stake.rs`
- Name: "Wooden Stake" -- MATCH
- Cost: {2} -- MATCH
- Types: Artifact -- MATCH
- Subtypes: ["Equipment"] -- MATCH
- Equip: {1} -- MATCH
- Continuous effect: ModifyPT { power: 1, toughness: 0, scope: Attached } -- MATCH (+1/+0)
- Triggers: Blocks, BecomesBlocked -- MATCH
- is_equipment set on resolve -- CORRECT

### Behavioral Analysis
- on_blocks: Checks if other creature is Vampire (registry + instance subtypes), destroys with try_destroy_no_regen -- MATCH ("can't be regenerated")
- on_becomes_blocked: Same Vampire check, destroys with try_destroy_no_regen -- MATCH

### ISSUE: "Destroy that creature" vs "Destroy that Vampire"
- Oracle says "destroy that creature" referring to the Vampire specifically. But the Scryfall text says "destroy that creature. It can't be regenerated." The implementation correctly destroys the Vampire with no-regen flag.
- Wait: re-reading the oracle: "Whenever equipped creature blocks or becomes blocked by a Vampire, destroy that creature." Here "that creature" refers to the Vampire, not the equipped creature. The implementation correctly destroys the other_creature/blocker_id (the Vampire), not the equipped creature. CORRECT.

## Verdict
**PASS** — Equipment with Vampire-destroying trigger correctly implemented, including can't-be-regenerated.

## Audit — 2026-04-01 15:09

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Equipped creature gets +1/+0.
Whenever equipped creature blocks or becomes blocked by a Vampire, destroy that creature. It can't be regenerated.
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
**Type line**: Artifact — Equipment
**Ruling**: [2011-09-22] The Vampire is destroyed before any combat damage is dealt.
**Status**: PASS

### Code issues
No issues found.

Minor oracle_text field discrepancies (cosmetic, not behavioral):
- Code oracle_text says "destroy that Vampire" — oracle says "destroy that creature" (semantically equivalent per ruling, and the code correctly destroys the Vampire)
- Code oracle_text omits "It can't be regenerated." — but the code correctly uses `try_destroy_no_regen`

### Tricky interactions checked
- Equipped creature blocks a Vampire: PASS — `on_blocks` checks Vampire subtype via both registry and instance subtypes, uses `try_destroy_no_regen`
- Equipped creature becomes blocked by a Vampire: PASS — `on_becomes_blocked` same logic
- Non-Vampire blocking: PASS — test `wooden_stake_does_not_destroy_non_vampire` confirms no destruction
- Trigger system dispatches to equipment: PASS — triggers.rs lines 760-770 create BlocksTrigger for attached equipment, lines 807-817 create BecomesBlockedTrigger for attached equipment
- Token Vampires detected: PASS — checks both `registry.card_data` subtypes and `obj.subtypes` on the game object

### Test coverage
- Card data correctness: `tier9_equipment.rs:wooden_stake_has_correct_data`
- +1/+0 power bonus: `tier9_equipment.rs:wooden_stake_grants_power`
- Destroys Vampire on block: `tier9_equipment.rs:wooden_stake_destroys_vampire_on_block`
- Does not destroy non-Vampire: `tier9_equipment.rs:wooden_stake_does_not_destroy_non_vampire`
- Equipped creature attacks and is blocked by Vampire (becomes_blocked direction): NOT TESTED
- "Can't be regenerated" (Vampire with regeneration shield): NOT TESTED
- Ruling — Vampire destroyed before combat damage: NOT TESTED (implicitly covered by trigger timing)
