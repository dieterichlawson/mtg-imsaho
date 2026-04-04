## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
**Type line**: Creature — Spirit Knight
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Mana cost / color identity**: Cost is `{W}{W}`, colors derived at game setup as `[Color::White]`. `has_keyword` check + color-share comparison in `combat.rs:640` uses runtime `obj.colors` — correctly reflects White. PASS
- **Intimidate blocks non-sharing, non-artifact creatures**: `combat.rs:627-644` — attacker with `Keyword::Intimidate` forces blocker to pass artifact OR shared-color test; non-matching blockers return `false`. PASS
- **Artifact creatures may block freely**: `combat.rs:632-634` checks `registry.card_data(blocker.card_id).map(|d| d.card_types.contains(&CardType::Artifact)).unwrap_or(false)`. All artifact creature cards in the current implementation (One-Eyed Scarecrow, Manor Gargoyle, Creepy Doll, Galvanic Juggernaut, Geistcatcher's Rig) are in the registry and pass this check. PASS
- **Artifact token edge case**: The registry-only check (`unwrap_or(false)`) would fail for a pure non-copy artifact token with `CardId(0)` — its `card_types` field on the object would not be consulted, unlike the pattern in `engine.rs:280-283` which correctly ORs in `obj.card_types`. However, **no pure artifact creature tokens are created anywhere in the current card set**; all tokens are non-artifact. Token copies via `create_token_copy` retain the original's `card_id`, so they still hit the registry. This edge case cannot be triggered in the current implementation. NOT AN ISSUE (theoretical only)
- **Color-share check is bidirectional**: `attacker.colors.iter().any(|c| blocker.colors.contains(c))` — correctly checks whether any attacker color appears in the blocker's colors. PASS
- **has_keyword checks object keywords AND registry AND aura grants**: `state.rs:987-1043` — covers object-level keywords (tokens/direct population), registry card data, aura `GrantKeyword` continuous effects, conditional keyword grants, and temporary `until_end_of_turn_keywords`. Spectral Rider's `Keyword::Intimidate` is found via the registry path. PASS
- **Token copy of Spectral Rider retains Intimidate**: `create_token_copy` sets `obj.card_id = card_id` (Spectral Rider's CardId) on the copy, so `has_keyword` still finds `Keyword::Intimidate` via registry lookup. PASS
- **Gruesome Deformity granting Intimidate to others**: `combat.rs:627` uses `has_keyword` which includes the `GrantKeyword` continuous-effect path — a creature granted Intimidate by Gruesome Deformity would also be subject to the same blocking logic. PASS (not specific to this card but confirms the general mechanism)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Card has `Keyword::Intimidate` in data: `tests/innistrad_cards.rs:123` (`spectral_rider_has_intimidate`)
- Non-sharing-color creature cannot block: `tests/keywords.rs:202` (`intimidate_blocks_different_color`)
- Same-color creature can block: `tests/keywords.rs:202` (`intimidate_blocks_different_color`)
- Artifact creature can block regardless of color: `tests/keywords.rs:227` (`artifact_creature_blocks_intimidate`)
- Artifact token edge case (pure non-copy token with Artifact in obj.card_types): NOT TESTED
