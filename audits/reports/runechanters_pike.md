# Audit: Runechanter's Pike

## Official Oracle
- **Name:** Runechanter's Pike
- **Cost:** {2}
- **Type:** Artifact — Equipment
- **Oracle Text:** Equipped creature has first strike and gets +X/+0, where X is the number of instant and sorcery cards in your graveyard.\nEquip {2}
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {2} — OK
- **Type:** Artifact, subtypes ["Equipment"] — OK
- **Oracle Text:** Matches — OK
- **P/T:** N/A — OK
- **Continuous Effects:** GrantKeyword FirstStrike on Attached — OK
- **dynamic_pt:** Counts instant/sorcery cards in controller's graveyard, returns (count, 0) — OK
- **Equip ability:** Equip {2}, sorcery speed, targets creature — OK
- **on_resolve:** Moves to battlefield, sets is_equipment — OK
- **on_activate_ability:** Attaches to target creature — OK

## Issues
1. **Minor: dynamic_pt counts from equipment's controller, not equipped creature's controller**: The dynamic_pt uses the equipment's own controller. If the equipment were somehow controlled by a different player than the equipped creature (unusual), this could differ. In practice this is fine.

## Verdict: PASS

## Audit — 2026-04-01 15:09

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Equipped creature has first strike and gets +X/+0, where X is the number of instant and sorcery cards in your graveyard.
Equip {2}
**Type line**: Artifact — Equipment
**Ruling**: [2011-09-22] The value of X is constantly updated as instant cards and sorcery cards are put into or removed from your graveyard.
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- First strike granted via continuous effect: PASS — `ContinuousEffect::GrantKeyword { keyword: Keyword::FirstStrike, scope: EffectScope::Attached }`
- Dynamic +X/+0 based on instant/sorcery count: PASS — `dynamic_pt` counts cards with `CardType::Instant` or `CardType::Sorcery` in controller's graveyard
- X is constantly updated (ruling): PASS — `dynamic_pt` is called each time P/T is computed, not cached
- Counts only controller's graveyard: PASS — filters by `o.owner == controller` where controller is the Pike's controller
- Equip cost {2} at sorcery speed: PASS — `cost: ManaCost::new(vec![ManaSymbol::Generic(2)])`, `sorcery_speed_only: true`
- Equipment enters battlefield unattached: PASS — `on_resolve` moves to battlefield and sets `is_equipment = true`
- Card types: PASS — `Artifact` with subtype `Equipment`
- Mana cost {2}: PASS

### Test coverage
- Card data correctness: `tier9_cards.rs:runechanters_pike_card_data`
- First strike + power bonus: `tier9_cards.rs:runechanters_pike_grants_first_strike_and_power_bonus`
- Equip ability: `tier9_cards.rs:runechanters_pike_equip_ability`
- X updates dynamically when cards enter/leave graveyard (ruling): `tier9_cards.rs:runechanters_pike_grants_first_strike_and_power_bonus` (adds cards to GY and checks updated power)
- 0 instants/sorceries gives +0/+0: `tier9_cards.rs:runechanters_pike_grants_first_strike_and_power_bonus` (checks base power before adding GY cards)
- Opponent's instant/sorcery cards not counted: NOT TESTED

## Audit - 2026-04-02

### Oracle Text (Scryfall)
- **Name:** Runechanter's Pike
- **Mana Cost:** {2}
- **Type:** Artifact — Equipment
- **Oracle Text:** Equipped creature has first strike and gets +X/+0, where X is the number of instant and sorcery cards in your graveyard. / Equip {2}

### Card Data Audit
- **Name:** Correct ("Runechanter's Pike")
- **Cost:** Correct ({2} = Generic(2))
- **Types:** Correct (Artifact, subtype Equipment)
- **Oracle Text String:** Correct
- **Keywords:** No keywords listed in code; first strike is granted via continuous effect to attached creature, not an intrinsic keyword. Acceptable.

### Behavior Audit
- **First strike:** Granted via `ContinuousEffect::GrantKeyword { keyword: Keyword::FirstStrike, scope: EffectScope::Attached }`. Correct.
- **+X/+0 (instant/sorcery count):** `dynamic_pt` counts instants and sorceries in the controller's graveyard. Used as an attached source so the value is added to the creature's P/T. Correct.
- **Equip {2}:** Activated ability with Generic(2) cost, sorcery-speed only, targets creature you control. Correct.
- **Equipment behavior:** `on_resolve` sets `is_equipment = true` and moves to battlefield. `on_activate_ability` sets `attached_to`. Correct.

### Result
**PASS**
