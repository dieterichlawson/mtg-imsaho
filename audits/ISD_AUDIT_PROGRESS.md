# Innistrad card audit — progress tracker

Every card in `mtg-engine/src/cards/isd/` audited against
`.claude/commands/check-card-procedure.md`. Per-card findings are appended to
`audits/{card_file}.md`; this file is the checklist.

**Tier** is a time budget, not a verdict. Score = lines + 45/behaviour-hook +
25/declared-trigger + 70 if double-faced; any DFC or any card with 5+ hooks is
Tier A regardless.

> An earlier tiering under-counted hooks and put 48 cards with real behaviour
> into the vanilla tier. Re-derived with the full hook set.

**187/249 audited.**

## Verified across all 249 at once

Procedure steps that are exhaustive rather than per-card. Every card below
inherits these; its own entry covers the rest.

| check | result |
|---|---|
| mana cost, card types, supertypes, subtypes, P/T, oracle text | 249/249 exact |
| keywords, both faces | 249/249 complete, none invented |
| flashback costs | 249/249 exact |
| trigger kinds vs. oracle phrasing | 249/249 consistent |
| step 9 anti-pattern scan | 96 candidates raised, 3 real, all fixed |

## Tier A — complex (DFCs, planeswalkers, 5+ hooks) — 28/28

| ✓ | card | file | loc | hooks | trig | dfc |
|---|---|---|---|---|---|---|
| x | Civilized Scholar | `civilized_scholar.rs` | 248 | 9 | 4 | yes |
| x | Garruk Relentless | `garruk_relentless.rs` | 300 | 7 | 0 |  |
| x | Mayor of Avabruck | `mayor_of_avabruck.rs` | 143 | 6 | 4 | yes |
| x | Daybreak Ranger | `daybreak_ranger.rs` | 143 | 6 | 3 | back-face P/T restated per card — fixed set-wide |
| x | Wooden Stake | `wooden_stake.rs` | 107 | 8 | 3 |  |
| x | Cloistered Youth | `cloistered_youth.rs` | 126 | 6 | 3 | yes |
| x | Ulvenwald Mystics | `ulvenwald_mystics.rs` | 113 | 6 | 3 | yes |
| x | Screeching Bat | `screeching_bat.rs` | 153 | 5 | 3 | yes |
| x | Delver of Secrets | `delver_of_secrets.rs` | 143 | 5 | 2 | yes |
| x | Grimgrin, Corpse-Born | `grimgrin_corpse_born.rs` | 141 | 6 | 2 | tapped itself after entering (CR 614.1c) — fixed |
| x | Liliana of the Veil | `liliana_of_the_veil.rs` | 275 | 4 | 0 |  |
| x | Instigator Gang | `instigator_gang.rs` | 115 | 4 | 3 | yes |
| x | Kruin Outlaw | `kruin_outlaw.rs` | 98 | 4 | 3 | yes |
| x | Trepanation Blade | `trepanation_blade.rs` | 145 | 5 | 2 | mill bypassed the pipeline — fixed in `move_object` |
| x | Hanweir Watchkeep | `hanweir_watchkeep.rs` | 88 | 4 | 3 | yes |
| x | Village Ironsmith | `village_ironsmith.rs` | 86 | 4 | 3 | yes |
| x | Curse of Oblivion | `curse_of_oblivion.rs` | 135 | 5 | 2 | counted tokens as cards (CR 109.1) — fixed |
| x | Gatstaf Shepherd | `gatstaf_shepherd.rs` | 85 | 4 | 3 | yes |
| x | Grizzled Outcasts | `grizzled_outcasts.rs` | 84 | 4 | 3 | yes |
| x | Tormented Pariah | `tormented_pariah.rs` | 84 | 4 | 3 | yes |
| x | Villagers of Estwald | `villagers_of_estwald.rs` | 84 | 4 | 3 | yes |
| x | Reckless Waif | `reckless_waif.rs` | 83 | 4 | 3 | yes |
| x | Bloodline Keeper | `bloodline_keeper.rs` | 156 | 4 | 0 | yes |
| x | Evil Twin | `evil_twin.rs` | 124 | 5 | 2 |  |
| x | Thraben Sentry | `thraben_sentry.rs` | 94 | 4 | 2 | yes |
| x | Ludevic's Test Subject | `ludevics_test_subject.rs` | 108 | 4 | 0 | yes |
| x | Curiosity | `curiosity.rs` | 80 | 5 | 2 |  |
| x | Runechanter's Pike | `runechanters_pike.rs` | 91 | 5 | 0 |  |

## Tier B — moderate (triggered or activated abilities) — 93/155

| ✓ | card | file | loc | hooks | trig | dfc |
|---|---|---|---|---|---|---|
| x | Charmbreaker Devils | `charmbreaker_devils.rs` | 119 | 4 | 3 |  |
| x | Bitterheart Witch | `bitterheart_witch.rs` | 176 | 3 | 2 | targeted trigger declared untargeted (CR 603.3d) — fixed |
| x | Grimoire of the Dead | `grimoire_of_the_dead.rs` | 176 | 4 | 0 |  |
|   | Fiend Hunter | `fiend_hunter.rs` | 92 | 4 | 3 |  |
| x | Curse of the Pierced Heart | `curse_of_the_pierced_heart.rs` | 110 | 4 | 2 | hand-written life change bypassed the damage pipeline — fixed |
| x | Olivia Voldaren | `olivia_voldaren.rs` | 151 | 3 | 2 |  |
| x | Graveyard Shovel | `graveyard_shovel.rs` | 132 | 4 | 0 | counted tokens as cards (CR 109.1) — fixed |
| x | Blazing Torch | `blazing_torch.rs` | 129 | 4 | 0 | sacrifice is a cost — `pay_activation_cost` |
|   | Moorland Haunt | `moorland_haunt.rs` | 120 | 4 | 0 |  |
| x | Curse of the Bloody Tome | `curse_of_the_bloody_tome.rs` | 67 | 4 | 2 |  |
|   | Morkrut Banshee | `morkrut_banshee.rs` | 67 | 4 | 2 |  |
| x | Claustrophobia | `claustrophobia.rs` | 57 | 4 | 2 |  |
|   | Mentor of the Meek | `mentor_of_the_meek.rs` | 102 | 3 | 2 |  |
|   | Skirsdag High Priest | `skirsdag_high_priest.rs` | 148 | 3 | 0 |  |
| x | Reaper from the Abyss | `reaper_from_the_abyss.rs` | 89 | 3 | 2 |  |
|   | Caravan Vigil | `caravan_vigil.rs` | 137 | 3 | 0 |  |
|   | Tribute to Hunger | `tribute_to_hunger.rs` | 91 | 4 | 0 |  |
|   | Brain Weevil | `brain_weevil.rs` | 132 | 3 | 0 |  |
| x | Demonmail Hauberk | `demonmail_hauberk.rs` | 87 | 4 | 0 | deviation recorded: a legal fizzling equip is not offered |
| x | Silver-Inlaid Dagger | `silver_inlaid_dagger.rs` | 86 | 4 | 0 |  |
| x | Butcher's Cleaver | `butchers_cleaver.rs` | 85 | 4 | 0 |  |
| x | Sharpened Pitchfork | `sharpened_pitchfork.rs` | 84 | 4 | 0 |  |
| x | Woodland Sleuth | `woodland_sleuth.rs` | 79 | 3 | 2 |  |
| x | Back from the Brink | `back_from_the_brink.rs` | 127 | 3 | 0 |  |
|   | Mikaeus, the Lunarch | `mikaeus_the_lunarch.rs` | 125 | 3 | 0 |  |
| x | Splinterfright | `splinterfright.rs` | 72 | 3 | 2 |  |
| x | Inquisitor's Flail | `inquisitors_flail.rs` | 75 | 4 | 0 |  |
| x | Cobbled Wings | `cobbled_wings.rs` | 74 | 4 | 0 |  |
|   | Hamlet Captain | `hamlet_captain.rs` | 89 | 2 | 3 |  |
|   | Snapcaster Mage | `snapcaster_mage.rs` | 69 | 3 | 2 |  |
| x | Angel of Flight Alabaster | `angel_of_flight_alabaster.rs` | 68 | 3 | 2 |  |
|   | Bloodgift Demon | `bloodgift_demon.rs` | 67 | 3 | 2 |  |
| x | Mask of Avacyn | `mask_of_avacyn.rs` | 69 | 4 | 0 |  |
| x | Hollowhenge Scavenger | `hollowhenge_scavenger.rs` | 61 | 3 | 2 |  |
|   | Witchbane Orb | `witchbane_orb.rs` | 61 | 3 | 2 |  |
|   | Skeletal Grimace | `skeletal_grimace.rs` | 65 | 4 | 0 |  |
|   | Geist-Honored Monk | `geist_honored_monk.rs` | 57 | 3 | 2 |  |
| x | Curse of Stalked Prey | `curse_of_stalked_prey.rs` | 56 | 3 | 2 |  |
|   | Divine Reckoning | `divine_reckoning.rs` | 147 | 2 | 0 |  |
| x | Falkenrath Noble | `falkenrath_noble.rs` | 72 | 2 | 3 |  |
| x | Selhoff Occultist | `selhoff_occultist.rs` | 71 | 2 | 3 |  |
|   | Geist of Saint Traft | `geist_of_saint_traft.rs` | 94 | 2 | 2 |  |
| x | Ghost Quarter | `ghost_quarter.rs` | 99 | 3 | 0 | CR 602.2a + CR 608.2b for abilities — fixed |
|   | Murder of Crows | `murder_of_crows.rs` | 89 | 2 | 2 |  |
|   | Ashmouth Hound | `ashmouth_hound.rs` | 56 | 2 | 3 |  |
|   | Undead Alchemist | `undead_alchemist.rs` | 102 | 2 | 1 |  |
|   | Elder Cathar | `elder_cathar.rs` | 76 | 2 | 2 |  |
|   | Burning Vengeance | `burning_vengeance.rs` | 75 | 2 | 2 |  |
| x | Manor Gargoyle | `manor_gargoyle.rs` | 80 | 3 | 0 |  |
| x | Into the Maw of Hell | `into_the_maw_of_hell.rs` | 79 | 3 | 0 |  |
| x | Endless Ranks of the Dead | `endless_ranks_of_the_dead.rs` | 73 | 2 | 2 |  |
|   | Kessig Wolf Run | `kessig_wolf_run.rs` | 76 | 3 | 0 |  |
|   | Night Terrors | `night_terrors.rs` | 76 | 3 | 0 |  |
| x | Avacynian Priest | `avacynian_priest.rs` | 73 | 3 | 0 | CR 608.2b for a card's own targeting restriction — fixed |
| x | Ghoulraiser | `ghoulraiser.rs` | 68 | 2 | 2 |  |
|   | Maw of the Mire | `maw_of_the_mire.rs` | 73 | 3 | 0 |  |
| x | Nevermore | `nevermore.rs` | 68 | 2 | 2 |  |
| x | Gavony Township | `gavony_township.rs` | 70 | 3 | 0 |  |
| x | Lost in the Mist | `lost_in_the_mist.rs` | 67 | 3 | 0 |  |
| x | Stensia Bloodhall | `stensia_bloodhall.rs` | 67 | 3 | 0 |  |
|   | Frightful Delusion | `frightful_delusion.rs` | 66 | 3 | 0 |  |
| x | Geistcatcher's Rig | `geistcatchers_rig.rs` | 59 | 2 | 2 |  |
| x | Heretic's Punishment | `heretics_punishment.rs` | 109 | 2 | 0 | mill bypassed `mill_one` — fixed; CR 602.2a |
|   | Mirror-Mad Phantasm | `mirror_mad_phantasm.rs` | 109 | 2 | 0 |  |
| x | Ghoulcaller's Chant | `ghoulcallers_chant.rs` | 63 | 3 | 0 |  |
|   | Nephalia Drownyard | `nephalia_drownyard.rs` | 62 | 3 | 0 |  |
| x | Slayer of the Wicked | `slayer_of_the_wicked.rs` | 57 | 2 | 2 |  |
|   | Sturmgeist | `sturmgeist.rs` | 54 | 2 | 2 |  |
|   | Ranger's Guile | `rangers_guile.rs` | 57 | 3 | 0 |  |
| x | Crossway Vampire | `crossway_vampire.rs` | 48 | 2 | 2 |  |
| x | Village Bell-Ringer | `village_bell_ringer.rs` | 48 | 2 | 2 |  |
| x | Dissipate | `dissipate.rs` | 52 | 3 | 0 |  |
|   | Runic Repetition | `runic_repetition.rs` | 52 | 3 | 0 |  |
|   | Bump in the Night | `bump_in_the_night.rs` | 50 | 3 | 0 |  |
| x | Armored Skaab | `armored_skaab.rs` | 44 | 2 | 2 |  |
| x | Bramblecrush | `bramblecrush.rs` | 49 | 3 | 0 |  |
| x | Ancient Grudge | `ancient_grudge.rs` | 47 | 3 | 0 |  |
|   | Naturalize | `naturalize.rs` | 46 | 3 | 0 |  |
|   | Smite the Monstrous | `smite_the_monstrous.rs` | 46 | 3 | 0 |  |
|   | Urgent Exorcism | `urgent_exorcism.rs` | 46 | 3 | 0 |  |
|   | Rebuke | `rebuke.rs` | 45 | 3 | 0 |  |
| x | Mindshrieker | `mindshrieker.rs` | 89 | 2 | 0 |  |
|   | Victim of Night | `victim_of_night.rs` | 44 | 3 | 0 |  |
| x | Moldgraf Monstrosity | `moldgraf_monstrosity.rs` | 82 | 1 | 2 |  |
| x | Wreath of Geists | `wreath_of_geists.rs` | 42 | 3 | 0 |  |
| x | Kessig Cagebreakers | `kessig_cagebreakers.rs` | 81 | 1 | 2 |  |
| x | Cellar Door | `cellar_door.rs` | 85 | 2 | 0 |  |
| x | Gutter Grime | `gutter_grime.rs` | 80 | 1 | 2 |  |
| x | Stitcher's Apprentice | `stitchers_apprentice.rs` | 83 | 2 | 0 |  |
|   | Full Moon's Rise | `full_moons_rise.rs` | 82 | 2 | 0 |  |
|   | Memory's Journey | `memorys_journey.rs` | 78 | 2 | 0 |  |
|   | Tree of Redemption | `tree_of_redemption.rs` | 76 | 2 | 0 |  |
| x | Clifftop Retreat | `clifftop_retreat.rs` | 74 | 2 | 0 |  |
| x | Hinterland Harbor | `hinterland_harbor.rs` | 74 | 2 | 0 |  |
| x | Isolated Chapel | `isolated_chapel.rs` | 74 | 2 | 0 |  |
| x | Sulfur Falls | `sulfur_falls.rs` | 74 | 2 | 0 |  |
| x | Woodland Cemetery | `woodland_cemetery.rs` | 74 | 2 | 0 |  |
| x | Elder of Laurels | `elder_of_laurels.rs` | 72 | 2 | 0 | counted creatures at announcement — fixed by CR 602.2a |
| x | Balefire Dragon | `balefire_dragon.rs` | 66 | 1 | 2 |  |
| x | Traveler's Amulet | `travelers_amulet.rs` | 71 | 2 | 0 |  |
| x | Abattoir Ghoul | `abattoir_ghoul.rs` | 61 | 1 | 2 |  |
| x | Disciple of Griselbrand | `disciple_of_griselbrand.rs` | 66 | 2 | 0 |  |
| x | Rage Thrower | `rage_thrower.rs` | 58 | 1 | 2 |  |
|   | Deranged Assistant | `deranged_assistant.rs` | 60 | 2 | 0 |  |
| x | Selfless Cathar | `selfless_cathar.rs` | 60 | 2 | 0 |  |
| x | Skirsdag Cultist | `skirsdag_cultist.rs` | 60 | 2 | 0 |  |
|   | Traitorous Blood | `traitorous_blood.rs` | 60 | 2 | 0 |  |
| x | Galvanic Juggernaut | `galvanic_juggernaut.rs` | 54 | 1 | 2 |  |
|   | Rakish Heir | `rakish_heir.rs` | 54 | 1 | 2 |  |
|   | Creepy Doll | `creepy_doll.rs` | 52 | 1 | 2 |  |
| x | Darkthicket Wolf | `darkthicket_wolf.rs` | 57 | 2 | 0 |  |
| x | Feral Ridgewolf | `feral_ridgewolf.rs` | 57 | 2 | 0 |  |
| x | Sever the Bloodline | `sever_the_bloodline.rs` | 57 | 2 | 0 |  |
|   | Champion of the Parish | `champion_of_the_parish.rs` | 50 | 1 | 2 |  |
| x | Harvest Pyre | `harvest_pyre.rs` | 55 | 2 | 0 |  |
|   | Kessig Wolf | `kessig_wolf.rs` | 55 | 2 | 0 |  |
|   | Manor Skeleton | `manor_skeleton.rs` | 55 | 2 | 0 |  |
|   | Silverchase Fox | `silverchase_fox.rs` | 54 | 2 | 0 |  |
| x | Bonds of Faith | `bonds_of_faith.rs` | 52 | 2 | 0 |  |
| x | Devil's Play | `devils_play.rs` | 52 | 2 | 0 |  |
|   | Lantern Spirit | `lantern_spirit.rs` | 52 | 2 | 0 |  |
|   | Spidery Grasp | `spidery_grasp.rs` | 52 | 2 | 0 |  |
| x | Pitchburn Devils | `pitchburn_devils.rs` | 46 | 1 | 2 |  |
| x | Stromkirk Noble | `stromkirk_noble.rs` | 46 | 1 | 2 |  |
| x | Ghoulcaller's Bell | `ghoulcallers_bell.rs` | 50 | 2 | 0 |  |
| x | Prey Upon | `prey_upon.rs` | 50 | 2 | 0 |  |
| x | Unruly Mob | `unruly_mob.rs` | 45 | 1 | 2 |  |
| x | Bloodcrazed Neonate | `bloodcrazed_neonate.rs` | 44 | 1 | 2 |  |
| x | Mausoleum Guard | `mausoleum_guard.rs` | 44 | 1 | 2 |  |
| x | Village Cannibals | `village_cannibals.rs` | 44 | 1 | 2 |  |
|   | Cackling Counterpart | `cackling_counterpart.rs` | 48 | 2 | 0 |  |
| x | Lumberknot | `lumberknot.rs` | 43 | 1 | 2 |  |
|   | Moment of Heroism | `moment_of_heroism.rs` | 48 | 2 | 0 |  |
| x | Falkenrath Marauders | `falkenrath_marauders.rs` | 42 | 1 | 2 |  |
|   | Grasp of Phantoms | `grasp_of_phantoms.rs` | 47 | 2 | 0 |  |
|   | Corpse Lunge | `corpse_lunge.rs` | 45 | 2 | 0 |  |
| x | Doomed Traveler | `doomed_traveler.rs` | 40 | 1 | 2 |  |
| x | Stromkirk Patrol | `stromkirk_patrol.rs` | 40 | 1 | 2 |  |
| x | Curse of Death's Hold | `curse_of_deaths_hold.rs` | 44 | 2 | 0 |  |
| x | Curse of the Nightly Hunt | `curse_of_the_nightly_hunt.rs` | 41 | 2 | 0 |  |
|   | Feeling of Dread | `feeling_of_dread.rs` | 41 | 2 | 0 |  |
|   | Nightbird's Clutches | `nightbirds_clutches.rs` | 40 | 2 | 0 |  |
|   | Unburial Rites | `unburial_rites.rs` | 40 | 2 | 0 |  |
|   | Purify the Grave | `purify_the_grave.rs` | 39 | 2 | 0 |  |
| x | Travel Preparations | `travel_preparations.rs` | 39 | 2 | 0 |  |
| x | Ghostly Possession | `ghostly_possession.rs` | 38 | 2 | 0 |  |
| x | Spectral Flight | `spectral_flight.rs` | 37 | 2 | 0 |  |
| x | Furor of the Bitten | `furor_of_the_bitten.rs` | 36 | 2 | 0 |  |
|   | Silent Departure | `silent_departure.rs` | 36 | 2 | 0 |  |
| x | Dead Weight | `dead_weight.rs` | 35 | 2 | 0 |  |
| x | Gruesome Deformity | `gruesome_deformity.rs` | 35 | 2 | 0 |  |
| x | Sensory Deprivation | `sensory_deprivation.rs` | 35 | 2 | 0 |  |
|   | Brimstone Volley | `brimstone_volley.rs` | 34 | 2 | 0 |  |
|   | Dream Twist | `dream_twist.rs` | 34 | 2 | 0 |  |
|   | Geistflame | `geistflame.rs` | 32 | 2 | 0 |  |

## Tier C — light (one behaviour hook) — 38/38

| ✓ | card | file | loc | hooks | trig | dfc |
|---|---|---|---|---|---|---|
| x | Moonmist | `moonmist.rs` | 98 | 1 | 0 |  |
| x | Unbreathing Horde | `unbreathing_horde.rs` | 90 | 1 | 0 |  |
| x | Past in Flames | `past_in_flames.rs` | 73 | 1 | 0 |  |
| x | Mulch | `mulch.rs` | 71 | 1 | 0 |  |
| x | Paraselene | `paraselene.rs` | 66 | 1 | 0 |  |
| x | Blasphemous Act | `blasphemous_act.rs` | 60 | 1 | 0 |  |
| x | Forbidden Alchemy | `forbidden_alchemy.rs` | 59 | 1 | 0 |  |
| x | Shimmering Grotto | `shimmering_grotto.rs` | 59 | 1 | 0 |  |
| x | Dearly Departed | `dearly_departed.rs` | 58 | 1 | 0 |  |
| x | Army of the Damned | `army_of_the_damned.rs` | 56 | 1 | 0 |  |
| x | Essence of the Wild | `essence_of_the_wild.rs` | 56 | 1 | 0 |  |
| x | Make a Wish | `make_a_wish.rs` | 56 | 1 | 0 |  |
| x | Creeping Renaissance | `creeping_renaissance.rs` | 55 | 1 | 0 |  |
| x | Laboratory Maniac | `laboratory_maniac.rs` | 55 | 1 | 0 |  |
| x | Hysterical Blindness | `hysterical_blindness.rs` | 49 | 1 | 0 |  |
| x | Vampiric Fury | `vampiric_fury.rs` | 49 | 1 | 0 |  |
| x | Festerhide Boar | `festerhide_boar.rs` | 47 | 1 | 0 |  |
| x | Gnaw to the Bone | `gnaw_to_the_bone.rs` | 45 | 1 | 0 |  |
| x | Somberwald Spider | `somberwald_spider.rs` | 44 | 1 | 0 |  |
| x | Parallel Lives | `parallel_lives.rs` | 43 | 1 | 0 |  |
| x | Spider Spawning | `spider_spawning.rs` | 43 | 1 | 0 |  |
| x | Spare from Evil | `spare_from_evil.rs` | 42 | 1 | 0 |  |
| x | Desperate Ravings | `desperate_ravings.rs` | 41 | 1 | 0 |  |
| x | Skaab Ruinator | `skaab_ruinator.rs` | 40 | 1 | 0 |  |
| x | Rally the Peasants | `rally_the_peasants.rs` | 39 | 1 | 0 |  |
| x | Rolling Temblor | `rolling_temblor.rs` | 39 | 1 | 0 |  |
| x | Scourge of Geier Reach | `scourge_of_geier_reach.rs` | 38 | 1 | 0 |  |
| x | Avacyn's Pilgrim | `avacyns_pilgrim.rs` | 37 | 1 | 0 |  |
| x | Boneyard Wurm | `boneyard_wurm.rs` | 37 | 1 | 0 |  |
| x | Stitched Drake | `stitched_drake.rs` | 37 | 1 | 0 |  |
| x | Altar's Reap | `altars_reap.rs` | 36 | 1 | 0 |  |
| x | Infernal Plunge | `infernal_plunge.rs` | 35 | 1 | 0 |  |
| x | Moan of the Unhallowed | `moan_of_the_unhallowed.rs` | 35 | 1 | 0 |  |
| x | Skaab Goliath | `skaab_goliath.rs` | 35 | 1 | 0 |  |
| x | Diregraf Ghoul | `diregraf_ghoul.rs` | 34 | 1 | 0 |  |
| x | Makeshift Mauler | `makeshift_mauler.rs` | 34 | 1 | 0 |  |
| x | Midnight Haunting | `midnight_haunting.rs` | 33 | 1 | 0 |  |
| x | Think Twice | `think_twice.rs` | 30 | 1 | 0 |  |

## Tier D — card data and static abilities only — 28/28

| ✓ | card | file | loc | hooks | trig | dfc |
|---|---|---|---|---|---|---|
| x | Angelic Overseer | `angelic_overseer.rs` | 38 | 0 | 0 |  |
| x | Battleground Geist | `battleground_geist.rs` | 36 | 0 | 0 |  |
| x | Gallows Warden | `gallows_warden.rs` | 36 | 0 | 0 |  |
| x | Heartless Summoning | `heartless_summoning.rs` | 34 | 0 | 0 |  |
| x | Orchard Spirit | `orchard_spirit.rs` | 34 | 0 | 0 |  |
| x | Elite Inquisitor | `elite_inquisitor.rs` | 32 | 0 | 0 |  |
| x | Night Revelers | `night_revelers.rs` | 31 | 0 | 0 |  |
| x | Grave Bramble | `grave_bramble.rs` | 29 | 0 | 0 |  |
| x | Invisible Stalker | `invisible_stalker.rs` | 28 | 0 | 0 |  |
| x | One-Eyed Scarecrow | `one_eyed_scarecrow.rs` | 28 | 0 | 0 |  |
| x | Rooftop Storm | `rooftop_storm.rs` | 28 | 0 | 0 |  |
| x | Vampire Interloper | `vampire_interloper.rs` | 28 | 0 | 0 |  |
| x | Chapel Geist | `chapel_geist.rs` | 26 | 0 | 0 |  |
| x | Stony Silence | `stony_silence.rs` | 26 | 0 | 0 |  |
| x | Abbey Griffin | `abbey_griffin.rs` | 25 | 0 | 0 |  |
| x | Ambush Viper | `ambush_viper.rs` | 25 | 0 | 0 |  |
| x | Intangible Virtue | `intangible_virtue.rs` | 25 | 0 | 0 |  |
| x | Kindercatch | `kindercatch.rs` | 25 | 0 | 0 |  |
| x | Markov Patrician | `markov_patrician.rs` | 25 | 0 | 0 |  |
| x | Moon Heron | `moon_heron.rs` | 25 | 0 | 0 |  |
| x | Spectral Rider | `spectral_rider.rs` | 25 | 0 | 0 |  |
| x | Voiceless Spirit | `voiceless_spirit.rs` | 25 | 0 | 0 |  |
| x | Typhoid Rats | `typhoid_rats.rs` | 24 | 0 | 0 |  |
| x | Fortress Crab | `fortress_crab.rs` | 23 | 0 | 0 |  |
| x | Riot Devils | `riot_devils.rs` | 23 | 0 | 0 |  |
| x | Rotting Fensnake | `rotting_fensnake.rs` | 23 | 0 | 0 |  |
| x | Thraben Purebloods | `thraben_purebloods.rs` | 23 | 0 | 0 |  |
| x | Walking Corpse | `walking_corpse.rs` | 23 | 0 | 0 |  |

