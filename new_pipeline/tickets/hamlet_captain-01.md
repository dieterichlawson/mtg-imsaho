---
id: hamlet_captain-01
status: fixed
card: Hamlet Captain
audit_run_id: 2026-04-19-hamlet_captain-audit
audit_model: sonnet
audit_tokens: 16223
audit_duration: 307
fixed_sha: fc41ee775c2558a71e0743f1f9af70a119e52574
fixed_at: 2026-08-23T17:28:19Z
test_file: mtg-engine/tests/characteristics_card_sweep.rs
fix_note: cluster fix: card code now reads characteristics through the GameState accessors (has_card_type / is_creature / has_subtype)
---

## Audit Finding

**Oracle text:**
> other Humans you control get +1/+1 until end of turn

**Code:**
> o.subtypes.iter().any(|s| s == "Human")
|| registry.card_data(o.card_id)
    .is_some_and(|d| d.subtypes.iter().any(|s| s == "Human"))

**Description:**
The `buff_humans` helper in hamlet_captain.rs (lines 69–73) determines Human subtype via two checks: first `obj.subtypes` (which `apply_transform` correctly updates to back-face subtypes on transformation), then a fallback to `registry.card_data(o.card_id)` which ALWAYS returns front-face data regardless of `o.is_transformed`. For any DFC whose front face has the Human subtype but back face does not — every front-face Human werewolf, Delver of Secrets, Cloistered Youth, and Civilized Scholar — once transformed, `obj.subtypes` correctly contains only the back-face types (e.g. ["Werewolf"]), but the registry fallback still finds "Human" in the front-face card data. The result is that Hamlet Captain incorrectly buffs transformed non-Human DFC creatures. The canonical fix is in `state.rs` `matches_filter::HasSubtype` (lines 869–886): branch on `o.is_transformed` and use `registry.get(o.card_id).and_then(|b| b.back_face_data())` when true, falling through to `registry.card_data()` only for the non-transformed case.

**Engine path:** mtg-engine/src/cards/isd/hamlet_captain.rs:69

**Required check:** 8d

**Affected cards:**
- Villagers of Estwald
- Reckless Waif
- Gatstaf Shepherd
- Hanweir Watchkeep
- Kruin Outlaw
- Daybreak Ranger
- Village Ironsmith
- Instigator Gang
- Tormented Pariah
- Grizzled Outcasts
- Ulvenwald Mystics
- Mayor of Avabruck
- Delver of Secrets
- Cloistered Youth
- Civilized Scholar

## Tests

### hamlet_captain_does_not_buff_transformed_werewolf
Scenario: Hamlet Captain attacks; a Villagers of Estwald that has transformed into Howlpack of Estwald (back face, Werewolf subtype only) is also on the battlefield; Hamlet Captain's buff should NOT apply to the transformed werewolf, but currently does because registry.card_data() returns front-face Human subtype.

### hamlet_captain_buffs_non_transformed_werewolf
Scenario: Hamlet Captain attacks; a non-transformed Villagers of Estwald (front face, Human Werewolf) is also on the battlefield; Hamlet Captain's buff correctly applies — this test guards against over-correction removing the buff for the non-transformed case.

