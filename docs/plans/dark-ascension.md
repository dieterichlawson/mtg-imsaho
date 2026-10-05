# Dark Ascension: the next card pool

Written 2026-10-05. The playtest nights have been finding smaller and
smaller defects for two weeks (`docs/plans/bug-pipeline-next.md`), and the
reports say why on nearly every page: "no card in the pool reaches this",
"unreachable in this pairing", "recorded as an idea instead". The engine
is converging on correctness *for Innistrad*. A new set is the one lever
that reopens the rules-bug class, because every new mechanic is new ground
the sweeps (`prompt_shapes.rs`, `deck_coverage.rs`, the invariant checker,
the nightly fuzz) walk automatically once the cards exist.

Dark Ascension is the natural next step: the same block, so the pool
stays one draft format (DKA-ISD-ISD), the same tribes and most of the
same mechanics, plus a handful of genuinely new ones.

## The set

158 cards (Scryfall, `set:dka`): 64 commons, 44 uncommons, 38 rares,
12 mythics; 78 creatures, 26 sorceries, 21 instants, 15 enchantments,
9 artifacts (2 legendary), 4 lands, 1 planeswalker; 13 transforming
double-faced cards. Card list in the appendix.

Nothing in the set has a cost shape the auto-tap planner has not met:
no three-colour cost and no two-colour cost with a repeated pip. The
planner's latent gaps (the 10-05 report) stay latent through this set.

## What is new to the engine

Grouped by mechanism, each a piece of engine work that several cards then
reuse. "Exists" means a card in Innistrad already exercises it.

| Mechanism | Cards | Status | Notes |
|---|---|---|---|
| **Undying** (CR 702.92) | 12 | new | A dies trigger that returns the card if it had no +1/+1 counter, with one. Needs last-known information for the counter check, the enters-with-counters replacement (exists, L34), and the new-object rule. Mikaeus, the Unhallowed grants it to every other non-Human creature, so it must be a keyword a continuous effect can add, not a per-card trigger. |
| **Fateful hour** | 7 | new | "If you have 5 or less life": a condition on statics (Gavony Ironwright, Village Survivors), on triggers (Thraben Doomsayer's activation is unconditional but Break of Day's bonus is), and on spells at resolution. The intervening-if machinery exists (`intervening_if.rs`); the static-condition half is new. |
| **The Increasing cycle** | 5 | partial | Flashback whose effect doubles when cast from a graveyard. The engine knows `from_graveyard` at cast; the resolving spell has to know it too. |
| **Emblems** (CR 114) | 1 | new | Sorin, Lord of Innistrad's -3. A new object type in the command zone that no effect can remove, with a continuous effect. |
| **Transforming non-creatures** | 2 | new | Chalice of Life // Chalice of Death (artifact), Elbrus, the Binding Blade // Withengar Unbound (equipment that becomes a 13/13 and unattaches). Every ISD transform is creature-to-creature; the DFC code assumes it. |
| **Copying instants and sorceries** | 2 | new | Increasing Vengeance, Curse of Echoes. ISD copies only permanent spells into tokens (Cackling Counterpart, Evil Twin). Copies of spells on the stack, with their targets and their "cast from graveyard" status (CR 707.10). |
| **Spell-count restrictions** | 1 | new | Curse of Exhaustion: "can't cast more than one spell each turn". A per-turn counter the legality check reads. |
| **Damage doubling** | 1 | new | Curse of Bloodletting: a replacement effect on damage to a player. The damage pipeline has prevention; doubling is the other direction. |
| **Playing cards from exile** | 1 | new | Fiend of the Shadows: "you may play that card for as long as it remains exiled". A permission attached to an exiled card, read by the legality check. |
| **Token copies of a card** | 1 | new | Séance: a token that is a copy of a creature card in a graveyard, exiled at end step. Copy-from-card rather than copy-from-permanent. |
| **Lords with hexproof** | 3 | partial | Drogskol, Diregraf and Stromkirk Captains: anthems exist; granting hexproof to a tribe is a continuous keyword grant (exists for Auras). |
| **Werewolves** | 9 | exists | The day/night transform conditions exist. Immerwolf ("can't transform") is a new restriction on the transform rule. |
| **Flashback, morbid, curses, equipment, fight, mill, tokens** | ~45 | exists | Reuse. |
| **Vanilla and keyword creatures** | ~40 | exists | Reuse `cards_vanilla_and_keywords.rs`'s shape. |

Rules-dense cards worth their own test files, in the order the Rules
Lawyer would probe them: Mikaeus, the Unhallowed (granted undying plus a
-1/-1 counter interaction that CR 702.92 rulings spell out), Sorin
(emblem), Elbrus (an equipment becoming a creature mid-combat), Curse of
Echoes (copies for every other player), Fiend of the Shadows, Séance,
Havengul Runebinder, Requiem Angel, Predator Ooze, Ghoultree, Alpha
Brawl, Feed the Pack.

## Stages

Each stage is a batch of small commits per CLAUDE.md, and each ends
green: `cargo check` with zero warnings and the full workspace suite.

1. **Data.** `data/sets/dka.json` in the shape of `isd.json`, with the
   set's own collation (10 commons, 3 uncommons, 1 rare or mythic, 1
   double-faced card in a common's slot, as the ISD file does it; check
   the `runs` and `approximations` sections against the printed sheets
   the way the ISD file documents). Oracle text and rulings for every
   card into `data/oracle_cache.json` through `scripts/oracle_lookup.py`
   (Scryfall is reachable from the cloud container). Card art through
   `scripts/gen_card_art.py` for the page.
2. **Mechanisms**, one commit each, with the tests that pin them before
   any card uses them: undying as a keyword plus its trigger; fateful
   hour as a condition helper; cast-from-graveyard awareness at
   resolution; emblems; non-creature transform; spell copies; the spell
   counter; damage doubling; play-from-exile; copy-from-card.
3. **Cards**, in batches that each land with a regenerated coverage deck
   set (`scripts/make_coverage_decks.py` from a registry dump; the ten
   colour-pair decks grow to about sixty cards and `deck_coverage.rs`
   keeps them honest): vanilla and keyword creatures first (the fuzz
   starts reaching the set), then the reuse mechanics (flashback, morbid,
   curses, equipment), then one mechanism's cards at a time from the
   table, rules-dense cards last.
4. **The draft runner.** `--set dka` and a DKA-ISD-ISD format, with
   `drafting.md`'s D1 (collation and conservation) re-run against the
   new file.
5. **The guides.** New Rules Lawyer ideas for each new mechanism (undying
   through every zone change and counter interaction; fateful hour at
   exactly 5 and exactly 6 life, on both sides of a life change mid-
   resolution; an emblem through a game reset; Elbrus unattaching
   mid-combat; a copied spell whose original was countered), added per
   the crew manual's "Adding an idea" rules.

Stage 1 is a day. Stage 2 is the week of real engine work. Stage 3 is
about 160 cards at the pace Innistrad's 250 were done, and the fuzz and
the nights start finding things from its first batch onward.

## Appendix: the card list

Collector number, name, cost (front face), type line, rarity, keywords.
From Scryfall on 2026-10-05.

| # | Card | Cost | Type | R | Keywords |
|---|---|---|---|---|---|
| 1 | Archangel's Light | `{7}{W}` | Sorcery | M |  |
| 2 | Bar the Door | `{2}{W}` | Instant | C |  |
| 3 | Break of Day | `{1}{W}` | Instant | C | Fateful hour |
| 4 | Burden of Guilt | `{W}` | Enchantment — Aura | C | Enchant |
| 5 | Curse of Exhaustion | `{2}{W}{W}` | Enchantment — Aura Curse | U | Enchant |
| 6 | Elgaud Inquisitor | `{3}{W}` | Creature — Human Cleric | C | Lifelink |
| 7 | Faith's Shield | `{W}` | Instant | U | Fateful hour |
| 8 | Gather the Townsfolk | `{1}{W}` | Sorcery | C | Fateful hour |
| 9 | Gavony Ironwright | `{2}{W}` | Creature — Human Soldier | U | Fateful hour |
| 10 | Hollowhenge Spirit | `{3}{W}` | Creature — Spirit | U | Flying, Flash |
| 11 | Increasing Devotion | `{3}{W}{W}` | Sorcery | R | Flashback |
| 12 | Lingering Souls | `{2}{W}` | Sorcery | U | Flashback |
| 13 | Loyal Cathar // Unhallowed Cathar | `{W}{W}` | Creature — Human Soldier // Creature — Zombie Soldier | C | Vigilance, Transform |
| 14 | Midnight Guard | `{2}{W}` | Creature — Human Soldier | C |  |
| 15 | Niblis of the Mist | `{2}{W}` | Creature — Spirit | C | Flying |
| 16 | Niblis of the Urn | `{1}{W}` | Creature — Spirit | U | Flying |
| 17 | Ray of Revelation | `{1}{W}` | Instant | C | Flashback |
| 18 | Requiem Angel | `{5}{W}` | Creature — Angel | R | Flying |
| 19 | Sanctuary Cat | `{W}` | Creature — Cat | C |  |
| 20 | Séance | `{2}{W}{W}` | Enchantment | R |  |
| 21 | Silverclaw Griffin | `{3}{W}{W}` | Creature — Griffin | C | Flying, First strike |
| 22 | Skillful Lunge | `{1}{W}` | Instant | C |  |
| 23 | Sudden Disappearance | `{5}{W}` | Sorcery | R |  |
| 24 | Thalia, Guardian of Thraben | `{1}{W}` | Legendary Creature — Human Soldier | R | First strike |
| 25 | Thraben Doomsayer | `{1}{W}{W}` | Creature — Human Cleric | R | Fateful hour |
| 26 | Thraben Heretic | `{1}{W}` | Creature — Human Wizard | U |  |
| 27 | Artful Dodge | `{U}` | Sorcery | C | Flashback |
| 28 | Beguiler of Wills | `{3}{U}{U}` | Creature — Human Wizard | M |  |
| 29 | Bone to Ash | `{2}{U}{U}` | Instant | C |  |
| 30 | Call to the Kindred | `{3}{U}` | Enchantment — Aura | R | Enchant |
| 31 | Chant of the Skifsang | `{2}{U}` | Enchantment — Aura | C | Enchant |
| 32 | Chill of Foreboding | `{2}{U}` | Sorcery | U | Flashback, Mill |
| 33 | Counterlash | `{4}{U}{U}` | Instant | R |  |
| 34 | Curse of Echoes | `{4}{U}` | Enchantment — Aura Curse | R | Enchant |
| 35 | Divination | `{2}{U}` | Sorcery | C |  |
| 36 | Dungeon Geists | `{2}{U}{U}` | Creature — Spirit | R | Flying |
| 37 | Geralf's Mindcrusher | `{4}{U}{U}` | Creature — Zombie Horror | R | Undying, Mill |
| 38 | Griptide | `{3}{U}` | Instant | C |  |
| 39 | Havengul Runebinder | `{2}{U}{U}` | Creature — Human Wizard | R |  |
| 40 | Headless Skaab | `{2}{U}` | Creature — Zombie Warrior | C |  |
| 41 | Increasing Confusion | `{X}{U}` | Sorcery | R | Flashback, Mill |
| 42 | Mystic Retrieval | `{3}{U}` | Sorcery | U | Flashback |
| 43 | Nephalia Seakite | `{3}{U}` | Creature — Bird | C | Flying, Flash |
| 44 | Niblis of the Breath | `{2}{U}` | Creature — Spirit | U | Flying |
| 45 | Relentless Skaabs | `{3}{U}{U}` | Creature — Zombie | U | Undying |
| 46 | Saving Grasp | `{U}` | Instant | C | Flashback |
| 47 | Screeching Skaab | `{1}{U}` | Creature — Zombie | C | Mill |
| 48 | Secrets of the Dead | `{2}{U}` | Enchantment | U |  |
| 49 | Shriekgeist | `{1}{U}` | Creature — Spirit | C | Flying, Mill |
| 50 | Soul Seizer // Ghastly Haunting | `{3}{U}{U}` | Creature — Spirit // Enchantment — Aura | U | Flying, Transform, Enchant |
| 51 | Stormbound Geist | `{1}{U}{U}` | Creature — Spirit | C | Flying, Undying |
| 52 | Thought Scour | `{U}` | Instant | C | Mill |
| 53 | Tower Geist | `{3}{U}` | Creature — Spirit | U | Flying |
| 54 | Black Cat | `{1}{B}` | Creature — Zombie Cat | C |  |
| 55 | Chosen of Markov // Markov's Servant | `{2}{B}` | Creature — Human // Creature — Vampire | C | Transform |
| 56 | Curse of Misfortunes | `{4}{B}` | Enchantment — Aura Curse | R | Enchant |
| 57 | Curse of Thirst | `{4}{B}` | Enchantment — Aura Curse | U | Enchant |
| 58 | Deadly Allure | `{B}` | Sorcery | U | Flashback |
| 59 | Death's Caress | `{3}{B}{B}` | Sorcery | C |  |
| 60 | Falkenrath Torturer | `{2}{B}` | Creature — Vampire | C |  |
| 61 | Farbog Boneflinger | `{4}{B}` | Creature — Zombie | U |  |
| 62 | Fiend of the Shadows | `{3}{B}{B}` | Creature — Vampire Wizard | R | Flying, Regenerate |
| 63 | Geralf's Messenger | `{B}{B}{B}` | Creature — Zombie | R | Undying |
| 64 | Gravecrawler | `{B}` | Creature — Zombie | R |  |
| 65 | Gravepurge | `{2}{B}` | Instant | C |  |
| 66 | Gruesome Discovery | `{2}{B}{B}` | Sorcery | C | Morbid |
| 67 | Harrowing Journey | `{4}{B}` | Sorcery | U |  |
| 68 | Highborn Ghoul | `{B}{B}` | Creature — Zombie | C | Intimidate |
| 69 | Increasing Ambition | `{4}{B}` | Sorcery | R | Flashback |
| 70 | Mikaeus, the Unhallowed | `{3}{B}{B}{B}` | Legendary Creature — Zombie Cleric | M | Intimidate |
| 71 | Ravenous Demon // Archdemon of Greed | `{3}{B}{B}` | Creature — Demon // Creature — Demon | R | Flying, Transform, Trample |
| 72 | Reap the Seagraf | `{2}{B}` | Sorcery | C | Flashback |
| 73 | Sightless Ghoul | `{3}{B}` | Creature — Zombie Soldier | C | Undying |
| 74 | Skirsdag Flayer | `{1}{B}` | Creature — Human Cleric | U |  |
| 75 | Spiteful Shadows | `{1}{B}` | Enchantment — Aura | C | Enchant |
| 76 | Tragic Slip | `{B}` | Instant | C | Morbid |
| 77 | Undying Evil | `{B}` | Instant | C |  |
| 78 | Vengeful Vampire | `{4}{B}{B}` | Creature — Vampire | U | Flying, Undying |
| 79 | Wakedancer | `{2}{B}` | Creature — Human Shaman | U | Morbid |
| 80 | Zombie Apocalypse | `{3}{B}{B}{B}` | Sorcery | R |  |
| 81 | Afflicted Deserter // Werewolf Ransacker | `{3}{R}` | Creature — Human Werewolf // Creature — Werewolf | U | Transform |
| 82 | Alpha Brawl | `{6}{R}{R}` | Sorcery | R |  |
| 83 | Blood Feud | `{4}{R}{R}` | Sorcery | U | Fight |
| 84 | Burning Oil | `{1}{R}` | Instant | U | Flashback |
| 85 | Curse of Bloodletting | `{3}{R}{R}` | Enchantment — Aura Curse | R | Enchant, Double |
| 86 | Erdwal Ripper | `{1}{R}{R}` | Creature — Vampire | C | Haste |
| 87 | Faithless Looting | `{R}` | Sorcery | C | Flashback |
| 88 | Fires of Undeath | `{2}{R}` | Instant | C | Flashback |
| 89 | Flayer of the Hatebound | `{5}{R}` | Creature — Devil | R | Undying |
| 90 | Fling | `{1}{R}` | Instant | C |  |
| 91 | Forge Devil | `{R}` | Creature — Devil | C |  |
| 92 | Heckling Fiends | `{2}{R}` | Creature — Devil | U |  |
| 93 | Hellrider | `{2}{R}{R}` | Creature — Devil | R | Haste |
| 94 | Hinterland Hermit // Hinterland Scourge | `{1}{R}` | Creature — Human Werewolf // Creature — Werewolf | C | Transform |
| 95 | Increasing Vengeance | `{R}{R}` | Instant | R | Flashback |
| 96 | Markov Blademaster | `{1}{R}{R}` | Creature — Vampire Warrior | R | Double strike |
| 97 | Markov Warlord | `{5}{R}` | Creature — Vampire Warrior | U | Haste |
| 98 | Mondronen Shaman // Tovolar's Magehunter | `{3}{R}` | Creature — Human Shaman Werewolf // Creature — Werewolf | R | Transform |
| 99 | Moonveil Dragon | `{3}{R}{R}{R}` | Creature — Dragon | M | Flying |
| 100 | Nearheath Stalker | `{4}{R}` | Creature — Vampire Rogue | C | Undying |
| 101 | Pyreheart Wolf | `{2}{R}` | Creature — Wolf | U | Undying |
| 102 | Russet Wolves | `{3}{R}` | Creature — Wolf | C |  |
| 103 | Scorch the Fields | `{4}{R}` | Sorcery | C |  |
| 104 | Shattered Perception | `{2}{R}` | Sorcery | U | Flashback |
| 105 | Talons of Falkenrath | `{1}{R}` | Enchantment — Aura | C | Enchant, Flash |
| 106 | Torch Fiend | `{1}{R}` | Creature — Devil | C |  |
| 107 | Wrack with Madness | `{3}{R}` | Sorcery | C |  |
| 108 | Briarpack Alpha | `{3}{G}` | Creature — Wolf | U | Flash |
| 109 | Clinging Mists | `{2}{G}` | Instant | C | Fateful hour |
| 110 | Crushing Vines | `{2}{G}` | Instant | C |  |
| 111 | Dawntreader Elk | `{1}{G}` | Creature — Elk | C |  |
| 112 | Deranged Outcast | `{1}{G}` | Creature — Human Rogue | R |  |
| 113 | Favor of the Woods | `{2}{G}` | Enchantment — Aura | C | Enchant |
| 114 | Feed the Pack | `{5}{G}` | Enchantment | R |  |
| 115 | Ghoultree | `{7}{G}` | Creature — Zombie Treefolk | R |  |
| 116 | Gravetiller Wurm | `{5}{G}` | Creature — Wurm | U | Trample, Morbid |
| 117 | Grim Flowering | `{5}{G}` | Sorcery | U |  |
| 118 | Hollowhenge Beast | `{3}{G}{G}` | Creature — Beast | C |  |
| 119 | Hunger of the Howlpack | `{G}` | Instant | C | Morbid |
| 120 | Increasing Savagery | `{2}{G}{G}` | Sorcery | R | Flashback |
| 121 | Kessig Recluse | `{2}{G}{G}` | Creature — Spider | C | Reach, Deathtouch |
| 122 | Lambholt Elder // Silverpelt Werewolf | `{2}{G}` | Creature — Human Werewolf // Creature — Werewolf | U | Transform |
| 123 | Lost in the Woods | `{3}{G}{G}` | Enchantment | R |  |
| 124 | Predator Ooze | `{G}{G}{G}` | Creature — Ooze | R | Indestructible |
| 125 | Scorned Villager // Moonscarred Werewolf | `{1}{G}` | Creature — Human Werewolf // Creature — Werewolf | C | Vigilance, Transform |
| 126 | Somberwald Dryad | `{1}{G}` | Creature — Dryad | C | Landwalk, Forestwalk |
| 127 | Strangleroot Geist | `{G}{G}` | Creature — Spirit | U | Undying, Haste |
| 128 | Tracker's Instincts | `{1}{G}` | Sorcery | U | Flashback |
| 129 | Ulvenwald Bear | `{2}{G}` | Creature — Bear | C | Morbid |
| 130 | Village Survivors | `{4}{G}` | Creature — Human | U | Fateful hour, Vigilance |
| 131 | Vorapede | `{2}{G}{G}{G}` | Creature — Insect | M | Undying, Vigilance, Trample |
| 132 | Wild Hunger | `{2}{G}` | Instant | C | Flashback |
| 133 | Wolfbitten Captive // Krallenhorde Killer | `{G}` | Creature — Human Werewolf // Creature — Werewolf | R | Transform |
| 134 | Young Wolf | `{G}` | Creature — Wolf | C | Undying |
| 135 | Diregraf Captain | `{1}{U}{B}` | Creature — Zombie Soldier | U | Deathtouch |
| 136 | Drogskol Captain | `{1}{W}{U}` | Creature — Spirit Soldier | U | Flying |
| 137 | Drogskol Reaver | `{5}{W}{U}` | Creature — Spirit | M | Flying, Lifelink, Double strike |
| 138 | Falkenrath Aristocrat | `{2}{B}{R}` | Creature — Vampire Noble | M | Flying, Haste |
| 139 | Havengul Lich | `{3}{U}{B}` | Creature — Zombie Wizard | M |  |
| 140 | Huntmaster of the Fells // Ravager of the Fells | `{2}{R}{G}` | Creature — Human Werewolf // Creature — Werewolf | M | Transform, Trample |
| 141 | Immerwolf | `{1}{R}{G}` | Creature — Wolf | U | Transform, Intimidate |
| 142 | Sorin, Lord of Innistrad | `{2}{W}{B}` | Legendary Planeswalker — Sorin | M |  |
| 143 | Stromkirk Captain | `{1}{B}{R}` | Creature — Vampire Soldier | U | First strike |
| 144 | Altar of the Lost | `{3}` | Artifact | U |  |
| 145 | Avacyn's Collar | `{1}` | Artifact — Equipment | U | Equip |
| 146 | Chalice of Life // Chalice of Death | `{3}` | Artifact // Artifact | U | Transform |
| 147 | Elbrus, the Binding Blade // Withengar Unbound | `{7}` | Legendary Artifact — Equipment // Legendary Creature — Demon | M | Flying, Transform, Trample, Equip, Intimidate |
| 148 | Executioner's Hood | `{2}` | Artifact — Equipment | C | Equip |
| 149 | Grafdigger's Cage | `{1}` | Artifact | R |  |
| 150 | Heavy Mattock | `{3}` | Artifact — Equipment | C | Equip |
| 151 | Helvault | `{3}` | Legendary Artifact | M |  |
| 152 | Jar of Eyeballs | `{3}` | Artifact | R |  |
| 153 | Warden of the Wall | `{3}` | Artifact | U |  |
| 154 | Wolfhunter's Quiver | `{1}` | Artifact — Equipment | U | Equip |
| 155 | Evolving Wilds | `` | Land | C |  |
| 156 | Grim Backwoods | `` | Land | R |  |
| 157 | Haunted Fengraf | `` | Land | C |  |
| 158 | Vault of the Archangel | `` | Land | R |  |
