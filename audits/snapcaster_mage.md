# Audit: Snapcaster Mage

## Oracle (Scryfall)
- **Name:** Snapcaster Mage
- **Cost:** {1}{U}
- **Type:** Creature -- Human Wizard
- **Oracle:** Flash. When Snapcaster Mage enters the battlefield, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.
- **P/T:** 2/1

## Implementation: `mtg-engine/src/cards/snapcaster_mage.rs`
- **Name:** Snapcaster Mage ✅
- **Cost:** {1}{U} ✅
- **Type:** Creature ✅
- **Subtypes:** Human, Wizard ✅
- **P/T:** 2/1 ✅
- **Keywords:** Flash ✅
- **Triggered ability:** EntersBattlefield ✅
- **on_enter_battlefield:** finds instant/sorcery in graveyard without flashback ✅
- **Flashback cost:** uses the card's mana cost ✅
- **until_end_of_turn_flashback:** correctly stored ✅

### Issue
- **SIMPLIFICATION:** The target is auto-selected (highest mana value card) rather than letting the player choose. The oracle says "target" which normally means the player picks. This is a minor simplification -- the AI picks the most expensive card, which is usually but not always correct.

## Verdict: PASS -- minor simplification in target selection

## Audit — 2026-04-02

**Oracle Text:**
> Flash
> When this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.

**Card Data:**
- Name: Snapcaster Mage — correct
- Cost: {1}{U} — correct
- Type: Creature — Human Wizard — correct
- P/T: 2/1 — correct
- Keywords: Flash — correct
- Triggered ability: ETB — correct

**Behavior:**
- ISSUE: The oracle says "target instant or sorcery card in your graveyard" — the player should choose which card to target. The implementation auto-selects the card with the highest mana value (`max_by_key`), giving the player no choice. This is incorrect when there are multiple eligible cards in the graveyard.
- Grants flashback with cost equal to the card's mana cost via `until_end_of_turn_flashback` — correct
- Skips cards that already have flashback — correct

**Result: ISSUE** — No player targeting: auto-selects highest mana value instant/sorcery instead of letting the player choose the target.
