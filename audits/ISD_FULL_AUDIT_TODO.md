# ISD full audit — every card, the whole procedure, one at a time

249 cards. Each row gets `/check-card-procedure` run on it alone: fetch its
oracle text and its rulings, read its implementation, search its tests, think
about its interactions, write the entry. Ordered most complex first.

Earlier passes over this set were done in batches — oracle text fetched for
twenty-five cards at once, implementations read side by side. That found real
bugs but it is not this. This list is the per-card pass.

| ✓ | card | file | loc | hooks | trig |
|---|------|------|-----|-------|------|
| x | Civilized Scholar | `civilized_scholar.rs` | 240 | 12 | 5 |
| x | Garruk Relentless | `garruk_relentless.rs` | 301 | 9 | 0 |
| x | Liliana of the Veil | `liliana_of_the_veil.rs` | 274 | 9 | 0 |
| x | Mayor of Avabruck | `mayor_of_avabruck.rs` | 135 | 7 | 4 |
| x | Screeching Bat | `screeching_bat.rs` | 145 | 7 | 3 |
| x | Cloistered Youth | `cloistered_youth.rs` | 115 | 7 | 4 |
| x | Wooden Stake | `wooden_stake.rs` | 102 | 8 | 2 |
| x | Daybreak Ranger | `daybreak_ranger.rs` | 135 | 7 | 2 |
| x | Delver of Secrets | `delver_of_secrets.rs` | 135 | 7 | 2 |
| x | Grimgrin, Corpse-Born | `grimgrin_corpse_born.rs` | 148 | 7 | 1 |
| x | Ulvenwald Mystics | `ulvenwald_mystics.rs` | 105 | 7 | 2 |
| x | Evil Twin | `evil_twin.rs` | 125 | 7 | 1 |
| x | Bitterheart Witch | `bitterheart_witch.rs` | 167 | 6 | 1 |
| x | Curse of Oblivion | `curse_of_oblivion.rs` | 140 | 6 | 2 |
| x | Charmbreaker Devils | `charmbreaker_devils.rs` | 119 | 5 | 3 |
| x | Bloodline Keeper | `bloodline_keeper.rs` | 148 | 6 | 0 |
| x | Divine Reckoning | `divine_reckoning.rs` | 146 | 6 | 0 |
| x | Moorland Haunt | `moorland_haunt.rs` | 132 | 6 | 0 |
| x | Trepanation Blade | `trepanation_blade.rs` | 139 | 5 | 1 |
| x | Curse of the Pierced Heart | `curse_of_the_pierced_heart.rs` | 109 | 5 | 2 |
| x | Instigator Gang | `instigator_gang.rs` | 107 | 5 | 2 |
| x | Curiosity | `curiosity.rs` | 79 | 6 | 1 |
| x | Blazing Torch | `blazing_torch.rs` | 147 | 5 | 0 |
| x | Fiend Hunter | `fiend_hunter.rs` | 91 | 5 | 2 |
| x | Kruin Outlaw | `kruin_outlaw.rs` | 90 | 5 | 2 |
| x | Caravan Vigil | `caravan_vigil.rs` | 136 | 5 | 0 |
| x | Olivia Voldaren | `olivia_voldaren.rs` | 152 | 4 | 1 |
| x | Graveyard Shovel | `graveyard_shovel.rs` | 131 | 5 | 0 |
| x | Hanweir Watchkeep | `hanweir_watchkeep.rs` | 80 | 5 | 2 |
| x | Village Ironsmith | `village_ironsmith.rs` | 78 | 5 | 2 |
| x | Gatstaf Shepherd | `gatstaf_shepherd.rs` | 77 | 5 | 2 |
| x | Grizzled Outcasts | `grizzled_outcasts.rs` | 76 | 5 | 2 |
| x | Tormented Pariah | `tormented_pariah.rs` | 76 | 5 | 2 |
| x | Villagers of Estwald | `villagers_of_estwald.rs` | 76 | 5 | 2 |
| x | Reckless Waif | `reckless_waif.rs` | 75 | 5 | 2 |
| x | Grimoire of the Dead | `grimoire_of_the_dead.rs` | 168 | 4 | 0 |
| x | Curse of the Bloody Tome | `curse_of_the_bloody_tome.rs` | 66 | 5 | 2 |
| x | Thraben Sentry | `thraben_sentry.rs` | 86 | 5 | 1 |
| x | Ludevic's Test Subject | `ludevics_test_subject.rs` | 100 | 5 | 0 |
| x | Hamlet Captain | `hamlet_captain.rs` | 88 | 4 | 2 |
| x | Reaper from the Abyss | `reaper_from_the_abyss.rs` | 88 | 4 | 2 |
| x | Skirsdag High Priest | `skirsdag_high_priest.rs` | 137 | 4 | 0 |
| x | Tribute to Hunger | `tribute_to_hunger.rs` | 92 | 5 | 0 |
| x | Morkrut Banshee | `morkrut_banshee.rs` | 66 | 5 | 1 |
| x | Brain Weevil | `brain_weevil.rs` | 131 | 4 | 0 |
| x | Mikaeus, the Lunarch | `mikaeus_the_lunarch.rs` | 131 | 4 | 0 |
| x | Runechanter's Pike | `runechanters_pike.rs` | 86 | 5 | 0 |
| x | Claustrophobia | `claustrophobia.rs` | 56 | 5 | 1 |
| x | Mentor of the Meek | `mentor_of_the_meek.rs` | 101 | 4 | 1 |
| x | Splinterfright | `splinterfright.rs` | 71 | 4 | 2 |
| x | Back from the Brink | `back_from_the_brink.rs` | 120 | 4 | 0 |
| x | Angel of Flight Alabaster | `angel_of_flight_alabaster.rs` | 67 | 4 | 2 |
| x | Bloodgift Demon | `bloodgift_demon.rs` | 66 | 4 | 2 |
| x | Skeletal Grimace | `skeletal_grimace.rs` | 64 | 5 | 0 |
| x | Ghost Quarter | `ghost_quarter.rs` | 104 | 4 | 0 |
| x | Woodland Sleuth | `woodland_sleuth.rs` | 79 | 4 | 1 |
| x | Elder Cathar | `elder_cathar.rs` | 75 | 4 | 1 |
| x | Snapcaster Mage | `snapcaster_mage.rs` | 68 | 4 | 1 |
| x | Witchbane Orb | `witchbane_orb.rs` | 60 | 4 | 1 |
| x | Demonmail Hauberk | `demonmail_hauberk.rs` | 82 | 4 | 0 |
| x | Geist-Honored Monk | `geist_honored_monk.rs` | 56 | 4 | 1 |
| x | Manor Gargoyle | `manor_gargoyle.rs` | 81 | 4 | 0 |
| x | Silver-Inlaid Dagger | `silver_inlaid_dagger.rs` | 81 | 4 | 0 |
| x | Undead Alchemist | `undead_alchemist.rs` | 101 | 3 | 1 |
| x | Butcher's Cleaver | `butchers_cleaver.rs` | 80 | 4 | 0 |
| x | Curse of Stalked Prey | `curse_of_stalked_prey.rs` | 55 | 4 | 1 |
| x | Into the Maw of Hell | `into_the_maw_of_hell.rs` | 80 | 4 | 0 |
| x | Sharpened Pitchfork | `sharpened_pitchfork.rs` | 79 | 4 | 0 |
| x | Endless Ranks of the Dead | `endless_ranks_of_the_dead.rs` | 72 | 3 | 2 |
| x | Hollowhenge Scavenger | `hollowhenge_scavenger.rs` | 52 | 4 | 1 |
| x | Falkenrath Noble | `falkenrath_noble.rs` | 71 | 3 | 2 |
| x | Heretic's Punishment | `heretics_punishment.rs` | 121 | 3 | 0 |
| x | Kessig Wolf Run | `kessig_wolf_run.rs` | 75 | 4 | 0 |
| x | Night Terrors | `night_terrors.rs` | 75 | 4 | 0 |
| x | Selhoff Occultist | `selhoff_occultist.rs` | 70 | 3 | 2 |
| x | Avacynian Priest | `avacynian_priest.rs` | 74 | 4 | 0 |
| x | Clifftop Retreat | `clifftop_retreat.rs` | 73 | 4 | 0 |
| x | Hinterland Harbor | `hinterland_harbor.rs` | 73 | 4 | 0 |
| x | Isolated Chapel | `isolated_chapel.rs` | 73 | 4 | 0 |
| x | Sulfur Falls | `sulfur_falls.rs` | 73 | 4 | 0 |
| x | Woodland Cemetery | `woodland_cemetery.rs` | 73 | 4 | 0 |
| x | Inquisitor's Flail | `inquisitors_flail.rs` | 70 | 4 | 0 |
| x | Cobbled Wings | `cobbled_wings.rs` | 69 | 4 | 0 |
| x | Gavony Township | `gavony_township.rs` | 69 | 4 | 0 |
| x | Lost in the Mist | `lost_in_the_mist.rs` | 68 | 4 | 0 |
| x | Murder of Crows | `murder_of_crows.rs` | 88 | 3 | 1 |
| x | Frightful Delusion | `frightful_delusion.rs` | 67 | 4 | 0 |
| x | Maw of the Mire | `maw_of_the_mire.rs` | 67 | 4 | 0 |
| x | Stensia Bloodhall | `stensia_bloodhall.rs` | 66 | 4 | 0 |
| x | Ghoulcaller's Chant | `ghoulcallers_chant.rs` | 65 | 4 | 0 |
| x | Mask of Avacyn | `mask_of_avacyn.rs` | 64 | 4 | 0 |
| x | Mirror-Mad Phantasm | `mirror_mad_phantasm.rs` | 108 | 3 | 0 |
| x | Nephalia Drownyard | `nephalia_drownyard.rs` | 61 | 4 | 0 |
| x | Ashmouth Hound | `ashmouth_hound.rs` | 55 | 3 | 2 |
| x | Ranger's Guile | `rangers_guile.rs` | 58 | 4 | 0 |
| x | Burning Vengeance | `burning_vengeance.rs` | 74 | 3 | 1 |
| x | Dissipate | `dissipate.rs` | 53 | 4 | 0 |
| x | Runic Repetition | `runic_repetition.rs` | 53 | 4 | 0 |
| x | Bramblecrush | `bramblecrush.rs` | 50 | 4 | 0 |
| x | Ancient Grudge | `ancient_grudge.rs` | 48 | 4 | 0 |
| x | Ghoulraiser | `ghoulraiser.rs` | 67 | 3 | 1 |
| x | Naturalize | `naturalize.rs` | 47 | 4 | 0 |
| x | Smite the Monstrous | `smite_the_monstrous.rs` | 47 | 4 | 0 |
| x | Urgent Exorcism | `urgent_exorcism.rs` | 47 | 4 | 0 |
| x | Rebuke | `rebuke.rs` | 46 | 4 | 0 |
| x | Victim of Night | `victim_of_night.rs` | 45 | 4 | 0 |
| x | Unbreathing Horde | `unbreathing_horde.rs` | 89 | 3 | 0 |
| x | Bump in the Night | `bump_in_the_night.rs` | 43 | 4 | 0 |
| x | Mindshrieker | `mindshrieker.rs` | 88 | 3 | 0 |
| x | Wreath of Geists | `wreath_of_geists.rs` | 41 | 4 | 0 |
| x | Cellar Door | `cellar_door.rs` | 84 | 3 | 0 |
| x | Geistcatcher's Rig | `geistcatchers_rig.rs` | 58 | 3 | 1 |
| x | Stitcher's Apprentice | `stitchers_apprentice.rs` | 82 | 3 | 0 |
| x | Full Moon's Rise | `full_moons_rise.rs` | 81 | 3 | 0 |
| x | Slayer of the Wicked | `slayer_of_the_wicked.rs` | 56 | 3 | 1 |
| x | Sturmgeist | `sturmgeist.rs` | 53 | 3 | 1 |
| x | Memory's Journey | `memorys_journey.rs` | 77 | 3 | 0 |
| x | Crossway Vampire | `crossway_vampire.rs` | 47 | 3 | 1 |
| x | Tree of Redemption | `tree_of_redemption.rs` | 72 | 3 | 0 |
| x | Village Bell-Ringer | `village_bell_ringer.rs` | 47 | 3 | 1 |
| x | Elder of Laurels | `elder_of_laurels.rs` | 71 | 3 | 0 |
| x | Traveler's Amulet | `travelers_amulet.rs` | 70 | 3 | 0 |
| x | Armored Skaab | `armored_skaab.rs` | 43 | 3 | 1 |
| x | Geist of Saint Traft | `geist_of_saint_traft.rs` | 86 | 2 | 1 |
| x | Moldgraf Monstrosity | `moldgraf_monstrosity.rs` | 84 | 2 | 1 |
| x | Nevermore | `nevermore.rs` | 63 | 3 | 0 |
| x | Disciple of Griselbrand | `disciple_of_griselbrand.rs` | 62 | 3 | 0 |
| x | Skirsdag Cultist | `skirsdag_cultist.rs` | 62 | 3 | 0 |
| x | Kessig Cagebreakers | `kessig_cagebreakers.rs` | 80 | 2 | 1 |
| x | Blasphemous Act | `blasphemous_act.rs` | 59 | 3 | 0 |
| x | Deranged Assistant | `deranged_assistant.rs` | 59 | 3 | 0 |
| x | Gutter Grime | `gutter_grime.rs` | 79 | 2 | 1 |
| x | Traitorous Blood | `traitorous_blood.rs` | 59 | 3 | 0 |
| x | Selfless Cathar | `selfless_cathar.rs` | 58 | 3 | 0 |
| x | Dearly Departed | `dearly_departed.rs` | 57 | 3 | 0 |
| x | Darkthicket Wolf | `darkthicket_wolf.rs` | 56 | 3 | 0 |
| x | Feral Ridgewolf | `feral_ridgewolf.rs` | 56 | 3 | 0 |
| x | Sever the Bloodline | `sever_the_bloodline.rs` | 56 | 3 | 0 |
| x | Harvest Pyre | `harvest_pyre.rs` | 54 | 3 | 0 |
| x | Kessig Wolf | `kessig_wolf.rs` | 54 | 3 | 0 |
| x | Manor Skeleton | `manor_skeleton.rs` | 54 | 3 | 0 |
| x | Silverchase Fox | `silverchase_fox.rs` | 53 | 3 | 0 |
| x | Moonmist | `moonmist.rs` | 97 | 2 | 0 |
| x | Bonds of Faith | `bonds_of_faith.rs` | 51 | 3 | 0 |
| x | Devil's Play | `devils_play.rs` | 51 | 3 | 0 |
| x | Lantern Spirit | `lantern_spirit.rs` | 51 | 3 | 0 |
| x | Spidery Grasp | `spidery_grasp.rs` | 51 | 3 | 0 |
| x | Ghoulcaller's Bell | `ghoulcallers_bell.rs` | 49 | 3 | 0 |
| x | Prey Upon | `prey_upon.rs` | 49 | 3 | 0 |
| x | Cackling Counterpart | `cackling_counterpart.rs` | 47 | 3 | 0 |
| x | Moment of Heroism | `moment_of_heroism.rs` | 47 | 3 | 0 |
| x | Grasp of Phantoms | `grasp_of_phantoms.rs` | 46 | 3 | 0 |
| x | Balefire Dragon | `balefire_dragon.rs` | 65 | 2 | 1 |
| x | Corpse Lunge | `corpse_lunge.rs` | 44 | 3 | 0 |
| x | Curse of Death's Hold | `curse_of_deaths_hold.rs` | 43 | 3 | 0 |
| x | Curse of the Nightly Hunt | `curse_of_the_nightly_hunt.rs` | 40 | 3 | 0 |
| x | Feeling of Dread | `feeling_of_dread.rs` | 40 | 3 | 0 |
| x | Nightbird's Clutches | `nightbirds_clutches.rs` | 39 | 3 | 0 |
| x | Unburial Rites | `unburial_rites.rs` | 39 | 3 | 0 |
| x | Purify the Grave | `purify_the_grave.rs` | 38 | 3 | 0 |
| x | Travel Preparations | `travel_preparations.rs` | 38 | 3 | 0 |
| x | Abattoir Ghoul | `abattoir_ghoul.rs` | 57 | 2 | 1 |
| x | Ghostly Possession | `ghostly_possession.rs` | 37 | 3 | 0 |
| x | Rage Thrower | `rage_thrower.rs` | 57 | 2 | 1 |
| x | Spectral Flight | `spectral_flight.rs` | 36 | 3 | 0 |
| x | Furor of the Bitten | `furor_of_the_bitten.rs` | 35 | 3 | 0 |
| x | Silent Departure | `silent_departure.rs` | 35 | 3 | 0 |
| x | Dead Weight | `dead_weight.rs` | 34 | 3 | 0 |
| x | Gruesome Deformity | `gruesome_deformity.rs` | 34 | 3 | 0 |
| x | Sensory Deprivation | `sensory_deprivation.rs` | 34 | 3 | 0 |
|   | Brimstone Volley | `brimstone_volley.rs` | 33 | 3 | 0 |
|   | Dream Twist | `dream_twist.rs` | 33 | 3 | 0 |
|   | Galvanic Juggernaut | `galvanic_juggernaut.rs` | 53 | 2 | 1 |
|   | Rakish Heir | `rakish_heir.rs` | 53 | 2 | 1 |
|   | Creepy Doll | `creepy_doll.rs` | 51 | 2 | 1 |
|   | Geistflame | `geistflame.rs` | 31 | 3 | 0 |
|   | Champion of the Parish | `champion_of_the_parish.rs` | 49 | 2 | 1 |
|   | Past in Flames | `past_in_flames.rs` | 73 | 2 | 0 |
|   | Mulch | `mulch.rs` | 70 | 2 | 0 |
|   | Pitchburn Devils | `pitchburn_devils.rs` | 45 | 2 | 1 |
|   | Stromkirk Noble | `stromkirk_noble.rs` | 45 | 2 | 1 |
|   | Unruly Mob | `unruly_mob.rs` | 44 | 2 | 1 |
|   | Bloodcrazed Neonate | `bloodcrazed_neonate.rs` | 43 | 2 | 1 |
|   | Mausoleum Guard | `mausoleum_guard.rs` | 43 | 2 | 1 |
|   | Village Cannibals | `village_cannibals.rs` | 43 | 2 | 1 |
|   | Lumberknot | `lumberknot.rs` | 42 | 2 | 1 |
|   | Falkenrath Marauders | `falkenrath_marauders.rs` | 41 | 2 | 1 |
|   | Doomed Traveler | `doomed_traveler.rs` | 39 | 2 | 1 |
|   | Stromkirk Patrol | `stromkirk_patrol.rs` | 39 | 2 | 1 |
|   | Forbidden Alchemy | `forbidden_alchemy.rs` | 58 | 2 | 0 |
|   | Paraselene | `paraselene.rs` | 58 | 2 | 0 |
|   | Shimmering Grotto | `shimmering_grotto.rs` | 58 | 2 | 0 |
|   | Army of the Damned | `army_of_the_damned.rs` | 55 | 2 | 0 |
|   | Essence of the Wild | `essence_of_the_wild.rs` | 55 | 2 | 0 |
|   | Make a Wish | `make_a_wish.rs` | 55 | 2 | 0 |
|   | Creeping Renaissance | `creeping_renaissance.rs` | 54 | 2 | 0 |
|   | Laboratory Maniac | `laboratory_maniac.rs` | 54 | 2 | 0 |
|   | Spare from Evil | `spare_from_evil.rs` | 53 | 2 | 0 |
|   | Vampiric Fury | `vampiric_fury.rs` | 50 | 2 | 0 |
|   | Hysterical Blindness | `hysterical_blindness.rs` | 48 | 2 | 0 |
|   | Festerhide Boar | `festerhide_boar.rs` | 46 | 2 | 0 |
|   | Somberwald Spider | `somberwald_spider.rs` | 43 | 2 | 0 |
|   | Parallel Lives | `parallel_lives.rs` | 42 | 2 | 0 |
|   | Spider Spawning | `spider_spawning.rs` | 42 | 2 | 0 |
|   | Diregraf Ghoul | `diregraf_ghoul.rs` | 41 | 2 | 0 |
|   | Rally the Peasants | `rally_the_peasants.rs` | 41 | 2 | 0 |
|   | Desperate Ravings | `desperate_ravings.rs` | 40 | 2 | 0 |
|   | Rolling Temblor | `rolling_temblor.rs` | 38 | 2 | 0 |
|   | Scourge of Geier Reach | `scourge_of_geier_reach.rs` | 38 | 2 | 0 |
|   | Avacyn's Pilgrim | `avacyns_pilgrim.rs` | 36 | 2 | 0 |
|   | Boneyard Wurm | `boneyard_wurm.rs` | 36 | 2 | 0 |
|   | Gnaw to the Bone | `gnaw_to_the_bone.rs` | 36 | 2 | 0 |
|   | Altar's Reap | `altars_reap.rs` | 35 | 2 | 0 |
|   | Infernal Plunge | `infernal_plunge.rs` | 34 | 2 | 0 |
|   | Moan of the Unhallowed | `moan_of_the_unhallowed.rs` | 34 | 2 | 0 |
|   | Midnight Haunting | `midnight_haunting.rs` | 32 | 2 | 0 |
|   | Skaab Ruinator | `skaab_ruinator.rs` | 31 | 2 | 0 |
|   | Think Twice | `think_twice.rs` | 29 | 2 | 0 |
|   | Angelic Overseer | `angelic_overseer.rs` | 37 | 1 | 0 |
|   | Battleground Geist | `battleground_geist.rs` | 35 | 1 | 0 |
|   | Gallows Warden | `gallows_warden.rs` | 35 | 1 | 0 |
|   | Heartless Summoning | `heartless_summoning.rs` | 33 | 1 | 0 |
|   | Orchard Spirit | `orchard_spirit.rs` | 33 | 1 | 0 |
|   | Elite Inquisitor | `elite_inquisitor.rs` | 31 | 1 | 0 |
|   | Night Revelers | `night_revelers.rs` | 30 | 1 | 0 |
|   | Grave Bramble | `grave_bramble.rs` | 28 | 1 | 0 |
|   | Stitched Drake | `stitched_drake.rs` | 28 | 1 | 0 |
|   | Invisible Stalker | `invisible_stalker.rs` | 27 | 1 | 0 |
|   | One-Eyed Scarecrow | `one_eyed_scarecrow.rs` | 27 | 1 | 0 |
|   | Rooftop Storm | `rooftop_storm.rs` | 27 | 1 | 0 |
|   | Skaab Goliath | `skaab_goliath.rs` | 27 | 1 | 0 |
|   | Vampire Interloper | `vampire_interloper.rs` | 27 | 1 | 0 |
|   | Makeshift Mauler | `makeshift_mauler.rs` | 26 | 1 | 0 |
|   | Chapel Geist | `chapel_geist.rs` | 25 | 1 | 0 |
|   | Stony Silence | `stony_silence.rs` | 25 | 1 | 0 |
|   | Abbey Griffin | `abbey_griffin.rs` | 24 | 1 | 0 |
|   | Ambush Viper | `ambush_viper.rs` | 24 | 1 | 0 |
|   | Intangible Virtue | `intangible_virtue.rs` | 24 | 1 | 0 |
|   | Kindercatch | `kindercatch.rs` | 24 | 1 | 0 |
|   | Markov Patrician | `markov_patrician.rs` | 24 | 1 | 0 |
|   | Moon Heron | `moon_heron.rs` | 24 | 1 | 0 |
|   | Spectral Rider | `spectral_rider.rs` | 24 | 1 | 0 |
|   | Voiceless Spirit | `voiceless_spirit.rs` | 24 | 1 | 0 |
|   | Typhoid Rats | `typhoid_rats.rs` | 23 | 1 | 0 |
|   | Fortress Crab | `fortress_crab.rs` | 22 | 1 | 0 |
|   | Riot Devils | `riot_devils.rs` | 22 | 1 | 0 |
|   | Rotting Fensnake | `rotting_fensnake.rs` | 22 | 1 | 0 |
|   | Thraben Purebloods | `thraben_purebloods.rs` | 22 | 1 | 0 |
|   | Walking Corpse | `walking_corpse.rs` | 22 | 1 | 0 |
