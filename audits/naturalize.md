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
