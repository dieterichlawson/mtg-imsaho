---
id: delver_of_secrets-01
status: closed-duplicate
card: Delver of Secrets
card_file: mtg-engine/src/cards/isd/delver_of_secrets.rs
created: 2026-04-14T20:53:00Z
audit_run_id: 2026-04-14-delver_of_secrets-audit
audit_model: opus
audit_tokens: 7789
audit_duration: 160
duplicate_of: merged-dfc-zone-cleanup-02
---

## Audit Finding

**Oracle text:**
> At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
>
> Back face (Insectile Aberration): Creature — Human Insect, 3/2, Flying

**Code:**
> `helpers.rs:286-289`: `apply_transform` writes back-face `name` ("Insectile Aberration"), `keywords` ([Flying]), and `subtypes` (["Human", "Insect"]) onto `obj`.
>
> `state.rs:572-583`: `move_object` zone-change cleanup clears `is_transformed` (line 580) but does NOT clear `name`, `keywords`, or `subtypes`.

**Description:**
When a transformed Delver of Secrets (showing Insectile Aberration) leaves the battlefield, `move_object` resets `is_transformed` to false but leaves the back-face name, keywords, and subtypes on the object. In the graveyard, hand, or exile, the card retains name "Insectile Aberration", keyword Flying, and subtypes Human Insect instead of reverting to front-face characteristics (name "Delver of Secrets", no keywords, subtypes Human Wizard). This violates CR 712.8a: "While a double-faced card is outside the game or in a zone other than the battlefield or stack, it has only the characteristics of its front face." This is the same engine-level issue documented in auditor-insights.md ("DFC transform and zone-change cleanup: `obj.name` has no registry fallback"). It could matter for effects that check card name, type, or keywords in non-battlefield zones (e.g., searching graveyard for a card named "Delver of Secrets" would fail to find it).

**Engine path:**
- helpers.rs:262-293 (`apply_transform` writes back-face fields)
- state.rs:572-583 (`move_object` cleanup omits name/keywords/subtypes)

**Required check:** 8a

**Affected cards:**
- Delver of Secrets // Insectile Aberration
- All DFCs that use `apply_transform` (every werewolf, Bloodline Keeper, Cloistered Youth, etc.)
