## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: You may pay {0} rather than pay the mana cost for Zombie creature spells you cast.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues

- Rooftop Storm alternative cost not offered for Zombie creature spells cast from the graveyard
  - Oracle text says: `"You may pay {0} rather than pay the mana cost for Zombie creature spells you cast."`
  - Code does: The graveyard-casting loop in `mtg-engine/src/engine.rs` (lines 665–748) does not apply the Rooftop Storm alternative cost. At line 708: `if !mana::can_pay(&player_state.mana_pool, fb_cost) { continue; }` — the loop gates on the normal mana cost with no Rooftop Storm check and no alternative-cost action generation. Skaab Ruinator (a Zombie Horror that can be cast from the graveyard via `can_cast_from_graveyard()`) therefore cannot be cast for free even when Rooftop Storm is on the battlefield. The hand-casting loop (lines 490–662) correctly generates the free alternative, but the graveyard-casting loop has no equivalent logic.

### Tricky interactions checked

- **"You may" is optional (player can still pay normal cost)**: PASS — when the player can afford normal cost, both normal-cost and alternative-cost `CastSpell` actions are added to the action list (engine.rs:626–641), so the choice is preserved.
- **Mandatory additional costs still apply when paying {0}**: PASS — sacrifice cost (engine.rs:1541–1566) and `ExileCreaturesFromGraveyard` cost (engine.rs:1568–1601) are both paid regardless of whether `alternative_cost` is set on the action. Makeshift Mauler (Zombie with exile-from-graveyard additional cost) would still require the exile even when cast for free via Rooftop Storm.
- **Mana value of spell unchanged**: PASS — all other engine code that reads mana value (e.g., Mindshrieker at mindshrieker.rs:72, Heretic's Punishment at heretics_punishment.rs:88–89) uses `card_data().cost.mana_value()` from the registry, not the `alternative_cost` on the action. The alternative cost only affects what mana is paid; the printed cost is not overwritten anywhere.
- **Rooftop Storm applies only to the controller's Zombies**: PASS — both `rooftop_storm_applies()` (engine.rs:52–55) and the inline check (engine.rs:616–618) use `o.controller == player/caster`, not `o.owner`.
- **Multiple Rooftop Storms in play**: PASS — both checks use `.any(...)`, so a second copy has no additional effect.
- **Non-Zombie creatures are unaffected**: PASS — both `is_zombie_creature` checks gate on `subtypes.iter().any(|s| s == "Zombie")`.
- **Zombie creature spells cast from graveyard (Skaab Ruinator)**: FAIL — graveyard-casting loop does not offer the Rooftop Storm free alternative (see Code issues above).
- **Token Zombie subtypes**: PASS (not applicable) — Rooftop Storm affects spells being cast, and tokens cannot be cast. Only registered non-token cards appear in the hand-casting loop; registry-based subtype checks are correct for this context.
- **Card data correctness**: PASS — name "Rooftop Storm", cost {5}{U} (`Generic(5), Colored(Blue)`), type `Enchantment`, no subtypes, oracle text matches verbatim, keywords `[]`, no flashback/continuous_effects/additional_cost/triggered_abilities.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Rooftop Storm makes Zombie creatures free (basic case): `tier14_cards.rs:209` (`rooftop_storm_makes_zombies_free`)
- Rooftop Storm does not affect non-Zombie creatures: `tier14_cards.rs:227` (`rooftop_storm_no_free_non_zombies`)
- Mandatory additional costs still apply when casting for free (Makeshift Mauler + Rooftop Storm): NOT TESTED
- Mana value of spell is unchanged when using the free cost: NOT TESTED
- Player's choice preserved when they can afford normal cost (both options generated): NOT TESTED
- Rooftop Storm applies to Zombie cast from graveyard (Skaab Ruinator + Rooftop Storm): NOT TESTED
- Rooftop Storm applies only to controller's Zombie spells (not opponent's): NOT TESTED
