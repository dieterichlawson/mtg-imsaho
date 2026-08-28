## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/36/stony-silence?utm_source=api
**Type line**: `Enchantment` — {1}{W}
**Oracle text**:
```
Activated abilities of artifacts can't be activated.
```
**Status**: PASS

### Code issues
No issues found.

### What was checked
This card declares no behaviour hooks — no `on_resolve`, no triggered abilities,
no activated abilities. Everything it does is card data plus declarative `continuous_effects`,
so the audit is steps 1, 2, 6 and 9 in full; step 3 is skipped, which the
procedure directs for vanilla creatures and basic spells.

- Mana cost, card types, supertypes, subtypes, power/toughness and oracle text
  compared character-for-character against the cached Scryfall entry: exact.
- Keywords checked against the oracle text of this face: complete, with no
  keyword declared that the text does not grant.
- Flashback cost: none, and the oracle names none.
- Trigger kinds: none declared, and the oracle text contains no triggered-ability
  phrasing that would need one.
- `continuous_effects` compared clause by clause against the static abilities in the oracle text, including the scope distinction between "creatures you control" (`Global`) and "**other** creatures you control" (`GlobalOther`).
- Step 9 anti-patterns: clean. No self spell-cleanup, no `obj.power` used as a
  creature test, no `CombatDamageDealt` for non-combat damage, no token created
  without its subtypes, no hook left undeclared.

### Tricky interactions checked
None apply: with no triggered or activated ability there is no stack entry to
outlive its source, no target to re-check on resolution, and no choice to
present.

### Test coverage
Registry-wide invariants in `card_data_invariants.rs` cover this card's data
consistency (P/T exactly on creatures, subtypes implying their card type, every
declared keyword printed on the card, no field declared twice).
Static-ability behaviour is exercised through the shared continuous-effects tests in `continuous_effects.rs` and `snapshot_anthems.rs`.


## Audit — 2026-08-28 20:21

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Activated abilities of artifacts can't be activated.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues
Card data matches (`mtg-engine/src/cards/isd/stony_silence.rs`: {1}{W} Enchantment, `ContinuousEffect::PreventArtifactAbilities`). The audit found the ban was offer-side only:

- **A submitted artifact activation walked past the ban** (`engine/actions/abilities.rs`, `engine/mana_sources.rs`).
  - Oracle text says: `Activated abilities of artifacts can't be activated.` (Ruling: "No abilities of artifacts can be activated, including mana abilities." Ruling: equip has a colon — it is an activated ability.)
  - Code did: only `legal_actions` filtered artifact abilities. A submitted `ActivateManaAbility` on Sol Ring, a `CastSpell` whose tap plan named Sol Ring, and a submitted equip all executed under Stony Silence.
  - Fix: the gate now also holds at the do-sites — `activate_mana_source` refuses a silenced source (covering standalone activations AND tap plans; the missing mana turns the cast into a funding-rehearsal refusal), and `activate_ability` refuses an artifact's ability before targets or costs. Committed separately.

### Tricky interactions checked
- Ruling "including mana abilities": both offer (auto-tap planner skips artifacts) and submit (new gate). PASS (after fix)
- Ruling "equip ... activated ability": equip is an `ActivatedAbilityDef` on the Equipment (an artifact) — refused. NEW test.
- Ruling "battlefield only; triggered abilities unaffected": the gate keys on battlefield objects' activated/mana abilities only; triggered abilities (Witchbane Orb's ETB) run through the trigger system untouched. PASS
- Affects ALL players' artifacts: `prevents_artifact_abilities` reads `global_effects`, not a controller's. PASS
- Abilities an artifact GRANTS to a creature (Blazing Torch's granted ability) are abilities of the creature, not of an artifact — correctly not blocked (gate keys on the ability's holder). PASS

### Test coverage
- Mana ability blocked / non-artifact unaffected (offer side): `cards_lands_and_mana_sources.rs` `stony_silence_blocks_artifact_mana_abilities`, `stony_silence_does_not_block_non_artifact_mana`
- Submit side (all three shapes): `submitted_targets.rs` `stony_silence_submits::{a_submitted_artifact_mana_ability_is_refused, a_submitted_tap_plan_naming_an_artifact_source_cannot_fund_a_cast, a_submitted_equip_is_refused}` (NEW)

Mutation checks: disabling the mana-source gate fails the standalone-activation test (the tap-plan one stays green because the funding rehearsal still refuses the unpayable {G} — layered defense); disabling the ability gate fails the equip test. Both bite.
