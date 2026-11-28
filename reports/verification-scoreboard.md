# Verification scoreboard

Counted by `scripts/verification-scoreboard.py`; the goals are in
`docs/plans/verification-goals.md`.

| Goal | Now | Done at |
|---|---|---|
| 1. cards with every proof written for them through the engine's cast, stack and resolution proved | 0 of 270 (and 219 with some not yet proved) | 270 |
| 1. those proofs: proved of written | 0 of 1434 | all |
| 1. harnesses that call a card's hook directly | 604 | 0 |
| A. lines the cards print (each face's frame and each line of its text) that a proof through the game claims | 595 of 998 (0 with every such proof proved) | 998, proved |
| B. rules the cards lean on (the keywords, combat, the turn, summoning sickness) with every row written for them through the game proved | 0 of 17 (17 with rows, 48 rows) | 17 |
| 2. cards whose proofs cover every case that applies | not counted yet | all |
| 3. changes to a card's code its proofs catch | not counted yet | every one |
| 4. engine functions a proof stands in for, unchecked | 165 | 0 |
| F. lines of the engine's source that are another under `cfg(kani)`, the build the proofs are of | 39 in 10 files | 0, or each proved to change nothing |
| 5. official rulings of the pool's cards that a proof through the game accounts for | 0 of 448 (0 proved); the Oracle cache has 578, 128 of them of cards not in the pool and 2 twice | 448, proved |
| 6. proofs that do not finish | 12 (and 1482 written and not yet run) | 0 |
| 7. permanents that can be pieces of the small boards | 129 of 199 (counted by hand, 2026-11-27) | 199 |

## Cards through the stack

| card | through the stack, proved of written | by its hook alone | declared |
|---|---|---|---|
| abattoir_ghoul | 0 of 5 | 2 | 0 |
| abbey_griffin | 0 of 5 | 1 | 0 |
| altars_reap | 0 of 7 | 8 | 0 |
| ambush_viper | 0 of 5 | 1 | 0 |
| ancient_grudge | 0 of 13 | 1 | 1 |
| angel_of_flight_alabaster | 0 of 5 | 1 | 0 |
| angelic_overseer | 0 of 5 | 2 | 1 |
| armored_skaab | - | 6 | 1 |
| army_of_the_damned | - | 1 | 0 |
| ashmouth_hound | 0 of 5 | 2 | 0 |
| avacynian_priest | 0 of 5 | 1 | 2 |
| avacyns_pilgrim | 0 of 9 | 1 | 1 |
| back_from_the_brink | 0 of 5 | 2 | 8 |
| balefire_dragon | 0 of 5 | 2 | 0 |
| battleground_geist | 0 of 5 | 1 | 1 |
| bitterheart_witch | 0 of 5 | 2 | 2 |
| blasphemous_act | 0 of 7 | 2 | 0 |
| blazing_torch | 0 of 10 | 3 | 1 |
| bloodcrazed_neonate | 0 of 5 | 2 | 0 |
| bloodgift_demon | 0 of 5 | 4 | 0 |
| bloodline_keeper | 0 of 5 | 2 | 8 |
| bonds_of_faith | 0 of 6 | 2 | 1 |
| boneyard_wurm | - | 2 | 1 |
| brain_weevil | 0 of 5 | 4 | 1 |
| bramblecrush | 0 of 12 | 4 | 1 |
| brimstone_volley | 0 of 15 | 6 | 1 |
| bump_in_the_night | 0 of 12 | 1 | 0 |
| burning_vengeance | 0 of 5 | 3 | 0 |
| butchers_cleaver | 0 of 10 | 2 | 2 |
| cackling_counterpart | - | 2 | 0 |
| caravan_vigil | - | 4 | 0 |
| cellar_door | 0 of 5 | 5 | 0 |
| champion_of_the_parish | - | 2 | 0 |
| chapel_geist | 0 of 5 | 1 | 0 |
| charmbreaker_devils | 0 of 5 | 3 | 1 |
| civilized_scholar | 0 of 5 | 7 | 3 |
| claustrophobia | - | 2 | 1 |
| clifftop_retreat | - | 3 | 2 |
| cloistered_youth | 0 of 5 | 6 | 1 |
| cobbled_wings | 0 of 10 | 2 | 2 |
| coral_merfolk | 0 of 5 | 1 | 0 |
| corpse_lunge | - | 3 | 0 |
| counterspell | 0 of 7 | 2 | 1 |
| creeping_renaissance | - | 2 | 1 |
| creepy_doll | 0 of 5 | 1 | 0 |
| crossway_vampire | - | 1 | 1 |
| curiosity | 0 of 6 | 6 | 1 |
| curse_of_deaths_hold | 0 of 6 | 1 | 0 |
| curse_of_oblivion | 0 of 6 | 6 | 0 |
| curse_of_stalked_prey | 0 of 6 | 3 | 0 |
| curse_of_the_bloody_tome | 0 of 6 | 5 | 0 |
| curse_of_the_nightly_hunt | 0 of 6 | 1 | 0 |
| curse_of_the_pierced_heart | 0 of 6 | 5 | 0 |
| darkthicket_wolf | 0 of 5 | 1 | 2 |
| daybreak_ranger | 0 of 5 | 4 | 3 |
| dead_weight | 0 of 6 | 1 | 1 |
| dearly_departed | - | 3 | 1 |
| delver_of_secrets | 0 of 5 | 5 | 0 |
| demonmail_hauberk | 0 of 5 | 1 | 2 |
| deranged_assistant | 0 of 5 | 3 | 0 |
| desperate_ravings | - | 2 | 0 |
| devils_play | - | 9 | 1 |
| diregraf_ghoul | - | 1 | 1 |
| disciple_of_griselbrand | 0 of 5 | 1 | 1 |
| dissipate | 0 of 5 | 2 | 1 |
| divination | 0 of 8 | 8 | 0 |
| divine_reckoning | - | 3 | 0 |
| doom_blade | 0 of 15 | 1 | 1 |
| doomed_traveler | 0 of 5 | 1 | 0 |
| dream_twist | 0 of 13 | 8 | 0 |
| elder_cathar | 0 of 5 | 2 | 0 |
| elder_of_laurels | 0 of 5 | 2 | 2 |
| elite_inquisitor | 0 of 5 | 5 | 1 |
| endless_ranks_of_the_dead | 0 of 5 | 4 | 0 |
| essence_of_the_wild | 0 of 5 | 2 | 1 |
| evil_twin | 0 of 5 | 1 | 6 |
| falkenrath_marauders | 0 of 5 | 2 | 0 |
| falkenrath_noble | 0 of 5 | 2 | 0 |
| feeling_of_dread | 0 of 12 | 3 | 0 |
| feral_ridgewolf | 0 of 5 | 1 | 2 |
| festerhide_boar | - | 2 | 1 |
| fiend_hunter | - | 5 | 1 |
| forbidden_alchemy | - | 3 | 0 |
| forest | 0 of 8 | 1 | 1 |
| fortress_crab | 0 of 5 | 1 | 0 |
| frightful_delusion | - | 3 | 2 |
| full_moons_rise | 0 of 5 | 1 | 2 |
| furor_of_the_bitten | 0 of 6 | 1 | 1 |
| gallows_warden | 0 of 5 | 1 | 1 |
| galvanic_juggernaut | 0 of 5 | 2 | 0 |
| garruk_relentless | 0 of 5 | 8 | 2 |
| gatstaf_shepherd | 0 of 5 | 2 | 0 |
| gavony_township | 0 of 8 | 1 | 2 |
| geist_honored_monk | - | 1 | 0 |
| geist_of_saint_traft | 0 of 5 | 1 | 0 |
| geistcatchers_rig | - | 1 | 1 |
| geistflame | 0 of 18 | 3 | 1 |
| ghost_quarter | 0 of 8 | 2 | 2 |
| ghostly_possession | 0 of 6 | 1 | 1 |
| ghoulcallers_bell | 0 of 5 | 3 | 1 |
| ghoulcallers_chant | - | 2 | 0 |
| ghoulraiser | - | 2 | 1 |
| giant_growth | 0 of 8 | 1 | 0 |
| glorious_anthem | 0 of 5 | 1 | 1 |
| gnaw_to_the_bone | 0 of 11 | 3 | 0 |
| goblin_piker | 0 of 5 | 1 | 0 |
| grasp_of_phantoms | 0 of 13 | 2 | 0 |
| grave_bramble | 0 of 5 | 3 | 1 |
| graveyard_shovel | 0 of 5 | 5 | 1 |
| grimgrin_corpse_born | - | 3 | 2 |
| grimoire_of_the_dead | 0 of 5 | 3 | 6 |
| grizzled_outcasts | 0 of 5 | 2 | 0 |
| grizzly_bears | 0 of 5 | 1 | 0 |
| gruesome_deformity | 0 of 6 | 1 | 1 |
| gutter_grime | 0 of 5 | 2 | 0 |
| hamlet_captain | 0 of 5 | 4 | 18 |
| hanweir_watchkeep | 0 of 5 | 4 | 0 |
| harvest_pyre | - | 2 | 0 |
| heartless_summoning | 0 of 5 | 5 | 1 |
| heretics_punishment | 0 of 5 | 1 | 1 |
| hinterland_harbor | - | 3 | 2 |
| hollowhenge_scavenger | - | 2 | 1 |
| holy_strength | 0 of 6 | 1 | 1 |
| hysterical_blindness | 0 of 5 | 1 | 1 |
| infernal_plunge | 0 of 7 | 1 | 0 |
| inquisitors_flail | 0 of 10 | 1 | 2 |
| instigator_gang | 0 of 5 | 4 | 0 |
| intangible_virtue | 0 of 5 | 1 | 1 |
| into_the_maw_of_hell | 0 of 6 | 1 | 0 |
| invisible_stalker | 0 of 5 | 2 | 1 |
| island | 0 of 8 | 1 | 1 |
| isolated_chapel | - | 3 | 2 |
| kalonian_tusker | 0 of 5 | 1 | 0 |
| kessig_cagebreakers | 0 of 5 | 1 | 0 |
| kessig_wolf | 0 of 5 | 1 | 2 |
| kessig_wolf_run | 0 of 8 | 2 | 2 |
| kindercatch | 0 of 5 | 1 | 0 |
| kruin_outlaw | 0 of 5 | 4 | 0 |
| laboratory_maniac | 0 of 5 | 3 | 0 |
| lantern_spirit | 0 of 5 | 2 | 1 |
| lava_axe | 0 of 11 | 2 | 1 |
| lightning_bolt | 0 of 15 | 3 | 1 |
| liliana_of_the_veil | 0 of 5 | 7 | 16 |
| lost_in_the_mist | 0 of 3 | 1 | 0 |
| ludevics_test_subject | 0 of 5 | 3 | 3 |
| lumberknot | 0 of 5 | 3 | 0 |
| make_a_wish | - | 4 | 0 |
| makeshift_mauler | 0 of 5 | 1 | 0 |
| manor_gargoyle | 0 of 5 | 1 | 2 |
| manor_skeleton | 0 of 5 | 2 | 1 |
| markov_patrician | 0 of 5 | 1 | 0 |
| mask_of_avacyn | 0 of 10 | 1 | 2 |
| mausoleum_guard | 0 of 5 | 1 | 0 |
| maw_of_the_mire | 0 of 8 | 1 | 0 |
| mayor_of_avabruck | 0 of 5 | 5 | 0 |
| memorys_journey | - | 1 | 0 |
| mentor_of_the_meek | - | 4 | 3 |
| midnight_haunting | 0 of 5 | 1 | 0 |
| mikaeus_the_lunarch | - | 2 | 2 |
| mindshrieker | 0 of 5 | 1 | 1 |
| mirror_mad_phantasm | 0 of 5 | 2 | 1 |
| moan_of_the_unhallowed | 0 of 9 | 1 | 0 |
| moldgraf_monstrosity | 0 of 5 | 3 | 0 |
| moment_of_heroism | 0 of 8 | 1 | 0 |
| moon_heron | 0 of 5 | 1 | 0 |
| moonmist | - | 1 | 0 |
| moorland_haunt | 0 of 8 | 1 | 9 |
| morkrut_banshee | - | 2 | 1 |
| mountain | 0 of 8 | 1 | 1 |
| mulch | - | 3 | 0 |
| murder_of_crows | 0 of 5 | 4 | 0 |
| naturalize | 0 of 12 | 2 | 1 |
| nephalia_drownyard | 0 of 8 | 4 | 1 |
| nevermore | - | 1 | 6 |
| night_revelers | 0 of 5 | 2 | 1 |
| night_terrors | - | 4 | 0 |
| nightbirds_clutches | 0 of 12 | 3 | 1 |
| olivia_voldaren | 0 of 5 | 3 | 1 |
| one_eyed_scarecrow | 0 of 5 | 0 | 1 |
| orchard_spirit | 0 of 5 | 1 | 1 |
| pacifism | 0 of 6 | 1 | 1 |
| parallel_lives | 0 of 5 | 2 | 1 |
| paraselene | 0 of 6 | 2 | 0 |
| past_in_flames | - | 1 | 16 |
| pitchburn_devils | 0 of 5 | 3 | 0 |
| plains | 0 of 8 | 1 | 1 |
| prey_upon | 0 of 9 | 1 | 0 |
| purify_the_grave | 0 of 12 | 2 | 0 |
| rage_thrower | 0 of 5 | 1 | 0 |
| rakish_heir | 0 of 5 | 2 | 0 |
| rally_the_peasants | 0 of 9 | 1 | 1 |
| rangers_guile | 0 of 8 | 1 | 0 |
| reaper_from_the_abyss | 0 of 5 | 2 | 1 |
| rebuke | 0 of 7 | 1 | 1 |
| reckless_waif | 0 of 5 | 2 | 0 |
| riot_devils | 0 of 5 | 1 | 0 |
| rolling_temblor | 0 of 10 | 1 | 0 |
| rooftop_storm | 0 of 5 | 3 | 1 |
| rotting_fensnake | 0 of 5 | 1 | 0 |
| runechanters_pike | 0 of 10 | 1 | 3 |
| runic_repetition | 0 of 8 | 1 | 0 |
| savannah_lions | 0 of 5 | 1 | 0 |
| scourge_of_geier_reach | 0 of 5 | 2 | 1 |
| screeching_bat | 0 of 5 | 6 | 1 |
| selfless_cathar | 0 of 5 | 1 | 2 |
| selhoff_occultist | 0 of 5 | 6 | 0 |
| sensory_deprivation | 0 of 6 | 1 | 1 |
| sever_the_bloodline | 0 of 11 | 1 | 0 |
| sharpened_pitchfork | 0 of 10 | 2 | 2 |
| shimmering_grotto | 0 of 5 | 1 | 1 |
| silent_departure | 0 of 13 | 1 | 0 |
| silver_inlaid_dagger | 0 of 10 | 2 | 2 |
| silverchase_fox | 0 of 5 | 1 | 1 |
| skaab_goliath | 0 of 5 | 1 | 0 |
| skaab_ruinator | 0 of 5 | 1 | 1 |
| skeletal_grimace | 0 of 6 | 2 | 1 |
| skirsdag_cultist | 0 of 5 | 3 | 1 |
| skirsdag_high_priest | 0 of 5 | 1 | 2 |
| slayer_of_the_wicked | - | 1 | 1 |
| smite_the_monstrous | 0 of 9 | 1 | 1 |
| snapcaster_mage | - | 1 | 2 |
| sol_ring | 0 of 8 | 1 | 1 |
| somberwald_spider | - | 2 | 1 |
| spare_from_evil | 0 of 5 | 1 | 1 |
| spectral_flight | 0 of 6 | 1 | 1 |
| spectral_rider | 0 of 5 | 1 | 0 |
| spider_spawning | 0 of 11 | 3 | 0 |
| spidery_grasp | 0 of 8 | 1 | 0 |
| splinterfright | - | 4 | 0 |
| stensia_bloodhall | 0 of 8 | 2 | 1 |
| stitched_drake | 0 of 5 | 1 | 0 |
| stitchers_apprentice | 0 of 5 | 2 | 1 |
| stony_silence | 0 of 5 | 2 | 1 |
| stromkirk_noble | 0 of 5 | 2 | 0 |
| stromkirk_patrol | 0 of 5 | 2 | 0 |
| sturmgeist | - | 6 | 0 |
| sulfur_falls | - | 3 | 2 |
| swamp | 0 of 8 | 1 | 1 |
| swords_to_plowshares | 0 of 10 | 2 | 0 |
| think_twice | 0 of 10 | 6 | 0 |
| thraben_purebloods | 0 of 5 | 1 | 0 |
| thraben_sentry | 0 of 5 | 4 | 1 |
| tormented_pariah | 0 of 5 | 2 | 0 |
| traitorous_blood | 0 of 8 | 1 | 0 |
| travel_preparations | 0 of 12 | 3 | 0 |
| travelers_amulet | 0 of 5 | 2 | 1 |
| tree_of_redemption | 0 of 5 | 1 | 1 |
| trepanation_blade | 0 of 10 | 5 | 2 |
| tribute_to_hunger | - | 2 | 0 |
| typhoid_rats | 0 of 5 | 1 | 0 |
| ulvenwald_mystics | 0 of 5 | 3 | 0 |
| unbreathing_horde | - | 2 | 1 |
| unburial_rites | 0 of 12 | 1 | 0 |
| undead_alchemist | 0 of 5 | 2 | 0 |
| unruly_mob | 0 of 5 | 2 | 0 |
| urgent_exorcism | 0 of 10 | 2 | 1 |
| vampire_interloper | 0 of 5 | 1 | 1 |
| vampiric_fury | 0 of 5 | 1 | 8 |
| victim_of_night | 0 of 11 | 1 | 1 |
| village_bell_ringer | - | 1 | 1 |
| village_cannibals | 0 of 5 | 2 | 0 |
| village_ironsmith | 0 of 5 | 2 | 0 |
| villagers_of_estwald | 0 of 5 | 2 | 0 |
| voiceless_spirit | 0 of 5 | 1 | 0 |
| walking_corpse | 0 of 5 | 1 | 0 |
| witchbane_orb | - | 1 | 1 |
| wooden_stake | 0 of 10 | 3 | 2 |
| woodland_cemetery | - | 3 | 2 |
| woodland_sleuth | - | 3 | 1 |
| wreath_of_geists | 0 of 6 | 3 | 2 |

## Engine functions stood in for

`attachment::illegality`, `cards::clear_ids`, `cards::helpers::apply_transform`, `cards::helpers::counter_spell`, `cards::helpers::counter_spell_exiling`, `cards::helpers::finish_library_search`, `cards::helpers::format_tap_plan_names`, `cards::helpers::present_optional_target_choice`, `cards::helpers::present_target_choice`, `cards::helpers::search_library`, `cards::helpers::shuffle_library`, `cards::helpers::sorted`, `cards::helpers::token_attack_options`, `cards::helpers::tokens_enter_combat_attacking`, `cards::helpers::transform_quietly`, `cards::load_ids`, `cards::store_ids`, `combat::announce_damage_assignment_order`, `combat::any_first_strike_in_combat`, `combat::can_block_at_all`, `combat::can_block_attacker`, `combat::combat_damage_step`, `combat::deals_in_step`, `combat::declare_attackers`, `combat::declare_blockers`, `combat::declare_blockers_with_registry`, `combat::divide_combat_damage`, `combat::eligible_attackers`, `combat::end_combat`, `combat::fight`, `combat::log_completed_order`, `combat::mark_dealt_first_strike`, `combat::minimum_blockers`, `damage::applicable_effects`, `damage::apply_chosen_effect`, `damage::deal_damage`, `damage::perform`, `damage::process_pending_damage`, `damage::queue_damage`, `destruction::death_event`, `destruction::destroy_sba`, `destruction::regenerate_sba`, `destruction::sacrifice_by`, `destruction::try_destroy`, `destruction::try_destroy_all`, `destruction::try_destroy_all_by`, `destruction::try_destroy_by`, `destruction::try_destroy_no_regen`, `engine::actions::abilities::finish_activation`, `engine::actions::abilities::pay_activation_costs`, `engine::actions::abilities::put_ability_on_stack`, `engine::actions::abilities::tap_cost_candidates`, `engine::actions::abilities::tapped_for_cost_line`, `engine::actions::combat::who_attacks`, `engine::activate_mana_source_reserving`, `engine::advance_mulligan_phase`, `engine::advance_step`, `engine::apply_pending_effect`, `engine::can_be_targeted_by`, `engine::can_target_player`, `engine::costs::pay_additional_cost`, `engine::demand_reserve`, `engine::discard_cards`, `engine::draw_cards`, `engine::effects::cast_log_line`, `engine::empty_mana_pools`, `engine::end_until_end_of_turn_effects`, `engine::finish_cleanup`, `engine::granted_flashback_costs`, `engine::matches_target_filter`, `engine::mill_cards`, `engine::mill_one`, `engine::pay_cost_with_sources`, `engine::payment_plan_with_sources`, `engine::perform_turn_based_actions`, `engine::submit_action_inner`, `engine::targeting::can_be_targeted_by`, `engine::targeting::legal_under_mode`, `engine::targeting::matches_target_filter`, `engine::targeting::targets_are_legal`, `engine::targeting::valid_targets_for_req`, `engine::valid_targets_for_req`, `funding::apply`, `funding::validate`, `replacement::apply`, `replacement::for_entering`, `replacement::process_pending_entry_choices`, `replacement::record_entry_choice`, `sba::annihilate_counters`, `sba::apply_legend_rule`, `sba::check_state_based_actions`, `sba::creature_fates`, `sba::one_pass`, `sba::players_lose`, `sba::raise_state_trigger`, `sba::remove_creatures`, `sba::remove_illegal_attachments`, `sba::remove_planeswalkers_without_loyalty`, `sba::remove_tokens_elsewhere`, `stack::is_target_legal`, `stack::resolve_spell_as`, `stack::resolve_top_of_stack`, `state::GameState::add_counters`, `state::GameState::add_counters_quiet`, `state::GameState::add_mana`, `state::GameState::add_regeneration_shield`, `state::GameState::arrives_tapped`, `state::GameState::attach`, `state::GameState::become_copy_of`, `state::GameState::can_attack`, `state::GameState::can_block`, `state::GameState::cant_be_blocked`, `state::GameState::cease_to_exist`, `state::GameState::change_control`, `state::GameState::change_life`, `state::GameState::change_life_for`, `state::GameState::choose_at_random`, `state::GameState::colors_of`, `state::GameState::create_token_copy`, `state::GameState::create_token_with_defined_pt`, `state::GameState::create_tokens_with_subtypes`, `state::GameState::discard_card`, `state::GameState::discard_card_for`, `state::GameState::effective_power`, `state::GameState::effective_toughness`, `state::GameState::end_game`, `state::GameState::end_game_if_decided`, `state::GameState::expire_control_effects`, `state::GameState::flip_coin`, `state::GameState::gain_control_while_source_controlled`, `state::GameState::has_card_type`, `state::GameState::has_keyword`, `state::GameState::has_protection_from`, `state::GameState::has_subtype`, `state::GameState::has_summoning_sickness`, `state::GameState::is_creature`, `state::GameState::is_equipment`, `state::GameState::is_legendary`, `state::GameState::matches_filter`, `state::GameState::move_object`, `state::GameState::move_object_under_control`, `state::GameState::move_spell_after_resolve`, `state::GameState::must_attack`, `state::GameState::name_of`, `state::GameState::objects_in_zone`, `state::GameState::player_can_be_enchanted_by`, `state::GameState::player_loses`, `state::GameState::put_into_library`, `state::GameState::random_below`, `state::GameState::remove_counters`, `state::GameState::set_base_pt`, `state::GameState::tap`, `state::GameState::tap_for`, `state::GameState::unattach`, `state::GameState::unattach_from_player`, `state::GameState::untap`, `state::GameState::untap_for`, `state::GameState::untaps_normally`, `state::Protections::covers`, `triggers::PendingTrigger::log_name`, `triggers::collect_triggers`, `triggers::order_question_words`, `triggers::process_pending_trigger_pushes`, `triggers::push_one_pending_trigger`, `triggers::target_requirement`

## Where the engine proved is not the engine shipped

Lines of the engine's source, outside the proofs, that are another under `cfg(kani)`:

- `mtg-engine/src/cards/mod.rs`: 8
- `mtg-engine/src/damage.rs`: 2
- `mtg-engine/src/engine.rs`: 1
- `mtg-engine/src/events.rs`: 1
- `mtg-engine/src/lib.rs`: 4
- `mtg-engine/src/replacement.rs`: 1
- `mtg-engine/src/state.rs`: 17
- `mtg-engine/src/triggers.rs`: 1
- `mtg-engine/src/triggers/collect/mod.rs`: 2
- `mtg-engine/src/types.rs`: 2

