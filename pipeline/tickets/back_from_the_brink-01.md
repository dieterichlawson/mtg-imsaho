---
id: back_from_the_brink-01
status: new
card: Back from the Brink
card_file: mtg-engine/src/cards/isd/back_from_the_brink.rs
created: 2026-04-14T21:24:13Z
audit_run_id: 2026-04-14-back_from_the_brink-audit
audit_model: opus
audit_tokens: 20377
audit_duration: 499
---

## Audit Finding

**Oracle text:**
> If the exiled creature card has {X} in its mana cost, X is considered to be zero. (Ruling, 2011-09-22)

**Code:**
> `activated_abilities()` at back_from_the_brink.rs:61-63 passes the creature's full registry mana cost (including `ManaSymbol::X`) to `ActivatedAbilityDef.cost`. The engine detects X at engine.rs:2638 (`ab.cost.symbols.iter().any(|s| matches!(s, ManaSymbol::X))`) and, when the player has available mana, opens a `ChooseXFunding` prompt (engine.rs:2686-2696) allowing X > 0.

**Description:**
When a creature card in the graveyard has {X} in its mana cost (e.g., Hangarback Walker at {X}{X}), Back from the Brink generates an activated ability whose cost includes ManaSymbol::X. The engine's X-cost ability handler then prompts the player to choose how much to fund X, allowing any value from 0 to the player's available mana. Per the official ruling, X is always considered to be zero when paying via Back from the Brink — the player should not be prompted and X should be forced to 0. The `activated_abilities()` method should strip ManaSymbol::X symbols from the creature's cost before setting the ability's cost.

**Engine path:**
- back_from_the_brink.rs:61-63 (mana cost passed through unchanged)
- engine.rs:2638-2706 (X-cost ability detection and ChooseXFunding prompt)

**Required check:** 8i

**Affected cards:**
- Back from the Brink (when targeting any X-cost creature in graveyard)

