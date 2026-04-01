## Audit — 2026-04-01

**Scryfall Oracle text**: If a Zombie you control would deal combat damage to a player, instead that player mills that many cards.\nWhenever a creature card is put into an opponent's graveyard from their library, exile that card and create a 2/2 black Zombie creature token.
**Scryfall type line**: Creature — Zombie
**Scryfall mana cost**: {3}{U}
**Scryfall P/T**: 4/2
**Status**: ISSUE

Findings:
- Name: Correct.
- Mana cost: {3}{U} — correct.
- Types: Creature — Zombie — correct.
- P/T: 4/2 — correct.
- Replacement effect (damage to mill): The implementation "restores" life after damage is dealt (`current_life + amount`). This is an approximation — the Oracle text says "instead", meaning damage should never be dealt at all. This matters for damage triggers (e.g., lifelink on the Zombie would not apply since damage is replaced, but the implementation deals damage first then heals).
- Mill + exile + token creation: Correctly implemented — mills cards, checks for creature cards, exiles them, and creates 2/2 black Zombie tokens for each.
- **ISSUE: The triggered ability scope is too narrow.** The Oracle text says "Whenever a creature card is put into an opponent's graveyard from their library" — this is a general triggered ability that should trigger on ANY mill effect, not just mill from Undead Alchemist's own replacement. The current implementation only exiles/creates tokens for cards milled by its own replacement effect within `on_any_combat_damage_to_player`, not for cards milled by other effects (e.g., Curse of the Bloody Tome).
- **ISSUE: Replacement effect is applied after damage rather than replacing it.** The life restoration hack doesn't prevent damage-related triggers from firing.
- Tests: `undead_alchemist_mills_instead_of_damage` covers the basic combat replacement path.
