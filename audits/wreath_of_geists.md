# Audit: Wreath of Geists

## Scryfall Reference
- **Name:** Wreath of Geists
- **Cost:** {G}
- **Type:** Enchantment — Aura
- **Oracle:** Enchant creature / Enchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/wreath_of_geists.rs`
- Name: "Wreath of Geists" -- MATCH
- Cost: {G} -- MATCH
- Types: Enchantment -- MATCH
- Subtypes: ["Aura"] -- MATCH
- Target: Creature -- MATCH
- on_resolve: Uses resolve_aura helper -- CORRECT
- dynamic_pt: Counts creature cards (power.is_some()) in controller's graveyard, returns (X, X) -- MATCH

### Note
- The oracle says "Enchant creature" (the enchant keyword), which is implicit for Auras targeting creatures. The implementation handles this via the Aura subtype and target requirement.

## Verdict
**PASS** — Aura with graveyard-based pump correctly implemented.

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Enchant creature / Enchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.
**Mana cost**: {G}
**Type line**: Enchantment — Aura
**Status**: ISSUE
### Code issues
1. **Oracle text string incomplete**: Oracle includes `"Enchant creature"` as the first line, but code oracle_text is only `"Enchanted creature gets +X/+X, where X is the number of creature cards in your graveyard."` — missing the `"Enchant creature\n"` prefix.
### Behavior
Behavior is correct: target_requirement is Creature (correct for Enchant creature), dynamic_pt computes X as count of creature cards (power.is_some()) in controller's graveyard, on_resolve delegates to resolve_aura helper. Logic is sound.
