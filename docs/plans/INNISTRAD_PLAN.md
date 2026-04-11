# Innistrad Implementation Plan

## Status

**Implemented (Tiers 0-4): 79 Innistrad cards + 16 non-Innistrad reference cards = 95 total**

Tiers 0-4 covered: vanilla creatures, keyword creatures, auras, combat tricks, targeted removal, bounce, fight, counters, tokens, ETB/dies/death-watch triggers, flashback, activated abilities, regeneration.

**Remaining: ~170 Innistrad cards across Tiers 5-12**

---

## Tier 5: Easy Wins (15 cards)

Cards that work with existing engine systems. No new engine features needed.

### Engine work: None (or very minor)

Variable P/T (Geist-Honored Monk, Wreath of Geists) needs `effective_power`/`effective_toughness` to check graveyard count or creature count. This may require a small extension to the existing P/T computation.

### Cards

| Card | Cost | Type | Effect |
|------|------|------|--------|
| Festerhide Boar | {3}{G} | 3/3 trample | Morbid: enters with two +1/+1 counters |
| Hollowhenge Scavenger | {3}{G}{G} | 4/5 | Morbid ETB: gain 5 life |
| Morkrut Banshee | {3}{B}{B} | 4/4 | Morbid ETB: -4/-4 to target creature |
| Crossway Vampire | {1}{R}{R} | 3/2 | ETB: target creature can't block this turn |
| Armored Skaab | {2}{U} | 1/4 | ETB: mill 4 |
| Geistcatcher's Rig | {6} | 4/5 artifact creature | ETB: may deal 4 to creature with flying |
| Ancient Grudge | {1}{R} | Instant | Destroy target artifact. Flashback {G} |
| Battleground Geist | {4}{U} | 3/3 flying | Other Spirits you control get +1/+0 |
| Gallows Warden | {4}{W} | 3/3 flying | Other Spirits you control get +0/+1 |
| Orchard Spirit | {2}{G} | 2/2 | Can't be blocked except by flying/reach |
| Selhoff Occultist | {2}{U} | 2/3 | When this or another creature dies, target player mills 1 |
| Murder of Crows | {3}{U}{U} | 4/4 flying | When another creature dies, may draw then discard |
| Spider Spawning | {4}{G} | Sorcery | Create 1/2 Spider with reach per creature card in graveyard. Flashback {6}{B} |
| Wreath of Geists | {G} | Aura | +X/+X where X = creature cards in your graveyard |
| Geist-Honored Monk | {3}{W}{W} | \*/\* vigilance | P/T = creatures you control. ETB: two 1/1 Spirit tokens |

### UI/AI work
- Add each card to LLM card knowledge section
- Spirit lords need oracle text in the prompt so AI understands the buff

### Testing
- Deterministic: 1-2 tests per card (ETB effect, morbid condition)
- AI scenarios: ~8 tests (one per card that requires a decision)

---

## Tier 6: Combat Damage Triggers (10 cards)

### Engine work: Small

- Add `on_combat_damage_to_player` hook to `CardBehavior` trait
- Process `CombatDamageDealt` events in `triggers.rs` (event already exists, just not dispatched)
- For Champion of the Parish: add `on_any_creature_enters` hook (ETB-watcher, similar to existing death-watcher `on_any_creature_dies`)

### Cards

| Card | Cost | Type | Effect |
|------|------|------|--------|
| Champion of the Parish | {W} | 1/1 Human | Whenever another Human enters, +1/+1 counter |
| Stromkirk Noble | {R} | 1/1 Vampire | Can't be blocked by Humans. Combat damage to player -> +1/+1 counter |
| Stromkirk Patrol | {4}{B} | 4/3 Vampire | Combat damage to player -> +1/+1 counter |
| Bloodcrazed Neonate | {1}{R} | 2/1 Vampire | Must attack. Combat damage to player -> +1/+1 counter |
| Falkenrath Marauders | {3}{R}{R} | 2/2 flying haste Vampire | Combat damage to player -> two +1/+1 counters |
| Rakish Heir | {2}{R} | 2/2 Vampire | Whenever ANY Vampire you control deals combat damage to player -> +1/+1 counter on it |
| Sturmgeist | {3}{U}{U} | \*/\* flying Spirit | P/T = cards in hand. Combat damage to player -> draw |
| Curiosity | {U} | Aura | Enchanted creature deals damage to opponent -> draw |
| Balefire Dragon | {5}{R}{R} | 6/6 flying Dragon | Combat damage to player -> deal that much to all their creatures |
| Abattoir Ghoul | {3}{B} | 3/2 first strike Zombie | When creature dealt damage by this dies, gain life = its toughness |

### UI/AI work
- Card knowledge for all 10 cards
- AI needs to understand "must attack" creatures (already have forced-attack display)
- Rakish Heir's group trigger needs clear explanation in prompt
- Variable P/T display for Sturmgeist

### Testing
- Engine tests: combat damage trigger fires, counter placed, multiple triggers in one combat
- Deterministic card tests: each card's trigger with setup combat
- AI scenarios: ~5 (attack with vampire to grow it, Champion grows from casting Humans, etc.)

---

## Tier 7: Upkeep/End-Step Triggers + Curses (12 cards)

### Engine work: Small-medium

- Process `StepStarted { step: Upkeep }` and `StepStarted { step: EndStep }` in `triggers.rs`
- Add `on_upkeep` and `on_end_step` hooks to `CardBehavior`
- For Curses: add enchant-player targeting (`TargetRequirement::Player` or similar — `Target::Player` already exists but aura attachment to players needs support)
- SBA: auras attached to players shouldn't be killed by the "unattached aura" check

### Cards

| Card | Cost | Type | Effect |
|------|------|------|--------|
| Curse of the Pierced Heart | {1}{R} | Aura Curse | Enchant player. Upkeep: deal 1 damage |
| Curse of the Bloody Tome | {2}{U} | Aura Curse | Enchant player. Upkeep: mill 2 |
| Curse of Oblivion | {3}{B} | Aura Curse | Enchant player. Upkeep: exile 2 from graveyard |
| Curse of the Nightly Hunt | {2}{R} | Aura Curse | Enchant player. Their creatures must attack |
| Curse of Death's Hold | {3}{B}{B} | Aura Curse | Enchant player. Their creatures get -1/-1 |
| Bloodgift Demon | {3}{B}{B} | 5/4 flying Demon | Upkeep: target player draws and loses 1 life |
| Boneyard Wurm | {1}{G} | \*/\* Wurm | P/T = creature cards in your graveyard |
| Splinterfright | {2}{G} | \*/\* trample Elemental | P/T = creatures in graveyard. Upkeep: mill 2 |
| Angel of Flight Alabaster | {4}{W} | 4/4 flying Angel | Upkeep: return Spirit from graveyard to hand |
| Endless Ranks of the Dead | {2}{B}{B} | Enchantment | Upkeep: create X 2/2 Zombies (X = half your Zombies) |
| Reaper from the Abyss | {3}{B}{B}{B} | 6/6 flying Demon | Morbid end step: destroy target non-Demon |
| Charmbreaker Devils | {5}{R} | 4/4 Devil | Upkeep: return random instant/sorcery from graveyard. Spell cast -> +4/+0 |

### UI/AI work
- Card knowledge for all cards, especially explaining curse effects
- Curse targeting: LLM prompt needs to explain "Cast Curse -> Opponent"
- CLI: show curses attached to players in the game view (currently only shows permanents on battlefield)

### Testing
- Engine tests: upkeep trigger fires at correct step, curse deals damage each upkeep
- Deterministic card tests: each curse effect, Splinterfright self-mill + P/T
- AI scenarios: ~5 (cast curse on opponent, Bloodgift Demon draw, etc.)

---

## Tier 8: Sacrifice-as-Cost (12 cards)

### Engine work: Medium

- Extend `ActivatedAbilityDef` with `requires_sacrifice: SacrificeCost` enum (None, SacrificeThis, SacrificeCreature)
- In `submit_action` ActivateAbility handler: perform sacrifice before resolving ability
- For additional casting costs (Altar's Reap): extend the casting flow to require choosing a creature to sacrifice
- Use existing `destruction::sacrifice()` function
- Edict effects (Tribute to Hunger): opponent chooses creature to sacrifice — new `ResolutionChoiceKind` variant

### Cards

| Card | Cost | Type | Effect |
|------|------|------|--------|
| Selfless Cathar | {W} | 1/1 Human | {1}{W}, Sacrifice this: your creatures get +1/+1 until EOT |
| Silverchase Fox | {1}{W} | 2/2 Fox | {1}{W}, Sacrifice this: exile target enchantment |
| Brain Weevil | {3}{B} | 1/1 intimidate Insect | Sacrifice this: target player discards 2. Sorcery speed |
| Disciple of Griselbrand | {1}{B} | 1/1 Human Cleric | {1}, Sacrifice a creature: gain life = its toughness |
| Skirsdag Cultist | {2}{R}{R} | 2/2 Human Shaman | {R},{T}, Sacrifice a creature: deal 2 to any target |
| Altar's Reap | {1}{B} | Instant | As additional cost, sacrifice a creature. Draw 2 |
| Infernal Plunge | {R} | Sorcery | As additional cost, sacrifice a creature. Add {R}{R}{R} |
| Tribute to Hunger | {2}{B} | Instant | Opponent sacrifices a creature. You gain life = toughness |
| Stitcher's Apprentice | {1}{U} | 1/2 Homunculus | {1}{U},{T}: Create 2/2 Homunculus, then sacrifice a creature |
| Corpse Lunge | {2}{B} | Instant | Exile creature from graveyard as cost. Deal damage = its power |
| Harvest Pyre | {1}{R} | Instant | Exile X cards from graveyard. Deal X damage to creature |
| Divine Reckoning | {2}{W}{W} | Sorcery | Each player chooses a creature, destroy the rest. Flashback {5}{W}{W} |

### UI/AI work
- LLM prompt must explain sacrifice costs clearly: "you must sacrifice a creature to activate this"
- Multi-step casting flow for Altar's Reap: cast spell -> choose creature to sacrifice
- Tribute to Hunger: opponent gets a ResolutionChoice to pick which creature dies
- Card knowledge for all 12 cards

### Testing
- Engine tests: sacrifice-as-cost removes creature, ability resolves after sacrifice, sacrifice fails if no valid creature
- Deterministic: each card with sacrifice setup
- AI scenarios: ~5 (sacrifice small creature for value, Altar's Reap for card draw, etc.)
- Important: test that sacrifice bypasses indestructible and regeneration (already verified in destruction pipeline tests)

---

## Tier 9: Equipment (12 cards)

### Engine work: Medium-large

- Add `EquipmentData` to `CardData` (equip cost, conditional bonuses)
- New action: equip (either as `ActivateAbility` with special handling or a dedicated `Action::Equip`)
- Equipment enters battlefield unattached (unlike auras)
- Fix SBA: unattached equipment should NOT go to graveyard (currently all permanents with `attached_to` logic assume auras)
- Equipment stays on battlefield when equipped creature dies (detach, don't destroy)
- Equipment grants keywords/P/T bonuses similar to auras (via `granted_keywords` and oracle text parsing)
- Equip is sorcery-speed only

### Cards

| Card | Cost | Type | Effect |
|------|------|------|--------|
| Cobbled Wings | {2} | Equipment | Flying. Equip {1} |
| Mask of Avacyn | {2} | Equipment | +1/+2, hexproof. Equip {3} |
| Silver-Inlaid Dagger | {1} | Equipment | +2/+0 (+3/+0 if Human). Equip {2} |
| Sharpened Pitchfork | {2} | Equipment | First strike (+1/+1 if Human). Equip {1} |
| Butcher's Cleaver | {3} | Equipment | +3/+0 (lifelink if Human). Equip {3} |
| Wooden Stake | {2} | Equipment | +1/+0. Destroys Vampires when blocking/blocked. Equip {1} |
| Runechanter's Pike | {2} | Equipment | First strike, +X/+0 (X = instants+sorceries in graveyard). Equip {2} |
| Trepanation Blade | {3} | Equipment | Attack trigger: mill until land, get +X/+0. Equip {2} |
| Blazing Torch | {1} | Equipment | Can't be blocked by Vampires/Zombies. Sacrifice: 2 damage. Equip {1} |
| Demonmail Hauberk | {4} | Equipment | +4/+2. Equip cost: sacrifice a creature |
| Inquisitor's Flail | {2} | Equipment | Double combat damage dealt and received. Equip {2} |
| Traveler's Amulet | {1} | Artifact | {1}, Sacrifice: search library for basic land |

### UI/AI work
- CLI: distinguish equipment from auras in battlefield display (e.g., `Creature 3/3 [E:Sword, A:Pacifism]`)
- LLM: format equip actions ("Equip Cobbled Wings onto Bear"), explain equip is sorcery-speed
- Card knowledge for all equipment with equip costs
- Right-hand card reference panel: show equip cost

### Testing
- Engine tests: equip attaches, equip detaches from old creature, equipment survives creature death, equipment bonuses apply, equip is sorcery-speed only
- Deterministic card tests per equipment
- AI scenarios: ~5 (equip flying onto attacker, equip hexproof for protection, etc.)

---

## Tier 10: Creature Activated Abilities (12 cards)

### Engine work: Medium

- Extend `ActivatedAbilityDef` with targeting (current abilities are untargeted)
- Add once-per-turn limit option
- Keyword-granting until EOT via activated ability
- Land activated abilities (same system, just on lands)

### Cards

| Card | Cost | Type | Effect |
|------|------|------|--------|
| Avacynian Priest | {1}{W} | 1/2 Human Cleric | {1},{T}: Tap target non-Human creature |
| Manor Skeleton | {1}{B} | 1/1 haste Skeleton | {1}{B}: Regenerate |
| Kessig Wolf | {2}{R} | 3/1 Wolf | {1}{R}: Gains first strike until EOT |
| Feral Ridgewolf | {2}{R} | 1/2 trample Wolf | {1}{R}: +2/+0 until EOT |
| Darkthicket Wolf | {1}{G} | 2/2 Wolf | {2}{G}: +2/+2 until EOT (once per turn) |
| Lantern Spirit | {2}{U} | 2/1 flying Spirit | {U}: Return to owner's hand |
| Elder of Laurels | {2}{G} | 2/3 Human Advisor | {3}{G}: Target creature gets +X/+X (X = creatures you control) |
| Mindshrieker | {1}{U} | 1/1 flying Spirit | {2}: Target player mills 1, this gets +X/+X = mana value |
| Skirsdag High Priest | {1}{B} | 1/2 Human Cleric | Morbid {T}, tap 2 creatures: create 5/5 flying Demon |
| Gavony Township | Land | | {2}{G}{W},{T}: +1/+1 counter on each creature you control |
| Nephalia Drownyard | Land | | {1}{U}{B},{T}: Target player mills 3 |
| Stensia Bloodhall | Land | | {3}{B}{R},{T}: Deal 2 to target player |

### UI/AI work
- LLM: targeted abilities need a two-step prompt (activate -> choose target)
- Card knowledge with clear activation costs
- Land abilities: AI must understand these don't require casting

### Testing
- Engine tests: targeted ability resolves, once-per-turn enforced, land abilities work
- Deterministic + AI scenarios: ~8

---

## Tier 11: Graveyard Interaction (12 cards)

### Engine work: Medium

- Exile from graveyard as additional casting cost (new field on CardData or CardBehavior hook)
- Return card from graveyard to hand (simple zone move, but needs targeting graveyard cards)
- Hand reveal + exile (Night Terrors)
- Put card on top of library (Grasp of Phantoms — new zone destination)

### Cards

| Card | Cost | Type | Effect |
|------|------|------|--------|
| Makeshift Mauler | {3}{U} | 4/5 Zombie | Additional cost: exile creature from graveyard |
| Stitched Drake | {1}{U}{U} | 3/4 flying Zombie | Additional cost: exile creature from graveyard |
| Skaab Goliath | {5}{U} | 6/9 trample Zombie | Additional cost: exile 2 creatures from graveyard |
| Ghoulcaller's Chant | {B} | Sorcery | Return creature from graveyard to hand OR 2 Zombies |
| Ghoulraiser | {1}{B}{B} | 2/2 Zombie | ETB: return random Zombie from graveyard to hand |
| Caravan Vigil | {G} | Sorcery | Search for basic land. Morbid: onto battlefield |
| Mulch | {1}{G} | Sorcery | Reveal top 4: lands to hand, rest to graveyard |
| Purify the Grave | {W} | Instant | Exile target card from graveyard. Flashback {W} |
| Grasp of Phantoms | {3}{U} | Sorcery | Put creature on top of library. Flashback {7}{U} |
| Night Terrors | {2}{B} | Sorcery | Reveal hand, exile a nonland card |
| Memory's Journey | {1}{U} | Instant | Shuffle up to 3 graveyard cards into library. Flashback {G} |
| Woodland Sleuth | {3}{G} | 2/3 Human Scout | Morbid ETB: return random creature from graveyard to hand |

### UI/AI work
- Graveyard-as-cost: LLM needs to understand "you must have creatures in graveyard to cast this"
- Hand reveal: need new UI for showing opponent's hand and choosing a card
- Card knowledge for all cards

### Testing
- Engine: exile-from-graveyard cost deducted, can't cast without enough graveyard creatures
- Deterministic + AI: ~8

---

## Tier 12: Miscellaneous Medium Complexity (15 cards)

Various one-off mechanics. Each card may need a small engine addition.

### Cards

| Card | Cost | Type | Needs |
|------|------|------|-------|
| Ashmouth Hound | {1}{R} | 2/1 | Block/blocked-by trigger (1 damage to creature) |
| Hamlet Captain | {1}{G} | 2/2 Human | Attack/block trigger: other Humans get +1/+1 until EOT |
| Night Revelers | {4}{R} | 4/4 Vampire | Conditional haste (if opponent has Human) |
| Elite Inquisitor | {W}{W} | 2/2 first strike vigilance | Protection from Vampires/Werewolves/Zombies |
| Angelic Overseer | {3}{W}{W} | 5/3 flying Angel | Hexproof + indestructible if you control a Human |
| Traitorous Blood | {1}{R}{R} | Sorcery | Gain control of creature until EOT, untap, haste + trample |
| Blasphemous Act | {8}{R} | Sorcery | Cost {1} less per creature. 13 damage to each creature |
| Spare from Evil | {1}{W} | Instant | Your creatures gain protection from non-Humans until EOT |
| Burning Vengeance | {2}{R} | Enchantment | When you cast from graveyard, deal 2 to any target |
| Army of the Damned | {5}{B}{B}{B} | Sorcery | Create 13 tapped 2/2 Zombies. Flashback {7}{B}{B}{B} |
| Cackling Counterpart | {1}{U}{U} | Instant | Create token copy of creature you control. Flashback {5}{U}{U} |
| Sever the Bloodline | {3}{B} | Sorcery | Exile target creature and all with same name. Flashback {5}{B}{B} |
| Scourge of Geier Reach | {3}{R}{R} | 3/3 Elemental | +1/+1 per creature opponents control |
| Festerhide Boar already in T5 | | | |
| Moonmist | {1}{G} | Instant | Transform all Humans. Prevent non-Wolf/Werewolf combat damage |

### Engine work per card varies
- Protection from types: generalize existing protection system
- Threaten (temporary control change): new mechanic
- Variable cost reduction: new casting cost logic
- Token copy: new token creation mode
- These are individually small but there's no single feature that unlocks many cards

---

## Tier 13: Transform / Double-Faced Cards (20 cards)

### Engine work: Large

- Dual-face card representation (two CardData per card, front/back)
- Transform mechanic (flip between faces, changing P/T, keywords, abilities)
- Werewolf day/night tracking (transform condition: no spells cast last turn / 2+ spells cast)
- 12 standard werewolves share the same transform condition
- 8 non-werewolf DFCs each have unique transform conditions

### Cards
12 werewolves (Reckless Waif, Gatstaf Shepherd, Village Ironsmith, Mayor of Avabruck, Daybreak Ranger, Villagers of Estwald, Hanweir Watchkeep, Instigator Gang, Tormented Pariah, Grizzled Outcasts, Ulvenwald Mystics, Kruin Outlaw) + 8 others (Delver of Secrets, Cloistered Youth, Civilized Scholar, Screeching Bat, Ludevic's Test Subject, Thraben Sentry, Bloodline Keeper, Garruk Relentless)

### UI/AI work
- CLI: show both faces, indicate which is active, show transform condition
- LLM: explain transform mechanic, when to cast werewolves (when opponent might not cast spells)
- Game view: expose current face information

### Testing
- Engine: transform triggers, day/night tracking, P/T changes on transform
- Deterministic: werewolf transforms both directions, non-werewolf conditions
- AI: ~5 (cast werewolf at right time, attack with transformed creature, etc.)

---

## Tier 14: Advanced Engine Systems (~25 cards)

These cards each require significant engine subsystems. Grouped by the system they need.

### 14a: X-Cost Spells (3 cards)

Engine work: Track X value during casting (player chooses X based on available mana), store on stack object, use during resolution.

| Card | Cost | Type | Effect |
|------|------|------|--------|
| Devil's Play | {X}{R} | Sorcery | Deal X damage to any target. Flashback {X}{R}{R}{R} |
| Mikaeus, the Lunarch | {X}{W} | Legendary 0/0 Human Cleric | Enters with X +1/+1 counters. {T}: add counter. {T}, remove counter: counter on each other creature |
| Kessig Wolf Run | Land | | {T}: Add {C}. {X}{R}{G},{T}: +X/+0 and trample |

UI/AI: LLM needs to understand choosing X, paying {X}{R} means "tap X+1 lands." Two-step flow: choose X, then pay.

### 14b: Planeswalkers (2 cards)

Engine work (large): Loyalty counters as starting loyalty. Loyalty abilities (not activated abilities — use +N/-N loyalty as cost, one per turn, sorcery speed). Planeswalkers can be attacked (defender chooses to redirect attackers). 0-loyalty SBA (goes to graveyard). Damage to planeswalkers redirected from player. Legend rule SBA (two legendaries with same name = choose one to keep).

| Card | Cost | Type | Effect |
|------|------|------|--------|
| Liliana of the Veil | {1}{B}{B} | Legendary Planeswalker (3 loyalty) | +1: each player discards. -2: opponent sacrifices creature. -6: split permanents, opponent sacrifices a pile |
| Garruk Relentless | {3}{G} | Legendary Planeswalker (3 loyalty) | 0: fight creature (transforms at 2 loyalty). Back: +1: 1/1 Wolf deathtouch. -1: sacrifice creature to tutor. -3: +X/+X trample |

UI/AI: Display loyalty counter, show loyalty abilities as actions, explain one-ability-per-turn and sorcery-speed restriction. CLI needs planeswalker display section. Attack redirection needs UI for choosing planeswalker vs player.

### 14c: Replacement Effects (4 cards)

Engine work (large): Framework for "if X would happen, instead Y" — intercepts events before they resolve. Each replacement effect registers a check. Multiple replacements on the same event need ordering (affected player chooses).

| Card | Cost | Type | Effect |
|------|------|------|--------|
| Laboratory Maniac | {2}{U} | 2/2 Human Wizard | If you would draw from empty library, you win instead |
| Parallel Lives | {3}{G} | Enchantment | If you would create tokens, create twice that many instead |
| Essence of the Wild | {3}{G}{G}{G} | 6/6 Avatar | Creatures you control enter as a copy of this |
| Unbreathing Horde | {2}{B} | 0/0 Zombie | Enters with counters = Zombies + Zombie cards in graveyard. If would take damage, remove counter instead |

### 14d: Dynamic Flashback Granting (2 cards)

Engine work (small-medium): Allow a spell/ability to temporarily give flashback to cards in graveyard. Needs per-object until-end-of-turn flashback cost tracking.

| Card | Cost | Type | Effect |
|------|------|------|--------|
| Snapcaster Mage | {1}{U} | 2/1 flash Human Wizard | ETB: target instant/sorcery in graveyard gains flashback (cost = mana cost) until EOT |
| Past in Flames | {3}{R} | Sorcery | All instants/sorceries in your graveyard gain flashback until EOT. Flashback {4}{R} |

### 14e: Cost Reduction (3 cards)

Engine work (medium): Continuous effects that modify casting costs. Needs a cost-modification layer in legal_actions that adjusts ManaCost before checking can_pay.

| Card | Cost | Type | Effect |
|------|------|------|--------|
| Blasphemous Act | {8}{R} | Sorcery | Costs {1} less per creature on battlefield. 13 damage to each creature |
| Heartless Summoning | {1}{B} | Enchantment | Creature spells cost {2} less. Your creatures get -1/-1 |
| Rooftop Storm | {5}{U} | Enchantment | You may pay {0} for Zombie creature spells |

Note: Blasphemous Act is also in Tier 12 for its mass-damage effect. Listed here for the cost reduction engine work.

### 14f: Gain Control (3 cards)

Engine work (medium): Change controller of a permanent. Temporary steal needs end-of-turn revert. Permanent steal needs tracking original controller.

| Card | Cost | Type | Effect |
|------|------|------|--------|
| Traitorous Blood | {1}{R}{R} | Sorcery | Gain control until EOT, untap, haste + trample |
| Olivia Voldaren | {2}{B}{R} | 3/3 flying legendary Vampire | {1}{R}: 1 damage to creature + it becomes Vampire + counter on Olivia. {3}{B}{B}: gain control of target Vampire |
| Grimgrin, Corpse-Born | {3}{U}{B} | 5/5 legendary Zombie | Enters tapped, doesn't untap. Sacrifice creature: untap + counter. Attack: destroy defender's creature + counter |

Note: Traitorous Blood is also in Tier 12. Listed here for control-change engine work.

### 14g: Copy Effects (2 cards)

Engine work (medium): Create a token that copies all characteristics of an existing permanent. Needs deep-clone of CardData/keywords/P/T/abilities.

| Card | Cost | Type | Effect |
|------|------|------|--------|
| Cackling Counterpart | {1}{U}{U} | Instant | Create token copy of creature you control. Flashback {5}{U}{U} |
| Evil Twin | {2}{U}{B} | 0/0 Shapeshifter | Enter as copy of any creature + {U}{B},{T}: destroy creature with same name |

Note: Cackling Counterpart is also in Tier 12. Listed here for the copy engine work.

### 14h: Miscellaneous Unique Effects (6 cards)

| Card | Cost | Type | Needs |
|------|------|------|-------|
| Nevermore | {1}{W}{W} | Enchantment | Name a card, prevent casting. Needs: card naming system |
| Stony Silence | {1}{W} | Enchantment | Artifact activated abilities can't be activated. Needs: ability restriction system |
| Manor Gargoyle | {5} | 4/4 artifact creature | Defender + indestructible while defender. {1}: lose defender, gain flying until EOT. Needs: conditional indestructible tied to keyword |
| Mirror-Mad Phantasm | {3}{U}{U} | 5/1 flying Spirit | Shuffle into library, reveal until copy found. Needs: library reveal loop |
| Tree of Redemption | {3}{G} | 0/13 defender Plant | {T}: Exchange life total with toughness. Needs: life/toughness exchange |
| Grimoire of the Dead | {4} | Legendary Artifact | Counters, sacrifice to reanimate all creatures from all graveyards as Zombies. Needs: mass reanimation + type changing |

---

## Recommended Implementation Order

| Order | Tier | Cards | Engine Effort | Cumulative ISD Cards |
|-------|------|-------|--------------|---------------------|
| 1 | 5: Easy wins | 15 | Small | 94 |
| 2 | 6: Combat damage triggers | 10 | Small | 104 |
| 3 | 7: Upkeep triggers + Curses | 12 | Small-medium | 116 |
| 4 | 8: Sacrifice-as-cost | 12 | Medium | 128 |
| 5 | 10: Creature activated abilities | 12 | Medium | 140 |
| 6 | 11: Graveyard interaction | 12 | Medium | 152 |
| 7 | 9: Equipment | 12 | Medium-large | 164 |
| 8 | 12: Misc medium | 15 | Various | 179 |
| 9 | 13: Transform/DFC | 20 | Large | 199 |
| 10 | 14a: X-cost | 3 | Medium | 202 |
| 11 | 14e: Cost reduction | 3 | Medium | 205 |
| 12 | 14d: Dynamic flashback | 2 | Small-medium | 207 |
| 13 | 14f: Gain control | 3 | Medium | 210 |
| 14 | 14g: Copy effects | 2 | Medium | 212 |
| 15 | 14b: Planeswalkers | 2 | Large | 214 |
| 16 | 14c: Replacement effects | 4 | Large | 218 |
| 17 | 14h: Misc unique | 6 | Various | 224 |

Note: Some Tier 12 cards depend on Tier 14 systems (Blasphemous Act needs cost reduction, Cackling Counterpart needs copy effects, Traitorous Blood needs gain control). Those cards move to the later tier when implemented. Total unique non-basic Innistrad cards is ~249; the gap between 224 and 249 is cards that appear in multiple tiers (counted once) and basic lands.

### Milestones

- **After Tier 8** (128 cards): Over half the set. Rich limited play with all common mechanics.
- **After Tier 11** (152 cards): Solid coverage. Most commons and uncommons implemented.
- **After Tier 13** (199 cards): ~80% of the set including werewolves.
- **After Tier 14** (224+ cards): Full set coverage. Every Innistrad card implemented.

### Cross-set value (features that also unlock Eldraine cards)

- **Sacrifice-as-cost** (Tier 8): Food tokens, Witch's Oven, Cauldron Familiar
- **Equipment** (Tier 9): Crystal Slipper, Shining Armor, Scalding Cauldron
- **Combat damage triggers** (Tier 6): Many Eldraine knights and adventure creatures
- **Upkeep/end-step triggers** (Tier 7): Doom Foretold, Piper of the Swarm
- **X-cost** (Tier 14a): Stonecoil Serpent, various Eldraine X spells
- **Planeswalkers** (Tier 14b): Oko, The Royal Scions, Garruk Cursed Huntsman
- **Cost reduction** (Tier 14e): Edgewall Innkeeper (free draw), various Eldraine cost effects
- **Gain control** (Tier 14f): Oko's Elk ability, Agent of Treachery
- **Copy effects** (Tier 14g): Questing Beast interactions, token copies
