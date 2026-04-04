## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant player
Creatures enchanted player controls get -1/-1.
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Continuous evaluation vs. snapshot: `continuous_pt_mods` is called fresh on every `effective_power`/`effective_toughness` invocation, so the -1/-1 re-evaluates dynamically as creatures enter or leave the cursed player's control — pass
- AttachedPlayer filter via `effect_applies_to`: for `EffectScope::Global(CreatureFilter::AttachedPlayer)`, the engine reads `source_id`'s `attached_to_player` field and compares it to the creature's `controller`; the `matches_filter` fallback (which incorrectly uses `controller != source_controller`) is bypassed by the special-case branch in `effect_applies_to` — pass
- Curse owner's own creatures unaffected: `effect_applies_to` checks `c.controller == attached_player`, not `c.controller != source_controller`, so the curse correctly affects all and only the cursed player's creatures regardless of who cast the curse — pass
- 1/1 creatures die to -1/-1: SBA calls `effective_toughness` (which includes continuous modifiers), and any creature with effective toughness ≤ 0 is sent to graveyard via rule 704.5f path — pass
- Curse absent `attached_to_player`: if the field is `None`, `effect_applies_to` returns `false` immediately — no spurious application — pass
- Aura resolution via `resolve_curse`: moves curse to `Zone::Battlefield` directly (correct; does not call `move_spell_after_resolve`), sets `attached_to_player`, and clears `summoning_sick` — pass
- Target is any player (including self): `TargetRequirement::PlayerOnly` does not restrict to opponents; a player could legally target themselves — pass
- "Enchant" not in `keywords` vec: "Enchant" is handled through `TargetRequirement::PlayerOnly` and `resolve_curse`; it is not a keyword ability in the engine's `Keyword` enum — pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Opponent's creatures get -1/-1: `mtg-engine/tests/tier7_cards.rs:197` — TESTED
- Curse owner's own creatures unaffected: `mtg-engine/tests/tier7_cards.rs:197` (asserts `own_creature` power stays 3) — TESTED
- 1/1 creatures dying from -1/-1 toughness: NOT TESTED
- Curse targeting self: NOT TESTED
- Effect correctly absent when `attached_to_player` is None: NOT TESTED
