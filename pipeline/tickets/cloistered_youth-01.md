---
id: cloistered_youth-01
status: closed-duplicate
card: Cloistered Youth
card_file: mtg-engine/src/cards/isd/cloistered_youth.rs
created: 2026-04-15T03:43:08Z
audit_run_id: 2026-04-14-cloistered_youth-audit
audit_model: opus
audit_tokens: 11289
audit_duration: 242
duplicate_of: merged-dfc-zone-cleanup-02
---

## Audit Finding

**Oracle text:**
> (CR 712.8a) "While a double-faced card is outside the game or in a zone other than the battlefield or stack, it has only the characteristics of its front face."

**Code:**
> `helpers::apply_transform` (helpers.rs:287-289) writes `obj.name = "Unholy Fiend"` and `obj.subtypes = ["Horror"]`. `move_object` (state.rs:572-583) clears `is_transformed` but does NOT clear `name` or `subtypes`.

**Description:**
When Cloistered Youth transforms into Unholy Fiend and then leaves the battlefield (dies, is exiled, bounced), `move_object` resets `is_transformed` to false but leaves `obj.name` as "Unholy Fiend" and `obj.subtypes` as ["Horror"]. Since `obj_name()` reads `obj.name` directly with no registry fallback, the card presents as "Unholy Fiend" (a Horror) in the graveyard instead of "Cloistered Youth" (a Human). This violates CR 712.8a which requires front-face characteristics in non-battlefield zones. This affects any game logic that checks card name or subtypes in the graveyard (e.g., "return target Human from your graveyard" would miss this card; delirium counting could see wrong types).

**Engine path:**
- helpers.rs:287-289 (`apply_transform` writes name/subtypes to object)
- state.rs:572-583 (`move_object` cleanup block — clears `is_transformed` but not `name`/`subtypes`)

**Required check:** 8a

**Affected cards:**
- Cloistered Youth // Unholy Fiend
- All other DFCs that use `helpers::apply_transform` (engine-wide issue)

## Tests

### cloistered_youth_name_reverts_on_death
Source ticket: (new)
Implementation: (not yet written)
Scenario: Cast Cloistered Youth. Transform it into Unholy Fiend via the upkeep trigger. Destroy Unholy Fiend (send to graveyard). Assert the card's name in the graveyard is "Cloistered Youth", not "Unholy Fiend". Assert subtypes are ["Human"], not ["Horror"].

### cloistered_youth_name_reverts_on_exile
Source ticket: (new)
Implementation: (not yet written)
Scenario: Cast Cloistered Youth. Transform it into Unholy Fiend. Exile it. Assert the card's name in exile is "Cloistered Youth". Assert subtypes are ["Human"].
