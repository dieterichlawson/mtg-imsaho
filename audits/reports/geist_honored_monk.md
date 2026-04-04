## Audit — 2026-04-02 21:03
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/17/geist-honored-monk)
**Oracle text**: Vigilance
Geist-Honored Monk's power and toughness are each equal to the number of creatures you control.
When this creature enters, create two 1/1 white Spirit creature tokens with flying.
**Type line**: Creature — Human Monk
**Status**: PASS
### Code issues
- Minor: stored `oracle_text` uses old template wording "When Geist-Honored Monk enters the battlefield" vs current Scryfall wording "When this creature enters". Functionally identical; no behavioral impact.
- `power: Some(0)` / `toughness: Some(0)` used as base values for a `*/*` creature. This works correctly because `dynamic_pt` overrides these values in `effective_power`/`effective_toughness`, so the `0` base is never exposed.
### Tricky interactions checked (min 3)
1. **Monk counts itself**: `dynamic_pt` counts all creatures controller controls on the battlefield including itself via `o.power.is_some()`. Ruling confirms: "As long as Geist-Honored Monk is on the battlefield, its second ability will count itself." Verified the Monk (with `power: Some(0)`) passes the `power.is_some()` filter. Correct.
2. **ETB token creation order**: The Monk enters first, then triggers create two Spirit tokens. After resolution, `dynamic_pt` returns 3 (Monk + 2 Spirits). Test `geist_honored_monk_dynamic_pt_and_tokens` verifies P/T is 3/3 with 3 creatures. Correct.
3. **Dynamic P/T updates as creatures enter/leave**: Because `dynamic_pt` is called fresh every time `effective_power`/`effective_toughness` is computed (no caching), the P/T correctly tracks the current creature count. If a Spirit dies, the Monk shrinks. If more creatures enter, it grows. Correct.
4. **P/T in all zones**: Ruling says "The ability that defines Geist-Honored Monk's power and toughness works in all zones." The `dynamic_pt` implementation does not check zone, so it would work if called from non-battlefield contexts. However, `effective_power`/`effective_toughness` are only called for battlefield objects in practice. This is an acceptable engine limitation since P/T in non-battlefield zones is rarely relevant (mainly for effects like Collected Company or Chord of Calling that check P/T in library, which are not in this card set).
### Test coverage
- `geist_honored_monk_dynamic_pt_and_tokens` in `tier5_cards.rs`: Verifies ETB creates 2 Spirit tokens, total creature count is 3, and effective P/T is 3/3. PASSES.
- LLM card knowledge in `llm.rs` is accurate: describes vigilance, `*/*` = creatures you control, ETB two 1/1 Spirits with flying.
