## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Creature spells you cast cost {2} less to cast.
Creatures you control get -1/-1.
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Reduction only applies to generic mana (ruling)**: The engine (`engine.rs:112-125`) iterates over `base_cost.symbols` and only reduces `ManaSymbol::Generic(n)` values, pushing all other symbols (`Colored`, `Colorless`, `X`) unchanged. Correctly implements the ruling that the reduction can only reduce the generic mana portion. PASS
- **Cost floor at 0 (can't go negative)**: When `remaining_reduction` exceeds the generic amount available, the excess is simply discarded after the loop ends — no underflow possible. PASS
- **"You cast" controller check**: `engine.rs:81` guards with `obj.controller != caster`, so only permanents controlled by the spell's caster contribute cost reduction. A stolen Heartless Summoning benefits the new controller, not the original owner. PASS
- **Continuous re-evaluation of -1/-1**: The `ModifyPT` effect is applied by `continuous_pt_mods` (called from `effective_power` / `effective_toughness`) on every query, not snapshotted at ETB. PASS
- **SBA kills creatures reduced to 0 toughness**: `sba.rs:64-66` calls `state.effective_toughness(id, registry)` which includes the -1/-1 from Heartless Summoning. A 1/1 entering under Heartless Summoning correctly gets toughness 0 and is put into the graveyard by SBA (rule 704.5f). PASS
- **Heartless Summoning not self-affected by -1/-1**: Heartless Summoning is an Enchantment with `power: None` and `toughness: None`. The SBA creature filter (`sba.rs:55`) checks `o.power.is_some()`, so Heartless Summoning is never treated as a creature and is not subject to its own debuff. PASS
- **Non-creature spells not reduced**: `engine.rs:87-92` matches `SpellFilter::CreatureSpells` only when `is_creature` is true (checked via `registry.card_data(card_id).map(|d| d.card_types.contains(&CardType::Creature))`). Instants, sorceries, enchantments etc. receive no reduction. PASS
- **Multiple Heartless Summonings stacking**: The engine accumulates `total_reduction` across all battlefield objects controlled by the caster (`engine.rs:79-98`). Two Heartless Summonings correctly give {4} total generic reduction. The -1/-1 debuffs also stack since `continuous_pt_mods` sums contributions from all sources. PASS
- **Reduction does not interact with `modified_cost` early return**: The only card in the codebase that overrides `modified_cost` is Blasphemous Act (a Sorcery, not a creature spell). No creature cards use `modified_cost`, so the early-return path at `engine.rs:63-68` cannot skip Heartless Summoning's reduction for any currently implemented creature spell. PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- -1/-1 applied to controlled creatures: `tests/tier14_cards.rs:149` (heartless_summoning_gives_minus_one)
- Creature spell cost reduced by {2} (generic): `tests/tier14_cards.rs:166` (heartless_summoning_reduces_creature_cost)
- Non-creature spells not reduced: `tests/tier14_cards.rs:189` (heartless_summoning_no_reduce_noncreature)
- Ruling — reduction only applies to generic mana (not colored): NOT TESTED
- SBA killing a 1-toughness creature when Heartless Summoning is on the battlefield: NOT TESTED
- Cost floor at 0 (creature spell with no generic mana): NOT TESTED
- Multiple Heartless Summonings stacking: NOT TESTED
