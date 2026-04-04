## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves. (Only double-faced cards can be transformed.)
**Type line**: Instant
**Status**: ISSUE

### Code issues

- Second Moonmist fails to transform Werewolf DFCs after they naturally untransform back to their Human front face
  - Oracle text says: `Transform all Humans.`
  - Code does: In `on_resolve` (lines 43–56 of `mtg-engine/src/cards/isd/moonmist.rs`), the Human-detection logic first checks `if !o.subtypes.is_empty()` and, when that is true, uses only `o.subtypes` without also consulting registry data for the face currently showing. When a prior Moonmist transformed a Werewolf DFC front→back it set `obj.subtypes = ["Werewolf"]` (back-face subtypes). If the creature subsequently untransforms back to its front face via the natural day/night upkeep trigger (e.g. `GatstafShepherd::on_upkeep` at `mtg-engine/src/cards/isd/gatstaf_shepherd.rs:83–89`), `is_transformed` is toggled to `false` but `obj.subtypes` is not cleared. A second Moonmist then sees `obj.subtypes` non-empty, enters the first branch, finds no `"Human"` in `["Werewolf"]`, and skips the creature — even though it is now displaying its front face whose registry subtypes are `["Human", "Werewolf"]`. Every werewolf DFC in the implementation has `"Human"` only on the front face and `"Werewolf"` on both faces, so this stale-subtypes path silently drops the Human detection for all of them. The engine's canonical subtype check (`matches_filter` in `state.rs:654–672`) handles this correctly by branching on `is_transformed` to pick the right registry face and then also ORing in `obj.subtypes`; Moonmist's inline check diverges from that pattern.

### Tricky interactions checked

- **Subtype check at damage time (ruling: "checked only as combat damage is dealt")**: PASS — `is_non_wolf_damage_prevented` is called inside `deal_damage_to_creature` / `deal_damage_to_player` at the moment damage is assigned, not at Moonmist resolution time. The check reads the creature's current subtypes dynamically via `get_subtypes`.
- **Creature enters after Moonmist resolves (ruling: "even if that creature wasn't on the battlefield when Moonmist resolved")**: PASS — The prevention flag `prevent_non_wolf_werewolf_combat_damage` is a global boolean; any creature dealing combat damage after it is set is subject to the check, regardless of when it entered.
- **Creature changes type between Moonmist and damage (ruling: "even if … was a Werewolf or a Wolf when Moonmist resolved")**: PASS — Because the type check fires at damage time via `is_non_wolf_damage_prevented`, a creature that was a Werewolf at resolution but is no longer one at damage time will have its damage prevented, and vice versa.
- **Non-DFC Humans not transformed**: PASS — `has_back_face` guard (line 58–61) ensures only cards with a back face are transformed; non-DFC Humans are skipped.
- **Back-face Humans (e.g. Thraben Militia)**: PASS — When `obj.subtypes` is empty and `is_transformed` is true, the code falls through to check back-face registry data for "Human", correctly finding Thraben Militia.
- **Wolf tokens damage prevention**: PASS — Tokens carry `obj.subtypes = ["Wolf"]` and `registry.card_data(CardId(0))` returns None; `get_subtypes` returns `["Wolf"]`, so Wolf tokens are correctly exempt.
- **Blocker damage prevention (non-Wolf blocker vs. Wolf attacker)**: PASS — `deal_damage_to_creature` applies `is_non_wolf_damage_prevented` to the `source` of damage, so a non-Wolf blocker's damage to a Wolf attacker is prevented, while the Wolf's damage to the blocker goes through.
- **Prevention flag cleared at end of turn**: PASS — `engine.rs:3026` clears `prevent_non_wolf_werewolf_combat_damage` in `Step::Cleanup`.
- **`move_spell_after_resolve` used (graveyard placement)**: PASS — Called at line 110.
- **Mandatory transform (no "you may")**: PASS — Moonmist's oracle text has no "you may"; the code unconditionally transforms all matching Humans with no player choice.
- **`get_subtypes` for already-transformed DFC (via upkeep, `obj.subtypes` still empty)**: PASS — `get_subtypes` in `combat.rs:356–368` reads `obj.subtypes` first (empty) then falls back to `registry.card_data()` front-face subtypes (which include "Werewolf"), so a Werewolf transformed via `on_upkeep` without updated instance subtypes is still correctly identified as a Werewolf for damage prevention.
- **Second Moonmist after natural untransform back to front face**: FAIL — described in Code Issues above.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Prevention flag set on resolution: `mtg-engine/tests/moonmist.rs:20` (`sets_prevention_flag`)
- Non-Wolf/non-Werewolf creature damage to player prevented: `mtg-engine/tests/moonmist.rs:32` (`prevents_non_wolf_combat_damage_to_player`)
- Wolf creature damage not prevented: `mtg-engine/tests/moonmist.rs:50` (`wolf_still_deals_damage`)
- Non-Wolf creature damage to creature prevented: `mtg-engine/tests/moonmist.rs:68` (`prevents_non_wolf_combat_damage_to_creature`)
- Front-face Human DFC transforms: `mtg-engine/tests/moonmist.rs:90` (`transforms_front_face_human`)
- Back-face Human DFC (Thraben Militia) transforms: `mtg-engine/tests/moonmist.rs:109` (`transforms_back_face_human`)
- Non-DFC Human not transformed: `mtg-engine/tests/moonmist.rs:133` (`does_not_transform_non_dfc_human`)
- Card data (Instant, CMC 2): `mtg-engine/tests/innistrad_simple_cards.rs:530` (`moonmist_card_data`)
- Subtype checked at damage time, not at resolution (ruling 2011-09-22): NOT TESTED
- Creature entering after Moonmist is subject to prevention (ruling 2011-09-22): NOT TESTED
- Werewolf deals damage after Moonmist transforms it (Wolf/Werewolf exempt after transform): NOT TESTED
- Second Moonmist after natural werewolf untransform (the bug identified above): NOT TESTED
- Blocker (non-Wolf) damage to Wolf attacker prevented: NOT TESTED
- Prevention flag cleared at end of turn: NOT TESTED
