# ISD Audit Issues — Sonnet 4.6 (2026-04-04)

Total: 699 issues across 136 cards

## Legend
- **VERIFIED**: Confirmed real issue by reading the code
- **FALSE POSITIVE**: Not actually an issue
- **NEEDS REVIEW**: Uncertain, requires human judgment

## abattoir_ghoul

- [ ] **Engine bug: DeathWatch trigger never collected when Ghoul and victim die simultaneously** (`mtg-engine/src/triggers.rs` lines 418–419)

- [ ] Oracle text says: `"Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness."` — no condition requiring the Ghoul to be on the battlefield at

- [ ] Code does: `state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.id != dead_id)` — watcher scan runs inside `collect_triggers`, which is called AFTER all SBA deaths have been applied. Wh

- [ ] **Engine bug: DeathWatch trigger cancelled at resolution if Ghoul left battlefield** (`mtg-engine/src/triggers.rs` lines 906–912)

- [ ] Oracle text says: `"Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness."` — no intervening-if clause.

- [ ] Code does: `if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { behavior.on_any_creature_dies(...) }` — if the trigger IS collected (Ghoul was on battlefield when `

## angelic_overseer

- [ ] Sequential SBA processing allows a protecting Human to die before Angelic Overseer's indestructibility is evaluated, causing Angelic Overseer to be incorrectly destroyed.

- [ ] Oracle text says: `"As long as you control a Human, this creature has hexproof and indestructible."` (and ruling: `"If you control a Human, and an effect tries to destroy each Human you control and An

- [ ] Code does: In `mtg-engine/src/sba.rs` lines 101–147, the `destroyed_ids` list is populated before any deaths occur, but the indestructibility check (`try_destroy` → `has_keyword(Indestructible)` → `ch

## armored_skaab

- [ ] ETB trigger suppressed when source leaves battlefield before resolution (`mtg-engine/src/triggers.rs`, lines 893–899)

- [ ] Oracle text says: `When this creature enters, mill four cards.`

- [ ] Code does: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { if let Some(behavior) = registry.get(card_id) { behavior.on_enter_battlefield(state, object_id, regis

## avacynian_priest

- [ ] Engine does not enforce summoning sickness for `{T}` activated abilities; a freshly entered Avacynian Priest can activate its tap ability on the same turn it enters the battlefield.

- [ ] Oracle text says: `{1}, {T}: Tap target non-Human creature.` — the `{T}` symbol in the cost means the ability cannot be activated while the creature has summoning sickness (CR 302.6: "A creature's act

- [ ] Code does: `mtg-engine/src/engine.rs` line 356: `if ab.requires_tap && obj_tapped { continue; }` — only skips when already tapped; there is no corresponding check for `obj.summoning_sick`. A creature 

## back_from_the_brink

- [ ] Token copies are created with no colors (`Vec::new()`) in `state.rs`

- [ ] Oracle text says: `Create a token that's a copy of that card.`

- [ ] Code does: In `state.rs` line 425, `create_token_copy` passes `Vec::new()` for colors with a `// colors TODO` comment: `Vec::new(), // colors TODO`. The source object's `.colors` field is read for `na

## balefire_dragon

- [ ] Battlefield guard in `on_combat_damage_to_player` suppresses the triggered effect if Balefire Dragon has left the battlefield at resolution time (`mtg-engine/src/cards/isd/balefire_dragon.rs`, lines 4

- [ ] Oracle text says: `Whenever Balefire Dragon deals combat damage to a player, it deals that much damage to each creature that player controls.`

- [ ] Code does: `if !state.get_object(self_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { return; }` — this early-return silently drops the entire effect if the dragon is no longer on the batt
  Per MTG CR 112.7a, once a triggered ability is on the stack it exists independently of its source; destroying or removing the source does not affect the ability. The oracle text has no "if Balefire Dr
  In the current engine, `resolve_next_trigger` for `CombatDamageToPlayer` (triggers.rs lines 921–924) correctly omits a battlefield check at the engine level, delegating entirely to the card handler. T

## bitterheart_witch

- [ ] `present_player_choice` builds target list without hexproof filtering (`mtg-engine/src/cards/isd/bitterheart_witch.rs:14-16`) and the corresponding `ChooseCurseThenAttach` path in the engine has the s

- [ ] Oracle text says: `"put it onto the battlefield attached to target player"` — "target player" means the player must be a legal target. The Scryfall ruling states: `"The Curse must be legally able to e

- [ ] Code does: `let player_targets: Vec<crate::actions::Target> = (0..state.players.len()).map(|i| crate::actions::Target::Player(PlayerId(i as u8))).collect();` — all players unconditionally. Neither `ca

## blazing_torch

- [ ] `AnyTarget` engine implementation does not include planeswalkers as valid targets

- [ ] Oracle text says: `"Blazing Torch deals 2 damage to any target."`

- [ ] Code does: `generate_ability_targets` for `TargetRequirement::AnyTarget` (engine.rs lines 1343–1358) only generates creatures (`o.power.is_some()`) and players. Planeswalkers — which are not creatures

## bloodcrazed_neonate

- [ ] Forced-attack logic in `engine.rs` (~line 1838) does not call `state.can_attack()`, so a Bloodcrazed Neonate enchanted with Pacifism (or any `PreventAttack` effect) is still force-added to combat even

- [ ] Oracle text says: `"This creature attacks each combat if able."`

- [ ] Code does: iterates battlefield creatures, checks `zone`, `controller`, `power.is_none()`, `tapped`, `summoning_sick`, and `Keyword::Defender`, but **never calls `new_state.can_attack(creature.id, reg

## bloodgift_demon

- [ ] Upkeep trigger incorrectly fizzles if Bloodgift Demon leaves the battlefield after its trigger is on the stack but before it resolves.

- [ ] Oracle text says: `"At the beginning of your upkeep, target player draws a card and loses 1 life."`

- [ ] Code does: In `mtg-engine/src/triggers.rs` lines 954–959, `resolve_next_trigger` wraps the entire `UpkeepTrigger` resolution in a battlefield check: `if state.get_object(object_id).map(|o| o.zone == Z

## bonds_of_faith

- [ ] **"as long as it's a Human" condition is snapshotted at ETB, never re-evaluated** — `mtg-engine/src/cards/isd/bonds_of_faith.rs` lines 39–69

- [ ] Oracle text says: `"Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block."`

- [ ] Code does: `on_enter_battlefield` checks the Human subtype once at entry time and writes a fixed `instance_continuous_effects` (`[ModifyPT { +2/+2 }]` for Human, `[PreventAttack, PreventBlock]` for no

- [ ] **Human subtype check at ETB only inspects registry data, missing object-level subtypes (tokens)** — `mtg-engine/src/cards/isd/bonds_of_faith.rs` lines 43–46

- [ ] Oracle text says: `"as long as it's a Human"`

- [ ] Code does: `state.get_object(target_id).and_then(|o| registry.card_data(o.card_id)).map(|d| d.subtypes.iter().any(|s| s == "Human")).unwrap_or(false)` — tokens use `card_id = CardId(0)` for which `reg

- [ ] **`oracle_text` field is missing the "Enchant creature" first line** — `mtg-engine/src/cards/isd/bonds_of_faith.rs` line 25

- [ ] Oracle text says: `"Enchant creature\nEnchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block."`

- [ ] Code does: `oracle_text: "Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.".into()` — the "Enchant creature" first line is absent. Other auras in the same se

## boneyard_wurm

- [ ] Graveyard zone display shows base P/T (0/0) instead of dynamically computed P/T

- [ ] Oracle text says: `Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.`

- [ ] Ruling says: `The ability that defines Boneyard Wurm's power and toughness works in all zones, not just the battlefield. If Boneyard Wurm is in your graveyard, it will count itself.`

- [ ] Code does: `card_view` in `mtg-engine/src/view.rs:221` uses `power: obj.power` for graveyard (and hand/exile) objects, which returns the raw base `Some(0)` instead of calling `state.effective_power(ob

- [ ] The underlying `dynamic_pt` function (`boneyard_wurm.rs:32–39`) is itself correct: it does not zone-restrict, and `objects_in_zone(Zone::Graveyard, controller)` would include the Wurm itself when it i

- [ ] Affected file: `mtg-engine/src/view.rs`, function `card_view` (line ~213), specifically `power: obj.power` at line 221.

## brain_weevil

- [ ] Incomplete discard when target player has 3+ cards in hand — `mtg-engine/src/cards/isd/brain_weevil.rs:64-75` + `mtg-engine/src/engine.rs:2009-2023`

- [ ] Oracle text says: `Target player discards two cards.`

- [ ] Code does: When the target player has 3 or more cards in hand, `on_activate_ability` sets up a single `ChooseCardFromHand` prompt (described as "1 of 2"), but `on_discard_choice` is never implemented 
  Concretely, engine.rs lines 2009–2023 handle `ChooseCardFromHand` resolution:
  ```
  behavior.on_discard_choice(&mut new_state, choice_source, *discard_id, registry);

## brimstone_volley

- [ ] `AnyTarget` in engine does not include planeswalkers as valid targets — `mtg-engine/src/engine.rs` lines 836–864, 1074–1089, 1343–1358

- [ ] Oracle text says: `"deals 3 damage to any target"` — per MTG rules "any target" means any creature, player, or planeswalker.

- [ ] Code does: All three `AnyTarget` branches in `generate_cast_actions_with_targets` and the two helper functions filter objects with `o.power.is_some()` (creatures only) and add players, but never inclu

## bump_in_the_night

- [ ] LLM player card knowledge is missing flashback ability (`mtg-player/src/llm.rs` line 84)

- [ ] Oracle text says: `Flashback {5}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)`

- [ ] Code does: `"- Bump in the Night ({B} sorcery): Target opponent loses 3 life."` — no flashback cost or graveyard-cast information listed. Every other flashback card in the same file (Think Twice, Drea

- [ ] `oracle_text` field in `card_data()` is incomplete (`mtg-engine/src/cards/isd/bump_in_the_night.rs` line 23)

- [ ] Oracle text says: `Target opponent loses 3 life.\nFlashback {5}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)`

- [ ] Code does: `oracle_text: "Target opponent loses 3 life.".into()` — the flashback reminder line is absent. This field is rendered verbatim to human players via the CLI card display (`mtg-player/src/cli

## burning_vengeance

- [ ] **Engine bug: `SpellCast` trigger dispatch restricted to instant/sorcery** — `mtg-engine/src/triggers.rs` lines 644–675

- [ ] Oracle text says: `Whenever you cast a spell from your graveyard`

- [ ] Code does: `let is_instant_sorcery = ... .map(|d| d.card_types.iter().any(|ct| matches!(ct, crate::types::CardType::Instant | crate::types::CardType::Sorcery))).unwrap_or(false); if is_instant_sorcery

- [ ] **Card bug: checks `cast_with_flashback` rather than "cast from graveyard"** — `mtg-engine/src/cards/isd/burning_vengeance.rs` lines 48–53

- [ ] Oracle text says: `Whenever you cast a spell from your graveyard`

- [ ] Code does: `let cast_from_gy = state.get_object(spell_id).map(|o| o.cast_with_flashback).unwrap_or(false); if !cast_from_gy { return; }` — The variable is named `cast_from_gy` but reads the `cast_with

- [ ] **Log message logged before target is chosen, and describes "opponent" inaccurately** — `mtg-engine/src/cards/isd/burning_vengeance.rs` lines 67–69

- [ ] Oracle text says: `this enchantment deals 2 damage to any target`

- [ ] Code does: `state.log(..., format!("Burning Vengeance deals 2 damage to opponent (flashback spell cast)"))` — This log line is written after `present_target_choice`, which (when there are multiple tar

## butchers_cleaver

- [ ] **Snapshot "as long as" — lifelink not re-evaluated when equipped creature transforms** (`mtg-engine/src/cards/isd/butchers_cleaver.rs`, lines 14–34 and 84–92)

- [ ] Oracle text says: `As long as equipped creature is a Human, it has lifelink.`

- [ ] Code does: `update_effects` is called exactly once in `on_activate_ability` when the equip ability resolves. It evaluates the Human condition at that moment and writes a static `instance_continuous_ef

- [ ] **Human check ignores transformed state — back-face DFCs incorrectly identified as Human at equip time** (`mtg-engine/src/cards/isd/butchers_cleaver.rs`, lines 15–18)

- [ ] Oracle text says: `As long as equipped creature is a Human, it has lifelink.`

- [ ] Code does:
  ```rust
  let is_human = state.get_object(creature_id)
  .and_then(|o| registry.card_data(o.card_id))

- [ ] **Human check ignores runtime object subtypes — Human tokens never get lifelink** (`mtg-engine/src/cards/isd/butchers_cleaver.rs`, lines 15–18)

- [ ] Oracle text says: `As long as equipped creature is a Human, it has lifelink.`

- [ ] Code does: the same `registry.card_data(o.card_id)` call. Tokens use a sentinel `CardId(0)`; `registry.card_data(CardId(0))` returns `None` (no registry entry for the sentinel), so the chain short-cir

## cackling_counterpart

- [ ] **Colors never copied when creating token copy** — `mtg-engine/src/state.rs`, `create_token_copy` function (line 426)

- [ ] Oracle text says: `"Create a token that's a copy of target creature you control."` — a copy must have all the copiable values of the source, including color.

- [ ] Code does: `Vec::new(), // colors TODO` — colors are hardcoded to empty regardless of the source creature's colors. In a real game, source creatures have colors derived from their mana cost (set in `s

- [ ] **Copying a token source loses its card_types, keywords, and subtypes** — `mtg-engine/src/state.rs`, `create_token_copy` function (lines 424–431)

- [ ] Oracle text says: `"Create a token that's a copy of target creature you control."` + Ruling: `"If the copied creature is a token, the token that's created copies the original characteristics of that t

- [ ] Code does: `let (colors, keywords, card_types, subtypes) = registry.card_data(card_id).map(...).unwrap_or_default();` — for token sources, `card_id = CardId(0)` (the sentinel set in `create_token_inte

## caravan_vigil

- [ ] Auto-selects first basic land in library order instead of presenting a player search choice (`mtg-engine/src/cards/isd/caravan_vigil.rs` lines 39–50)

- [ ] Oracle text says: `"Search your library for a basic land card, reveal it, put it into your hand, then shuffle."`

- [ ] Code does: `let basic_land = player.library_order.iter().find(|&&obj_id| { ... }).copied();` — this blindly picks the first matching basic land in library order, never presenting the player with a cho

## charmbreaker_devils

- [ ] Upkeep trigger fires spuriously on the stack during the opponent's upkeep

- [ ] Oracle text says: `At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.`

- [ ] Code does: `triggers.rs` lines 597–639 dispatch `UpkeepTrigger` for **all** battlefield permanents that have an Upkeep trigger description whenever any `StepStarted { step: Upkeep }` event fires, with

- [ ] SpellCast trigger fires spuriously on the stack when the opponent casts an instant or sorcery

- [ ] Oracle text says: `Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.`

- [ ] Code does: `triggers.rs` lines 644–676 dispatch `SpellCastWatch` for **all** battlefield permanents that have a SpellCast trigger description whenever any instant or sorcery is cast by any player. Cha

## civilized_scholar

- [ ] **Stale `attacked_this_turn` flag causes Homicidal Brute to skip transform-back** (`mtg-engine/src/cards/isd/civilized_scholar.rs`, lines 166–191)

- [ ] Oracle text says: `"At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it."` — "this turn" means the current controller's turn; the conditio

- [ ] Code does: `on_end_step` guards with `if !is_transformed || state.active_player != controller { return; }` and places the `card_state.remove("attacked_this_turn")` clear **after** that guard. When the

- [ ] **EndStep trigger registered on front face, causing spurious stack entry when not transformed** (`mtg-engine/src/cards/isd/civilized_scholar.rs`, lines 38–48)

- [ ] Oracle text says: Civilized Scholar's (front face) oracle text is `"{T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it."` — no end-

- [ ] Code does: `card_data().triggered_abilities` lists `TriggerKind::EndStep` on the front face (`triggered_abilities: vec![TriggeredAbilityDef { kind: TriggerKind::Attacks, ... }, TriggeredAbilityDef { k

## clifftop_retreat

- [ ] `controller_has_matching_land` only checks runtime object subtypes (`o.subtypes`), never the registry — so real Mountain and Plains cards are never detected, causing Clifftop Retreat to always enter t

- [ ] Oracle text says: `This land enters tapped unless you control a Mountain or a Plains.`

- [ ] Code does: `o.subtypes.iter().any(|s| s == "Mountain") || o.subtypes.iter().any(|s| s == "Plains")` — but `o.subtypes` is always `Vec::new()` for non-token cards because `setup_game` (`engine.rs` line

## cloistered_youth

- [ ] Spurious upkeep trigger fires for Unholy Fiend (transformed state)

- [ ] Oracle text says: `At the beginning of your end step, you lose 1 life.` (Unholy Fiend back face has NO upkeep ability)

- [ ] Code does: `trigger_description` in `mtg-engine/src/triggers.rs` line 314 always checks front face triggers first regardless of `is_transformed`. When the permanent is transformed (`is_transformed=tru

## corpse_lunge

- [ ] Engine auto-selects the exiled creature without presenting a player choice

- [ ] Oracle text says: `"As an additional cost to cast this spell, exile a creature card from your graveyard."`

- [ ] Code does: `engine.rs:1574–1584` — `// Pick highest-power creatures first (better default for Corpse Lunge). … exile_candidates.sort_by(|a, b| b.1.cmp(&a.1)); // Highest power first` — the engine unco

- [ ] Test `corpse_lunge_picks_highest_power_creature` (tier8_cards.rs:538) enshrines the wrong auto-selection behavior

- [ ] Oracle text says: `"exile a creature card from your graveyard"` (player chooses which one)

- [ ] Code does: the test asserts `assert_eq!(big_obj.zone, Zone::Exile, "Highest-power creature should be exiled")` — it verifies and locks in the auto-selection of the highest-power creature as the only p

## creepy_doll

- [ ] **Lethal-damage + regeneration scenario: winning the coin flip fails to destroy the creature** (`mtg-engine/src/engine.rs` lines 3118–3125, interacting with `mtg-engine/src/triggers.rs` lines 926–931)

- [ ] Oracle text says: `"If you win the flip, destroy that creature."`

- [ ] Second ruling says: `"If the combat damage Creepy Doll deals to a creature is lethal, you'll still flip a coin. If the creature is still on the battlefield (perhaps because it regenerated), it could b

- [ ] Code does: The engine game loop calls `triggers::process_triggers` (which both collects **and resolves** all triggered abilities, including the coin-flip `try_destroy`) **before** it enters the SBA lo

## crossway_vampire

- [ ] ETB trigger is suppressed if Crossway Vampire leaves the battlefield before the trigger resolves.

- [ ] Oracle text says: `When this creature enters, target creature can't block this turn.`

- [ ] Code does: `mtg-engine/src/triggers.rs:893-900` — `resolve_next_trigger` for `PendingTrigger::EnteredBattlefield` contains `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_o

## curse_of_oblivion

- [ ] Upkeep trigger fires during every player's upkeep, not only the enchanted player's upkeep — spurious triggers placed on stack during the non-cursed player's turn.

- [ ] Oracle text says: `"At the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard."`

- [ ] Code does: In `triggers.rs` `collect_triggers()`, the `GameEvent::StepStarted { step: Upkeep }` branch (lines 597–643) iterates over ALL battlefield permanents with a non-empty `TriggerKind::Upkeep` d

## curse_of_the_nightly_hunt

- [ ] The post-DeclareAttackers forced-attack enforcement loop does not check `state.can_attack()`, causing creatures under "can't attack" effects (e.g., Pacifism) to be illegally added to combat.

- [ ] Oracle text says: `"Creatures enchanted player controls attack each combat if able."`

- [ ] Ruling says: `"If, during the enchanted player's declare attackers step, a creature they control is tapped, is affected by a spell or ability that says it can't attack, or hasn't been under that playe

- [ ] Code does: `mtg-engine/src/engine.rs` lines 1825–1847 iterate all battlefield creatures controlled by the active player and filter only for `tapped`, `summoning_sick`, and the `Defender` keyword befor
  By contrast, `combat::eligible_attackers` (used to build the `must_attack` hint shown to the player in `legal_actions`) correctly calls `state.can_attack()` at line 581, so the Pacifism'd creature wou

## curse_of_the_pierced_heart

- [ ] Dealing damage to a planeswalker via the choice path does not remove loyalty counters

- [ ] Oracle text says: `"this Aura deals 1 damage to that player or a planeswalker that player controls"`

- [ ] Code does: when a planeswalker target is chosen, `apply_pending_effect` in `mtg-engine/src/engine.rs` line 2181 executes `obj.damage_marked += amount` on the planeswalker object. No loyalty counters a

- [ ] The upkeep trigger goes on the stack during every player's upkeep, not only the enchanted player's upkeep

- [ ] Oracle text says: `"At the beginning of enchanted player's upkeep"`

- [ ] Code does: `mtg-engine/src/triggers.rs` lines 597–643, `StepStarted::Upkeep` handling collects all battlefield permanents with a non-empty `TriggerKind::Upkeep` description and pushes them onto the st

## darkthicket_wolf

- [ ] `abilities_activated_this_turn` is never cleared between turns — engine bug causes once-per-turn restriction to become once-per-game permanently

- [ ] Oracle text says: `Activate only once each turn.`

- [ ] Code does: `engine.rs:1778` inserts `ability_index` into `obj.abilities_activated_this_turn` when activated, and `engine.rs:358` uses `activated_this_turn.contains(&ab.ability_index)` to suppress the 

## daybreak_ranger

- [ ] Engine never increments `spells_cast_this_turn` or populates `spells_cast_last_turn`, breaking both transform conditions in actual gameplay.

- [ ] Oracle text says (front face): `At the beginning of each upkeep, if no spells were cast last turn, transform this creature.`

- [ ] Oracle text says (back face): `At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.`

- [ ] Code does: `state.spells_cast_last_turn` is declared in `state.rs:131` and read by `daybreak_ranger.rs:15-19`, but no code anywhere in the engine ever increments `state.spells_cast_this_turn` when a s

## dearly_departed

- [ ] **Engine: `AnyCreatureEnters` watcher scan only checks `Zone::Battlefield`; Dearly Departed in the graveyard is never dispatched a trigger** (`mtg-engine/src/triggers.rs:368-369`)

- [ ] Oracle text says: `"As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it."`

- [ ] Code does: `state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.id != *object)` — the watcher collection for `AnyCreatureEnters` exclusively scans `Zone::Battlefield`. Dearly Departed r

- [ ] **Engine: `EnterWatch` trigger resolution also requires watcher to be on `Zone::Battlefield`** (`mtg-engine/src/triggers.rs:914-915`)

- [ ] Oracle text says: `"As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it."`

- [ ] Code does: `if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` — even if an `EnterWatch` trigger for Dearly Departed were somehow queued, the resolve handler would 

## delver_of_secrets

- [ ] **"You may reveal" choice suppressed when top card is not an instant or sorcery** (`mtg-engine/src/cards/isd/delver_of_secrets.rs` lines 104–118)

- [ ] Oracle text says: `"You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature."`

- [ ] Ruling says: `"You may reveal the card even if it's not an instant or sorcery."`

- [ ] Code does: `if top_is_instant_or_sorcery { state.awaiting_action = Some(AwaitingAction::ResolutionChoice { ... }); } // If not an instant or sorcery, nothing happens`

- [ ] The choice to reveal is gated on the top card being an instant or sorcery. The oracle text presents "you may reveal" as an unconditional option; only the *transform* consequence is conditional. When t

- [ ] The test `delver_does_not_transform_when_top_card_is_creature` (line 1027) actively enshrines this wrong behavior: `assert!(state.awaiting_action.is_none(), "No choice should be presented for non-inst

- [ ] **Transformed Insectile Aberration gets a spurious upkeep trigger on the stack** (engine bug in `mtg-engine/src/triggers.rs` lines 311–327, affecting `delver_of_secrets.rs`)

- [ ] Oracle text for Insectile Aberration says: `"Flying"` — no upkeep trigger.

- [ ] Code does: `trigger_description` in `triggers.rs` always checks front-face triggers first regardless of `is_transformed`: `// Check front face triggers. if let Some(t) = behavior.card_data().triggered

## demonmail_hauberk

- [ ] Player cannot choose which creature to sacrifice for the Equip cost

- [ ] Oracle text says: `"Equip—Sacrifice a creature."` (the player pays the cost by sacrificing a creature of their choice)

- [ ] Code does: In `mtg-engine/src/engine.rs` lines 1750–1759, `SacrificeCost::SacrificeCreature` is handled by auto-selecting the first eligible creature from `objects_in_zone(...).iter().find(|o| o.power

- [ ] Additionally, in `legal_actions` (engine.rs lines 368–373), the engine only checks that at least one creature exists (`any(|o| o.power.is_some())`), then generates one `ActivateAbility` action per equ

- [ ] The Scryfall ruling states: "You can sacrifice the creature Demonmail Hauberk is equipping in order to equip it to another creature." This explicitly confirms the player has a choice. With auto-select

## disciple_of_griselbrand

- [ ] Player cannot choose which creature to sacrifice when multiple are available (`mtg-engine/src/engine.rs` lines 1750–1759)

- [ ] Oracle text says: `{1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.`

- [ ] Code does: `SacrificeCost::SacrificeCreature => { // For now, auto-sacrifice the first eligible creature. // TODO: Present choice to player when there are multiple options. let creature = new_state.ob

- [ ] `oracle_text` field wording does not match Scryfall oracle text (`mtg-engine/src/cards/isd/disciple_of_griselbrand.rs` line 25)

- [ ] Oracle text says: `{1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.`

- [ ] Code does: `oracle_text: "{1}, Sacrifice a creature: You gain life equal to that creature's toughness.".into()` — "that creature's" instead of "the sacrificed creature's".

## elder_cathar

- [ ] Transformed DFC incorrectly treated as Human in both the single-target auto-select path and the multi-target engine path

- [ ] Oracle text says: `If that creature is a Human, put two +1/+1 counters on it instead.`

- [ ] Code does (`elder_cathar.rs:50-58`, single-target path):
  ```rust
  let is_human = state.get_object(id)
  .map(|o| {

## elite_inquisitor

- [ ] Protection's targeting restriction is not enforced in the engine's ability-targeting path (`mtg-engine/src/engine.rs:758-768` and `mtg-engine/src/engine.rs:1305-1312`)

- [ ] Oracle text says: `Protection from Vampires, from Werewolves, and from Zombies`

- [ ] Per MTG rule 702.16c, protection means (among other things) the protected permanent "can't be the target of spells or abilities from sources" with the stated quality.

- [ ] Code does: `can_be_targeted` in `engine.rs` at lines 758–768 only checks hexproof; it does not check whether the source of the ability is a Vampire, Werewolf, or Zombie:
  ```rust
  fn can_be_targeted(state: &GameState, target_id: ObjectId, caster: PlayerId, registry: &CardRegistry) -> bool {
  if state.has_keyword(target_id, Keyword::Hexproof, registry) {

- [ ] Concrete cases broken by this:
  1. **Olivia Voldaren** (`mtg-engine/src/cards/isd/olivia_voldaren.rs`) is a Vampire with activated ability `{1}{R}: Deal 1 damage to another target creature`. Olivia can incorrectly target the Elite I
  2. **Nightfall Predator** (back face of `mtg-engine/src/cards/isd/daybreak_ranger.rs`) is a Werewolf with activated ability `{R}, {T}: This creature fights target creature`. Nightfall Predator can inc
  3. **Grimgrin, Corpse-Born** (`mtg-engine/src/cards/isd/grimgrin_corpse_born.rs`) is a Zombie whose `on_attacks` triggered ability builds its target list with no protection check (`state.objects_in_zo

## essence_of_the_wild

- [ ] **ETB abilities of creatures entering as EotW copies still fire** — `mtg-engine/src/triggers.rs:344-392` and `mtg-engine/src/state.rs:524-575`

- [ ] Oracle text says: `"Because creatures you control enter as copies of Essence of the Wild, any 'enters' triggered abilities printed on such creatures won't trigger."` (official ruling 2011-09-22)

- [ ] Code does: `apply_entering_copy_replacement` (state.rs:524) updates the entering creature's name, power, toughness, colors, card_types, subtypes, keywords, and instance_oracle_text — but does NOT upda

- [ ] **EotW entering via non-`on_resolve` path does not apply replacement effect** — `mtg-engine/src/cards/isd/essence_of_the_wild.rs:40-53`

- [ ] Oracle text says: `"Creatures you control enter as a copy of this creature."` — a continuous replacement effect that applies whenever EotW is on the battlefield, regardless of how it arrived.

- [ ] Code does: The `entering_copy_source` flag (which is what `apply_entering_copy_replacement` in state.rs:540 checks) is only set to `true` in `on_resolve` (essence_of_the_wild.rs:46: `obj.entering_copy

## evil_twin

- [ ] **Activated ability inaccessible after copying (engine.rs + evil_twin.rs)**

- [ ] Oracle text says: `"except it has '{U}{B}, {T}: Destroy target creature with the same name as this creature.'"`

- [ ] Code does: The `CopyCreature` handler at `engine.rs:2458` sets `obj.card_id = card_id` (the target creature's card_id). After this, action generation at `engine.rs:326` calls `registry.get(obj_card_id

- [ ] **ETB abilities of the copied creature never trigger (triggers.rs)**

- [ ] Oracle text says (from rulings): `"Any enters-the-battlefield abilities of the copied creature will trigger when Evil Twin enters the battlefield."`

- [ ] Code does: `collect_triggers` (`triggers.rs:344–363`) reads the current `card_id` off the object at trigger-collection time, which is Evil Twin's own card_id (the copy hasn't happened yet). The result

- [ ] **`is_evil_twin` marker set before the optional copy choice is made (evil_twin.rs:53–55)**

- [ ] Oracle text says: `"You may have this creature enter as a copy of any creature on the battlefield, except it has…"` — the destroy ability is part of the *copy* clause; it should only exist if a copy i

- [ ] Code does: `evil_twin.rs:49–65` sets `is_evil_twin` in `card_state` before calling `present_optional_target_choice`. If the player declines the optional choice, `obj.card_id` remains as Evil Twin's ca

## falkenrath_noble

- [ ] **Issue 1 — "target player" is auto-targeted instead of chosen** (`mtg-engine/src/cards/isd/falkenrath_noble.rs`, lines 59–68)

- [ ] Oracle text says: `"target player loses 1 life and you gain 1 life"`

- [ ] Code does: `fn drain(state: &mut GameState, controller: PlayerId) { let opponent = state.opponent(controller); ... }` — the `drain` helper hard-codes the opponent as the life-loss target, with a comme

- [ ] **Issue 2 — simultaneous death triggers only once instead of twice** (`mtg-engine/src/triggers.rs`, lines 418–421)

- [ ] Oracle ruling says: `"If Falkenrath Noble and another creature die at the same time, Falkenrath Noble's triggered ability will trigger for each of them."`

- [ ] Code does: In `collect_triggers`, the death-watch scan is `state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.id != dead_id)`. When multiple creatures die in the same SBA pass (`check_

- [ ] **Issue 3 — DeathWatch trigger incorrectly fizzles if Noble leaves the battlefield between trigger and resolution** (`mtg-engine/src/triggers.rs`, lines 906–912, and `mtg-engine/src/cards/isd/falkenra

- [ ] Oracle text says: `"Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life."` — no requirement that Noble remain on the battlefield at resolution.

- [ ] Code does: `resolve_next_trigger` for `PendingTrigger::DeathWatch` guards with `if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` — if Noble is bounced, destroyed,

## fiend_hunter

- [ ] ETB trigger silently dropped when Fiend Hunter has left the battlefield before resolution

- [ ] Oracle text says: `"When this creature enters, you may exile another target creature."`

- [ ] Official ruling says: `"If Fiend Hunter leaves the battlefield before its first ability has resolved, its second ability will trigger and do nothing. Then its first ability will resolve and exile the 

- [ ] Code does: In `mtg-engine/src/triggers.rs` lines 893–899, the `EnteredBattlefield` trigger resolution has a guard `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)`

## full_moons_rise

- [ ] Activated ability description in `mtg-engine/src/cards/isd/full_moons_rise.rs` line 57 says "Wolf and Werewolf" when oracle says "Werewolf" only. This description is embedded directly into the log mes

- [ ] Oracle text says: `Sacrifice this enchantment: Regenerate all Werewolf creatures you control.`

- [ ] Code does: `description: "Sacrifice: Regenerate all Wolf and Werewolf creatures you control".into()`

## furor_of_the_bitten

- [ ] Missing "Enchant creature\n" prefix in `oracle_text` field — `mtg-engine/src/cards/isd/furor_of_the_bitten.rs:22`

- [ ] Oracle text says: `"Enchant creature\nEnchanted creature gets +2/+2 and attacks each combat if able."`

- [ ] Code does: `oracle_text: "Enchanted creature gets +2/+2 and attacks each combat if able.".into()`

- [ ] All other auras in the same set that have been verified (dead_weight, curiosity, sensory_deprivation, wreath_of_geists, claustrophobia) include the "Enchant creature\n" prefix. Furor omits it. This is

- [ ] Forced-attack enforcement ignores Haste overriding summoning sickness — `mtg-engine/src/engine.rs:1827`

- [ ] Oracle text says: `"attacks each combat if able"` (ruling: "If the enchanted creature can't attack for any reason (such as being tapped or having come under that player's control that turn), then it d

- [ ] Code does: `|| creature.summoning_sick {  continue; }` — unconditionally skips summoning-sick creatures without checking for Haste.

- [ ] By contrast, `eligible_attackers` in `combat.rs:577` correctly uses `(!o.summoning_sick || state.has_keyword(o.id, Keyword::Haste, registry))`. A creature enchanted by Furor that has Haste AND summoni

- [ ] Forced-attack enforcement skips `can_attack()` check — `mtg-engine/src/engine.rs:1838–1846`

- [ ] Oracle text says: `"attacks each combat if able"`

- [ ] Code does: checks only for the `ForceAttack` continuous effect, Defender keyword, tapped, and summoning sick. It does NOT call `state.can_attack()`, which checks for `PreventAttack` continuous effects

- [ ] If a creature has both `ForceAttack` (Furor) and a `PreventAttack` effect (Pacifism on the same creature, or Bonds of Faith on a non-Human), the `eligible_attackers` function correctly excludes it (th

## galvanic_juggernaut

- [ ] Forced attack logic ignores `can_attack()`, violating the "if able" clause

- [ ] File: `mtg-engine/src/engine.rs` lines 1822–1847 (DeclareAttackers forced-attacker loop)

- [ ] Oracle text says: `"This creature attacks each combat if able."`

- [ ] Code does: The loop checks `creature.tapped`, `creature.summoning_sick`, and `has_keyword(Defender)` as "unable" conditions, but does **not** call `state.can_attack(creature.id, registry)`. `can_attac

## garruk_relentless

- [ ] **`abilities_activated_this_turn` never cleared between turns** — `mtg-engine/src/engine.rs:1942`, `mtg-engine/src/engine.rs:3006-3061`

- [ ] Oracle text says: Garruk's loyalty abilities are activatable once per turn, implying they reset each turn. The rulings confirm "You can't activate a loyalty ability of Garruk Relentless and **later th

- [ ] Code does: When a loyalty ability activates (`engine.rs:1942`), the sentinel `999` is inserted into `obj.abilities_activated_this_turn`. The legal-actions generator (`engine.rs:415`) skips loyalty abi

- [ ] **`is_legendary` not set in `on_resolve`** — `mtg-engine/src/cards/isd/garruk_relentless.rs:313-321`

- [ ] Oracle text says: `"Legendary Planeswalker — Garruk"` — Garruk has the Legendary supertype, making the Legend Rule (CR 704.5k) apply.

- [ ] Code does: `on_resolve` sets `obj.card_types = vec![CardType::Planeswalker]` but never sets `obj.is_legendary = true`. The Legend Rule SBA in `sba.rs:290` gates entirely on `obj.is_legendary`: `if obj

## gatstaf_shepherd

- [ ] **Engine never increments `spells_cast_this_turn`; `spells_cast_last_turn` is always empty, making both transform conditions permanently wrong**

- [ ] `mtg-engine/src/engine.rs` lines 1479–1666: The `Action::CastSpell` handler fires `GameEvent::SpellCast` but never writes to `state.spells_cast_this_turn`. Searched entire codebase: `spells_cast_this_

- [ ] `mtg-engine/src/engine.rs` lines 2880–2903 (turn transition in `advance_step`) and lines 3006–3061 (`Step::Cleanup` in `perform_turn_based_actions`): Neither location copies `spells_cast_this_turn` to

- [ ] **Front face consequence**: `werewolf_should_transform` checks `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum()` (line 12 of `gatstaf_shepherd.rs`), then `total_spells_las

- [ ] **Back face consequence**: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` (line 16 of `gatstaf_shepherd.rs`) on an empty map always returns `false`. Gatstaf Howler never transforms ba

- [ ] Oracle text says (front): `"if no spells were cast last turn"` — should only trigger when the total spell count for the turn was zero.

- [ ] Oracle text says (back): `"if a player cast two or more spells last turn"` — should trigger when any one player cast ≥ 2 spells last turn.

- [ ] Code does: front face transforms unconditionally every upkeep (after turn 1); back face never transforms back.

- [ ] **Log message names the wrong source card when transforming back to front face**

- [ ] `mtg-engine/src/cards/isd/gatstaf_shepherd.rs` lines 87–89:
  ```rust
  state.log(crate::state::LogLevel::Event,
  format!("Gatstaf Shepherd transforms into {}", name));

- [ ] The format string hardcodes `"Gatstaf Shepherd"` as the subject of transformation. When the card is on its back face (Gatstaf Howler) and transforms back, `is_transformed` is set to `false`, `name` be

- [ ] Oracle text does not specify log message content, but this is an implementation inaccuracy that misrepresents the game event to observers.

## geist_of_saint_traft

- [ ] Extra tokens created by Parallel Lives doubling are not tapped, not attacking, and not added to `end_of_combat_exiles` — `mtg-engine/src/cards/isd/geist_of_saint_traft.rs` lines 57–81

- [ ] Oracle text says: `"create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat."` and ruling says: `"If you create more than one Angel token (mo

- [ ] Code does: `let token_id = state.create_token_with_subtypes(...)` returns only the primary token ID; `create_token_with_subtypes` internally creates `2^N - 1` extra copies via Parallel Lives but those

## geistcatchers_rig

- [ ] **Target selection deferred to resolution instead of stack-placement** (`mtg-engine/src/cards/isd/geistcatchers_rig.rs` lines 40–59, `mtg-engine/src/triggers.rs` lines 344–364)

- [ ] Oracle text says: `When this creature enters, you may have it deal 4 damage to target creature with flying.`

- [ ] Scryfall ruling says: `The target creature with flying is chosen when the ability triggers and goes on the stack. You choose whether or not Geistcatcher's Rig will deal 4 damage to it when the ability

- [ ] Code does: Target selection happens entirely inside `on_enter_battlefield`, which is called at trigger resolution (via `resolve_next_trigger` → `behavior.on_enter_battlefield`). The `collect_triggers`

- [ ] **`optional: true` conflates target selection with the "you may" decision** (`mtg-engine/src/cards/isd/geistcatchers_rig.rs` line 56)

- [ ] Oracle text says (per ruling): target is mandatory at stack time (if legal targets exist); `You choose whether or not Geistcatcher's Rig will deal 4 damage to it when the ability resolves.`

- [ ] Code does: `ResolutionChoiceKind::ChooseTarget { ..., optional: true, ... }` — presents a single merged choice at resolution that lets the player pick a target OR pick `None` (skip). This allows decli

- [ ] **ETB trigger silently suppressed if source leaves battlefield before resolution** (`mtg-engine/src/triggers.rs` lines 893–899)

- [ ] Oracle text says: `When this creature enters, you may have it deal 4 damage to target creature with flying.`

- [ ] Code does: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { behavior.on_enter_battlefield(...) }` — if Geistcatcher's Rig leaves the battlefield after the trigge

## geistflame

- [ ] Engine's `AnyTarget` implementation excludes planeswalkers as valid targets (`mtg-engine/src/engine.rs` lines 836–864, 1074–1090, 1343–1358; `mtg-engine/src/cards/mod.rs` line 244–245)

- [ ] Oracle text says: `"Geistflame deals 1 damage to any target."`

- [ ] Code does: `TargetRequirement::AnyTarget` is described in its own comment as "Target any creature or player" and the engine's three implementations of `AnyTarget` target generation all filter objects 

- [ ] `resolve_damage` helper does not remove loyalty counters when dealing damage to a planeswalker (`mtg-engine/src/cards/helpers.rs` lines 52–62)

- [ ] Oracle text says: `"Geistflame deals 1 damage to any target."` — damage to a planeswalker causes it to lose that many loyalty counters (MTG CR 120.3).

- [ ] Code does: `obj.damage_marked += amount;` — marks `damage_marked` on the object rather than subtracting from loyalty counters. There is no SBA or other mechanism that converts `damage_marked` to loyal

## ghost_quarter

- [ ] **"May" search is forced — controller gets no choice** (`mtg-engine/src/cards/isd/ghost_quarter.rs`, line 81–100)

- [ ] Oracle text says: `"Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle."`

- [ ] Code does: `// Its controller may search for a basic land (auto-search).` — immediately proceeds with `.find()` and puts the land onto the battlefield with no `AwaitingAction::ResolutionChoice` or `Ye

- [ ] **Missing shuffle after library search** (`mtg-engine/src/cards/isd/ghost_quarter.rs`, lines 92–101)

- [ ] Oracle text says: `"put it onto the battlefield, then shuffle."`

- [ ] Code does: calls `state.move_object(land_id, Zone::Battlefield)` and returns; there is no call to `library_order.shuffle()`. Comparable search cards in the same engine (Caravan Vigil `caravan_vigil.rs

## ghoulcallers_chant

- [ ] `build_cast_target_spec` returns `CastTargetSpec::SingleTarget` for modal spells containing a `TwoTargets` inner mode, blocking interactive players from choosing mode 2

- [ ] Oracle text says: `• Return two target Zombie cards from your graveyard to your hand.`

- [ ] Code does: In `engine.rs:1212-1219`, `build_cast_target_spec` for `ModalChoice` iterates over each mode and calls `valid_targets_for_req` on it. For mode 2 (`TwoTargets(GraveyardCreatureOfSubtype("Zom

## ghoulraiser

- [ ] ETB trigger silently skipped if Ghoulraiser leaves the battlefield before the trigger resolves — engine bug in `mtg-engine/src/triggers.rs` lines 893–899

- [ ] Oracle text says: `When this creature enters, return a Zombie card at random from your graveyard to your hand.`

- [ ] Code does: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { if let Some(behavior) = registry.get(card_id) { behavior.on_enter_battlefield(state, object_id, regis

## grave_bramble

- [ ] Protection from Zombies incorrectly prevents Grave Bramble from blocking Zombie attackers (`mtg-engine/src/combat.rs:699`)

- [ ] Oracle text says: `protection from Zombies`

- [ ] Code does: `can_block_attacker` contains the check `if has_protection_from_creature(state, blocker_id, attacker_id, registry) { return false; }` at line 699. When Grave Bramble is `blocker_id` and a Z

- [ ] Grimgrin, Corpse-Born's triggered ability can target Grave Bramble despite protection from Zombies (`mtg-engine/src/cards/isd/grimgrin_corpse_born.rs:99–103`)

- [ ] Oracle text says: `protection from Zombies`

- [ ] Code does: `on_attacks` builds the target list as `state.objects_in_zone(Zone::Battlefield, defender).iter().filter(|o| o.power.is_some()).map(|o| Target::Object(o.id)).collect()` with no protection c

## grimgrin_corpse_born

- [ ] Auto-sacrifice for the activated ability doesn't present a player choice when multiple sacrifice targets are available.

- [ ] Oracle text says: `Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.`

- [ ] Code does: `engine.rs` lines 1761–1772: `SacrificeCost::SacrificeAnotherCreature` handling calls `.find(|o| o.power.is_some() && o.id != *object_id)` and auto-sacrifices whichever creature comes first

## grimoire_of_the_dead

- [ ] Legend rule not applied to legendary creature cards returned by ability 2 if they were never previously on the battlefield (`mtg-engine/src/cards/isd/grimoire_of_the_dead.rs:151-163`, `mtg-engine/src/

- [ ] Oracle text says: `Put all creature cards from all graveyards onto the battlefield under your control.`

- [ ] Code does: `state.move_object(cid, Zone::Battlefield)` moves the creature to the battlefield, but `is_legendary` is only set to `true` in `on_resolve` — called only when a card resolves from the stack

## grizzled_outcasts

- [ ] Engine never updates `spells_cast_this_turn` or `spells_cast_last_turn`; both transform conditions are always wrong

- [ ] File: `mtg-engine/src/engine.rs` lines 1657–1665 (spell cast) and 2882–2895 (turn transition)

- [ ] Oracle text says (front face): `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."`

- [ ] Oracle text says (back face): `"At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."`

- [ ] Code does: `spells_cast_this_turn` is initialized to `HashMap::new()` in `state.rs` line 230 and never incremented anywhere when a spell is cast. `spells_cast_last_turn` is initialized to `HashMap::ne

- [ ] Consequence for front face: `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum()` (grizzled_outcasts.rs line 12) always equals 0, so `total_spells_last_turn == 0 && !state.is_

- [ ] Consequence for back face: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` (line 16) is always false — Krallenhorde Wantons **never transforms back** even when two or more spells were 

- [ ] Log message is incorrect when transforming back to front face

- [ ] File: `mtg-engine/src/cards/isd/grizzled_outcasts.rs` line 87–88

- [ ] Code does: `format!("Grizzled Outcasts transforms into {}", name)` where `name` is computed after flipping `is_transformed`. When transforming from Krallenhorde Wantons → Grizzled Outcasts, `name = "G

- [ ] The log should read `"Krallenhorde Wantons transforms into Grizzled Outcasts"`. The source name is hardcoded as `"Grizzled Outcasts"` regardless of which face was active before the flip.

## gruesome_deformity

- [ ] Artifact creature tokens cannot block creatures with intimidate, violating the oracle rule "can't be blocked except by artifact creatures and/or creatures that share a color with it."

- [ ] Oracle text says: `"It can't be blocked except by artifact creatures and/or creatures that share a color with it."`

- [ ] Code does: `mtg-engine/src/combat.rs:632-634`:
  ```rust
  let is_artifact = registry.card_data(blocker.card_id)
  .map(|d| d.card_types.contains(&crate::types::CardType::Artifact))

## gutter_grime

- [ ] Token `is_token` check fails in real gameplay: Gutter Grime incorrectly triggers when a token you control dies

- [ ] Oracle text says: `"Whenever a nontoken creature you control dies"`

- [ ] Code does: `let was_token = state.get_object(dead_id).map(|o| o.is_token).unwrap_or(false);` (`gutter_grime.rs:53`). In real gameplay via `run_game_loop_inner` (`engine.rs:3119-3126`), the SBA loop ru

## hamlet_captain

- [ ] Trigger does not resolve if Hamlet Captain leaves the battlefield before the trigger resolves.

- [ ] Oracle text says: `Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn.`

- [ ] Code does: In `mtg-engine/src/triggers.rs` lines 980–986, the `AttacksTrigger` resolution is gated by `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)`. Equivalent

## hanweir_watchkeep

- [ ] Engine never updates `spells_cast_this_turn` or `spells_cast_last_turn`, causing both werewolf transform conditions to evaluate incorrectly in actual gameplay.

- [ ] Oracle text says (front face): `At the beginning of each upkeep, if no spells were cast last turn, transform this creature.`

- [ ] Oracle text says (back face): `At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.`

- [ ] Code does: `state.spells_cast_this_turn` is declared in `state.rs:127` and initialized to an empty HashMap at `state.rs:230`, but the engine's `Action::CastSpell` handler in `engine.rs` (lines 1479–16

- [ ] Front face: `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum()` always equals 0; condition `total_spells_last_turn == 0 && !state.is_first_turn` is always `true` after turn 

- [ ] Back face: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` is always `false` (HashMap is empty) → Bane of Hanweir never transforms back, regardless of how many spells were cast.

## harvest_pyre

- [ ] Player cannot choose which specific cards to exile; engine arbitrarily picks cards

- [ ] Oracle text says: `"exile X cards from your graveyard"` — in MTG, the casting player freely selects which X cards to exile from their graveyard as the additional cost

- [ ] Code does: `mtg-engine/src/engine.rs` lines 1613–1616: `new_state.objects.values().filter(|o| o.zone == Zone::Graveyard && o.owner == player && o.id != *object_id).map(|o| o.id).take(x as usize).colle

## heretics_punishment

- [ ] `AnyTarget` engine implementation excludes planeswalkers as valid targets

- [ ] Oracle text says: `Choose any target`

- [ ] Code does: Both `valid_targets_for_req` (`engine.rs:1074–1090`) and `generate_ability_targets` (`engine.rs:1343–1358`) filter battlefield objects with `.filter(|o| o.power.is_some())`, which selects o

## hinterland_harbor

- [ ] `controller_has_matching_land` only checks object-level subtypes (`o.subtypes`), missing registry-stored subtypes for regularly-played Forest/Island cards

- [ ] File: `mtg-engine/src/cards/isd/hinterland_harbor.rs`, lines 17–23

- [ ] Oracle text says: `"This land enters tapped unless you control a Forest or an Island."`

- [ ] Code does: `o.subtypes.iter().any(|s| s == "Forest") || o.subtypes.iter().any(|s| s == "Island")` — only checks `obj.subtypes`, which is `Vec::new()` for all regular (non-token) cards. For regular car
  Supporting evidence:

- [ ] `state.rs` line 1205: `/// Subtypes on this object (for tokens — regular cards use CardData.subtypes via registry).`

- [ ] `engine.rs` `setup_game` (lines 2670–2682): copies `colors`, `name`, `keywords`, `card_types` to the object but never copies `card_data.subtypes` to `obj.subtypes`.

- [ ] `forest.rs` line 15: `subtypes: vec!["Forest".into()]` — this lives in `CardData`, not the `GameObject`.

- [ ] By contrast, `check_condition` in `state.rs` (lines 1084–1093) correctly checks both `o.subtypes` AND `registry.card_data(o.card_id).subtypes` when testing for subtype control.

- [ ] The existing test `clifftop_retreat_enters_untapped_with_mountain` (same pattern) masks this bug by manually setting `state.get_object_mut(mtn).unwrap().subtypes = vec!["Mountain".into()]` — which is 

## hollowhenge_scavenger

- [ ] ETB trigger resolution silently skipped when source leaves battlefield before trigger resolves (`mtg-engine/src/triggers.rs:893-899`)

- [ ] Oracle text says: `"you gain 5 life"` (the effect has no dependency on the source remaining on the battlefield)

- [ ] Code does: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { behavior.on_enter_battlefield(...) }` — if the Scavenger is destroyed in response to its own ETB trig

## inquisitors_flail

- [ ] Fight damage incorrectly doubled by Flail: `mtg-engine/src/combat.rs` lines 452–454

- [ ] Oracle text says: `"If equipped creature would deal combat damage, it deals double that damage instead."` / `"If another creature would deal combat damage to equipped creature, it deals double that da

- [ ] Code does: `deal_damage_to_creature` unconditionally applies `amount *= combat_damage_multiplier(state, source, registry); amount *= combat_damage_multiplier(state, target, registry);` for **all** dam

## instigator_gang

- [ ] **Engine never increments `spells_cast_this_turn` or updates `spells_cast_last_turn`**: The fields `state.spells_cast_this_turn` and `state.spells_cast_last_turn` are declared in `mtg-engine/src/state
  Consequence for Instigator Gang (front face, `!is_transformed`): `werewolf_should_transform` at `instigator_gang.rs:13–15` computes `total_spells_last_turn: u32 = state.spells_cast_last_turn.values().

- [ ] Oracle text says: `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."`

- [ ] Code does: `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum();` — always 0, so condition always fires (engine never populates the map)
  Consequence for Wildblood Pack (back face, `is_transformed`): `werewolf_should_transform` at `instigator_gang.rs:17` checks `state.spells_cast_last_turn.values().any(|&count| count >= 2)` — always `fa

- [ ] Oracle text says: `"At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."`

- [ ] Code does: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` — always false (engine never populates the map)

- [ ] **Log message names wrong source when transforming back** (`instigator_gang.rs:119–121`): When Wildblood Pack transforms back to Instigator Gang (`was_transformed = true`), the log prints `"Instigator

- [ ] Oracle text says: (transformation is bidirectional between two named faces)

- [ ] Code does: `format!("Instigator Gang transforms into {}", name)` at line 121 — "Instigator Gang" is always the stated source, even when the source face is Wildblood Pack

## into_the_maw_of_hell

- [ ] `is_valid_target` accepts creatures for the land target slot, allowing the card to be cast with no legal land target

- [ ] Oracle text says: `"Destroy target land. Into the Maw of Hell deals 13 damage to target creature."`

- [ ] Code does: `fn is_valid_target` returns `is_land || is_creature` for any object, with no slot distinction. The engine's `valid_targets_for_req` for `TargetRequirement::PermanentWithFilter(_)` ignores 
  Concrete consequence: if there are no lands on the battlefield but 2+ creatures, the engine will offer Into the Maw of Hell as a legal cast action, placing a creature's ObjectId in the land slot. `on_

- [ ] Card file: `mtg-engine/src/cards/isd/into_the_maw_of_hell.rs` lines 40–56 (`is_valid_target`)

- [ ] Engine file: `mtg-engine/src/engine.rs` lines 1067–1072 (`valid_targets_for_req` for `PermanentWithFilter`)

## isolated_chapel

- [ ] Subtype check in `controller_has_matching_land` only reads `obj.subtypes` on game objects, missing subtypes stored in the registry for regular (non-token) cards

- [ ] Oracle text says: `This land enters tapped unless you control a Plains or a Swamp.`

- [ ] Code does: `o.subtypes.iter().any(|s| s == "Plains") || o.subtypes.iter().any(|s| s == "Swamp")` (`mtg-engine/src/cards/isd/isolated_chapel.rs` lines 21–23) — this only checks the runtime `obj.subtype

## kessig_cagebreakers

- [ ] **Attack trigger silently discarded if Kessig is destroyed before resolution** (`mtg-engine/src/triggers.rs:980-985` and `mtg-engine/src/cards/isd/kessig_cagebreakers.rs:39-42`)
  The engine's `resolve_next_trigger` guards the `AttacksTrigger` resolution with a battlefield check. The card's own `on_attacks` then also early-returns if the source is not on the battlefield:

- [ ] Oracle text says: `"Whenever this creature attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard."`

- [ ] Code does (engine, `triggers.rs:980-985`):
  ```rust
  PendingTrigger::AttacksTrigger { object_id, card_id, .. } => {
  if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {

- [ ] Code does (card, `kessig_cagebreakers.rs:39-42`):
  ```rust
  let controller = match state.get_object(self_id) {
  Some(o) if o.zone == Zone::Battlefield => o.controller,

- [ ] **Parallel Lives doubled tokens are not set as tapped and attacking** (`mtg-engine/src/cards/isd/kessig_cagebreakers.rs:61-76` and `mtg-engine/src/state.rs:314-348`)

- [ ] Oracle text says: `"create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard"`

- [ ] Code does (`kessig_cagebreakers.rs:61-76`):
  ```rust
  for _ in 0..creature_count {
  let token_id = state.create_token_with_subtypes(...);

- [ ] `create_token_with_subtypes` (`state.rs:314-348`) creates the primary token and then creates extra copies for Parallel Lives, but returns only the primary token's `ObjectId`; the extra copies' IDs are
  ```rust
  let id = self.create_token_internal(...);           // primary
  for _ in 0..extra_copies {

## kruin_outlaw

- [ ] **Engine never increments `spells_cast_this_turn` when a spell is cast, and never saves it to `spells_cast_last_turn` at turn end** (`mtg-engine/src/engine.rs` CastSpell handler ~line 1657; `advance_s

- [ ] Oracle text says: `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."` and `"At the beginning of each upkeep, if a player cast two or more spells last turn, 

- [ ] Code does: The `CastSpell` action handler pushes `GameEvent::SpellCast` (line 1657) but never executes `state.spells_cast_this_turn.entry(player).and_modify(|e| *e += 1).or_insert(1)`. The `advance_st

- [ ] **Log message says "Kruin Outlaw transforms into Kruin Outlaw" when back face transforms back to front face** (`mtg-engine/src/cards/isd/kruin_outlaw.rs` lines 103–104)

- [ ] Oracle text says: the back face (Terror of Kruin Pass) transforms into Kruin Outlaw.

- [ ] Code does: `format!("Kruin Outlaw transforms into {}", name)` is used unconditionally. When `is_transformed` is toggled from `true` to `false`, `name` is `"Kruin Outlaw"`, producing the log entry `"Kr

## liliana_of_the_veil

- [ ] **+1 ability: Player 1's discard resolves before Player 2 even makes their choice, violating the "all at the same time" ruling.**

- [ ] File: `mtg-engine/src/cards/isd/liliana_of_the_veil.rs` lines 136–144 (auto-discard path) and `mtg-engine/src/engine.rs` lines 2012–2022 (choice path)

- [ ] Oracle text ruling says: `"first the player whose turn it is chooses a card in hand without revealing it, then each other player in turn order does the same. Then all the chosen cards are discarded at

- [ ] Code does: In the auto-discard path (single card), `state.move_object(card_id, Zone::Graveyard)` and `state.events.push(GameEvent::Discarded { ... })` execute at lines 136–140 for the first player, an

## ludevics_test_subject

- [ ] **`card_state` (hatchling counters) not reset on zone change** — `mtg-engine/src/state.rs`, `move_object()` lines 479–487

- [ ] Oracle text says: `{1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all of them and transform it.` (per MTG rule 400.7, counters are lo

- [ ] Code does: `move_object()` clears `obj.counters` (standard counter map) but does NOT clear `obj.card_state`. Ludevic's Test Subject stores hatchling counters in `card_state` under the key `"hatchling_

- [ ] **`is_transformed` not reset on zone change** — `mtg-engine/src/state.rs`, `move_object()` lines 479–487

- [ ] Oracle text says the front face is Ludevic's Test Subject (0/3 Defender); when a DFC leaves the battlefield it is treated as its front face in all other zones (MTG rule 711.7b), so it must re-enter as

- [ ] Code does: `move_object()` does not reset `obj.is_transformed`. If Ludevic's Abomination (the transformed state, `is_transformed = true`) is bounced to hand (e.g., by Silent Departure or Lost in the M

## makeshift_mauler

- [ ] Engine auto-selects which creature to exile instead of giving the player a choice (`mtg-engine/src/engine.rs` ~line 1574)

- [ ] Oracle text says: `"As an additional cost to cast this spell, exile a creature card from your graveyard."`

- [ ] Code does: `// Pick highest-power creatures first (better default for Corpse Lunge). ... exile_candidates.sort_by(|a, b| b.1.cmp(&a.1)); // Highest power first` — the engine sorts graveyard creatures 

## mask_of_avacyn

- [ ] **Duplicate equip action generated via attached-aura loop enables broken re-equip** (`mtg-engine/src/engine.rs:331-338`, `mtg-engine/src/cards/isd/mask_of_avacyn.rs:59-65`)

- [ ] Oracle text says: `Equip {3}` — paying {3} at sorcery speed attaches the Mask to a creature you control.

- [ ] Code does: When the Mask is already attached to a creature, `legal_actions` iterates the equipped creature as `obj_id` and the attached-aura loop (engine.rs:331-338) calls `MaskOfAvacyn::activated_abi

## maw_of_the_mire

- [ ] `is_valid_target` only consults the registry for `CardType::Land`, missing land tokens whose types are stored on `obj.card_types` (not in registry)

- [ ] Oracle text says: `Destroy target land.`

- [ ] Code does: `registry.card_data(obj.card_id).map(|d| d.card_types.contains(&CardType::Land)).unwrap_or(false)` — tokens have `card_id: CardId(0)`, so `registry.card_data(CardId(0))` returns `None`, and

## mayor_of_avabruck

- [ ] **Engine never populates `spells_cast_last_turn`** — both transform conditions are permanently broken in actual gameplay.

- [ ] `mtg-engine/src/state.rs:127,131`: `spells_cast_this_turn` and `spells_cast_last_turn` are defined as `HashMap<PlayerId, u32>`, initialized empty, and **never written to** anywhere in the engine sourc

- [ ] `mtg-engine/src/engine.rs` `CastSpell` handler (lines 1479–1666): handles spell casting in full but contains no increment of `spells_cast_this_turn`.

- [ ] `mtg-engine/src/engine.rs` `advance_step` (lines 2867–2904): handles end-of-turn transition (sets new active player, increments turn_number, clears `creature_died_this_turn`) but never transfers `spel

- [ ] Consequence for front face: `total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum()` is always 0; condition `total_spells_last_turn == 0 && !state.is_first_turn` is always true after

- [ ] Oracle text says: `"if no spells were cast last turn, transform this creature"`

- [ ] Code does: evaluates `spells_cast_last_turn` which is always empty → condition always true → always transforms

- [ ] Consequence for back face: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` is always false. Howlpack Alpha **never** transforms back regardless of how many spells were cast.

- [ ] Oracle text says: `"if a player cast two or more spells last turn, transform this creature"`

- [ ] Code does: evaluates `spells_cast_last_turn` which is always empty → condition always false → never transforms back

- [ ] All tests bypass this by directly inserting into `spells_cast_last_turn` (e.g., `state.spells_cast_last_turn.insert(P0, 2)`), so unit tests pass even though the engine never populates the field.

- [ ] **Log message hardcodes wrong source name when transforming back** — `mtg-engine/src/cards/isd/mayor_of_avabruck.rs:119`

- [ ] Oracle text says: (transform event — Howlpack Alpha becomes Mayor of Avabruck)

- [ ] Code does: `format!("Mayor of Avabruck transforms into {}", name)` — `name` is `"Mayor of Avabruck"` when transforming back, producing log message `"Mayor of Avabruck transforms into Mayor of Avabruck

## memorys_journey

- [ ] **Missing mandatory `Target::Player` target — opponent cannot be targeted with 0 card targets, and player hexproof is never checked** (`mtg-engine/src/cards/isd/memorys_journey.rs` lines 39–43, 49–55)

- [ ] Oracle text says: `"Target player shuffles up to three target cards from their graveyard into their library."` and ruling: `"You don't have to target any cards when you cast Memory's Journey, but you 

- [ ] Code does: `target_requirement` returns `TargetRequirement::ModalChoice(vec![TargetRequirement::UpToTargets(3, Box::new(TargetRequirement::GraveyardCardOwnedByCaster)), TargetRequirement::UpToTargets(
  The consequences are:
  1. **0-card case always shuffles controller's library.** The engine generates `CastSpell { targets: vec![] }` for the k=0 combination in both UpToTargets modes. On resolve, `on_resolve` runs `unwrap_o
  2. **Player hexproof not checked.** `can_target_player` (which gates on Witchbane Orb hexproof) is only called when a `Target::Player` flows through a `PlayerOnly` / `AnyTarget` requirement. Because `

## mentor_of_the_meek

- [ ] **"You may pay {1}" is auto-paid instead of presenting a player choice** (`mtg-engine/src/cards/isd/mentor_of_the_meek.rs`, lines 55–80)

- [ ] Oracle text says: `you may pay {1}. If you do, draw a card.`

- [ ] Code does: `// "You may pay {1}" — auto-pay if the controller has any mana in pool (pays {1}). ... if pool.total() >= 1 { ... if paid { crate::engine::draw_cards(state, controller, 1); } }` — automati

- [ ] **Power checked at resolution time, not at ETB trigger time** (`mtg-engine/src/cards/isd/mentor_of_the_meek.rs`, line 51; `mtg-engine/src/triggers.rs`, lines 366–392)

- [ ] Oracle text says (ruling 2025-01-24): `Mentor of the Meek's ability checks the power of the other creature only as it enters. If that creature's power is 2 or less, the ability will trigger. Once the 

- [ ] Code does: In `collect_triggers`, the `EnterWatch` trigger is dispatched for **all entering creatures** regardless of power (the only filter is `o.power.is_some()` — is it a creature). The power ≤ 2 c

- [ ] **Test enshrines wrong auto-pay behavior** (`mtg-engine/tests/tier15_cards.rs`, lines 71–99)

- [ ] Oracle text says: `you may pay {1}. If you do, draw a card.`

- [ ] Code does: The test pre-loads colorless mana and calls `on_any_creature_enters` directly, then asserts `hand_count == 1` without any `awaiting_action` / YesNo interaction. This confirms and validates 

## mikaeus_the_lunarch

- [ ] Summoning sickness not enforced for {T} activated abilities

- [ ] Oracle text says: `{T}: Put a +1/+1 counter on Mikaeus.` and `{T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature you control.`

- [ ] Code does: `mtg-engine/src/engine.rs` line 356: `if ab.requires_tap && obj_tapped { continue; }` — this only skips if the permanent is already tapped; it does not check `summoning_sick`. Neither does 

## mirror_mad_phantasm

- [ ] **Reveal loop uses `draw_top_card()` which sets `has_drawn_from_empty = true` on library exhaustion, causing the player to incorrectly lose the game**

- [ ] File: `mtg-engine/src/cards/isd/mirror_mad_phantasm.rs`, lines 83–97 (loop), exercising `mtg-engine/src/state.rs` line 1315

- [ ] Oracle text says: `"they reveal cards from the top of that library until a card named Mirror-Mad Phantasm is revealed. The player puts that card onto the battlefield and all other cards revealed this 

- [ ] Code does: `state.get_player_mut(owner).draw_top_card()` is called in the reveal loop. `draw_top_card()` in `state.rs:1314-1316` contains `if self.library_order.is_empty() { self.has_drawn_from_empty 

- [ ] **Token copy of Mirror-Mad Phantasm activating the ability is incorrectly found in the reveal loop and enters the battlefield**

- [ ] File: `mtg-engine/src/cards/isd/mirror_mad_phantasm.rs`, lines 69–115; SBA check in `mtg-engine/src/sba.rs` line 307–314 runs only after the ability resolves

- [ ] Oracle text ruling says: `"If no card named Mirror-Mad Phantasm is revealed (possibly because it was a card copying Mirror-Mad Phantasm or it was a token), all cards from that library will be put into

- [ ] Code does: `state.move_object(object_id, Zone::Library)` moves the token to Zone::Library (line 69). SBAs do not run until after `on_activate_ability` returns. During the loop, `draw_top_card()` retur

## moldgraf_monstrosity

- [ ] **`on_dies` unconditionally exiles regardless of current zone, violating ruling 2 about simultaneous Monstrosity deaths**

- [ ] `mtg-engine/src/cards/isd/moldgraf_monstrosity.rs` lines 48-51

- [ ] Oracle text says (per Scryfall ruling 2011-09-22): `"If two Moldgraf Monstrosities die simultaneously, the first ability to resolve could return the other Moldgraf Monstrosity to the battlefield. If i

- [ ] Code does: `state.move_object(object_id, Zone::Exile);` unconditionally, with no check that the object is currently in the Graveyard. When two Monstrosities die at once, the first trigger can return t

## moonmist

- [ ] Second Moonmist fails to transform Werewolf DFCs after they naturally untransform back to their Human front face

- [ ] Oracle text says: `Transform all Humans.`

- [ ] Code does: In `on_resolve` (lines 43–56 of `mtg-engine/src/cards/isd/moonmist.rs`), the Human-detection logic first checks `if !o.subtypes.is_empty()` and, when that is true, uses only `o.subtypes` wi

## moorland_haunt

- [ ] Player cannot choose which creature card to exile when multiple are in the graveyard (`mtg-engine/src/cards/isd/moorland_haunt.rs` lines 85–96 and `mtg-engine/src/engine.rs` lines 399–406)

- [ ] Oracle text says: `Exile a creature card from your graveyard:`

- [ ] Code does: In `on_activate_ability`, auto-selects the first creature card found via `.next()` on a `HashMap` iterator (non-deterministic order). In `legal_actions`, a single `ActivateAbility { targets

## murder_of_crows

- [ ] Simultaneous death: Murder of Crows' triggered ability does not fire when it dies at the same time as another creature — `mtg-engine/src/triggers.rs:418-440`

- [ ] Oracle text says: `"Whenever another creature dies, you may draw a card. If you do, discard a card."` and the ruling says: `"If another creature dies at the same time as Murder of Crows, its last abil

- [ ] Code does: In `collect_triggers`, the death-watch watcher scan is `filter(|o| o.zone == Zone::Battlefield && o.id != dead_id)`. In `destruction.rs:destroy()`, the `CreatureDied` event is pushed and th

## naturalize

- [ ] `is_valid_target` checks only registry data for card types, missing artifact/enchantment tokens

- [ ] File: `mtg-engine/src/cards/isd/naturalize.rs`, lines 40–42

- [ ] Oracle text says: `Destroy target artifact or enchantment.` (no restriction on tokens)

- [ ] Code does: `registry.card_data(obj.card_id).map(|d| d.card_types.contains(&CardType::Artifact) || d.card_types.contains(&CardType::Enchantment)).unwrap_or(false)` — tokens always have `card_id: CardId

## nevermore

- [ ] **Auto-selection of card name instead of player choice** (`mtg-engine/src/cards/isd/nevermore.rs:41-53`)

- [ ] Oracle text says: `"As this enchantment enters, choose a nonland card name."`

- [ ] Code does: Auto-selects the first nonland card from the *opponent's hand* (with a hardcoded fallback to `"Lightning Bolt"` if the opponent's hand has no nonland cards). The controller is never asked t
  ```rust
  let chosen_name = state.objects.values()
  .filter(|o| o.zone == Zone::Hand && o.owner == opponent)

- [ ] **Nevermore ban not enforced for flashback casts** (`mtg-engine/src/engine.rs:665-747`)

- [ ] Oracle text says: `"Spells with the chosen name can't be cast."`

- [ ] Code does: The Nevermore ban is checked only in the "Cast spells from hand" section (`engine.rs:488-491`). The "Cast spells via flashback from graveyard" section (`engine.rs:665-747`) contains no Neve
  ```rust
  // Check Nevermore: spells with the banned name can't be cast.
  if nevermore_banned.iter().any(|n| *n == data.name) {

## night_terrors

- [ ] **Night Terrors is never moved off the stack when the target player has multiple nonland cards in hand** (`mtg-engine/src/cards/isd/night_terrors.rs:63-70`, `mtg-engine/src/engine.rs:2003-2008`)

- [ ] Oracle text says: `"Exile that card."` — Night Terrors must fully resolve (including cleanup to graveyard) after exiling the chosen card.

- [ ] Code does: When `nonland_cards.len() > 1`, `on_resolve` calls `present_target_choice(... PendingEffect::ExileAndStore ... false)` and returns early (line 70) without calling `move_spell_after_resolve`

- [ ] **Wrong `PendingEffect` variant used for Night Terrors** (`mtg-engine/src/cards/isd/night_terrors.rs:66`)

- [ ] Oracle text says: `"Exile that card."` — Night Terrors is a sorcery with no LTB ability; it simply exiles the chosen card permanently.

- [ ] Code does: Uses `PendingEffect::ExileAndStore { source_id: object_id, source_name: "Night Terrors".into() }`. The `ExileAndStore` handler (engine.rs:2259-2261) writes `source_obj.card_state.insert("ex

## olivia_voldaren

- [ ] `TargetFilter::HasSubtype` in `matches_ability_target_filter` only checks `obj.subtypes`, not registry card data subtypes — ability 1 cannot target real Vampire creature cards

- [ ] `mtg-engine/src/engine.rs` lines 1266–1268 (and duplicated at line 1397 in `matches_target_filter`)

- [ ] Oracle text says: `{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.`

- [ ] Code does: `TargetFilter::HasSubtype(subtype) => { obj.subtypes.contains(subtype) }` — only checks `obj.subtypes`, which is `Vec::new()` for all non-token creature cards. Regular Vampire cards (Markov

- [ ] In-card guard check for ability 1 also only checks `obj.subtypes`, missing real Vampire cards

- [ ] `mtg-engine/src/cards/isd/olivia_voldaren.rs` lines 129–131

- [ ] Oracle text says: `{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.`

- [ ] Code does: `let is_vampire = state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield && o.subtypes.contains(&"Vampire".to_string())).unwrap_or(false);` — the same incomplete check. Even if a 

## parallel_lives

- [ ] Extra Parallel Lives copies do not inherit post-creation token properties ("tapped", "tapped and attacking", dynamic P/T, combat assignment, delayed exile), because `create_token_with_subtypes` only r

- [ ] Oracle text says (ruling 2023-09-01): `"Everything that is specified by the effect creating the original token or tokens will also be true about the additional token or tokens created by Parallel Live

- [ ] Code does (`mtg-engine/src/state.rs` lines 337–348):
  ```rust
  // Create the primary token.
  let id = self.create_token_internal(name, owner, power, toughness,

## paraselene

- [ ] Enchantment detection only checks the registry, missing enchantment tokens (`mtg-engine/src/cards/isd/paraselene.rs` lines 36–40)

- [ ] Oracle text says: `"Destroy all enchantments."`

- [ ] Code does: `registry.card_data(o.card_id).map(|d| d.card_types.contains(&CardType::Enchantment)).unwrap_or(false)` — for token objects, `card_id` is the sentinel `CardId(0)`, so `registry.card_data(Ca

## past_in_flames

- [ ] `until_end_of_turn_flashback` is never cleared at end-of-turn cleanup, so flashback grants persist indefinitely across turns.

- [ ] Oracle text says: `gains flashback until end of turn`

- [ ] Code does: `mtg-engine/src/engine.rs` lines 3020–3025 clear `until_end_of_turn_effects`, `until_end_of_turn_keywords`, `until_end_of_turn_cant_block`, `until_end_of_turn_protection`, and `until_end_of

- [ ] Cards with no mana cost that are instants or sorceries receive `ManaCost::free()` as their flashback cost, making them castable for {0} when the oracle text and ruling say they cannot be cast via flas

- [ ] Oracle text says: `The flashback cost is equal to its mana cost.` — Ruling: `If a card with no mana cost gains flashback, it has no flashback cost. It can't be cast this way.`

- [ ] Code does: `mtg-engine/src/cards/isd/past_in_flames.rs` line 53: `d.cost.clone().unwrap_or(ManaCost::free())` — when `d.cost` is `None` (card has no mana cost), `ManaCost::free()` is used as the grant

## pitchburn_devils

- [ ] `any_targets` helper omits planeswalkers; Pitchburn Devils' trigger cannot target them

- [ ] Oracle text says: `it deals 3 damage to any target`

- [ ] Code does: `let targets = crate::cards::helpers::any_targets(state);` which calls `creature_targets` (filtered by `o.power.is_some()`) plus all players — but planeswalkers have `power: None` and are s

## prey_upon

- [ ] **fight() emits CombatDamageDealt instead of NonCombatDamageDealt, applying combat-specific effects to fight damage** (`mtg-engine/src/combat.rs:467`, `mtg-engine/src/combat.rs:429–436`, `mtg-engine/s

- [ ] Oracle text says: `(Each deals damage equal to its power to the other.)` — this is reminder text for the fight keyword action, which is non-combat damage. The engine itself distinguishes the two in `e

- [ ] Code does: `deal_damage_to_creature` in `fight()` emits `GameEvent::CombatDamageDealt { source, target: DamageTarget::Object(target), amount, }` (line 467–471). Because `fight()` calls the same `deal_
  1. `has_damage_prevention` (`PreventCombatDamage`) at line 430: `if has_damage_prevention(state, source, registry) || has_damage_prevention(state, target, registry) { return; }` — Ghostly Possession's
  2. `is_non_wolf_damage_prevented` at line 435: `if is_non_wolf_damage_prevented(state, source, registry) { return; }` — Moonmist's combat-damage-only prevention incorrectly blocks fight damage from no
  3. `combat_damage_multiplier` at lines 452–454: `amount *= combat_damage_multiplier(state, source, registry); amount *= combat_damage_multiplier(state, target, registry);` — Inquisitor's Flail's `Doub

- [ ] Additionally, because `CombatDamageDealt` (not `NonCombatDamageDealt`) is emitted, the trigger system (`triggers.rs:459–487`) fires `DealsCombatDamageToCreature` triggers during fight. Concretely: if 

- [ ] **One illegal target does not prevent fight, violating the Scryfall ruling** (`mtg-engine/src/stack.rs:79–86`, `mtg-engine/src/cards/isd/prey_upon.rs:35–52`, `mtg-engine/src/combat.rs:158–168`)

- [ ] Oracle text says (ruling): `If either target is an illegal target as Prey Upon resolves, no creature will deal or be dealt damage.`

- [ ] Code does: `stack.rs:80`: `let any_legal = targets.iter().any(|t| is_target_legal(state, t, &target_req)); if !any_legal { state.log(..., format!("{} fizzled (all targets illegal)", name)); ... return

## rage_thrower

- [ ] **Issue 1 — Simultaneous death: Rage Thrower's trigger does not fire for a creature dying at the same time** (`mtg-engine/src/triggers.rs`, lines 418–440)

- [ ] Oracle text says (ruling 2011-09-22): `"If Rage Thrower dies at the same time as another creature, its ability will trigger."`

- [ ] Code does: In `collect_triggers`, the death-watch watcher scan is `state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.id != dead_id)`. When multiple creatures die in the same SBA pass 

- [ ] **Issue 2 — DeathWatch trigger incorrectly fizzles if Rage Thrower leaves the battlefield after triggering but before resolution** (`mtg-engine/src/triggers.rs`, lines 906–912; `mtg-engine/src/cards/i

- [ ] Oracle text says: `"Whenever another creature dies, this creature deals 2 damage to target player or planeswalker."` — no condition that Rage Thrower remains on the battlefield at resolution. Per CR 1

- [ ] Code does: `resolve_next_trigger` for `PendingTrigger::DeathWatch` guards with `if state.get_object(watcher_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` — if Rage Thrower is bounced, des

- [ ] **Issue 3 — Damage targeting a planeswalker does not reduce loyalty counters** (`mtg-engine/src/engine.rs`, lines 2179–2191; `mtg-engine/src/cards/isd/rage_thrower.rs`, line 56)

- [ ] Oracle text says: `"this creature deals 2 damage to target player or planeswalker"` — damage dealt to a planeswalker reduces its loyalty counters by the damage amount (MTG rule 306.6).

- [ ] Code does: Rage Thrower uses `PendingEffect::DealDamage { amount: 2, ... }` (rage_thrower.rs line 56). In `apply_pending_effect`, the `(Target::Object(id), PendingEffect::DealDamage { ... })` arm only

- [ ] **Issue 4 — Trigger description and target-choice prompt omit "or planeswalker"** (`mtg-engine/src/cards/isd/rage_thrower.rs`, lines 33 and 57)

- [ ] Oracle text says: `"deals 2 damage to target player or planeswalker"`

- [ ] Code does: `TriggeredAbilityDef { description: "deal 2 damage to target player".into() }` (line 33) and `present_target_choice(... "Rage Thrower: deal 2 damage to target player" ...)` (line 57). Both 

## reaper_from_the_abyss

- [ ] Intervening-if clause not enforced at trigger collection time (`mtg-engine/src/triggers.rs:604–641`, `mtg-engine/src/cards/isd/reaper_from_the_abyss.rs:34–37,47–49`)

- [ ] Oracle text says: `"Morbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature."`

- [ ] Code does: In `collect_triggers`, the `StepStarted { step: Step::EndStep }` handler (triggers.rs lines 597–642) unconditionally queues an `EndStepTrigger` for any permanent whose `TriggerKind::EndStep

## reckless_waif

- [ ] **Engine never populates `spells_cast_this_turn` or `spells_cast_last_turn`; both conditions are permanently broken**

- [ ] Oracle text says (front face): `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."`

- [ ] Oracle text says (back face): `"At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."`

- [ ] Card does (front face check, `mtg-engine/src/cards/isd/reckless_waif.rs:12-14`): `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum(); if !is_transformed { total_spells_last_t

- [ ] Card does (back face check, `mtg-engine/src/cards/isd/reckless_waif.rs:16`): `state.spells_cast_last_turn.values().any(|&count| count >= 2)`

- [ ] Engine does (`mtg-engine/src/engine.rs` CastSpell handler, lines 1479–1666): never increments `spells_cast_this_turn` when a spell is cast — zero matches for `spells_cast_this_turn` in the entire file

- [ ] Engine does (`mtg-engine/src/engine.rs` `advance_step`, lines 2882–2895): never transfers `spells_cast_this_turn` into `spells_cast_last_turn` at turn end — the only fields cleared are `is_first_turn`

- [ ] Net effect: `spells_cast_last_turn` is always an empty `HashMap`. Therefore `total_spells_last_turn` is always 0 → front face condition is permanently true after turn 1 (waif always transforms on ever

## rooftop_storm

- [ ] Rooftop Storm alternative cost not offered for Zombie creature spells cast from the graveyard

- [ ] Oracle text says: `"You may pay {0} rather than pay the mana cost for Zombie creature spells you cast."`

- [ ] Code does: The graveyard-casting loop in `mtg-engine/src/engine.rs` (lines 665–748) does not apply the Rooftop Storm alternative cost. At line 708: `if !mana::can_pay(&player_state.mana_pool, fb_cost)

## selhoff_occultist

- [ ] Selhoff Occultist's `AnyCreatureDies` trigger does not fire for creatures that die simultaneously with it (e.g., in a board wipe).

- [ ] Oracle text says: `"Whenever this creature or another creature dies, target player mills a card."`

- [ ] Code does: The death-watch watcher scan in `mtg-engine/src/triggers.rs:418` filters watchers with `o.zone == Zone::Battlefield`. By the time `collect_triggers` runs after SBA processing, the Occultist

## sensory_deprivation

- [ ] Engine does not check hexproof when evaluating target legality at resolution time

- [ ] Oracle text says: `Enchant creature` (targeting rules apply; per CR 608.2b, a target that gains hexproof after being chosen is an illegal target at resolution, and the spell is countered by game rules

- [ ] Code does: `is_target_legal` in `mtg-engine/src/stack.rs:8-41` only checks zone legality (`obj.zone == Zone::Battlefield`), not whether the target has gained hexproof since the spell was cast. `resolv

## sever_the_bloodline

- [ ] Engine does not re-check hexproof legality at resolution time (`mtg-engine/src/stack.rs:8-41`)

- [ ] Oracle text says (via ruling 2025-01-24): `"If the target creature is an illegal target by the time Sever the Bloodline tries to resolve, the spell won't resolve. You won't exile any creatures at all.

- [ ] A creature is an illegal target for an opponent's spell if it has hexproof (CR 608.2b). If the targeted creature gains hexproof in response to Sever the Bloodline (e.g., via Ranger's Guile, which is i

- [ ] Code does: `is_target_legal` only checks zone (`_ => obj.zone == Zone::Battlefield || obj.zone == Zone::Stack`), not hexproof. The `can_be_targeted` hexproof check runs only during legal action genera

- [ ] This is an engine-wide issue documented in `mtg-engine/tests/spell_fizzle.rs:192-226` (`bolt_target_gains_hexproof_before_resolution`), which confirms the engine does not re-check hexproof at resoluti

## sharpened_pitchfork

- [ ] "As long as" condition is evaluated once at equip time, not continuously re-evaluated.

- [ ] Oracle text says: `As long as equipped creature is a Human, it gets +1/+1.`

- [ ] Code does: `update_effects` (sharpened_pitchfork.rs:14–34) evaluates `is_human` at the moment of equipping (called only from `on_activate_ability` at line 90) and bakes the result into `instance_conti

- [ ] Human subtype check in `update_effects` only reads registry data, not runtime `obj.subtypes`, missing Human tokens.

- [ ] Oracle text says: `As long as equipped creature is a Human, it gets +1/+1.`

- [ ] Code does (sharpened_pitchfork.rs:15–18):
  ```rust
  let is_human = state.get_object(creature_id)
  .and_then(|o| registry.card_data(o.card_id))

## silver_inlaid_dagger

- [ ] "As long as" Human condition is evaluated once at equip time and never re-evaluated

- [ ] Oracle text says: `"As long as equipped creature is a Human, it gets an additional +1/+0."`

- [ ] Code does: `update_effects()` (silver_inlaid_dagger.rs lines 15–29) is called only from `on_activate_ability()` (line 86). It sets `instance_continuous_effects` once, based on the Human check at equip

- [ ] `update_effects()` does not check `o.subtypes` when detecting Human subtype, missing token Humans

- [ ] Oracle text says: `"As long as equipped creature is a Human, it gets an additional +1/+0."`

- [ ] Code does: `state.get_object(creature_id).and_then(|o| registry.card_data(o.card_id)).map(|d| d.subtypes.iter().any(|s| s == "Human")).unwrap_or(false)` (silver_inlaid_dagger.rs lines 16–19). This onl

## skaab_goliath

- [ ] Engine auto-selects which creatures to exile rather than giving the player the choice (`mtg-engine/src/engine.rs:1574–1600`)

- [ ] Oracle text says: `"As an additional cost to cast this spell, exile two creature cards from your graveyard."`

- [ ] Code does: `exile_candidates.sort_by(|a, b| b.1.cmp(&a.1)); // Highest power first` then `let exile_candidates: Vec<_> = exile_candidates.into_iter().take(n).collect();` — the engine silently picks th

## skaab_ruinator

- [ ] Engine auto-selects which creature cards to exile as the additional cost, rather than presenting the player with a choice (`mtg-engine/src/engine.rs` lines 1574–1600)

- [ ] Oracle text says: `"As an additional cost to cast this spell, exile three creature cards from your graveyard."`

- [ ] Code does: `exile_candidates.sort_by(|a, b| b.1.cmp(&a.1)); // Highest power first` then `let exile_candidates: Vec<_> = exile_candidates.into_iter().take(n).collect();` — auto-selects the three highe

## skirsdag_cultist

- [ ] **`AnyTarget` does not include planeswalkers as valid targets** — `mtg-engine/src/engine.rs` lines 1343–1358 (activated ability target generation) and lines 1074–1089 (spell target generation)

- [ ] Oracle text says: `This creature deals 2 damage to any target.`

- [ ] Code does: `generate_ability_targets` for `TargetRequirement::AnyTarget` filters for `o.power.is_some()` (creatures only) and then adds players. Planeswalkers — which have no `power` set and are not p

- [ ] **Engine auto-selects which creature to sacrifice instead of presenting a player choice** — `mtg-engine/src/engine.rs` lines 1750–1759

- [ ] Oracle text says: `Sacrifice a creature:` (controller chooses which creature to sacrifice)

- [ ] Code does: `let creature = new_state.objects_in_zone(Zone::Battlefield, player).iter().find(|o| o.power.is_some()).map(|o| o.id);` — the engine auto-picks the first creature found in iteration order (

## skirsdag_high_priest

- [ ] Auto-selection of which two creatures to tap — `mtg-engine/src/cards/isd/skirsdag_high_priest.rs` lines 68–73

- [ ] Oracle text says: `{T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying.`

- [ ] Code does: `let to_tap: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller).iter().filter(|o| o.id != object_id && o.power.is_some() && !o.tapped).take(2).map(|o| o.id).collect();`

- [ ] "Tap two untapped creatures you control" is a cost the player pays, meaning the player must choose which two untapped creatures to tap. When the controller has more than two untapped creatures (beside

## slayer_of_the_wicked

- [ ] Subtype check only reads registry data, missing token subtypes (`slayer_of_the_wicked.rs` lines 41–43)

- [ ] Oracle text says: `"you may destroy target Vampire, Werewolf, or Zombie"`

- [ ] Code does: `registry.card_data(o.card_id).map(|d| d.subtypes.iter().any(|s| s == "Vampire" || s == "Werewolf" || s == "Zombie")).unwrap_or(false)` — tokens are created with `card_id: CardId(0)` (senti

## smite_the_monstrous

- [ ] Target legality at resolution does not re-check the power condition (`mtg-engine/src/stack.rs:8-41`)

- [ ] Oracle text says: `Destroy target creature with power 4 or greater.`

- [ ] Code does: `is_target_legal` in `stack.rs` checks only zone for `CreatureWithFilter(_)` targets — it falls through to the wildcard branch `_ => obj.zone == Zone::Battlefield || obj.zone == Zone::Stack

## snapcaster_mage

- [ ] **`until_end_of_turn_flashback` is never cleared at end of turn** (`mtg-engine/src/engine.rs:3006–3061`)

- [ ] Oracle text says: `gains flashback until end of turn`

- [ ] Code does: The Cleanup step clears `until_end_of_turn_effects`, `until_end_of_turn_keywords`, `until_end_of_turn_cant_block`, `until_end_of_turn_protection`, and `until_end_of_turn_removed_keywords`, 

- [ ] **Snapcaster Mage incorrectly excludes cards with innate flashback from eligible targets** (`mtg-engine/src/cards/isd/snapcaster_mage.rs:48–53`)

- [ ] Oracle text says: `target instant or sorcery card in your graveyard` (no restriction on whether the card already has flashback)

- [ ] Code does: `.filter(|o| { registry.card_data(o.card_id).map(|d| { (d.card_types.contains(&CardType::Instant) || d.card_types.contains(&CardType::Sorcery)) && d.flashback_cost.is_none() }).unwrap_or(fa

## spare_from_evil

- [ ] Protection's "T" (targeting) aspect not enforced by engine — non-Human creature activated abilities can still target protected creatures

- [ ] Oracle text says: `"gain protection from non-Human creatures until end of turn"` (protection means DEBT: Damage, Enchanting, Blocking, Targeting prevented from non-Human creature sources)

- [ ] Code does: `can_be_targeted` in `mtg-engine/src/engine.rs` line 758 only checks hexproof: `if state.has_keyword(target_id, Keyword::Hexproof, registry)`. It does not consult `until_end_of_turn_protect

- [ ] Protection's "D" (damage) aspect not enforced for non-combat damage from non-Human creature sources

- [ ] Oracle text says: `"gain protection from non-Human creatures until end of turn"` (protection prevents all damage from non-Human creature sources, not just combat damage)

- [ ] Code does: `apply_pending_effect` in `mtg-engine/src/engine.rs` lines 2154–2191 for `PendingEffect::DealDamage` does not check `until_end_of_turn_protection`. It only checks `PreventDamageRemoveCounte

## splinterfright

- [ ] Upkeep trigger does not resolve if Splinterfright has left the battlefield between trigger collection and resolution

- [ ] Oracle text says: `"At the beginning of your upkeep, mill two cards."`

- [ ] Code does: In `mtg-engine/src/triggers.rs:954-959`, `resolve_next_trigger` checks `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` before calling `on_upkeep`. If 

## stitched_drake

- [ ] Engine auto-selects which creature to exile; player cannot choose

- [ ] Oracle text says: `"As an additional cost to cast this spell, exile a creature card from your graveyard."`

- [ ] Code does: In `mtg-engine/src/engine.rs` around line 1574–1600, the engine picks creatures to exile by sorting descending by power (`exile_candidates.sort_by(|a, b| b.1.cmp(&a.1)); // Highest power fi

## stitchers_apprentice

- [ ] Engine bug: `trigger_event_index` desync causes `CreatureDied` events from the sacrifice to be skipped when ETB-watch permanents (Champion of the Parish, Mentor of the Meek, Dearly Departed) are prese

- [ ] Oracle text says: `{1}{U}, {T}: Create a 2/2 blue Homunculus creature token, then sacrifice a creature.`

- [ ] Mechanism: After `ActivateAbility` resolves, `process_triggers` is called. `collect_triggers` processes the `EnteredBattlefield` event (index 0), sets `state.trigger_event_index = 1` (`triggers.rs:873

- [ ] When the player then submits `ResolveChoice` for the sacrifice, `submit_action` clones the state (copying `trigger_event_index = 1`) and calls `new_state.events.clear()` (`engine.rs:1450`) — clearing 

- [ ] Code does: `new_state.events.clear()` in `engine.rs:1450` without resetting `new_state.trigger_event_index`, combined with `state.trigger_event_index = events.len()` in `triggers.rs:873` persisting ac

## sturmgeist

- [ ] Draw skipped when Sturmgeist leaves battlefield before trigger resolves (`mtg-engine/src/cards/isd/sturmgeist.rs:46-49`)

- [ ] Oracle text says: `"Whenever this creature deals combat damage to a player, draw a card."`

- [ ] Code does: `let controller = match state.get_object(self_id) { Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return, };` — if Sturmgeist is not in `Zone::Battlefield` when the trigger r

## sulfur_falls

- [ ] `controller_has_matching_land` only checks `o.subtypes` (runtime object field) but never consults the registry, so it fails to detect basic Island and Mountain cards on the battlefield. The function s

- [ ] Oracle text says: `"This land enters tapped unless you control an Island or a Mountain."`

- [ ] Code does: `o.subtypes.iter().any(|s| s == "Island") || o.subtypes.iter().any(|s| s == "Mountain")` — `o.subtypes` is `Vec::new()` for all regular card objects (`mtg-engine/src/cards/isd/sulfur_falls.

- [ ] Contrast with the correct dual-check pattern used in `state.rs` `check_condition` (lines 1085–1092): `o.subtypes.iter().any(|s| s == subtype) || registry.card_data(o.card_id).map(|d| d.subtypes.iter()

## thraben_sentry

- [ ] **"you may" is bypassed — card always auto-transforms, player never gets a choice** (`mtg-engine/src/cards/isd/thraben_sentry.rs`, lines 72–76)

- [ ] Oracle text says: `"you may transform this creature"`

- [ ] Code does: `// Auto-transform (simplified "you may" — always yes). if let Some(obj) = state.get_object_mut(self_id) { obj.is_transformed = true; obj.name = "Thraben Militia".into(); }`

- [ ] The engine has a `YesNo` resolution-choice mechanism (`AwaitingAction::ResolutionChoice { choice: ResolutionChoiceKind::YesNo { ... } }`) used by other "you may" DFC cards (e.g., Screeching Bat, Clois

- [ ] **Vigilance incorrectly retained on back face after transform** (`mtg-engine/src/cards/isd/thraben_sentry.rs`, lines 73–76)

- [ ] Oracle text says (back face): `"Trample"` (Thraben Militia has Trample; Vigilance is only on the front face)

- [ ] Code does: sets `obj.is_transformed = true` and `obj.name = "Thraben Militia".into()` but does **not** update `obj.keywords`. The object's `keywords` field remains `[Vigilance]` (the front face value)

- [ ] `has_keyword()` in `state.rs` checks `obj.keywords` **first** (step 0, line 1000): `if obj.keywords.contains(&keyword) { return true; }`. Because `obj.keywords` still holds `[Vigilance]`, `has_keyword

- [ ] **Test enshrines wrong auto-transform behavior** (`mtg-engine/tests/tier15_cards.rs`, lines 1392–1409, test `thraben_sentry_transforms_when_creature_dies`)

- [ ] The test calls `on_any_creature_dies` directly and asserts `is_transformed == true` without checking for `state.awaiting_action`. If the bug were fixed to use `YesNo`, the transform would not happen i

## tormented_pariah

- [ ] **Engine never tracks spells cast per turn; `spells_cast_last_turn` is always empty in real gameplay** (`mtg-engine/src/engine.rs`, `mtg-engine/src/state.rs`)

- [ ] Oracle text says: `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."` / `"At the beginning of each upkeep, if a player cast two or more spells last turn, tr

- [ ] Code does: `state.spells_cast_this_turn` is defined in `state.rs:127` as `HashMap<PlayerId, u32>` and initialized to `HashMap::new()` (`state.rs:230`) but is **never incremented anywhere in the engine

- [ ] **Log message incorrectly names source when Rampaging Werewolf transforms back** (`mtg-engine/src/cards/isd/tormented_pariah.rs:87–88`)

- [ ] Oracle text says: the creature is named "Rampaging Werewolf" while in its back-face form.

- [ ] Code does: `format!("Tormented Pariah transforms into {}", name)` — when the card is on its back face and transforms back, `name` is `"Tormented Pariah"`, yielding the log string `"Tormented Pariah tr

## traitorous_blood

- [ ] Control change is never reverted at end of turn — engine cleanup step does not process `until_end_of_turn_control_changes`

- [ ] Oracle text says: `"Gain control of target creature until end of turn."`

- [ ] Code does: `state.until_end_of_turn_control_changes.push((*creature_id, original));` records the original controller, but `mtg-engine/src/engine.rs` lines 3020–3025 (the cleanup step) only clears `unt

## travel_preparations

- [ ] LLM card knowledge in `mtg-player/src/llm.rs` line 111 describes one target instead of up to two

- [ ] Oracle text says: `"Put a +1/+1 counter on each of up to two target creatures."`

- [ ] Code does: `"- Travel Preparations ({1}{G} sorcery, flashback {1}{W}): Put a +1/+1 counter on target creature."` — says "target creature" (singular, no "up to two"), so the LLM player will never plan 

## travelers_amulet

- [ ] **No player choice when multiple basic lands exist** (`mtg-engine/src/cards/isd/travelers_amulet.rs:57`)

- [ ] Oracle text says: `Search your library for a basic land card`

- [ ] Code does: `player.library_order.iter().find(|&&lib_id| { ... })` — auto-selects the first matching basic land in library order, never presenting a choice to the player. The engine has a `ChooseFromLi

- [ ] **"then shuffle" is not implemented** (`mtg-engine/src/cards/isd/travelers_amulet.rs:83`)

- [ ] Oracle text says: `then shuffle`

- [ ] Code does: `// Shuffle (no-op in our engine, library is treated as ordered for gameplay).` — no shuffle is performed. This comment is factually incorrect: the engine supports real shuffling via `rand:

## tribute_to_hunger

- [ ] Missing `is_valid_target` override to enforce "target opponent" restriction

- [ ] Oracle text says: `"Target opponent sacrifices a creature of their choice."`

- [ ] Code does: `fn target_requirement(&self) -> TargetRequirement { TargetRequirement::PlayerOnly }` with no `is_valid_target` override. The default `is_valid_target` (in `cards/mod.rs:290`) returns `true

## ulvenwald_mystics

- [ ] Engine never increments `spells_cast_this_turn` and never transfers it to `spells_cast_last_turn`; both transform conditions are permanently wrong in real gameplay

- [ ] File: `mtg-engine/src/engine.rs` CastSpell handler (lines 1479–1666): no increment of `spells_cast_this_turn` when a spell is cast; `advance_step` turn-end transition (lines 2867–2895): no rollover of

- [ ] Oracle text says (front face): `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."`

- [ ] Oracle text says (back face): `"At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."`

- [ ] Code does (`mtg-engine/src/cards/isd/ulvenwald_mystics.rs` lines 15–19): `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum();` / `total_spells_last_turn == 0 && !state.is_fir

- [ ] Log message is incorrect when transforming from back face (Ulvenwald Primordials) to front face (Ulvenwald Mystics)

- [ ] File: `mtg-engine/src/cards/isd/ulvenwald_mystics.rs` line 117

- [ ] Oracle text says: `"transform this creature"` (the source is the current face, Ulvenwald Primordials)

- [ ] Code does: `format!("Ulvenwald Mystics transforms into {}", name)` — hardcodes "Ulvenwald Mystics" as the source regardless of which face is currently active. When back→front occurs, `name = "Ulvenwal

## unbreathing_horde

- [ ] "Enters with counters" replacement effect does not fire when Unbreathing Horde enters the battlefield via reanimation (e.g., Unburial Rites)

- [ ] Oracle text says: `"This creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard."`

- [ ] Code does: The counter logic lives entirely in `on_resolve` (`mtg-engine/src/cards/isd/unbreathing_horde.rs:42-81`). When Unburial Rites reanimates the Horde it calls `state.move_object(id, Zone::Batt
  Additionally, the ruling "If Unbreathing Horde enters from a graveyard, it will count itself when determining how many +1/+1 counters it enters with" is also unimplemented for the reanimation path: by

## unburial_rites

- [ ] **Missing `target_requirement()` override — spell treated as untargeted**

- [ ] File: `mtg-engine/src/cards/isd/unburial_rites.rs`, line 11–65

- [ ] Oracle text says: `"Return target creature card from your graveyard to the battlefield."`

- [ ] Code does: `UnburialRites` does not override `target_requirement()`. The `CardBehavior` trait default (see `cards/mod.rs` line 284–286) returns `TargetRequirement::None`. This means the engine generat

- [ ] **Target selected at resolution time, not at cast time — ignores `targets` parameter**

- [ ] File: `mtg-engine/src/cards/isd/unburial_rites.rs`, lines 31–64; specifically the `_targets` parameter at line 31

- [ ] Oracle text says: `"Return target creature card from your graveyard to the battlefield."` (target is chosen at cast time per CR 601.2c)

- [ ] Code does: `fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry)` — the `_targets` parameter is explicitly marked unused (underscore prefix).

- [ ] **Spell can be cast with no legal targets**

- [ ] File: `mtg-engine/src/cards/isd/unburial_rites.rs` (missing `target_requirement`) / `mtg-engine/src/engine.rs` line 833

- [ ] Oracle text says: `"Return target creature card from your graveyard to the battlefield."` (CR 601.2c: you may not cast a spell that requires targets if there are no legal targets)

- [ ] Code does: Because `target_requirement()` returns `TargetRequirement::None`, the engine always generates a cast action for Unburial Rites regardless of whether any creature cards exist in the controll

## undead_alchemist

- [ ] **Second triggered ability only fires from Undead Alchemist's own mill, not from all sources** (`mtg-engine/src/cards/isd/undead_alchemist.rs:82-99`)

- [ ] Oracle text says: `"Whenever a creature card is put into an opponent's graveyard from their library, exile that card and create a 2/2 black Zombie creature token."`

- [ ] Code does: The exile-and-create-token logic is implemented inline inside `on_any_combat_damage_to_player` (lines 82–99), which is only called when a Zombie deals combat damage. There is no separate tr

- [ ] **Multiple Undead Alchemists cause incorrect life restoration (net life gain) and double milling** (`mtg-engine/src/cards/isd/undead_alchemist.rs:63-99`)

- [ ] Oracle text says (ruling 2011-09-22): `"If you control multiple Undead Alchemists, the multiple replacement abilities will have no added effect. Combat damage dealt to a player by a Zombie you control

- [ ] Code does: Each Alchemist independently registers as a `TriggerKind::AnyCombatDamageToPlayer` watcher. When a Zombie deals X damage, triggers.rs creates one `CombatDamageWatch` trigger per Alchemist o

- [ ] **First-strike Zombie dealing lethal combat damage causes player loss before Alchemist trigger fires** (`mtg-engine/src/combat.rs:146-153`, `mtg-engine/src/cards/isd/undead_alchemist.rs:45-105`)

- [ ] Oracle text says: `"If a Zombie you control would deal combat damage to a player, instead that player mills that many cards."`

- [ ] Code does: In `combat.rs::deal_combat_damage` (line 146–147), after the first-strike damage step, SBAs are run synchronously (`while crate::sba::check_state_based_actions_with_registry(state, Some(reg

- [ ] **Lifelink on the Zombie source incorrectly grants life when Undead Alchemist's replacement applies** (`mtg-engine/src/combat.rs:539-549`, `mtg-engine/src/cards/isd/undead_alchemist.rs:45-105`)

- [ ] Oracle text says: `"If a Zombie you control would deal combat damage to a player, instead that player mills that many cards."` — no damage is dealt; the event is replaced.

- [ ] Code does: `deal_damage_to_player` applies lifelink gain immediately when damage is dealt (lines 539–549 of `combat.rs`), before any trigger fires. Because the Alchemist's replacement is modeled as a 

## unruly_mob

- [ ] Simultaneous death: trigger does not fire when Unruly Mob dies in the same SBA pass as another creature you control.

- [ ] Oracle text says: `Whenever another creature you control dies, put a +1/+1 counter on this creature.`

- [ ] Official ruling says: `If Unruly Mob and another creature you control die simultaneously (perhaps because they were both attacking or blocking), Unruly Mob won't be on the battlefield as its triggered

- [ ] Code does: In `mtg-engine/src/triggers.rs` lines 418–419, the DeathWatch watcher scan filters `o.zone == Zone::Battlefield`. By the time `collect_triggers` is called, all creatures killed in the same 

## urgent_exorcism

- [ ] `is_valid_target` only checks `registry.card_data(obj.card_id)` for subtypes/card_types, missing Spirit tokens

- [ ] Oracle text says: `Destroy target Spirit or enchantment.`

- [ ] Code does (`mtg-engine/src/cards/isd/urgent_exorcism.rs` lines 40–45):
  ```rust
  registry.card_data(obj.card_id)
  .map(|d| {

## vampiric_fury

- [ ] Vampire subtype check in `on_resolve` only reads `registry.card_data(obj.card_id)` and never checks `obj.subtypes` — `mtg-engine/src/cards/isd/vampiric_fury.rs:44-46`

- [ ] Oracle text says: `"Vampire creatures you control get +2/+0 and gain first strike until end of turn."`

- [ ] Code does: `registry.card_data(obj.card_id).map(|data| data.subtypes.iter().any(|s| s == "Vampire")).unwrap_or(false)` — this check ignores `obj.subtypes`, which means three concrete in-game cases are
  1. **Vampire tokens** (e.g., the 2/2 black Vampire token created by Bloodline Keeper's `{T}` ability): tokens have `card_id = CardId(0)` (sentinel), so `registry.card_data(CardId(0))` returns `None`, 
  2. **Olivia-made Vampires**: Olivia Voldaren's `{1}{R}` ability appends `"Vampire"` to `obj.subtypes` of the damaged creature (`olivia_voldaren.rs:108-109`). After the ability resolves, that creature 
  3. **Transformed Stalking Vampire** (Screeching Bat back face): `apply_transform` sets `obj.subtypes = ["Vampire"]` and `obj.is_transformed = true` (`helpers.rs:261`), but `card_id` still points to th

## victim_of_night

- [ ] `is_valid_target` does not check `obj.subtypes` for the excluded subtypes — only `registry.card_data(obj.card_id).subtypes` is checked. Tokens are created with `card_id: CardId(0)` (a sentinel with no

- [ ] Oracle text says: `Destroy target non-Vampire, non-Werewolf, non-Zombie creature.`

- [ ] Code does:
  ```rust
  if let Some(data) = registry.card_data(obj.card_id) {
  !data.subtypes.iter().any(|s| s == "Vampire" || s == "Werewolf" || s == "Zombie")

## village_bell_ringer

- [ ] ETB trigger resolution skipped if VBR leaves the battlefield before the trigger resolves (`mtg-engine/src/triggers.rs`, line 894–899)

- [ ] Oracle text says: `"When this creature enters, untap all creatures you control."`

- [ ] Code does: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { ... behavior.on_enter_battlefield(...) }` — if VBR is bounced or destroyed in response to its own ETB

## village_cannibals

- [ ] **Missing `o.subtypes` check for Human tokens** (`mtg-engine/src/cards/isd/village_cannibals.rs` lines 39–42)

- [ ] Oracle text says: `Whenever another Human creature dies, put a +1/+1 counter on this creature.`

- [ ] Code does:
  ```rust
  let is_human = state.get_object(dead_id)
  .and_then(|o| registry.card_data(o.card_id))

- [ ] **Spurious DeathWatch triggers on non-Human deaths** (`mtg-engine/src/triggers.rs` lines 422–441)

- [ ] Oracle text says: `Whenever another Human creature dies` — the trigger condition requires the dying creature to be a Human. Per MTG rules, the trigger only goes on the stack when this condition is met

- [ ] Code does: The DeathWatch watcher loop (lines 422–441) pushes a `PendingTrigger::DeathWatch` for Village Cannibals on **every** creature death, with no `if !desc.is_empty()` guard. Village Cannibals h

- [ ] **Simultaneous deaths: Village Cannibals doesn't trigger when it dies alongside a Human** (`mtg-engine/src/triggers.rs` lines 417–441, `mtg-engine/src/sba.rs` lines 53–147)

- [ ] Oracle text says: `Whenever another Human creature dies` — per MTG CR 704.3, simultaneous state-based deaths happen at the same time; a watcher that dies in the same event batch was on the battlefield

- [ ] Code does: `check_state_based_actions_with_registry` collects all creatures to destroy into `destroyed_ids`/`zero_toughness_ids` vectors, then processes them **sequentially**, moving each one to the g

## village_ironsmith

- [ ] Engine never tracks spells cast per turn, so transform conditions are always wrong in a real game (`mtg-engine/src/engine.rs` — turn transition in `advance_step`, and `CastSpell` handler in `submit_ac

- [ ] Oracle text says (front face): `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."`

- [ ] Oracle text says (back face): `"At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."`

- [ ] Code does: The card reads `state.spells_cast_last_turn` (front face, `village_ironsmith.rs:12`: `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum();`; back face, `village_iro

- [ ] Consequence: `total_spells_last_turn` is always 0, so Village Ironsmith (front face) always transforms on upkeep after turn 1, even when spells were cast. `spells_cast_last_turn.values().any(|&count| 

- [ ] Incorrect log message when Ironfang transforms back to Village Ironsmith (`mtg-engine/src/cards/isd/village_ironsmith.rs:87–88`)

- [ ] Oracle text: Ironfang transforms into Village Ironsmith (the source of the transform is Ironfang)

- [ ] Code does: `format!("Village Ironsmith transforms into {}", name)` — the prefix is hardcoded as "Village Ironsmith" regardless of which face is currently showing. When Ironfang transforms back, the lo

## villagers_of_estwald

- [ ] **Engine never populates `spells_cast_last_turn` in real games** (`mtg-engine/src/engine.rs`, `mtg-engine/src/state.rs`)

- [ ] Oracle text says (front): `"At the beginning of each upkeep, if no spells were cast last turn, transform this creature."`

- [ ] Oracle text says (back): `"At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."`

- [ ] Code does: `state.spells_cast_last_turn` is declared in `state.rs` (line 131) and initialized to an empty `HashMap::new()` (line 231), but is **never populated during play**. The `CastSpell` action ha

- [ ] `total_spells_last_turn` (sum over empty map) is always 0 → front-face condition `total_spells_last_turn == 0 && !state.is_first_turn` is always true after turn 1 → Villagers always transforms at ever

- [ ] `state.spells_cast_last_turn.values().any(|&count| count >= 2)` over an empty map is always false → Howlpack never transforms back, even after 2+ spells were cast.

- [ ] **Log message is wrong when Howlpack transforms back to Villagers** (`mtg-engine/src/cards/isd/villagers_of_estwald.rs`, line 88)

- [ ] Oracle text implies: a transform from Howlpack of Estwald back to Villagers of Estwald.

- [ ] Code does: `format!("Villagers of Estwald transforms into {}", name)` where `name` is "Villagers of Estwald" (because `obj.is_transformed` was just set to `false`). The log reads "Villagers of Estwald

## witchbane_orb

- [ ] **ETB trigger suppressed when Witchbane Orb leaves the battlefield before trigger resolves** — `mtg-engine/src/triggers.rs` lines 893–898

- [ ] Oracle text says: `"When this artifact enters, destroy all Curses attached to you."`

- [ ] Code does: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { behavior.on_enter_battlefield(state, object_id, registry); }` — the "destroy Curses" effect is silent

- [ ] **Hexproof not re-validated at spell resolution for player targets** — `mtg-engine/src/stack.rs` line 39

- [ ] Oracle text says: `"You have hexproof. (You can't be the target of spells or abilities your opponents control, including Aura spells.)"`

- [ ] Code does: `Target::Player(_) => true` — `is_target_legal` unconditionally considers every player target legal without checking hexproof. Per CR 608.2b, if a target becomes illegal between when a spel

## woodland_cemetery

- [ ] Subtype detection in `controller_has_matching_land` only checks `obj.subtypes` (object-level field), which is always empty for non-token regular cards — causing Woodland Cemetery to always enter tappe

- [ ] Oracle text says: `"This land enters tapped unless you control a Swamp or a Forest."`

- [ ] Code does: `o.subtypes.iter().any(|s| s == "Swamp") || o.subtypes.iter().any(|s| s == "Forest")` (`mtg-engine/src/cards/isd/woodland_cemetery.rs`, lines 19–22). For all non-token cards (including basi

## woodland_sleuth

- [ ] **Intervening-if condition not checked at trigger-collection time** (`mtg-engine/src/triggers.rs` lines 344–363)

- [ ] Oracle text says: `"When this creature enters, if a creature died this turn, return a creature card at random from your graveyard to your hand."`

- [ ] Code does: `if registry.get(card_id).is_some() { ... ap_triggers.push(trigger); ... }` — the ETB trigger is unconditionally pushed onto the stack whenever Woodland Sleuth enters the battlefield, with 

- [ ] **Woodland Sleuth cannot be returned to its own hand when it dies in response to its ETB trigger** — two bugs, both must be fixed:
  1. Engine guard in `mtg-engine/src/triggers.rs` lines 893–899:

- [ ] Ruling says: `"Woodland Sleuth could die in response to its own morbid ability. If this happens, the ability could return Woodland Sleuth to its owner's hand."`

- [ ] Code does: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { behavior.on_enter_battlefield(...) }` — the trigger resolution is entirely skipped when the Sleuth is
  2. Card-level guard in `mtg-engine/src/cards/isd/woodland_sleuth.rs` lines 45–48:

- [ ] Ruling says: `"Woodland Sleuth could die in response to its own morbid ability. If this happens, the ability could return Woodland Sleuth to its owner's hand."`

- [ ] Code does: `let controller = match state.get_object(object_id) { Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return, };` — even if the engine's check were fixed, the card itself early

