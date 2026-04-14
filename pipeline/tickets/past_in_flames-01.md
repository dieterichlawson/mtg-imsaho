---
id: past_in_flames-01
status: new
card: Past in Flames
card_file: mtg-engine/src/cards/isd/past_in_flames.rs
created: 2026-04-14T20:57:22Z
audit_run_id: 2026-04-14-past_in_flames-audit
audit_model: opus
audit_tokens: 20370
audit_duration: 422
---

## Audit Finding

**Oracle text:**
> Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.

**Code:**
> `d.cost.clone().unwrap_or(ManaCost::free())` — `past_in_flames.rs:53`

**Description:**
When Past in Flames grants flashback, it determines the flashback cost via `d.cost.clone().unwrap_or(ManaCost::free())`. For cards with `cost: None` (no mana cost, e.g., suspend-only cards like Living End or Ancestral Vision), this falls back to `ManaCost::free()` — an empty mana cost that is trivially payable. Per the ruling, such cards should have no flashback cost and therefore be uncastable via PiF's granted flashback. The fix is to skip cards where `d.cost` is `None` instead of falling back to a free cost.

**Engine path:**
- `mtg-engine/src/cards/isd/past_in_flames.rs:53` — `unwrap_or(ManaCost::free())`
- `mtg-engine/src/engine.rs:1226-1229` — dynamic flashback offering uses the stored cost directly
- `mtg-engine/src/engine.rs:2221-2224` — dynamic flashback casting uses the stored cost directly

**Required check:** 8j (ruling: "If a card with no mana cost gains flashback, it has no flashback cost")

**Affected cards:**
- Past in Flames
- Any no-mana-cost instant/sorcery that could end up in the graveyard (e.g., if the card pool expands beyond Innistrad)

