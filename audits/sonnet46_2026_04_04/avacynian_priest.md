## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {1}, {T}: Tap target non-Human creature.
**Type line**: Creature — Human Cleric
**Status**: ISSUE

### Code issues

- Engine does not enforce summoning sickness for `{T}` activated abilities; a freshly entered Avacynian Priest can activate its tap ability on the same turn it enters the battlefield.
  - Oracle text says: `{1}, {T}: Tap target non-Human creature.` — the `{T}` symbol in the cost means the ability cannot be activated while the creature has summoning sickness (CR 302.6: "A creature's activated ability with the tap symbol ({T}) or the untap symbol ({Q}) in its activation cost can't be activated unless the creature has been under its controller's control continuously since their most recent turn began.")
  - Code does: `mtg-engine/src/engine.rs` line 356: `if ab.requires_tap && obj_tapped { continue; }` — only skips when already tapped; there is no corresponding check for `obj.summoning_sick`. A creature with `summoning_sick = true` is not skipped, so a turn-1 Avacynian Priest that just entered the battlefield is offered the `{1}, {T}:` action in `legal_actions`.

### Tricky interactions checked

- Non-Human check covers both registry card data and runtime object subtypes (tokens): checked `is_valid_target` at lines 57–60 of `avacynian_priest.rs` — checks `registry.card_data(o.card_id)` subtypes OR `o.subtypes` directly. Tokens store subtypes on the object (not in the registry), so both paths are needed and both are present. PASS.
- Priest cannot target itself (Priest is a Human): `is_valid_target` sets `is_human = true` for the Priest's own card_id because `registry.card_data` returns subtypes containing "Human". `!is_human` evaluates to `false`, so the Priest is excluded. PASS.
- Priest can target opponent's non-Human creatures (no controller restriction): `is_valid_target` only checks zone, power (is creature), and non-Human subtype — no `obj.controller` filter. PASS.
- Summoning sickness blocks `{T}` ability activation: engine check at line 356 is `if ab.requires_tap && obj_tapped { continue; }` — no `summoning_sick` check. FAIL (ISSUE above).
- `{T}` cost is paid before effect (correct ordering): `submit_action` sets `tapped = true` at line 1739–1741 before calling `on_activate_ability` at line 1802. PASS.
- `generate_ability_targets` calls `is_valid_target` for `TargetRequirement::Creature`: confirmed at `engine.rs` line 1302 — `.filter(|t| behavior.is_valid_target(state, controller, t, registry))` is applied. The non-Human filter in `is_valid_target` is therefore enforced when building legal actions. PASS.
- Target must be on the battlefield: `is_valid_target` checks `o.zone == Zone::Battlefield`. PASS.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Card stats (name, cost, P/T, subtypes): `mtg-engine/tests/activated_abilities.rs:272-281`
- Ability taps a non-Human creature: `mtg-engine/tests/activated_abilities.rs:283-309`
- Cannot target Human creatures: `mtg-engine/tests/activated_abilities.rs:311-330`
- Priest becomes tapped as part of cost / cannot re-activate while tapped: `mtg-engine/tests/activated_abilities.rs:332-354`
- Summoning sickness prevents `{T}` activation (turn entered): NOT TESTED
- Targeting tokens (non-Human token cannot be targeted? yes; Human token cannot? also yes): NOT TESTED
- Priest cannot target itself: NOT TESTED
