## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This creature can't be blocked except by creatures with flying or reach.
**Type line**: Creature — Spirit
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- `EffectScope::OnSelf` correctly scopes the block restriction to Orchard Spirit itself: in `combat.rs` the loop calls `effect_applies_to(attacker_id, OnSelf, source.id, ...)` which reduces to `attacker_id == source.id`, firing only when Orchard Spirit is the attacker: pass
- `CreatureFilter::Or([HasKeyword(Flying), HasKeyword(Reach)])` correctly allows either flying or reach: `matches_filter` for `Or` uses `any()`, so either keyword satisfies the filter: pass
- `has_keyword` checks all sources of keywords: object-level keywords (covers tokens with keywords on `obj.keywords`), card definition keywords, `GrantKeyword` continuous effects, and `until_end_of_turn_keywords` temporary grants — a blocker that gains reach or flying by any means will satisfy the filter: pass
- Keyword removal (`until_end_of_turn_removed_keywords`) is checked first in `has_keyword`, so a creature that temporarily loses flying or reach is correctly denied blocking: pass
- The engine's separate flying-attacker check (lines 619–624 of `combat.rs`) is for when the *attacker* has Flying — Orchard Spirit has no Flying keyword, so that path never fires for it; the `BlockRestriction` path is the sole enforcer, which is correct: pass
- The `BlockRestriction` restriction is a continuous, always-on effect (not snapshot at ETB), so it applies correctly for the entire duration Orchard Spirit is on the battlefield: pass

### Test coverage
- Ground creature cannot block Orchard Spirit: `mtg-engine/tests/tier5_cards.rs` — `orchard_spirit_not_blocked_by_ground` (line 146)
- Flying creature (Chapel Geist) can block Orchard Spirit: `mtg-engine/tests/tier5_cards.rs` — `orchard_spirit_blocked_by_flyer` (line 159)
- Reach creature (Somberwald Spider) can block Orchard Spirit: `mtg-engine/tests/tier5_cards.rs` — `orchard_spirit_blocked_by_reach` (line 172)
- Blocker gaining flying/reach via continuous effect (GrantKeyword) grants block permission: NOT TESTED
- Blocker gaining reach via temporary until-EOT grant can block: NOT TESTED
- Token with flying/reach keyword on obj.keywords can block: NOT TESTED
