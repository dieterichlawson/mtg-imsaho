# ISD Audit Progress

## Status
Batches 1-4 complete (20 cards audited). Starting batch 5.
13 PASS, 7 ISSUE/MINOR.

## Cards with ISSUE status (need /implement-card fix)
- **Garruk Relentless**: State-triggered transform implemented as immediate SBA instead of going on stack
- **Civilized Scholar**: Oracle text field uses old card-name wording. Behavior correct — text-only fix.
- **Ludevic's Test Subject**: Manual transform instead of helpers::apply_transform()
- **Evil Twin**: 6 issues — mandatory copy (should be optional), auto-selected target, card_types not copied, subtypes merged, destroy ability target filter wrong, is_evil_twin not copiable
- **Fiend Hunter**: Minor — LTB doesn't reset controller to owner; missing O-Ring trick test
- **Burning Vengeance**: Only triggers on flashback (cast_with_flashback), not all graveyard casts
- **Rooftop Storm**: Alternative cost implemented as unconditional free cast — should be optional, bypasses additional costs

## Completed Audits (new PASS) — 13 cards
- Geist of Saint Traft, Grimgrin Corpse-Born, Grimoire of the Dead, Liliana of the Veil
- Cloistered Youth, Delver of Secrets, Kruin Outlaw, Daybreak Ranger
- Mayor of Avabruck, Laboratory Maniac
- Charmbreaker Devils, Creepy Doll, Skaab Ruinator

## Systemic Issues Found
- `crate::combat::fight` emits `CombatDamageDealt` for fight damage — should be NonCombatDamageDealt

## Remaining to Audit (~69 cards)

### Batch 5: Complex remaining
- Essence of the Wild, Endless Ranks of the Dead, Gutter Grime, Heretics Punishment, Full Moons Rise

### Batch 6+: All remaining cards
altars_reap, blazing_torch, bonds_of_faith, bump_in_the_night, butchers_cleaver,
caravan_vigil, cellar_door, champion_of_the_parish, chapel_geist, claustrophobia,
clifftop_retreat, cobbled_wings, corpse_lunge, creeping_renaissance, crossway_vampire,
curiosity, curse_of_deaths_hold, curse_of_oblivion, curse_of_stalked_prey,
curse_of_the_bloody_tome, curse_of_the_nightly_hunt, darkthicket_wolf, dead_weight,
dearly_departed, demonmail_hauberk, deranged_assistant, desperate_ravings, devils_play,
diregraf_ghoul, disciple_of_griselbrand, dissipate, divine_reckoning, doomed_traveler,
dream_twist, elder_cathar, elder_of_laurels, elite_inquisitor, falkenrath_marauders,
falkenrath_noble, feeling_of_dread, feral_ridgewolf, festerhide_boar, forbidden_alchemy,
fortress_crab, frightful_delusion, furor_of_the_bitten, gallows_warden,
galvanic_juggernaut, ghoulcallers_chant, ghoulraiser, grasp_of_phantoms,
graveyard_shovel, harvest_pyre, infernal_plunge, inquisitors_flail, kessig_wolf_run,
memorys_journey, purify_the_grave, runic_repetition, scourge_of_geier_reach,
stony_silence, unbreathing_horde, witchbane_orb, wooden_stake
