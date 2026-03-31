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
