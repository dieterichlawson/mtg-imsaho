# Card Implementation Issues

## Category 1: Critical Code Bugs

- [ ] **gnaw_to_the_bone**: Flashback cost is {3}{G} but should be {2}{G} (line 26)
- [ ] **blazing_torch**: Damage source should be the torch, not the equipped creature (line 129). Per Scryfall ruling: "The source of the damage is Blazing Torch, not the equipped creature."
- [ ] **blazing_torch**: Copy-paste error in log message references "Burning Vengeance" (line 68)
- [ ] **charmbreaker_devils**: on_spell_cast doesn't filter for instant/sorcery — triggers on ANY spell (lines 75-92)
- [ ] **skirsdag_high_priest**: `sorcery_speed_only: true` is incorrect — Oracle only restricts to morbid, not sorcery speed (line 60)
- [ ] **skirsdag_high_priest**: Oracle text field says "Activate only as a sorcery" but should say "Activate only if a creature died this turn" (line 25)
- [ ] **creepy_doll**: Uses TriggerKind::Blocks/BecomesBlocked but Oracle says "Whenever Creepy Doll deals combat damage to a creature" — trigger timing is wrong (lines 30-38)
- [ ] **daybreak_ranger**: Nightfall Predator fight restricts to opponent's creatures only, but Oracle allows targeting any creature (line 128)
- [ ] **makeshift_mauler**: Potential double-exile — AdditionalCost::ExileCreaturesFromGraveyard(1) AND manual exile in on_resolve
- [ ] **olivia_voldaren**: Ability 0 allows targeting self but Oracle says "another target creature"
- [ ] **olivia_voldaren**: Ability 1 target filter uses Any instead of Vampire-only
- [ ] **olivia_voldaren**: Control duration not tracked (should last "as long as you control Olivia")
- [ ] **grimgrin_corpse_born**: Attack trigger +1/+1 counter always added regardless of whether destroy succeeded
- [ ] **graveyard_shovel**: Targets a card but Oracle says "Target player exiles a card from their graveyard" — targeting model is wrong
- [ ] **graveyard_shovel**: Oracle text in code is wrong (line 23)

## Category 2: Token Subtype Checking (systemic — same pattern in all)

These cards only check `registry.card_data(o.card_id)` for subtypes but don't check `o.subtypes` on the object. Tokens have card_id=0 so registry returns None, missing token subtypes.

- [ ] **bonds_of_faith**: Human check misses tokens (lines 43-46)
- [ ] **butchers_cleaver**: Human check misses tokens (lines 15-18)
- [ ] **elder_cathar**: Human check misses tokens (lines 50-53)
- [ ] **sharpened_pitchfork**: Human check misses tokens (lines 15-18)
- [ ] **silver_inlaid_dagger**: Human check misses tokens (lines 16-18)
- [ ] **slayer_of_the_wicked**: Vampire/Werewolf/Zombie check misses tokens (lines 41-43)
- [ ] **urgent_exorcism**: Spirit check misses tokens (lines 40-44)
- [ ] **vampiric_fury**: Vampire check misses tokens (lines 44-46)
- [ ] **victim_of_night**: Vampire/Werewolf/Zombie check misses tokens (line 42)
- [ ] **village_cannibals**: Human check misses tokens (lines 39-42)

## Category 3: Missing damaged_by Tracking

These cards mark damage but don't push to `obj.damaged_by`, breaking cards that check damage source.

- [ ] **corpse_lunge**: Missing damaged_by push (lines 57-60)
- [ ] **into_the_maw_of_hell**: Missing damaged_by push (line 71)
- [ ] **rolling_temblor**: Missing damaged_by push (lines 37-39)
- [ ] **skirsdag_cultist**: Missing damaged_by push (lines 54-57)
- [ ] **heretics_punishment**: Missing damaged_by push (lines 82-85)

## Category 4: LLM Card Knowledge Errors

- [ ] **intangible_virtue**: LLM says "Your creatures get +1/+1" — should say "Creature tokens you control get +1/+1 and have vigilance" (llm.rs line 103)
- [ ] **feeling_of_dread**: LLM says "Tap target creature" — should say "Tap up to two target creatures" (llm.rs line 110)
- [ ] **fiend_hunter**: LLM says "exiles an opponent's creature" — should say "another target creature" (llm.rs line 102)
- [ ] **forbidden_alchemy**: LLM says "Draw 1 card, mill 3" — should say "Look at top four, put one in hand, rest in graveyard" (llm.rs line 113)
- [ ] **frightful_delusion**: LLM says "Counter target spell" — should say "Counter target spell unless its controller pays {1}" (llm.rs line 73)
- [ ] **travel_preparations**: LLM says "Put a +1/+1 counter on target creature" — should say "each of up to two target creatures" (llm.rs line 109)

## Category 5: Other Code Issues

- [ ] **burning_vengeance**: Incorrect log message "deals 2 damage to opponent" before target chosen (line 67-68)
- [ ] **full_moons_rise**: Activated ability description says "Wolf and Werewolf" but Oracle only says "Werewolf" (line 58). Code comment on line 9 also wrong.
- [ ] **heretics_punishment**: Oracle text uses old wording "reveal top 3... bottom of library" but should say "mill three cards" (line 26)
- [ ] **instigator_gang**: Back face triggered_abilities missing TriggerKind::Upkeep entry
- [ ] **kruin_outlaw**: Back face (Terror of Kruin Pass) missing global evasion ability for all Werewolves
- [ ] **gutter_grime**: Slime counters stored as PlusOnePlusOne (line 19); tokens have static P/T instead of dynamic
- [ ] **memorys_journey**: Shuffles all players' libraries instead of just targeted player's
- [ ] **curse_of_the_pierced_heart**: Oracle says "deals 1 damage to that player or a planeswalker that player controls" but only damages player
- [ ] **lava_axe**: Target restriction only allows players, Oracle says "target player or planeswalker"
- [ ] **stensia_bloodhall**: Target restriction only allows players, Oracle says "target player or planeswalker"
- [ ] **rage_thrower**: Target restriction only allows players, Oracle says "target player or planeswalker"
- [ ] **diregraf_ghoul**: Oracle text says "enters the battlefield tapped" but Scryfall says "This creature enters tapped" (cosmetic)
- [ ] **essence_of_the_wild**: Uses trigger instead of replacement effect; oracle text cosmetic mismatch
- [ ] **festerhide_boar**: Oracle describes replacement effect ("enters with") but code uses ETB trigger
- [ ] **harvest_pyre**: Oracle text in code says "any number" but should say "X"; exile done at resolution not as cost
- [ ] **inquisitors_flail**: Offensive double damage approximated via dynamic_pt (changes actual power); defensive doubling not implemented
- [ ] **back_from_the_brink**: Ability cost uses flat Generic(2) instead of exiled creature's mana cost

## Category 6: Documented Simplifications (auto-targeting, "you may")

These are acknowledged in code comments as simplifications:

- [ ] **altars_reap**: Auto-selects sacrifice target
- [ ] **bitterheart_witch**: Auto-search, auto-target
- [ ] **caravan_vigil**: Morbid "you may" auto-chooses battlefield
- [ ] **civilized_scholar**: Auto-picks creature to discard
- [ ] **cloistered_youth**: Transform not optional ("you may")
- [ ] **curiosity**: Auto-draws instead of optional
- [ ] **delver_of_secrets**: Auto-reveals instead of optional
- [ ] **divine_reckoning**: Auto-selects creature with highest toughness
- [ ] **evil_twin**: Auto-copies first creature
- [ ] **falkenrath_noble**: Auto-targets opponent
- [ ] **ghost_quarter**: "May search" auto-applied
- [ ] **grimgrin_corpse_born**: Sacrifice and attack trigger auto-pick targets
- [ ] **grimoire_of_the_dead**: Auto-picks discard
- [ ] **harvest_pyre**: Auto-exiles all graveyard cards
- [ ] **infernal_plunge**: Auto-picks sacrifice target
- [ ] **mentor_of_the_meek**: Auto-pays {1}
- [ ] **moorland_haunt**: Auto-picks creature to exile
- [ ] **nevermore**: Auto-picks card name (looks at opponent's hand incorrectly)
- [ ] **night_terrors**: Auto-selects nonland card
- [ ] **screeching_bat**: Auto-pays transform cost
- [ ] **snapcaster_mage**: Auto-selects target
- [ ] **stitchers_apprentice**: Auto-selects sacrifice target
- [ ] **thraben_sentry**: Transform not optional ("you may")
- [ ] **corpse_lunge**: Auto-picks highest-power creature to exile
- [ ] **creeping_renaissance**: Type choice hardcoded to "creature"
- [ ] **liliana_of_the_veil**: Multiple simplifications (auto-discard, auto-target, simplified -6)
- [ ] **garruk_relentless**: Auto-targeting fight, back face not implemented
- [ ] **back_from_the_brink**: Auto-picks creature to exile

## Category 7: Missing Tests

- [ ] **ancient_grudge**: No tests
- [ ] **armored_skaab**: No tests
- [ ] **endless_ranks_of_the_dead**: No tests
- [ ] **essence_of_the_wild**: No tests
- [ ] **selhoff_occultist**: No tests

## Category 8: Engine Limitations (cannot fix without engine changes)

- [ ] **stony_silence**: Static ability not enforced (needs engine system for preventing artifact ability activation)
- [ ] **witchbane_orb**: Player hexproof not implemented (needs engine system)
- [ ] **unbreathing_horde**: Damage prevention replacement effect not implemented
- [ ] **moonmist**: Combat damage prevention not implemented
- [ ] **bonds_of_faith**: Human check not continuous (effect locked at ETB)
