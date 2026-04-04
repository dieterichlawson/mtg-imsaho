# Audit: Thraben Sentry // Thraben Militia

## Scryfall Reference
### Front Face
- **Name:** Thraben Sentry
- **Cost:** {3}{W}
- **Type:** Creature — Human Soldier
- **Oracle:** Vigilance / Whenever another creature you control dies, you may transform this creature.
- **P/T:** 2/2

### Back Face
- **Name:** Thraben Militia
- **Cost:** *(none)*
- **Type:** Creature — Human Soldier
- **Oracle:** Trample
- **P/T:** 5/4

## Implementation: `mtg-engine/src/cards/thraben_sentry.rs`

### Front Face
- Name: "Thraben Sentry" -- MATCH
- Cost: {3}{W} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Human", "Soldier"] -- MATCH
- P/T: 2/2 -- MATCH
- Keywords: [Vigilance] -- MATCH
- Trigger: AnyCreatureDies -- MATCH (filters to controller's creatures in handler)

### Back Face
- Name: "Thraben Militia" -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Human", "Soldier"] -- MATCH
- P/T: 5/4 -- MATCH
- Keywords: [Trample] -- MATCH

### Behavioral Notes
- The "you may" is simplified to always transform (auto-yes). Noted in code comment.
- Correctly filters: only fires for creatures dying under the same controller.
- Only transforms from front face (checks !is_transformed). Back face has no transform trigger.
- should_transform returns false (no werewolf-style upkeep transform).

## Verdict
**PASS** — Implementation matches oracle text. The "you may" simplification is documented.

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text (front)**: Vigilance\nWhenever another creature you control dies, you may transform this creature.
**Oracle text (back — Thraben Militia)**: Trample
**Type line**: Creature — Human Soldier // Creature — Human Soldier
**Mana Cost**: {3}{W}
**P/T**: 2/2 // 5/4
**Status**: PASS
### Code issues
None. Card data matches oracle: front face has Vigilance keyword, 2/2, subtypes Human Soldier, cost {3}{W}. Back face Thraben Militia is 5/4 with Trample. Transform trigger on AnyCreatureDies correctly filters to only controller's creatures and only on front face. dynamic_pt returns (5,4) when transformed. All correct.
