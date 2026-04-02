# ISD Audit Progress

## Status
Working through 89 ISD cards that need re-auditing (43 informal + 46 ISSUE).
Sorting by complexity: planeswalkers/DFCs first, simple cards last.
Batches 1-2 complete (10 cards). Starting batch 3.

## Cards with ISSUE status (need /implement-card fix)
- **Garruk Relentless**: State-triggered transform implemented as immediate SBA instead of going on stack as proper triggered ability
- **Civilized Scholar**: Oracle text field uses old card-name wording instead of current "this creature" template. Also missing "tap this creature, then" in back face oracle text. Behavior is correct — text-only fix needed.

## Completed Audits (new PASS) — 8 cards
- Geist of Saint Traft: PASS
- Grimgrin, Corpse-Born: PASS
- Grimoire of the Dead: PASS
- Liliana of the Veil: PASS
- Cloistered Youth: PASS
- Delver of Secrets: PASS
- Kruin Outlaw: PASS
- Daybreak Ranger: PASS

## Systemic Issues Found
- `crate::combat::fight` emits `CombatDamageDealt` for fight damage, but fight damage is NOT combat damage per MTG rules. Affects all cards using fight. (Found during Daybreak Ranger audit)

## Next Up

### Batch 3: DFCs + complex
- Ludevics Test Subject
- Mayor of Avabruck
- Fiend Hunter
- Laboratory Maniac
- Evil Twin

### Batch 4: Complex triggered/activated
- Charmbreaker Devils, Creepy Doll, Burning Vengeance, Rooftop Storm, Skaab Ruinator

### Batch 5+: Remaining ~69 cards
altars_reap, blazing_torch, bonds_of_faith, bump_in_the_night, butchers_cleaver,
burning_vengeance, caravan_vigil, cellar_door, champion_of_the_parish, chapel_geist,
claustrophobia, clifftop_retreat, cobbled_wings, corpse_lunge, creeping_renaissance,
crossway_vampire, curiosity, curse_of_deaths_hold, curse_of_oblivion,
curse_of_stalked_prey, curse_of_the_bloody_tome, curse_of_the_nightly_hunt,
darkthicket_wolf, dead_weight, dearly_departed, demonmail_hauberk, deranged_assistant,
desperate_ravings, devils_play, diregraf_ghoul, disciple_of_griselbrand, dissipate,
divine_reckoning, doomed_traveler, dream_twist, elder_cathar, elder_of_laurels,
elite_inquisitor, endless_ranks_of_the_dead, essence_of_the_wild, falkenrath_marauders,
falkenrath_noble, feeling_of_dread, feral_ridgewolf, festerhide_boar, forbidden_alchemy,
fortress_crab, frightful_delusion, full_moons_rise, furor_of_the_bitten,
gallows_warden, galvanic_juggernaut, ghoulcallers_chant, ghoulraiser, grasp_of_phantoms,
graveyard_shovel, gutter_grime, harvest_pyre, heretics_punishment, infernal_plunge,
inquisitors_flail, kessig_wolf_run, laboratory_maniac, memorys_journey,
purify_the_grave, rooftop_storm, runic_repetition, scourge_of_geier_reach,
skaab_ruinator, stony_silence, unbreathing_horde, witchbane_orb, wooden_stake
