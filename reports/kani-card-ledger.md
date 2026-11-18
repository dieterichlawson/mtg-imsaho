# Card proofs at b6d6c43

What these are: the Kani card harnesses that are kept, each of what a
card declares (abilities and costs, targets, triggers, mana abilities,
static and replacement effects) or of what its own code writes to the
game state. They are not proofs that a card works when it resolves:
the 715 harnesses that claimed that by asserting a record of the calls a
card made, with recorders standing in for the engine, were removed
(2026-11-18). What a card does through the engine is checked with the
engine itself, on every small board, by `tests/card_semantics.rs`
(`docs/plans/cards-in-the-loop.md`, part B).

Each harness's last result. The 275 kept from before are as the last run
of every harness left them (`scripts/kani/times.txt`), their own text
unchanged by the removal; 39 of them, the two fastest of each module,
and the ten new ones of each Equipment's check of its equip target were
run again after it, at 5daffcf, and all 49 prove.


285 harnesses over 147 cards and 4 tables of cards; 123 cards have none.

- SUCCESSFUL: 285

| card | harnesses | proved | not proved |
|---|---|---|---|
| ancient_grudge | 1 | 1 |  |
| angelic_overseer | 1 | 1 |  |
| armored_skaab | 1 | 1 |  |
| avacynian_priest | 2 | 2 |  |
| avacyns_pilgrim | 1 | 1 |  |
| back_from_the_brink | 8 | 8 |  |
| battleground_geist | 1 | 1 |  |
| bitterheart_witch | 2 | 2 |  |
| blazing_torch | 1 | 1 |  |
| bloodline_keeper | 8 | 8 |  |
| bonds_of_faith | 1 | 1 |  |
| boneyard_wurm | 1 | 1 |  |
| brain_weevil | 1 | 1 |  |
| bramblecrush | 1 | 1 |  |
| brimstone_volley | 1 | 1 |  |
| butchers_cleaver | 2 | 2 |  |
| charmbreaker_devils | 1 | 1 |  |
| civilized_scholar | 3 | 3 |  |
| claustrophobia | 1 | 1 |  |
| clifftop_retreat | 2 | 2 |  |
| cloistered_youth | 1 | 1 |  |
| cobbled_wings | 2 | 2 |  |
| counterspell | 1 | 1 |  |
| creeping_renaissance | 1 | 1 |  |
| crossway_vampire | 1 | 1 |  |
| curiosity | 1 | 1 |  |
| darkthicket_wolf | 2 | 2 |  |
| daybreak_ranger | 3 | 3 |  |
| dead_weight | 1 | 1 |  |
| dearly_departed | 1 | 1 |  |
| demonmail_hauberk | 2 | 2 |  |
| devils_play | 1 | 1 |  |
| diregraf_ghoul | 1 | 1 |  |
| disciple_of_griselbrand | 1 | 1 |  |
| dissipate | 1 | 1 |  |
| doom_blade | 1 | 1 |  |
| elder_of_laurels | 2 | 2 |  |
| elite_inquisitor | 1 | 1 |  |
| essence_of_the_wild | 1 | 1 |  |
| evil_twin | 6 | 6 |  |
| feral_ridgewolf | 2 | 2 |  |
| festerhide_boar | 1 | 1 |  |
| fiend_hunter | 1 | 1 |  |
| forest | 1 | 1 |  |
| frightful_delusion | 2 | 2 |  |
| full_moons_rise | 2 | 2 |  |
| furor_of_the_bitten | 1 | 1 |  |
| gallows_warden | 1 | 1 |  |
| garruk_relentless | 2 | 2 |  |
| gavony_township | 2 | 2 |  |
| geistcatchers_rig | 1 | 1 |  |
| geistflame | 1 | 1 |  |
| ghost_quarter | 2 | 2 |  |
| ghostly_possession | 1 | 1 |  |
| ghoulcallers_bell | 1 | 1 |  |
| ghoulraiser | 1 | 1 |  |
| glorious_anthem | 1 | 1 |  |
| grave_bramble | 1 | 1 |  |
| graveyard_shovel | 1 | 1 |  |
| grimgrin_corpse_born | 2 | 2 |  |
| grimoire_of_the_dead | 6 | 6 |  |
| gruesome_deformity | 1 | 1 |  |
| hamlet_captain | 18 | 18 |  |
| heartless_summoning | 1 | 1 |  |
| heretics_punishment | 1 | 1 |  |
| hinterland_harbor | 2 | 2 |  |
| hollowhenge_scavenger | 1 | 1 |  |
| holy_strength | 1 | 1 |  |
| hysterical_blindness | 1 | 1 |  |
| inquisitors_flail | 2 | 2 |  |
| intangible_virtue | 1 | 1 |  |
| invisible_stalker | 1 | 1 |  |
| island | 1 | 1 |  |
| isolated_chapel | 2 | 2 |  |
| kessig_wolf | 2 | 2 |  |
| kessig_wolf_run | 2 | 2 |  |
| lantern_spirit | 1 | 1 |  |
| lava_axe | 1 | 1 |  |
| lightning_bolt | 1 | 1 |  |
| liliana_of_the_veil | 16 | 16 |  |
| ludevics_test_subject | 3 | 3 |  |
| manor_gargoyle | 2 | 2 |  |
| manor_skeleton | 1 | 1 |  |
| mask_of_avacyn | 2 | 2 |  |
| mentor_of_the_meek | 3 | 3 |  |
| mikaeus_the_lunarch | 2 | 2 |  |
| mindshrieker | 1 | 1 |  |
| mirror_mad_phantasm | 1 | 1 |  |
| moorland_haunt | 9 | 9 |  |
| morkrut_banshee | 1 | 1 |  |
| mountain | 1 | 1 |  |
| naturalize | 1 | 1 |  |
| nephalia_drownyard | 1 | 1 |  |
| nevermore | 6 | 6 |  |
| night_revelers | 1 | 1 |  |
| nightbirds_clutches | 1 | 1 |  |
| olivia_voldaren | 1 | 1 |  |
| one_eyed_scarecrow | 1 | 1 |  |
| orchard_spirit | 1 | 1 |  |
| pacifism | 1 | 1 |  |
| parallel_lives | 1 | 1 |  |
| past_in_flames | 16 | 16 |  |
| plains | 1 | 1 |  |
| rally_the_peasants | 1 | 1 |  |
| reaper_from_the_abyss | 1 | 1 |  |
| rebuke | 1 | 1 |  |
| rooftop_storm | 1 | 1 |  |
| runechanters_pike | 3 | 3 |  |
| scourge_of_geier_reach | 1 | 1 |  |
| screeching_bat | 1 | 1 |  |
| selfless_cathar | 2 | 2 |  |
| sensory_deprivation | 1 | 1 |  |
| sharpened_pitchfork | 2 | 2 |  |
| shimmering_grotto | 1 | 1 |  |
| silver_inlaid_dagger | 2 | 2 |  |
| silverchase_fox | 1 | 1 |  |
| skaab_ruinator | 1 | 1 |  |
| skeletal_grimace | 1 | 1 |  |
| skirsdag_cultist | 1 | 1 |  |
| skirsdag_high_priest | 2 | 2 |  |
| slayer_of_the_wicked | 1 | 1 |  |
| smite_the_monstrous | 1 | 1 |  |
| snapcaster_mage | 2 | 2 |  |
| sol_ring | 1 | 1 |  |
| somberwald_spider | 1 | 1 |  |
| spare_from_evil | 1 | 1 |  |
| spectral_flight | 1 | 1 |  |
| stensia_bloodhall | 1 | 1 |  |
| stitchers_apprentice | 1 | 1 |  |
| stony_silence | 1 | 1 |  |
| sulfur_falls | 2 | 2 |  |
| swamp | 1 | 1 |  |
| thraben_sentry | 1 | 1 |  |
| travelers_amulet | 1 | 1 |  |
| tree_of_redemption | 1 | 1 |  |
| trepanation_blade | 2 | 2 |  |
| unbreathing_horde | 1 | 1 |  |
| urgent_exorcism | 1 | 1 |  |
| vampire_interloper | 1 | 1 |  |
| vampiric_fury | 8 | 8 |  |
| victim_of_night | 1 | 1 |  |
| village_bell_ringer | 1 | 1 |  |
| witchbane_orb | 1 | 1 |  |
| wooden_stake | 2 | 2 |  |
| woodland_cemetery | 2 | 2 |  |
| woodland_sleuth | 1 | 1 |  |
| wreath_of_geists | 2 | 2 |  |

## Tables of cards

| harness | result | time |
|---|---|---|
| `activated::targeted_abilities_accept_what_their_text_targets` | SUCCESSFUL | 105s |
| `curses::the_static_curses_print_what_their_text_says` | SUCCESSFUL | 10s |
| `curses::the_upkeep_curses_trigger_in_the_enchanted_players_upkeep` | SUCCESSFUL | 33s |
| `targets::the_two_target_spells_accept_what_their_text_targets` | SUCCESSFUL | 31s |

## Cards with no harness

What these do is the engine's, played with it by `tests/card_semantics.rs`, or printed data alone.

abattoir_ghoul, abbey_griffin, altars_reap, ambush_viper, angel_of_flight_alabaster, army_of_the_damned, ashmouth_hound, balefire_dragon, blasphemous_act, bloodcrazed_neonate, bloodgift_demon, bump_in_the_night, burning_vengeance, cackling_counterpart, caravan_vigil, cellar_door, champion_of_the_parish, chapel_geist, coral_merfolk, corpse_lunge, creepy_doll, curse_of_deaths_hold, curse_of_oblivion, curse_of_stalked_prey, curse_of_the_bloody_tome, curse_of_the_nightly_hunt, curse_of_the_pierced_heart, delver_of_secrets, deranged_assistant, desperate_ravings, divination, divine_reckoning, doomed_traveler, dream_twist, elder_cathar, endless_ranks_of_the_dead, falkenrath_marauders, falkenrath_noble, feeling_of_dread, forbidden_alchemy, fortress_crab, galvanic_juggernaut, gatstaf_shepherd, geist_honored_monk, geist_of_saint_traft, ghoulcallers_chant, giant_growth, gnaw_to_the_bone, goblin_piker, grasp_of_phantoms, grizzled_outcasts, grizzly_bears, gutter_grime, hanweir_watchkeep, harvest_pyre, infernal_plunge, instigator_gang, into_the_maw_of_hell, kalonian_tusker, kessig_cagebreakers, kindercatch, kruin_outlaw, laboratory_maniac, lost_in_the_mist, lumberknot, make_a_wish, makeshift_mauler, markov_patrician, mausoleum_guard, maw_of_the_mire, mayor_of_avabruck, memorys_journey, midnight_haunting, moan_of_the_unhallowed, moldgraf_monstrosity, moment_of_heroism, moon_heron, moonmist, mulch, murder_of_crows, night_terrors, paraselene, pitchburn_devils, prey_upon, purify_the_grave, rage_thrower, rakish_heir, rangers_guile, reckless_waif, riot_devils, rolling_temblor, rotting_fensnake, runic_repetition, savannah_lions, selhoff_occultist, sever_the_bloodline, silent_departure, skaab_goliath, spectral_rider, spider_spawning, spidery_grasp, splinterfright, stitched_drake, stromkirk_noble, stromkirk_patrol, sturmgeist, swords_to_plowshares, think_twice, thraben_purebloods, tormented_pariah, traitorous_blood, travel_preparations, tribute_to_hunger, typhoid_rats, ulvenwald_mystics, unburial_rites, undead_alchemist, unruly_mob, village_cannibals, village_ironsmith, villagers_of_estwald, voiceless_spirit, walking_corpse

