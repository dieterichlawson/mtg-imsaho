# Audit: Elite Inquisitor

## Reference (Scryfall)
- **Name:** Elite Inquisitor
- **Cost:** {W}{W}
- **Type:** Creature -- Human Soldier
- **Oracle:** First strike, vigilance. Protection from Vampires, from Werewolves, and from Zombies.
- **P/T:** 2/2

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({W}{W})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Human, Soldier)
- Oracle text: CORRECT
- P/T: CORRECT (2/2)
- Keywords: CORRECT (FirstStrike, Vigilance)
- Protection from Vampires: CORRECT (ProtectionFromSubtype "Vampire")
- Protection from Werewolves: CORRECT (ProtectionFromSubtype "Werewolf")
- Protection from Zombies: CORRECT (ProtectionFromSubtype "Zombie")

## Issues
None found.

---

## Audit 2 (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
- **Name:** Elite Inquisitor
- **Mana Cost:** {W}{W}
- **Type Line:** Creature — Human Soldier
- **P/T:** 2/2
- **Oracle Text:**
  First strike, vigilance
  Protection from Vampires, from Werewolves, and from Zombies
- **Keywords:** Vigilance, First strike, Protection

### Implementation: `mtg-engine/src/cards/isd/elite_inquisitor.rs`

#### Card Data
| Field       | Oracle                  | Implementation          | Match |
|-------------|-------------------------|-------------------------|-------|
| Name        | Elite Inquisitor        | "Elite Inquisitor"      | YES   |
| Mana Cost   | {W}{W}                  | [White, White]          | YES   |
| Types       | Creature                | Creature                | YES   |
| Supertypes  | (none)                  | (none)                  | YES   |
| Subtypes    | Human Soldier           | Human, Soldier          | YES   |
| Power       | 2                       | 2                       | YES   |
| Toughness   | 2                       | 2                       | YES   |

#### Keywords
| Keyword      | Oracle | Implementation | Match |
|--------------|--------|----------------|-------|
| First strike | YES    | YES            | YES   |
| Vigilance    | YES    | YES            | YES   |

#### Protection
| Protection From | Oracle | Implementation (ProtectionFromSubtype) | Match |
|-----------------|--------|----------------------------------------|-------|
| Vampires        | YES    | "Vampire" (OnSelf)                     | YES   |
| Werewolves      | YES    | "Werewolf" (OnSelf)                    | YES   |
| Zombies         | YES    | "Zombie" (OnSelf)                      | YES   |

### Tests (`mtg-engine/tests/tier12_cards.rs`)
- `elite_inquisitor_keywords` -- verifies first strike and vigilance. **PASS**
- `elite_inquisitor_protection_prevents_damage` -- verifies combat damage from a Vampire is prevented. **PASS**
- `elite_inquisitor_cant_be_blocked_by_zombies` -- verifies a Zombie cannot block Elite Inquisitor. **PASS**

### Verdict
**PASS** -- No mismatches found. All card data, keywords, and protection abilities match the oracle text. All three tests pass.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: First strike, vigilance
Protection from Vampires, from Werewolves, and from Zombies
**Type line**: Creature — Human Soldier
**Status**: PASS

### Code issues
No issues found.

---

## Audit — 2026-04-02 20:54

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: First strike, vigilance
Protection from Vampires, from Werewolves, and from Zombies
**Type line**: Creature — Human Soldier
**Status**: PASS

### Code issues
None. All card data fields (name, cost {W}{W}, types, subtypes Human/Soldier, P/T 2/2, oracle text) match the Scryfall oracle exactly. Keywords (FirstStrike, Vigilance) are correctly declared. Protection is implemented via three `ContinuousEffect::ProtectionFromSubtype` entries for "Vampire", "Werewolf", and "Zombie" with `EffectScope::OnSelf`, which is correct. No anti-patterns (no hardcoded IDs, no unsafe, no unwrap).

### Tricky interactions checked (min 3)
1. **Protection from Werewolves vs. transformed werewolves**: The subtype "Werewolf" is used, which matches both faces of DFC werewolves when transformed. Confirmed `has_protection_from_creature` checks `get_subtypes` which includes both object-level and card-data subtypes.
2. **First strike + protection interaction**: Elite Inquisitor deals first strike damage before normal combat damage. Even if a Vampire/Zombie/Werewolf survives first strike, its normal combat damage is independently prevented by protection. Both are handled by separate engine subsystems (keyword-based damage ordering + `has_protection_from_creature` in damage application).
3. **Vigilance + protection for blocking**: Elite Inquisitor doesn't tap to attack, so it can block on the opponent's turn. Combined with protection, opposing Vampires/Werewolves/Zombies cannot be assigned as blockers against it, verified in `can_block_attacker`.

### Test coverage
- `elite_inquisitor_keywords` — verifies FirstStrike and Vigilance keywords. **PASS**
- `elite_inquisitor_protection_prevents_damage` — Vampire deals combat damage to Elite Inquisitor; damage is 0. **PASS**
- `elite_inquisitor_cant_be_blocked_by_zombies` — Zombie fails `can_block_attacker` check. **PASS**
- Also used as a supporting creature in `hamlet_captain_buffs_humans_on_block` test. **PASS**
