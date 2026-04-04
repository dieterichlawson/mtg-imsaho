# Audit: Paraselene

## Reference (Scryfall/API)
- **Name:** Paraselene
- **Mana Cost:** {2}{W}
- **Type:** Sorcery
- **Oracle:** Destroy all enchantments. You gain 1 life for each enchantment destroyed this way.

## Implementation: `paraselene.rs`
- **Name:** Paraselene -- CORRECT
- **Mana Cost:** {2}{W} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **P/T:** None -- CORRECT
- **Keywords:** None -- CORRECT
- **oracle_text field:** Matches oracle -- CORRECT
- **Behavior:** Finds all enchantments on battlefield, calls try_destroy on each, counts successful destructions, gains that much life for controller -- CORRECT

## Verdict: PASS

## Audit -- 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Destroy all enchantments. You gain 1 life for each enchantment destroyed this way.
**Type line**: Sorcery
**Status**: PASS
### Code issues
None. Card data matches oracle: name, cost {2}{W}, type Sorcery, destroys all enchantments and gains 1 life per enchantment destroyed. Correctly uses try_destroy and checks DestroyResult::Died to count only actually destroyed enchantments. Behavior is correct.
