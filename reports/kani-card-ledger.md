# Card proofs at the engine-under-card commit

What these are. Two kinds of Kani harness, counted apart in the table.

**Declared** (`proofs::cards`, 285): what a card declares (abilities and
costs, targets, triggers, mana abilities, static and replacement
effects) or what its own code writes to the game state. They are not
proofs that a card works when it resolves: the 715 harnesses that
claimed that by asserting a record of the calls a card made, with
recorders standing in for the engine, were removed (2026-11-18).

**With the engine** (`proofs::engine_under_card`, 603): the card's own
hook run with the engine's own actions under it and no recorder, the
board afterwards asserted against the card's text; or, for a permanent
with no hook to run, the card on the battlefield and the board read
back through the engine's own layers and queries, or the event its
text replaces run through the engine. 87 of the 88 spells and 181 of
the 182 permanents (2026-11-20). What is left out of a harness, and
why, is said where the harness stands; `docs/plans/cards-in-the-loop.md`
(B2) lists the two cards with no such harness and those whose harness
is of part of their text.

What a card does through cast, stack, resolution, state-based actions
and triggers together is checked with the engine itself, on every small
board, by `tests/card_semantics.rs` (`docs/plans/cards-in-the-loop.md`,
part B).

Each harness's last result (`scripts/kani/times.txt`). The 285 declared
ones are as the last run of every harness left them, 49 run again after
the removal; the 603 with the engine were each run at the code as
committed, from 2026-11-18 to 2026-11-20, on a machine busy with other
runs (their times are upper bounds: 15.9 hours of solver time in all,
the longest 12 minutes).

888 harnesses over 269 cards and 4 tables of cards: 285 of what a card declares or writes itself, and 603 of 268 cards resolving with the engine itself under them. 1 cards have none.

- SUCCESSFUL: 888

| card | declared | with the engine | proved | not proved |
|---|---|---|---|---|
| abattoir_ghoul | 0 | 2 | 2 |  |
| abbey_griffin | 0 | 1 | 1 |  |
| altars_reap | 0 | 8 | 8 |  |
| ambush_viper | 0 | 1 | 1 |  |
| ancient_grudge | 1 | 1 | 2 |  |
| angel_of_flight_alabaster | 0 | 1 | 1 |  |
| angelic_overseer | 1 | 2 | 3 |  |
| armored_skaab | 1 | 6 | 7 |  |
| ashmouth_hound | 0 | 2 | 2 |  |
| avacynian_priest | 2 | 1 | 3 |  |
| avacyns_pilgrim | 1 | 1 | 2 |  |
| back_from_the_brink | 8 | 2 | 10 |  |
| balefire_dragon | 0 | 2 | 2 |  |
| battleground_geist | 1 | 1 | 2 |  |
| bitterheart_witch | 2 | 2 | 4 |  |
| blasphemous_act | 0 | 2 | 2 |  |
| blazing_torch | 1 | 3 | 4 |  |
| bloodcrazed_neonate | 0 | 2 | 2 |  |
| bloodgift_demon | 0 | 4 | 4 |  |
| bloodline_keeper | 8 | 2 | 10 |  |
| bonds_of_faith | 1 | 2 | 3 |  |
| boneyard_wurm | 1 | 2 | 3 |  |
| brain_weevil | 1 | 4 | 5 |  |
| bramblecrush | 1 | 4 | 5 |  |
| brimstone_volley | 1 | 6 | 7 |  |
| bump_in_the_night | 0 | 1 | 1 |  |
| burning_vengeance | 0 | 3 | 3 |  |
| butchers_cleaver | 2 | 2 | 4 |  |
| cackling_counterpart | 0 | 2 | 2 |  |
| caravan_vigil | 0 | 4 | 4 |  |
| cellar_door | 0 | 5 | 5 |  |
| champion_of_the_parish | 0 | 2 | 2 |  |
| chapel_geist | 0 | 1 | 1 |  |
| charmbreaker_devils | 1 | 3 | 4 |  |
| civilized_scholar | 3 | 7 | 10 |  |
| claustrophobia | 1 | 2 | 3 |  |
| clifftop_retreat | 2 | 3 | 5 |  |
| cloistered_youth | 1 | 6 | 7 |  |
| cobbled_wings | 2 | 2 | 4 |  |
| coral_merfolk | 0 | 1 | 1 |  |
| corpse_lunge | 0 | 3 | 3 |  |
| counterspell | 1 | 2 | 3 |  |
| creeping_renaissance | 1 | 2 | 3 |  |
| creepy_doll | 0 | 1 | 1 |  |
| crossway_vampire | 1 | 1 | 2 |  |
| curiosity | 1 | 6 | 7 |  |
| curse_of_deaths_hold | 0 | 1 | 1 |  |
| curse_of_oblivion | 0 | 6 | 6 |  |
| curse_of_stalked_prey | 0 | 3 | 3 |  |
| curse_of_the_bloody_tome | 0 | 5 | 5 |  |
| curse_of_the_nightly_hunt | 0 | 1 | 1 |  |
| curse_of_the_pierced_heart | 0 | 5 | 5 |  |
| darkthicket_wolf | 2 | 1 | 3 |  |
| daybreak_ranger | 3 | 4 | 7 |  |
| dead_weight | 1 | 1 | 2 |  |
| dearly_departed | 1 | 3 | 4 |  |
| delver_of_secrets | 0 | 5 | 5 |  |
| demonmail_hauberk | 2 | 1 | 3 |  |
| deranged_assistant | 0 | 3 | 3 |  |
| desperate_ravings | 0 | 2 | 2 |  |
| devils_play | 1 | 9 | 10 |  |
| diregraf_ghoul | 1 | 1 | 2 |  |
| disciple_of_griselbrand | 1 | 1 | 2 |  |
| dissipate | 1 | 2 | 3 |  |
| divination | 0 | 8 | 8 |  |
| divine_reckoning | 0 | 3 | 3 |  |
| doom_blade | 1 | 1 | 2 |  |
| doomed_traveler | 0 | 1 | 1 |  |
| dream_twist | 0 | 8 | 8 |  |
| elder_cathar | 0 | 2 | 2 |  |
| elder_of_laurels | 2 | 2 | 4 |  |
| elite_inquisitor | 1 | 5 | 6 |  |
| endless_ranks_of_the_dead | 0 | 4 | 4 |  |
| essence_of_the_wild | 1 | 2 | 3 |  |
| evil_twin | 6 | 1 | 7 |  |
| falkenrath_marauders | 0 | 2 | 2 |  |
| falkenrath_noble | 0 | 2 | 2 |  |
| feeling_of_dread | 0 | 3 | 3 |  |
| feral_ridgewolf | 2 | 1 | 3 |  |
| festerhide_boar | 1 | 2 | 3 |  |
| fiend_hunter | 1 | 5 | 6 |  |
| forbidden_alchemy | 0 | 3 | 3 |  |
| forest | 1 | 1 | 2 |  |
| fortress_crab | 0 | 1 | 1 |  |
| frightful_delusion | 2 | 3 | 5 |  |
| full_moons_rise | 2 | 1 | 3 |  |
| furor_of_the_bitten | 1 | 1 | 2 |  |
| gallows_warden | 1 | 1 | 2 |  |
| galvanic_juggernaut | 0 | 2 | 2 |  |
| garruk_relentless | 2 | 8 | 10 |  |
| gatstaf_shepherd | 0 | 2 | 2 |  |
| gavony_township | 2 | 1 | 3 |  |
| geist_honored_monk | 0 | 1 | 1 |  |
| geist_of_saint_traft | 0 | 1 | 1 |  |
| geistcatchers_rig | 1 | 1 | 2 |  |
| geistflame | 1 | 3 | 4 |  |
| ghost_quarter | 2 | 2 | 4 |  |
| ghostly_possession | 1 | 1 | 2 |  |
| ghoulcallers_bell | 1 | 3 | 4 |  |
| ghoulcallers_chant | 0 | 2 | 2 |  |
| ghoulraiser | 1 | 2 | 3 |  |
| giant_growth | 0 | 1 | 1 |  |
| glorious_anthem | 1 | 1 | 2 |  |
| gnaw_to_the_bone | 0 | 3 | 3 |  |
| goblin_piker | 0 | 1 | 1 |  |
| grasp_of_phantoms | 0 | 2 | 2 |  |
| grave_bramble | 1 | 3 | 4 |  |
| graveyard_shovel | 1 | 5 | 6 |  |
| grimgrin_corpse_born | 2 | 3 | 5 |  |
| grimoire_of_the_dead | 6 | 3 | 9 |  |
| grizzled_outcasts | 0 | 2 | 2 |  |
| grizzly_bears | 0 | 1 | 1 |  |
| gruesome_deformity | 1 | 1 | 2 |  |
| gutter_grime | 0 | 2 | 2 |  |
| hamlet_captain | 18 | 4 | 22 |  |
| hanweir_watchkeep | 0 | 4 | 4 |  |
| harvest_pyre | 0 | 2 | 2 |  |
| heartless_summoning | 1 | 5 | 6 |  |
| heretics_punishment | 1 | 1 | 2 |  |
| hinterland_harbor | 2 | 3 | 5 |  |
| hollowhenge_scavenger | 1 | 2 | 3 |  |
| holy_strength | 1 | 1 | 2 |  |
| hysterical_blindness | 1 | 1 | 2 |  |
| infernal_plunge | 0 | 1 | 1 |  |
| inquisitors_flail | 2 | 1 | 3 |  |
| instigator_gang | 0 | 4 | 4 |  |
| intangible_virtue | 1 | 1 | 2 |  |
| into_the_maw_of_hell | 0 | 1 | 1 |  |
| invisible_stalker | 1 | 2 | 3 |  |
| island | 1 | 1 | 2 |  |
| isolated_chapel | 2 | 3 | 5 |  |
| kalonian_tusker | 0 | 1 | 1 |  |
| kessig_cagebreakers | 0 | 1 | 1 |  |
| kessig_wolf | 2 | 1 | 3 |  |
| kessig_wolf_run | 2 | 2 | 4 |  |
| kindercatch | 0 | 1 | 1 |  |
| kruin_outlaw | 0 | 4 | 4 |  |
| laboratory_maniac | 0 | 3 | 3 |  |
| lantern_spirit | 1 | 2 | 3 |  |
| lava_axe | 1 | 2 | 3 |  |
| lightning_bolt | 1 | 3 | 4 |  |
| liliana_of_the_veil | 16 | 7 | 23 |  |
| lost_in_the_mist | 0 | 1 | 1 |  |
| ludevics_test_subject | 3 | 3 | 6 |  |
| lumberknot | 0 | 3 | 3 |  |
| make_a_wish | 0 | 4 | 4 |  |
| makeshift_mauler | 0 | 1 | 1 |  |
| manor_gargoyle | 2 | 1 | 3 |  |
| manor_skeleton | 1 | 2 | 3 |  |
| markov_patrician | 0 | 1 | 1 |  |
| mask_of_avacyn | 2 | 1 | 3 |  |
| mausoleum_guard | 0 | 1 | 1 |  |
| maw_of_the_mire | 0 | 1 | 1 |  |
| mayor_of_avabruck | 0 | 5 | 5 |  |
| memorys_journey | 0 | 1 | 1 |  |
| mentor_of_the_meek | 3 | 4 | 7 |  |
| midnight_haunting | 0 | 1 | 1 |  |
| mikaeus_the_lunarch | 2 | 2 | 4 |  |
| mindshrieker | 1 | 1 | 2 |  |
| mirror_mad_phantasm | 1 | 2 | 3 |  |
| moan_of_the_unhallowed | 0 | 1 | 1 |  |
| moldgraf_monstrosity | 0 | 3 | 3 |  |
| moment_of_heroism | 0 | 1 | 1 |  |
| moon_heron | 0 | 1 | 1 |  |
| moonmist | 0 | 1 | 1 |  |
| moorland_haunt | 9 | 1 | 10 |  |
| morkrut_banshee | 1 | 2 | 3 |  |
| mountain | 1 | 1 | 2 |  |
| mulch | 0 | 3 | 3 |  |
| murder_of_crows | 0 | 4 | 4 |  |
| naturalize | 1 | 2 | 3 |  |
| nephalia_drownyard | 1 | 4 | 5 |  |
| nevermore | 6 | 1 | 7 |  |
| night_revelers | 1 | 2 | 3 |  |
| night_terrors | 0 | 4 | 4 |  |
| nightbirds_clutches | 1 | 3 | 4 |  |
| olivia_voldaren | 1 | 3 | 4 |  |
| one_eyed_scarecrow | 1 | 0 | 1 |  |
| orchard_spirit | 1 | 1 | 2 |  |
| pacifism | 1 | 1 | 2 |  |
| parallel_lives | 1 | 2 | 3 |  |
| paraselene | 0 | 2 | 2 |  |
| past_in_flames | 16 | 1 | 17 |  |
| pitchburn_devils | 0 | 3 | 3 |  |
| plains | 1 | 1 | 2 |  |
| prey_upon | 0 | 1 | 1 |  |
| purify_the_grave | 0 | 2 | 2 |  |
| rage_thrower | 0 | 1 | 1 |  |
| rakish_heir | 0 | 2 | 2 |  |
| rally_the_peasants | 1 | 1 | 2 |  |
| rangers_guile | 0 | 1 | 1 |  |
| reaper_from_the_abyss | 1 | 2 | 3 |  |
| rebuke | 1 | 1 | 2 |  |
| reckless_waif | 0 | 2 | 2 |  |
| riot_devils | 0 | 1 | 1 |  |
| rolling_temblor | 0 | 1 | 1 |  |
| rooftop_storm | 1 | 3 | 4 |  |
| rotting_fensnake | 0 | 1 | 1 |  |
| runechanters_pike | 3 | 1 | 4 |  |
| runic_repetition | 0 | 1 | 1 |  |
| savannah_lions | 0 | 1 | 1 |  |
| scourge_of_geier_reach | 1 | 2 | 3 |  |
| screeching_bat | 1 | 6 | 7 |  |
| selfless_cathar | 2 | 1 | 3 |  |
| selhoff_occultist | 0 | 6 | 6 |  |
| sensory_deprivation | 1 | 1 | 2 |  |
| sever_the_bloodline | 0 | 1 | 1 |  |
| sharpened_pitchfork | 2 | 2 | 4 |  |
| shimmering_grotto | 1 | 1 | 2 |  |
| silent_departure | 0 | 1 | 1 |  |
| silver_inlaid_dagger | 2 | 2 | 4 |  |
| silverchase_fox | 1 | 1 | 2 |  |
| skaab_goliath | 0 | 1 | 1 |  |
| skaab_ruinator | 1 | 1 | 2 |  |
| skeletal_grimace | 1 | 2 | 3 |  |
| skirsdag_cultist | 1 | 3 | 4 |  |
| skirsdag_high_priest | 2 | 1 | 3 |  |
| slayer_of_the_wicked | 1 | 1 | 2 |  |
| smite_the_monstrous | 1 | 1 | 2 |  |
| snapcaster_mage | 2 | 1 | 3 |  |
| sol_ring | 1 | 1 | 2 |  |
| somberwald_spider | 1 | 2 | 3 |  |
| spare_from_evil | 1 | 1 | 2 |  |
| spectral_flight | 1 | 1 | 2 |  |
| spectral_rider | 0 | 1 | 1 |  |
| spider_spawning | 0 | 3 | 3 |  |
| spidery_grasp | 0 | 1 | 1 |  |
| splinterfright | 0 | 4 | 4 |  |
| stensia_bloodhall | 1 | 2 | 3 |  |
| stitched_drake | 0 | 1 | 1 |  |
| stitchers_apprentice | 1 | 2 | 3 |  |
| stony_silence | 1 | 2 | 3 |  |
| stromkirk_noble | 0 | 2 | 2 |  |
| stromkirk_patrol | 0 | 2 | 2 |  |
| sturmgeist | 0 | 6 | 6 |  |
| sulfur_falls | 2 | 3 | 5 |  |
| swamp | 1 | 1 | 2 |  |
| swords_to_plowshares | 0 | 2 | 2 |  |
| think_twice | 0 | 6 | 6 |  |
| thraben_purebloods | 0 | 1 | 1 |  |
| thraben_sentry | 1 | 4 | 5 |  |
| tormented_pariah | 0 | 2 | 2 |  |
| traitorous_blood | 0 | 1 | 1 |  |
| travel_preparations | 0 | 3 | 3 |  |
| travelers_amulet | 1 | 2 | 3 |  |
| tree_of_redemption | 1 | 1 | 2 |  |
| trepanation_blade | 2 | 5 | 7 |  |
| tribute_to_hunger | 0 | 2 | 2 |  |
| typhoid_rats | 0 | 1 | 1 |  |
| ulvenwald_mystics | 0 | 3 | 3 |  |
| unbreathing_horde | 1 | 2 | 3 |  |
| unburial_rites | 0 | 1 | 1 |  |
| undead_alchemist | 0 | 2 | 2 |  |
| unruly_mob | 0 | 2 | 2 |  |
| urgent_exorcism | 1 | 2 | 3 |  |
| vampire_interloper | 1 | 1 | 2 |  |
| vampiric_fury | 8 | 1 | 9 |  |
| victim_of_night | 1 | 1 | 2 |  |
| village_bell_ringer | 1 | 1 | 2 |  |
| village_cannibals | 0 | 2 | 2 |  |
| village_ironsmith | 0 | 2 | 2 |  |
| villagers_of_estwald | 0 | 2 | 2 |  |
| voiceless_spirit | 0 | 1 | 1 |  |
| walking_corpse | 0 | 1 | 1 |  |
| witchbane_orb | 1 | 1 | 2 |  |
| wooden_stake | 2 | 3 | 5 |  |
| woodland_cemetery | 2 | 3 | 5 |  |
| woodland_sleuth | 1 | 3 | 4 |  |
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

army_of_the_damned

