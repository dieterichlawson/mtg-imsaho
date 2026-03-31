# Audit: Full Moon's Rise

## Reference (Scryfall)
- **Name:** Full Moon's Rise
- **Cost:** {1}{G}
- **Type:** Enchantment
- **Oracle:** Werewolf creatures you control get +1/+0 and have trample. Sacrifice Full Moon's Rise: Regenerate all Werewolf creatures you control.
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({1}{G})
- Type: CORRECT (Enchantment)
- Oracle text: PARTIALLY INCORRECT -- the implementation oracle text says "Werewolf creatures" for the static ability but also says "Regenerate all Werewolf creatures" for the sacrifice ability, which matches. However, the code comment at the top says "Werewolf and Wolf creatures" which does not match Oracle.
- P/T: CORRECT (N/A)
- +1/+0 to Werewolf creatures: CORRECT (continuous effect ModifyPT with HasSubtype("Werewolf"))
- Trample to Werewolf creatures: CORRECT (GrantKeyword Trample with HasSubtype("Werewolf"))
- Sacrifice ability: CORRECT (SacrificeCost::SacrificeThis)
- Regeneration effect: The on_activate_ability only regenerates creatures with "Werewolf" subtype.

## Issues
**ISSUE: Code comment says "Werewolf and Wolf" but Oracle only says "Werewolf".** The doc comment at line 9 says "Werewolf and Wolf creatures" but the actual Scryfall oracle text only says "Werewolf creatures." The continuous_effects correctly only filter for HasSubtype("Werewolf"). The activated ability description also mentions "Wolf and Werewolf" but the actual filter in on_activate_ability only checks for "Werewolf" -- so the code behavior is correct, but the comments/descriptions are misleading.
