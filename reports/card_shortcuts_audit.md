# Card Shortcuts Audit

Audit date: 2026-04-12

A manual review of every implemented card file for deviations from MTG oracle text behavior.

**Failing tests:** `mtg-engine/tests/card_shortcuts.rs` -- 12 failing tests, 2 passing controls.

---

## SIGNIFICANT -- game behavior deviates from oracle text

### 1. Charmbreaker Devils -- +4/+0 triggers on ANY spell, not just instants/sorceries
**File:** `isd/charmbreaker_devils.rs:75-91`
**Test:** `charmbreaker_devils_no_pump_on_creature_spell`
**Oracle:** "Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn."
**Bug:** `on_spell_cast` grants +4/+0 on ANY spell you cast -- no check that the spell is an instant or sorcery. Casting a creature spell incorrectly triggers the pump.

### 2. Snapcaster Mage -- can't target cards that already have printed flashback
**File:** `isd/snapcaster_mage.rs:52-57`
**Test:** `snapcaster_targets_card_with_printed_flashback`
**Oracle:** "target instant or sorcery card in your graveyard gains flashback until end of turn"
**Bug:** Filters out cards where `d.flashback_cost.is_some()`. The real Snapcaster can target any instant or sorcery, including ones that already have flashback (granting a second flashback cost equal to their mana cost).

### 3. Trepanation Blade -- land detection uses empty obj.card_types instead of registry
**File:** `isd/trepanation_blade.rs:88-90`
**Test:** `trepanation_blade_stops_on_land`
**Oracle:** "reveals cards from the top of their library until they reveal a land card"
**Bug:** Checks `o.card_types.contains(&CardType::Land)` on the object directly. Regular card objects don't have `card_types` populated (types come from registry), so this check always returns false and the entire library gets milled.

### 4. Caravan Vigil -- doesn't let player choose which basic land
**File:** `isd/caravan_vigil.rs:38-49`
**Test:** `caravan_vigil_presents_choice_among_basics`
**Oracle:** "Search your library for a basic land card"
**Bug:** Uses `.find()` to get the first basic land in library order. In MTG, searching your library means the player chooses which card to get.

### 5. Vampiric Fury -- doesn't check instance subtypes for Vampire
**File:** `isd/vampiric_fury.rs:42-48`
**Test:** `vampiric_fury_buffs_instance_vampire`
**Oracle:** "Vampire creatures you control get +2/+0 and gain first strike"
**Bug:** Only checks `registry.card_data()` for the "Vampire" subtype. Doesn't check `obj.subtypes`. A creature turned into a Vampire by Olivia Voldaren won't receive the buff.

### 6. Memory's Journey -- doesn't enforce cards come from target player's graveyard
**File:** `isd/memorys_journey.rs:56-68`
**Test:** `memorys_journey_rejects_wrong_players_graveyard_card`
**Oracle:** "Target player shuffles up to three target cards from their graveyard into their library."
**Bug:** Accepts any graveyard card target regardless of owner. Cards from any player's graveyard can be shuffled into the targeted player's library.

### 7. Rolling Temblor -- doesn't track damage source
**File:** `isd/rolling_temblor.rs:37-39`
**Test:** `rolling_temblor_records_damage_source`
**Oracle:** "Rolling Temblor deals 2 damage to each creature without flying."
**Bug:** Sets `obj.damage_marked += 2` but doesn't push to `obj.damaged_by`. Cards that care about damage sources (Abattoir Ghoul, Rage Thrower) won't see Rolling Temblor as the source.

### 8. Slayer of the Wicked -- only checks registry subtypes, not instance subtypes
**File:** `isd/slayer_of_the_wicked.rs:42-46`
**Test:** `slayer_of_the_wicked_sees_instance_vampire`
**Oracle:** "you may destroy target Vampire, Werewolf, or Zombie"
**Bug:** Only checks `registry.card_data().subtypes`. Doesn't check `obj.subtypes`. A creature that gained the Vampire subtype via Olivia Voldaren wouldn't be a valid target.

### 9. Mayor of Avabruck -- front face incorrectly has "Werewolf" subtype
**File:** `isd/mayor_of_avabruck.rs:33`
**Test:** `mayor_of_avabruck_front_face_not_werewolf`
**Oracle:** Front face is "Human Advisor" only. The Werewolf subtype belongs on the back face (Howlpack Alpha).
**Bug:** `subtypes: vec!["Human", "Advisor", "Werewolf"]`. This means Victim of Night incorrectly can't target it, and Moonmist may interact with it incorrectly.

### 10. Festerhide Boar -- morbid counters only applied when cast, not when reanimated
**File:** `isd/festerhide_boar.rs:34-43`
**Test:** `festerhide_boar_gets_morbid_counters_when_reanimated`
**Oracle:** "This creature enters with two +1/+1 counters on it if a creature died this turn."
**Bug:** Counter logic is only in `on_resolve` (the cast path). There's no `has_etb_handler`/`on_enter_battlefield` override. If the Boar is reanimated (e.g., Unburial Rites moves it to battlefield), the morbid counters are never applied.

### 11. Lava Axe -- can't target planeswalkers
**File:** `lava_axe.rs:29-30`
**Test:** `lava_axe_target_requirement_includes_planeswalkers`
**Oracle:** "Lava Axe deals 5 damage to target player or planeswalker."
**Bug:** Uses `TargetRequirement::PlayerOnly`, so the engine won't present planeswalkers as targeting options.

### 12. Civilized Scholar -- creature detection uses power heuristic instead of registry
**File:** `isd/civilized_scholar.rs:126`
**Test:** `civilized_scholar_detects_creature_via_registry`
**Oracle:** "If a creature card is discarded this way"
**Bug:** Checks `o.power.is_some()` to determine if the discarded card is a creature. Should check registry card_types instead. A creature card whose object doesn't have power set (e.g., not fully initialized) won't trigger the transform.

---

## UNTESTABLE -- real deviations that can't be covered by a unit test

### 13. Nevermore -- can only name implemented cards, not any card
**File:** `isd/nevermore.rs:47-55`
**Oracle:** "As this enchantment enters, choose a nonland card name."
**Issue:** Only offers names from `registry.all_names()`. In real MTG you can name any card. This is a fundamental UI limitation (no free-text input).

### 14. Nevermore -- "As enters" implemented as ETB trigger
**File:** `isd/nevermore.rs:41-67`
**Oracle:** "As this enchantment enters" -- replacement effect (can't be responded to).
**Issue:** Uses `on_enter_battlefield` (triggered ability, goes on stack). The timing difference means an opponent could theoretically respond before the name is chosen. Not testable because the engine doesn't model respondability at this granularity.

---

## NOT ISSUES -- verified correct or acceptable

- **Forbidden Alchemy** -- doc comment says "Simplified: draw 1 card, mill 3" but the actual code correctly reveals top 4, lets player choose, and mills the rest. Stale comment only.
- **Daybreak Ranger** -- front face IS "Human Archer Werewolf" per Scryfall. The subtypes in the code are correct.
- **Bloodline Keeper** -- creates a new CardRegistry in `activated_abilities`. Wasteful but functionally correct.
- **Scourge of Geier Reach** -- only counts one opponent's creatures. Acceptable for 2-player games.
- **Swords to Plowshares** -- reads effective power before exiling. Correct per MTG rules (last known information).
- **Parallel Lives** -- declared as `ReplacementEffect::DoubleTokens`. Correctness depends on engine handling, not the card file.
- **Grimoire of the Dead** -- study counters not explicitly removed as cost, but the Grimoire is sacrificed so the effect is identical.
- **Ghoulcaller's Chant** -- modal choice handled by engine's `TargetRequirement::ModalChoice`. Card code correctly moves all targets to hand.
- **Sever the Bloodline** -- uses `power.is_some()` heuristic for creature check, which is the standard engine convention. Can't fail for real creatures.
- **Grasp of Phantoms** -- `move_object` to Zone::Library does not add to `library_order`, so the subsequent `insert(0, ...)` is correct (not a double-insert).
- **Festerhide Boar (cast path)** -- when cast, counters are added before ETB triggers fire (triggers are deferred to `process_triggers`). Mentor of the Meek interaction works correctly on the cast path.

---

## CARDS VERIFIED AS CORRECT (notable complex implementations)
- Liliana of the Veil -- all 3 abilities properly implemented with player choice
- Fiend Hunter -- both ETB exile and LTB return implemented, uses card_state for tracking
- Geist of Saint Traft -- Angel token created tapped/attacking, exiled at end of combat
- Olivia Voldaren -- both abilities correct, steal tracking with revert on leave
- Grimgrin, Corpse-Born -- enters tapped, doesn't untap, sacrifice ability, attack trigger all correct
- Evil Twin -- copy on ETB, activated destroy ability with same-name filter
- Divine Reckoning -- each player chooses, turn order respected
- Unbreathing Horde -- enters-with counters, damage prevention replacement effect
- Undead Alchemist -- damage replacement with mill-to-exile-to-token chain
- Back from the Brink -- per-creature activated abilities with correct mana costs
- Brimstone Volley -- morbid check correct
- All werewolf DFCs -- transform conditions correctly check spell counts
