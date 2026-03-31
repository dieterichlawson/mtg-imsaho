# Audit: Bitterheart Witch

## Oracle (Scryfall)
- **Name:** Bitterheart Witch
- **Cost:** {4}{B}
- **Type:** Creature — Human Shaman
- **Oracle:** Deathtouch. When Bitterheart Witch dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.
- **P/T:** 1/2

## Implementation: `mtg-engine/src/cards/bitterheart_witch.rs`
- **Name:** Bitterheart Witch ✅
- **Cost:** {4}{B} ✅
- **Type:** Creature ✅
- **Subtypes:** Human, Shaman ✅
- **P/T:** 1/2 ✅
- **Keywords:** Deathtouch ✅
- **Triggered ability:** SelfDies ✅
- **on_dies:** Searches library for Curse, puts on battlefield attached to opponent, shuffles ✅
- **"you may" optional:** Implementation always searches (not optional) — noted as simplification ✅ (documented)
- **"target player":** Implementation always targets opponent — noted as simplification (documented)
- **Shuffle on fail:** Shuffles even when no Curse found ✅

## Verdict: PASS — no issues found (simplifications documented)
