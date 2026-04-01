## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant creature\nEnchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.
**Scryfall type line**: Enchantment — Aura
**Scryfall mana cost**: {G}
**Status**: PASS

Findings:
- Name: Correct.
- Mana cost: {G} — correct.
- Types: Enchantment — Aura — correct.
- Oracle text stored: "Enchanted creature gets +X/+X, where X is the number of creature cards in your graveyard." — missing "Enchant creature" keyword line, though the targeting is implemented.
- Targeting: `target_requirement` returns `Creature` — correct for "Enchant creature".
- Resolution: Uses `resolve_aura` helper — correct.
- Dynamic P/T: `dynamic_pt` counts creature cards (checked via `o.power.is_some()`) in the controller's graveyard owned by the controller. Returns (X, X). Correct.
- **ISSUE: `dynamic_pt` uses `o.owner == controller` but should arguably use the aura's controller.** It uses `state.get_object(object_id)?.controller` where `object_id` is the aura, so this is actually correct — it checks the aura controller's graveyard.
- **Minor ISSUE: Oracle text is missing "Enchant creature" line.** This is a display-only issue; the targeting is correctly implemented via `target_requirement`.
- Tests: `wreath_of_geists_dynamic_buff` and `wreath_of_geists_updates_dynamically` in tier5_cards.rs.

Note: The "Enchant creature" omission from oracle_text is cosmetic only; targeting behavior is correct.

## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant creature. Enchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.
**Scryfall type line**: Enchantment — Aura
**Mana cost**: {G}
**Status**: PASS

No issues found. Dynamic P/T via dynamic_pt, Aura resolution via resolve_aura, creature count in graveyard all correct.
