# TODO

## Game state serialization
Add the ability to serialize a game state to a file and resume from it. This would let us set up specific board/hand/mana configurations to test particular interactions (e.g. Counterspell with 2 untapped Islands and an opponent's spell on the stack) without relying on RNG to produce the right conditions.

---

# Audit Bug List

All bugs found in the full Scryfall-verified audit of 270 Innistrad cards.
Each must be fixed — no shortcuts, no deferred work.

## Already Fixed This Session
- [x] Skaab Ruinator — cast from graveyard (added can_cast_from_graveyard)
- [x] Stony Silence — artifact ability restriction (added legal_actions check)
- [x] Blazing Torch — missing damaged_by on creature damage
- [x] Daybreak Ranger — missing "Ranger" subtype
- [x] Ludevic's Test Subject — missing "Egg" subtype on front face
- [x] Mausoleum Guard — Spirit tokens lack "Spirit" subtype
- [x] Midnight Haunting — Spirit tokens lack "Spirit" subtype
- [x] Harvest Pyre — missing damaged_by tracking

## Real Bugs — Card-Level Fixes (no engine work needed)
- [x] **Civilized Scholar** — Homicidal Brute doesn't tap before transforming back. Add `obj.tapped = true` before transform. (Already fixed)
- [x] **Gutter Grime** — Ooze token P/T should be dynamic (equal to slime counters on Gutter Grime), not static.
- [x] **Kruin Outlaw** — Back face (Terror of Kruin Pass) menace only on self. Should be a global continuous effect: "Each Werewolf you control can't be blocked except by two or more creatures."
- [x] **Mayor of Avabruck** — Back face (Howlpack Alpha) missing Upkeep trigger metadata in triggered_abilities. The on_upkeep is implemented but never called because no TriggerKind::Upkeep is declared on back face. (Already fixed — back face has both Upkeep and EndStep triggers declared)
- [x] **Moldgraf Monstrosity** — Creature selection from graveyard is deterministic (first 2). Oracle says random. Use rand. (Already fixed — uses rand::seq::SliceRandom)
- [x] **Olivia Voldaren** — (a) Steal effect should end when Olivia leaves the battlefield. (b) Ability 1 should filter targets to Vampires only.

## Real Bugs — Need Engine Work

### Graveyard/Exile Targeting System
The engine cannot target cards in graveyards or exile. These cards all need it:
- [x] **Ghoulcaller's Chant** — needs mode selection + graveyard creature targeting (Added ModalChoice, GraveyardCreature, GraveyardCreatureOfSubtype to TargetRequirement)
- [x] **Graveyard Shovel** — target player exiles a card from their graveyard (player choice). Fixed: targets PlayerOnly, targeted player chooses card via ResolutionChoice.
- [x] **Memory's Journey** — target player shuffles up to 3 target graveyard cards into library. Fixed: cards must come from one player's graveyard using ModalChoice + GraveyardCardOwnedByCaster/Opponent.
- [x] **Purify the Grave** — exile target card from a graveyard (any graveyard, player choice). Already working with GraveyardCard targeting.
- [x] **Runic Repetition** — return target exiled flashback card to hand. Already working with ExileCard targeting + flashback filter.

### End of Combat Trigger
- [x] **Geist of Saint Traft** — Angel token should be exiled at end of combat, not end step. Needs TriggerKind::EndCombat + PendingTrigger::EndCombatTrigger + on_end_combat hook + StepStarted::EndCombat processing. (Already implemented — EndCombat trigger system exists in engine and card uses on_end_combat hook)

### Combat Damage to Creature Trigger
- [x] **Creepy Doll** — currently fires at block declaration. Should fire when combat damage is actually dealt to a creature. Needs TriggerKind::DealsCombatDamageToCreature or similar. (Fixed: added DealsCombatDamageToCreature trigger kind + PendingTrigger::CombatDamageToCreature + on_deals_combat_damage_to_creature hook)

### Mana Ability Callback
- [ ] **Deranged Assistant** — mill cost never executed. Needs on_activate_mana_ability callback in CardBehavior + engine to call it during ActivateManaAbility handling.

### Damage Prevention / Replacement
- [ ] **Moonmist** — "Prevent all combat damage that non-Wolf and non-Werewolf creatures would deal this turn." Needs a combat damage prevention system (per-turn flags checked during deal_combat_damage).
- [ ] **Unbreathing Horde** — "If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter." Needs per-creature damage interception.

### Player Hexproof
- [ ] **Witchbane Orb** — "You have hexproof." Needs player targeting restriction in legal_actions (skip spells/abilities targeting a player with hexproof).

### Planeswalker Damage Redirect
- [ ] **Curse of the Pierced Heart** — Oracle says "deals 1 damage to that player or a planeswalker that player controls." Needs damage redirect to planeswalker option.

### Multi-Step Casting (Additional Costs at Cast Time)
- [ ] **Infernal Plunge** — sacrifice should happen when casting, not at resolution. Needs the casting flow to prompt for additional costs before the spell goes on the stack.

### X-Cost Activated Abilities
- [ ] **Kessig Wolf Run** — X-cost ability simplified to fixed cost. Needs X-cost support for activated abilities (player chooses X, pays accordingly).

### Double Damage Replacement
- [ ] **Inquisitor's Flail** — combat damage doubling is approximated via power boost. Needs actual damage multiplication in combat damage step. Also missing defensive doubling entirely.

### Modal Spells
- [ ] **Creeping Renaissance** — "Choose a permanent type." Needs a mode/type selection UI so the player picks creature/artifact/enchantment/land/planeswalker.

### Garruk Back Face
- [ ] **Garruk Relentless** — back face (Garruk, the Veil-Cursed) abilities entirely unimplemented. Needs: loyalty ability implementations for the back face (-1 create Wolf with deathtouch, -1 sacrifice creature to tutor, -3 creatures get +X/+X trample).
