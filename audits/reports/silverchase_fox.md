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

## Audit — 2026-04-02

**Oracle Text:**
> {1}{W}, Sacrifice this creature: Exile target enchantment.

**Card Data:**
- Name: Silverchase Fox — correct
- Cost: {1}{W} — correct
- Type: Creature — Fox — correct
- P/T: 2/2 — correct

**Behavior:**
- Activated ability costs {1}{W} with SacrificeCost::SacrificeThis — correct
- Targets a permanent with CardType::Enchantment — correct
- On activation, exiles the target from battlefield — correct

**Result: PASS**
