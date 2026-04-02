# ISD Audit Progress

## Status
Batches 1-7 complete (35 cards audited). 24 PASS, 11 ISSUE.

## Cards with ISSUE status (need /implement-card fix)
- **Garruk Relentless**: State-triggered transform as immediate SBA instead of stack
- **Civilized Scholar**: Oracle text field wording (text-only fix)
- **Ludevic's Test Subject**: Manual transform instead of helpers::apply_transform()
- **Evil Twin**: 6 issues — mandatory copy, auto-selected target, incomplete copy, wrong target filter
- **Fiend Hunter**: LTB doesn't reset controller to owner
- **Burning Vengeance**: Only triggers on flashback, not all graveyard casts. Also triggers.rs SpellCast filter blocks non-instant/sorcery
- **Rooftop Storm**: Alternative cost implemented as unconditional free cast
- **Essence of the Wild**: Replacement effect modeled as triggered ability, incomplete copy
- **Heretic's Punishment**: Wrong order (damage before mill), missing damaged_by, outdated oracle text
- **Caravan Vigil**: Morbid "you may" auto-selects instead of presenting choice
- **Claustrophobia**: ETB tap during resolution instead of as triggered ability on stack

## Completed Audits (new PASS) — 24 cards
- Geist of Saint Traft, Grimgrin Corpse-Born, Grimoire of the Dead, Liliana of the Veil
- Cloistered Youth, Delver of Secrets, Kruin Outlaw, Daybreak Ranger
- Mayor of Avabruck, Laboratory Maniac, Charmbreaker Devils, Creepy Doll, Skaab Ruinator
- Endless Ranks of the Dead, Full Moon's Rise, Gutter Grime
- Altar's Reap, Blazing Torch, Bonds of Faith, Bump in the Night, Butcher's Cleaver
- Cellar Door, Champion of the Parish, Chapel Geist

## Systemic Issues Found
- `crate::combat::fight` emits CombatDamageDealt for fight damage (should be NonCombat)
- `triggers.rs` SpellCast filter blocks non-instant/sorcery from SpellCast events
- Registry-only Human subtype check pattern (Butcher's Cleaver, Bonds of Faith, etc.)

## Remaining to Audit (~54 cards)

### Batch 8
clifftop_retreat, cobbled_wings, corpse_lunge, creeping_renaissance, crossway_vampire

### Batch 9
curiosity, curse_of_deaths_hold, curse_of_oblivion, curse_of_stalked_prey, curse_of_the_bloody_tome

### Batch 10
curse_of_the_nightly_hunt, darkthicket_wolf, dead_weight, dearly_departed, demonmail_hauberk

### Batch 11+
deranged_assistant, desperate_ravings, devils_play, diregraf_ghoul, disciple_of_griselbrand,
dissipate, divine_reckoning, doomed_traveler, dream_twist, elder_cathar, elder_of_laurels,
elite_inquisitor, falkenrath_marauders, falkenrath_noble, feeling_of_dread, feral_ridgewolf,
festerhide_boar, forbidden_alchemy, fortress_crab, frightful_delusion, furor_of_the_bitten,
gallows_warden, galvanic_juggernaut, ghoulcallers_chant, ghoulraiser, grasp_of_phantoms,
graveyard_shovel, harvest_pyre, infernal_plunge, inquisitors_flail, kessig_wolf_run,
memorys_journey, purify_the_grave, runic_repetition, scourge_of_geier_reach,
stony_silence, unbreathing_horde, witchbane_orb, wooden_stake
