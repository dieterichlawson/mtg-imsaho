# Issue-to-Test Mapping

204 issues: 139 classified, 65 unclassified

## bug_victim_of_night (token_subtype) (26 issues)

- back_from_the_brink: Token copies are created with no colors (`Vec::new()`) in `state.rs`
- bonds_of_faith: **Human subtype check at ETB only inspects registry data, missing object-level subtypes (tokens)** — `mtg-engine/src/cards/isd/bonds_of_faith.rs` line
- butchers_cleaver: **Human check ignores runtime object subtypes — Human tokens never get lifelink** (`mtg-engine/src/cards/isd/butchers_cleaver.rs`, lines 15–18)
- cackling_counterpart: **Colors never copied when creating token copy** — `mtg-engine/src/state.rs`, `create_token_copy` function (line 426)
- cackling_counterpart: **Copying a token source loses its card_types, keywords, and subtypes** — `mtg-engine/src/state.rs`, `create_token_copy` function (lines 424–431)
- geist_of_saint_traft: Extra tokens created by Parallel Lives doubling are not tapped, not attacking, and not added to `end_of_combat_exiles` — `mtg-engine/src/cards/isd/gei
- gruesome_deformity: Artifact creature tokens cannot block creatures with intimidate, violating the oracle rule "can't be blocked except by artifact creatures and/or creat
- gutter_grime: Token `is_token` check fails in real gameplay: Gutter Grime incorrectly triggers when a token you control dies
- isolated_chapel: Subtype check in `controller_has_matching_land` only reads `obj.subtypes` on game objects, missing subtypes stored in the registry for regular (non-to
- kessig_cagebreakers: **Parallel Lives doubled tokens are not set as tapped and attacking** (`mtg-engine/src/cards/isd/kessig_cagebreakers.rs:61-76` and `mtg-engine/src/sta
- maw_of_the_mire: `is_valid_target` only consults the registry for `CardType::Land`, missing land tokens whose types are stored on `obj.card_types` (not in registry)
- mirror_mad_phantasm: **Token copy of Mirror-Mad Phantasm activating the ability is incorrectly found in the reveal loop and enters the battlefield**
- naturalize: `is_valid_target` checks only registry data for card types, missing artifact/enchantment tokens
- olivia_voldaren: `TargetFilter::HasSubtype` in `matches_ability_target_filter` only checks `obj.subtypes`, not registry card data subtypes — ability 1 cannot target re
- olivia_voldaren: In-card guard check for ability 1 also only checks `obj.subtypes`, missing real Vampire cards
- parallel_lives: Extra Parallel Lives copies do not inherit post-creation token properties ("tapped", "tapped and attacking", dynamic P/T, combat assignment, delayed e
- paraselene: Enchantment detection only checks the registry, missing enchantment tokens (`mtg-engine/src/cards/isd/paraselene.rs` lines 36–40)
- sharpened_pitchfork: Human subtype check in `update_effects` only reads registry data, not runtime `obj.subtypes`, missing Human tokens.
- silver_inlaid_dagger: `update_effects()` does not check `o.subtypes` when detecting Human subtype, missing token Humans
- slayer_of_the_wicked: Subtype check only reads registry data, missing token subtypes (`slayer_of_the_wicked.rs` lines 41–43)
- sulfur_falls: `controller_has_matching_land` only checks `o.subtypes` (runtime object field) but never consults the registry, so it fails to detect basic Island and
- urgent_exorcism: `is_valid_target` only checks `registry.card_data(obj.card_id)` for subtypes/card_types, missing Spirit tokens
- vampiric_fury: Vampire subtype check in `on_resolve` only reads `registry.card_data(obj.card_id)` and never checks `obj.subtypes` — `mtg-engine/src/cards/isd/vampiri
- victim_of_night: `is_valid_target` does not check `obj.subtypes` for the excluded subtypes — only `registry.card_data(obj.card_id).subtypes` is checked. Tokens are cre
- village_cannibals: **Missing `o.subtypes` check for Human tokens** (`mtg-engine/src/cards/isd/village_cannibals.rs` lines 39–42)
- woodland_cemetery: Subtype detection in `controller_has_matching_land` only checks `obj.subtypes` (object-level field), which is always empty for non-token regular cards

## bug_etb_trigger_suppressed (15 issues)

- abattoir_ghoul: **Engine bug: DeathWatch trigger cancelled at resolution if Ghoul left battlefield** (`mtg-engine/src/triggers.rs` lines 906–912)
- armored_skaab: ETB trigger suppressed when source leaves battlefield before resolution (`mtg-engine/src/triggers.rs`, lines 893–899)
- bloodgift_demon: Upkeep trigger incorrectly fizzles if Bloodgift Demon leaves the battlefield after its trigger is on the stack but before it resolves.
- crossway_vampire: ETB trigger is suppressed if Crossway Vampire leaves the battlefield before the trigger resolves.
- falkenrath_noble: **Issue 3 — DeathWatch trigger incorrectly fizzles if Noble leaves the battlefield between trigger and resolution** (`mtg-engine/src/triggers.rs`, lin
- fiend_hunter: ETB trigger silently dropped when Fiend Hunter has left the battlefield before resolution
- geistcatchers_rig: **ETB trigger silently suppressed if source leaves battlefield before resolution** (`mtg-engine/src/triggers.rs` lines 893–899)
- ghoulraiser: ETB trigger silently skipped if Ghoulraiser leaves the battlefield before the trigger resolves — engine bug in `mtg-engine/src/triggers.rs` lines 893–
- hollowhenge_scavenger: ETB trigger resolution silently skipped when source leaves battlefield before trigger resolves (`mtg-engine/src/triggers.rs:893-899`)
- mentor_of_the_meek: **Power checked at resolution time, not at ETB trigger time** (`mtg-engine/src/cards/isd/mentor_of_the_meek.rs`, line 51; `mtg-engine/src/triggers.rs`
- rage_thrower: **Issue 2 — DeathWatch trigger incorrectly fizzles if Rage Thrower leaves the battlefield after triggering but before resolution** (`mtg-engine/src/tr
- stitchers_apprentice: Engine bug: `trigger_event_index` desync causes `CreatureDied` events from the sacrifice to be skipped when ETB-watch permanents (Champion of the Pari
- village_bell_ringer: ETB trigger resolution skipped if VBR leaves the battlefield before the trigger resolves (`mtg-engine/src/triggers.rs`, line 894–899)
- witchbane_orb: **ETB trigger suppressed when Witchbane Orb leaves the battlefield before trigger resolves** — `mtg-engine/src/triggers.rs` lines 893–898
- woodland_sleuth: **Woodland Sleuth cannot be returned to its own hand when it dies in response to its ETB trigger** — two bugs, both must be fixed:

## bug_falkenrath_noble (auto_target) (15 issues)

- caravan_vigil: Auto-selects first basic land in library order instead of presenting a player search choice (`mtg-engine/src/cards/isd/caravan_vigil.rs` lines 39–50)
- corpse_lunge: Engine auto-selects the exiled creature without presenting a player choice
- corpse_lunge: Test `corpse_lunge_picks_highest_power_creature` (tier8_cards.rs:538) enshrines the wrong auto-selection behavior
- elder_cathar: Transformed DFC incorrectly treated as Human in both the single-target auto-select path and the multi-target engine path
- falkenrath_noble: **Issue 1 — "target player" is auto-targeted instead of chosen** (`mtg-engine/src/cards/isd/falkenrath_noble.rs`, lines 59–68)
- grimgrin_corpse_born: Auto-sacrifice for the activated ability doesn't present a player choice when multiple sacrifice targets are available.
- makeshift_mauler: Engine auto-selects which creature to exile instead of giving the player a choice (`mtg-engine/src/engine.rs` ~line 1574)
- mentor_of_the_meek: **"You may pay {1}" is auto-paid instead of presenting a player choice** (`mtg-engine/src/cards/isd/mentor_of_the_meek.rs`, lines 55–80)
- nevermore: **Auto-selection of card name instead of player choice** (`mtg-engine/src/cards/isd/nevermore.rs:41-53`)
- skaab_goliath: Engine auto-selects which creatures to exile rather than giving the player the choice (`mtg-engine/src/engine.rs:1574–1600`)
- skaab_ruinator: Engine auto-selects which creature cards to exile as the additional cost, rather than presenting the player with a choice (`mtg-engine/src/engine.rs` 
- skirsdag_cultist: **Engine auto-selects which creature to sacrifice instead of presenting a player choice** — `mtg-engine/src/engine.rs` lines 1750–1759
- skirsdag_high_priest: Auto-selection of which two creatures to tap — `mtg-engine/src/cards/isd/skirsdag_high_priest.rs` lines 68–73
- stitched_drake: Engine auto-selects which creature to exile; player cannot choose
- travelers_amulet: **No player choice when multiple basic lands exist** (`mtg-engine/src/cards/isd/travelers_amulet.rs:57`)

## COSMETIC_log (11 issues)

- burning_vengeance: **Log message logged before target is chosen, and describes "opponent" inaccurately** — `mtg-engine/src/cards/isd/burning_vengeance.rs` lines 67–69
- full_moons_rise: Activated ability description in `mtg-engine/src/cards/isd/full_moons_rise.rs` line 57 says "Wolf and Werewolf" when oracle says "Werewolf" only. This
- gatstaf_shepherd: **Log message names the wrong source card when transforming back to front face**
- grizzled_outcasts: Log message is incorrect when transforming back to front face
- instigator_gang: **Log message names wrong source when transforming back** (`instigator_gang.rs:119–121`): When Wildblood Pack transforms back to Instigator Gang (`was
- kruin_outlaw: **Log message says "Kruin Outlaw transforms into Kruin Outlaw" when back face transforms back to front face** (`mtg-engine/src/cards/isd/kruin_outlaw.
- mayor_of_avabruck: **Log message hardcodes wrong source name when transforming back** — `mtg-engine/src/cards/isd/mayor_of_avabruck.rs:119`
- tormented_pariah: **Log message incorrectly names source when Rampaging Werewolf transforms back** (`mtg-engine/src/cards/isd/tormented_pariah.rs:87–88`)
- ulvenwald_mystics: Log message is incorrect when transforming from back face (Ulvenwald Primordials) to front face (Ulvenwald Mystics)
- village_ironsmith: Incorrect log message when Ironfang transforms back to Village Ironsmith (`mtg-engine/src/cards/isd/village_ironsmith.rs:87–88`)
- villagers_of_estwald: **Log message is wrong when Howlpack transforms back to Villagers** (`mtg-engine/src/cards/isd/villagers_of_estwald.rs`, line 88)

## bug_spells_cast_count (11 issues)

- daybreak_ranger: Engine never increments `spells_cast_this_turn` or populates `spells_cast_last_turn`, breaking both transform conditions in actual gameplay.
- gatstaf_shepherd: **Engine never increments `spells_cast_this_turn`; `spells_cast_last_turn` is always empty, making both transform conditions permanently wrong**
- grizzled_outcasts: Engine never updates `spells_cast_this_turn` or `spells_cast_last_turn`; both transform conditions are always wrong
- hanweir_watchkeep: Engine never updates `spells_cast_this_turn` or `spells_cast_last_turn`, causing both werewolf transform conditions to evaluate incorrectly in actual 
- instigator_gang: **Engine never increments `spells_cast_this_turn` or updates `spells_cast_last_turn`**: The fields `state.spells_cast_this_turn` and `state.spells_cas
- kruin_outlaw: **Engine never increments `spells_cast_this_turn` when a spell is cast, and never saves it to `spells_cast_last_turn` at turn end** (`mtg-engine/src/e
- mayor_of_avabruck: **Engine never populates `spells_cast_last_turn`** — both transform conditions are permanently broken in actual gameplay.
- reckless_waif: **Engine never populates `spells_cast_this_turn` or `spells_cast_last_turn`; both conditions are permanently broken**
- tormented_pariah: **Engine never tracks spells cast per turn; `spells_cast_last_turn` is always empty in real gameplay** (`mtg-engine/src/engine.rs`, `mtg-engine/src/st
- ulvenwald_mystics: Engine never increments `spells_cast_this_turn` and never transfers it to `spells_cast_last_turn`; both transform conditions are permanently wrong in 
- villagers_of_estwald: **Engine never populates `spells_cast_last_turn` in real games** (`mtg-engine/src/engine.rs`, `mtg-engine/src/state.rs`)

## bug_simultaneous_death (8 issues)

- abattoir_ghoul: **Engine bug: DeathWatch trigger never collected when Ghoul and victim die simultaneously** (`mtg-engine/src/triggers.rs` lines 418–419)
- falkenrath_noble: **Issue 2 — simultaneous death triggers only once instead of twice** (`mtg-engine/src/triggers.rs`, lines 418–421)
- moldgraf_monstrosity: **`on_dies` unconditionally exiles regardless of current zone, violating ruling 2 about simultaneous Monstrosity deaths**
- murder_of_crows: Simultaneous death: Murder of Crows' triggered ability does not fire when it dies at the same time as another creature — `mtg-engine/src/triggers.rs:4
- rage_thrower: **Issue 1 — Simultaneous death: Rage Thrower's trigger does not fire for a creature dying at the same time** (`mtg-engine/src/triggers.rs`, lines 418–
- selhoff_occultist: Selhoff Occultist's `AnyCreatureDies` trigger does not fire for creatures that die simultaneously with it (e.g., in a board wipe).
- unruly_mob: Simultaneous death: trigger does not fire when Unruly Mob dies in the same SBA pass as another creature you control.
- village_cannibals: **Simultaneous deaths: Village Cannibals doesn't trigger when it dies alongside a Human** (`mtg-engine/src/triggers.rs` lines 417–441, `mtg-engine/src

## FP_anytarget_pw (6 issues)

- blazing_torch: `AnyTarget` engine implementation does not include planeswalkers as valid targets
- brimstone_volley: `AnyTarget` in engine does not include planeswalkers as valid targets — `mtg-engine/src/engine.rs` lines 836–864, 1074–1089, 1343–1358
- geistflame: Engine's `AnyTarget` implementation excludes planeswalkers as valid targets (`mtg-engine/src/engine.rs` lines 836–864, 1074–1090, 1343–1358; `mtg-engi
- heretics_punishment: `AnyTarget` engine implementation excludes planeswalkers as valid targets
- pitchburn_devils: `any_targets` helper omits planeswalkers; Pitchburn Devils' trigger cannot target them
- skirsdag_cultist: **`AnyTarget` does not include planeswalkers as valid targets** — `mtg-engine/src/engine.rs` lines 1343–1358 (activated ability target generation) and

## FP_spurious_trigger (6 issues)

- charmbreaker_devils: Upkeep trigger fires spuriously on the stack during the opponent's upkeep
- charmbreaker_devils: SpellCast trigger fires spuriously on the stack when the opponent casts an instant or sorcery
- cloistered_youth: Spurious upkeep trigger fires for Unholy Fiend (transformed state)
- curse_of_oblivion: Upkeep trigger fires during every player's upkeep, not only the enchanted player's upkeep — spurious triggers placed on stack during the non-cursed pl
- delver_of_secrets: **Transformed Insectile Aberration gets a spurious upkeep trigger on the stack** (engine bug in `mtg-engine/src/triggers.rs` lines 311–327, affecting 
- village_cannibals: **Spurious DeathWatch triggers on non-Human deaths** (`mtg-engine/src/triggers.rs` lines 422–441)

## FP_force_attack (4 issues)

- bloodcrazed_neonate: Forced-attack logic in `engine.rs` (~line 1838) does not call `state.can_attack()`, so a Bloodcrazed Neonate enchanted with Pacifism (or any `PreventA
- curse_of_the_nightly_hunt: The post-DeclareAttackers forced-attack enforcement loop does not check `state.can_attack()`, causing creatures under "can't attack" effects (e.g., Pa
- furor_of_the_bitten: Forced-attack enforcement skips `can_attack()` check — `mtg-engine/src/engine.rs:1838–1846`
- galvanic_juggernaut: Forced attack logic ignores `can_attack()`, violating the "if able" clause

## bug_bonds_of_faith (snapshot) (4 issues)

- bonds_of_faith: **"as long as it's a Human" condition is snapshotted at ETB, never re-evaluated** — `mtg-engine/src/cards/isd/bonds_of_faith.rs` lines 39–69
- butchers_cleaver: **Snapshot "as long as" — lifelink not re-evaluated when equipped creature transforms** (`mtg-engine/src/cards/isd/butchers_cleaver.rs`, lines 14–34 a
- sharpened_pitchfork: "As long as" condition is evaluated once at equip time, not continuously re-evaluated.
- silver_inlaid_dagger: "As long as" Human condition is evaluated once at equip time and never re-evaluated

## COSMETIC_oracle (4 issues)

- bonds_of_faith: **`oracle_text` field is missing the "Enchant creature" first line** — `mtg-engine/src/cards/isd/bonds_of_faith.rs` line 25
- bump_in_the_night: `oracle_text` field in `card_data()` is incomplete (`mtg-engine/src/cards/isd/bump_in_the_night.rs` line 23)
- disciple_of_griselbrand: `oracle_text` field wording does not match Scryfall oracle text (`mtg-engine/src/cards/isd/disciple_of_griselbrand.rs` line 25)
- furor_of_the_bitten: Missing "Enchant creature\n" prefix in `oracle_text` field — `mtg-engine/src/cards/isd/furor_of_the_bitten.rs:22`

## bug_summoning_sickness (3 issues)

- avacynian_priest: Engine does not enforce summoning sickness for `{T}` activated abilities; a freshly entered Avacynian Priest can activate its tap ability on the same 
- furor_of_the_bitten: Forced-attack enforcement ignores Haste overriding summoning sickness — `mtg-engine/src/engine.rs:1827`
- mikaeus_the_lunarch: Summoning sickness not enforced for {T} activated abilities

## bug_hexproof_resolution (3 issues)

- sensory_deprivation: Engine does not check hexproof when evaluating target legality at resolution time
- sever_the_bloodline: Engine does not re-check hexproof legality at resolution time (`mtg-engine/src/stack.rs:8-41`)
- witchbane_orb: **Hexproof not re-validated at spell resolution for player targets** — `mtg-engine/src/stack.rs` line 39

## COSMETIC_llm (2 issues)

- bump_in_the_night: LLM player card knowledge is missing flashback ability (`mtg-player/src/llm.rs` line 84)
- travel_preparations: LLM card knowledge in `mtg-player/src/llm.rs` line 111 describes one target instead of up to two

## bug_hinterland_checkland (2 issues)

- clifftop_retreat: `controller_has_matching_land` only checks runtime object subtypes (`o.subtypes`), never the registry — so real Mountain and Plains cards are never de
- hinterland_harbor: `controller_has_matching_land` only checks object-level subtypes (`o.subtypes`), missing registry-stored subtypes for regularly-played Forest/Island c

## bug_once_per_turn (2 issues)

- darkthicket_wolf: `abilities_activated_this_turn` is never cleared between turns — engine bug causes once-per-turn restriction to become once-per-game permanently
- garruk_relentless: **`abilities_activated_this_turn` never cleared between turns** — `mtg-engine/src/engine.rs:1942`, `mtg-engine/src/engine.rs:3006-3061`

## bug_protection_targeting (2 issues)

- elite_inquisitor: Protection's targeting restriction is not enforced in the engine's ability-targeting path (`mtg-engine/src/engine.rs:758-768` and `mtg-engine/src/engi
- spare_from_evil: Protection's "T" (targeting) aspect not enforced by engine — non-Human creature activated abilities can still target protected creatures

## bug_ghost_quarter_shuffle (2 issues)

- ghost_quarter: **Missing shuffle after library search** (`mtg-engine/src/cards/isd/ghost_quarter.rs`, lines 92–101)
- travelers_amulet: **"then shuffle" is not implemented** (`mtg-engine/src/cards/isd/travelers_amulet.rs:83`)

## bug_card_state_zone (2 issues)

- ludevics_test_subject: **`card_state` (hatchling counters) not reset on zone change** — `mtg-engine/src/state.rs`, `move_object()` lines 479–487
- ludevics_test_subject: **`is_transformed` not reset on zone change** — `mtg-engine/src/state.rs`, `move_object()` lines 479–487

## bug_delver_reveal (1 issues)

- delver_of_secrets: **"You may reveal" choice suppressed when top card is not an instant or sorcery** (`mtg-engine/src/cards/isd/delver_of_secrets.rs` lines 104–118)

## bug_evil_twin_marker (1 issues)

- evil_twin: **`is_evil_twin` marker set before the optional copy choice is made (evil_twin.rs:53–55)**

## bug_planeswalker_damage (1 issues)

- geistflame: `resolve_damage` helper does not remove loyalty counters when dealing damage to a planeswalker (`mtg-engine/src/cards/helpers.rs` lines 52–62)

## bug_grimoire_legend (1 issues)

- grimoire_of_the_dead: Legend rule not applied to legendary creature cards returned by ability 2 if they were never previously on the battlefield (`mtg-engine/src/cards/isd/

## bug_nevermore_flashback (1 issues)

- nevermore: **Nevermore ban not enforced for flashback casts** (`mtg-engine/src/engine.rs:665-747`)

## bug_night_terrors (1 issues)

- night_terrors: **Night Terrors is never moved off the stack when the target player has multiple nonland cards in hand** (`mtg-engine/src/cards/isd/night_terrors.rs:6

## bug_prey_upon_damage (1 issues)

- prey_upon: **fight() emits CombatDamageDealt instead of NonCombatDamageDealt, applying combat-specific effects to fight damage** (`mtg-engine/src/combat.rs:467`,

## bug_thraben_vigilance (1 issues)

- thraben_sentry: **Vigilance incorrectly retained on back face after transform** (`mtg-engine/src/cards/isd/thraben_sentry.rs`, lines 73–76)

## bug_control_revert (1 issues)

- traitorous_blood: Control change is never reverted at end of turn — engine cleanup step does not process `until_end_of_turn_control_changes`

## bug_unbreathing_counters (1 issues)

- unbreathing_horde: "Enters with counters" replacement effect does not fire when Unbreathing Horde enters the battlefield via reanimation (e.g., Unburial Rites)

## bug_undead_alchemist (1 issues)

- undead_alchemist: **Multiple Undead Alchemists cause incorrect life restoration (net life gain) and double milling** (`mtg-engine/src/cards/isd/undead_alchemist.rs:63-9

## UNCLASSIFIED (65)

- angelic_overseer: Sequential SBA processing allows a protecting Human to die before Angelic Overseer's indestructibility is evaluated, causing Angelic Overseer to be in
- balefire_dragon: Battlefield guard in `on_combat_damage_to_player` suppresses the triggered effect if Balefire Dragon has left the battlefield at resolution time (`mtg
- bitterheart_witch: `present_player_choice` builds target list without hexproof filtering (`mtg-engine/src/cards/isd/bitterheart_witch.rs:14-16`) and the corresponding `C
- boneyard_wurm: Graveyard zone display shows base P/T (0/0) instead of dynamically computed P/T
- brain_weevil: Incomplete discard when target player has 3+ cards in hand — `mtg-engine/src/cards/isd/brain_weevil.rs:64-75` + `mtg-engine/src/engine.rs:2009-2023`
- burning_vengeance: **Engine bug: `SpellCast` trigger dispatch restricted to instant/sorcery** — `mtg-engine/src/triggers.rs` lines 644–675
- burning_vengeance: **Card bug: checks `cast_with_flashback` rather than "cast from graveyard"** — `mtg-engine/src/cards/isd/burning_vengeance.rs` lines 48–53
- butchers_cleaver: **Human check ignores transformed state — back-face DFCs incorrectly identified as Human at equip time** (`mtg-engine/src/cards/isd/butchers_cleaver.r
- civilized_scholar: **Stale `attacked_this_turn` flag causes Homicidal Brute to skip transform-back** (`mtg-engine/src/cards/isd/civilized_scholar.rs`, lines 166–191)
- civilized_scholar: **EndStep trigger registered on front face, causing spurious stack entry when not transformed** (`mtg-engine/src/cards/isd/civilized_scholar.rs`, line
- creepy_doll: **Lethal-damage + regeneration scenario: winning the coin flip fails to destroy the creature** (`mtg-engine/src/engine.rs` lines 3118–3125, interactin
- curse_of_the_pierced_heart: Dealing damage to a planeswalker via the choice path does not remove loyalty counters
- curse_of_the_pierced_heart: The upkeep trigger goes on the stack during every player's upkeep, not only the enchanted player's upkeep
- dearly_departed: **Engine: `AnyCreatureEnters` watcher scan only checks `Zone::Battlefield`; Dearly Departed in the graveyard is never dispatched a trigger** (`mtg-eng
- dearly_departed: **Engine: `EnterWatch` trigger resolution also requires watcher to be on `Zone::Battlefield`** (`mtg-engine/src/triggers.rs:914-915`)
- demonmail_hauberk: Player cannot choose which creature to sacrifice for the Equip cost
- disciple_of_griselbrand: Player cannot choose which creature to sacrifice when multiple are available (`mtg-engine/src/engine.rs` lines 1750–1759)
- essence_of_the_wild: **ETB abilities of creatures entering as EotW copies still fire** — `mtg-engine/src/triggers.rs:344-392` and `mtg-engine/src/state.rs:524-575`
- essence_of_the_wild: **EotW entering via non-`on_resolve` path does not apply replacement effect** — `mtg-engine/src/cards/isd/essence_of_the_wild.rs:40-53`
- evil_twin: **Activated ability inaccessible after copying (engine.rs + evil_twin.rs)**
- evil_twin: **ETB abilities of the copied creature never trigger (triggers.rs)**
- garruk_relentless: **`is_legendary` not set in `on_resolve`** — `mtg-engine/src/cards/isd/garruk_relentless.rs:313-321`
- geistcatchers_rig: **Target selection deferred to resolution instead of stack-placement** (`mtg-engine/src/cards/isd/geistcatchers_rig.rs` lines 40–59, `mtg-engine/src/t
- geistcatchers_rig: **`optional: true` conflates target selection with the "you may" decision** (`mtg-engine/src/cards/isd/geistcatchers_rig.rs` line 56)
- ghost_quarter: **"May" search is forced — controller gets no choice** (`mtg-engine/src/cards/isd/ghost_quarter.rs`, line 81–100)
- ghoulcallers_chant: `build_cast_target_spec` returns `CastTargetSpec::SingleTarget` for modal spells containing a `TwoTargets` inner mode, blocking interactive players fr
- grave_bramble: Protection from Zombies incorrectly prevents Grave Bramble from blocking Zombie attackers (`mtg-engine/src/combat.rs:699`)
- grave_bramble: Grimgrin, Corpse-Born's triggered ability can target Grave Bramble despite protection from Zombies (`mtg-engine/src/cards/isd/grimgrin_corpse_born.rs:
- hamlet_captain: Trigger does not resolve if Hamlet Captain leaves the battlefield before the trigger resolves.
- harvest_pyre: Player cannot choose which specific cards to exile; engine arbitrarily picks cards
- inquisitors_flail: Fight damage incorrectly doubled by Flail: `mtg-engine/src/combat.rs` lines 452–454
- into_the_maw_of_hell: `is_valid_target` accepts creatures for the land target slot, allowing the card to be cast with no legal land target
- kessig_cagebreakers: **Attack trigger silently discarded if Kessig is destroyed before resolution** (`mtg-engine/src/triggers.rs:980-985` and `mtg-engine/src/cards/isd/kes
- liliana_of_the_veil: **+1 ability: Player 1's discard resolves before Player 2 even makes their choice, violating the "all at the same time" ruling.**
- mask_of_avacyn: **Duplicate equip action generated via attached-aura loop enables broken re-equip** (`mtg-engine/src/engine.rs:331-338`, `mtg-engine/src/cards/isd/mas
- memorys_journey: **Missing mandatory `Target::Player` target — opponent cannot be targeted with 0 card targets, and player hexproof is never checked** (`mtg-engine/src
- mentor_of_the_meek: **Test enshrines wrong auto-pay behavior** (`mtg-engine/tests/tier15_cards.rs`, lines 71–99)
- mirror_mad_phantasm: **Reveal loop uses `draw_top_card()` which sets `has_drawn_from_empty = true` on library exhaustion, causing the player to incorrectly lose the game**
- moonmist: Second Moonmist fails to transform Werewolf DFCs after they naturally untransform back to their Human front face
- moorland_haunt: Player cannot choose which creature card to exile when multiple are in the graveyard (`mtg-engine/src/cards/isd/moorland_haunt.rs` lines 85–96 and `mt
- night_terrors: **Wrong `PendingEffect` variant used for Night Terrors** (`mtg-engine/src/cards/isd/night_terrors.rs:66`)
- past_in_flames: `until_end_of_turn_flashback` is never cleared at end-of-turn cleanup, so flashback grants persist indefinitely across turns.
- past_in_flames: Cards with no mana cost that are instants or sorceries receive `ManaCost::free()` as their flashback cost, making them castable for {0} when the oracl
- prey_upon: **One illegal target does not prevent fight, violating the Scryfall ruling** (`mtg-engine/src/stack.rs:79–86`, `mtg-engine/src/cards/isd/prey_upon.rs:
- rage_thrower: **Issue 3 — Damage targeting a planeswalker does not reduce loyalty counters** (`mtg-engine/src/engine.rs`, lines 2179–2191; `mtg-engine/src/cards/isd
- rage_thrower: **Issue 4 — Trigger description and target-choice prompt omit "or planeswalker"** (`mtg-engine/src/cards/isd/rage_thrower.rs`, lines 33 and 57)
- reaper_from_the_abyss: Intervening-if clause not enforced at trigger collection time (`mtg-engine/src/triggers.rs:604–641`, `mtg-engine/src/cards/isd/reaper_from_the_abyss.r
- rooftop_storm: Rooftop Storm alternative cost not offered for Zombie creature spells cast from the graveyard
- smite_the_monstrous: Target legality at resolution does not re-check the power condition (`mtg-engine/src/stack.rs:8-41`)
- snapcaster_mage: **`until_end_of_turn_flashback` is never cleared at end of turn** (`mtg-engine/src/engine.rs:3006–3061`)
- snapcaster_mage: **Snapcaster Mage incorrectly excludes cards with innate flashback from eligible targets** (`mtg-engine/src/cards/isd/snapcaster_mage.rs:48–53`)
- spare_from_evil: Protection's "D" (damage) aspect not enforced for non-combat damage from non-Human creature sources
- splinterfright: Upkeep trigger does not resolve if Splinterfright has left the battlefield between trigger collection and resolution
- sturmgeist: Draw skipped when Sturmgeist leaves battlefield before trigger resolves (`mtg-engine/src/cards/isd/sturmgeist.rs:46-49`)
- thraben_sentry: **"you may" is bypassed — card always auto-transforms, player never gets a choice** (`mtg-engine/src/cards/isd/thraben_sentry.rs`, lines 72–76)
- thraben_sentry: **Test enshrines wrong auto-transform behavior** (`mtg-engine/tests/tier15_cards.rs`, lines 1392–1409, test `thraben_sentry_transforms_when_creature_d
- tribute_to_hunger: Missing `is_valid_target` override to enforce "target opponent" restriction
- unburial_rites: **Missing `target_requirement()` override — spell treated as untargeted**
- unburial_rites: **Target selected at resolution time, not at cast time — ignores `targets` parameter**
- unburial_rites: **Spell can be cast with no legal targets**
- undead_alchemist: **Second triggered ability only fires from Undead Alchemist's own mill, not from all sources** (`mtg-engine/src/cards/isd/undead_alchemist.rs:82-99`)
- undead_alchemist: **First-strike Zombie dealing lethal combat damage causes player loss before Alchemist trigger fires** (`mtg-engine/src/combat.rs:146-153`, `mtg-engin
- undead_alchemist: **Lifelink on the Zombie source incorrectly grants life when Undead Alchemist's replacement applies** (`mtg-engine/src/combat.rs:539-549`, `mtg-engine
- village_ironsmith: Engine never tracks spells cast per turn, so transform conditions are always wrong in a real game (`mtg-engine/src/engine.rs` — turn transition in `ad
- woodland_sleuth: **Intervening-if condition not checked at trigger-collection time** (`mtg-engine/src/triggers.rs` lines 344–363)
