# Audit: Grave Bramble

## Oracle Reference (Scryfall)
- Cost: {1}{G}{G}
- Type: Creature -- Plant
- P/T: 3/4
- Oracle: "Defender, protection from Zombies"

## Implementation: grave_bramble.rs

## Issues Found

No issues found. Name, cost ({1}{G}{G}), type (Creature), subtype (Plant), P/T (3/4), defender keyword, and protection from Zombies (via ContinuousEffect::ProtectionFromSubtype) all match.

## Verdict: PASS

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
```
Defender, protection from Zombies
```

### Findings
- Name, cost ({1}{G}{G}), type (Creature -- Plant), P/T (3/4) all match.
- Defender keyword present in keywords vec -- correct.
- Protection from Zombies via ContinuousEffect::ProtectionFromSubtype with subtype "Zombie" and OnSelf scope -- correct.

### Verdict: PASS

---

## Audit — 2026-04-02 21:09
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Defender, protection from Zombies
**Type line**: Creature — Plant
**Status**: ISSUE

### Code issues
1. **LLM card knowledge missing protection from Zombies** (`mtg-player/src/llm.rs` line 64): The AI player description says `Grave Bramble ({1}{G}{G} 3/4 defender): Can't attack, but great blocker.` but does not mention "protection from Zombies." The AI player will not know about this key defensive ability and may make suboptimal blocking/combat decisions against Zombie creatures.
2. **Engine limitation -- protection does not prevent targeting**: `can_be_targeted` in `mtg-engine/src/engine.rs` only checks hexproof, not protection. A Zombie spell could target Grave Bramble despite its protection. This is an engine-wide limitation, not specific to this card.
3. **Engine limitation -- protection does not prevent enchanting/equipping**: No validation that a Zombie aura/equipment cannot enchant/equip Grave Bramble. Engine-wide limitation.

### Tricky interactions checked (min 3)
1. **Zombie combat damage to Grave Bramble**: Protection correctly prevents all combat damage from Zombie creatures (tested in `grave_bramble_protection_prevents_zombie_damage`).
2. **Grave Bramble dealing damage back**: Grave Bramble still deals its 3 combat damage to a blocking/blocked Zombie (protection is one-directional for damage). Verified in the same test.
3. **Zombie cannot block Grave Bramble**: `can_block` in combat.rs checks `has_protection_from_creature` in both directions, so a Zombie couldn't block Grave Bramble. Moot in practice since Grave Bramble has defender and can't attack, but the logic is correctly implemented.
4. **Defender prevents attacking**: `eligible_attackers` in combat.rs checks `has_keyword(Keyword::Defender)` and excludes such creatures. Tested in `defender_cannot_attack`.
5. **Defender allows blocking**: `eligible_blockers` does NOT exclude defenders. Tested in `defender_can_block`.

### Test coverage
- `keywords.rs::defender_cannot_attack` -- Grave Bramble cannot attack
- `keywords.rs::defender_can_block` -- Grave Bramble can still block
- `card_mechanics.rs::grave_bramble_protection_prevents_zombie_damage` -- Zombie deals 0 damage to Grave Bramble; Grave Bramble deals 3 to Zombie
- **Missing**: No test for Zombie-sourced spells being unable to target Grave Bramble (engine limitation)
- **Missing**: No test for non-Zombie damage going through normally
