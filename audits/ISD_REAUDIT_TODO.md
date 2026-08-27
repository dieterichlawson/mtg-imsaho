# Re-audit todo — cards checked by set-wide sweep, not by the per-card procedure

The first pass over these 100 cards verified them through exhaustive sweeps
(card data across all 249, anti-pattern scans, registry-derived family checks)
rather than by running `/check-card-procedure` on each one. The sweeps were real
and found real bugs, but they are not the same thing as the procedure: they never
fetched per-card *rulings*, and step 5 — think about tricky interactions — has no
sweep equivalent.

Ordered most complex first, so the complex cards get the most time.

| ✓ | card | file | loc | hooks | trig |
|---|---|---|---|---|---|
| x | Mayor of Avabruck | `mayor_of_avabruck.rs` | 135 | 7 | 3 |
| x | Screeching Bat | `screeching_bat.rs` | 145 | 7 | 2 |
| x | Cloistered Youth | `cloistered_youth.rs` | 115 | 7 | 2 |
| x | Delver of Secrets | `delver_of_secrets.rs` | 135 | 7 | 1 |
| x | Charmbreaker Devils | `charmbreaker_devils.rs` | 119 | 5 | 2 |
| x | Instigator Gang | `instigator_gang.rs` | 107 | 5 | 2 |
| x | Kruin Outlaw | `kruin_outlaw.rs` | 90 | 5 | 2 |
| x | Hanweir Watchkeep | `hanweir_watchkeep.rs` | 80 | 5 | 2 |
| x | Village Ironsmith | `village_ironsmith.rs` | 78 | 5 | 2 |
| x | Gatstaf Shepherd | `gatstaf_shepherd.rs` | 77 | 5 | 2 |
| x | Grizzled Outcasts | `grizzled_outcasts.rs` | 76 | 5 | 2 |
| x | Tormented Pariah | `tormented_pariah.rs` | 76 | 5 | 2 |
| x | Villagers of Estwald | `villagers_of_estwald.rs` | 76 | 5 | 2 |
| x | Reckless Waif | `reckless_waif.rs` | 75 | 5 | 2 |
| x | Back from the Brink | `back_from_the_brink.rs` | 120 | 4 | 0 |
| x | Woodland Sleuth | `woodland_sleuth.rs` | 79 | 4 | 1 |
| x | Splinterfright | `splinterfright.rs` | 71 | 4 | 1 |
| x | Angel of Flight Alabaster | `angel_of_flight_alabaster.rs` | 67 | 4 | 1 |
| x | Into the Maw of Hell | `into_the_maw_of_hell.rs` | 80 | 4 | 0 |
| x | Hollowhenge Scavenger | `hollowhenge_scavenger.rs` | 52 | 4 | 1 |
|   | Falkenrath Noble | `falkenrath_noble.rs` | 71 | 3 | 2 |
|   | Selhoff Occultist | `selhoff_occultist.rs` | 70 | 3 | 2 |
|   | Clifftop Retreat | `clifftop_retreat.rs` | 73 | 4 | 0 |
|   | Hinterland Harbor | `hinterland_harbor.rs` | 73 | 4 | 0 |
|   | Isolated Chapel | `isolated_chapel.rs` | 73 | 4 | 0 |
|   | Sulfur Falls | `sulfur_falls.rs` | 73 | 4 | 0 |
|   | Woodland Cemetery | `woodland_cemetery.rs` | 73 | 4 | 0 |
|   | Lost in the Mist | `lost_in_the_mist.rs` | 68 | 4 | 0 |
|   | Ghoulcaller's Chant | `ghoulcallers_chant.rs` | 65 | 4 | 0 |
|   | Dissipate | `dissipate.rs` | 53 | 4 | 0 |
|   | Endless Ranks of the Dead | `endless_ranks_of_the_dead.rs` | 72 | 3 | 1 |
|   | Bramblecrush | `bramblecrush.rs` | 50 | 4 | 0 |
|   | Ancient Grudge | `ancient_grudge.rs` | 48 | 4 | 0 |
|   | Ghoulraiser | `ghoulraiser.rs` | 67 | 3 | 1 |
|   | Unbreathing Horde | `unbreathing_horde.rs` | 89 | 3 | 0 |
|   | Wreath of Geists | `wreath_of_geists.rs` | 41 | 4 | 0 |
|   | Geistcatcher's Rig | `geistcatchers_rig.rs` | 58 | 3 | 1 |
|   | Slayer of the Wicked | `slayer_of_the_wicked.rs` | 56 | 3 | 1 |
|   | Crossway Vampire | `crossway_vampire.rs` | 47 | 3 | 1 |
|   | Village Bell-Ringer | `village_bell_ringer.rs` | 47 | 3 | 1 |
|   | Armored Skaab | `armored_skaab.rs` | 43 | 3 | 1 |
|   | Moldgraf Monstrosity | `moldgraf_monstrosity.rs` | 84 | 2 | 1 |
|   | Nevermore | `nevermore.rs` | 63 | 3 | 0 |
|   | Kessig Cagebreakers | `kessig_cagebreakers.rs` | 80 | 2 | 1 |
|   | Blasphemous Act | `blasphemous_act.rs` | 59 | 3 | 0 |
|   | Gutter Grime | `gutter_grime.rs` | 79 | 2 | 1 |
|   | Selfless Cathar | `selfless_cathar.rs` | 58 | 3 | 0 |
|   | Dearly Departed | `dearly_departed.rs` | 57 | 3 | 0 |
|   | Sever the Bloodline | `sever_the_bloodline.rs` | 56 | 3 | 0 |
|   | Harvest Pyre | `harvest_pyre.rs` | 54 | 3 | 0 |
|   | Moonmist | `moonmist.rs` | 97 | 2 | 0 |
|   | Devil's Play | `devils_play.rs` | 51 | 3 | 0 |
|   | Prey Upon | `prey_upon.rs` | 49 | 3 | 0 |
|   | Balefire Dragon | `balefire_dragon.rs` | 65 | 2 | 1 |
|   | Abattoir Ghoul | `abattoir_ghoul.rs` | 57 | 2 | 1 |
|   | Rage Thrower | `rage_thrower.rs` | 57 | 2 | 1 |
|   | Galvanic Juggernaut | `galvanic_juggernaut.rs` | 53 | 2 | 1 |
|   | Past in Flames | `past_in_flames.rs` | 72 | 2 | 0 |
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
|   | Vampiric Fury | `vampiric_fury.rs` | 50 | 2 | 0 |
|   | Hysterical Blindness | `hysterical_blindness.rs` | 48 | 2 | 0 |
|   | Festerhide Boar | `festerhide_boar.rs` | 46 | 2 | 0 |
|   | Spare from Evil | `spare_from_evil.rs` | 46 | 2 | 0 |
|   | Somberwald Spider | `somberwald_spider.rs` | 43 | 2 | 0 |
|   | Parallel Lives | `parallel_lives.rs` | 42 | 2 | 0 |
|   | Spider Spawning | `spider_spawning.rs` | 42 | 2 | 0 |
|   | Rally the Peasants | `rally_the_peasants.rs` | 41 | 2 | 0 |
|   | Desperate Ravings | `desperate_ravings.rs` | 40 | 2 | 0 |
|   | Rolling Temblor | `rolling_temblor.rs` | 38 | 2 | 0 |
|   | Scourge of Geier Reach | `scourge_of_geier_reach.rs` | 37 | 2 | 0 |
|   | Avacyn's Pilgrim | `avacyns_pilgrim.rs` | 36 | 2 | 0 |
|   | Boneyard Wurm | `boneyard_wurm.rs` | 36 | 2 | 0 |
|   | Gnaw to the Bone | `gnaw_to_the_bone.rs` | 36 | 2 | 0 |
|   | Altar's Reap | `altars_reap.rs` | 35 | 2 | 0 |
|   | Infernal Plunge | `infernal_plunge.rs` | 34 | 2 | 0 |
|   | Moan of the Unhallowed | `moan_of_the_unhallowed.rs` | 34 | 2 | 0 |
|   | Midnight Haunting | `midnight_haunting.rs` | 32 | 2 | 0 |
|   | Skaab Ruinator | `skaab_ruinator.rs` | 31 | 2 | 0 |
|   | Think Twice | `think_twice.rs` | 29 | 2 | 0 |
|   | Stitched Drake | `stitched_drake.rs` | 28 | 1 | 0 |
|   | Skaab Goliath | `skaab_goliath.rs` | 27 | 1 | 0 |
|   | Makeshift Mauler | `makeshift_mauler.rs` | 26 | 1 | 0 |
