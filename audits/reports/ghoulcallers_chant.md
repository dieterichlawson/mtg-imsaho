# Audit: Ghoulcaller's Chant

## Oracle Reference (Scryfall)
- Cost: {B}
- Type: Sorcery
- Oracle: "Choose one --
  * Return target creature card from your graveyard to your hand.
  * Return two target Zombie creature cards from your graveyard to your hand."

NOTE: Current Scryfall oracle text says "Zombie cards" not "Zombie creature cards" for mode 2. However the original Innistrad printing says "Zombie creature cards". The current oracle errata simplified it.

## Implementation: ghoulcallers_chant.rs

## Issues Found

1. **ISSUE: Mode selection is automated, not player-chosen** - The implementation auto-selects mode 2 (return two Zombies) whenever there are 2+ Zombies in graveyard, and falls back to mode 1 otherwise. Per Oracle, the player chooses which mode. A player might want to return a single non-Zombie creature even when Zombies are available.

2. **BUG (from prior audit): Oracle text says "Zombie creature cards" but current errata says "Zombie cards"** - The engine filters for creature AND Zombie (lines 43-49), but updated oracle only requires Zombie subtype. Low severity since all Zombies in the set are creatures.

Otherwise correct: cost ({B}), type (Sorcery), oracle text structure matches.

## Verdict: ISSUES FOUND (2 issues)

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Choose one -- / Return target creature card from your graveyard to your hand. / Return two target Zombie cards from your graveyard to your hand.
**Scryfall type line**: Sorcery
**Status**: ISSUE

Mana cost {B}: correct. Type Sorcery: correct. Uses `move_spell_after_resolve`: correct (no graveyard anti-pattern). Modal targeting via `TargetRequirement::ModalChoice`: correct structure.

on_resolve: moves each targeted card from graveyard to hand, then calls `move_spell_after_resolve`: correct behavior.

Issues found:
1. **Mode 2 filters for "Zombie creature cards" but oracle says "Zombie cards"**: The `TargetRequirement` for mode 2 uses `GraveyardCreatureOfSubtype("Zombie")`, which requires the target to be both a creature card and have the Zombie subtype. The current Scryfall oracle text says "two target Zombie cards" (not "Zombie creature cards"). While all Zombie cards in Innistrad are creatures, this is technically a stricter filter than the oracle requires. Low severity since Zombie is a creature subtype and non-creature Zombie cards are extremely rare.

Tests present in `tests/ghoulcallers_chant.rs`, `tests/innistrad_simple_cards.rs`, and `tests/tier11_cards.rs`.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Choose one — Return target creature card from your graveyard to your hand; or return two target Zombie cards from your graveyard to your hand.
**Type line**: Sorcery
**Status**: ISSUE

Mana cost {B}: correct. Type Sorcery: correct. No subtypes: correct. Uses `move_spell_after_resolve`: correct. Modal targeting via `TargetRequirement::ModalChoice`: correct structure. `on_resolve` moves each targeted card from graveyard to hand: correct.

`is_valid_target` checks that the target is in the caster's graveyard (`o.owner == caster`): correct -- both modes return cards from "your graveyard".

Issues found:
1. **Mode 2 filters for "Zombie creature cards" but oracle says "Zombie cards"** (persists from prior audit): The `TargetRequirement` for mode 2 uses `GraveyardCreatureOfSubtype("Zombie")`, requiring the target to be both a creature card and have the Zombie subtype. Current Scryfall oracle says "two target Zombie cards" not "Zombie creature cards." While all Zombie-subtyped cards in Innistrad are creatures, this is technically a stricter filter than oracle requires. Low severity.

Tests in `tests/tier11_cards.rs` cover: mode 1 returning a creature, mode 2 returning two zombies. No anti-patterns found.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Choose one — / • Return target creature card from your graveyard to your hand. / • Return two target Zombie cards from your graveyard to your hand.
**Type line**: Sorcery
**Status**: ISSUE

Mana cost {B}: correct. Type Sorcery: correct. Uses `move_spell_after_resolve`: correct.

Modal targeting via `TargetRequirement::ModalChoice`: correct structure. Mode 1: `GraveyardCreature` (return target creature card): correct. Mode 2: `TwoTargets` of `GraveyardCreatureOfSubtype("Zombie")`: functionally correct.

on_resolve: iterates targets, moves each from graveyard to hand, calls `move_spell_after_resolve`: correct.

Issues found (persisting from prior audit):
1. **Mode 2 uses "Zombie creature cards" filter but oracle says "Zombie cards"**: The `GraveyardCreatureOfSubtype("Zombie")` requirement checks for both creature type and Zombie subtype. Current Scryfall oracle says "two target Zombie cards" not "Zombie creature cards." While Zombie is a creature subtype (making non-creature Zombie cards extremely rare in practice), the filter is technically more restrictive than the oracle requires. Low severity.

Tests in `tests/tier11_cards.rs`: returns creature from graveyard, returns two zombies. Good basic coverage. No anti-patterns found.

## Audit — 2026-04-01 12:00

**Oracle text source**: Scryfall via WebSearch, confirmed by Gatherer via WebSearch
**Oracle text**: Choose one — • Return target creature card from your graveyard to your hand. • Return two target Zombie cards from your graveyard to your hand.
**Type line**: Sorcery
**Status**: ISSUE

Mana cost {B}: correct. Type Sorcery: correct. No subtypes/supertypes: correct. Uses `move_spell_after_resolve`: correct. Modal targeting via `TargetRequirement::ModalChoice`: correct structure. `on_resolve` iterates targets and moves each from graveyard to hand: correct. `is_valid_target` checks `o.zone == Zone::Graveyard && o.owner == caster`: correct (both modes target "your graveyard").

Tests in `tests/ghoulcallers_chant.rs` cover: mode 1 returning a creature, mode 2 returning two zombies, legal actions for mode 1 and mode 2, no mode 2 for non-zombies, cannot target opponent's graveyard, mixed graveyard correct modes. Good coverage.

Issues found:
1. **Oracle text string says "Zombie creature cards" but Scryfall oracle says "Zombie cards"** (`/home/user/mtg-imsaho/mtg-engine/src/cards/ghoulcallers_chant.rs`, line 24):
   - Oracle text says: `Return two target Zombie cards from your graveyard to your hand.`
   - Code does: `oracle_text` field contains `"Return two target Zombie creature cards from your graveyard to your hand."` and `GraveyardCreatureOfSubtype("Zombie")` (line 38-39) enforces the target must be a creature card with Zombie subtype. The current oracle only requires "Zombie cards" -- any card with the Zombie subtype, not necessarily a creature. Low severity since Zombie is a creature subtype and non-creature Zombie cards are extremely rare in practice.

## Audit — 2026-04-01 16:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Choose one —
• Return target creature card from your graveyard to your hand.
• Return two target Zombie cards from your graveyard to your hand.
**Type line**: Sorcery
**Status**: ISSUE

### Code issues
1. **Oracle text field says "Zombie creature cards" but oracle says "Zombie cards"** (`mtg-engine/src/cards/isd/ghoulcallers_chant.rs` line 24):
   - Oracle text says: `Return two target Zombie cards from your graveyard to your hand.`
   - Code oracle_text says: `Return two target Zombie creature cards from your graveyard to your hand.`
   - The word "creature" is added in the code but not present in the current oracle text. The targeting requirement `GraveyardCreatureOfSubtype("Zombie")` also enforces creature type, which is technically more restrictive. Low severity since only creatures have the Zombie subtype in practice.

Card data verified correct: mana cost {B}, card_types (Sorcery), no supertypes, no subtypes, no P/T, no keywords. oracle_text structure (modal with two modes) correct. Uses `move_spell_after_resolve`: correct. Modal targeting via `TargetRequirement::ModalChoice` with `GraveyardCreature` for mode 1 and `TwoTargets(GraveyardCreatureOfSubtype("Zombie"), GraveyardCreatureOfSubtype("Zombie"))` for mode 2: correct structure. `is_valid_target` checks `o.zone == Zone::Graveyard && o.owner == caster`: correct (both modes target "your graveyard"). `on_resolve` iterates targets and moves each from graveyard to hand: correct.

### Tricky interactions checked
- Mode 1 returns any creature card: pass
- Mode 2 returns exactly two Zombie cards: pass
- Cannot target opponent's graveyard: pass (is_valid_target checks owner)
- Mode 2 not available with fewer than 2 Zombies: pass (tested)
- Mixed graveyard (some Zombies, some non-Zombies): pass (tested)

### Test coverage
- Mode 1 returns creature: `mtg-engine/tests/ghoulcallers_chant.rs:22` (mode1_returns_one_creature_from_graveyard)
- Mode 2 returns two Zombies: `mtg-engine/tests/ghoulcallers_chant.rs:38` (mode2_returns_two_zombies_from_graveyard)
- Legal actions include mode 1: `mtg-engine/tests/ghoulcallers_chant.rs:61` (legal_actions_include_mode1_single_creature)
- Legal actions include mode 2: `mtg-engine/tests/ghoulcallers_chant.rs:88` (legal_actions_include_mode2_two_zombies)
- No mode 2 for non-Zombies: `mtg-engine/tests/ghoulcallers_chant.rs:119` (legal_actions_no_mode2_for_non_zombies)
- Cannot target opponent's graveyard: `mtg-engine/tests/ghoulcallers_chant.rs:161` (cannot_target_opponents_graveyard)
- Mixed graveyard correct modes: `mtg-engine/tests/ghoulcallers_chant.rs:183` (mixed_graveyard_correct_modes)
- Fizzle (targets leave graveyard before resolution): NOT TESTED

## Audit — 2026-04-01 17:00

**Oracle text source**: Scryfall API (cached)
**Oracle text**: Choose one —
• Return target creature card from your graveyard to your hand.
• Return two target Zombie cards from your graveyard to your hand.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

Previously flagged oracle text mismatch has been fixed: the code's oracle_text field (line 24) now says "Zombie cards" matching the current Scryfall oracle, not "Zombie creature cards."

The target requirement for mode 2 still uses `GraveyardCreatureOfSubtype("Zombie")` which adds a creature type check the oracle does not require. However, the Zombie subtype is only found on creature cards in the Innistrad card pool, so this has zero practical impact. Not flagged as an issue.

Card data verified correct:
- Mana cost: {B}
- Card types: Sorcery
- No supertypes, subtypes, P/T, keywords
- oracle_text: matches Scryfall
- Modal targeting via `TargetRequirement::ModalChoice`: mode 1 = GraveyardCreature, mode 2 = TwoTargets(GraveyardCreatureOfSubtype("Zombie"), GraveyardCreatureOfSubtype("Zombie"))
- is_valid_target: checks `o.zone == Zone::Graveyard && o.owner == caster` -- correct for "your graveyard"
- on_resolve: iterates targets, moves each from graveyard to hand, calls `move_spell_after_resolve` -- correct

### Tricky interactions checked
- Mode 1 returns any creature card: pass
- Mode 2 returns exactly two Zombie cards: pass
- Cannot target opponent's graveyard: pass (is_valid_target checks owner)
- Mode 2 not available with fewer than 2 Zombies: pass (tested)
- Mixed graveyard (some Zombies, some non-Zombies): pass (tested)
- Sorcery speed only: pass (card type Sorcery)

### Test coverage
- Mode 1 returns creature: `mtg-engine/tests/ghoulcallers_chant.rs:22` (mode1_returns_one_creature_from_graveyard)
- Mode 2 returns two Zombies: `mtg-engine/tests/ghoulcallers_chant.rs:38` (mode2_returns_two_zombies_from_graveyard)
- Legal actions include mode 1: `mtg-engine/tests/ghoulcallers_chant.rs:61` (legal_actions_include_mode1_single_creature)
- Legal actions include mode 2: `mtg-engine/tests/ghoulcallers_chant.rs:88` (legal_actions_include_mode2_two_zombies)
- No mode 2 for non-Zombies: `mtg-engine/tests/ghoulcallers_chant.rs:119` (legal_actions_no_mode2_for_non_zombies)
- Cannot target opponent's graveyard: `mtg-engine/tests/ghoulcallers_chant.rs:161` (cannot_target_opponents_graveyard)
- Mixed graveyard correct modes: `mtg-engine/tests/ghoulcallers_chant.rs:183` (mixed_graveyard_correct_modes)
- Fizzle (targets leave graveyard before resolution): NOT TESTED

## Audit — 2026-04-01 14:30

**Oracle text source**: Oracle cache (Scryfall API), cached 2026-04-01
**Oracle text**: Choose one —
• Return target creature card from your graveyard to your hand.
• Return two target Zombie cards from your graveyard to your hand.
**Type line**: Sorcery
**Mana cost**: {B}
**Status**: PASS

### Code issues
No issues found.

Card data verified:
- Mana cost {B}: correct (Colored(Black))
- Type: Sorcery: correct
- No supertypes: correct
- No subtypes: correct
- No P/T: correct
- No keywords: correct
- Oracle text: matches

Behavior verified:
- `target_requirement` uses `ModalChoice` with two modes: correct
  - Mode 1: `GraveyardCreature` — one creature card from graveyard: correct
  - Mode 2: `TwoTargets(GraveyardCreatureOfSubtype("Zombie"), GraveyardCreatureOfSubtype("Zombie"))` — two Zombie cards: correct
- `is_valid_target` restricts to caster's graveyard (zone == Graveyard && owner == caster): correct
- `on_resolve` moves each target from graveyard to hand with zone check: correct
- Uses `move_spell_after_resolve`: correct (not raw `move_object`)

### Tricky interactions checked
- Mode selection (1 creature vs 2 Zombies): pass — ModalChoice generates separate actions per mode
- Targets must be in caster's graveyard only: pass
- Mode 2 requires exactly 2 Zombie targets: pass

### Test coverage
- Mode 1 (return one creature): `mtg-engine/tests/tier11_cards.rs:105` (ghoulcallers_chant_returns_creature_from_graveyard)
- Mode 2 (return two Zombies): `mtg-engine/tests/tier11_cards.rs:120` (ghoulcallers_chant_returns_two_zombies)
- Fizzle (targets leave graveyard before resolution): NOT TESTED

## Audit — 2026-04-01 18:00

**Oracle text source**: Scryfall API (cached via oracle_lookup.py)
**Oracle text**: Choose one —
• Return target creature card from your graveyard to your hand.
• Return two target Zombie cards from your graveyard to your hand.
**Type line**: Sorcery
**Mana cost**: {B}
**Status**: PASS

### Code issues
No issues found.

Card data verified:
- Mana cost {B}: correct (Colored(Black))
- Card types: Sorcery: correct
- No supertypes, subtypes, P/T, keywords: correct
- Oracle text field: matches Scryfall

Behavior verified:
- `target_requirement`: `ModalChoice` with two modes: correct
  - Mode 1: `GraveyardCreature` -- targets one creature card from graveyard: correct
  - Mode 2: `TwoTargets(GraveyardCreatureOfSubtype("Zombie"), GraveyardCreatureOfSubtype("Zombie"))` -- targets two Zombie cards: correct. Note the engine requires these to be creature cards, but the Zombie subtype only appears on creatures in the Innistrad pool. Zero practical impact.
- `is_valid_target`: checks `o.zone == Zone::Graveyard && o.owner == caster`: correct for "your graveyard"
- `on_resolve`: iterates targets, moves each from graveyard to hand with zone check, calls `move_spell_after_resolve`: correct
- Uses `move_spell_after_resolve` (not raw `move_object`): correct

Not in LLM card knowledge section.

### Tricky interactions checked
- Mode 1 returns any creature card from your graveyard: pass
- Mode 2 returns exactly two Zombie cards from your graveyard: pass
- Cannot target opponent's graveyard: pass (tested)
- Mode 2 not available with fewer than 2 Zombies: pass (tested)
- Mixed graveyard (Zombies and non-Zombies): pass (tested)
- Sorcery speed only: pass (CardType::Sorcery)

### Test coverage
- Mode 1 returns creature: `mtg-engine/tests/ghoulcallers_chant.rs:22`
- Mode 2 returns two Zombies: `mtg-engine/tests/ghoulcallers_chant.rs:38`
- Legal actions include mode 1: `mtg-engine/tests/ghoulcallers_chant.rs:61`
- Legal actions include mode 2: `mtg-engine/tests/ghoulcallers_chant.rs:88`
- No mode 2 for non-Zombies: `mtg-engine/tests/ghoulcallers_chant.rs:119`
- Cannot target opponent's graveyard: `mtg-engine/tests/ghoulcallers_chant.rs:161`
- Mixed graveyard correct modes: `mtg-engine/tests/ghoulcallers_chant.rs:183`
- Fizzle (targets leave graveyard before resolution): NOT TESTED

## Audit — 2026-04-01 14:48

**Oracle text source**: Oracle cache (Scryfall API), cached 2026-04-01
**Oracle text**: Choose one —
* Return target creature card from your graveyard to your hand.
* Return two target Zombie cards from your graveyard to your hand.
**Type line**: Sorcery
**Mana cost**: {B}
**Status**: PASS

### Code issues
No issues found.

Card data verified:
- Mana cost {B}: correct (Colored(Black))
- Card types: Sorcery: correct
- No supertypes, subtypes, P/T, keywords: correct
- Oracle text field: matches Scryfall

Behavior verified:
- `target_requirement`: `ModalChoice` with two modes: correct
  - Mode 1: `GraveyardCreature` -- targets one creature card from graveyard: correct
  - Mode 2: `TwoTargets(GraveyardCreatureOfSubtype("Zombie"), GraveyardCreatureOfSubtype("Zombie"))` -- targets two Zombie cards: correct. The engine requires these to be creature cards, but the Zombie subtype only appears on creatures in the Innistrad pool. Zero practical impact.
- `is_valid_target`: checks `o.zone == Zone::Graveyard && o.owner == caster`: correct for "your graveyard"
- `on_resolve`: iterates targets, moves each from graveyard to hand with zone check, calls `move_spell_after_resolve`: correct
- Uses `move_spell_after_resolve` (not raw `move_object` to graveyard): correct

No anti-patterns detected. Not in LLM card knowledge section.

### Tricky interactions checked
- Mode 1 returns any creature card from your graveyard: pass
- Mode 2 returns exactly two Zombie cards from your graveyard: pass
- Cannot target opponent's graveyard: pass (is_valid_target checks owner)
- Mode 2 not available with fewer than 2 Zombies: pass (tested)
- Sorcery speed only: pass (CardType::Sorcery)
- Target validation checks zone is still Graveyard on resolution: pass (line 61)

### Test coverage
- Mode 1 returns creature: `mtg-engine/tests/tier11_cards.rs:105` (ghoulcallers_chant_returns_creature_from_graveyard)
- Mode 2 returns two Zombies: `mtg-engine/tests/tier11_cards.rs:120` (ghoulcallers_chant_returns_two_zombies)
- Fizzle (targets leave graveyard before resolution): NOT TESTED

## Audit — 2026-04-02

**Oracle text source**: Scryfall API (cached 2026-04-01, via oracle_lookup.py)
**Oracle text**: Choose one —
• Return target creature card from your graveyard to your hand.
• Return two target Zombie cards from your graveyard to your hand.
**Type line**: Sorcery
**Mana cost**: {B}
**Status**: PASS

### Card data
- Mana cost {B} (Colored(Black)): correct
- Card types: Sorcery: correct
- No supertypes, subtypes, P/T, keywords: correct
- Oracle text field (line 24): matches Scryfall verbatim

### Behavior

**Modal choice presentation**: `target_requirement` returns `ModalChoice` with two modes. The engine's `generate_cast_actions_with_targets` (engine.rs:845-851) iterates each mode and generates separate cast actions, so the player sees distinct options for mode 1 vs mode 2. Correct.

**Mode 1 -- return target creature card from your graveyard to your hand**: Uses `GraveyardCreature`. The engine (engine.rs:973-985) filters for creature cards in all graveyards, then the card's `is_valid_target` restricts to `o.zone == Zone::Graveyard && o.owner == caster`. Correct -- targets one creature card in your graveyard.

**Mode 2 -- return two target Zombie cards from your graveyard to your hand**: Uses `TwoTargets(GraveyardCreatureOfSubtype("Zombie"), GraveyardCreatureOfSubtype("Zombie"))`. The engine (engine.rs:852-866) generates the Cartesian product of valid targets and enforces `t1 != t2` (must be two different cards). The engine (engine.rs:987-1004) filters for creature cards with Zombie subtype in all graveyards, then `is_valid_target` restricts to caster's graveyard. The `GraveyardCreatureOfSubtype` additionally requires the card to be a creature, which the oracle does not strictly require (oracle says "Zombie cards" not "Zombie creature cards"). However, the Zombie subtype only appears on creatures in the Innistrad card pool, so this has zero practical impact. Not flagged as an issue.

**on_resolve**: Iterates all targets, checks each is still in the graveyard (zone check on line 61), moves to hand, logs the event, then calls `move_spell_after_resolve`. Correct.

### Tricky interactions checked
- Mode 1 returns any creature card from your graveyard: pass
- Mode 2 returns exactly two different Zombie cards from your graveyard: pass (TwoTargets + t1 != t2 check)
- Cannot target opponent's graveyard: pass (is_valid_target checks owner == caster)
- Mode 2 not available with fewer than 2 Zombies in graveyard: pass (Cartesian product yields nothing)
- Sorcery speed only: pass (CardType::Sorcery)
- Resolution zone check: pass (line 61 verifies target still in graveyard before moving)

### Test coverage
- Mode 1 returns creature from graveyard: `mtg-engine/tests/tier11_cards.rs:105`
- Mode 2 returns two Zombies from graveyard: `mtg-engine/tests/tier11_cards.rs:120`
- Both tests pass.
- Fizzle (targets leave graveyard before resolution): NOT TESTED

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Choose one —\n• Return target creature card from your graveyard to your hand.\n• Return two target Zombie cards from your graveyard to your hand.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found. Modal targeting is correctly implemented with ModalChoice containing GraveyardCreature for mode 1 and TwoTargets of GraveyardCreatureOfSubtype("Zombie") for mode 2. The is_valid_target correctly ensures targets are in the caster's graveyard. On resolve, all targeted cards are moved from graveyard to hand.

## Audit — 2026-04-02 21:09

**Oracle text source**: Scryfall API (via `oracle_lookup.py`, cached 2026-04-01)
**Oracle text**: Choose one —
• Return target creature card from your graveyard to your hand.
• Return two target Zombie cards from your graveyard to your hand.
**Type line**: Sorcery
**Mana cost**: {B}
**Status**: PASS

### Code issues
No issues found.

Card data verified (`mtg-engine/src/cards/isd/ghoulcallers_chant.rs`):
- Mana cost {B} (`Colored(Black)`): correct
- Card types: `Sorcery`: correct
- No supertypes, subtypes, P/T, keywords: correct
- Oracle text field (line 24): matches Scryfall verbatim

Behavior verified:
- `target_requirement` returns `ModalChoice` with two modes: correct
  - Mode 1: `GraveyardCreature` -- targets one creature card from graveyard: correct
  - Mode 2: `TwoTargets(GraveyardCreatureOfSubtype("Zombie"), GraveyardCreatureOfSubtype("Zombie"))` -- targets two Zombie cards: correct. The engine additionally requires targets to be creature cards (via `GraveyardCreatureOfSubtype`), but Zombie is a creature subtype so all Zombie cards in the Innistrad pool are creatures. Zero practical impact.
- `is_valid_target` (lines 44-55): checks `o.zone == Zone::Graveyard && o.owner == caster`: correct for "your graveyard"
- `on_resolve` (lines 57-71): iterates targets, checks each is still in graveyard (zone check at line 61), moves to hand, logs the event, calls `move_spell_after_resolve`: correct. No graveyard anti-pattern.
- Engine `TwoTargets` Cartesian product enforces `t1 != t2` (engine.rs line 993): correct -- cannot target the same Zombie twice.
- Engine `ModalChoice` generates separate cast actions per mode (engine.rs lines 979-984): correct -- player sees distinct options for mode 1 vs mode 2.

### Tricky interactions checked (min 3)
- Mode 2 requires exactly two *different* Zombie cards: pass (TwoTargets + engine `t1 != t2` check)
- Cannot target opponent's graveyard: pass (`is_valid_target` checks `owner == caster`)
- Mode 2 not available with fewer than 2 Zombies in graveyard: pass (Cartesian product yields empty)
- Mixed graveyard: mode 1 available for any creature, mode 2 only for Zombies: pass (tested)
- Resolution zone check prevents returning cards already removed from graveyard: pass (line 61)
- Sorcery speed only: pass (`CardType::Sorcery`)

### Test coverage
All 7 tests pass (`cargo test --test ghoulcallers_chant`):
- `mode1_returns_one_creature_from_graveyard` (line 22)
- `mode2_returns_two_zombies_from_graveyard` (line 38)
- `legal_actions_include_mode1_single_creature` (line 61)
- `legal_actions_include_mode2_two_zombies` (line 88)
- `legal_actions_no_mode2_for_non_zombies` (line 119)
- `cannot_target_opponents_graveyard` (line 161)
- `mixed_graveyard_correct_modes` (line 183)
Additional tests in `mtg-engine/tests/tier11_cards.rs` (lines 105, 120).
Not tested: fizzle when targets leave graveyard before resolution (low priority -- on_resolve has zone check).
