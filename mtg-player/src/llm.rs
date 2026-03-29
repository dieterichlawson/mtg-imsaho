use std::env;
use std::fs::OpenOptions;
use std::io::Write;

use mtg_engine::actions::{Action, CombatPrompt};
use mtg_engine::ids::ObjectId;
use mtg_engine::types::CardType;
use mtg_engine::view::GameView;
use reqwest::blocking::Client;

use crate::Player;

const SYSTEM_PROMPT: &str = r#"You are playing Magic: The Gathering. Respond with ONLY your choice. No explanation, no reasoning, just the answer.

## Response format
- Action selection: a single number (e.g. "3")
- Choosing attackers: space-separated numbers, "all", or "none"
- Choosing blockers: "blocker:attacker" pairs (e.g. "0:0 1:2"), or "none"

You may briefly reason about your decision. Your FINAL LINE must be ONLY your answer — a single number, space-separated numbers, "all", or "none". Nothing else on that line.

Example response for action selection:
I should tap my Forest to build toward casting Kalonian Tusker next action.
ANSWER: 1

Example response for attackers:
I have two 3/3s and opponent has no blockers. Attack with everything.
ANSWER: all

Example response for blockers:
Block the 3/3 with my 2/1 to prevent damage.
ANSWER: 0:0

The system parses ONLY the last line. If the last line isn't a valid number/format, you default to passing.

## Key rules
- Mana pools empty at EVERY step boundary. Tap lands and cast spells in the same step.
- The "Cast" option only appears AFTER you have enough mana in pool. Tap lands first.
- Generic mana (numbers like {1}, {2}) can be paid with ANY color. For example, {1}{G} can be paid with {G}{G} — the first {G} pays the generic {1} cost. So if a spell costs {1}{G}, tapping two Forests ({G}{G}) is enough.
- Spells go on the stack and resolve when both players pass priority.
- Creatures have summoning sickness — can't attack the turn they enter. [S] means sick.
- Play one land per turn, only during your main phase.
- Instants can be cast anytime you have priority (including during combat or opponent's turn).
- Sorceries, creatures, enchantments, and artifacts can only be cast during your main phase with an empty stack.
- Targeted spells show their target in the action (e.g. "Cast Lightning Bolt → Goblin Piker 2/1").
- Attack to win! Creatures deal damage to the opponent when unblocked.

## Card knowledge

### Creatures
- Abbey Griffin ({3}{W} 2/2 flying, vigilance): Flyer that doesn't tap when attacking.
- Chapel Geist ({1}{W}{W} 2/3 flying): Solid flying body.
- Voiceless Spirit ({2}{W} 2/1 flying, first strike): Deals damage before blockers hit back.
- Moon Heron ({3}{U} 3/2 flying): Evasive flyer.
- Typhoid Rats ({B} 1/1 deathtouch): Kills anything it damages. Great blocker.
- Markov Patrician ({2}{B} 3/1 lifelink): You gain life equal to damage dealt.
- Ambush Viper ({1}{G} 2/1 flash, deathtouch): Cast anytime (flash)! Surprise blocker that kills anything.
- Vampire Interloper ({1}{B} 2/1 flying): Can't block.
- Spectral Rider ({W}{W} 2/2 intimidate): Only blocked by artifact creatures or same-color creatures.
- Invisible Stalker ({1}{U} 1/1 hexproof): Can't be targeted by opponents. Can't be blocked.
- Somberwald Spider ({4}{G} 2/4 reach): Can block flyers.
- Diregraf Ghoul ({B} 2/2): Enters tapped.
- Grave Bramble ({1}{G}{G} 3/4 defender): Can't attack, but great blocker.
- One-Eyed Scarecrow ({3} 2/3 artifact creature, defender): Can't attack.

### Spells
- Lightning Bolt ({R} instant): Deal 3 damage to any target. Use to kill creatures or finish off opponent.
- Giant Growth ({G} instant): Target creature gets +3/+3 until end of turn. SAVE THIS FOR COMBAT — cast it during DeclareBlockers to pump your attacker or save a blocker.
- Doom Blade ({1}{B} instant): Destroy target nonblack creature. Can cast during combat to remove a blocker before damage.
- Swords to Plowshares ({W} instant): Exile target creature. Controller gains life equal to power. Best used on big threats.
- Counterspell ({U}{U} instant): Counter target spell on the stack. Use when opponent casts a threatening creature or spell.
- Dissipate ({1}{U}{U} instant): Counter target spell and exile it (not graveyard).
- Frightful Delusion ({2}{U} instant): Counter target spell. Its controller discards a card.
- Lost in the Mist ({3}{U}{U} instant): Counter target spell AND return target permanent to its owner's hand. Two-for-one!
- Holy Strength ({W} aura): Enchanted creature gets +1/+2. Cast on your creatures during main phase.
- Pacifism ({1}{W} aura): Enchanted creature can't attack or block. Cast on opponent's creatures.
- Rebuke ({2}{W} instant): Destroy target ATTACKING creature. Cast during combat after attackers are declared.
- Smite the Monstrous ({3}{W} instant): Destroy target creature with power 4 or greater.
- Victim of Night ({B}{B} instant): Destroy target non-Vampire, non-Werewolf, non-Zombie creature.
- Geistflame ({R} instant): Deal 1 damage to any target.
- Brimstone Volley ({2}{R} instant): Deal 3 damage to any target.
- Bump in the Night ({B} sorcery): Target opponent loses 3 life.
- Silent Departure ({U} sorcery): Return target creature to its owner's hand.
- Naturalize ({1}{G} instant): Destroy target artifact or enchantment. Use to remove auras like Pacifism from your creatures!
- Urgent Exorcism ({1}{W} instant): Destroy target Spirit or enchantment.
- Bramblecrush ({2}{G}{G} sorcery): Destroy target noncreature permanent (lands, artifacts, enchantments).
- Prey Upon ({G} sorcery): Target creature you control fights target creature you don't control. Both deal damage equal to their power to the other.
- Moment of Heroism ({1}{W} instant): Target creature gets +2/+2 and gains lifelink until end of turn.
- Ranger's Guile ({G} instant): Target creature gets +1/+1 and gains hexproof until end of turn.
- Spidery Grasp ({2}{G} instant): Untap target creature. It gets +2/+4 and gains reach until end of turn.
- Rally the Peasants ({2}{W} instant): Creatures you control get +2/+0 until end of turn.
- Vampiric Fury ({1}{R} instant): Vampire creatures you control get +2/+0 and gain first strike until end of turn.
- Midnight Haunting ({2}{W} instant): Create two 1/1 white Spirit creature tokens with flying. Great for surprise blockers!
- Moan of the Unhallowed ({2}{B}{B} sorcery): Create two 2/2 black Zombie creature tokens.
- Doomed Traveler ({W} creature 1/1): When it dies, creates a 1/1 Spirit token with flying. Good early blocker.
- Mausoleum Guard ({3}{W} creature 2/2): When it dies, creates two 1/1 Spirit tokens with flying.
- Village Bell-Ringer ({2}{W} creature 1/4 flash): When it enters, untaps all your creatures. Cast during combat to untap blockers!
- Slayer of the Wicked ({3}{W} creature 3/2): When it enters, destroys a Vampire, Werewolf, or Zombie.
- Pitchburn Devils ({4}{R} creature 3/3): When it dies, deals 3 damage to opponent.
- Falkenrath Noble ({3}{B} creature 2/2 flying): Whenever ANY creature dies, opponent loses 1 life and you gain 1.
- Rage Thrower ({5}{R} creature 4/2): Whenever another creature dies, deals 2 damage to opponent.
- Fiend Hunter ({1}{W}{W} creature 1/3): When it enters, exiles an opponent's creature.
- Intangible Virtue ({1}{W} enchantment): Your creatures get +1/+1.
- Unruly Mob ({1}{W} creature 1/1): Gets a +1/+1 counter whenever another of your creatures dies.
- Lumberknot ({2}{G}{G} creature 1/1 hexproof): Gets a +1/+1 counter whenever any creature dies. Can't be targeted!
- Elder Cathar ({2}{W} creature 2/2): When it dies, puts a +1/+1 counter on one of your creatures.
- Think Twice ({1}{U} instant, flashback {2}{U}): Draw a card. Can cast from graveyard!
- Dream Twist ({U} instant, flashback {1}{U}): Target player mills 3 cards.
- Travel Preparations ({1}{G} sorcery, flashback {1}{W}): Put a +1/+1 counter on target creature.
- Feeling of Dread ({1}{W} instant, flashback {1}{U}): Tap target creature.
- Nightbird's Clutches ({1}{R} sorcery, flashback {3}{R}): Tap target creature so it can't block.
- Gnaw to the Bone ({2}{G} instant, flashback {2}{G}): Gain 2 life per creature in your graveyard.
- Forbidden Alchemy ({2}{U} instant, flashback {6}{B}): Draw 1 card, mill 3.
- Rolling Temblor ({2}{R} sorcery, flashback {4}{R}{R}): 2 damage to each creature without flying.
- Unburial Rites ({4}{B} sorcery, flashback {3}{W}): Return a creature from your graveyard to the battlefield.
- Desperate Ravings ({1}{R} instant, flashback {2}{U}): Draw 2 cards, discard 1.

## Flashback
Cards with flashback can be cast from your graveyard for their flashback cost. After resolving, they are exiled (not returned to graveyard). Look for "Flashback" in the action list — these are graveyard casts. Tap lands to get mana, then the Flashback option appears.

## Strategy tips
- Save instants for combat! Giant Growth during DeclareBlockers makes your 2/2 into a 5/5. Lightning Bolt during DeclareAttackers can kill a would-be blocker.
- Don't cast Giant Growth during your main phase — it wears off at end of turn and wastes it if there's no combat.
- Use removal (Doom Blade, Swords, Lightning Bolt) on your opponent's biggest creatures, especially before they attack you.
- Cast creatures and play lands during PrecombatMain. Use PostcombatMain for spells you want to cast after seeing how combat went.
- Don't tap lands unless you have something to cast with the mana. Tapping 1 land when you need 2 for a spell just wastes it.
- TAP THE RIGHT COLORS! Look at the costs of cards in your hand. If you need {G}{G}, tap Forests not Mountains. If you need {1}{R}, tap one Mountain and one of anything. The action list shows "Tap Forest" vs "Tap Mountain" — pick the ones matching your spell's colored requirements first.

## CRITICAL RULE: Only tap lands during PrecombatMain or PostcombatMain

The step name is shown at the top of every prompt (e.g. "T3 PrecombatMain", "T3 Upkeep", "T3 Draw").
Mana pools empty between EVERY step. If you tap during Upkeep, Draw, BeginCombat, EndStep, or any non-main step, the mana disappears before you can use it and your lands are tapped for nothing.

EXCEPTION: You MAY tap lands during combat steps (DeclareAttackers, DeclareBlockers) to cast instants like Lightning Bolt or Giant Growth. This is useful for removing blockers or pumping attackers.

## Mistake example: Tapping during Upkeep or Draw

```
T3 Upkeep You:20hp Opp:20hp,6cards
Your board: 2xForest
0:Pass 1:Tap Forest 2:Tap Forest 3:Concede
```
If you answer 1 here (tap Forest), you get {G} in your pool. But when Upkeep ends and Draw begins, YOUR MANA POOL EMPTIES. By the time you reach PrecombatMain, the mana is gone and your lands are tapped for nothing.
Answer: 0

Same applies to Draw step — don't tap unless you have an instant to cast RIGHT NOW.

## Correct example: Full main phase sequence

```
T3 PrecombatMain You:20hp Opp:20hp,6cards
Your board: 2xForest
Hand: Kalonian Tusker{G}{G} 3/3, Forest, Forest
0:Pass 1:Tap Forest 2:Tap Forest 3:Play Forest 4:Play Forest 5:Concede
```
Answer: 1 (tap Forest → {G})

```
T3 PrecombatMain You:20hp Opp:20hp,6cards
Pool: {Green: 1}
Your board: 2xForest(1tapped)
Hand: Kalonian Tusker{G}{G} 3/3, Forest, Forest
0:Pass 1:Tap Forest 2:Play Forest 3:Play Forest 4:Concede
```
Answer: 1 (tap Forest → {G}{G})

```
T3 PrecombatMain You:20hp Opp:20hp,6cards
Pool: {Green: 2}
Your board: 2xForest(tapped)
Hand: Kalonian Tusker{G}{G} 3/3, Forest, Forest
0:Pass 1:Cast Kalonian Tusker 2:Play Forest 3:Play Forest 4:Concede
```
Answer: 1 (cast!)

## Correct example: Using an instant during combat

```
T5 DeclareBlockers You:20hp Opp:18hp,4cards
Pool: {Green: 1}
Your board: 2xForest(1tapped), Grizzly Bears 2/2[T]
Opp board: 2xPlains, Savannah Lions 2/1
Hand: Giant Growth{G}
Attackers: 0:Grizzly Bears 2/2
0:Pass 1:Cast Giant Growth → Grizzly Bears 2/2 2:Concede
```
Answer: 1 (pump your attacking creature to 5/5 before blockers deal damage!)

## Correct example: Declare attackers

```
T5 DeclareAttackers You:20hp Opp:20hp,5cards
Your board: 3xForest(tapped), Kalonian Tusker 3/3, Kalonian Tusker 3/3
Opp board: 2xMountain, Goblin Piker 2/1
Choose attackers: 0:Kalonian Tusker 3/3 1:Kalonian Tusker 3/3
Numbers, 'all', or 'none'
```
Answer: all

## Correct example: Declare blockers

```
T6 DeclareBlockers You:17hp Opp:20hp,5cards
Your board: 3xMountain, Goblin Piker 2/1, Goblin Piker 2/1
Opp board: 3xForest(tapped), Kalonian Tusker 3/3[T], Kalonian Tusker 3/3[T]
Attackers: 0:Kalonian Tusker 3/3 1:Kalonian Tusker 3/3
Your blockers: 0:Goblin Piker 2/1 1:Goblin Piker 2/1
Format: 'blocker:attacker' pairs, or 'none'
```
Answer: 0:0 1:1
(Block both. Your 2/1s die but prevent 6 damage.)

IMPORTANT: For blocking, the format is BLOCKER_NUMBER:ATTACKER_NUMBER (e.g. "0:0" NOT "0:" or "0>0"). Both numbers are required.
"#;

#[derive(Clone)]
pub enum Provider {
    Anthropic,
    Gemini,
}

pub struct LlmPlayer {
    name: String,
    client: Client,
    api_key: String,
    model: String,
    provider: Provider,
    log_file: Option<String>,
}

impl LlmPlayer {
    pub fn new(name: &str) -> Self {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .expect("ANTHROPIC_API_KEY environment variable must be set");

        Self {
            name: name.to_string(),
            client: Client::new(),
            api_key,
            model: "claude-sonnet-4-20250514".to_string(),
            provider: Provider::Anthropic,
            log_file: None,
        }
    }

    pub fn new_gemini(name: &str) -> Self {
        let api_key = env::var("GEMINI_API_KEY")
            .expect("GEMINI_API_KEY environment variable must be set");

        Self {
            name: name.to_string(),
            client: Client::new(),
            api_key,
            model: "gemini-2.5-flash".to_string(),
            provider: Provider::Gemini,
            log_file: None,
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    pub fn with_log(mut self, path: &str) -> Self {
        // Truncate the file at start.
        let _ = std::fs::write(path, "");
        self.log_file = Some(path.to_string());
        self
    }

    fn log(&self, label: &str, content: &str) {
        if let Some(path) = &self.log_file {
            if let Ok(mut f) = OpenOptions::new().append(true).create(true).open(path) {
                let _ = writeln!(f, "=== {} [{}] ===", label, self.name);
                let _ = writeln!(f, "{}", content);
                let _ = writeln!(f);
            }
        }
    }

    /// Check if the AI should auto-pass (nothing interesting to do).
    fn should_auto_pass(_view: &GameView, actions: &[Action]) -> bool {
        // Auto-pass when the only options are Pass and Concede.
        let has_pass = actions.iter().any(|a| matches!(a, Action::PassPriority));
        if !has_pass {
            return false;
        }
        actions.iter().all(|a| matches!(a, Action::PassPriority | Action::Concede))
    }

    /// Compact game state — much shorter than the CLI version.
    fn format_state_compact(view: &GameView) -> String {
        let mut s = String::new();
        s.push_str(&format!("T{} {:?} ", view.turn_number, view.step));
        s.push_str(&format!("You:{}hp Opp:", view.your_life));
        for opp in &view.opponents {
            s.push_str(&format!("{}hp,{}cards ", opp.life, opp.hand_size));
        }
        s.push('\n');

        if !view.your_mana_pool.is_empty() {
            s.push_str(&format!("Pool: {:?}\n", view.your_mana_pool.mana));
        }

        // Battlefield — compact
        let your_perms: Vec<_> = view.battlefield.iter().filter(|p| p.controller == view.you).collect();
        let opp_perms: Vec<_> = view.battlefield.iter().filter(|p| p.controller != view.you).collect();
        let all_perms: Vec<_> = view.battlefield.iter().collect();

        if !your_perms.is_empty() {
            s.push_str("Your board: ");
            s.push_str(&Self::format_perms_compact(&your_perms, &all_perms));
            s.push('\n');
        }
        if !opp_perms.is_empty() {
            s.push_str("Opp board: ");
            s.push_str(&Self::format_perms_compact(&opp_perms, &all_perms));
            s.push('\n');
        }

        // Stack
        if !view.stack.is_empty() {
            s.push_str("Stack: ");
            let items: Vec<String> = view.stack.iter()
                .map(|i| {
                    let who = if i.controller == view.you { "your" } else { "opp's" };
                    format!("{} ({})", i.name, who)
                })
                .collect();
            s.push_str(&items.join(", "));
            s.push('\n');
        }

        // Hand
        if !view.your_hand.is_empty() {
            s.push_str("Hand: ");
            let cards: Vec<String> = view.your_hand.iter()
                .map(|c| {
                    let cost = c.cost.as_ref().map(|co| format!("{}", co)).unwrap_or_default();
                    let pt = match (c.power, c.toughness) {
                        (Some(p), Some(t)) => format!(" {}/{}", p, t),
                        _ => String::new(),
                    };
                    format!("{}{}{}", c.name, cost, pt)
                })
                .collect();
            s.push_str(&cards.join(", "));
            s.push('\n');
        }

        // Show flashback-eligible cards in your graveyard.
        let your_gy = view.graveyards.iter()
            .find(|(pid, _)| *pid == view.you)
            .map(|(_, cards)| cards);
        if let Some(gy_cards) = your_gy {
            let fb_cards: Vec<String> = gy_cards.iter()
                .filter(|c| c.flashback_cost.is_some())
                .map(|c| {
                    let fb = c.flashback_cost.as_ref().unwrap();
                    format!("{} (flashback {})", c.name, fb)
                })
                .collect();
            if !fb_cards.is_empty() {
                s.push_str("Graveyard flashback: ");
                s.push_str(&fb_cards.join(", "));
                s.push('\n');
            }
        }

        s
    }

    fn format_perms_compact(perms: &[&mtg_engine::view::PermanentView], all_perms: &[&mtg_engine::view::PermanentView]) -> String {
        // Group lands by name with tapped count.
        let lands: Vec<_> = perms.iter().filter(|p| p.card_types.contains(&CardType::Land)).collect();
        let creatures: Vec<_> = perms.iter().filter(|p| p.card_types.contains(&CardType::Creature)).collect();
        let other: Vec<_> = perms.iter().filter(|p|
            !p.card_types.contains(&CardType::Land) && !p.card_types.contains(&CardType::Creature)
        ).collect();

        let mut parts = Vec::new();

        if !lands.is_empty() {
            let mut land_groups: Vec<(String, usize, usize)> = Vec::new();
            for land in &lands {
                if let Some(entry) = land_groups.iter_mut().find(|(n, _, _)| *n == land.name) {
                    if land.tapped { entry.2 += 1; } else { entry.1 += 1; }
                } else {
                    let (u, t) = if land.tapped { (0, 1) } else { (1, 0) };
                    land_groups.push((land.name.clone(), u, t));
                }
            }
            for (name, untapped, tapped) in &land_groups {
                let total = untapped + tapped;
                if *tapped == 0 {
                    parts.push(format!("{}x{}", total, name));
                } else if *untapped == 0 {
                    parts.push(format!("{}x{}(tapped)", total, name));
                } else {
                    parts.push(format!("{}x{}({}tapped)", total, name, tapped));
                }
            }
        }

        // Collect aura names by what they're attached to — search ALL permanents
        // so we find auras that cross controller boundaries (e.g., opponent's
        // Pacifism on your creature).
        let mut aura_map: std::collections::HashMap<mtg_engine::ids::ObjectId, Vec<String>> = std::collections::HashMap::new();
        for o in all_perms {
            if o.attached_to.is_some() && !o.card_types.contains(&CardType::Land) && !o.card_types.contains(&CardType::Creature) {
                if let Some(target_id) = o.attached_to {
                    aura_map.entry(target_id).or_default().push(o.name.clone());
                }
            }
        }

        for c in &creatures {
            let t = if c.tapped { "T" } else { "" };
            let s = if c.summoning_sick { "S" } else { "" };
            let d = if c.damage_marked > 0 { format!("{}d", c.damage_marked) } else { String::new() };
            let flags = format!("{}{}{}", t, s, d);
            let flags_str = if flags.is_empty() { String::new() } else { format!("[{}]", flags) };
            let auras = aura_map.get(&c.object_id)
                .map(|names| format!("({})", names.join(",")))
                .unwrap_or_default();
            parts.push(format!("{} {}/{}{}{}", c.name, c.power.unwrap_or(0), c.toughness.unwrap_or(0), flags_str, auras));
        }

        // Show non-aura other permanents.
        for o in &other {
            if o.attached_to.is_some() { continue; } // skip auras, shown with creature
            let t = if o.tapped { "[T]" } else { "" };
            parts.push(format!("{}{}", o.name, t));
        }

        parts.join(", ")
    }

    /// Format a single non-CastSpell action for the collapsed display.
    fn format_single_action(view: &GameView, action: &Action) -> String {
        match action {
            Action::PassPriority => "Pass".into(),
            Action::PlayLand { object_id } => format!("Play {}", Self::obj_name(view, *object_id)),
            Action::ActivateManaAbility { object_id, .. } => format!("Tap {}", Self::obj_name(view, *object_id)),
            Action::ActivateAbility { object_id, .. } => format!("Activate {}", Self::obj_name(view, *object_id)),
            Action::Concede => "Concede".into(),
            Action::DiscardCards { cards } => format!("Discard {} cards", cards.len()),
            Action::ResolveChoice { choice } => {
                use mtg_engine::actions::ResolvedChoice;
                match choice {
                    ResolvedChoice::PayDecision(true) => "Pay {1}".into(),
                    ResolvedChoice::PayDecision(false) => "Don't pay (countered)".into(),
                    ResolvedChoice::ChosenTarget(Some(t)) => {
                        match t {
                            mtg_engine::actions::Target::Object(id) => Self::obj_name(view, *id),
                            mtg_engine::actions::Target::Player(pid) => {
                                if *pid == view.you { "You".into() } else { "Opponent".into() }
                            }
                        }
                    }
                    ResolvedChoice::ChosenTarget(None) => "Decline".into(),
                    ResolvedChoice::ChosenCard(id) => Self::obj_name(view, *id),
                }
            }
            other => format!("{}", other),
        }
    }

    /// Second API call: choose targets for a castable spell.
    fn choose_cast_targets(&mut self, view: &GameView, spell: &mtg_engine::actions::CastableSpell, legal_actions: &[Action]) -> Action {
        use mtg_engine::actions::{CastTargetSpec, Target};

        match &spell.target_spec {
            CastTargetSpec::NoTargets => {
                Action::CastSpell { object_id: spell.object_id, targets: vec![] }
            }
            CastTargetSpec::SingleTarget(options) => {
                if options.len() == 1 {
                    return Action::CastSpell { object_id: spell.object_id, targets: vec![options[0].clone()] };
                }
                let target = self.prompt_target_selection(view, &spell.name, options);
                Action::CastSpell { object_id: spell.object_id, targets: vec![target] }
            }
            CastTargetSpec::TwoTargets(options1, options2) => {
                let t1 = self.prompt_target_selection(view, &format!("{} (first target)", spell.name), options1);
                let remaining: Vec<_> = options2.iter().filter(|t| **t != t1).cloned().collect();
                if remaining.is_empty() {
                    // Fallback: find any matching expanded action
                    return self.fallback_to_expanded(spell.object_id, legal_actions);
                }
                let t2 = self.prompt_target_selection(view, &format!("{} (second target)", spell.name), &remaining);
                Action::CastSpell { object_id: spell.object_id, targets: vec![t1, t2] }
            }
            CastTargetSpec::UpToTargets { max, options } => {
                // For the LLM, present all options and ask to pick numbers.
                let target_list: String = options.iter().enumerate()
                    .map(|(i, t)| {
                        let desc = match t {
                            Target::Object(id) => Self::obj_name(view, *id),
                            Target::Player(pid) => if *pid == view.you { "you".into() } else { "opponent".into() },
                        };
                        format!("{}:{}", i, desc)
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                let prompt = format!(
                    "Choose up to {} targets for {}:\n{}\nRespond with space-separated numbers (e.g. '0 2')",
                    max, spell.name, target_list
                );
                self.log("TARGETS", &prompt);
                let response = self.call_api(&prompt);
                self.log("TARGET-RESPONSE", &response);

                let last_line = response.lines().rev()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or(&response)
                    .trim();
                let answer = last_line.strip_prefix("ANSWER:").or_else(|| last_line.strip_prefix("Answer:"))
                    .unwrap_or(last_line)
                    .trim();

                let chosen: Vec<Target> = answer.split_whitespace()
                    .filter_map(|s| s.parse::<usize>().ok())
                    .filter(|&i| i < options.len())
                    .map(|i| options[i].clone())
                    .collect();

                if chosen.is_empty() {
                    // Pick at least one — use the first option.
                    Action::CastSpell { object_id: spell.object_id, targets: vec![options[0].clone()] }
                } else {
                    Action::CastSpell { object_id: spell.object_id, targets: chosen }
                }
            }
        }
    }

    /// Make a second API call to select one target from a list.
    fn prompt_target_selection(&mut self, view: &GameView, spell_name: &str, options: &[mtg_engine::actions::Target]) -> mtg_engine::actions::Target {
        let target_list: String = options.iter().enumerate()
            .map(|(i, t)| {
                let desc = match t {
                    mtg_engine::actions::Target::Object(id) => Self::obj_name(view, *id),
                    mtg_engine::actions::Target::Player(pid) => if *pid == view.you { "you".into() } else { "opponent".into() },
                };
                format!("{}:{}", i, desc)
            })
            .collect::<Vec<_>>()
            .join(" ");
        let prompt = format!("Choose target for {}:\n{}", spell_name, target_list);
        self.log("TARGETS", &prompt);
        let idx = self.choose_with_retry(&prompt, options.len(), &[]);
        options[idx.min(options.len() - 1)].clone()
    }

    /// Fallback: find the first matching expanded action for this spell.
    fn fallback_to_expanded(&self, object_id: ObjectId, legal_actions: &[Action]) -> Action {
        legal_actions.iter()
            .find(|a| matches!(a, Action::CastSpell { object_id: oid, .. } if *oid == object_id))
            .cloned()
            .unwrap_or(Action::PassPriority)
    }

    fn obj_name(view: &GameView, id: ObjectId) -> String {
        view.battlefield.iter()
            .find(|p| p.object_id == id)
            .map(|p| p.name.clone())
            .or_else(|| view.your_hand.iter()
                .find(|c| c.object_id == id)
                .map(|c| c.name.clone()))
            .or_else(|| view.stack.iter()
                .find(|s| s.object_id == id)
                .map(|s| s.name.clone()))
            .or_else(|| view.graveyards.iter()
                .flat_map(|(_, cards)| cards.iter())
                .find(|c| c.object_id == id)
                .map(|c| c.name.clone()))
            .unwrap_or_else(|| format!("{}", id))
    }

    fn call_api(&self, user_message: &str) -> String {
        self.log("PROMPT", user_message);

        match self.provider {
            Provider::Anthropic => self.call_anthropic(user_message),
            Provider::Gemini => self.call_gemini(user_message),
        }
    }

    fn call_anthropic(&self, user_message: &str) -> String {
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 4096,
            "system": SYSTEM_PROMPT,
            "messages": [
                {"role": "user", "content": user_message}
            ]
        });

        for attempt in 0..3 {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(2u64.pow(attempt as u32));
                self.log("RETRY", &format!("Retrying in {}s...", delay.as_secs()));
                std::thread::sleep(delay);
            }

            let response = self.client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send();

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let json: serde_json::Value = resp.json().unwrap_or_default();
                        let result = json["content"][0]["text"]
                            .as_str()
                            .unwrap_or("0")
                            .trim()
                            .to_string();
                        self.log("RESPONSE", &result);
                        return result;
                    }

                    let status = resp.status();
                    let text = resp.text().unwrap_or_default();
                    self.log("API-ERROR", &format!("{}: {}", status, text));
                    self.log("ERROR", &format!("API {} - {}", status, text));

                    // Retry on overload (529) or rate limit (429).
                    let code = status.as_u16();
                    if code == 529 || code == 429 {
                        continue;
                    }
                    // Non-retryable error.
                    return "0".to_string();
                }
                Err(e) => {
                    self.log("REQUEST-FAILED", &format!("{}", e));
                    self.log("ERROR", &format!("Request failed: {}", e));
                    continue;
                }
            }
        }

        self.log("ERROR", "All retries exhausted, defaulting to 0");
        "0".to_string()
    }

    fn call_gemini(&self, user_message: &str) -> String {
        let body = serde_json::json!({
            "contents": [
                {"role": "user", "parts": [{"text": user_message}]}
            ],
            "systemInstruction": {"parts": [{"text": SYSTEM_PROMPT}]},
            "generationConfig": {"maxOutputTokens": 512}
        });

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        for attempt in 0..3 {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(2u64.pow(attempt as u32));
                self.log("RETRY", &format!("Retrying in {}s...", delay.as_secs()));
                std::thread::sleep(delay);
            }

            let response = self.client
                .post(&url)
                .header("content-type", "application/json")
                .json(&body)
                .send();

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let json: serde_json::Value = resp.json().unwrap_or_default();
                        let result = json["candidates"][0]["content"]["parts"][0]["text"]
                            .as_str()
                            .unwrap_or("0")
                            .trim()
                            .to_string();
                        self.log("RESPONSE", &result);
                        return result;
                    }

                    let status = resp.status();
                    let text = resp.text().unwrap_or_default();
                    self.log("API-ERROR", &format!("{}: {}", status, text));
                    self.log("ERROR", &format!("API {} - {}", status, text));

                    let code = status.as_u16();
                    if code == 429 || code == 503 || code == 529 {
                        continue;
                    }
                    return "0".to_string();
                }
                Err(e) => {
                    self.log("REQUEST-FAILED", &format!("{}", e));
                    self.log("ERROR", &format!("Request failed: {}", e));
                    continue;
                }
            }
        }

        self.log("ERROR", "All retries exhausted, defaulting to 0");
        "0".to_string()
    }

    fn parse_action_index(&self, response: &str, max: usize) -> Option<usize> {
        // Check the last non-empty line first (where the model puts its final answer).
        let last_line = response.lines().rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or(response)
            .trim();

        // Strip "ANSWER:" prefix if present.
        let answer = last_line.strip_prefix("ANSWER:").or_else(|| last_line.strip_prefix("Answer:"))
            .unwrap_or(last_line)
            .trim();

        for word in answer.split_whitespace() {
            if let Ok(n) = word.parse::<usize>() {
                if n < max {
                    return Some(n);
                }
            }
        }

        // Fallback: scan the entire response for any valid number.
        for word in response.split_whitespace() {
            if let Ok(n) = word.parse::<usize>() {
                if n < max {
                    return Some(n);
                }
            }
        }
        None
    }

    /// Call the API with a retry on malformed response.
    fn choose_with_retry(&self, prompt: &str, max: usize, actions: &[Action]) -> usize {
        for attempt in 0..2 {
            let response = self.call_api(prompt);
            if let Some(idx) = self.parse_action_index(&response, max) {
                // Double-check concede: ask the model to confirm.
                if matches!(actions.get(idx), Some(Action::Concede)) {
                    self.log("CONCEDE-CHECK", "AI chose Concede, confirming...");
                    let confirm = self.call_api(
                        "You chose to CONCEDE the game. Are you sure? Reply ONLY 'yes' or 'no'."
                    );
                    let last = confirm.lines().rev()
                        .find(|l| !l.trim().is_empty())
                        .unwrap_or("")
                        .trim()
                        .to_lowercase();
                    if !last.contains("yes") {
                        self.log("CONCEDE-CHECK", "Concede cancelled, passing instead");
                        return 0;
                    }
                }
                self.log("CHOSE", &format!("action {}", idx));
                return idx;
            }
            if attempt == 0 {
                self.log("MALFORMED", &format!("'{}', retrying...", response));
            } else {
                self.log("MALFORMED", &format!("Retry also malformed '{}', defaulting to 0", response));
            }
        }
        0
    }
}

impl Player for LlmPlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn choose_action(&mut self, view: &GameView, legal: &mtg_engine::engine::LegalActions) -> Action {
        let legal_actions = &legal.actions;
        // Auto-pass when there's nothing interesting to do.
        if Self::should_auto_pass(view, legal_actions) {
            self.log("AUTO-PASS", &format!("Step: {:?}, active: p#{}", view.step, view.active_player.0));
            return Action::PassPriority;
        }

        // Build collapsed display: non-CastSpell actions + one per castable spell.
        let mut display_labels = Vec::new();
        enum DisplayEntry {
            Direct(usize),   // index into legal_actions
            Cast(usize),     // index into legal.castable_spells
        }
        let mut display_entries: Vec<DisplayEntry> = Vec::new();
        let mut seen_spell_objects: Vec<mtg_engine::ids::ObjectId> = Vec::new();

        for (i, action) in legal_actions.iter().enumerate() {
            match action {
                Action::CastSpell { object_id, .. } => {
                    if !seen_spell_objects.contains(object_id) {
                        if let Some(cs_idx) = legal.castable_spells.iter()
                            .position(|cs| cs.object_id == *object_id)
                        {
                            seen_spell_objects.push(*object_id);
                            let cs = &legal.castable_spells[cs_idx];
                            let verb = if cs.is_flashback { "Flashback" } else { "Cast" };
                            display_labels.push(format!("{} {}", verb, cs.name));
                            display_entries.push(DisplayEntry::Cast(cs_idx));
                        }
                    }
                }
                _ => {
                    display_labels.push(Self::format_single_action(view, action));
                    display_entries.push(DisplayEntry::Direct(i));
                }
            }
        }

        let state_str = Self::format_state_compact(view);
        let actions_str: String = display_labels.iter().enumerate()
            .map(|(i, label)| format!("{}:{} ", i, label))
            .collect();
        let prompt = format!("{}\n{}", state_str, actions_str);

        self.log("THINKING", &format!("{} actions (collapsed from {})", display_labels.len(), legal_actions.len()));
        let idx = self.choose_with_retry(&prompt, display_labels.len(), legal_actions);

        if idx >= display_entries.len() {
            return Action::PassPriority;
        }

        match &display_entries[idx] {
            DisplayEntry::Direct(action_idx) => {
                legal_actions[*action_idx].clone()
            }
            DisplayEntry::Cast(cs_idx) => {
                let cs = &legal.castable_spells[*cs_idx];
                self.choose_cast_targets(view, cs, legal_actions)
            }
        }
    }

    fn choose_cards_to_bottom(
        &mut self,
        _view: &GameView,
        hand: &[mtg_engine::view::CardView],
        count: usize,
    ) -> Vec<ObjectId> {
        hand.iter().take(count).map(|c| c.object_id).collect()
    }
}

impl LlmPlayer {
    pub fn choose_combat(&mut self, view: &GameView, prompt: &CombatPrompt) -> Action {
        match prompt {
            CombatPrompt::ChooseAttackers { eligible, must_attack, defending_player } => {
                if eligible.is_empty() {
                    return Action::DeclareAttackers { attackers: vec![] };
                }

                let state = Self::format_state_compact(view);
                let mut combat_text = String::new();
                if !must_attack.is_empty() {
                    combat_text.push_str("MUST ATTACK: ");
                    for &id in must_attack.iter() {
                        if let Some(idx) = eligible.iter().position(|&e| e == id) {
                            let p = view.battlefield.iter().find(|p| p.object_id == id);
                            let name = p.map(|p| format!("{} {}/{}", p.name, p.power.unwrap_or(0), p.toughness.unwrap_or(0)))
                                .unwrap_or_else(|| format!("{}", id));
                            combat_text.push_str(&format!("{}:{} ", idx, name));
                        }
                    }
                    combat_text.push('\n');
                }
                combat_text.push_str("Choose attackers: ");
                for (i, &id) in eligible.iter().enumerate() {
                    let p = view.battlefield.iter().find(|p| p.object_id == id);
                    let name = p.map(|p| format!("{} {}/{}", p.name, p.power.unwrap_or(0), p.toughness.unwrap_or(0)))
                        .unwrap_or_else(|| format!("{}", id));
                    let forced = if must_attack.contains(&id) { " [MUST]" } else { "" };
                    combat_text.push_str(&format!("{}:{}{} ", i, name, forced));
                }
                combat_text.push_str("\nNumbers, 'all', or 'none' (forced attackers are auto-included)");

                self.log("THINKING", "attackers...");
                let response = self.call_api(&format!("{}\n{}", state, combat_text));
                self.log("ATTACKERS", &response);

                // Parse from last line, strip "ANSWER:" prefix.
                let last_line = response.lines().rev()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or(&response)
                    .trim();
                let answer = last_line.strip_prefix("ANSWER:").or_else(|| last_line.strip_prefix("Answer:"))
                    .unwrap_or(last_line)
                    .trim()
                    .to_lowercase();

                if answer.contains("none") || answer.is_empty() {
                    return Action::DeclareAttackers { attackers: vec![] };
                }
                if answer.contains("all") {
                    return Action::DeclareAttackers {
                        attackers: eligible.iter().map(|&id| (id, *defending_player)).collect(),
                    };
                }

                let attackers = answer.split_whitespace()
                    .filter_map(|s| s.parse::<usize>().ok())
                    .filter(|&i| i < eligible.len())
                    .map(|i| (eligible[i], *defending_player))
                    .collect();
                Action::DeclareAttackers { attackers }
            }

            CombatPrompt::ChooseBlockers { eligible_blockers, attackers } => {
                if eligible_blockers.is_empty() || attackers.is_empty() {
                    return Action::DeclareBlockers { assignments: vec![] };
                }

                let state = Self::format_state_compact(view);
                let mut combat_text = String::from("Attackers: ");
                for (i, &id) in attackers.iter().enumerate() {
                    let name = Self::obj_name(view, id);
                    combat_text.push_str(&format!("{}:{} ", i, name));
                }
                combat_text.push_str("\nYour blockers: ");
                for (i, &id) in eligible_blockers.iter().enumerate() {
                    let name = Self::obj_name(view, id);
                    combat_text.push_str(&format!("{}:{} ", i, name));
                }
                combat_text.push_str("\nFormat: 'blocker:attacker' pairs, or 'none'");

                self.log("THINKING", "blockers...");
                let response = self.call_api(&format!("{}\n{}", state, combat_text));
                self.log("BLOCKERS", &response);

                // Parse from last line, strip "ANSWER:" prefix.
                let last_line = response.lines().rev()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or(&response)
                    .trim();
                let answer = last_line.strip_prefix("ANSWER:").or_else(|| last_line.strip_prefix("Answer:"))
                    .unwrap_or(last_line)
                    .trim();

                let lower = answer.to_lowercase();
                if lower.contains("none") || lower.is_empty() {
                    return Action::DeclareBlockers { assignments: vec![] };
                }

                let mut assignments = Vec::new();
                for pair in answer.split_whitespace() {
                    // Accept both "0:0" and "0->0" formats
                    let parts: Vec<&str> = if pair.contains("->") {
                        pair.split("->").collect()
                    } else {
                        pair.split(':').collect()
                    };
                    if parts.len() == 2 {
                        if let (Ok(b), Ok(a)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                            if b < eligible_blockers.len() && a < attackers.len() {
                                assignments.push((eligible_blockers[b], attackers[a]));
                            }
                        }
                    }
                }
                Action::DeclareBlockers { assignments }
            }
        }
    }
}
