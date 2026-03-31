# Audit: Silverchase Fox

## Oracle (Scryfall)
- **Name:** Silverchase Fox
- **Cost:** {1}{W}
- **Type:** Creature -- Fox
- **Oracle:** {1}{W}, Sacrifice Silverchase Fox: Exile target enchantment.
- **P/T:** 2/2

## Implementation: `mtg-engine/src/cards/silverchase_fox.rs`
- **Name:** Silverchase Fox ✅
- **Cost:** {1}{W} ✅
- **Type:** Creature ✅
- **Subtypes:** Fox ✅
- **P/T:** 2/2 ✅
- **Activated ability cost:** {1}{W}, SacrificeSelf ✅
- **Target:** PermanentWithFilter(HasCardType(Enchantment)) ✅
- **Effect:** exiles target enchantment from battlefield ✅
- **Instant speed:** sorcery_speed_only: false ✅

## Verdict: PASS -- no issues found
