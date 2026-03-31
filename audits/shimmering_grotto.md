# Audit: Shimmering Grotto

## Official Oracle
- **Name:** Shimmering Grotto
- **Cost:** None
- **Type:** Land
- **Oracle Text:** {T}: Add {C}.\n{1}, {T}: Add one mana of any color.
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** None — OK
- **Type:** Land — OK
- **Oracle Text:** Matches — OK
- **P/T:** N/A — OK
- **Mana Abilities:** Produces (Colorless, 1) with requires_tap — OK
- **Activated Abilities:** 5 separate activated abilities for each color ({W}, {U}, {B}, {R}, {G}), each costing {1} and requiring tap — OK
- **on_activate_ability:** Adds the chosen color mana to controller's pool — OK

## Issues
None found. The "any color" is correctly modeled as 5 separate abilities.

## Verdict: PASS
