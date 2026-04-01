## Audit — 2026-04-01

**Scryfall Oracle text (front — Screeching Bat)**: Flying\nAt the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform Screeching Bat.
**Scryfall Oracle text (back — Stalking Vampire)**: At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform Stalking Vampire.
**Scryfall type line**: Creature — Bat // Creature — Vampire
**Mana cost**: {2}{B}
**P/T**: 2/2 // 5/5
**Status**: ISSUE

**Issue: Back face (Stalking Vampire) should NOT have flying, but front face does.**

The front face has flying, but Stalking Vampire (back face) does not have flying per Oracle text. The implementation correctly does not list flying in the back face keywords (keywords: vec![]), and the front face has Keyword::Flying. However, the `dynamic_pt` approach means the creature's keywords may not update on transform since there is no explicit keyword removal/addition on transform. Whether flying persists on the back face depends on the engine's transform handling -- if the engine swaps to back_face_data keywords, this is fine. If it only swaps P/T, the Vampire would incorrectly retain flying.

**Minor concern**: The auto-pay logic for the "you may" choice automatically pays if mana is available. This is a simplification (the player doesn't get to decline), but is noted in code comments.

- Tests: `screeching_bat_transforms_at_upkeep_with_mana` in tier15_cards.rs
