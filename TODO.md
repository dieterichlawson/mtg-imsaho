# TODO

## Game state serialization
Add the ability to serialize a game state to a file and resume from it. This would let us set up specific board/hand/mana configurations to test particular interactions (e.g. Counterspell with 2 untapped Islands and an opponent's spell on the stack) without relying on RNG to produce the right conditions.

## Audit issues

### Liliana of the Veil (5 issues)
- +1: auto-picks first card from hand instead of letting each player choose their discard
- -2: no targeting (should target a player), auto-picks creature to sacrifice instead of letting the targeted player choose
- -6: completely simplified — no pile division or player choice, just sacrifices ~half of opponent's permanents
- -6: `half.max(1)` forces at least 1 sacrifice; per rulings a pile can be empty
- Oracle text field says "pile of your choice" but oracle says "pile of their choice"

### Grimgrin, Corpse-Born (5 issues)
- Sacrifice ability auto-selects creature instead of letting player choose
- Sacrifice cost not declared in ActivatedAbilityDef (uses SacrificeCost::None)
- Attack trigger auto-targets instead of letting player choose which creature to destroy
- +1/+1 counter added unconditionally even when attack trigger has no valid target
- Attack trigger uses `state.opponent()` instead of combat state for defending player

### Instigator Gang (1 issue — engine-level)
- Doesn't buff itself when attacking — `AnyCreatureAttacks` watcher in `triggers.rs:708` excludes the attacker from seeing its own attack event. Affects both Instigator Gang (+1/+0) and Wildblood Pack (+3/+0).

### Delver of Secrets (1 issue)
- "You may reveal" choice not presented to player — auto-transforms when top card is instant/sorcery

### Screeching Bat (1 issue)
- "You may" upkeep transform auto-decided — always pays and transforms when mana available

### Cloistered Youth (3 issues)
- "You may" upkeep transform auto-decided — always transforms with no player choice
- Front face declares EndStep triggered_ability that belongs only to back face (Unholy Fiend), causing phantom trigger
- Back face declares empty triggered_abilities, so Upkeep trigger dispatches to front face logic spuriously

### Fiend Hunter (1 issue)
- LLM card knowledge (`mtg-player/src/llm.rs:102`) says "exiles an opponent's creature" but card targets any creature and exile is optional ("you may")

### Grimoire of the Dead (2 issues)
- Discard auto-selects first card from hand instead of letting player choose
- Study counters stored as `card_state` hack instead of proper counter system (invisible to proliferate etc.)
