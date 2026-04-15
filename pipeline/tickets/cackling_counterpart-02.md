---
id: cackling_counterpart-02
status: closed-duplicate
card: Cackling Counterpart
card_file: mtg-engine/src/cards/isd/cackling_counterpart.rs
created: 2026-04-14T21:25:14Z
audit_run_id: 2026-04-14-cackling_counterpart-audit
audit_model: opus
audit_tokens: 27785
audit_duration: 560
duplicate_of: merged-token-copy-inconsistent-01
---

## Audit Finding

**Code:**
> `state.rs:487` — `let (name, power, toughness, card_id, is_legendary) = match source { Some(o) => (o.name.clone(), o.power, o.toughness, o.card_id, o.is_legendary), ... };`

**Description:**
`create_token_copy` reads `power` and `toughness` from the runtime game object (`o.power`, `o.toughness`), which can include non-copy modifications. Per the ruling, the copy should reflect the printed values only. Cards that directly mutate `obj.power` or `obj.toughness` at runtime — such as Tree of Redemption (`tree_of_redemption.rs:72`: `obj.toughness = Some(current_life)`) — cause the copy to inherit the modified values instead of the printed ones. The registry has the printed P/T via `card_data(card_id).power`/`.toughness`, but the function reads from the object instead. This is the mirror image of Finding 1: for keywords/subtypes/colors, the function reads from the registry (ignoring object-level changes); for P/T, it reads from the object (including non-copy changes). Neither approach is universally correct.

**Engine path:**
- state.rs:487 (`create_token_copy` — reads o.power, o.toughness)
- cards/isd/tree_of_redemption.rs:72 (example of direct P/T mutation: `obj.toughness = Some(current_life)`)

**Required check:** 8g

**Affected cards:**
- Cackling Counterpart (when targeting a creature whose P/T was modified by a non-copy effect)
- Any card that calls `create_token_copy`
