//! Written by `scripts/kani-printed-rows.py` from the rows of
//! `through_the_stack.rs` and the Oracle's type lines; not to be edited
//! by hand. Each card with rows there, cast in the other player's first
//! main phase (1) and in its caster's own upkeep (2): an instant is the
//! row named above it over again at that time, and a sorcery is refused
//! and nothing is done.

use super::*;

an_instant_at! { 1
    // altars_reap_sacrifices_a_creature_as_it_is_cast_and_draws_two
    altars_reap_is_cast_in_the_other_players_turn: sacrifices_a_creature_to((Box::new(AltarsReap), true, FIRST));
    // ancient_grudge_destroys_the_second_players_artifact
    ancient_grudge_is_cast_in_the_other_players_turn: destroys((Box::new(AncientGrudge), Permanent::Artifact, FIRST, (1, 1), Guard::Nothing));
    // ancient_grudge_flashed_back_destroys_an_artifact_and_is_exiled
    ancient_grudge_is_flashed_back_in_the_other_players_turn: destroys((Box::new(AncientGrudge), Permanent::Artifact, FLASHBACK, (1, 1), Guard::Nothing));
    // brimstone_volley_destroys_the_other_players_small_creature
    brimstone_volley_is_cast_in_the_other_players_turn: damages_a_creature((Box::new(BrimstoneVolley), Amount::Morbid { died: false }, (Permanent::Creature, 0), FIRST, 1));
    // doom_blade_destroys_the_second_players_creature
    doom_blade_is_cast_in_the_other_players_turn: destroys((Box::new(DoomBlade), Permanent::Creature, FIRST, (1, 1), Guard::Nothing));
    // dream_twist_mills_the_top_three_of_the_other_players_four
    dream_twist_is_cast_in_the_other_players_turn: mills((Box::new(DreamTwist), 3, 4, FIRST, 1));
    // dream_twist_flashed_back_mills_three_and_is_exiled
    dream_twist_is_flashed_back_in_the_other_players_turn: mills((Box::new(DreamTwist), 3, 4, FLASHBACK, 1));
    // feeling_of_dread_at_one_creature_taps_it
    feeling_of_dread_is_cast_in_the_other_players_turn: does_to_each_of_up_to_two((Box::new(FeelingOfDread), ToEach::Tap, 1, FIRST));
    // feeling_of_dread_flashed_back_taps_two_and_is_exiled
    feeling_of_dread_is_flashed_back_in_the_other_players_turn: does_to_each_of_up_to_two((Box::new(FeelingOfDread), ToEach::Tap, 2, FLASHBACK));
    // geistflame_marks_one_on_the_other_players_creature
    geistflame_is_cast_in_the_other_players_turn: damages_a_creature((Box::new(Geistflame), Amount::Fixed(1), (Permanent::Creature, 0), FIRST, 1));
    // geistflame_flashed_back_takes_one_life_and_is_exiled
    geistflame_is_flashed_back_in_the_other_players_turn: damages_a_player((Box::new(Geistflame), Amount::Fixed(1), FLASHBACK, 1));
    // giant_growth_gives_your_creature_three_and_three
    giant_growth_is_cast_in_the_other_players_turn: pumps((Box::new(GiantGrowth), (3, 3), None, false, FIRST, 0));
    // gnaw_to_the_bone_gains_nothing_with_no_creature_card
    gnaw_to_the_bone_is_cast_in_the_other_players_turn: gnaw_to_the_bone((0, FIRST));
    // gnaw_to_the_bone_flashed_back_gains_two_and_is_exiled
    gnaw_to_the_bone_is_flashed_back_in_the_other_players_turn: gnaw_to_the_bone((1, FLASHBACK));
    // hysterical_blindness_takes_four_power_from_the_other_players_creatures
    hysterical_blindness_is_cast_in_the_other_players_turn: gives_each((Box::new(HystericalBlindness), ToAll::TheirsMinusFour, FIRST));
    // lightning_bolt_destroys_the_other_players_small_creature
    lightning_bolt_is_cast_in_the_other_players_turn: damages_a_creature((Box::new(LightningBolt), Amount::Fixed(3), (Permanent::Creature, 0), FIRST, 1));
    // midnight_haunting_makes_its_caster_two_flying_spirits
    midnight_haunting_is_cast_in_the_other_players_turn: makes_tokens((Box::new(MidnightHaunting), 2, (1, 1), Color::White, "Spirit", Some(Keyword::Flying), FIRST));
    // moment_of_heroism_gives_your_creature_two_and_two_and_lifelink
    moment_of_heroism_is_cast_in_the_other_players_turn: pumps((Box::new(MomentOfHeroism), (2, 2), Some(Keyword::Lifelink), false, FIRST, 0));
    // naturalize_destroys_the_second_players_artifact
    naturalize_is_cast_in_the_other_players_turn: destroys((Box::new(Naturalize), Permanent::Artifact, FIRST, (1, 1), Guard::Nothing));
    // purify_the_grave_exiles_a_card_from_the_other_players_graveyard
    purify_the_grave_is_cast_in_the_other_players_turn: moves_a_card((Box::new(PurifyTheGrave), Zone::Graveyard, Zone::Exile, FIRST, 1));
    // purify_the_grave_flashed_back_exiles_a_card_and_is_exiled
    purify_the_grave_is_flashed_back_in_the_other_players_turn: moves_a_card((Box::new(PurifyTheGrave), Zone::Graveyard, Zone::Exile, FLASHBACK, 1));
    // rally_the_peasants_gives_its_casters_creatures_two_power
    rally_the_peasants_is_cast_in_the_other_players_turn: gives_each((Box::new(RallyThePeasants), ToAll::YoursPlusTwo, FIRST));
    // rally_the_peasants_flashed_back_gives_two_power_and_is_exiled
    rally_the_peasants_is_flashed_back_in_the_other_players_turn: gives_each((Box::new(RallyThePeasants), ToAll::YoursPlusTwo, FLASHBACK));
    // rangers_guile_gives_your_creature_one_and_one_and_hexproof
    rangers_guile_is_cast_in_the_other_players_turn: pumps((Box::new(RangersGuile), (1, 1), Some(Keyword::Hexproof), false, FIRST, 0));
    // smite_the_monstrous_destroys_the_second_players_four_power
    smite_the_monstrous_is_cast_in_the_other_players_turn: destroys((Box::new(SmiteTheMonstrous), Permanent::Monstrous, FIRST, (1, 1), Guard::Nothing));
    // spare_from_evil_protects_its_casters_creatures_from_what_is_no_human
    spare_from_evil_is_cast_in_the_other_players_turn: gives_each((Box::new(SpareFromEvil), ToAll::YoursProtectionFromNonHumans, FIRST));
    // spidery_grasp_untaps_your_creature_and_gives_it_two_and_four_and_reach
    spidery_grasp_is_cast_in_the_other_players_turn: pumps((Box::new(SpideryGrasp), (2, 4), Some(Keyword::Reach), true, FIRST, 0));
    // swords_to_plowshares_exiles_the_other_players_creature_and_they_gain_two
    swords_to_plowshares_is_cast_in_the_other_players_turn: swords_to_plowshares((Permanent::Creature, FIRST, (1, 1)));
    // think_twice_cast_by_the_first_player_draws_the_top_of_two
    think_twice_is_cast_in_the_other_players_turn: draws((Box::new(ThinkTwice), 1, 2, FIRST));
    // think_twice_flashed_back_draws_a_card_and_is_exiled
    think_twice_is_flashed_back_in_the_other_players_turn: draws((Box::new(ThinkTwice), 1, 2, FLASHBACK));
    // urgent_exorcism_destroys_the_second_players_spirit
    urgent_exorcism_is_cast_in_the_other_players_turn: destroys((Box::new(UrgentExorcism), Permanent::Spirit, FIRST, (1, 1), Guard::Nothing));
    // vampiric_fury_gives_its_casters_vampires_two_power_and_first_strike
    vampiric_fury_is_cast_in_the_other_players_turn: gives_each((Box::new(VampiricFury), ToAll::YourVampiresPlusTwoAndFirstStrike, FIRST));
    // victim_of_night_destroys_the_second_players_creature
    victim_of_night_is_cast_in_the_other_players_turn: destroys((Box::new(VictimOfNight), Permanent::Creature, FIRST, (1, 1), Guard::Nothing));
}

an_instant_at! { 2
    // altars_reap_sacrifices_a_creature_as_it_is_cast_and_draws_two
    altars_reap_is_cast_in_its_casters_upkeep: sacrifices_a_creature_to((Box::new(AltarsReap), true, FIRST));
    // ancient_grudge_destroys_the_second_players_artifact
    ancient_grudge_is_cast_in_its_casters_upkeep: destroys((Box::new(AncientGrudge), Permanent::Artifact, FIRST, (1, 1), Guard::Nothing));
    // ancient_grudge_flashed_back_destroys_an_artifact_and_is_exiled
    ancient_grudge_is_flashed_back_in_its_casters_upkeep: destroys((Box::new(AncientGrudge), Permanent::Artifact, FLASHBACK, (1, 1), Guard::Nothing));
    // brimstone_volley_destroys_the_other_players_small_creature
    brimstone_volley_is_cast_in_its_casters_upkeep: damages_a_creature((Box::new(BrimstoneVolley), Amount::Morbid { died: false }, (Permanent::Creature, 0), FIRST, 1));
    // doom_blade_destroys_the_second_players_creature
    doom_blade_is_cast_in_its_casters_upkeep: destroys((Box::new(DoomBlade), Permanent::Creature, FIRST, (1, 1), Guard::Nothing));
    // dream_twist_mills_the_top_three_of_the_other_players_four
    dream_twist_is_cast_in_its_casters_upkeep: mills((Box::new(DreamTwist), 3, 4, FIRST, 1));
    // dream_twist_flashed_back_mills_three_and_is_exiled
    dream_twist_is_flashed_back_in_its_casters_upkeep: mills((Box::new(DreamTwist), 3, 4, FLASHBACK, 1));
    // feeling_of_dread_at_one_creature_taps_it
    feeling_of_dread_is_cast_in_its_casters_upkeep: does_to_each_of_up_to_two((Box::new(FeelingOfDread), ToEach::Tap, 1, FIRST));
    // feeling_of_dread_flashed_back_taps_two_and_is_exiled
    feeling_of_dread_is_flashed_back_in_its_casters_upkeep: does_to_each_of_up_to_two((Box::new(FeelingOfDread), ToEach::Tap, 2, FLASHBACK));
    // geistflame_marks_one_on_the_other_players_creature
    geistflame_is_cast_in_its_casters_upkeep: damages_a_creature((Box::new(Geistflame), Amount::Fixed(1), (Permanent::Creature, 0), FIRST, 1));
    // geistflame_flashed_back_takes_one_life_and_is_exiled
    geistflame_is_flashed_back_in_its_casters_upkeep: damages_a_player((Box::new(Geistflame), Amount::Fixed(1), FLASHBACK, 1));
    // giant_growth_gives_your_creature_three_and_three
    giant_growth_is_cast_in_its_casters_upkeep: pumps((Box::new(GiantGrowth), (3, 3), None, false, FIRST, 0));
    // gnaw_to_the_bone_gains_nothing_with_no_creature_card
    gnaw_to_the_bone_is_cast_in_its_casters_upkeep: gnaw_to_the_bone((0, FIRST));
    // gnaw_to_the_bone_flashed_back_gains_two_and_is_exiled
    gnaw_to_the_bone_is_flashed_back_in_its_casters_upkeep: gnaw_to_the_bone((1, FLASHBACK));
    // hysterical_blindness_takes_four_power_from_the_other_players_creatures
    hysterical_blindness_is_cast_in_its_casters_upkeep: gives_each((Box::new(HystericalBlindness), ToAll::TheirsMinusFour, FIRST));
    // lightning_bolt_destroys_the_other_players_small_creature
    lightning_bolt_is_cast_in_its_casters_upkeep: damages_a_creature((Box::new(LightningBolt), Amount::Fixed(3), (Permanent::Creature, 0), FIRST, 1));
    // midnight_haunting_makes_its_caster_two_flying_spirits
    midnight_haunting_is_cast_in_its_casters_upkeep: makes_tokens((Box::new(MidnightHaunting), 2, (1, 1), Color::White, "Spirit", Some(Keyword::Flying), FIRST));
    // moment_of_heroism_gives_your_creature_two_and_two_and_lifelink
    moment_of_heroism_is_cast_in_its_casters_upkeep: pumps((Box::new(MomentOfHeroism), (2, 2), Some(Keyword::Lifelink), false, FIRST, 0));
    // naturalize_destroys_the_second_players_artifact
    naturalize_is_cast_in_its_casters_upkeep: destroys((Box::new(Naturalize), Permanent::Artifact, FIRST, (1, 1), Guard::Nothing));
    // purify_the_grave_exiles_a_card_from_the_other_players_graveyard
    purify_the_grave_is_cast_in_its_casters_upkeep: moves_a_card((Box::new(PurifyTheGrave), Zone::Graveyard, Zone::Exile, FIRST, 1));
    // purify_the_grave_flashed_back_exiles_a_card_and_is_exiled
    purify_the_grave_is_flashed_back_in_its_casters_upkeep: moves_a_card((Box::new(PurifyTheGrave), Zone::Graveyard, Zone::Exile, FLASHBACK, 1));
    // rally_the_peasants_gives_its_casters_creatures_two_power
    rally_the_peasants_is_cast_in_its_casters_upkeep: gives_each((Box::new(RallyThePeasants), ToAll::YoursPlusTwo, FIRST));
    // rally_the_peasants_flashed_back_gives_two_power_and_is_exiled
    rally_the_peasants_is_flashed_back_in_its_casters_upkeep: gives_each((Box::new(RallyThePeasants), ToAll::YoursPlusTwo, FLASHBACK));
    // rangers_guile_gives_your_creature_one_and_one_and_hexproof
    rangers_guile_is_cast_in_its_casters_upkeep: pumps((Box::new(RangersGuile), (1, 1), Some(Keyword::Hexproof), false, FIRST, 0));
    // smite_the_monstrous_destroys_the_second_players_four_power
    smite_the_monstrous_is_cast_in_its_casters_upkeep: destroys((Box::new(SmiteTheMonstrous), Permanent::Monstrous, FIRST, (1, 1), Guard::Nothing));
    // spare_from_evil_protects_its_casters_creatures_from_what_is_no_human
    spare_from_evil_is_cast_in_its_casters_upkeep: gives_each((Box::new(SpareFromEvil), ToAll::YoursProtectionFromNonHumans, FIRST));
    // spidery_grasp_untaps_your_creature_and_gives_it_two_and_four_and_reach
    spidery_grasp_is_cast_in_its_casters_upkeep: pumps((Box::new(SpideryGrasp), (2, 4), Some(Keyword::Reach), true, FIRST, 0));
    // swords_to_plowshares_exiles_the_other_players_creature_and_they_gain_two
    swords_to_plowshares_is_cast_in_its_casters_upkeep: swords_to_plowshares((Permanent::Creature, FIRST, (1, 1)));
    // think_twice_cast_by_the_first_player_draws_the_top_of_two
    think_twice_is_cast_in_its_casters_upkeep: draws((Box::new(ThinkTwice), 1, 2, FIRST));
    // think_twice_flashed_back_draws_a_card_and_is_exiled
    think_twice_is_flashed_back_in_its_casters_upkeep: draws((Box::new(ThinkTwice), 1, 2, FLASHBACK));
    // urgent_exorcism_destroys_the_second_players_spirit
    urgent_exorcism_is_cast_in_its_casters_upkeep: destroys((Box::new(UrgentExorcism), Permanent::Spirit, FIRST, (1, 1), Guard::Nothing));
    // vampiric_fury_gives_its_casters_vampires_two_power_and_first_strike
    vampiric_fury_is_cast_in_its_casters_upkeep: gives_each((Box::new(VampiricFury), ToAll::YourVampiresPlusTwoAndFirstStrike, FIRST));
    // victim_of_night_destroys_the_second_players_creature
    victim_of_night_is_cast_in_its_casters_upkeep: destroys((Box::new(VictimOfNight), Permanent::Creature, FIRST, (1, 1), Guard::Nothing));
}

a_sorcery_at! { 1
    // blasphemous_act_with_no_creature_costs_all_nine
    blasphemous_act_is_not_cast_in_the_other_players_turn: blasphemous_act((0, FIRST));
    // bramblecrush_destroys_the_second_players_artifact
    bramblecrush_is_not_cast_in_the_other_players_turn: destroys((Box::new(Bramblecrush), Permanent::Artifact, FIRST, (1, 1), Guard::Nothing));
    // bump_in_the_night_takes_three_life_from_the_second_player
    bump_in_the_night_is_not_cast_in_the_other_players_turn: bump_in_the_night(FIRST);
    // bump_in_the_night_flashed_back_takes_three_life_and_is_exiled
    bump_in_the_night_is_not_flashed_back_in_the_other_players_turn: bump_in_the_night(FLASHBACK);
    // divination_cast_by_the_first_player_draws_the_top_two_of_three
    divination_is_not_cast_in_the_other_players_turn: draws((Box::new(Divination), 2, 3, FIRST));
    // grasp_of_phantoms_puts_the_other_players_creature_on_their_library_of_two
    grasp_of_phantoms_is_not_cast_in_the_other_players_turn: grasp_of_phantoms((2, FIRST, (1, 1)));
    // grasp_of_phantoms_flashed_back_puts_a_creature_on_its_library_and_is_exiled
    grasp_of_phantoms_is_not_flashed_back_in_the_other_players_turn: grasp_of_phantoms((1, FLASHBACK, (1, 1)));
    // infernal_plunge_sacrifices_a_creature_as_it_is_cast_and_adds_three_red
    infernal_plunge_is_not_cast_in_the_other_players_turn: sacrifices_a_creature_to((Box::new(InfernalPlunge), false, FIRST));
    // into_the_maw_of_hell_destroys_a_land_and_a_stout_creature
    into_the_maw_of_hell_is_not_cast_in_the_other_players_turn: destroys_a_land_and((Box::new(IntoTheMawOfHell), true, FIRST, 1));
    // lava_axe_takes_five_life_from_the_other_player
    lava_axe_is_not_cast_in_the_other_players_turn: damages_a_player((Box::new(LavaAxe), Amount::Fixed(5), FIRST, 1));
    // maw_of_the_mire_destroys_the_other_players_land_and_gains_four
    maw_of_the_mire_is_not_cast_in_the_other_players_turn: destroys_a_land_and((Box::new(MawOfTheMire), false, FIRST, 1));
    // moan_of_the_unhallowed_makes_its_caster_two_zombies
    moan_of_the_unhallowed_is_not_cast_in_the_other_players_turn: makes_tokens((Box::new(MoanOfTheUnhallowed), 2, (2, 2), Color::Black, "Zombie", None, FIRST));
    // moan_of_the_unhallowed_flashed_back_makes_two_zombies_and_is_exiled
    moan_of_the_unhallowed_is_not_flashed_back_in_the_other_players_turn: makes_tokens((Box::new(MoanOfTheUnhallowed), 2, (2, 2), Color::Black, "Zombie", None, FLASHBACK));
    // nightbirds_clutches_at_one_creature_stops_it_blocking
    nightbirds_clutches_is_not_cast_in_the_other_players_turn: does_to_each_of_up_to_two((Box::new(NightbirdsClutches), ToEach::CantBlock, 1, FIRST));
    // nightbirds_clutches_flashed_back_stops_two_blocking_and_is_exiled
    nightbirds_clutches_is_not_flashed_back_in_the_other_players_turn: does_to_each_of_up_to_two((Box::new(NightbirdsClutches), ToEach::CantBlock, 2, FLASHBACK));
    // paraselene_destroys_both_enchantments_and_gains_two
    paraselene_is_not_cast_in_the_other_players_turn: paraselene((FIRST, false));
    // prey_upon_has_your_creature_fight_the_other_players_and_both_die
    prey_upon_is_not_cast_in_the_other_players_turn: prey_upon((FIRST, Permanent::Creature));
    // rolling_temblor_marks_two_on_each_creature
    rolling_temblor_is_not_cast_in_the_other_players_turn: rolling_temblor((FIRST, false));
    // rolling_temblor_flashed_back_marks_two_on_each_creature_and_is_exiled
    rolling_temblor_is_not_flashed_back_in_the_other_players_turn: rolling_temblor((FLASHBACK, false));
    // runic_repetition_returns_your_exiled_card_with_flashback
    runic_repetition_is_not_cast_in_the_other_players_turn: runic_repetition((FIRST, true));
    // sever_the_bloodline_exiles_a_creature_and_its_namesake_and_no_other
    sever_the_bloodline_is_not_cast_in_the_other_players_turn: sever_the_bloodline(FIRST);
    // sever_the_bloodline_flashed_back_exiles_both_and_is_exiled
    sever_the_bloodline_is_not_flashed_back_in_the_other_players_turn: sever_the_bloodline(FLASHBACK);
    // silent_departure_returns_the_other_players_creature
    silent_departure_is_not_cast_in_the_other_players_turn: silent_departure((FIRST, (1, 1)));
    // silent_departure_flashed_back_returns_a_creature_and_is_exiled
    silent_departure_is_not_flashed_back_in_the_other_players_turn: silent_departure((FLASHBACK, (1, 1)));
    // spider_spawning_makes_no_spider_with_no_creature_card
    spider_spawning_is_not_cast_in_the_other_players_turn: spider_spawning((0, FIRST));
    // spider_spawning_flashed_back_makes_a_spider_and_is_exiled
    spider_spawning_is_not_flashed_back_in_the_other_players_turn: spider_spawning((1, FLASHBACK));
    // traitorous_blood_takes_the_other_players_creature
    traitorous_blood_is_not_cast_in_the_other_players_turn: traitorous_blood((FIRST, 1));
    // travel_preparations_at_one_creature_puts_a_counter_on_it
    travel_preparations_is_not_cast_in_the_other_players_turn: does_to_each_of_up_to_two((Box::new(TravelPreparations), ToEach::Counter, 1, FIRST));
    // travel_preparations_flashed_back_puts_two_counters_and_is_exiled
    travel_preparations_is_not_flashed_back_in_the_other_players_turn: does_to_each_of_up_to_two((Box::new(TravelPreparations), ToEach::Counter, 2, FLASHBACK));
    // unburial_rites_returns_a_creature_card_from_your_graveyard
    unburial_rites_is_not_cast_in_the_other_players_turn: moves_a_card((Box::new(UnburialRites), Zone::Graveyard, Zone::Battlefield, FIRST, 0));
    // unburial_rites_flashed_back_returns_a_creature_card_and_is_exiled
    unburial_rites_is_not_flashed_back_in_the_other_players_turn: moves_a_card((Box::new(UnburialRites), Zone::Graveyard, Zone::Battlefield, FLASHBACK, 0));
}

a_sorcery_at! { 2
    // blasphemous_act_with_no_creature_costs_all_nine
    blasphemous_act_is_not_cast_in_its_casters_upkeep: blasphemous_act((0, FIRST));
    // bramblecrush_destroys_the_second_players_artifact
    bramblecrush_is_not_cast_in_its_casters_upkeep: destroys((Box::new(Bramblecrush), Permanent::Artifact, FIRST, (1, 1), Guard::Nothing));
    // bump_in_the_night_takes_three_life_from_the_second_player
    bump_in_the_night_is_not_cast_in_its_casters_upkeep: bump_in_the_night(FIRST);
    // bump_in_the_night_flashed_back_takes_three_life_and_is_exiled
    bump_in_the_night_is_not_flashed_back_in_its_casters_upkeep: bump_in_the_night(FLASHBACK);
    // divination_cast_by_the_first_player_draws_the_top_two_of_three
    divination_is_not_cast_in_its_casters_upkeep: draws((Box::new(Divination), 2, 3, FIRST));
    // grasp_of_phantoms_puts_the_other_players_creature_on_their_library_of_two
    grasp_of_phantoms_is_not_cast_in_its_casters_upkeep: grasp_of_phantoms((2, FIRST, (1, 1)));
    // grasp_of_phantoms_flashed_back_puts_a_creature_on_its_library_and_is_exiled
    grasp_of_phantoms_is_not_flashed_back_in_its_casters_upkeep: grasp_of_phantoms((1, FLASHBACK, (1, 1)));
    // infernal_plunge_sacrifices_a_creature_as_it_is_cast_and_adds_three_red
    infernal_plunge_is_not_cast_in_its_casters_upkeep: sacrifices_a_creature_to((Box::new(InfernalPlunge), false, FIRST));
    // into_the_maw_of_hell_destroys_a_land_and_a_stout_creature
    into_the_maw_of_hell_is_not_cast_in_its_casters_upkeep: destroys_a_land_and((Box::new(IntoTheMawOfHell), true, FIRST, 1));
    // lava_axe_takes_five_life_from_the_other_player
    lava_axe_is_not_cast_in_its_casters_upkeep: damages_a_player((Box::new(LavaAxe), Amount::Fixed(5), FIRST, 1));
    // maw_of_the_mire_destroys_the_other_players_land_and_gains_four
    maw_of_the_mire_is_not_cast_in_its_casters_upkeep: destroys_a_land_and((Box::new(MawOfTheMire), false, FIRST, 1));
    // moan_of_the_unhallowed_makes_its_caster_two_zombies
    moan_of_the_unhallowed_is_not_cast_in_its_casters_upkeep: makes_tokens((Box::new(MoanOfTheUnhallowed), 2, (2, 2), Color::Black, "Zombie", None, FIRST));
    // moan_of_the_unhallowed_flashed_back_makes_two_zombies_and_is_exiled
    moan_of_the_unhallowed_is_not_flashed_back_in_its_casters_upkeep: makes_tokens((Box::new(MoanOfTheUnhallowed), 2, (2, 2), Color::Black, "Zombie", None, FLASHBACK));
    // nightbirds_clutches_at_one_creature_stops_it_blocking
    nightbirds_clutches_is_not_cast_in_its_casters_upkeep: does_to_each_of_up_to_two((Box::new(NightbirdsClutches), ToEach::CantBlock, 1, FIRST));
    // nightbirds_clutches_flashed_back_stops_two_blocking_and_is_exiled
    nightbirds_clutches_is_not_flashed_back_in_its_casters_upkeep: does_to_each_of_up_to_two((Box::new(NightbirdsClutches), ToEach::CantBlock, 2, FLASHBACK));
    // paraselene_destroys_both_enchantments_and_gains_two
    paraselene_is_not_cast_in_its_casters_upkeep: paraselene((FIRST, false));
    // prey_upon_has_your_creature_fight_the_other_players_and_both_die
    prey_upon_is_not_cast_in_its_casters_upkeep: prey_upon((FIRST, Permanent::Creature));
    // rolling_temblor_marks_two_on_each_creature
    rolling_temblor_is_not_cast_in_its_casters_upkeep: rolling_temblor((FIRST, false));
    // rolling_temblor_flashed_back_marks_two_on_each_creature_and_is_exiled
    rolling_temblor_is_not_flashed_back_in_its_casters_upkeep: rolling_temblor((FLASHBACK, false));
    // runic_repetition_returns_your_exiled_card_with_flashback
    runic_repetition_is_not_cast_in_its_casters_upkeep: runic_repetition((FIRST, true));
    // sever_the_bloodline_exiles_a_creature_and_its_namesake_and_no_other
    sever_the_bloodline_is_not_cast_in_its_casters_upkeep: sever_the_bloodline(FIRST);
    // sever_the_bloodline_flashed_back_exiles_both_and_is_exiled
    sever_the_bloodline_is_not_flashed_back_in_its_casters_upkeep: sever_the_bloodline(FLASHBACK);
    // silent_departure_returns_the_other_players_creature
    silent_departure_is_not_cast_in_its_casters_upkeep: silent_departure((FIRST, (1, 1)));
    // silent_departure_flashed_back_returns_a_creature_and_is_exiled
    silent_departure_is_not_flashed_back_in_its_casters_upkeep: silent_departure((FLASHBACK, (1, 1)));
    // spider_spawning_makes_no_spider_with_no_creature_card
    spider_spawning_is_not_cast_in_its_casters_upkeep: spider_spawning((0, FIRST));
    // spider_spawning_flashed_back_makes_a_spider_and_is_exiled
    spider_spawning_is_not_flashed_back_in_its_casters_upkeep: spider_spawning((1, FLASHBACK));
    // traitorous_blood_takes_the_other_players_creature
    traitorous_blood_is_not_cast_in_its_casters_upkeep: traitorous_blood((FIRST, 1));
    // travel_preparations_at_one_creature_puts_a_counter_on_it
    travel_preparations_is_not_cast_in_its_casters_upkeep: does_to_each_of_up_to_two((Box::new(TravelPreparations), ToEach::Counter, 1, FIRST));
    // travel_preparations_flashed_back_puts_two_counters_and_is_exiled
    travel_preparations_is_not_flashed_back_in_its_casters_upkeep: does_to_each_of_up_to_two((Box::new(TravelPreparations), ToEach::Counter, 2, FLASHBACK));
    // unburial_rites_returns_a_creature_card_from_your_graveyard
    unburial_rites_is_not_cast_in_its_casters_upkeep: moves_a_card((Box::new(UnburialRites), Zone::Graveyard, Zone::Battlefield, FIRST, 0));
    // unburial_rites_flashed_back_returns_a_creature_card_and_is_exiled
    unburial_rites_is_not_flashed_back_in_its_casters_upkeep: moves_a_card((Box::new(UnburialRites), Zone::Graveyard, Zone::Battlefield, FLASHBACK, 0));
}

// 128 rows for 46 cards. Cards with rows by hand and none here, and why:
// - counterspell: cast in response to the other player's spell, in their turn
// - dissipate: cast in response to the other player's spell, in their turn
// - lost_in_the_mist: cast in response to the other player's spell, in their turn
// - rebuke: cast at an attacking creature, in the declare attackers step of the other player's turn
