# ISD Audit Issues — Classified

Source: Sonnet 4.6 audit, 2026-04-04

## Summary

- **VERIFIED**: 115 issues
- **FALSE_POSITIVE**: 0 issues
- **NEEDS_REVIEW**: 175 issues
- **Total**: 290 issues

## VERIFIED issues by category

### engine missing feature (20 issues, 19 cards)

- **blazing_torch**: `AnyTarget` engine implementation does not include planeswalkers as valid targets
- **bloodcrazed_neonate**: Forced-attack logic in `engine.rs` (~line 1838) does not call `state.can_attack()`, so a Bloodcrazed Neonate enchanted with Pacifism (or any `PreventA
- **boneyard_wurm**: The underlying `dynamic_pt` function (`boneyard_wurm.rs:32–39`) is itself correct: it does not zone-restrict, and `objects_in_zone(Zone::Graveyard, co
- **brimstone_volley**: `AnyTarget` in engine does not include planeswalkers as valid targets — `mtg-engine/src/engine.rs` lines 836–864, 1074–1089, 1343–1358
- **demonmail_hauberk**: Additionally, in `legal_actions` (engine.rs lines 368–373), the engine only checks that at least one creature exists (`any(|o| o.power.is_some())`), t
- **essence_of_the_wild**: **EotW entering via non-`on_resolve` path does not apply replacement effect** — `mtg-engine/src/cards/isd/essence_of_the_wild.rs:40-53`
- **instigator_gang**: **Engine never increments `spells_cast_this_turn` or updates `spells_cast_last_turn`**: The fields `state.spells_cast_this_turn` and `state.spells_cas
- **memorys_journey**: **Missing mandatory `Target::Player` target — opponent cannot be targeted with 0 card targets, and player hexproof is never checked** (`mtg-engine/src
- **nevermore**: **Nevermore ban not enforced for flashback casts** (`mtg-engine/src/engine.rs:665-747`)
- **prey_upon**: **One illegal target does not prevent fight, violating the Scryfall ruling** (`mtg-engine/src/stack.rs:79–86`, `mtg-engine/src/cards/isd/prey_upon.rs:
- **reaper_from_the_abyss**: Intervening-if clause not enforced at trigger collection time (`mtg-engine/src/triggers.rs:604–641`, `mtg-engine/src/cards/isd/reaper_from_the_abyss.r
- **sensory_deprivation**: Engine does not check hexproof when evaluating target legality at resolution time
- **sever_the_bloodline**: Engine does not re-check hexproof legality at resolution time (`mtg-engine/src/stack.rs:8-41`)
- **sever_the_bloodline**: This is an engine-wide issue documented in `mtg-engine/tests/spell_fizzle.rs:192-226` (`bolt_target_gains_hexproof_before_resolution`), which confirms
- **skirsdag_cultist**: **`AnyTarget` does not include planeswalkers as valid targets** — `mtg-engine/src/engine.rs` lines 1343–1358 (activated ability target generation) and
- **smite_the_monstrous**: Target legality at resolution does not re-check the power condition (`mtg-engine/src/stack.rs:8-41`)
- **traitorous_blood**: Control change is never reverted at end of turn — engine cleanup step does not process `until_end_of_turn_control_changes`
- **travelers_amulet**: **"then shuffle" is not implemented** (`mtg-engine/src/cards/isd/travelers_amulet.rs:83`)
- **unburial_rites**: File: `mtg-engine/src/cards/isd/unburial_rites.rs` (missing `target_requirement`) / `mtg-engine/src/engine.rs` line 833
- **village_cannibals**: **Missing `o.subtypes` check for Human tokens** (`mtg-engine/src/cards/isd/village_cannibals.rs` lines 39–42)

### subtype check misses tokens (18 issues, 18 cards)

- **bonds_of_faith**: **Human subtype check at ETB only inspects registry data, missing object-level subtypes (tokens)** — `mtg-engine/src/cards/isd/bonds_of_faith.rs` line
- **clifftop_retreat**: `controller_has_matching_land` only checks runtime object subtypes (`o.subtypes`), never the registry — so real Mountain and Plains cards are never de
- **hinterland_harbor**: `controller_has_matching_land` only checks object-level subtypes (`o.subtypes`), missing registry-stored subtypes for regularly-played Forest/Island c
- **isolated_chapel**: Subtype check in `controller_has_matching_land` only reads `obj.subtypes` on game objects, missing subtypes stored in the registry for regular (non-to
- **kessig_cagebreakers**: `create_token_with_subtypes` (`state.rs:314-348`) creates the primary token and then creates extra copies for Parallel Lives, but returns only the pri
- **maw_of_the_mire**: `is_valid_target` only consults the registry for `CardType::Land`, missing land tokens whose types are stored on `obj.card_types` (not in registry)
- **naturalize**: `is_valid_target` checks only registry data for card types, missing artifact/enchantment tokens
- **olivia_voldaren**: `TargetFilter::HasSubtype` in `matches_ability_target_filter` only checks `obj.subtypes`, not registry card data subtypes — ability 1 cannot target re
- **parallel_lives**: Extra Parallel Lives copies do not inherit post-creation token properties ("tapped", "tapped and attacking", dynamic P/T, combat assignment, delayed e
- **paraselene**: Enchantment detection only checks the registry, missing enchantment tokens (`mtg-engine/src/cards/isd/paraselene.rs` lines 36–40)
- **sharpened_pitchfork**: Human subtype check in `update_effects` only reads registry data, not runtime `obj.subtypes`, missing Human tokens.
- **silver_inlaid_dagger**: `update_effects()` does not check `o.subtypes` when detecting Human subtype, missing token Humans
- **slayer_of_the_wicked**: Subtype check only reads registry data, missing token subtypes (`slayer_of_the_wicked.rs` lines 41–43)
- **sulfur_falls**: `controller_has_matching_land` only checks `o.subtypes` (runtime object field) but never consults the registry, so it fails to detect basic Island and
- **urgent_exorcism**: `is_valid_target` only checks `registry.card_data(obj.card_id)` for subtypes/card_types, missing Spirit tokens
- **vampiric_fury**: Vampire subtype check in `on_resolve` only reads `registry.card_data(obj.card_id)` and never checks `obj.subtypes` — `mtg-engine/src/cards/isd/vampiri
- **victim_of_night**: `is_valid_target` does not check `obj.subtypes` for the excluded subtypes — only `registry.card_data(obj.card_id).subtypes` is checked. Tokens are cre
- **woodland_cemetery**: Subtype detection in `controller_has_matching_land` only checks `obj.subtypes` (object-level field), which is always empty for non-token regular cards

### auto-selects instead of player choice (15 issues, 14 cards)

- **caravan_vigil**: Auto-selects first basic land in library order instead of presenting a player search choice (`mtg-engine/src/cards/isd/caravan_vigil.rs` lines 39–50)
- **corpse_lunge**: Engine auto-selects the exiled creature without presenting a player choice
- **corpse_lunge**: Test `corpse_lunge_picks_highest_power_creature` (tier8_cards.rs:538) enshrines the wrong auto-selection behavior
- **demonmail_hauberk**: The Scryfall ruling states: "You can sacrifice the creature Demonmail Hauberk is equipping in order to equip it to another creature." This explicitly 
- **elder_cathar**: Transformed DFC incorrectly treated as Human in both the single-target auto-select path and the multi-target engine path
- **falkenrath_noble**: **Issue 1 — "target player" is auto-targeted instead of chosen** (`mtg-engine/src/cards/isd/falkenrath_noble.rs`, lines 59–68)
- **grimgrin_corpse_born**: Auto-sacrifice for the activated ability doesn't present a player choice when multiple sacrifice targets are available.
- **makeshift_mauler**: Engine auto-selects which creature to exile instead of giving the player a choice (`mtg-engine/src/engine.rs` ~line 1574)
- **nevermore**: **Auto-selection of card name instead of player choice** (`mtg-engine/src/cards/isd/nevermore.rs:41-53`)
- **skaab_goliath**: Engine auto-selects which creatures to exile rather than giving the player the choice (`mtg-engine/src/engine.rs:1574–1600`)
- **skaab_ruinator**: Engine auto-selects which creature cards to exile as the additional cost, rather than presenting the player with a choice (`mtg-engine/src/engine.rs` 
- **skirsdag_cultist**: **Engine auto-selects which creature to sacrifice instead of presenting a player choice** — `mtg-engine/src/engine.rs` lines 1750–1759
- **skirsdag_high_priest**: Auto-selection of which two creatures to tap — `mtg-engine/src/cards/isd/skirsdag_high_priest.rs` lines 68–73
- **stitched_drake**: Engine auto-selects which creature to exile; player cannot choose
- **stitchers_apprentice**: Engine bug: `trigger_event_index` desync causes `CreatureDied` events from the sacrifice to be skipped when ETB-watch permanents (Champion of the Pari

### log message (13 issues, 11 cards)

- **burning_vengeance**: **Log message logged before target is chosen, and describes "opponent" inaccurately** — `mtg-engine/src/cards/isd/burning_vengeance.rs` lines 67–69
- **full_moons_rise**: Activated ability description in `mtg-engine/src/cards/isd/full_moons_rise.rs` line 57 says "Wolf and Werewolf" when oracle says "Werewolf" only. This
- **gatstaf_shepherd**: **Log message names the wrong source card when transforming back to front face**
- **gatstaf_shepherd**: The format string hardcodes `"Gatstaf Shepherd"` as the subject of transformation. When the card is on its back face (Gatstaf Howler) and transforms b
- **gatstaf_shepherd**: Oracle text does not specify log message content, but this is an implementation inaccuracy that misrepresents the game event to observers.
- **grizzled_outcasts**: Log message is incorrect when transforming back to front face
- **instigator_gang**: **Log message names wrong source when transforming back** (`instigator_gang.rs:119–121`): When Wildblood Pack transforms back to Instigator Gang (`was
- **kruin_outlaw**: **Log message says "Kruin Outlaw transforms into Kruin Outlaw" when back face transforms back to front face** (`mtg-engine/src/cards/isd/kruin_outlaw.
- **mayor_of_avabruck**: **Log message hardcodes wrong source name when transforming back** — `mtg-engine/src/cards/isd/mayor_of_avabruck.rs:119`
- **tormented_pariah**: **Log message incorrectly names source when Rampaging Werewolf transforms back** (`mtg-engine/src/cards/isd/tormented_pariah.rs:87–88`)
- **ulvenwald_mystics**: Log message is incorrect when transforming from back face (Ulvenwald Primordials) to front face (Ulvenwald Mystics)
- **village_ironsmith**: Incorrect log message when Ironfang transforms back to Village Ironsmith (`mtg-engine/src/cards/isd/village_ironsmith.rs:87–88`)
- **villagers_of_estwald**: **Log message is wrong when Howlpack transforms back to Villagers** (`mtg-engine/src/cards/isd/villagers_of_estwald.rs`, line 88)

### engine: trigger dispatch/zone (11 issues, 11 cards)

- **abattoir_ghoul**: **Engine bug: DeathWatch trigger cancelled at resolution if Ghoul left battlefield** (`mtg-engine/src/triggers.rs` lines 906–912)
- **armored_skaab**: ETB trigger suppressed when source leaves battlefield before resolution (`mtg-engine/src/triggers.rs`, lines 893–899)
- **crossway_vampire**: ETB trigger is suppressed if Crossway Vampire leaves the battlefield before the trigger resolves.
- **dearly_departed**: **Engine: `AnyCreatureEnters` watcher scan only checks `Zone::Battlefield`; Dearly Departed in the graveyard is never dispatched a trigger** (`mtg-eng
- **geistcatchers_rig**: **ETB trigger silently suppressed if source leaves battlefield before resolution** (`mtg-engine/src/triggers.rs` lines 893–899)
- **ghoulraiser**: ETB trigger silently skipped if Ghoulraiser leaves the battlefield before the trigger resolves — engine bug in `mtg-engine/src/triggers.rs` lines 893–
- **hollowhenge_scavenger**: ETB trigger resolution silently skipped when source leaves battlefield before trigger resolves (`mtg-engine/src/triggers.rs:893-899`)
- **stitchers_apprentice**: When the player then submits `ResolveChoice` for the sacrifice, `submit_action` clones the state (copying `trigger_event_index = 1`) and calls `new_st
- **sturmgeist**: Draw skipped when Sturmgeist leaves battlefield before trigger resolves (`mtg-engine/src/cards/isd/sturmgeist.rs:46-49`)
- **village_bell_ringer**: ETB trigger resolution skipped if VBR leaves the battlefield before the trigger resolves (`mtg-engine/src/triggers.rs`, line 894–899)
- **witchbane_orb**: **ETB trigger suppressed when Witchbane Orb leaves the battlefield before trigger resolves** — `mtg-engine/src/triggers.rs` lines 893–898

### engine: simultaneous events (9 issues, 8 cards)

- **abattoir_ghoul**: **Engine bug: DeathWatch trigger never collected when Ghoul and victim die simultaneously** (`mtg-engine/src/triggers.rs` lines 418–419)
- **falkenrath_noble**: **Issue 2 — simultaneous death triggers only once instead of twice** (`mtg-engine/src/triggers.rs`, lines 418–421)
- **moldgraf_monstrosity**: **`on_dies` unconditionally exiles regardless of current zone, violating ruling 2 about simultaneous Monstrosity deaths**
- **murder_of_crows**: Simultaneous death: Murder of Crows' triggered ability does not fire when it dies at the same time as another creature — `mtg-engine/src/triggers.rs:4
- **rage_thrower**: **Issue 1 — Simultaneous death: Rage Thrower's trigger does not fire for a creature dying at the same time** (`mtg-engine/src/triggers.rs`, lines 418–
- **selhoff_occultist**: Selhoff Occultist's `AnyCreatureDies` trigger does not fire for creatures that die simultaneously with it (e.g., in a board wipe).
- **unruly_mob**: Simultaneous death: trigger does not fire when Unruly Mob dies in the same SBA pass as another creature you control.
- **unruly_mob**: Official ruling says: `If Unruly Mob and another creature you control die simultaneously (perhaps because they were both attacking or blocking), Unrul
- **village_cannibals**: **Simultaneous deaths: Village Cannibals doesn't trigger when it dies alongside a Human** (`mtg-engine/src/triggers.rs` lines 417–441, `mtg-engine/src

### oracle text mismatch (5 issues, 4 cards)

- **bonds_of_faith**: **`oracle_text` field is missing the "Enchant creature" first line** — `mtg-engine/src/cards/isd/bonds_of_faith.rs` line 25
- **bump_in_the_night**: `oracle_text` field in `card_data()` is incomplete (`mtg-engine/src/cards/isd/bump_in_the_night.rs` line 23)
- **disciple_of_griselbrand**: `oracle_text` field wording does not match Scryfall oracle text (`mtg-engine/src/cards/isd/disciple_of_griselbrand.rs` line 25)
- **furor_of_the_bitten**: Missing "Enchant creature\n" prefix in `oracle_text` field — `mtg-engine/src/cards/isd/furor_of_the_bitten.rs:22`
- **furor_of_the_bitten**: All other auras in the same set that have been verified (dead_weight, curiosity, sensory_deprivation, wreath_of_geists, claustrophobia) include the "E

### engine: summoning sickness (4 issues, 3 cards)

- **avacynian_priest**: Engine does not enforce summoning sickness for `{T}` activated abilities; a freshly entered Avacynian Priest can activate its tap ability on the same 
- **furor_of_the_bitten**: Forced-attack enforcement ignores Haste overriding summoning sickness — `mtg-engine/src/engine.rs:1827`
- **furor_of_the_bitten**: By contrast, `eligible_attackers` in `combat.rs:577` correctly uses `(!o.summoning_sick || state.has_keyword(o.id, Keyword::Haste, registry))`. A crea
- **mikaeus_the_lunarch**: Summoning sickness not enforced for {T} activated abilities

### "as long as" snapshot (4 issues, 4 cards)

- **bonds_of_faith**: **"as long as it's a Human" condition is snapshotted at ETB, never re-evaluated** — `mtg-engine/src/cards/isd/bonds_of_faith.rs` lines 39–69
- **butchers_cleaver**: **Snapshot "as long as" — lifelink not re-evaluated when equipped creature transforms** (`mtg-engine/src/cards/isd/butchers_cleaver.rs`, lines 14–34 a
- **sharpened_pitchfork**: "As long as" condition is evaluated once at equip time, not continuously re-evaluated.
- **silver_inlaid_dagger**: "As long as" Human condition is evaluated once at equip time and never re-evaluated

### test enshrines wrong behavior (4 issues, 3 cards)

- **delver_of_secrets**: The test `delver_does_not_transform_when_top_card_is_creature` (line 1027) actively enshrines this wrong behavior: `assert!(state.awaiting_action.is_n
- **mentor_of_the_meek**: **Test enshrines wrong auto-pay behavior** (`mtg-engine/tests/tier15_cards.rs`, lines 71–99)
- **thraben_sentry**: **Test enshrines wrong auto-transform behavior** (`mtg-engine/tests/tier15_cards.rs`, lines 1392–1409, test `thraben_sentry_transforms_when_creature_d
- **thraben_sentry**: The test calls `on_any_creature_dies` directly and asserts `is_transformed == true` without checking for `state.awaiting_action`. If the bug were fixe

### engine: force-attack missing checks (3 issues, 3 cards)

- **curse_of_the_nightly_hunt**: The post-DeclareAttackers forced-attack enforcement loop does not check `state.can_attack()`, causing creatures under "can't attack" effects (e.g., Pa
- **furor_of_the_bitten**: Forced-attack enforcement skips `can_attack()` check — `mtg-engine/src/engine.rs:1838–1846`
- **galvanic_juggernaut**: Forced attack logic ignores `can_attack()`, violating the "if able" clause

### engine: planeswalker damage (3 issues, 3 cards)

- **curse_of_the_pierced_heart**: Dealing damage to a planeswalker via the choice path does not remove loyalty counters
- **geistflame**: `resolve_damage` helper does not remove loyalty counters when dealing damage to a planeswalker (`mtg-engine/src/cards/helpers.rs` lines 52–62)
- **rage_thrower**: **Issue 3 — Damage targeting a planeswalker does not reduce loyalty counters** (`mtg-engine/src/engine.rs`, lines 2179–2191; `mtg-engine/src/cards/isd

### LLM knowledge (2 issues, 2 cards)

- **bump_in_the_night**: LLM player card knowledge is missing flashback ability (`mtg-player/src/llm.rs` line 84)
- **travel_preparations**: LLM card knowledge in `mtg-player/src/llm.rs` line 111 describes one target instead of up to two

### engine: protection targeting (2 issues, 2 cards)

- **elite_inquisitor**: Protection's targeting restriction is not enforced in the engine's ability-targeting path (`mtg-engine/src/engine.rs:758-768` and `mtg-engine/src/engi
- **spare_from_evil**: Protection's "T" (targeting) aspect not enforced by engine — non-Human creature activated abilities can still target protected creatures

### "may" not optional (1 issues, 1 cards)

- **ghost_quarter**: **"May" search is forced — controller gets no choice** (`mtg-engine/src/cards/isd/ghost_quarter.rs`, line 81–100)

### missing shuffle (1 issues, 1 cards)

- **ghost_quarter**: **Missing shuffle after library search** (`mtg-engine/src/cards/isd/ghost_quarter.rs`, lines 92–101)

## FALSE POSITIVE


## NEEDS REVIEW (175 issues)

### angelic_overseer
- Sequential SBA processing allows a protecting Human to die before Angelic Overseer's indestructibility is evaluated, causing Angelic Overseer to be incorrectly destroyed.

### back_from_the_brink
- Token copies are created with no colors (`Vec::new()`) in `state.rs`

### balefire_dragon
- Battlefield guard in `on_combat_damage_to_player` suppresses the triggered effect if Balefire Dragon has left the battlefield at resolution time (`mtg-engine/src/cards/isd/balefire_dragon.rs`, lines 4

### bitterheart_witch
- `present_player_choice` builds target list without hexproof filtering (`mtg-engine/src/cards/isd/bitterheart_witch.rs:14-16`) and the corresponding `ChooseCurseThenAttach` path in the engine has the s

### bloodgift_demon
- Upkeep trigger incorrectly fizzles if Bloodgift Demon leaves the battlefield after its trigger is on the stack but before it resolves.

### boneyard_wurm
- Graveyard zone display shows base P/T (0/0) instead of dynamically computed P/T
- Ruling says: `The ability that defines Boneyard Wurm's power and toughness works in all zones, not just the battlefield. If Boneyard Wurm is in your graveyard, it will count itself.`
- Affected file: `mtg-engine/src/view.rs`, function `card_view` (line ~213), specifically `power: obj.power` at line 221.

### brain_weevil
- Incomplete discard when target player has 3+ cards in hand — `mtg-engine/src/cards/isd/brain_weevil.rs:64-75` + `mtg-engine/src/engine.rs:2009-2023`

### burning_vengeance
- **Engine bug: `SpellCast` trigger dispatch restricted to instant/sorcery** — `mtg-engine/src/triggers.rs` lines 644–675
- **Card bug: checks `cast_with_flashback` rather than "cast from graveyard"** — `mtg-engine/src/cards/isd/burning_vengeance.rs` lines 48–53

### butchers_cleaver
- **Human check ignores transformed state — back-face DFCs incorrectly identified as Human at equip time** (`mtg-engine/src/cards/isd/butchers_cleaver.rs`, lines 15–18)
- **Human check ignores runtime object subtypes — Human tokens never get lifelink** (`mtg-engine/src/cards/isd/butchers_cleaver.rs`, lines 15–18)

### cackling_counterpart
- **Colors never copied when creating token copy** — `mtg-engine/src/state.rs`, `create_token_copy` function (line 426)
- **Copying a token source loses its card_types, keywords, and subtypes** — `mtg-engine/src/state.rs`, `create_token_copy` function (lines 424–431)

### charmbreaker_devils
- Upkeep trigger fires spuriously on the stack during the opponent's upkeep
- SpellCast trigger fires spuriously on the stack when the opponent casts an instant or sorcery

### civilized_scholar
- **Stale `attacked_this_turn` flag causes Homicidal Brute to skip transform-back** (`mtg-engine/src/cards/isd/civilized_scholar.rs`, lines 166–191)
- **EndStep trigger registered on front face, causing spurious stack entry when not transformed** (`mtg-engine/src/cards/isd/civilized_scholar.rs`, lines 38–48)

### cloistered_youth
- Spurious upkeep trigger fires for Unholy Fiend (transformed state)

### creepy_doll
- **Lethal-damage + regeneration scenario: winning the coin flip fails to destroy the creature** (`mtg-engine/src/engine.rs` lines 3118–3125, interacting with `mtg-engine/src/triggers.rs` lines 926–931)
- Second ruling says: `"If the combat damage Creepy Doll deals to a creature is lethal, you'll still flip a coin. If the creature is still on the battlefield (perhaps because it regenerated), it could b

### curse_of_oblivion
- Upkeep trigger fires during every player's upkeep, not only the enchanted player's upkeep — spurious triggers placed on stack during the non-cursed player's turn.

### curse_of_the_nightly_hunt
- Ruling says: `"If, during the enchanted player's declare attackers step, a creature they control is tapped, is affected by a spell or ability that says it can't attack, or hasn't been under that playe

### curse_of_the_pierced_heart
- The upkeep trigger goes on the stack during every player's upkeep, not only the enchanted player's upkeep

### darkthicket_wolf
- `abilities_activated_this_turn` is never cleared between turns — engine bug causes once-per-turn restriction to become once-per-game permanently

### daybreak_ranger
- Engine never increments `spells_cast_this_turn` or populates `spells_cast_last_turn`, breaking both transform conditions in actual gameplay.

### dearly_departed
- **Engine: `EnterWatch` trigger resolution also requires watcher to be on `Zone::Battlefield`** (`mtg-engine/src/triggers.rs:914-915`)

### delver_of_secrets
- **"You may reveal" choice suppressed when top card is not an instant or sorcery** (`mtg-engine/src/cards/isd/delver_of_secrets.rs` lines 104–118)
- Ruling says: `"You may reveal the card even if it's not an instant or sorcery."`
- The choice to reveal is gated on the top card being an instant or sorcery. The oracle text presents "you may reveal" as an unconditional option; only the *transform* consequence is conditional. When t
- **Transformed Insectile Aberration gets a spurious upkeep trigger on the stack** (engine bug in `mtg-engine/src/triggers.rs` lines 311–327, affecting `delver_of_secrets.rs`)
- Oracle text for Insectile Aberration says: `"Flying"` — no upkeep trigger.

### demonmail_hauberk
- Player cannot choose which creature to sacrifice for the Equip cost

### disciple_of_griselbrand
- Player cannot choose which creature to sacrifice when multiple are available (`mtg-engine/src/engine.rs` lines 1750–1759)

### elite_inquisitor
- Per MTG rule 702.16c, protection means (among other things) the protected permanent "can't be the target of spells or abilities from sources" with the stated quality.
- Concrete cases broken by this:

### essence_of_the_wild
- **ETB abilities of creatures entering as EotW copies still fire** — `mtg-engine/src/triggers.rs:344-392` and `mtg-engine/src/state.rs:524-575`

### evil_twin
- **Activated ability inaccessible after copying (engine.rs + evil_twin.rs)**
- **ETB abilities of the copied creature never trigger (triggers.rs)**
- **`is_evil_twin` marker set before the optional copy choice is made (evil_twin.rs:53–55)**

### falkenrath_noble
- Oracle ruling says: `"If Falkenrath Noble and another creature die at the same time, Falkenrath Noble's triggered ability will trigger for each of them."`
- **Issue 3 — DeathWatch trigger incorrectly fizzles if Noble leaves the battlefield between trigger and resolution** (`mtg-engine/src/triggers.rs`, lines 906–912, and `mtg-engine/src/cards/isd/falkenra

### fiend_hunter
- ETB trigger silently dropped when Fiend Hunter has left the battlefield before resolution
- Official ruling says: `"If Fiend Hunter leaves the battlefield before its first ability has resolved, its second ability will trigger and do nothing. Then its first ability will resolve and exile the 

### furor_of_the_bitten
- If a creature has both `ForceAttack` (Furor) and a `PreventAttack` effect (Pacifism on the same creature, or Bonds of Faith on a non-Human), the `eligible_attackers` function correctly excludes it (th

### galvanic_juggernaut
- File: `mtg-engine/src/engine.rs` lines 1822–1847 (DeclareAttackers forced-attacker loop)

### garruk_relentless
- **`abilities_activated_this_turn` never cleared between turns** — `mtg-engine/src/engine.rs:1942`, `mtg-engine/src/engine.rs:3006-3061`
- **`is_legendary` not set in `on_resolve`** — `mtg-engine/src/cards/isd/garruk_relentless.rs:313-321`

### gatstaf_shepherd
- **Engine never increments `spells_cast_this_turn`; `spells_cast_last_turn` is always empty, making both transform conditions permanently wrong**
- `mtg-engine/src/engine.rs` lines 1479–1666: The `Action::CastSpell` handler fires `GameEvent::SpellCast` but never writes to `state.spells_cast_this_turn`. Searched entire codebase: `spells_cast_this_
- `mtg-engine/src/engine.rs` lines 2880–2903 (turn transition in `advance_step`) and lines 3006–3061 (`Step::Cleanup` in `perform_turn_based_actions`): Neither location copies `spells_cast_this_turn` to
- **Front face consequence**: `werewolf_should_transform` checks `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum()` (line 12 of `gatstaf_shepherd.rs`), then `total_spells_las
- **Back face consequence**: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` (line 16 of `gatstaf_shepherd.rs`) on an empty map always returns `false`. Gatstaf Howler never transforms ba
- `mtg-engine/src/cards/isd/gatstaf_shepherd.rs` lines 87–89:

### geist_of_saint_traft
- Extra tokens created by Parallel Lives doubling are not tapped, not attacking, and not added to `end_of_combat_exiles` — `mtg-engine/src/cards/isd/geist_of_saint_traft.rs` lines 57–81

### geistcatchers_rig
- **Target selection deferred to resolution instead of stack-placement** (`mtg-engine/src/cards/isd/geistcatchers_rig.rs` lines 40–59, `mtg-engine/src/triggers.rs` lines 344–364)
- Scryfall ruling says: `The target creature with flying is chosen when the ability triggers and goes on the stack. You choose whether or not Geistcatcher's Rig will deal 4 damage to it when the ability
- **`optional: true` conflates target selection with the "you may" decision** (`mtg-engine/src/cards/isd/geistcatchers_rig.rs` line 56)

### geistflame
- Engine's `AnyTarget` implementation excludes planeswalkers as valid targets (`mtg-engine/src/engine.rs` lines 836–864, 1074–1090, 1343–1358; `mtg-engine/src/cards/mod.rs` line 244–245)

### ghoulcallers_chant
- `build_cast_target_spec` returns `CastTargetSpec::SingleTarget` for modal spells containing a `TwoTargets` inner mode, blocking interactive players from choosing mode 2

### grave_bramble
- Protection from Zombies incorrectly prevents Grave Bramble from blocking Zombie attackers (`mtg-engine/src/combat.rs:699`)
- Grimgrin, Corpse-Born's triggered ability can target Grave Bramble despite protection from Zombies (`mtg-engine/src/cards/isd/grimgrin_corpse_born.rs:99–103`)

### grimoire_of_the_dead
- Legend rule not applied to legendary creature cards returned by ability 2 if they were never previously on the battlefield (`mtg-engine/src/cards/isd/grimoire_of_the_dead.rs:151-163`, `mtg-engine/src/

### grizzled_outcasts
- Engine never updates `spells_cast_this_turn` or `spells_cast_last_turn`; both transform conditions are always wrong
- File: `mtg-engine/src/engine.rs` lines 1657–1665 (spell cast) and 2882–2895 (turn transition)
- Consequence for front face: `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum()` (grizzled_outcasts.rs line 12) always equals 0, so `total_spells_last_turn == 0 && !state.is_
- Consequence for back face: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` (line 16) is always false — Krallenhorde Wantons **never transforms back** even when two or more spells were 
- File: `mtg-engine/src/cards/isd/grizzled_outcasts.rs` line 87–88
- The log should read `"Krallenhorde Wantons transforms into Grizzled Outcasts"`. The source name is hardcoded as `"Grizzled Outcasts"` regardless of which face was active before the flip.

### gruesome_deformity
- Artifact creature tokens cannot block creatures with intimidate, violating the oracle rule "can't be blocked except by artifact creatures and/or creatures that share a color with it."

### gutter_grime
- Token `is_token` check fails in real gameplay: Gutter Grime incorrectly triggers when a token you control dies

### hamlet_captain
- Trigger does not resolve if Hamlet Captain leaves the battlefield before the trigger resolves.

### hanweir_watchkeep
- Engine never updates `spells_cast_this_turn` or `spells_cast_last_turn`, causing both werewolf transform conditions to evaluate incorrectly in actual gameplay.
- Front face: `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum()` always equals 0; condition `total_spells_last_turn == 0 && !state.is_first_turn` is always `true` after turn 
- Back face: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` is always `false` (HashMap is empty) → Bane of Hanweir never transforms back, regardless of how many spells were cast.

### harvest_pyre
- Player cannot choose which specific cards to exile; engine arbitrarily picks cards

### heretics_punishment
- `AnyTarget` engine implementation excludes planeswalkers as valid targets

### hinterland_harbor
- File: `mtg-engine/src/cards/isd/hinterland_harbor.rs`, lines 17–23
- `state.rs` line 1205: `/// Subtypes on this object (for tokens — regular cards use CardData.subtypes via registry).`
- `engine.rs` `setup_game` (lines 2670–2682): copies `colors`, `name`, `keywords`, `card_types` to the object but never copies `card_data.subtypes` to `obj.subtypes`.
- `forest.rs` line 15: `subtypes: vec!["Forest".into()]` — this lives in `CardData`, not the `GameObject`.
- By contrast, `check_condition` in `state.rs` (lines 1084–1093) correctly checks both `o.subtypes` AND `registry.card_data(o.card_id).subtypes` when testing for subtype control.
- The existing test `clifftop_retreat_enters_untapped_with_mountain` (same pattern) masks this bug by manually setting `state.get_object_mut(mtn).unwrap().subtypes = vec!["Mountain".into()]` — which is 

### inquisitors_flail
- Fight damage incorrectly doubled by Flail: `mtg-engine/src/combat.rs` lines 452–454

### into_the_maw_of_hell
- `is_valid_target` accepts creatures for the land target slot, allowing the card to be cast with no legal land target
- Card file: `mtg-engine/src/cards/isd/into_the_maw_of_hell.rs` lines 40–56 (`is_valid_target`)
- Engine file: `mtg-engine/src/engine.rs` lines 1067–1072 (`valid_targets_for_req` for `PermanentWithFilter`)

### kessig_cagebreakers
- **Attack trigger silently discarded if Kessig is destroyed before resolution** (`mtg-engine/src/triggers.rs:980-985` and `mtg-engine/src/cards/isd/kessig_cagebreakers.rs:39-42`)
- **Parallel Lives doubled tokens are not set as tapped and attacking** (`mtg-engine/src/cards/isd/kessig_cagebreakers.rs:61-76` and `mtg-engine/src/state.rs:314-348`)

### kruin_outlaw
- **Engine never increments `spells_cast_this_turn` when a spell is cast, and never saves it to `spells_cast_last_turn` at turn end** (`mtg-engine/src/engine.rs` CastSpell handler ~line 1657; `advance_s

### liliana_of_the_veil
- **+1 ability: Player 1's discard resolves before Player 2 even makes their choice, violating the "all at the same time" ruling.**
- File: `mtg-engine/src/cards/isd/liliana_of_the_veil.rs` lines 136–144 (auto-discard path) and `mtg-engine/src/engine.rs` lines 2012–2022 (choice path)
- Oracle text ruling says: `"first the player whose turn it is chooses a card in hand without revealing it, then each other player in turn order does the same. Then all the chosen cards are discarded at

### ludevics_test_subject
- **`card_state` (hatchling counters) not reset on zone change** — `mtg-engine/src/state.rs`, `move_object()` lines 479–487
- **`is_transformed` not reset on zone change** — `mtg-engine/src/state.rs`, `move_object()` lines 479–487

### mask_of_avacyn
- **Duplicate equip action generated via attached-aura loop enables broken re-equip** (`mtg-engine/src/engine.rs:331-338`, `mtg-engine/src/cards/isd/mask_of_avacyn.rs:59-65`)

### mayor_of_avabruck
- **Engine never populates `spells_cast_last_turn`** — both transform conditions are permanently broken in actual gameplay.
- `mtg-engine/src/state.rs:127,131`: `spells_cast_this_turn` and `spells_cast_last_turn` are defined as `HashMap<PlayerId, u32>`, initialized empty, and **never written to** anywhere in the engine sourc
- `mtg-engine/src/engine.rs` `CastSpell` handler (lines 1479–1666): handles spell casting in full but contains no increment of `spells_cast_this_turn`.
- `mtg-engine/src/engine.rs` `advance_step` (lines 2867–2904): handles end-of-turn transition (sets new active player, increments turn_number, clears `creature_died_this_turn`) but never transfers `spel
- Consequence for front face: `total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum()` is always 0; condition `total_spells_last_turn == 0 && !state.is_first_turn` is always true after
- Consequence for back face: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` is always false. Howlpack Alpha **never** transforms back regardless of how many spells were cast.
- All tests bypass this by directly inserting into `spells_cast_last_turn` (e.g., `state.spells_cast_last_turn.insert(P0, 2)`), so unit tests pass even though the engine never populates the field.

### mentor_of_the_meek
- **"You may pay {1}" is auto-paid instead of presenting a player choice** (`mtg-engine/src/cards/isd/mentor_of_the_meek.rs`, lines 55–80)
- **Power checked at resolution time, not at ETB trigger time** (`mtg-engine/src/cards/isd/mentor_of_the_meek.rs`, line 51; `mtg-engine/src/triggers.rs`, lines 366–392)

### mirror_mad_phantasm
- **Reveal loop uses `draw_top_card()` which sets `has_drawn_from_empty = true` on library exhaustion, causing the player to incorrectly lose the game**
- File: `mtg-engine/src/cards/isd/mirror_mad_phantasm.rs`, lines 83–97 (loop), exercising `mtg-engine/src/state.rs` line 1315
- **Token copy of Mirror-Mad Phantasm activating the ability is incorrectly found in the reveal loop and enters the battlefield**
- File: `mtg-engine/src/cards/isd/mirror_mad_phantasm.rs`, lines 69–115; SBA check in `mtg-engine/src/sba.rs` line 307–314 runs only after the ability resolves
- Oracle text ruling says: `"If no card named Mirror-Mad Phantasm is revealed (possibly because it was a card copying Mirror-Mad Phantasm or it was a token), all cards from that library will be put into

### moldgraf_monstrosity
- `mtg-engine/src/cards/isd/moldgraf_monstrosity.rs` lines 48-51

### moonmist
- Second Moonmist fails to transform Werewolf DFCs after they naturally untransform back to their Human front face

### moorland_haunt
- Player cannot choose which creature card to exile when multiple are in the graveyard (`mtg-engine/src/cards/isd/moorland_haunt.rs` lines 85–96 and `mtg-engine/src/engine.rs` lines 399–406)

### naturalize
- File: `mtg-engine/src/cards/isd/naturalize.rs`, lines 40–42

### night_terrors
- **Night Terrors is never moved off the stack when the target player has multiple nonland cards in hand** (`mtg-engine/src/cards/isd/night_terrors.rs:63-70`, `mtg-engine/src/engine.rs:2003-2008`)
- **Wrong `PendingEffect` variant used for Night Terrors** (`mtg-engine/src/cards/isd/night_terrors.rs:66`)

### olivia_voldaren
- `mtg-engine/src/engine.rs` lines 1266–1268 (and duplicated at line 1397 in `matches_target_filter`)
- In-card guard check for ability 1 also only checks `obj.subtypes`, missing real Vampire cards
- `mtg-engine/src/cards/isd/olivia_voldaren.rs` lines 129–131

### past_in_flames
- `until_end_of_turn_flashback` is never cleared at end-of-turn cleanup, so flashback grants persist indefinitely across turns.
- Cards with no mana cost that are instants or sorceries receive `ManaCost::free()` as their flashback cost, making them castable for {0} when the oracle text and ruling say they cannot be cast via flas

### pitchburn_devils
- `any_targets` helper omits planeswalkers; Pitchburn Devils' trigger cannot target them

### prey_upon
- **fight() emits CombatDamageDealt instead of NonCombatDamageDealt, applying combat-specific effects to fight damage** (`mtg-engine/src/combat.rs:467`, `mtg-engine/src/combat.rs:429–436`, `mtg-engine/s
- Additionally, because `CombatDamageDealt` (not `NonCombatDamageDealt`) is emitted, the trigger system (`triggers.rs:459–487`) fires `DealsCombatDamageToCreature` triggers during fight. Concretely: if 

### rage_thrower
- **Issue 2 — DeathWatch trigger incorrectly fizzles if Rage Thrower leaves the battlefield after triggering but before resolution** (`mtg-engine/src/triggers.rs`, lines 906–912; `mtg-engine/src/cards/i
- **Issue 4 — Trigger description and target-choice prompt omit "or planeswalker"** (`mtg-engine/src/cards/isd/rage_thrower.rs`, lines 33 and 57)

### reckless_waif
- **Engine never populates `spells_cast_this_turn` or `spells_cast_last_turn`; both conditions are permanently broken**
- Card does (front face check, `mtg-engine/src/cards/isd/reckless_waif.rs:12-14`): `let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum(); if !is_transformed { total_spells_last_t
- Card does (back face check, `mtg-engine/src/cards/isd/reckless_waif.rs:16`): `state.spells_cast_last_turn.values().any(|&count| count >= 2)`
- Engine does (`mtg-engine/src/engine.rs` CastSpell handler, lines 1479–1666): never increments `spells_cast_this_turn` when a spell is cast — zero matches for `spells_cast_this_turn` in the entire file
- Engine does (`mtg-engine/src/engine.rs` `advance_step`, lines 2882–2895): never transfers `spells_cast_this_turn` into `spells_cast_last_turn` at turn end — the only fields cleared are `is_first_turn`
- Net effect: `spells_cast_last_turn` is always an empty `HashMap`. Therefore `total_spells_last_turn` is always 0 → front face condition is permanently true after turn 1 (waif always transforms on ever

### rooftop_storm
- Rooftop Storm alternative cost not offered for Zombie creature spells cast from the graveyard

### sever_the_bloodline
- A creature is an illegal target for an opponent's spell if it has hexproof (CR 608.2b). If the targeted creature gains hexproof in response to Sever the Bloodline (e.g., via Ranger's Guile, which is i

### skirsdag_high_priest
- "Tap two untapped creatures you control" is a cost the player pays, meaning the player must choose which two untapped creatures to tap. When the controller has more than two untapped creatures (beside

### snapcaster_mage
- **`until_end_of_turn_flashback` is never cleared at end of turn** (`mtg-engine/src/engine.rs:3006–3061`)
- **Snapcaster Mage incorrectly excludes cards with innate flashback from eligible targets** (`mtg-engine/src/cards/isd/snapcaster_mage.rs:48–53`)

### spare_from_evil
- Protection's "D" (damage) aspect not enforced for non-combat damage from non-Human creature sources

### splinterfright
- Upkeep trigger does not resolve if Splinterfright has left the battlefield between trigger collection and resolution

### stitchers_apprentice
- Mechanism: After `ActivateAbility` resolves, `process_triggers` is called. `collect_triggers` processes the `EnteredBattlefield` event (index 0), sets `state.trigger_event_index = 1` (`triggers.rs:873

### sulfur_falls
- Contrast with the correct dual-check pattern used in `state.rs` `check_condition` (lines 1085–1092): `o.subtypes.iter().any(|s| s == subtype) || registry.card_data(o.card_id).map(|d| d.subtypes.iter()

### thraben_sentry
- **"you may" is bypassed — card always auto-transforms, player never gets a choice** (`mtg-engine/src/cards/isd/thraben_sentry.rs`, lines 72–76)
- The engine has a `YesNo` resolution-choice mechanism (`AwaitingAction::ResolutionChoice { choice: ResolutionChoiceKind::YesNo { ... } }`) used by other "you may" DFC cards (e.g., Screeching Bat, Clois
- **Vigilance incorrectly retained on back face after transform** (`mtg-engine/src/cards/isd/thraben_sentry.rs`, lines 73–76)
- `has_keyword()` in `state.rs` checks `obj.keywords` **first** (step 0, line 1000): `if obj.keywords.contains(&keyword) { return true; }`. Because `obj.keywords` still holds `[Vigilance]`, `has_keyword

### tormented_pariah
- **Engine never tracks spells cast per turn; `spells_cast_last_turn` is always empty in real gameplay** (`mtg-engine/src/engine.rs`, `mtg-engine/src/state.rs`)

### travelers_amulet
- **No player choice when multiple basic lands exist** (`mtg-engine/src/cards/isd/travelers_amulet.rs:57`)

### tribute_to_hunger
- Missing `is_valid_target` override to enforce "target opponent" restriction

### ulvenwald_mystics
- Engine never increments `spells_cast_this_turn` and never transfers it to `spells_cast_last_turn`; both transform conditions are permanently wrong in real gameplay
- File: `mtg-engine/src/engine.rs` CastSpell handler (lines 1479–1666): no increment of `spells_cast_this_turn` when a spell is cast; `advance_step` turn-end transition (lines 2867–2895): no rollover of
- File: `mtg-engine/src/cards/isd/ulvenwald_mystics.rs` line 117

### unbreathing_horde
- "Enters with counters" replacement effect does not fire when Unbreathing Horde enters the battlefield via reanimation (e.g., Unburial Rites)

### unburial_rites
- **Missing `target_requirement()` override — spell treated as untargeted**
- File: `mtg-engine/src/cards/isd/unburial_rites.rs`, line 11–65
- **Target selected at resolution time, not at cast time — ignores `targets` parameter**
- File: `mtg-engine/src/cards/isd/unburial_rites.rs`, lines 31–64; specifically the `_targets` parameter at line 31
- **Spell can be cast with no legal targets**

### undead_alchemist
- **Second triggered ability only fires from Undead Alchemist's own mill, not from all sources** (`mtg-engine/src/cards/isd/undead_alchemist.rs:82-99`)
- **Multiple Undead Alchemists cause incorrect life restoration (net life gain) and double milling** (`mtg-engine/src/cards/isd/undead_alchemist.rs:63-99`)
- **First-strike Zombie dealing lethal combat damage causes player loss before Alchemist trigger fires** (`mtg-engine/src/combat.rs:146-153`, `mtg-engine/src/cards/isd/undead_alchemist.rs:45-105`)
- **Lifelink on the Zombie source incorrectly grants life when Undead Alchemist's replacement applies** (`mtg-engine/src/combat.rs:539-549`, `mtg-engine/src/cards/isd/undead_alchemist.rs:45-105`)

### village_cannibals
- **Spurious DeathWatch triggers on non-Human deaths** (`mtg-engine/src/triggers.rs` lines 422–441)

### village_ironsmith
- Engine never tracks spells cast per turn, so transform conditions are always wrong in a real game (`mtg-engine/src/engine.rs` — turn transition in `advance_step`, and `CastSpell` handler in `submit_ac
- Consequence: `total_spells_last_turn` is always 0, so Village Ironsmith (front face) always transforms on upkeep after turn 1, even when spells were cast. `spells_cast_last_turn.values().any(|&count| 
- Oracle text: Ironfang transforms into Village Ironsmith (the source of the transform is Ironfang)

### villagers_of_estwald
- **Engine never populates `spells_cast_last_turn` in real games** (`mtg-engine/src/engine.rs`, `mtg-engine/src/state.rs`)
- `total_spells_last_turn` (sum over empty map) is always 0 → front-face condition `total_spells_last_turn == 0 && !state.is_first_turn` is always true after turn 1 → Villagers always transforms at ever
- `state.spells_cast_last_turn.values().any(|&count| count >= 2)` over an empty map is always false → Howlpack never transforms back, even after 2+ spells were cast.
- Oracle text implies: a transform from Howlpack of Estwald back to Villagers of Estwald.

### witchbane_orb
- **Hexproof not re-validated at spell resolution for player targets** — `mtg-engine/src/stack.rs` line 39

### woodland_sleuth
- **Intervening-if condition not checked at trigger-collection time** (`mtg-engine/src/triggers.rs` lines 344–363)
- **Woodland Sleuth cannot be returned to its own hand when it dies in response to its ETB trigger** — two bugs, both must be fixed:
- Ruling says: `"Woodland Sleuth could die in response to its own morbid ability. If this happens, the ability could return Woodland Sleuth to its owner's hand."`
- Ruling says: `"Woodland Sleuth could die in response to its own morbid ability. If this happens, the ability could return Woodland Sleuth to its owner's hand."`

