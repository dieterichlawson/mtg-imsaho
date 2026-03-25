# Exemplar Cards & Phases

## Phase 1: Vanilla Creatures (implemented)

Vanilla creatures, basic lands, combat, mana, the stack, state-based actions.

**Cards**: Forest, Mountain, Kalonian Tusker (3/3 GG), Goblin Piker (2/1 1R)

**Engine features**: Turn structure, priority, casting creature spells via the stack,
playing lands, mana abilities, combat (attack/block/damage), state-based actions,
summoning sickness, cleanup discard, immutable game state, Player trait.

## Phase 2: Starter Set

All main card types with simple real cards. Two playable 40-card decks.

### Lands (5 — 3 new)
| Card | Oracle Text |
|------|------------|
| Plains | {T}: Add {W} |
| Island | {T}: Add {U} |
| Swamp | {T}: Add {B} |
| Forest | {T}: Add {G} *(done)* |
| Mountain | {T}: Add {R} *(done)* |

### Creatures (5 — 3 new)
| Card | Cost | P/T |
|------|------|-----|
| Savannah Lions | {W} | 2/1 |
| Grizzly Bears | {1}{G} | 2/2 |
| Walking Corpse | {1}{B} | 2/2 |
| Goblin Piker | {1}{R} | 2/1 *(done)* |
| Kalonian Tusker | {G}{G} | 3/3 *(done)* |

### Instants (4 — all new)
| Card | Cost | Effect |
|------|------|--------|
| Lightning Bolt | {R} | Deal 3 to any target |
| Giant Growth | {G} | Target creature gets +3/+3 until end of turn |
| Doom Blade | {1}{B} | Destroy target nonblack creature |
| Swords to Plowshares | {W} | Exile target creature. Controller gains life equal to its power |

### Sorceries (2 — all new)
| Card | Cost | Effect |
|------|------|--------|
| Divination | {2}{U} | Draw two cards |
| Lava Axe | {4}{R} | Deal 5 to target player |

### Enchantments (3 — all new)
| Card | Cost | Effect |
|------|------|--------|
| Glorious Anthem | {1}{W}{W} | Creatures you control get +1/+1 |
| Holy Strength | {W} | Aura — enchanted creature gets +1/+2 |
| Pacifism | {1}{W} | Aura — enchanted creature can't attack or block |

### Artifacts (1 — new)
| Card | Cost | Effect |
|------|------|--------|
| Sol Ring | {1} | {T}: Add {C}{C} |

### Engine features needed
1. **Targeting** — Lightning Bolt, Doom Blade, Swords, Giant Growth, Lava Axe
2. **Instant-speed casting** — respond to spells/abilities with priority
3. **"Until end of turn" effects** — Giant Growth
4. **Destroy/exile effects** — Doom Blade, Swords to Plowshares
5. **Sorceries** — timing variant of instants (graveyard after resolution)
6. **Auras** — attachment system (Holy Strength, Pacifism)
7. **Continuous effects** — Glorious Anthem, Pacifism restrictions
8. **Artifact permanents** — Sol Ring (non-land mana source)

### Sample decks
**Red/Green**: 10 Mountain, 10 Forest, 4 Goblin Piker, 4 Grizzly Bears, 4 Kalonian Tusker, 4 Lightning Bolt, 4 Giant Growth

**White/Black**: 10 Plains, 10 Swamp, 4 Savannah Lions, 4 Walking Corpse, 4 Swords to Plowshares, 4 Doom Blade, 2 Holy Strength, 2 Pacifism

---

## Exemplar Cards (future tiers)

Cards that stress different engine subsystems, organized by complexity.

### Tier 1: Targeting + Instants
| Card | What it tests |
|------|--------------|
| **Lightning Bolt** | Targeting creatures/players, damage spells, instant speed |
| **Swords to Plowshares** | Exile (not destroy), opponent gains life, targeting restrictions |
| **Counterspell** | Targeting spells on the stack, spell negation |

### Tier 2: Triggered Abilities
| Card | What it tests |
|------|--------------|
| **Rhystic Study** | Triggered ability on opponent's cast, opponent makes a choice |
| **Panharmonicon** | Modifies other triggered abilities (doubles ETB triggers) |
| **Skullclamp** | Equipment (attach/detach), triggered ability on equipped creature dying |

### Tier 3: Activated Abilities + Costs
| Card | What it tests |
|------|--------------|
| **Sensei's Divining Top** | Non-mana activated abilities, library manipulation, choices during resolution |
| **Birthing Pod** | Sacrifice as a cost, searching the library, mana + non-mana costs combined |
| **Fetchlands** (Windswept Heath) | Sacrifice self, search library for a card matching criteria, life payment |

### Tier 4: Continuous Effects
| Card | What it tests |
|------|--------------|
| **Blood Moon** | Continuous effect changing land types (layer 4), removes abilities |
| **Humility** | Layer system — removes all abilities, sets all P/T to 1/1, timestamp ordering |
| **Doubling Season** | Replacement effect on counters and tokens |

### Tier 5: Complex Stack Interaction
| Card | What it tests |
|------|--------------|
| **Narset's Reversal** | Copy spells, retarget, bounce spell off stack |
| **Force of Will** | Alternative costs (exile a card + pay life instead of mana) |
| **Cyclonic Rift** | Overload (alternative cost that changes "target" to "each") |

### Tier 6: The Hard Stuff
| Card | What it tests |
|------|--------------|
| **Necropotence** | Delayed triggers, replacement effects on draw, activated abilities with life cost |
| **Teferi's Protection** | Phasing, continuous effects with duration, protection from everything |
| **Thassa's Oracle** | Win-the-game triggered ability, devotion (counting mana symbols) |
