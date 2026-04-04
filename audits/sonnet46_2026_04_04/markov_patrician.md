## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Lifelink (Damage dealt by this creature also causes you to gain that much life.)
**Type line**: Creature — Vampire
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Lifelink with damage prevention (e.g., Ghostly Possession): `deal_damage_to_creature` returns early before reaching the lifelink block, so no life is gained when damage is prevented — correct per MTG rules: pass
- Lifelink with combat damage multiplier (Inquisitor's Flail): `amount` is multiplied before the lifelink check at line 474, so life gained equals the multiplied damage — correct: pass
- `has_keyword` covers all keyword sources (object keywords vec, registry card data, continuous effect grants, conditional keywords): confirms lifelink is detected correctly regardless of how it was granted: pass
- Lifelink fires for creature-to-creature damage (`deal_damage_to_creature`, combat.rs:473-484) and creature-to-player damage (`deal_damage_to_player`, combat.rs:538-549): both code paths present: pass
- Module registration: `pub mod markov_patrician;` present in `isd/mod.rs` line 138: pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Keywords contain Lifelink, power=3, toughness=1: `mtg-engine/tests/innistrad_cards.rs:96` (`markov_patrician_has_lifelink`)
- Lifelink actually causes controller to gain life during combat: NOT TESTED
- Mana cost {2}{B}: NOT TESTED
- Subtype Vampire: NOT TESTED
