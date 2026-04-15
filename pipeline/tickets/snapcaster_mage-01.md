---
id: snapcaster_mage-01
status: deduped
card: Snapcaster Mage
card_file: mtg-engine/src/cards/isd/snapcaster_mage.rs
created: 2026-04-14T20:56:41Z
audit_run_id: 2026-04-14-snapcaster_mage-audit
audit_model: opus
audit_tokens: 19116
audit_duration: 381
deduped_into: merged-cost-none-free-01
---

## Audit Finding

**Oracle text:**
> "If a card with no mana cost gains flashback, it has no flashback cost. It can't be cast this way." (Ruling, 2021-03-19)

**Code:**
> `snapcaster_mage.rs:63-65`: `let cost = registry.card_data(obj.card_id).and_then(|d| d.cost.clone()).unwrap_or_else(ManaCost::free);`
> `engine.rs:3922-3924`: `.and_then(|d| d.cost.clone()).unwrap_or(ManaCost::free())`

**Description:**
When the targeted card has no mana cost (`CardData.cost == None`), both the Snapcaster handler and the engine's `PendingEffect::GrantFlashback` resolver fall back to `ManaCost::free()` (an empty symbol list). This grants a flashback cost of {0}, making the card castable for free from the graveyard. Per the Scryfall ruling (which restates CR 107.2), a card with no mana cost simply cannot be cast via this flashback grant. The correct behavior is to either skip the grant entirely or record the flashback cost as `None` so the offering code rejects it.

**Engine path:**
- snapcaster_mage.rs:63-65 (`on_enter_battlefield` — `unwrap_or_else(ManaCost::free)`)
- engine.rs:3922-3924 (`PendingEffect::GrantFlashback` resolver — same pattern)
- engine.rs:1226-1228 (offering path reads `GrantFlashback.cost` and offers the spell)

**Required check:** 8j (ruling: "If a card with no mana cost gains flashback...")

**Affected cards:**
- Snapcaster Mage
- Past in Flames (if it uses the same `PendingEffect::GrantFlashback` path)
- Any future card that grants flashback dynamically
