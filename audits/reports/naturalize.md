# Audit: Naturalize

## Official Oracle
- **Name:** Naturalize
- **Cost:** {1}{G}
- **Type:** Instant
- **Oracle:** Destroy target artifact or enchantment.

## Implementation: `mtg-engine/src/cards/naturalize.rs`
- **Name:** Naturalize -- CORRECT
- **Cost:** {1}{G} -- CORRECT
- **Type:** Instant -- CORRECT
- **Target:** PermanentWithFilter(HasCardType [Artifact, Enchantment]) -- CORRECT
- **is_valid_target:** Checks battlefield, artifact or enchantment -- CORRECT
- **on_resolve:** Uses helpers::resolve_destroy -- CORRECT

## Verdict
**PASS** -- No issues found.

## Audit — 2026-04-02

**Oracle source**: Scryfall  
**Card**: Naturalize  
**Type**: Instant | **Cost**: {1}{G}  
**Oracle text**: "Destroy target artifact or enchantment."

### Checks
- Name: "Naturalize" -- PASS
- Cost: {1}{G} -- PASS
- Type: Instant -- PASS
- Target requirement: PermanentWithFilter for Artifact or Enchantment -- PASS
- Target validation: Checks battlefield zone, verifies card_types contain Artifact or Enchantment -- PASS
- Effect: Calls `resolve_destroy` helper -- PASS

**Verdict: PASS**
