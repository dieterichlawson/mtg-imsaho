# Audit: Brain Weevil

## Oracle (Scryfall/API)
- **Name:** Brain Weevil
- **Cost:** {3}{B}
- **Type:** Creature — Insect
- **Oracle:** Intimidate. Sacrifice Brain Weevil: Target player discards two cards. Activate only as a sorcery.
- **P/T:** 1/1

## Implementation: `brain_weevil.rs`
- **Name:** Brain Weevil -- CORRECT
- **Cost:** {3}{B} -- CORRECT
- **Type:** Creature — Insect -- CORRECT
- **P/T:** 1/1 -- CORRECT
- **Keywords:** Intimidate -- CORRECT
- **Activated ability:** Sacrifice self, target player discards 2, sorcery speed -- CORRECT
- **Sacrifice cost:** SacrificeCost::SacrificeThis -- CORRECT
- **Target:** PlayerOnly -- CORRECT
- **Discard handling:** Handles <= 2 cards (auto-discard) and > 2 cards (player choice) -- CORRECT

## Verdict: PASS -- No issues found
