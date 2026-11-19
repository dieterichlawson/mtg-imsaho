# Card proofs at the engine-under-card commit

What these are. Two kinds of Kani harness, counted apart in the table.

**Declared** (`proofs::cards`, 285): what a card declares (abilities and
costs, targets, triggers, mana abilities, static and replacement
effects) or what its own code writes to the game state. They are not
proofs that a card works when it resolves: the 715 harnesses that
claimed that by asserting a record of the calls a card made, with
recorders standing in for the engine, were removed (2026-11-18).

**With the engine** (`proofs::engine_under_card`, 368): the card's own
hook run with the engine's own actions under it and no recorder, the
board afterwards asserted against the card's text. 73 of the 88 spells,
and permanents' hooks: enters, death, combat-damage, upkeep, end-step,
attack and block triggers, Equipment, and the activated abilities that
resolve without a question (2026-11-20). What is left out of a harness,
and why, is said where the harness stands; `docs/plans/cards-in-the-loop.md`
(B2) lists what is not yet proved at all.

What a card does through cast, stack, resolution, state-based actions
and triggers together is checked with the engine itself, on every small
board, by `tests/card_semantics.rs` (`docs/plans/cards-in-the-loop.md`,
part B).

Each harness's last result (`scripts/kani/times.txt`). The 285 declared
ones are as the last run of every harness left them, 49 run again after
the removal; the 368 with the engine were each run at the code as
committed, between 2026-11-18 and 2026-11-20, on a machine busy with
other runs (their times are upper bounds: 9.2 hours of solver time in
all, the longest 12 minutes).

653 harnesses over 230 cards and 4 tables of cards: 285 of what a card declares or writes itself, and 368 of 165 cards resolving with the engine itself under them. 40 cards have none.

- SUCCESSFUL: 653

| card | declared | with the engine | proved | not proved |
|---|---|---|---|---|
| abattoir_ghoul | 0 | 2 | 2 |  |
| altars_reap | 0 | 8 | 8 |  |
| ancient_grudge | 1 | 1 | 2 |  |
| angel_of_flight_alabaster | 0 | 1 | 1 |  |
| angelic_overseer | 1 | 0 | 1 |  |
| armored_skaab | 1 | 6 | 7 |  |
| ashmouth_hound | 0 | 2 | 2 |  |
| avacynian_priest | 2 | 1 | 3 |  |
| avacyns_pilgrim | 1 | 0 | 1 |  |
| back_from_the_brink | 8 | 0 | 8 |  |
| battleground_geist | 1 | 0 | 1 |  |
| bitterheart_witch | 2 | 0 | 2 |  |
| blasphemous_act | 0 | 2 | 2 |  |
| blazing_torch | 1 | 1 | 2 |  |
| bloodcrazed_neonate | 0 | 2 | 2 |  |
| bloodgift_demon | 0 | 4 | 4 |  |
| bloodline_keeper | 8 | 2 | 10 |  |
| bonds_of_faith | 1 | 2 | 3 |  |
| boneyard_wurm | 1 | 0 | 1 |  |
| brain_weevil | 1 | 0 | 1 |  |
| bramblecrush | 1 | 4 | 5 |  |
| brimstone_volley | 1 | 6 | 7 |  |
| bump_in_the_night | 0 | 1 | 1 |  |
| butchers_cleaver | 2 | 2 | 4 |  |
| cellar_door | 0 | 5 | 5 |  |
| champion_of_the_parish | 0 | 2 | 2 |  |
| charmbreaker_devils | 1 | 1 | 2 |  |
| civilized_scholar | 3 | 0 | 3 |  |
| claustrophobia | 1 | 2 | 3 |  |
| clifftop_retreat | 2 | 0 | 2 |  |
| cloistered_youth | 1 | 6 | 7 |  |
| cobbled_wings | 2 | 2 | 4 |  |
| corpse_lunge | 0 | 3 | 3 |  |
| counterspell | 1 | 2 | 3 |  |
| creeping_renaissance | 1 | 0 | 1 |  |
| crossway_vampire | 1 | 1 | 2 |  |
| curiosity | 1 | 6 | 7 |  |
| curse_of_deaths_hold | 0 | 1 | 1 |  |
| curse_of_oblivion | 0 | 1 | 1 |  |
| curse_of_stalked_prey | 0 | 3 | 3 |  |
| curse_of_the_bloody_tome | 0 | 5 | 5 |  |
| curse_of_the_nightly_hunt | 0 | 1 | 1 |  |
| curse_of_the_pierced_heart | 0 | 5 | 5 |  |
| darkthicket_wolf | 2 | 1 | 3 |  |
| daybreak_ranger | 3 | 3 | 6 |  |
| dead_weight | 1 | 1 | 2 |  |
| dearly_departed | 1 | 0 | 1 |  |
| demonmail_hauberk | 2 | 1 | 3 |  |
| deranged_assistant | 0 | 3 | 3 |  |
| devils_play | 1 | 9 | 10 |  |
| diregraf_ghoul | 1 | 0 | 1 |  |
| disciple_of_griselbrand | 1 | 1 | 2 |  |
| dissipate | 1 | 2 | 3 |  |
| divination | 0 | 8 | 8 |  |
| doom_blade | 1 | 1 | 2 |  |
| doomed_traveler | 0 | 1 | 1 |  |
| dream_twist | 0 | 8 | 8 |  |
| elder_cathar | 0 | 2 | 2 |  |
| elder_of_laurels | 2 | 2 | 4 |  |
| elite_inquisitor | 1 | 0 | 1 |  |
| endless_ranks_of_the_dead | 0 | 4 | 4 |  |
| essence_of_the_wild | 1 | 0 | 1 |  |
| evil_twin | 6 | 1 | 7 |  |
| falkenrath_marauders | 0 | 2 | 2 |  |
| falkenrath_noble | 0 | 2 | 2 |  |
| feeling_of_dread | 0 | 3 | 3 |  |
| feral_ridgewolf | 2 | 1 | 3 |  |
| festerhide_boar | 1 | 0 | 1 |  |
| fiend_hunter | 1 | 0 | 1 |  |
| forest | 1 | 0 | 1 |  |
| frightful_delusion | 2 | 0 | 2 |  |
| full_moons_rise | 2 | 1 | 3 |  |
| furor_of_the_bitten | 1 | 1 | 2 |  |
| gallows_warden | 1 | 0 | 1 |  |
| galvanic_juggernaut | 0 | 2 | 2 |  |
| garruk_relentless | 2 | 0 | 2 |  |
| gatstaf_shepherd | 0 | 2 | 2 |  |
| gavony_township | 2 | 1 | 3 |  |
| geist_honored_monk | 0 | 1 | 1 |  |
| geistcatchers_rig | 1 | 1 | 2 |  |
| geistflame | 1 | 3 | 4 |  |
| ghost_quarter | 2 | 0 | 2 |  |
| ghostly_possession | 1 | 1 | 2 |  |
| ghoulcallers_bell | 1 | 3 | 4 |  |
| ghoulcallers_chant | 0 | 2 | 2 |  |
| ghoulraiser | 1 | 0 | 1 |  |
| giant_growth | 0 | 1 | 1 |  |
| glorious_anthem | 1 | 0 | 1 |  |
| gnaw_to_the_bone | 0 | 3 | 3 |  |
| grasp_of_phantoms | 0 | 2 | 2 |  |
| grave_bramble | 1 | 0 | 1 |  |
| graveyard_shovel | 1 | 0 | 1 |  |
| grimgrin_corpse_born | 2 | 3 | 5 |  |
| grimoire_of_the_dead | 6 | 0 | 6 |  |
| grizzled_outcasts | 0 | 2 | 2 |  |
| gruesome_deformity | 1 | 1 | 2 |  |
| gutter_grime | 0 | 2 | 2 |  |
| hamlet_captain | 18 | 4 | 22 |  |
| hanweir_watchkeep | 0 | 4 | 4 |  |
| harvest_pyre | 0 | 2 | 2 |  |
| heartless_summoning | 1 | 0 | 1 |  |
| heretics_punishment | 1 | 0 | 1 |  |
| hinterland_harbor | 2 | 0 | 2 |  |
| hollowhenge_scavenger | 1 | 2 | 3 |  |
| holy_strength | 1 | 1 | 2 |  |
| hysterical_blindness | 1 | 1 | 2 |  |
| infernal_plunge | 0 | 1 | 1 |  |
| inquisitors_flail | 2 | 1 | 3 |  |
| instigator_gang | 0 | 4 | 4 |  |
| intangible_virtue | 1 | 0 | 1 |  |
| into_the_maw_of_hell | 0 | 1 | 1 |  |
| invisible_stalker | 1 | 0 | 1 |  |
| island | 1 | 0 | 1 |  |
| isolated_chapel | 2 | 0 | 2 |  |
| kessig_cagebreakers | 0 | 1 | 1 |  |
| kessig_wolf | 2 | 1 | 3 |  |
| kessig_wolf_run | 2 | 2 | 4 |  |
| kruin_outlaw | 0 | 4 | 4 |  |
| lantern_spirit | 1 | 2 | 3 |  |
| lava_axe | 1 | 2 | 3 |  |
| lightning_bolt | 1 | 3 | 4 |  |
| liliana_of_the_veil | 16 | 0 | 16 |  |
| lost_in_the_mist | 0 | 1 | 1 |  |
| ludevics_test_subject | 3 | 3 | 6 |  |
| lumberknot | 0 | 3 | 3 |  |
| manor_gargoyle | 2 | 1 | 3 |  |
| manor_skeleton | 1 | 2 | 3 |  |
| mask_of_avacyn | 2 | 1 | 3 |  |
| mausoleum_guard | 0 | 1 | 1 |  |
| maw_of_the_mire | 0 | 1 | 1 |  |
| mayor_of_avabruck | 0 | 5 | 5 |  |
| mentor_of_the_meek | 3 | 0 | 3 |  |
| midnight_haunting | 0 | 1 | 1 |  |
| mikaeus_the_lunarch | 2 | 2 | 4 |  |
| mindshrieker | 1 | 0 | 1 |  |
| mirror_mad_phantasm | 1 | 0 | 1 |  |
| moan_of_the_unhallowed | 0 | 1 | 1 |  |
| moment_of_heroism | 0 | 1 | 1 |  |
| moorland_haunt | 9 | 1 | 10 |  |
| morkrut_banshee | 1 | 2 | 3 |  |
| mountain | 1 | 0 | 1 |  |
| murder_of_crows | 0 | 4 | 4 |  |
| naturalize | 1 | 2 | 3 |  |
| nephalia_drownyard | 1 | 4 | 5 |  |
| nevermore | 6 | 0 | 6 |  |
| night_revelers | 1 | 0 | 1 |  |
| nightbirds_clutches | 1 | 3 | 4 |  |
| olivia_voldaren | 1 | 0 | 1 |  |
| one_eyed_scarecrow | 1 | 0 | 1 |  |
| orchard_spirit | 1 | 0 | 1 |  |
| pacifism | 1 | 1 | 2 |  |
| parallel_lives | 1 | 0 | 1 |  |
| paraselene | 0 | 2 | 2 |  |
| past_in_flames | 16 | 0 | 16 |  |
| pitchburn_devils | 0 | 3 | 3 |  |
| plains | 1 | 0 | 1 |  |
| prey_upon | 0 | 1 | 1 |  |
| purify_the_grave | 0 | 2 | 2 |  |
| rage_thrower | 0 | 1 | 1 |  |
| rakish_heir | 0 | 2 | 2 |  |
| rally_the_peasants | 1 | 1 | 2 |  |
| rangers_guile | 0 | 1 | 1 |  |
| reaper_from_the_abyss | 1 | 2 | 3 |  |
| rebuke | 1 | 1 | 2 |  |
| reckless_waif | 0 | 2 | 2 |  |
| rolling_temblor | 0 | 1 | 1 |  |
| rooftop_storm | 1 | 0 | 1 |  |
| runechanters_pike | 3 | 1 | 4 |  |
| runic_repetition | 0 | 1 | 1 |  |
| scourge_of_geier_reach | 1 | 0 | 1 |  |
| screeching_bat | 1 | 0 | 1 |  |
| selfless_cathar | 2 | 1 | 3 |  |
| selhoff_occultist | 0 | 6 | 6 |  |
| sensory_deprivation | 1 | 1 | 2 |  |
| sever_the_bloodline | 0 | 1 | 1 |  |
| sharpened_pitchfork | 2 | 2 | 4 |  |
| shimmering_grotto | 1 | 0 | 1 |  |
| silent_departure | 0 | 1 | 1 |  |
| silver_inlaid_dagger | 2 | 2 | 4 |  |
| silverchase_fox | 1 | 1 | 2 |  |
| skaab_ruinator | 1 | 0 | 1 |  |
| skeletal_grimace | 1 | 1 | 2 |  |
| skirsdag_cultist | 1 | 3 | 4 |  |
| skirsdag_high_priest | 2 | 1 | 3 |  |
| slayer_of_the_wicked | 1 | 1 | 2 |  |
| smite_the_monstrous | 1 | 1 | 2 |  |
| snapcaster_mage | 2 | 0 | 2 |  |
| sol_ring | 1 | 0 | 1 |  |
| somberwald_spider | 1 | 0 | 1 |  |
| spare_from_evil | 1 | 0 | 1 |  |
| spectral_flight | 1 | 1 | 2 |  |
| spider_spawning | 0 | 3 | 3 |  |
| spidery_grasp | 0 | 1 | 1 |  |
| splinterfright | 0 | 4 | 4 |  |
| stensia_bloodhall | 1 | 2 | 3 |  |
| stitchers_apprentice | 1 | 0 | 1 |  |
| stony_silence | 1 | 0 | 1 |  |
| stromkirk_noble | 0 | 2 | 2 |  |
| stromkirk_patrol | 0 | 2 | 2 |  |
| sturmgeist | 0 | 6 | 6 |  |
| sulfur_falls | 2 | 0 | 2 |  |
| swamp | 1 | 0 | 1 |  |
| swords_to_plowshares | 0 | 2 | 2 |  |
| think_twice | 0 | 6 | 6 |  |
| thraben_sentry | 1 | 4 | 5 |  |
| tormented_pariah | 0 | 2 | 2 |  |
| traitorous_blood | 0 | 1 | 1 |  |
| travel_preparations | 0 | 3 | 3 |  |
| travelers_amulet | 1 | 0 | 1 |  |
| tree_of_redemption | 1 | 1 | 2 |  |
| trepanation_blade | 2 | 1 | 3 |  |
| tribute_to_hunger | 0 | 2 | 2 |  |
| ulvenwald_mystics | 0 | 3 | 3 |  |
| unbreathing_horde | 1 | 0 | 1 |  |
| unburial_rites | 0 | 1 | 1 |  |
| undead_alchemist | 0 | 2 | 2 |  |
| unruly_mob | 0 | 2 | 2 |  |
| urgent_exorcism | 1 | 2 | 3 |  |
| vampire_interloper | 1 | 0 | 1 |  |
| vampiric_fury | 8 | 1 | 9 |  |
| victim_of_night | 1 | 1 | 2 |  |
| village_bell_ringer | 1 | 1 | 2 |  |
| village_cannibals | 0 | 2 | 2 |  |
| village_ironsmith | 0 | 2 | 2 |  |
| villagers_of_estwald | 0 | 2 | 2 |  |
| witchbane_orb | 1 | 0 | 1 |  |
| wooden_stake | 2 | 3 | 5 |  |
| woodland_cemetery | 2 | 0 | 2 |  |
| woodland_sleuth | 1 | 0 | 1 |  |
| wreath_of_geists | 2 | 3 | 5 |  |

## Tables of cards

| harness | result | time |
|---|---|---|
| `activated::targeted_abilities_accept_what_their_text_targets` | SUCCESSFUL | 105s |
| `curses::the_static_curses_print_what_their_text_says` | SUCCESSFUL | 10s |
| `curses::the_upkeep_curses_trigger_in_the_enchanted_players_upkeep` | SUCCESSFUL | 33s |
| `targets::the_two_target_spells_accept_what_their_text_targets` | SUCCESSFUL | 31s |

## Cards with no harness

What these do is the engine's, played with it by `tests/card_semantics.rs`, or printed data alone.

abbey_griffin, ambush_viper, army_of_the_damned, balefire_dragon, burning_vengeance, cackling_counterpart, caravan_vigil, chapel_geist, coral_merfolk, creepy_doll, delver_of_secrets, desperate_ravings, divine_reckoning, forbidden_alchemy, fortress_crab, geist_of_saint_traft, goblin_piker, grizzly_bears, kalonian_tusker, kindercatch, laboratory_maniac, make_a_wish, makeshift_mauler, markov_patrician, memorys_journey, moldgraf_monstrosity, moon_heron, moonmist, mulch, night_terrors, riot_devils, rotting_fensnake, savannah_lions, skaab_goliath, spectral_rider, stitched_drake, thraben_purebloods, typhoid_rats, voiceless_spirit, walking_corpse

