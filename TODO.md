# TODO

## Game state serialization
Add the ability to serialize a game state to a file and resume from it. This would let us set up specific board/hand/mana configurations to test particular interactions (e.g. Counterspell with 2 untapped Islands and an opponent's spell on the stack) without relying on RNG to produce the right conditions.

---

# Audit Bug List

All bugs found in the full Scryfall-verified audit of 270 Innistrad cards.

## Card-Level Bugs

- [ ] **Blazing Torch** — Damage source is the torch (`object_id`) instead of the equipped creature. Oracle says "Equipped creature deals 2 damage to any target" so `damaged_by` and the `NonCombatDamageDealt` event source should reference the attached creature, not the torch itself.

- [ ] **Civilized Scholar** — (a) Auto-picks a creature card for discard (lines 110-115) instead of letting the player choose. Oracle says "discard a card" with transform trigger "if a creature card is discarded this way." (b) Back face (Homicidal Brute) has empty `triggered_abilities` vec despite having an `on_end_step` hook — needs `TriggerKind::EndStep` declared.

- [ ] **Garruk Relentless** — (a) Front face fight ability auto-targets the strongest opponent creature instead of letting the controller choose. Oracle says "target creature." (b) Transform condition ("when Garruk has two or fewer loyalty counters") is a state-triggered ability but only checks after `on_loyalty_ability`. Won't trigger from combat damage or other non-loyalty sources. (c) Creature-to-planeswalker damage in the fight ability doesn't emit a `NonCombatDamageDealt` event — just decrements loyalty directly.

- [ ] **Geist of Saint Traft** — Angel token tracking uses `card_state.insert("angel_token", id)` which overwrites on multiple attacks in the same combat (e.g., with extra combat steps). Should use a vec or unique keys per token.

- [ ] **Ghoulcaller's Chant** — Mode 2 uses `GraveyardCreatureOfSubtype("Zombie")` but oracle says "two target Zombie cards" (not "Zombie creature cards"). Could miss non-creature Zombies in graveyard. Very minor since Zombies are almost always creatures.

- [ ] **Harvest Pyre** — Exiles ALL cards from controller's graveyard at resolve (lines 44-47) instead of letting the player choose X cards to exile. Oracle says "Exile X cards from your graveyard" where X determines the damage dealt. Player should choose both X and which cards.

- [ ] **Kruin Outlaw / Terror of Kruin Pass** — (a) Back face uses `MinimumBlockers` effect instead of the menace keyword. (b) Oracle says "Each Werewolf you control" has menace — should be a global continuous effect on all your Werewolves, not just self. (c) Back face `triggered_abilities` vec is empty despite needing `TriggerKind::Upkeep` for the transform-back check.

- [ ] **Memory's Journey** — Target player selection is implicit (auto-derived from modal choice of "your graveyard" vs "opponent's graveyard") rather than explicit targeting. Oracle says "target player shuffles up to three target cards from their graveyard into their library" — the player should be a target.

- [ ] **Moonmist** — Transform filter uses `!o.is_transformed` (line 34) which prevents already-transformed Werewolves from transforming back. Oracle says "Transform all Humans" — should transform any Human regardless of current face. Also may incorrectly transform non-Werewolf DFCs that happen to be Human.

- [ ] **Olivia Voldaren** — Ability 0 (ping for 1 damage) uses `TargetFilter::Any` which could allow targeting Olivia herself. Oracle says "target creature" with no self-exclusion, but the +1/+1 counter and Vampire-making only apply to "that creature" if it survives, so self-targeting would be odd but technically legal. Low severity.

## Bugs Needing Engine Work

- [ ] **Skaab Ruinator** — "As an additional cost to cast this spell, exile three creature cards from your graveyard." Currently exiles at resolve time (`on_resolve` lines 41-58), not at cast time. Needs the additional-cost-at-cast-time system to handle exile-from-graveyard costs (similar to how `Infernal Plunge` sacrifice was fixed).

- [ ] **Stony Silence** — "Activated abilities of artifacts can't be activated." The static ability is declared but not enforced in `legal_actions`. Needs engine support to check for artifact-ability-restriction effects when generating legal actions for activated abilities.
