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
