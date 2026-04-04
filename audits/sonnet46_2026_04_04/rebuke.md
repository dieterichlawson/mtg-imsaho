## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Destroy target attacking creature.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Mana cost {2}{W}: matches `ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::White)])` — pass
- Instant type, no supertypes/subtypes: code sets `card_types: vec![CardType::Instant]`, `supertypes: vec![]`, `subtypes: vec![]` — pass
- No power/toughness (instant): `power: None, toughness: None` — pass
- Target restriction ("attacking"): `TargetRequirement::CreatureWithFilter(TargetFilter::Attacking)` is declared, and `is_valid_target` checks `state.combat.as_ref().map(|c| c.attackers.contains_key(id)).unwrap_or(false)`. Engine target-generation at `engine.rs:866` delegates to `is_valid_target`, so only creatures in `c.attackers` appear as valid cast targets — pass
- No controller restriction on target (oracle says "attacking creature" not "attacking creature you don't control"): `is_valid_target` performs no `controller` check, so the caster's own attacking creatures are also valid targets — pass
- "Destroy" (not sacrifice): `on_resolve` calls `helpers::resolve_destroy` → `destruction::try_destroy`, which correctly respects indestructible and regeneration shields — pass
- Spell cleanup after resolution: `resolve_destroy` calls `state.move_spell_after_resolve(spell_id)`, which sends it to graveyard (not exile, since `flashback_cost: None`) — pass
- Fizzle if target leaves battlefield before resolution: `stack.rs::is_target_legal` checks `obj.zone == Zone::Battlefield`; if the creature is moved to another zone it fizzles — pass
- Fizzle if target stops being "attacking" before resolution (engine `is_target_legal` does not re-check filter): `stack.rs::is_target_legal` for a `CreatureWithFilter` target only checks `obj.zone == Zone::Battlefield`, not whether the creature is still in `c.attackers`. In principle this could allow the spell to resolve against a creature that has been removed from combat without leaving the battlefield. In the current engine, `remove_from_combat` is only called (a) inside `try_destroy`→`regenerate` (which happens during Rebuke's own resolution, not before it) and (b) in a legacy SBA code path that runs before priority is given and therefore before Rebuke can be cast. No card in the current engine can remove a creature from combat during the priority window between Rebuke being placed on the stack and it resolving. The theoretical engine limitation has no practical impact on Rebuke's behavior — pass (no practical impact)
- Indestructible creature: `try_destroy` checks `has_keyword(id, Keyword::Indestructible, registry)` and returns `DestroyResult::Indestructible` without moving the creature — pass
- Regeneration shield: `try_destroy` checks `obj.regeneration_shields > 0` and calls `regenerate` (tap, clear damage, remove from combat, consume shield) instead of destroying — pass

### Test coverage
- Basic case: Rebuke destroys the attacking creature and cannot target a non-attacker: `tier2_spells.rs:184` (`rebuke_destroys_attacking_creature`) — TESTED
- Rebuke against indestructible creature (should not destroy): NOT TESTED
- Rebuke against a creature with a regeneration shield (should regenerate, not die): NOT TESTED
- Rebuke fizzle when target leaves battlefield between cast and resolution: NOT TESTED
