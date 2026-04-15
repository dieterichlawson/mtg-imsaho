---
id: devils_play-02
status: closed-duplicate
card: Devil's Play
card_file: mtg-engine/src/cards/isd/devils_play.rs
created: 2026-04-14T21:24:56Z
audit_run_id: 2026-04-14-devils_play-audit
audit_model: opus
audit_tokens: 12112
audit_duration: 301
duplicate_of: merged-flashback-cost-reduction-01
---

## Audit Finding

**Oracle text:**
> Flashback {X}{R}{R}{R}

**Code:**
> `engine.rs:2219-2227`:
> ```rust
> } else if is_flashback {
>     let dynamic_fb = new_state.until_end_of_turn.iter()
>         .find_map(|e| if let crate::state::TemporaryEffect::GrantFlashback { target, cost } = e {
>             if *target == *object_id { Some(cost.clone()) } else { None }
>         } else { None });
>     dynamic_fb.unwrap_or_else(|| {
>         data.flashback_cost.expect("flashback cast on card without flashback_cost")
>     })
> ```
> The flashback branch returns the raw flashback cost. The non-flashback branch (line 2229-2230) calls `effective_spell_cost()` which applies cost reductions. The flashback branch does not.

**Description:**
Per CR 601.2f, the total cost of a spell is computed by starting with the mana cost or alternative cost (flashback cost qualifies as an alternative cost per CR 702.34a), adding cost increases, then applying cost reductions. The engine's `CastSpell` handler applies `effective_spell_cost` (which gathers `ReduceCost` continuous effects from controlled permanents) only for non-flashback casts. Flashback casts use the raw `flashback_cost` from `CardData` without any cost modification. This means cost reduction effects (e.g., a creature with `ReduceCost { reduction: 1, filter: SpellFilter::All }`) would reduce Devil's Play's normal cast cost but NOT its flashback cost, violating CR 601.2f. The same skip also applies in `legal_actions` (engine.rs:1263-1267), where the flashback affordability check uses the raw cost.

**Engine path:**
- `mtg-engine/src/engine.rs:2219-2227` — flashback cost computed without `effective_spell_cost`
- `mtg-engine/src/engine.rs:2229-2230` — non-flashback path correctly applies `effective_spell_cost`
- `mtg-engine/src/engine.rs:261` — `effective_spell_cost` definition (gathers ReduceCost effects)

**Required check:** 8i

**Affected cards:**
- Devil's Play
- All cards with `flashback_cost` (every flashback card in the engine)
