# Audit: Dead Weight

## Scryfall Reference
- **Name:** Dead Weight
- **Cost:** {B}
- **Type:** Enchantment -- Aura
- **Oracle:** Enchant creature. Enchanted creature gets -2/-2.
- **P/T:** N/A
- **Keywords:** Enchant

## Implementation: `dead_weight.rs`
- **Name:** Dead Weight -- CORRECT
- **Cost:** {B} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Subtypes:** ["Aura"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Continuous effect:** ModifyPT { power: -2, toughness: -2, scope: Attached } -- CORRECT
- **Target:** TargetRequirement::Creature -- CORRECT

## Issues
None

---

## Audit 2 (2026-04-02)

### Oracle Text (Scryfall)
```
Enchant creature
Enchanted creature gets -2/-2.
```
- **Name:** Dead Weight
- **Cost:** {B}
- **Type:** Enchantment — Aura
- **Keywords:** Enchant

### Implementation (`mtg-engine/src/cards/isd/dead_weight.rs`)

| Field | Oracle | Implementation | Verdict |
|---|---|---|---|
| Name | Dead Weight | `"Dead Weight"` | CORRECT |
| Mana cost | {B} | `ManaCost::new(vec![ManaSymbol::Colored(Color::Black)])` | CORRECT |
| Card types | Enchantment | `vec![CardType::Enchantment]` | CORRECT |
| Subtypes | Aura | `vec!["Aura".into()]` | CORRECT |
| P/T | N/A | `None / None` | CORRECT |
| Keywords | Enchant | `vec![]` | ACCEPTABLE — engine has no `Keyword::Enchant` variant; "Enchant creature" is modeled structurally via `TargetRequirement::Creature` and `resolve_aura` |
| Oracle text | "Enchanted creature gets -2/-2." | `"Enchanted creature gets -2/-2."` | CORRECT |
| Continuous effect | -2/-2 to enchanted creature | `ModifyPT { power: -2, toughness: -2, scope: EffectScope::Attached }` | CORRECT |
| Target requirement | Enchant creature | `TargetRequirement::Creature` | CORRECT |
| on_resolve | Attach aura to target creature | `resolve_aura(state, object_id, targets)` | CORRECT |

### Aura Mechanics
- **resolve_aura:** Checks target is still on battlefield, moves aura to battlefield, sets `attached_to`. If target is gone, aura goes to graveyard via `move_spell_after_resolve`. CORRECT.
- **Continuous effect application:** `effect_applies_to` with `EffectScope::Attached` checks `source.attached_to == creature_id`. Only the enchanted creature receives -2/-2. CORRECT.
- **SBA 704.5m:** Unattached auras on the battlefield are moved to graveyard. Implemented in `sba.rs`. CORRECT.
- **SBA 704.5f:** Zero-toughness creatures (e.g. 2/2 enchanted with Dead Weight becomes 0/0) go to graveyard. Implemented in `sba.rs` using `effective_toughness` which includes continuous effects. CORRECT.

### Test Coverage
- `dead_weight_kills_small_creature` in `mtg-engine/tests/innistrad_cards.rs`: Casts Dead Weight on a 2/2, asserts effective P/T is 0/0, runs SBA, asserts creature is in graveyard. CORRECT.

### Issues
None. Implementation is fully correct and matches oracle text.
