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

## Audit — 2026-04-02

### Oracle Text (Scryfall)
> Equipped creature gets +1/+0.
> Whenever equipped creature blocks or becomes blocked by a Vampire, destroy that creature. It can't be regenerated.
> Equip {1}

### Implementation: `mtg-engine/src/cards/isd/wooden_stake.rs`

### Checklist
- [x] Mana cost: `{2}` — correct
- [x] Card types: Artifact with Equipment subtype — correct
- [x] Continuous effect: +1/+0 via `ContinuousEffect::ModifyPT { power: 1, toughness: 0, scope: EffectScope::Attached }` — correct
- [x] Equip cost: `{1}`, sorcery speed only, targets creature you control — correct
- [x] Triggered ability fires on both "blocks" and "becomes blocked by" — correct (two `TriggeredAbilityDef` entries with `TriggerKind::Blocks` and `TriggerKind::BecomesBlocked`)
- [x] Vampire subtype check: checks both registry `card_data` subtypes AND `obj.subtypes` on the game object — correct
- [x] Destruction uses `try_destroy_no_regen` — correct, matches "It can't be regenerated"
- [x] `on_resolve` moves to battlefield and sets `is_equipment = true` — correct

### Issues

1. **Oracle text string mismatch — "destroy that Vampire" vs "destroy that creature"**
   - Oracle says: `destroy that creature. It can't be regenerated.`
   - Implementation `oracle_text` field says: `destroy that Vampire`
   - The code comment (line 9) and trigger descriptions (lines 33, 37) also say "destroy that Vampire".
   - Functional behavior is correct (the Vampire is correctly identified and destroyed). Only the stored text string is inaccurate.

2. **Missing "It can't be regenerated" in oracle_text string**
   - Oracle says: `destroy that creature. It can't be regenerated.`
   - Implementation `oracle_text` omits: `It can't be regenerated.`
   - Behavior is correct (`try_destroy_no_regen` is used), but the text is incomplete.

### Tests
- `wooden_stake_has_correct_data` — verifies card data basics
- `wooden_stake_grants_power` — verifies +1/+0 buff
- `wooden_stake_destroys_vampire_on_block` — verifies block trigger destroys vampire
- `wooden_stake_does_not_destroy_non_vampire` — verifies non-vampires are unaffected
- No test for the "becomes blocked by" direction (equipped creature attacks, vampire blocks)

### Verdict
Two minor text-only issues in the `oracle_text` field and comments. All functional behavior is correct.
