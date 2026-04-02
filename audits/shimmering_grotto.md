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

## Audit - 2026-04-02

### Oracle Text (Scryfall)
- **Name:** Shimmering Grotto
- **Mana Cost:** (none, Land)
- **Type:** Land
- **Oracle Text:** {T}: Add {C}. / {1}, {T}: Add one mana of any color.

### Card Data Audit
- **Name:** Correct ("Shimmering Grotto")
- **Cost:** Correct (None)
- **Types:** Correct (Land)
- **Oracle Text String:** Correct

### Behavior Audit
- **{T}: Add {C}:** Mana ability producing (ManaType::Colorless, 1), requires tap, only when untapped on battlefield. Correct.
- **{1}, {T}: Add one mana of any color:** Implemented as 5 separate activated abilities (W, U, B, R, G), each costing Generic(1) and requiring tap. Correctly models color choice.
- **Tap exclusivity:** Both ability types require tap, so only one can be used per untap cycle. Correct.

### Result
**PASS**
