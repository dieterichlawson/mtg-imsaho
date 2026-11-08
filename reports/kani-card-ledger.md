# Card proofs at 782c4ed

Every card harness, in one pass over every harness the proofs define,
three lanes at 3.5 GB each. The pass began at 156d7ad and moved to
782c4ed after 896 harnesses, 84 of them card harnesses: card proofs at
156d7ad were running a third slower than in the run before (final7, the
same steps: the machine, not the code), and the rest of the pass is the
code being merged. Between the two the engine changed in what a
trigger's log line names and when, in combat's first-strike check, a
modal spell's targets and the declared blocks' log line (each written
without `flatten`), in the trigger order answer and in the autotap
planner. The 84 are all `cards::activated`'s, activated abilities' own
code with paying stood in for, and reach none of those. Mentor of the Meek's
two may-pay harnesses are at 321bcd4, where their proof was brought up to
#940 (asked only when a payment can be planned).

Not proved below means out of memory at 3.5 GB. Each of the 96 such card
harnesses proved alone, at 7 or 12 GB, in the run before; they are being
run alone again at 321bcd4, and this ledger is refreshed from that.
`docs/plans/formal-verification.md` says what each proof is over and what
its bounds are.

989 harnesses over 248 cards and 4 tables of cards; 22 cards are their printed data alone.

- OOM: 96
- SUCCESSFUL: 893

| card | harnesses | proved | not proved |
|---|---|---|---|
| abattoir_ghoul | 1 | 1 |  |
| altars_reap | 1 | 1 |  |
| ancient_grudge | 2 | 2 |  |
| angel_of_flight_alabaster | 1 | 1 |  |
| angelic_overseer | 1 | 1 |  |
| armored_skaab | 2 | 2 |  |
| army_of_the_damned | 1 | 1 |  |
| ashmouth_hound | 1 | 1 |  |
| avacynian_priest | 3 | 2 | `avacynian_priest_targets_a_non_human_creature` (OOM) |
| avacyns_pilgrim | 1 | 1 |  |
| back_from_the_brink | 9 | 9 |  |
| balefire_dragon | 1 | 0 | `balefire_dragon_deals_that_much_to_each_of_their_creatures` (OOM) |
| battleground_geist | 1 | 1 |  |
| bitterheart_witch | 12 | 11 | `bitterheart_witch_attaches_the_curse_to_its_target` (OOM) |
| blasphemous_act | 1 | 0 | `blasphemous_act_costs_less_and_deals_thirteen_to_each_creature` (OOM) |
| blazing_torch | 3 | 2 | `blazing_torch_gives_its_creature_a_sacrifice_to_deal_two` (OOM) |
| bloodcrazed_neonate | 1 | 1 |  |
| bloodgift_demon | 1 | 1 |  |
| bloodline_keeper | 10 | 10 |  |
| bonds_of_faith | 2 | 2 |  |
| boneyard_wurm | 1 | 1 |  |
| brain_weevil | 2 | 2 |  |
| bramblecrush | 2 | 2 |  |
| brimstone_volley | 2 | 2 |  |
| bump_in_the_night | 1 | 1 |  |
| burning_vengeance | 1 | 1 |  |
| butchers_cleaver | 2 | 2 |  |
| cackling_counterpart | 1 | 1 |  |
| caravan_vigil | 40 | 39 | `caravan_vigil_of_the_second_player_with_morbid_puts_the_land_into_hand_on_no` (OOM) |
| cellar_door | 10 | 10 |  |
| champion_of_the_parish | 1 | 1 |  |
| charmbreaker_devils | 2 | 1 | `charmbreaker_devils_returns_an_instant_or_sorcery_at_random` (OOM) |
| civilized_scholar | 8 | 8 |  |
| claustrophobia | 3 | 3 |  |
| clifftop_retreat | 2 | 2 |  |
| cloistered_youth | 5 | 5 |  |
| cobbled_wings | 2 | 2 |  |
| corpse_lunge | 1 | 1 |  |
| counterspell | 2 | 2 |  |
| creeping_renaissance | 3 | 1 | `creeping_renaissance_returns_every_creature_card` (OOM), `creeping_renaissance_returns_every_land_card` (OOM) |
| creepy_doll | 1 | 1 |  |
| crossway_vampire | 2 | 2 |  |
| curiosity | 3 | 3 |  |
| curse_of_deaths_hold | 1 | 1 |  |
| curse_of_oblivion | 25 | 25 |  |
| curse_of_stalked_prey | 2 | 2 |  |
| curse_of_the_bloody_tome | 2 | 2 |  |
| curse_of_the_nightly_hunt | 1 | 1 |  |
| curse_of_the_pierced_heart | 2 | 1 | `curse_of_the_pierced_heart_deals_one_damage_to_its_player_or_their_planeswalker` (OOM) |
| darkthicket_wolf | 2 | 2 |  |
| daybreak_ranger | 5 | 5 |  |
| dead_weight | 2 | 2 |  |
| dearly_departed | 1 | 1 |  |
| delver_of_secrets | 8 | 5 | `delver_of_secrets_of_the_second_player_over_lightning_bolt` (OOM), `delver_of_secrets_of_the_second_player_over_lava_axe` (OOM), `delver_of_secrets_of_the_second_player_over_a_blank_card` (OOM) |
| demonmail_hauberk | 2 | 2 |  |
| deranged_assistant | 1 | 1 |  |
| desperate_ravings | 1 | 1 |  |
| devils_play | 2 | 2 |  |
| diregraf_ghoul | 1 | 1 |  |
| disciple_of_griselbrand | 2 | 2 |  |
| dissipate | 2 | 2 |  |
| divination | 1 | 1 |  |
| divine_reckoning | 54 | 54 |  |
| doom_blade | 3 | 3 |  |
| doomed_traveler | 1 | 1 |  |
| dream_twist | 1 | 1 |  |
| elder_cathar | 1 | 1 |  |
| elder_of_laurels | 2 | 2 |  |
| elite_inquisitor | 1 | 1 |  |
| endless_ranks_of_the_dead | 1 | 1 |  |
| essence_of_the_wild | 1 | 1 |  |
| evil_twin | 7 | 7 |  |
| falkenrath_marauders | 1 | 1 |  |
| falkenrath_noble | 1 | 1 |  |
| feeling_of_dread | 5 | 5 |  |
| feral_ridgewolf | 2 | 2 |  |
| festerhide_boar | 1 | 1 |  |
| fiend_hunter | 3 | 3 |  |
| forbidden_alchemy | 16 | 4 | `forbidden_alchemy_over_two_keeping_the_second` (OOM), `forbidden_alchemy_over_three_keeping_the_first` (OOM), `forbidden_alchemy_over_three_keeping_the_second` (OOM), `forbidden_alchemy_over_four_keeping_the_first` (OOM), `forbidden_alchemy_over_four_keeping_the_second` (OOM), `forbidden_alchemy_over_four_keeping_the_third` (OOM), `forbidden_alchemy_over_four_keeping_the_fourth` (OOM), `forbidden_alchemy_over_five_keeping_the_first` (OOM), `forbidden_alchemy_over_five_keeping_the_second` (OOM), `forbidden_alchemy_over_five_keeping_the_third` (OOM), `forbidden_alchemy_over_five_keeping_the_fourth` (OOM), `forbidden_alchemy_over_five_answering_the_fifth_not_looked_at` (OOM) |
| forest | 1 | 1 |  |
| frightful_delusion | 14 | 12 | `frightful_delusion_against_the_first_players_spell_paid_for_with_two_cards_in_hand` (OOM), `frightful_delusion_against_the_second_players_spell_paid_for_with_two_cards_in_hand` (OOM) |
| full_moons_rise | 10 | 10 |  |
| furor_of_the_bitten | 2 | 2 |  |
| gallows_warden | 1 | 1 |  |
| galvanic_juggernaut | 1 | 1 |  |
| garruk_relentless | 30 | 25 | `garruk_relentless_fights_his_target` (OOM), `garruk_the_veil_cursed_sacrifices_then_searches_for_the_first_player_over_their_creature_card_and_their_creature_token` (OOM), `garruk_the_veil_cursed_sacrifices_then_searches_for_the_first_player_over_a_card_in_a_graveyard_and_their_creature_token` (OOM), `garruk_the_veil_cursed_sacrifices_then_searches_for_the_second_player_over_their_creature_card_and_their_creature_token` (OOM), `garruk_the_veil_cursed_sacrifices_then_searches_for_the_second_player_over_a_card_in_a_graveyard_and_their_creature_token` (OOM) |
| gatstaf_shepherd | 1 | 1 |  |
| gavony_township | 3 | 3 |  |
| geist_honored_monk | 1 | 0 | `geist_honored_monk_counts_your_creatures_and_brings_two_spirits` (OOM) |
| geist_of_saint_traft | 1 | 1 |  |
| geistcatchers_rig | 2 | 2 |  |
| geistflame | 2 | 2 |  |
| ghost_quarter | 13 | 9 | `ghost_quarter_destroys_the_first_players_card_and_offers_the_forest_on_top` (OOM), `ghost_quarter_destroys_the_first_players_card_and_offers_the_forest_under_a_land` (OOM), `ghost_quarter_destroys_the_second_players_card_and_offers_the_forest_on_top` (OOM), `ghost_quarter_destroys_the_second_players_card_and_offers_the_forest_under_a_land` (OOM) |
| ghostly_possession | 2 | 2 |  |
| ghoulcallers_bell | 2 | 2 |  |
| ghoulcallers_chant | 1 | 0 | `ghoulcallers_chant_returns_its_targets_to_hand` (OOM) |
| ghoulraiser | 2 | 1 | `ghoulraiser_returns_a_zombie_card_at_random` (OOM) |
| giant_growth | 1 | 1 |  |
| glorious_anthem | 1 | 1 |  |
| gnaw_to_the_bone | 1 | 1 |  |
| grasp_of_phantoms | 1 | 1 |  |
| grave_bramble | 1 | 1 |  |
| graveyard_shovel | 2 | 1 | `graveyard_shovel_exiles_a_card_and_gains_life_for_a_creature` (OOM) |
| grimgrin_corpse_born | 4 | 3 | `grimgrin_destroys_a_defending_creature_and_grows` (OOM) |
| grimoire_of_the_dead | 21 | 19 | `grimoire_of_the_dead_has_its_abilities_with_an_empty_hand_for_the_first_player` (OOM), `grimoire_of_the_dead_has_its_abilities_with_an_empty_hand_for_the_second_player` (OOM) |
| grizzled_outcasts | 1 | 1 |  |
| gruesome_deformity | 2 | 2 |  |
| gutter_grime | 1 | 1 |  |
| hamlet_captain | 18 | 18 |  |
| hanweir_watchkeep | 1 | 1 |  |
| harvest_pyre | 1 | 1 |  |
| heartless_summoning | 1 | 1 |  |
| heretics_punishment | 27 | 27 |  |
| hinterland_harbor | 2 | 2 |  |
| hollowhenge_scavenger | 2 | 2 |  |
| holy_strength | 2 | 2 |  |
| hysterical_blindness | 1 | 1 |  |
| infernal_plunge | 1 | 1 |  |
| inquisitors_flail | 2 | 2 |  |
| instigator_gang | 1 | 1 |  |
| intangible_virtue | 1 | 1 |  |
| into_the_maw_of_hell | 1 | 1 |  |
| invisible_stalker | 1 | 1 |  |
| island | 1 | 1 |  |
| isolated_chapel | 2 | 2 |  |
| kessig_cagebreakers | 1 | 0 | `kessig_cagebreakers_brings_a_wolf_for_each_creature_card_in_your_graveyard` (OOM) |
| kessig_wolf | 2 | 2 |  |
| kessig_wolf_run | 2 | 2 |  |
| kruin_outlaw | 1 | 1 |  |
| laboratory_maniac | 1 | 1 |  |
| lantern_spirit | 2 | 2 |  |
| lava_axe | 2 | 2 |  |
| lightning_bolt | 2 | 2 |  |
| liliana_of_the_veil | 116 | 113 | `liliana_of_the_veil_makes_each_player_discard_with_the_first_player_active_over_hands_of_two_and_two_neither_lost` (OOM), `liliana_of_the_veil_makes_the_first_player_sacrifice_a_creature` (OOM), `liliana_of_the_veil_makes_the_second_player_sacrifice_a_creature` (OOM) |
| lost_in_the_mist | 1 | 1 |  |
| ludevics_test_subject | 4 | 4 |  |
| lumberknot | 1 | 1 |  |
| make_a_wish | 64 | 64 |  |
| manor_gargoyle | 2 | 2 |  |
| manor_skeleton | 2 | 2 |  |
| mask_of_avacyn | 2 | 2 |  |
| mausoleum_guard | 1 | 1 |  |
| maw_of_the_mire | 1 | 1 |  |
| mayor_of_avabruck | 2 | 2 |  |
| memorys_journey | 1 | 0 | `memorys_journey_shuffles_its_targets_into_their_library` (OOM) |
| mentor_of_the_meek | 5 | 5 |  |
| midnight_haunting | 1 | 1 |  |
| mikaeus_the_lunarch | 3 | 2 | `mikaeus_the_lunarch_puts_counters_on_itself_or_on_your_other_creatures` (OOM) |
| mindshrieker | 5 | 5 |  |
| mirror_mad_phantasm | 2 | 1 | `mirror_mad_phantasm_reveals_until_a_phantasm_card` (OOM) |
| moan_of_the_unhallowed | 1 | 1 |  |
| moldgraf_monstrosity | 2 | 0 | `moldgraf_monstrosity_in_its_graveyard_exiles_itself_and_returns_two_at_random` (OOM), `moldgraf_monstrosity_gone_to_exile_returns_two_at_random` (OOM) |
| moment_of_heroism | 1 | 1 |  |
| moonmist | 16 | 5 | `moonmist_over_a_card_neither_human_nor_wolf_and_a_human_wolf_token` (OOM), `moonmist_over_a_human_card_and_a_token_neither_human_nor_wolf` (OOM), `moonmist_over_a_human_card_and_a_human_token` (OOM), `moonmist_over_a_human_card_and_a_wolf_token` (OOM), `moonmist_over_a_human_card_and_a_human_wolf_token` (OOM), `moonmist_over_a_wolf_card_and_a_human_token` (OOM), `moonmist_over_a_wolf_card_and_a_human_wolf_token` (OOM), `moonmist_over_a_human_wolf_card_and_a_token_neither_human_nor_wolf` (OOM), `moonmist_over_a_human_wolf_card_and_a_human_token` (OOM), `moonmist_over_a_human_wolf_card_and_a_wolf_token` (OOM), `moonmist_over_a_human_wolf_card_and_a_human_wolf_token` (OOM) |
| moorland_haunt | 17 | 9 | `moorland_haunt_of_the_first_player_with_its_creature_card_exiles_a_creature_card_and_makes_a_spirit` (OOM), `moorland_haunt_of_the_first_player_with_a_noncreature_card_exiles_a_creature_card_and_makes_a_spirit` (OOM), `moorland_haunt_of_the_first_player_with_the_others_creature_card_exiles_a_creature_card_and_makes_a_spirit` (OOM), `moorland_haunt_of_the_first_player_with_the_others_noncreature_card_exiles_a_creature_card_and_makes_a_spirit` (OOM), `moorland_haunt_of_the_second_player_with_the_others_creature_card_exiles_a_creature_card_and_makes_a_spirit` (OOM), `moorland_haunt_of_the_second_player_with_the_others_noncreature_card_exiles_a_creature_card_and_makes_a_spirit` (OOM), `moorland_haunt_of_the_second_player_with_its_creature_card_exiles_a_creature_card_and_makes_a_spirit` (OOM), `moorland_haunt_of_the_second_player_with_a_noncreature_card_exiles_a_creature_card_and_makes_a_spirit` (OOM) |
| morkrut_banshee | 2 | 2 |  |
| mountain | 1 | 1 |  |
| mulch | 6 | 1 | `mulch_over_a_library_of_one` (OOM), `mulch_over_a_library_of_two` (OOM), `mulch_over_a_library_of_three` (OOM), `mulch_over_a_library_of_four` (OOM), `mulch_over_a_library_of_five` (OOM) |
| murder_of_crows | 8 | 8 |  |
| naturalize | 2 | 2 |  |
| nephalia_drownyard | 2 | 2 |  |
| nevermore | 6 | 6 |  |
| night_revelers | 1 | 1 |  |
| night_terrors | 7 | 6 | `night_terrors_over_grizzly_bears_and_grizzly_bears` (OOM) |
| nightbirds_clutches | 1 | 1 |  |
| olivia_voldaren | 3 | 3 |  |
| one_eyed_scarecrow | 1 | 1 |  |
| orchard_spirit | 1 | 1 |  |
| pacifism | 2 | 2 |  |
| parallel_lives | 1 | 1 |  |
| paraselene | 1 | 0 | `paraselene_destroys_all_enchantments_and_gains_a_life_for_each` (OOM) |
| past_in_flames | 16 | 16 |  |
| pitchburn_devils | 1 | 1 |  |
| plains | 1 | 1 |  |
| prey_upon | 1 | 1 |  |
| purify_the_grave | 1 | 1 |  |
| rage_thrower | 1 | 1 |  |
| rakish_heir | 1 | 1 |  |
| rally_the_peasants | 1 | 1 |  |
| rangers_guile | 1 | 1 |  |
| reaper_from_the_abyss | 2 | 1 | `reaper_from_the_abyss_triggers_after_a_death_targeting_a_non_demon` (OOM) |
| rebuke | 2 | 2 |  |
| reckless_waif | 1 | 1 |  |
| rolling_temblor | 1 | 0 | `rolling_temblor_deals_two_to_each_creature_without_flying` (OOM) |
| rooftop_storm | 1 | 1 |  |
| runechanters_pike | 3 | 3 |  |
| runic_repetition | 1 | 1 |  |
| scourge_of_geier_reach | 1 | 1 |  |
| screeching_bat | 6 | 6 |  |
| selfless_cathar | 2 | 2 |  |
| selhoff_occultist | 1 | 1 |  |
| sensory_deprivation | 2 | 2 |  |
| sever_the_bloodline | 6 | 2 | `sever_the_bloodline_targeting_the_card_with_the_token_of_its_name` (OOM), `sever_the_bloodline_targeting_the_card_with_the_token_of_another_name` (OOM), `sever_the_bloodline_targeting_the_token_of_the_cards_name` (OOM), `sever_the_bloodline_targeting_the_token_of_another_name` (OOM) |
| sharpened_pitchfork | 2 | 2 |  |
| shimmering_grotto | 1 | 1 |  |
| silent_departure | 1 | 1 |  |
| silver_inlaid_dagger | 2 | 2 |  |
| silverchase_fox | 2 | 2 |  |
| skaab_ruinator | 1 | 1 |  |
| skeletal_grimace | 3 | 3 |  |
| skirsdag_cultist | 2 | 2 |  |
| skirsdag_high_priest | 3 | 3 |  |
| slayer_of_the_wicked | 2 | 2 |  |
| smite_the_monstrous | 2 | 2 |  |
| snapcaster_mage | 2 | 1 | `snapcaster_mage_gives_its_target_flashback_at_its_mana_cost` (OOM) |
| sol_ring | 1 | 1 |  |
| somberwald_spider | 1 | 1 |  |
| spare_from_evil | 1 | 1 |  |
| spectral_flight | 2 | 2 |  |
| spider_spawning | 1 | 1 |  |
| spidery_grasp | 1 | 1 |  |
| splinterfright | 1 | 1 |  |
| stensia_bloodhall | 2 | 1 | `stensia_bloodhall_deals_two_to_its_target` (OOM) |
| stitchers_apprentice | 5 | 1 | `stitchers_apprentice_of_the_first_player_makes_a_homunculus_then_sacrifices_a_creature` (OOM), `stitchers_apprentice_of_the_second_player_makes_a_homunculus_then_sacrifices_a_creature` (OOM), `stitchers_apprentice_gone_makes_the_first_player_a_homunculus_then_sacrifices_a_creature` (OOM), `stitchers_apprentice_gone_makes_the_second_player_a_homunculus_then_sacrifices_a_creature` (OOM) |
| stony_silence | 1 | 1 |  |
| stromkirk_noble | 1 | 1 |  |
| stromkirk_patrol | 1 | 1 |  |
| sturmgeist | 1 | 1 |  |
| sulfur_falls | 2 | 2 |  |
| swamp | 1 | 1 |  |
| swords_to_plowshares | 1 | 1 |  |
| think_twice | 1 | 1 |  |
| thraben_sentry | 5 | 5 |  |
| tormented_pariah | 1 | 1 |  |
| traitorous_blood | 1 | 1 |  |
| travel_preparations | 5 | 5 |  |
| travelers_amulet | 5 | 5 |  |
| tree_of_redemption | 2 | 2 |  |
| trepanation_blade | 10 | 10 |  |
| tribute_to_hunger | 3 | 0 | `tribute_to_hunger_makes_them_choose_a_creature_to_sacrifice` (OOM), `tribute_to_hunger_sacrifices_the_card_they_chose_and_you_gain_its_toughness` (OOM), `tribute_to_hunger_sacrifices_the_token_they_chose_and_you_gain_its_toughness` (OOM) |
| ulvenwald_mystics | 4 | 4 |  |
| unbreathing_horde | 1 | 1 |  |
| unburial_rites | 1 | 1 |  |
| undead_alchemist | 2 | 2 |  |
| unruly_mob | 1 | 1 |  |
| urgent_exorcism | 2 | 2 |  |
| vampire_interloper | 1 | 1 |  |
| vampiric_fury | 8 | 8 |  |
| victim_of_night | 2 | 2 |  |
| village_bell_ringer | 2 | 1 | `village_bell_ringer_untaps_your_creatures` (OOM) |
| village_cannibals | 1 | 1 |  |
| village_ironsmith | 1 | 1 |  |
| villagers_of_estwald | 1 | 1 |  |
| witchbane_orb | 2 | 1 | `witchbane_orb_destroys_the_curses_on_you` (OOM) |
| wooden_stake | 3 | 3 |  |
| woodland_cemetery | 2 | 2 |  |
| woodland_sleuth | 2 | 1 | `woodland_sleuth_returns_a_creature_card_at_random_after_a_death` (OOM) |
| wreath_of_geists | 3 | 3 |  |

## Tables of cards

| harness | result | time |
|---|---|---|
| `activated::targeted_abilities_accept_what_their_text_targets` | SUCCESSFUL | 105s |
| `curses::the_static_curses_print_what_their_text_says` | SUCCESSFUL | 10s |
| `curses::the_upkeep_curses_trigger_in_the_enchanted_players_upkeep` | SUCCESSFUL | 33s |
| `targets::the_two_target_spells_accept_what_their_text_targets` | SUCCESSFUL | 31s |

## Cards that are their printed data alone

abbey_griffin, ambush_viper, chapel_geist, coral_merfolk, fortress_crab, goblin_piker, grizzly_bears, kalonian_tusker, kindercatch, makeshift_mauler, markov_patrician, moon_heron, riot_devils, rotting_fensnake, savannah_lions, skaab_goliath, spectral_rider, stitched_drake, thraben_purebloods, typhoid_rats, voiceless_spirit, walking_corpse

