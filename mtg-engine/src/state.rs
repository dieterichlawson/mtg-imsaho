use std::collections::{HashMap, HashSet};

use serde::{Serialize, Deserialize};

use crate::ids::{ObjectId, PlayerId, CardId};
use crate::types::{Zone, Step, ManaPool, ContinuousEffect};

/// An entry on the stack — a spell, triggered ability, or activated ability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StackEntry {
    /// A spell (instant, sorcery, creature, etc.) on the stack.
    Spell(ObjectId),
    /// A triggered ability on the stack. Resolves by calling the card behavior.
    Trigger(crate::triggers::PendingTrigger),
    /// An activated ability on the stack (CR 602.2a).
    Ability {
        source_id: ObjectId,
        ability_index: usize,
        behavior_card_id: CardId,
        targets: Vec<crate::actions::Target>,
        activator: PlayerId,
        x_value: Option<u32>,
    },
}

impl StackEntry {
    /// Get the `ObjectId` if this is a spell.
    #[must_use]
    pub fn as_spell(&self) -> Option<ObjectId> {
        match self {
            StackEntry::Spell(id) => Some(*id),
            StackEntry::Trigger(_) | StackEntry::Ability { .. } => None,
        }
    }

    /// Get the trigger if this is a triggered ability.
    #[must_use]
    pub fn as_trigger(&self) -> Option<&crate::triggers::PendingTrigger> {
        match self {
            StackEntry::Trigger(t) => Some(t),
            StackEntry::Spell(_) | StackEntry::Ability { .. } => None,
        }
    }

    /// Description for display purposes.
    #[must_use]
    pub fn display_name(&self, registry: &crate::cards::CardRegistry) -> String {
        match self {
            StackEntry::Spell(id) => format!("Spell({})", id.0),
            StackEntry::Trigger(t) => t.display_name(registry),
            StackEntry::Ability { behavior_card_id, .. } => {
                let name = registry.card_data(*behavior_card_id)
                    .map_or_else(|| "Unknown".into(), |d| d.name.clone());
                format!("{name} ability")
            }
        }
    }
}

/// A pending delayed end-of-combat exile trigger. Stored on `GameState` until
/// the end of combat step begins, at which point it is converted into a
/// `PendingTrigger::DelayedTokenExile` and placed on the stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndOfCombatExileEntry {
    pub target_id: ObjectId,
    /// The permanent whose ability created the delayed trigger. It may be gone
    /// by end of combat — the delayed ability fires anyway (CR 603.7d) — but
    /// the trigger still names it as its source on the stack.
    pub source_id: ObjectId,
    pub source_card_id: CardId,
    pub controller: PlayerId,
    pub description: String,
}

/// The complete, immutable game state. Clone to produce new states.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    /// All game objects keyed by their unique ID.
    pub objects: HashMap<ObjectId, GameObject>,
    /// Monotonic counter for generating unique `ObjectIds`.
    pub next_object_id: u64,

    /// Player states, indexed by PlayerId.0.
    pub players: Vec<PlayerState>,

    /// Turn structure.
    pub turn_number: u32,
    pub active_player: PlayerId,
    pub priority_player: Option<PlayerId>,
    pub step: Step,

    /// The stack. Last element is top of stack. Contains spells and triggered abilities.
    pub stack: Vec<StackEntry>,

    /// Combat state, present only during combat phase.
    pub combat: Option<CombatState>,

    /// Delayed end-of-combat exile triggers (CR 603.7) created by earlier effects,
    /// e.g. Geist of Saint Traft's "exile that token at end of combat". Drained
    /// into the stack when the end of combat step begins; fires independently of
    /// the source permanent's presence on the battlefield (CR 603.7d).
    pub end_of_combat_exiles: Vec<EndOfCombatExileEntry>,

    /// Whether the game is waiting for attackers/blockers declaration.
    pub awaiting_action: Option<AwaitingAction>,

    /// CR 510.5: when first/double strikers are in combat, the combat damage
    /// step happens twice. Set after the first-strike damage instance; tells
    /// `advance_step` to repeat `Step::CombatDamage` (with a full SBA /
    /// trigger / priority round in between) instead of moving to EndCombat.
    #[serde(default)]
    pub combat_damage_step_pending: bool,
    /// Which face the trigger currently resolving fired from, if any.
    ///
    /// A double-faced card's faces carry different abilities, so an
    /// intervening-if re-checked on resolution (CR 603.4) must test the
    /// condition of the face that triggered — the permanent may have flipped
    /// in between. Set by the trigger dispatcher around the card's hook and
    /// cleared afterwards; `None` outside a trigger resolution, which is how a
    /// trigger-time check knows to read the current face instead.
    #[serde(default, skip)]
    pub resolving_trigger_from_back_face: Option<bool>,

    /// The spell currently mid-resolution because it presented a player
    /// choice (`awaiting_action`). The ENGINE owns moving a resolved spell
    /// off the stack: `stack::resolve_spell` for spells that finish in one
    /// step, and `engine::finish_spell_resolution_if_idle` once the choice
    /// chain completes. Card code must never call `move_spell_after_resolve`
    /// from a pending-effect handler.
    #[serde(default)]
    pub resolving_spell: Option<ObjectId>,

    /// Game result, if the game is over.
    pub result: Option<GameResult>,

    /// Number of consecutive priority passes (resets on any non-pass action).
    pub consecutive_passes: u32,

    /// Whether this is the very first turn of the game (skip draw step).
    pub is_first_turn: bool,

    /// Events generated by the most recent action.
    pub events: Vec<crate::events::GameEvent>,

    /// Game log with levels. File gets everything, UI filters by level.
    pub game_log: Vec<LogEntry>,

    /// All temporary effects that expire at end of turn (cleanup step).
    #[serde(default)]
    pub until_end_of_turn: Vec<TemporaryEffect>,

    /// "Gain control of X for as long as ..." effects still in force.
    #[serde(default)]
    pub control_effects: Vec<ControlEffect>,

    /// Whether a creature has died this turn (for morbid).
    #[serde(default)]
    pub creature_died_this_turn: bool,

    /// Day/night state for werewolf transform tracking.
    #[serde(default)]
    pub day_night: Option<DayNight>,

    /// Number of spells cast this turn by each player (for werewolf transforms).
    #[serde(default)]
    pub num_spells_cast_this_turn: HashMap<PlayerId, u32>,

    /// Spells cast last turn (saved at turn start for werewolf condition checking).
    #[serde(default)]
    pub num_spells_cast_last_turn: HashMap<PlayerId, u32>,

    /// X value chosen for the most recently activated X-cost ability.
    /// Set by the engine before the ability goes on the stack; cards read this.
    #[serde(default)]
    pub last_activated_x_value: Option<u32>,

    /// Context stashed between the `ActivateAbility` handler and the follow-up
    /// `ChooseXFunding` resolution for X-cost activated abilities. Unlike
    /// spells (whose targets/flashback flag are persisted on the stack object
    /// itself), activated abilities don't use the stack in this engine, so we
    /// need a side channel to remember what to fire once the player has
    /// funded X.
    #[serde(default)]
    pub pending_ability_effect: Option<PendingAbilityEffect>,

    /// Context stashed between a `CastSpell` action and the follow-up
    /// `ChooseXFunding` resolution for X-cost spells. See [`PendingSpellCast`].
    /// Populated when the `CastSpell` handler sets up the prompt; consumed
    /// when the prompt resolves so the engine can execute the full casting
    /// sequence atomically. While this is `Some`, the spell is NOT on the
    /// stack — it remains in its origin zone until funding completes.
    #[serde(default)]
    pub pending_spell_cast: Option<PendingSpellCast>,

    /// Index for trigger processing resumption after a resolution choice.
    #[serde(default)]
    pub trigger_event_index: usize,

    /// Pending triggered abilities waiting to resolve, in APNAP order.
    /// Active player's triggers at the front (bottom of "stack"),
    /// non-active player's at the back (top). Resolved LIFO from the back.
    #[serde(default)]
    pub pending_triggers: Vec<crate::triggers::PendingTrigger>,

    /// CR 603.3d: triggers collected but not yet pushed onto the stack
    /// because they need target selection (or are queued behind one that does).
    /// AP triggers must all be pushed before NAP triggers; within each bucket,
    /// pop from the front and push onto `state.stack` after target choice.
    #[serde(default)]
    pub pending_trigger_pushes_ap: Vec<crate::triggers::PendingTrigger>,
    #[serde(default)]
    pub pending_trigger_pushes_nap: Vec<crate::triggers::PendingTrigger>,

    /// Queue of (player, `bottom_count`) pairs waiting for the London-mulligan
    /// bottoming sub-phase. Populated as each player finishes their keep/mull
    /// decision. Drained by `advance_mulligan_phase`.
    #[serde(default)]
    pub pending_mulligan_bottoms: Vec<(PlayerId, usize)>,

    /// Position within the current London-mulligan round, indexed from
    /// the active player. Within a round each non-kept player makes one
    /// keep/mull decision in turn order. When `mulligan_round_position`
    /// reaches the player count, the round ends.
    #[serde(default)]
    pub mulligan_round_position: u8,

    /// Whether any player has chosen `MulliganMull` in the current
    /// London-mulligan round. Used at end-of-round to decide whether to
    /// start a new round or transition to bottoming.
    #[serde(default)]
    pub mulligan_round_mulled: bool,
}

/// Log level for game log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    /// Every priority pass, mana tap, etc.
    Debug = 0,
    /// Actions taken: land plays, phase changes.
    Info = 1,
    /// Important game events: spells cast/resolved, combat, life changes.
    Event = 2,
    /// Major game moments: turn changes, player death, game over.
    Milestone = 3,
}

/// A single log entry with a level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
}

/// A temporary effect that expires during the cleanup step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemporaryEffect {
    /// Modify power and/or toughness (e.g., Giant Growth +3/+3).
    ModifyPT { target: ObjectId, power_mod: i32, toughness_mod: i32 },
    /// Grant a keyword (e.g., Moment of Heroism grants lifelink).
    GrantKeyword { target: ObjectId, keyword: crate::types::Keyword },
    /// Remove a keyword (e.g., Manor Gargoyle loses defender).
    RemoveKeyword { target: ObjectId, keyword: crate::types::Keyword },
    /// Creature can't block this turn (e.g., Nightbird's Clutches).
    CantBlock { target: ObjectId },
    /// Grant protection from creatures matching a filter (e.g., Spare from Evil).
    GrantProtection { target: ObjectId, filter: crate::types::CreatureFilter },
    /// Temporary control change; reverts at cleanup (e.g., Traitorous Blood).
    ChangeControl { target: ObjectId, original_controller: PlayerId },
    /// Grant flashback to a card in the graveyard (e.g., Snapcaster Mage).
    GrantFlashback { target: ObjectId, cost: crate::types::ManaCost },
    /// "Prevent all combat damage that would be dealt this turn by creatures
    /// other than <filter>." The filter names the creatures that still deal
    /// damage; everything else is prevented. Moonmist supplies Wolves and
    /// Werewolves — the engine does not know that, and shouldn't.
    PreventCombatDamageExcept { filter: crate::types::CreatureFilter },
    /// P/T modifier that disappears if source leaves the battlefield.
    /// Used by static abilities like Instigator Gang's "attacking creatures get +1/+0".
    ModifyPTWhileSourceInPlay {
        target: ObjectId,
        source: ObjectId,
        power_mod: i32,
        toughness_mod: i32,
    },
}

/// The single battlefield permanent an until-end-of-turn effect is attached
/// to, if any. Such effects end when that permanent leaves the battlefield
/// (CR 400.7 — the returning object is new and must not inherit them).
/// Controller-scoped, global, and graveyard-targeted effects return `None`.
///
/// Exhaustive by design: a new `TemporaryEffect` variant must be classified
/// here rather than silently defaulting.
fn until_eot_object_target(effect: &TemporaryEffect) -> Option<ObjectId> {
    match effect {
        TemporaryEffect::ModifyPT { target, .. }
        | TemporaryEffect::GrantKeyword { target, .. }
        | TemporaryEffect::RemoveKeyword { target, .. }
        | TemporaryEffect::CantBlock { target }
        | TemporaryEffect::GrantProtection { target, .. }
        | TemporaryEffect::ChangeControl { target, .. }
        | TemporaryEffect::ModifyPTWhileSourceInPlay { target, .. } => Some(*target),
        // Targets a card in the graveyard, not a battlefield permanent.
        TemporaryEffect::GrantFlashback { .. } => None,
        // Controller-scoped or global — not tied to one permanent.
        TemporaryEffect::PreventCombatDamageExcept { .. }
        => None,
    }
}

impl GameState {
    /// Create a new game state for a given number of players.
    #[must_use]
    pub fn new(num_players: u8) -> Self {
        let players = (0..num_players)
            .map(|i| PlayerState::new(PlayerId(i)))
            .collect();

        Self {
            objects: HashMap::new(),
            next_object_id: 1,
            players,
            turn_number: 1,
            active_player: PlayerId(0),
            priority_player: None,
            step: Step::Untap,
            stack: Vec::new(),
            combat: None,
            end_of_combat_exiles: Vec::new(),
            awaiting_action: None,
            combat_damage_step_pending: false,
            resolving_trigger_from_back_face: None,
            resolving_spell: None,
            result: None,
            consecutive_passes: 0,
            is_first_turn: true,
            events: Vec::new(),
            game_log: Vec::new(),
            until_end_of_turn: Vec::new(),
            control_effects: Vec::new(),
            creature_died_this_turn: false,
            day_night: None,
            num_spells_cast_this_turn: HashMap::new(),
            num_spells_cast_last_turn: HashMap::new(),
            last_activated_x_value: None,
            pending_ability_effect: None,
            pending_spell_cast: None,
            trigger_event_index: 0,
            pending_triggers: Vec::new(),
            pending_trigger_pushes_ap: Vec::new(),
            pending_trigger_pushes_nap: Vec::new(),
            pending_mulligan_bottoms: Vec::new(),
            mulligan_round_position: 0,
            mulligan_round_mulled: false,
        }
    }

    /// Allocate a fresh `ObjectId`.
    pub fn next_id(&mut self) -> ObjectId {
        let id = ObjectId(self.next_object_id);
        self.next_object_id += 1;
        id
    }

    /// Create a new game object and add it to the state.
    pub fn create_object(
        &mut self,
        card_id: CardId,
        owner: PlayerId,
        zone: Zone,
        power: Option<i32>,
        toughness: Option<i32>,
    ) -> ObjectId {
        let id = self.next_id();
        let obj = GameObject {
            id,
            card_id,
            name: String::new(), // Set by caller or setup_game
            owner,
            controller: owner,
            zone,
            tapped: false,
            summoning_sick: zone == Zone::Battlefield,
            damage_marked: 0,
            dealt_deathtouch_damage: false, damaged_by: Vec::new(),
            power,
            toughness,
            colors: Vec::new(),
            keywords: Vec::new(),
            card_types: Vec::new(),
            subtypes: Vec::new(),
            targets: Vec::new(),
            attached_to: None,
            attached_to_player: None,
            last_attached_to_player: None,
            zone_change_count: 0,
            copy_grantor: None,
            is_token: false,
            is_legendary: false,
            cast_with_flashback: false,
            instance_oracle_text: None,
            instance_continuous_effects: None,
            card_state: HashMap::new(),
            counters: HashMap::new(),
            regeneration_shields: 0,
            is_transformed: false,
            x_value: None,
            chosen_mode: None,
            abilities_activated_this_turn: std::collections::HashSet::new(),
            entering_copy_source: false,
            state_trigger_on_stack: false,
            attacked_on_turn: None,
            last_controller: None,
        };
        self.objects.insert(id, obj);
        id
    }

    /// Create a token on the battlefield. Returns all created IDs
    /// (primary + Parallel Lives extras).
    pub fn create_token(
        &mut self,
        name: &str,
        owner: PlayerId,
        power: i32,
        toughness: i32,
        colors: Vec<crate::types::Color>,
        card_types: Vec<crate::types::CardType>,
        keywords: Vec<crate::types::Keyword>,
        registry: &crate::cards::CardRegistry,
    ) -> Vec<ObjectId> {
        self.create_token_with_subtypes(name, owner, power, toughness, colors, card_types, keywords, vec![], registry)
    }

    /// Create a token on the battlefield with specific creature subtypes.
    /// If a permanent with `ReplacementEffect::DoubleTokens` is on the battlefield
    /// under the same controller, extra copies of the token are created.
    ///
    /// `name` is the name the *effect gives* the token, and is almost always
    /// empty: CR 111.4 says a token's name is its subtypes plus the word
    /// "Token" unless the effect names it, so a "1/1 white Spirit creature
    /// token" is named `Spirit Token`. No card in this set names a token, so
    /// the derived name is what every one of them gets.
    ///
    /// This used to be passed in by each card, and they disagreed — five cards
    /// make a 1/1 white flying Spirit and four of them called it `Spirit` while
    /// Moorland Haunt called it `Spirit Token`. Two cards in the set match
    /// creatures *by name* (Sever the Bloodline's "all other creatures with the
    /// same name", Evil Twin's granted ability), so the disagreement was
    /// reachable, not cosmetic.
    pub fn create_token_with_subtypes(
        &mut self,
        name: &str,
        owner: PlayerId,
        power: i32,
        toughness: i32,
        colors: Vec<crate::types::Color>,
        card_types: Vec<crate::types::CardType>,
        keywords: Vec<crate::types::Keyword>,
        subtypes: Vec<String>,
        registry: &crate::cards::CardRegistry,
    ) -> Vec<ObjectId> {
        // CR 111.4: the token's name is its subtypes plus "Token", unless the
        // effect gave it one.
        let derived;
        let name = if name.is_empty() && !subtypes.is_empty() {
            derived = format!("{} Token", subtypes.join(" "));
            derived.as_str()
        } else {
            name
        };

        // CR 614: a replacement effect may change how many tokens are created
        // (Parallel Lives). Two doublers compound, which falls out of running
        // the event through each in turn.
        let after = crate::replacement::apply(
            self,
            crate::replacement::ReplaceableEvent::CreatesTokens { controller: owner, count: 1 },
            registry,
        );
        let extra_copies = match after {
            Some(crate::replacement::ReplaceableEvent::CreatesTokens { count, .. }) =>
                count.saturating_sub(1),
            // Replaced entirely, or replaced with a different kind of event:
            // no tokens are created.
            _ => return Vec::new(),
        };

        let total = 1 + extra_copies as usize;
        let mut all_ids = Vec::with_capacity(total);

        // Create extra doubled copies first (cloning inputs).
        for _ in 0..extra_copies {
            let id = self.create_token_internal(name, owner, power, toughness,
                colors.clone(), card_types.clone(), keywords.clone(), subtypes.clone(), registry);
            all_ids.push(id);
        }
        // Create the final token, consuming the inputs.
        let id = self.create_token_internal(name, owner, power, toughness,
            colors, card_types, keywords, subtypes, registry);
        all_ids.push(id);

        all_ids
    }

    /// Internal token creation without Parallel Lives doubling.
    fn create_token_internal(
        &mut self,
        name: &str,
        owner: PlayerId,
        power: i32,
        toughness: i32,
        colors: Vec<crate::types::Color>,
        card_types: Vec<crate::types::CardType>,
        keywords: Vec<crate::types::Keyword>,
        subtypes: Vec<String>,
        registry: &crate::cards::CardRegistry,
    ) -> ObjectId {
        let id = self.next_id();
        let obj = GameObject {
            id,
            card_id: CardId(0), // sentinel for tokens
            name: name.to_string(),
            owner,
            controller: owner,
            zone: Zone::Battlefield,
            tapped: false,
            summoning_sick: true,
            damage_marked: 0,
            dealt_deathtouch_damage: false, damaged_by: Vec::new(),
            power: Some(power),
            toughness: Some(toughness),
            colors,
            keywords,
            card_types,
            subtypes,
            targets: Vec::new(),
            attached_to: None,
            attached_to_player: None,
            last_attached_to_player: None,
            zone_change_count: 0,
            copy_grantor: None,
            is_token: true,
            is_legendary: false,
            cast_with_flashback: false,
            instance_oracle_text: None,
            instance_continuous_effects: None,
            card_state: HashMap::new(),
            counters: HashMap::new(),
            regeneration_shields: 0,
            is_transformed: false,
            x_value: None,
            abilities_activated_this_turn: std::collections::HashSet::new(),
            chosen_mode: None,
            entering_copy_source: false,
            state_trigger_on_stack: false,
            attacked_on_turn: None,
            last_controller: None,
        };
        self.objects.insert(id, obj);
        // A token enters the battlefield like anything else, so the same
        // replacement effects apply (CR 614.1c/d).
        let entering = self.plan_entering(id, None, registry);
        self.apply_entering(&entering, registry);

        let controller = self.get_object(id).map_or(owner, |o| o.controller);
        self.events.push(crate::events::GameEvent::EnteredBattlefield {
            object: id,
            controller,
        });
        id
    }

    /// Create a token that copies all characteristics of a source permanent.
    /// Used by Cackling Counterpart, Evil Twin, etc.
    pub fn create_token_copy(
        &mut self,
        source_id: ObjectId,
        owner: PlayerId,
        registry: &crate::cards::CardRegistry,
    ) -> ObjectId {
        let source = self.get_object(source_id);
        let (name, power, toughness, card_id, is_legendary, obj_colors, obj_keywords, obj_card_types, obj_subtypes) = match source {
            Some(o) => (o.name.clone(), o.power, o.toughness, o.card_id, o.is_legendary, o.colors.clone(), o.keywords.clone(), o.card_types.clone(), o.subtypes.clone()),
            None => return ObjectId(0),
        };
        let (colors, keywords, card_types, subtypes) = registry.card_data(card_id)
            .map(|d| {
                // Derive colors from mana cost.
                let mut cols = Vec::new();
                if let Some(ref cost) = d.cost {
                    for sym in &cost.symbols {
                        if let crate::types::ManaSymbol::Colored(c) = sym {
                            if !cols.contains(c) {
                                cols.push(*c);
                            }
                        }
                    }
                }
                (cols, d.keywords.clone(), d.card_types.clone(), d.subtypes.clone())
            })
            .unwrap_or_else(|| (obj_colors, obj_keywords, obj_card_types, obj_subtypes));

        let all_ids = self.create_token_with_subtypes(
            &name,
            owner,
            power.unwrap_or(0),
            toughness.unwrap_or(0),
            colors,
            card_types,
            keywords,
            subtypes.clone(),
            registry,
        );
        // Copy the card_id and is_legendary so ALL tokens (including Parallel Lives extras)
        // get the same CardBehavior and are correctly flagged for the legend rule.
        for &token_id in &all_ids {
            if let Some(obj) = self.get_object_mut(token_id) {
                obj.card_id = card_id;
                obj.is_legendary = is_legendary;
            }
        }
        all_ids.into_iter().next().unwrap_or(ObjectId(0))
    }

    /// Move an object to a new zone.
    /// Per MTG rules, changing zones makes it a "new object" — we increment `zone_change_count`.
    /// Move a permanent onto the battlefield under a specified controller
    /// (CR 110.2 — reanimation and steal effects put it in under *their*
    /// controller, not its owner).
    ///
    /// The controller is set BEFORE the zone change, because `move_object`
    /// emits `EnteredBattlefield` during the move and the event carries the
    /// controller as it stands at that moment. Cards that moved first and
    /// assigned afterwards fixed the object but left the event — and every
    /// `AnyCreatureEnters` watcher reading it — with the previous controller.
    pub fn move_object_under_control(
        &mut self,
        id: ObjectId,
        to: Zone,
        controller: PlayerId,
        registry: &crate::cards::CardRegistry,
    ) {
        if let Some(obj) = self.get_object_mut(id) {
            obj.controller = controller;
        }
        self.move_object(id, to, registry);
    }

    pub fn move_object(&mut self, id: ObjectId, to: Zone, registry: &crate::cards::CardRegistry) {
        // Collect log info before mutating.
        let log_msg = self.objects.get(&id).and_then(|obj| {
            if obj.zone == Zone::Battlefield && to != Zone::Battlefield && obj.power.is_some() {
                let dest = match to {
                    Zone::Graveyard => "died",
                    Zone::Exile => "was exiled",
                    _ => "left the battlefield",
                };
                Some(format!("{} {}", obj.name, dest))
            } else {
                None
            }
        });

        if let Some(msg) = log_msg {
            self.log(LogLevel::Event, msg);
        }

        let from = self.objects.get(&id).map(|o| o.zone);
        // Capture controller before any mutation: CR 603.10c requires LTB
        // triggers to be controlled by whoever controlled the permanent
        // immediately before it left the battlefield.
        let pre_move_controller = self.objects.get(&id).map(|o| o.controller);
        // A mill emits `CreatureCardMilled`, and both facts have to be read
        // before the move: `is_creature` consults the battlefield-shaped
        // characteristics layer, and the library the card left is its owner's.
        let owner_before_move = self.objects.get(&id).map_or(PlayerId(0), |o| o.owner);
        let was_creature = self.is_creature(id, registry);

        // A tracked mid-resolution spell that leaves the stack (moved by a
        // pending-effect handler, or entering the battlefield as a
        // permanent) no longer needs engine cleanup.
        if self.resolving_spell == Some(id) && to != Zone::Stack {
            self.resolving_spell = None;
        }

        // CR 616.1: replacement effects are applied against the game state as
        // it was BEFORE the event, so the whole entering event is worked out
        // here rather than after the zone change. Unbreathing Horde entering
        // from the graveyard counts itself precisely because it is still in
        // the graveyard at this moment.
        let entering = if to == Zone::Battlefield && from.is_some_and(|z| z != Zone::Battlefield) {
            Some(self.plan_entering(id, from, registry))
        } else {
            None
        };

        // CR 400.7 / 712.8a: a permanent leaving the battlefield becomes a new
        // object printed as its front face. Its name and base P/T have to come
        // back from the registry, and that lookup needs `&self`, so it happens
        // before the mutable borrow below. For a copy, "printed" means the card
        // it was before the copy effect — which `copy_grantor` remembers.
        let printed_reset: Option<(String, Option<i32>, Option<i32>)> =
            if to != Zone::Battlefield && self.get_object(id).is_some_and(|o| !o.is_token) {
                self.get_object(id).and_then(|o| {
                    let printed_card = o.copy_grantor.unwrap_or(o.card_id);
                    registry.get(printed_card).map(|b| {
                        let d = b.card_data();
                        (d.name, d.power, d.toughness)
                    })
                })
            } else {
                None
            };

        if let Some(obj) = self.objects.get_mut(&id) {
            let from = obj.zone;
            obj.zone = to;
            obj.zone_change_count += 1;

            // Reset battlefield-specific state when leaving the battlefield.
            // Note: card_state is NOT cleared here — LTB triggers need it
            // (e.g., Fiend Hunter's "exiled_creature"). It's cleared on re-entry.
            if from == Zone::Battlefield && to != Zone::Battlefield {
                // CR 108.4: a card has a controller only while it represents a
                // permanent or a spell. Off the battlefield its owner acts as
                // its controller, so a stolen creature that dies stops being
                // the thief's — Boneyard Wurm reads its own controller to pick
                // whose graveyard to count, and a card whose controller was
                // never reset counted the wrong one. Last known information is
                // already captured above in `pre_move_controller`.
                obj.last_controller = Some(obj.controller);
                obj.controller = obj.owner;
                obj.tapped = false;
                obj.summoning_sick = false;
                // CR 400.7: what comes back is a new object, and a new object
                // has not attacked — even in the same turn.
                obj.attacked_on_turn = None;
                obj.damage_marked = 0;
                obj.dealt_deathtouch_damage = false; obj.damaged_by.clear();
                if let Some(att) = obj.attached_to {
                    obj.card_state.insert("last_attached_to".into(), att);
                }
                obj.attached_to = None;
                // The player-attachment counterpart, for Curses. Without this
                // a Curse kept `attached_to_player` in the graveyard, so any
                // effect returning it to the battlefield would re-attach it to
                // that player with no targeting and no consent. The value is
                // kept alongside as last known information — a Curse's upkeep
                // trigger already on the stack still knows whom it cursed.
                obj.last_attached_to_player = obj.attached_to_player;
                obj.attached_to_player = None;
                obj.counters.clear();
                obj.regeneration_shields = 0;
                // CR 400.7: a permanent that changes zones becomes a new object
                // with no memory of what happened to the old one. For a
                // non-token card the printed characteristics live in the
                // registry, so these object-level vectors hold only what an
                // effect granted at runtime — Olivia Voldaren's "Vampire",
                // Grimoire of the Dead's "Zombie" and black. Those must not
                // follow the card into the graveyard and back onto the
                // battlefield. Tokens are the exception: their object-level
                // fields ARE their printed characteristics.
                //
                // This runs before the CR 712.8a revert below, which writes the
                // front face onto a transformed DFC's object and would
                // otherwise be wiped by the clear.
                // `card_types` is deliberately left alone: nothing in the set
                // grants a card type at runtime (only the copy-effect and
                // token paths write it), so there is no stale grant to drop.
                if !obj.is_token {
                    obj.subtypes.clear();
                    obj.colors.clear();
                }
                // A copy stops being a copy: `card_id` is what makes it one, so
                // restore the card it is printed as. Otherwise an Evil Twin
                // that died carried the copied creature's identity into the
                // graveyard, and reanimating it ran that creature's ETB
                // handler instead of its own — it came back as a permanent
                // copy that could never offer its own choice again.
                if let Some(printed) = obj.copy_grantor.take() {
                    obj.card_id = printed;
                }
                // CR 712.8a: a DFC off the battlefield has only its front
                // face. Clearing `is_transformed` does most of it — every
                // characteristics accessor resolves through `face_data`, which
                // reads that flag. What has no registry lookup behind it is
                // `name` and the base P/T, so those are written back from the
                // printed card: the Tree of Redemption's toughness exchange
                // otherwise followed it into the graveyard and came back with
                // it, and a copy kept the copied creature's name.
                obj.is_transformed = false;
                if let Some((name, power, toughness)) = printed_reset {
                    obj.name = name;
                    obj.power = power;
                    obj.toughness = toughness;
                }
                obj.instance_continuous_effects = None;
                obj.instance_oracle_text = None;
            }

            // Clear cast_with_flashback when moving back to hand or library
            // (e.g. Runic Repetition returns an exiled flashback card to hand).
            // The flag is set during cast and would otherwise persist.
            if matches!(to, Zone::Hand | Zone::Library) {
                obj.cast_with_flashback = false;
            }

            // Set summoning sickness and clear stale state when entering the battlefield.
            if to == Zone::Battlefield && from != Zone::Battlefield {
                obj.card_state.clear();
                obj.last_attached_to_player = None;
                obj.summoning_sick = true;
            }
        }

        // CR 614.1d: a permanent that "enters as a copy" via a player choice
        // (Evil Twin) resolves that choice through an ETB trigger, so it
        // briefly exists as its printed 0/0 before the copy applies. Arm the
        // SBA copy-guard AT ENTRY — the single moment before any SBA runs —
        // so the 0/0 isn't destroyed in the window before the trigger
        // resolves. The guard is a transient flag cleared when the copy
        // decision concludes (see the CopyCreature handler and the copy-choice
        // resolution path); SBA consults only that flag, never a static
        // card property.
        if to == Zone::Battlefield && from != Some(Zone::Battlefield)
            && registry.get(self.objects.get(&id).map_or(CardId(0), |o| o.card_id))
                .is_some_and(super::cards::CardBehavior::enters_with_pending_copy_choice)
        {
            if let Some(obj) = self.objects.get_mut(&id) {
                obj.entering_copy_source = true;
            }
        }

        // CR 614.12: "As [this] enters, choose ..." happens as it enters, so
        // the choice is made here rather than from a trigger on the stack.
        // Nevermore used to declare an `EntersBattlefield` trigger, which left
        // it on the battlefield with no name chosen while the choice sat on
        // the stack — long enough for an opponent to cast the very card it was
        // about to name.
        if to == Zone::Battlefield && from != Some(Zone::Battlefield) {
            let card_id = self.objects.get(&id).map_or(CardId(0), |o| o.card_id);
            if registry.get(card_id).is_some_and(super::cards::CardBehavior::chooses_as_it_enters) {
                if let Some(behavior) = registry.get(card_id) {
                    behavior.on_enter_battlefield(self, id, &[], registry);
                }
            }
        }

        // CR 400.7: when a permanent leaves the battlefield it becomes a new
        // object. End any until-end-of-turn effect attached to it, so a
        // same-turn return reusing this ObjectId is a clean object rather than
        // inheriting stale buffs/grants/control changes.
        if from == Some(Zone::Battlefield) && to != Zone::Battlefield {
            self.until_end_of_turn.retain(|e| until_eot_object_target(e) != Some(id));
        }

        // Emit zone-change events outside the mutable borrow.
        if let Some(from_zone) = from {
            if from_zone == Zone::Battlefield && to != Zone::Battlefield {
                self.events.push(crate::events::GameEvent::LeftBattlefield {
                    object: id,
                    to,
                    last_controller: pre_move_controller.unwrap_or(PlayerId(0)),
                });
            }
            // Library to graveyard is a mill (CR 701.13a), and being one is a
            // property of the zone change, not of the caller having remembered
            // a helper. It used to be emitted by `engine::mill_one` alone, and
            // four cards moved library cards to the graveyard by hand —
            // Trepanation Blade milling the *defending* player, which is
            // exactly whose graveyard Undead Alchemist watches ("whenever a
            // creature card is put into an opponent's graveyard from their
            // library"). Whether a watcher cares is the collector's decision.
            if from_zone == Zone::Library && to == Zone::Graveyard && was_creature {
                self.events.push(crate::events::GameEvent::CreatureCardMilled {
                    object: id,
                    milled_player: owner_before_move,
                });
            }
            if to == Zone::Battlefield && from_zone != Zone::Battlefield {
                // Worked out before the move; applied now, before
                // EnteredBattlefield is emitted, so nothing observes a window
                // in which the permanent is untapped or missing its counters.
                if let Some(entering) = entering {
                    self.apply_entering(&entering, registry);
                }

                let controller = self.get_object(id).map_or(PlayerId(0), |o| o.controller);
                self.events.push(crate::events::GameEvent::EnteredBattlefield {
                    object: id,
                    controller,
                });
            }
        }
    }




    /// Work out how a permanent will enter the battlefield, after every
    /// applicable replacement effect (CR 614).
    ///
    /// Separate from applying it because CR 616.1 evaluates replacements
    /// against the game state *before* the event — for something changing
    /// zones, that means before the move. Unbreathing Horde entering from the
    /// graveyard counts itself for exactly this reason.
    fn plan_entering(
        &mut self,
        id: ObjectId,
        from: Option<Zone>,
        registry: &crate::cards::CardRegistry,
    ) -> crate::replacement::EnteringPermanent {
        let controller = self.get_object(id).map_or(PlayerId(0), |o| o.controller);
        crate::replacement::for_entering(
            self,
            crate::replacement::EnteringPermanent {
                object: id,
                from,
                controller,
                tapped: false,
                counters: Vec::new(),
                copy_of: None,
            },
            registry,
        )
    }

    /// Apply a planned entering event: become a copy, gain counters, arrive
    /// tapped. Runs once the object is on the battlefield but before
    /// `EnteredBattlefield` is emitted, so nothing observes a window in which
    /// the permanent is untapped or missing its counters.
    fn apply_entering(
        &mut self,
        entering: &crate::replacement::EnteringPermanent,
        registry: &crate::cards::CardRegistry,
    ) {
        let id = entering.object;
        if let Some(card_id) = entering.copy_of {
            self.become_copy_of(id, card_id, registry);
        }
        for (counter_type, count) in &entering.counters {
            self.add_counters(id, *counter_type, *count);
        }
        if entering.tapped {
            if let Some(obj) = self.get_object_mut(id) {
                obj.tapped = true;
            }
            let name = self.obj_name(id);
            self.log(LogLevel::Info, format!("{name} enters tapped"));
        }
    }

    /// Give `entering_id` the copiable values of `card_id` (CR 706.2).
    fn become_copy_of(
        &mut self,
        entering_id: ObjectId,
        card_id: crate::ids::CardId,
        registry: &crate::cards::CardRegistry,
    ) {
        // Get copiable values from card data (the authoritative source for characteristics).
        let source_data = Some(card_id).and_then(|card_id| {
            registry.card_data(card_id).map(|d| {
                (
                    d.name.clone(),
                    d.power.unwrap_or(0),
                    d.toughness.unwrap_or(0),
                    // Derive colors from mana cost.
                    d.cost.as_ref().map(|c| {
                        let mut cols = Vec::new();
                        for sym in &c.symbols {
                            if let crate::types::ManaSymbol::Colored(c) = sym {
                                if !cols.contains(c) { cols.push(*c); }
                            }
                        }
                        cols
                    }).unwrap_or_default(),
                    d.card_types.clone(),
                    d.subtypes.clone(),
                    d.keywords.clone(),
                    d.oracle_text.clone(),
                )
            })
        });

        if let Some((name, power, toughness, colors, card_types, subtypes, keywords, oracle_text)) = source_data {
            let old_name = self.get_object(entering_id).map(|o| o.name.clone()).unwrap_or_default();
            if let Some(obj) = self.get_object_mut(entering_id) {
                obj.name.clone_from(&name);
                obj.power = Some(power);
                obj.toughness = Some(toughness);
                obj.colors = colors;
                obj.card_types = card_types;
                obj.subtypes = subtypes;
                obj.keywords = keywords;
                obj.instance_continuous_effects = Some(vec![]);
                obj.instance_oracle_text = Some(oracle_text);
            }
            self.log(LogLevel::Event,
                format!("{old_name} enters as a copy of {name} ({power}/{toughness})"));
        }
    }

    /// Get an object by ID.
    #[must_use]
    pub fn get_object(&self, id: ObjectId) -> Option<&GameObject> {
        self.objects.get(&id)
    }

    /// Return "`CardName` (#id)" for use in log messages.
    #[must_use]
    pub fn obj_name(&self, id: ObjectId) -> String {
        let name = self.get_object(id).map_or_else(|| "?".into(), |o| o.name.clone());
        format!("{} (#{})", name, id.0)
    }

    /// Get a mutable reference to an object by ID.
    pub fn get_object_mut(&mut self, id: ObjectId) -> Option<&mut GameObject> {
        self.objects.get_mut(&id)
    }

    /// Get a player by ID.
    #[must_use]
    pub fn get_player(&self, id: PlayerId) -> &PlayerState {
        &self.players[id.0 as usize]
    }

    /// Get a mutable reference to a player by ID.
    pub fn get_player_mut(&mut self, id: PlayerId) -> &mut PlayerState {
        &mut self.players[id.0 as usize]
    }

    /// Get all objects in a zone owned/controlled by a player.
    /// For Library/Hand/Graveyard: filter by owner (per rule 400.3).
    /// For Battlefield: filter by controller.
    #[must_use]
    pub fn objects_in_zone(&self, zone: Zone, player: PlayerId) -> Vec<&GameObject> {
        let mut result: Vec<_> = self.objects.values().filter(|obj| {
            obj.zone == zone && match zone {
                Zone::Library | Zone::Hand | Zone::Graveyard => obj.owner == player,
                Zone::Battlefield => obj.controller == player,
                _ => true,
            }
        }).collect();
        result.sort_by_key(|o| o.id);
        result
    }

    /// Get all objects in a zone (regardless of player).
    #[must_use]
    pub fn all_objects_in_zone(&self, zone: Zone) -> Vec<&GameObject> {
        let mut result: Vec<_> = self.objects.values().filter(|obj| obj.zone == zone).collect();
        result.sort_by_key(|o| o.id);
        result
    }

    /// Get the next player after the given player (turn order).
    #[must_use]
    pub fn next_player(&self, player: PlayerId) -> PlayerId {
        let next = (player.0 + 1) % u8::try_from(self.players.len()).unwrap_or(u8::MAX);
        PlayerId(next)
    }

    /// Get the opponent in a 2-player game.
    #[must_use]
    pub fn opponent(&self, player: PlayerId) -> PlayerId {
        self.next_player(player)
    }

    /// Number of alive players.
    #[must_use]
    pub fn alive_player_count(&self) -> usize {
        self.players.iter().filter(|p| !p.lost).count()
    }

    /// Add a log entry.
    pub fn log(&mut self, level: LogLevel, msg: String) {
        self.game_log.push(LogEntry { level, message: msg });
    }

    /// Check if a creature matches a `CreatureFilter`, evaluated from the perspective
    /// of the effect's source permanent.
    #[must_use]
    pub fn matches_filter(
        &self,
        creature_id: ObjectId,
        filter: &crate::types::CreatureFilter,
        source_controller: PlayerId,
        registry: &crate::cards::CardRegistry,
    ) -> bool {
        use crate::types::CreatureFilter;
        let Some(creature) = self.get_object(creature_id) else { return false; };
        match filter {
            CreatureFilter::ControlledByYou => creature.controller == source_controller,
            CreatureFilter::ControlledByOpponent => creature.controller != source_controller,
            CreatureFilter::ControlledByYouToken => creature.controller == source_controller && creature.is_token,
            CreatureFilter::HasSubtype(subtype) => self.has_subtype(creature_id, subtype, registry),
            CreatureFilter::HasCardType(ct) => self.has_card_type(creature_id, *ct, registry),
            CreatureFilter::HasKeyword(kw) => self.has_keyword(creature_id, *kw, registry),
            CreatureFilter::And(filters) => filters.iter().all(|f| self.matches_filter(creature_id, f, source_controller, registry)),
            CreatureFilter::Or(filters) => filters.iter().any(|f| self.matches_filter(creature_id, f, source_controller, registry)),
            CreatureFilter::Not(inner) => !self.matches_filter(creature_id, inner, source_controller, registry),
            CreatureFilter::Attacking => {
                self.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&creature_id))
            }
            CreatureFilter::Blocking => {
                self.combat.as_ref().is_some_and(|c|
                    c.blocker_assignments.values().any(|bs| bs.contains(&creature_id)))
            }
            CreatureFilter::ControlledByAttachedPlayer => {
                // This filter requires knowing the source object's attached_to_player.
                // It's resolved in effect_applies_to which has source_id.
                // If called directly from matches_filter without source context,
                // fall back to Opponents (the common case for curses).
                creature.controller != source_controller
            }
        }
    }

    /// Check if a continuous effect applies to a given creature.
    #[must_use]
    fn effect_applies_to(
        &self,
        creature_id: ObjectId,
        scope: &crate::types::EffectScope,
        source_id: ObjectId,
        source_controller: PlayerId,
        registry: &crate::cards::CardRegistry,
    ) -> bool {
        use crate::types::EffectScope;
        match scope {
            EffectScope::OnSelf => creature_id == source_id,
            EffectScope::Attached => {
                self.get_object(source_id)
                    .and_then(|o| o.attached_to)
                    .is_some_and(|target| target == creature_id)
            }
            EffectScope::Global(filter) => {
                // For AttachedPlayer filter, use the source's attached_to_player.
                if matches!(filter, crate::types::CreatureFilter::ControlledByAttachedPlayer) {
                    let attached_player = self.get_object(source_id)
                        .and_then(|o| o.attached_to_player);
                    if let Some(player) = attached_player {
                        let creature = self.get_object(creature_id);
                        return creature.is_some_and(|c| c.controller == player);
                    }
                    return false;
                }
                self.matches_filter(creature_id, filter, source_controller, registry)
            }
            EffectScope::GlobalOther(filter) => {
                creature_id != source_id && self.matches_filter(creature_id, filter, source_controller, registry)
            }
        }
    }

    /// Total (`power_mod`, `toughness_mod`) applying to a creature.
    ///
    /// The static half is the one walk; the dynamic half is auras whose bonus
    /// is computed from the board (Wreath of Geists' "+X/+X where X is the
    /// number of creature cards in your graveyard"), which no enum variant can
    /// express, so the aura's behavior is asked directly.
    fn continuous_pt_mods(&self, creature_id: ObjectId, registry: &crate::cards::CardRegistry) -> (i32, i32) {
        let (mut power, mut toughness) = (0, 0);
        self.walk_effects(
            creature_id,
            &|e| matches!(e, ContinuousEffect::ModifyPT { .. }),
            registry,
            &mut |e, _| {
                if let ContinuousEffect::ModifyPT { power: p, toughness: t, .. } = e {
                    power += p;
                    toughness += t;
                }
                true
            },
        );
        for source in self.objects.values() {
            if source.zone != Zone::Battlefield || source.attached_to != Some(creature_id) {
                continue;
            }
            if let Some(behavior) = registry.get(source.card_id) {
                if let Some((p, t)) = behavior.dynamic_pt(self, source.id, registry) {
                    power += p;
                    toughness += t;
                }
            }
        }
        (power, toughness)
    }

    /// Visit every continuous effect that applies to `id` and satisfies
    /// `want`, from every source on the battlefield.
    ///
    /// The one walk. `has_effect`, `count_effect` and the P/T accumulation are
    /// all this function; before, each was its own loop over `self.objects`,
    /// as were the two that handled conditional effects, and the caller had to
    /// dig the `EffectScope` out of the variant itself and hand it back — the
    /// same six-line closure written eleven times.
    ///
    /// `want` is tested against the *unwrapped* effect and before the
    /// condition, so `When { SelfHasKeyword(..), .. }` is only evaluated by a
    /// query that actually wants what it wraps. That matters: evaluating
    /// conditions eagerly would send `has_keyword` back through itself.
    ///
    /// `visit` receives the effect and the permanent providing it — several
    /// effects are relative to their source's controller — and returns false
    /// to stop the walk.
    pub(crate) fn walk_effects(
        &self,
        id: ObjectId,
        want: &dyn Fn(&crate::types::ContinuousEffect) -> bool,
        registry: &crate::cards::CardRegistry,
        visit: &mut dyn FnMut(&crate::types::ContinuousEffect, &GameObject) -> bool,
    ) {
        for source in self.objects.values() {
            if source.zone != Zone::Battlefield {
                continue;
            }
            for effect in self.continuous_effects_of(source.id, registry) {
                let (inner, condition) = effect.unwrap_condition();
                if !want(inner) {
                    continue;
                }
                let Some(scope) = inner.scope() else { continue };
                if !self.effect_applies_to(id, scope, source.id, source.controller, registry) {
                    continue;
                }
                if let Some(c) = condition {
                    if !self.check_condition(c, source.id, source.controller, registry) {
                        continue;
                    }
                }
                if !visit(inner, source) {
                    return;
                }
            }
        }
    }

    /// Whether any continuous effect matching `want` applies to `id`.
    ///
    /// ```ignore
    /// state.has_effect(id, &|e| matches!(e, ContinuousEffect::PreventAttack { .. }), registry)
    /// ```
    #[must_use]
    pub fn has_effect(
        &self,
        id: ObjectId,
        want: &dyn Fn(&crate::types::ContinuousEffect) -> bool,
        registry: &crate::cards::CardRegistry,
    ) -> bool {
        let mut found = false;
        self.walk_effects(id, want, registry, &mut |_, _| { found = true; false });
        found
    }

    /// How many sources apply a matching continuous effect to `id`.
    #[must_use]
    pub fn count_effect(
        &self,
        id: ObjectId,
        want: &dyn Fn(&crate::types::ContinuousEffect) -> bool,
        registry: &crate::cards::CardRegistry,
    ) -> u32 {
        let mut count = 0;
        self.walk_effects(id, want, registry, &mut |_, _| { count += 1; true });
        count
    }

    /// Continuous effects provided by permanents `player` controls, with
    /// `When` conditions evaluated.
    ///
    /// Cost effects are keyed to who is casting, not to an `EffectScope`, so
    /// they cannot come through `walk_effects`. Reading them here still means
    /// reading them through `continuous_effects_of` — the two cost functions
    /// used to walk `card_data().continuous_effects` directly and so ignored
    /// a transformed permanent's back face and anything granted at runtime.
    #[must_use]
    pub fn effects_controlled_by(
        &self,
        player: crate::ids::PlayerId,
        registry: &crate::cards::CardRegistry,
    ) -> Vec<crate::types::ContinuousEffect> {
        let mut out = Vec::new();
        for source in self.objects.values() {
            if source.zone != Zone::Battlefield || source.controller != player {
                continue;
            }
            for effect in self.continuous_effects_of(source.id, registry) {
                let (inner, condition) = effect.unwrap_condition();
                if condition.is_some_and(|c| !self.check_condition(c, source.id, source.controller, registry)) {
                    continue;
                }
                out.push(inner.clone());
            }
        }
        out
    }

    /// Continuous effects that modify the rules of the game rather than a
    /// permanent — the ones with no `EffectScope`. Conditions are evaluated,
    /// so a `When`-wrapped rule modification only shows up while it holds.
    #[must_use]
    pub fn global_effects(&self, registry: &crate::cards::CardRegistry) -> Vec<crate::types::ContinuousEffect> {
        let mut out = Vec::new();
        for source in self.objects.values() {
            if source.zone != Zone::Battlefield {
                continue;
            }
            for effect in self.continuous_effects_of(source.id, registry) {
                let (inner, condition) = effect.unwrap_condition();
                if inner.scope().is_some() {
                    continue;
                }
                if condition.is_some_and(|c| !self.check_condition(c, source.id, source.controller, registry)) {
                    continue;
                }
                out.push(inner.clone());
            }
        }
        out
    }

    /// Get the effective power of a creature, including continuous effects,
    /// dynamic P/T, counters, and "until end of turn" effects.
    #[must_use]
    pub fn effective_power(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Option<i32> {
        let obj = self.get_object(id)?;

        // Check if this token has dynamic P/T tied to counter count on a source permanent
        // (e.g., Gutter Grime Ooze tokens whose P/T = slime counters on Gutter Grime).
        let mut power = if let Some(source_id) = obj.card_state.get("pt_source_counter") {
            let counter_type_val = obj.card_state.get("pt_source_counter_type")
                .map_or(0, |v| v.0);
            let counter_type = match counter_type_val {
                1 => crate::types::CounterType::Slime,
                _ => crate::types::CounterType::PlusOnePlusOne,
            };
            self.get_object(*source_id)
                .map_or(0, |src| i32::try_from(*src.counters.get(&counter_type).unwrap_or(&0)).unwrap_or(i32::MAX))
        } else if let Some(behavior) = registry.get(obj.card_id) {
            // Check if this creature's own card has dynamic P/T (e.g.,
            // Geist-Honored Monk). Only creatures (base P/T set — CDA
            // creatures use the Some(0) sentinel) consult their own
            // dynamic_pt: equipment/aura dynamic_pt contributes to the
            // attached creature, not to the source itself.
            if obj.power.is_some() {
                if let Some((p, _)) = behavior.dynamic_pt(self, id, registry) {
                    p
                } else if obj.is_transformed {
                    // CR 712.8: a transformed permanent has its back face's
                    // characteristics, P/T included. `obj.power` is the front
                    // face's, stamped at creation, so nineteen DFCs each
                    // carried a `dynamic_pt` that did nothing but repeat their
                    // own `back_face_data` — one derived fact written twice,
                    // in two places free to disagree.
                    behavior.back_face_data().and_then(|d| d.power).or(obj.power)?
                } else {
                    obj.power?
                }
            } else {
                obj.power?
            }
        } else {
            obj.power?
        };

        // Continuous effects (auras, anthems, debuffs — including dynamic aura P/T).
        let (p_mod, _) = self.continuous_pt_mods(id, registry);
        power += p_mod;

        // +1/+1 and -1/-1 counter bonuses.
        power += i32::try_from(*obj.counters.get(&crate::types::CounterType::PlusOnePlusOne).unwrap_or(&0)).unwrap_or(i32::MAX);
        power -= i32::try_from(*obj.counters.get(&crate::types::CounterType::MinusOneMinusOne).unwrap_or(&0)).unwrap_or(i32::MAX);

        // Until-end-of-turn effects.
        for effect in &self.until_end_of_turn {
            match effect {
                TemporaryEffect::ModifyPT { target, power_mod, .. } if *target == id => {
                    power += power_mod;
                }
                TemporaryEffect::ModifyPTWhileSourceInPlay { target, source, power_mod, .. } if *target == id => {
                    if self.get_object(*source).is_some_and(|o| o.zone == Zone::Battlefield) {
                        power += power_mod;
                    }
                }
                _ => {}
            }
        }

        Some(power)
    }

    /// Get the effective toughness of a creature.
    #[must_use]
    pub fn effective_toughness(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Option<i32> {
        let obj = self.get_object(id)?;

        // Check if this token has dynamic P/T tied to counter count on a source permanent.
        let mut toughness = if let Some(source_id) = obj.card_state.get("pt_source_counter") {
            let counter_type_val = obj.card_state.get("pt_source_counter_type")
                .map_or(0, |v| v.0);
            let counter_type = match counter_type_val {
                1 => crate::types::CounterType::Slime,
                _ => crate::types::CounterType::PlusOnePlusOne,
            };
            self.get_object(*source_id)
                .map_or(0, |src| i32::try_from(*src.counters.get(&counter_type).unwrap_or(&0)).unwrap_or(i32::MAX))
        } else if let Some(behavior) = registry.get(obj.card_id) {
            // Check if this creature's own card has dynamic P/T. Same
            // creature-only guard as effective_power — see comment there.
            if obj.toughness.is_some() {
                if let Some((_, t)) = behavior.dynamic_pt(self, id, registry) {
                    t
                } else if obj.is_transformed {
                    // The back face's printed toughness — see effective_power.
                    behavior.back_face_data().and_then(|d| d.toughness).or(obj.toughness)?
                } else {
                    obj.toughness?
                }
            } else {
                obj.toughness?
            }
        } else {
            obj.toughness?
        };

        let (_, t_mod) = self.continuous_pt_mods(id, registry);
        toughness += t_mod;

        // +1/+1 and -1/-1 counter bonuses.
        toughness += i32::try_from(*obj.counters.get(&crate::types::CounterType::PlusOnePlusOne).unwrap_or(&0)).unwrap_or(i32::MAX);
        toughness -= i32::try_from(*obj.counters.get(&crate::types::CounterType::MinusOneMinusOne).unwrap_or(&0)).unwrap_or(i32::MAX);

        for effect in &self.until_end_of_turn {
            match effect {
                TemporaryEffect::ModifyPT { target, toughness_mod, .. } if *target == id => {
                    toughness += toughness_mod;
                }
                TemporaryEffect::ModifyPTWhileSourceInPlay { target, source, toughness_mod, .. } if *target == id => {
                    if self.get_object(*source).is_some_and(|o| o.zone == Zone::Battlefield) {
                        toughness += toughness_mod;
                    }
                }
                _ => {}
            }
        }

        Some(toughness)
    }

    /// Check if a creature is prevented from attacking (e.g., by Pacifism).
    #[must_use]
    pub fn can_attack(&self, creature_id: ObjectId, registry: &crate::cards::CardRegistry) -> bool {
        // Conditional "can't attack" (Bonds of Faith on a non-Human) comes
        // through the same query — one walk, not two.
        !self.has_effect(creature_id, &|e| matches!(e, ContinuousEffect::PreventAttack { .. }), registry)
    }

    /// Check if a creature is prevented from blocking.
    #[must_use]
    pub fn can_block(&self, creature_id: ObjectId, registry: &crate::cards::CardRegistry) -> bool {
        !self.has_effect(creature_id, &|e| matches!(e, ContinuousEffect::PreventBlock { .. }), registry)
    }

    /// CR 508.1d: whether an effect requires this creature to attack if able
    /// (Curse of the Nightly Hunt, Furor of the Bitten).
    #[must_use]
    pub fn must_attack(&self, creature_id: ObjectId, registry: &crate::cards::CardRegistry) -> bool {
        self.has_effect(creature_id, &|e| matches!(e, ContinuousEffect::ForceAttack { .. }), registry)
    }

    /// CR 509.1b: whether an effect makes this creature unblockable outright
    /// (Invisible Stalker). A creature that can only be blocked by certain
    /// creatures is a different rule — see `can_block_attacker`.
    #[must_use]
    pub fn cant_be_blocked(&self, creature_id: ObjectId, registry: &crate::cards::CardRegistry) -> bool {
        self.has_effect(creature_id, &|e| matches!(e, ContinuousEffect::CantBeBlocked { .. }), registry)
    }

    /// CR 502.2: whether this permanent untaps during its controller's untap
    /// step. False while something holds it down (Claustrophobia).
    #[must_use]
    pub fn untaps_normally(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> bool {
        !self.has_effect(id, &|e| matches!(e, ContinuousEffect::PreventUntap { .. }), registry)
    }

    /// Check if a creature on the battlefield has a given keyword ability.
    /// Checks static card keywords, continuous effect grants, aura grants, and until-EOT grants.
    #[must_use]
    pub fn has_keyword(&self, creature_id: ObjectId, keyword: crate::types::Keyword, registry: &crate::cards::CardRegistry) -> bool {
        let obj = match self.get_object(creature_id) {
            Some(o) if o.zone == Zone::Battlefield => o,
            _ => return false,
        };

        // Check if this keyword was temporarily removed until end of turn.
        if self.until_end_of_turn.iter().any(|e| matches!(e,
            TemporaryEffect::RemoveKeyword { target, keyword: kw }
            if *target == creature_id && *kw == keyword
        )) {
            return false;
        }

        // 1. Static keywords from card definition (or back face if transformed).
        // For cards with a registry entry the registry is authoritative — this
        // avoids returning stale front-face keywords after a transform that did
        // not go through helpers::apply_transform.
        if let Some(behavior) = registry.get(obj.card_id) {
            if obj.is_transformed {
                if let Some(back) = behavior.back_face_data() {
                    if back.keywords.contains(&keyword) {
                        return true;
                    }
                }
            } else if behavior.card_data().keywords.contains(&keyword) {
                return true;
            }
        } else {
            // No registry entry (tokens, anonymous objects): `obj.keywords` is
            // where their printed keywords live.
            //
            // Deliberately NOT unioned in for a card that HAS a face, unlike
            // subtypes and colors. Those are granted at runtime by writing the
            // object vector (Olivia Voldaren's "Vampire", Grimoire of the
            // Dead's black), so they have to be unioned. Keywords have a real
            // effects layer instead — `ContinuousEffect::GrantKeyword` and
            // `TemporaryEffect`, handled below — and nothing grants one by
            // writing here. Unioning would resurrect a stale front-face
            // keyword on a transformed DFC.
            if obj.keywords.contains(&keyword) {
                return true;
            }
        }

        // 2. Keywords from continuous effects (auras with GrantKeyword, anthem keyword grants).
        // Conditional grants ("has lifelink as long as it's a Human") come
        // through here too — `has_effect` unwraps the condition. This used to
        // be a second, near-identical walk in `has_conditional_keyword`.
        if self.has_effect(creature_id,
            &|e| matches!(e, ContinuousEffect::GrantKeyword { keyword: kw, .. } if *kw == keyword),
            registry)
        {
            return true;
        }

        // 3. Temporary keyword grants (until end of turn).
        for effect in &self.until_end_of_turn {
            match effect {
                TemporaryEffect::GrantKeyword { target, keyword: kw } if *target == creature_id && *kw == keyword => {
                    return true;
                }
                _ => {}
            }
        }

        false
    }

    /// Check if a creature has protection from a given source.
    /// Returns true if the target has protection from the source's subtypes or matches
    /// a `ProtectionFrom` filter. Used for targeting, blocking, and damage prevention.
    #[must_use]
    pub fn has_protection_from(&self, target_id: ObjectId, source_id: ObjectId, registry: &crate::cards::CardRegistry) -> bool {
        use crate::types::ContinuousEffect;

        // Get the source's subtypes (active face — transform-aware).
        let source_subtypes: Vec<String> = self.subtypes_of(source_id, registry);

        // Check ProtectionFromSubtype effects on the target.
        let has_subtype_protection = self.has_effect(target_id,
            &|e| matches!(e, ContinuousEffect::ProtectionFromSubtype { subtype, .. }
                if source_subtypes.iter().any(|s| s == subtype)),
            registry);
        if has_subtype_protection {
            return true;
        }

        // Check filter-based static ProtectionFrom effects (e.g., protection
        // from a color or card type granted by a permanent). The filter is
        // read against the granting permanent's controller, so this needs the
        // source the walk found it on.
        let mut protected = false;
        self.walk_effects(
            target_id,
            &|e| matches!(e, ContinuousEffect::ProtectionFrom { .. }),
            registry,
            &mut |e, src_obj| {
                if let ContinuousEffect::ProtectionFrom { filter, .. } = e {
                    if self.matches_filter(source_id, filter, src_obj.controller, registry) {
                        protected = true;
                        return false;
                    }
                }
                true
            },
        );
        if protected {
            return true;
        }

        let target_controller = self.get_object(target_id)
            .map_or(crate::ids::PlayerId(0), |o| o.controller);

        // Check until-end-of-turn protection grants (e.g., Spare from Evil).
        for effect in &self.until_end_of_turn {
            match effect {
                TemporaryEffect::GrantProtection { target, filter } if *target == target_id => {
                    if self.matches_filter(source_id, filter, target_controller, registry) {
                        return true;
                    }
                }
                _ => {}
            }
        }

        false
    }

    /// Evaluate an `EffectCondition` for a given controller.
    fn check_condition(&self, condition: &crate::types::EffectCondition, source_id: ObjectId, controller: crate::ids::PlayerId, registry: &crate::cards::CardRegistry) -> bool {
        use crate::types::EffectCondition;
        match condition {
            EffectCondition::YouControlSubtype(subtype) => {
                self.objects.values()
                    .filter(|o| o.zone == Zone::Battlefield && o.controller == controller)
                    .any(|o| self.has_subtype(o.id, subtype, registry))
            }
            EffectCondition::OpponentControlsSubtype(subtype) => {
                let opponent = self.opponent(controller);
                self.objects.values()
                    .filter(|o| o.zone == Zone::Battlefield && o.controller == opponent)
                    .any(|o| self.has_subtype(o.id, subtype, registry))
            }
            EffectCondition::SelfHasKeyword(kw) => {
                let removed = self.until_end_of_turn.iter().any(|e| matches!(e,
                    TemporaryEffect::RemoveKeyword { target, keyword }
                    if *target == source_id && *keyword == *kw
                ));
                if removed {
                    return false;
                }
                self.has_keyword(source_id, *kw, registry)
            }
            EffectCondition::AttachedHasSubtype(subtype) => {
                // Subtypes are additive: `obj.subtypes` holds only what was
                // granted at runtime (Olivia's "Vampire", Grimoire's "Zombie"),
                // while the creature's printed types live on its active face.
                // Treating a non-empty `obj.subtypes` as the whole truth made a
                // Human that Olivia had turned into a Vampire stop counting as
                // a Human. `has_subtype` unions both, and reads the back face
                // for a transformed DFC.
                self.get_object(source_id)
                    .and_then(|o| o.attached_to)
                    .is_some_and(|target_id| self.has_subtype(target_id, subtype, registry))
            }
            EffectCondition::AttachedLacksSubtype(subtype) => {
                !self.check_condition(&EffectCondition::AttachedHasSubtype(subtype.clone()), source_id, controller, registry)
            }
        }
    }

    /// Change control of a battlefield permanent (CR 800.4a).
    ///
    /// The permanent becomes summoning-sick for the new controller: it hasn't
    /// been under their control continuously since their most recent turn began
    /// (CR 302.6 / 508.1a), so it can't attack or use tap/untap abilities until
    /// their next untap step — unless it has haste, which is checked at
    /// use-time (`eligible_attackers`, tap-ability legality), not here. Effects
    /// that grant haste alongside the steal (e.g. Act-of-Treason variants) thus
    /// still work. This is the single correct way to reassign controller for an
    /// in-play permanent; assigning `obj.controller` directly skips the
    /// summoning-sickness reset and is a bug.
    pub fn change_control(&mut self, id: ObjectId, new_controller: PlayerId) {
        if let Some(obj) = self.get_object_mut(id) {
            if obj.controller != new_controller {
                obj.controller = new_controller;
                obj.summoning_sick = true;
            }
        }
    }

    /// "Gain control of `object` for as long as `source`'s controller
    /// controls it" (CR 611.2b).
    ///
    /// Records the duration so the engine can end it — see `ControlEffect`.
    /// Card code must not keep its own list of what it stole: doing that meant
    /// the control effect only ended in the one way that card happened to
    /// check for.
    pub fn gain_control_while_source_controlled(
        &mut self,
        object: ObjectId,
        source: ObjectId,
        registry: &crate::cards::CardRegistry,
    ) {
        let Some(source_obj) = self.get_object(source) else { return };
        let source_controller = source_obj.controller;
        let Some(obj) = self.get_object(object) else { return };
        let original_controller = obj.controller;
        if original_controller == source_controller {
            return;
        }
        let _ = registry;
        self.change_control(object, source_controller);
        self.control_effects.push(ControlEffect {
            object,
            controller: source_controller,
            original_controller,
            source,
            source_controller,
        });
    }

    /// End every control effect whose condition has stopped being true, giving
    /// each permanent back to whoever had it (CR 611.2b). Returns true if
    /// anything changed.
    ///
    /// Run as a state-based action, which is the closest the engine has to
    /// "the moment the condition becomes false".
    pub fn expire_control_effects(&mut self) -> bool {
        let ended: Vec<ControlEffect> = self.control_effects.iter()
            .filter(|e| {
                // The source has to still be on the battlefield AND still be
                // controlled by the player who gained control.
                !self.get_object(e.source)
                    .is_some_and(|s| s.zone == Zone::Battlefield && s.controller == e.source_controller)
            })
            .cloned()
            .collect();
        if ended.is_empty() {
            return false;
        }
        self.control_effects.retain(|e| !ended.contains(e));
        for effect in ended {
            if self.get_object(effect.object).is_some_and(|o| o.zone == Zone::Battlefield) {
                let name = self.obj_name(effect.object);
                self.change_control(effect.object, effect.original_controller);
                self.log(LogLevel::Event, format!(
                    "{name} returns to p{}: the control effect's condition no longer holds",
                    effect.original_controller.0));
            }
        }
        true
    }

    /// Move a spell that has finished resolving to the appropriate zone.
    /// Flashback spells go to exile (CR 702.33a); others go to graveyard.
    ///
    /// Cleanup of a *resolving* spell is the engine's, not card code's — see
    /// [`GameState::resolving_spell`]. `crate`-visible so that stays true; a
    /// guard test in `test_suite_guards.rs` keeps card code from calling it.
    pub(crate) fn move_spell_after_resolve(&mut self, object_id: ObjectId, registry: &crate::cards::CardRegistry) {
        let exile = self.get_object(object_id)
            .is_some_and(|o| o.cast_with_flashback);
        if exile {
            self.move_object(object_id, Zone::Exile, registry);
        } else {
            self.move_object(object_id, Zone::Graveyard, registry);
        }
    }

    /// CR 701.5a: a countered spell is put into its owner's graveyard — or
    /// into exile if it was cast with flashback, the same zone rule a
    /// resolved spell follows.
    ///
    /// Countering moves a spell *other* than the one resolving, which is why
    /// this is public where [`GameState::move_spell_after_resolve`] is not:
    /// a counterspell disposing of its target is not a spell cleaning up
    /// after itself.
    pub fn move_countered_spell(&mut self, object_id: ObjectId, registry: &crate::cards::CardRegistry) {
        self.move_spell_after_resolve(object_id, registry);
    }

    /// Check if a player has hexproof (e.g., from Witchbane Orb).
    #[must_use]
    pub fn player_has_hexproof(&self, player: PlayerId, registry: &crate::cards::CardRegistry) -> bool {
        self.objects.values().any(|o| {
            o.zone == Zone::Battlefield
                && o.controller == player
                && registry.get(o.card_id)
                    .is_some_and(super::cards::CardBehavior::grants_player_hexproof)
        })
    }

    /// Whether a player has protection from `color`.
    #[must_use]
    pub fn player_has_protection_from(&self, player: PlayerId, color: crate::types::Color, registry: &crate::cards::CardRegistry) -> bool {
        self.objects.values().any(|o| {
            o.zone == Zone::Battlefield
                && o.controller == player
                && registry.get(o.card_id)
                    .is_some_and(|b| b.grants_player_protection_from().contains(&color))
        })
    }

    /// Whether `aura_id` can legally be attached to `player`.
    ///
    /// CR 702.16b: a player with protection from a color can't be enchanted by
    /// Auras of that color. CR 303.4h: an Aura that would enter the
    /// battlefield attached to something it can't legally enchant doesn't
    /// enter at all — so this is checked both when offering the choice and
    /// again when the attachment is actually made.
    #[must_use]
    pub fn player_can_be_enchanted_by(&self, aura_id: ObjectId, player: PlayerId, registry: &crate::cards::CardRegistry) -> bool {
        !self.colors_of(aura_id, registry).into_iter()
            .any(|c| self.player_has_protection_from(player, c, registry))
    }

    /// Add counters to a permanent.
    pub fn add_counters(&mut self, id: ObjectId, counter_type: crate::types::CounterType, count: u32) {
        // CR 121.1: counters go on permanents. A permanent that has left the
        // battlefield is a different object, so a counter aimed at it lands
        // nowhere — an ability that resolves after its source was destroyed
        // simply fails to put the counter on.
        //
        // Without this, Gutter Grime destroyed in response to its own trigger
        // still gained a slime counter in the graveyard, and since P/T reads
        // counters regardless of zone, the Ooze it made came in 1/1 instead of
        // the 0/0 the ruling requires. The counter then rode along if the
        // Grime was ever reanimated.
        if self.objects.get(&id).is_none_or(|o| o.zone != Zone::Battlefield) {
            return;
        }
        if let Some(obj) = self.objects.get_mut(&id) {
            *obj.counters.entry(counter_type).or_insert(0) += count;
        }
    }

    /// Remove up to `count` counters of a type from a permanent.
    ///
    /// Removal, unlike `add_counters`, is not restricted to the battlefield:
    /// counters can be removed as a cost from a permanent that is being
    /// sacrificed in the same cost, and the removal has to happen before the
    /// zone change clears them all (CR 601.2h).
    pub fn remove_counters(&mut self, id: ObjectId, counter_type: crate::types::CounterType, count: u32) {
        if let Some(obj) = self.objects.get_mut(&id) {
            if let Some(current) = obj.counters.get_mut(&counter_type) {
                *current = current.saturating_sub(count);
                if *current == 0 {
                    obj.counters.remove(&counter_type);
                }
            }
        }
    }

    /// Get the number of counters of a type on a permanent.
    #[must_use]
    pub fn get_counter_count(&self, id: ObjectId, counter_type: crate::types::CounterType) -> u32 {
        self.get_object(id)
            .and_then(|o| o.counters.get(&counter_type))
            .copied()
            .unwrap_or(0)
    }

    /// Is the game over?
    #[must_use]
    pub fn is_game_over(&self) -> bool {
        self.result.is_some()
    }

    /// Change a player's life total and emit the `LifeChanged` event.
    ///
    /// Every caller used to hand-roll this — read `life`, write `life`, push
    /// the event — which meant a site that forgot the event silently broke any
    /// "whenever you gain life" watcher. `delta` is signed: negative loses.
    pub fn change_life(&mut self, player: crate::ids::PlayerId, delta: i32) {
        if delta == 0 {
            return;
        }
        let old = self.get_player(player).life;
        let new_life = old + delta;
        self.get_player_mut(player).life = new_life;
        self.events.push(crate::events::GameEvent::LifeChanged { player, old, new_life });
    }

    /// `change_life` with a positive amount (CR 118.3).
    pub fn gain_life(&mut self, player: crate::ids::PlayerId, amount: i32) {
        self.change_life(player, amount);
    }

    /// `change_life` with a negative amount. Note this is life LOSS, which is
    /// not damage — it bypasses protection, prevention and damage triggers.
    pub fn lose_life(&mut self, player: crate::ids::PlayerId, amount: i32) {
        self.change_life(player, -amount);
    }

    /// Discard a card: move it to its owner's graveyard and announce it
    /// (CR 701.8a).
    ///
    /// The move and the `Discarded` event were written out side by side at
    /// thirteen call sites; a site that forgot the event silently broke every
    /// discard watcher (Murder of Crows, Civilized Scholar's transform).
    pub fn discard_card(&mut self, id: ObjectId, registry: &crate::cards::CardRegistry) {
        let Some(player) = self.get_object(id).map(|o| o.owner) else { return; };
        self.move_object(id, Zone::Graveyard, registry);
        self.events.push(crate::events::GameEvent::Discarded { player, object: id });
    }

    // ===== Characteristics layer =====
    //
    // THE RULE: an object's characteristics are
    //
    //     printed (its active face)  UNION  granted (its object-level fields)
    //
    // `face_data` is the printed half: the back face when a DFC is
    // transformed, the front face otherwise. The object-level vectors
    // (`card_types`, `subtypes`, `colors`, `keywords`) are the granted half —
    // what an effect added at runtime, like Olivia Voldaren's "Vampire" or
    // Grimoire of the Dead's "Zombie". Tokens are the one exception: they have
    // no registry face, so their object-level fields carry their printed
    // characteristics instead.
    //
    // Union, never override, and never duplicate the face onto the object.
    // Both of those went wrong here before: `card_types_of` and `colors_of`
    // used to return the object's vector *instead of* the face's whenever it
    // was non-empty, while `subtypes_of` unioned — so the same question got two
    // different answers depending on which field you asked about. And
    // `setup_game` used to copy every card's data onto its object while
    // `create_object` left it empty, so a card's raw fields were populated in a
    // real game and empty in a test, and code reading them directly appeared to
    // work while silently doing nothing under test.
    //
    // ALWAYS go through these accessors. Reading `obj.card_types` /
    // `obj.subtypes` / `obj.colors` / `obj.keywords` / `obj.name` directly, or
    // calling `registry.card_data` (which is always the FRONT face), is a bug —
    // `characteristics_invariant.rs` fails the build if card code does it.

    /// The `CardData` of the object's active face: the back face for a
    /// transformed DFC, the front face otherwise. `None` for objects with
    /// no registry entry (anonymous test objects).
    #[must_use]
    pub fn face_data(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Option<crate::cards::CardData> {
        let obj = self.get_object(id)?;
        let behavior = registry.get(obj.card_id)?;
        if obj.is_transformed {
            if let Some(back) = behavior.back_face_data() {
                return Some(back);
            }
        }
        Some(behavior.card_data())
    }

    /// The object's name, from its active face — the back face when a DFC is
    /// transformed. `obj.name` is only authoritative for tokens, which have no
    /// registry face; for a real card it is a display cache that goes stale
    /// (CR 712.8a: a DFC outside the battlefield has its front face's name).
    #[must_use]
    pub fn name_of(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> String {
        if let Some(data) = self.face_data(id, registry) {
            return data.name;
        }
        self.get_object(id).map(|o| o.name.clone()).unwrap_or_default()
    }

    /// Card types of the object: the union of the active face's types and any
    /// granted at runtime. Union, not override — see the module rule above.
    #[must_use]
    pub fn card_types_of(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Vec<crate::types::CardType> {
        let mut types = self.get_object(id).map(|o| o.card_types.clone()).unwrap_or_default();
        if let Some(data) = self.face_data(id, registry) {
            for t in data.card_types {
                if !types.contains(&t) {
                    types.push(t);
                }
            }
        }
        // CR 205.1b: power and toughness belong to creatures, so an object
        // carrying a runtime P/T is one — that is what "becomes a 3/3" means.
        //
        // This lived in `is_creature` alone, as `|| o.power.is_some()`, which
        // left `has_card_type(Creature)` answering "no" about objects the rest
        // of the engine treated as creatures. Spare from Evil is where the
        // split showed: its "non-Human *creature*" filter has to ask about
        // card types, and got a different board than `is_creature` sees.
        if !types.contains(&crate::types::CardType::Creature)
            && self.get_object(id).is_some_and(|o| o.power.is_some())
        {
            types.push(crate::types::CardType::Creature);
        }
        types
    }

    /// Whether the object has the given card type on its active face.
    #[must_use]
    pub fn has_card_type(&self, id: ObjectId, card_type: crate::types::CardType, registry: &crate::cards::CardRegistry) -> bool {
        self.card_types_of(id, registry).contains(&card_type)
    }

    /// The player a Curse is attached to, falling back to the player it was
    /// attached to immediately before it left the battlefield.
    ///
    /// CR 113.7a and 608.2: a Curse's triggered ability on the stack resolves
    /// even if the Curse is destroyed in response, and it still knows whom it
    /// cursed. Every Curse used to answer this itself as
    /// `get_object(id).filter(|o| o.zone == Battlefield).attached_to_player`,
    /// which is exactly the ability the rule says does not go away.
    #[must_use]
    pub fn attached_player(&self, id: ObjectId) -> Option<PlayerId> {
        let obj = self.get_object(id)?;
        obj.attached_to_player.or(obj.last_attached_to_player)
    }

    /// The creatures a player controls right now, as a fixed list.
    ///
    /// CR 611.2c: a continuous effect created by a resolving spell or ability
    /// affects the objects that were there when it resolved, and that set never
    /// changes afterwards. Pump spells and one-shot ability effects take their
    /// list from here; a permanent's *static* anthem does not, because that one
    /// really does pick up creatures as they arrive.
    #[must_use]
    pub fn creatures_controlled_snapshot(&self, controller: PlayerId, registry: &crate::cards::CardRegistry) -> Vec<ObjectId> {
        self.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.controller == controller)
            .filter(|o| self.is_creature(o.id, registry))
            .map(|o| o.id)
            .collect()
    }

    /// Who controls this object, or — once it has left the battlefield — who
    /// controlled it last (CR 608.2g).
    ///
    /// For anything still on the battlefield this is just `controller`. It
    /// differs only for a permanent that has left, where `controller` has been
    /// reset to the owner and the answer a resolving ability wants is the
    /// player who controlled the permanent when it triggered.
    #[must_use]
    pub fn last_known_controller(&self, id: ObjectId) -> PlayerId {
        let Some(obj) = self.get_object(id) else { return PlayerId(0) };
        if obj.zone == Zone::Battlefield {
            return obj.controller;
        }
        obj.last_controller.unwrap_or(obj.controller)
    }

    /// Whether this permanent was declared as an attacker this turn (CR 508.1).
    ///
    /// Homicidal Brute's "if this creature didn't attack this turn" is the
    /// caller. Note the ruling this satisfies: an attack made while the card
    /// was front-face-up still counts once it has transformed, because
    /// transforming does not make a new object (CR 712.8).
    #[must_use]
    pub fn attacked_this_turn(&self, id: ObjectId) -> bool {
        self.get_object(id).and_then(|o| o.attacked_on_turn) == Some(self.turn_number)
    }

    /// Whether the object is a creature. Card types are the whole answer —
    /// `card_types_of` already counts a runtime power and toughness as
    /// creature-ness (CR 205.1b), so this and `has_card_type(Creature)` cannot
    /// disagree.
    #[must_use]
    pub fn is_creature(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> bool {
        self.has_card_type(id, crate::types::CardType::Creature, registry)
    }

    /// Subtypes of the object: the union of object-level subtypes and the
    /// active face's subtypes.
    #[must_use]
    pub fn subtypes_of(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Vec<String> {
        let mut subs = self.get_object(id).map(|o| o.subtypes.clone()).unwrap_or_default();
        if let Some(data) = self.face_data(id, registry) {
            for s in data.subtypes {
                if !subs.contains(&s) {
                    subs.push(s);
                }
            }
        }
        subs
    }

    /// Whether the object has the given subtype on its active face.
    #[must_use]
    pub fn has_subtype(&self, id: ObjectId, subtype: &str, registry: &crate::cards::CardRegistry) -> bool {
        self.get_object(id).is_some_and(|o| o.subtypes.iter().any(|s| s == subtype))
            || self.face_data(id, registry)
                .is_some_and(|d| d.subtypes.iter().any(|s| s == subtype))
    }

    /// CR 301.5: an Equipment is an artifact with the Equipment subtype.
    ///
    /// This was a per-object `is_equipment` bool that eleven cards each set in
    /// an `on_resolve` override — overrides that otherwise only repeated the
    /// trait default's "move a permanent to the battlefield". Anything that put
    /// an Equipment onto the battlefield by another route left the flag false,
    /// and `sba.rs` then treated it as an unattached Aura and put it into the
    /// graveyard (CR 704.5m) instead of detaching it and leaving it there.
    ///
    /// Derived from the subtype, through the characteristics layer, so a
    /// granted Equipment subtype counts too.
    pub fn is_equipment(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> bool {
        self.has_subtype(id, "Equipment", registry)
    }

    /// CR 109.1: a "card" is a physical game object. A token is not one, so
    /// any effect whose text says "card" must exclude tokens.
    ///
    /// This matters in the graveyard specifically. SBA removes a token from a
    /// non-battlefield zone (CR 704.5e), but that is a discrete pass, not
    /// something that happens the instant the token arrives — so a count taken
    /// mid-resolution can see a token that is on its way out.
    #[must_use]
    pub fn is_card(&self, id: ObjectId) -> bool {
        self.get_object(id).is_some_and(|o| !o.is_token)
    }

    /// Whether this permanent can pay a `{T}` cost right now.
    ///
    /// Three conditions, and they are the same for every permanent in the
    /// game: it has to be on the battlefield, it has to be untapped, and — if
    /// it is a creature — it has to have been under its controller's control
    /// since their most recent turn began, unless it has haste (CR 302.6).
    /// Non-creature permanents are never affected by summoning sickness even
    /// though the flag is set on them.
    ///
    /// Card code must not re-derive this. Two of the twenty-odd cards with a
    /// `{T}` ability used to spell the check out by hand; both forgot haste,
    /// and the nineteen others forgot summoning sickness entirely. The engine
    /// applies this gate when it enumerates abilities, so a card's
    /// `activated_abilities` / `mana_abilities` states only the conditions
    /// particular to that ability.
    #[must_use]
    pub fn can_pay_tap_cost(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> bool {
        let Some(obj) = self.get_object(id) else { return false; };
        if obj.zone != Zone::Battlefield || obj.tapped {
            return false;
        }
        !(obj.summoning_sick
            && self.is_creature(id, registry)
            && !self.has_keyword(id, crate::types::Keyword::Haste, registry))
    }

    /// Printed keywords of the object: the active face's, or the object's own
    /// for something with no registry face (a generic token, whose
    /// `obj.keywords` ARE its printed keywords).
    ///
    /// This is the printed set only — keywords granted by continuous or
    /// temporary effects are not included. Ask `has_keyword` for the full
    /// picture; use this when copying, which copies printed values (CR 707.2).
    #[must_use]
    pub fn printed_keywords_of(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Vec<crate::types::Keyword> {
        if let Some(data) = self.face_data(id, registry) {
            return data.keywords;
        }
        self.get_object(id).map(|o| o.keywords.clone()).unwrap_or_default()
    }

    /// Card types as printed on the active face, ignoring anything granted at
    /// runtime. The copy counterpart of `card_types_of`.
    #[must_use]
    pub fn printed_card_types_of(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Vec<crate::types::CardType> {
        if let Some(data) = self.face_data(id, registry) {
            return data.card_types;
        }
        self.get_object(id).map(|o| o.card_types.clone()).unwrap_or_default()
    }

    /// Subtypes as printed on the active face, ignoring anything granted at
    /// runtime — Olivia Voldaren's "Vampire", Grimoire of the Dead's "Zombie".
    /// The copy counterpart of `subtypes_of`.
    #[must_use]
    pub fn printed_subtypes_of(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Vec<String> {
        if let Some(data) = self.face_data(id, registry) {
            return data.subtypes;
        }
        self.get_object(id).map(|o| o.subtypes.clone()).unwrap_or_default()
    }

    /// Colors as printed — derived from the active face's mana cost — ignoring
    /// anything granted at runtime (Grimoire of the Dead's black). The copy
    /// counterpart of `colors_of`.
    #[must_use]
    pub fn printed_colors_of(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Vec<crate::types::Color> {
        if let Some(data) = self.face_data(id, registry) {
            let mut cols = Vec::new();
            if let Some(cost) = data.cost {
                for sym in &cost.symbols {
                    if let crate::types::ManaSymbol::Colored(c) = sym {
                        if !cols.contains(c) {
                            cols.push(*c);
                        }
                    }
                }
            }
            return cols;
        }
        self.get_object(id).map(|o| o.colors.clone()).unwrap_or_default()
    }

    /// Power and toughness as printed on the active face, ignoring runtime
    /// grants. The copy counterpart of `effective_power`/`effective_toughness`.
    #[must_use]
    pub fn printed_pt_of(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> (Option<i32>, Option<i32>) {
        if let Some(data) = self.face_data(id, registry) {
            return (data.power, data.toughness);
        }
        self.get_object(id).map_or((None, None), |o| (o.power, o.toughness))
    }

    /// Colors of the object: the union of any granted at runtime (Grimoire of
    /// the Dead's black) and those derived from the active face's mana cost.
    /// (Color indicators are not modeled.)
    #[must_use]
    pub fn colors_of(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Vec<crate::types::Color> {
        let mut cols = self.get_object(id).map(|o| o.colors.clone()).unwrap_or_default();
        if let Some(cost) = self.face_data(id, registry).and_then(|d| d.cost) {
            for sym in &cost.symbols {
                if let crate::types::ManaSymbol::Colored(c) = sym {
                    if !cols.contains(c) {
                        cols.push(*c);
                    }
                }
            }
        }
        cols
    }

    /// Continuous effects the object provides: instance-level overrides if
    /// present (e.g. equipment granting effects), otherwise the active face's.
    #[must_use]
    pub fn continuous_effects_of(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Vec<crate::types::ContinuousEffect> {
        let Some(obj) = self.get_object(id) else { return Vec::new() };
        if let Some(ref inst) = obj.instance_continuous_effects {
            return inst.clone();
        }
        self.face_data(id, registry).map(|d| d.continuous_effects).unwrap_or_default()
    }

    /// Triggered abilities of the object's active face.
    #[must_use]
    pub fn triggered_abilities_of(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Vec<crate::cards::TriggeredAbilityDef> {
        self.face_data(id, registry).map(|d| d.triggered_abilities).unwrap_or_default()
    }
}


/// A single game object — an instance of a card on the battlefield, in hand, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameObject {
    pub id: ObjectId,
    pub card_id: CardId,
    pub name: String,
    pub owner: PlayerId,
    pub controller: PlayerId,
    pub zone: Zone,

    // Battlefield state
    pub tapped: bool,
    pub summoning_sick: bool,
    /// Who controlled this permanent immediately before it left the
    /// battlefield (CR 608.2g — last known information).
    ///
    /// A triggered ability on the stack outlives its source (CR 113.7a), so a
    /// handler resolving after the permanent has died still has to know whose
    /// ability it is. `controller` is reset to the owner on the way out, per
    /// CR 108.4, so it cannot answer that. The Curse pair
    /// `last_attached_to_player` exists for the same reason.
    pub last_controller: Option<PlayerId>,
    /// The turn this permanent was last declared as an attacker, if any
    /// (CR 508.1). "Didn't attack this turn" is a plain fact about the game,
    /// asked by Homicidal Brute among others; it is not a triggered ability,
    /// and a card that needs the answer should not have to fabricate one to
    /// record it.
    ///
    /// Turn-stamped rather than a bare flag so it needs no per-turn sweep to
    /// clear, and kept across a transform — CR 712.8, transforming does not
    /// make a new object, so an attack made by one face counts for the other.
    pub attacked_on_turn: Option<u32>,
    pub damage_marked: u32,
    /// Which creatures have dealt damage to this creature this turn.
    /// Used by Abattoir Ghoul ("when creature dealt damage by this dies").
    #[serde(default)]
    pub damaged_by: Vec<ObjectId>,
    /// Whether this creature has been dealt damage by a source with deathtouch.
    pub dealt_deathtouch_damage: bool,

    // Characteristics (from card data, may be modified by effects later)
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub colors: Vec<crate::types::Color>,
    /// Keywords on this object (populated from `card_data` for real cards, set directly for tokens).
    pub keywords: Vec<crate::types::Keyword>,
    /// Card types on this object (populated from `card_data`, set directly for tokens).
    pub card_types: Vec<crate::types::CardType>,
    /// Subtypes on this object (for tokens — regular cards use CardData.subtypes via registry).
    #[serde(default)]
    pub subtypes: Vec<String>,

    // Targets chosen when this spell was cast (only relevant while on the stack).
    pub targets: Vec<crate::actions::Target>,

    // Aura/equipment: what this is attached to.
    pub attached_to: Option<ObjectId>,
    // Curse aura: attached to a player (instead of an object).
    pub attached_to_player: Option<PlayerId>,
    /// Who this was attached to immediately before it left the battlefield —
    /// last known information (CR 608.2) for a Curse whose triggered ability
    /// is still on the stack after the Curse is destroyed. The object-valued
    /// counterpart is `card_state["last_attached_to"]`.
    #[serde(default)]
    pub last_attached_to_player: Option<PlayerId>,

    // Tracks zone changes for staleness detection (XMage pattern).
    pub zone_change_count: u32,

    /// CR 706.2: when a copy effect says "except it has <ability>", the copy's
    /// `card_id` becomes the copied card and the granting card's identity would
    /// otherwise be lost. This records the card whose copy effect produced this
    /// object, so the engine can ask THAT card's behavior for the abilities it
    /// added — without knowing which card it is. `None` for anything that is
    /// not such a copy.
    #[serde(default)]
    pub copy_grantor: Option<CardId>,

    /// Whether this object is a token (tokens cease to exist when not on the battlefield).
    pub is_token: bool,
    /// Whether this permanent is legendary (for the legend rule SBA).
    #[serde(default)]
    pub is_legendary: bool,

    /// Whether this spell was cast using flashback (exiled instead of going to graveyard).
    #[serde(default)]
    pub cast_with_flashback: bool,

    /// Per-instance oracle text override (e.g., Bonds of Faith conditional effect).
    /// When set, aura parsing uses this instead of the card's static oracle text.
    #[serde(default)]
    pub instance_oracle_text: Option<String>,

    /// Per-instance continuous effects that override the card's static effects.
    /// Set by `on_enter_battlefield` for conditional cards (e.g., Bonds of Faith).
    #[serde(default)]
    pub instance_continuous_effects: Option<Vec<crate::types::ContinuousEffect>>,

    /// Card-specific persistent state (e.g., Fiend Hunter stores the exiled creature ID).
    /// Keyed by purpose string, value is an `ObjectId`.
    #[serde(default)]
    pub card_state: HashMap<String, ObjectId>,

    /// Counters on this permanent (+1/+1, -1/-1, etc.).
    pub counters: HashMap<crate::types::CounterType, u32>,

    /// Number of regeneration shields (consumed instead of destruction).
    #[serde(default)]
    pub regeneration_shields: u32,

    /// Whether this double-faced card is on its back face.
    #[serde(default)]
    pub is_transformed: bool,

    /// Whether a state-triggered ability (CR 603.8) for this object is currently
    /// on the stack. While true, the trigger won't fire again.
    #[serde(default)]
    pub state_trigger_on_stack: bool,

    /// Chosen X value for X-cost spells (stored while on the stack).
    #[serde(default)]
    pub x_value: Option<u32>,

    /// Chosen mode index for `ModalChoice` spells (stored while on the stack).
    /// Set when a spell with a `ModalChoice` target requirement is cast.
    #[serde(default)]
    pub chosen_mode: Option<usize>,

    /// Activated abilities used this turn (for once-per-turn tracking).
    #[serde(default)]
    pub abilities_activated_this_turn: std::collections::HashSet<usize>,

    /// Whether this permanent is an entering-battlefield copy source (replacement effect).
    /// When true, other creatures entering the battlefield under the same controller
    /// enter as a copy of this permanent instead of their original form (CR 614.1d).
    /// Used by Essence of the Wild and similar cards.
    #[serde(default)]
    pub entering_copy_source: bool,
}

/// A player's state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: PlayerId,
    pub life: i32,
    pub mana_pool: ManaPool,
    pub land_plays_remaining: u32,
    pub lost: bool,
    pub has_drawn_from_empty: bool,
    /// Order of cards in library (first element is top of library).
    pub library_order: Vec<ObjectId>,
    /// Number of mulligans this player has taken during the opening-hand phase.
    /// Used to determine how many cards must be bottomed after keeping.
    #[serde(default)]
    pub mulligan_count: u32,
    /// True once this player has chosen `MulliganKeep` (or hit the cap).
    /// Players with `mulligan_kept = true` are skipped in subsequent
    /// mulligan rounds.
    #[serde(default)]
    pub mulligan_kept: bool,
}

impl PlayerState {
    #[must_use]
    pub fn new(id: PlayerId) -> Self {
        Self {
            id,
            life: 40, // commander starting life
            mana_pool: ManaPool::new(),
            land_plays_remaining: 1,
            lost: false,
            has_drawn_from_empty: false,
            library_order: Vec::new(),
            mulligan_count: 0,
            mulligan_kept: false,
        }
    }

    /// Draw the top card of the library. Returns None if library is empty.
    pub fn draw_top_card(&mut self) -> Option<ObjectId> {
        if self.library_order.is_empty() {
            self.has_drawn_from_empty = true;
            None
        } else {
            Some(self.library_order.remove(0))
        }
    }

    /// Remove the top card from the library without setting `has_drawn_from_empty`.
    /// Used for reveal/search loops that are NOT drawing (e.g., Mirror-Mad Phantasm).
    pub fn reveal_top_card(&mut self) -> Option<ObjectId> {
        if self.library_order.is_empty() {
            None
        } else {
            Some(self.library_order.remove(0))
        }
    }
}

/// One "gain control of X for as long as you control this permanent" effect
/// (CR 611.2b).
///
/// The duration ends the *moment* its condition stops being true, however that
/// happens — the source leaving the battlefield is the obvious way, but an
/// opponent taking the source with Act of Treason ends it just as surely, with
/// no zone change to notice. Reverting only in the source's
/// `on_leave_battlefield` handled the first and missed the second, so the
/// condition is written down here and checked as a state-based action instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlEffect {
    /// The permanent whose control was gained.
    pub object: ObjectId,
    /// Who controls it while the effect lasts.
    pub controller: PlayerId,
    /// Who controlled it before, and gets it back when the effect ends.
    pub original_controller: PlayerId,
    /// The permanent the duration depends on ("...for as long as you control
    /// **Olivia Voldaren**").
    pub source: ObjectId,
    /// Who has to keep controlling `source`. The effect ends if `source`
    /// leaves the battlefield or comes under anyone else's control.
    pub source_controller: PlayerId,
}

/// Combat state, tracking attackers and blockers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CombatState {
    /// Map of attacker `ObjectId` -> defending `PlayerId`.
    pub attackers: HashMap<ObjectId, PlayerId>,
    /// Map of attacker `ObjectId` -> list of blockers assigned to it.
    pub blocker_assignments: HashMap<ObjectId, Vec<ObjectId>>,
    /// Attackers that became blocked when blockers were declared. Blocked-ness
    /// is permanent for the combat (CR 509.2): an attacker whose blockers all
    /// leave combat is still blocked (deals no combat damage without trample),
    /// which `blocker_assignments` alone can't express once its list empties.
    #[serde(default)]
    pub blocked_attackers: HashSet<ObjectId>,
    /// Creatures that had first/double strike when first-strike combat damage
    /// was dealt (CR 510.5): they don't deal damage again in the regular
    /// combat damage step unless they have double strike.
    #[serde(default)]
    pub dealt_first_strike: HashSet<ObjectId>,
}

impl CombatState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Context for an X-cost activated ability whose effect is deferred until
/// the player completes the funding choice. Populated when the
/// `ActivateAbility` handler sets up a `ChooseXFunding` prompt; consumed
/// when that prompt resolves to fire the ability's effect with the
/// now-known X value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAbilityEffect {
    /// The permanent that owns the ability.
    pub source_id: ObjectId,
    /// Which of its activated abilities was activated.
    pub ability_index: usize,
    /// The card whose behavior contributed this ability (may differ from
    /// the source permanent for aura-granted abilities / Evil Twin).
    pub behavior_card_id: CardId,
    /// Targets chosen at activation time.
    pub targets: Vec<crate::actions::Target>,
    /// Human-readable description of the ability (for log message).
    pub description: String,
    /// Player who activated the ability.
    pub activator: PlayerId,
}

/// Context stashed between `CastSpell` action submission and the follow-up
/// `ChooseXFunding` resolution for X-cost spells. The spell stays in its
/// originating zone (Hand for normal casts, Graveyard for flashback/
/// cast-from-graveyard) until funding completes — per CR 601.2h → 601.2i
/// the total cost is paid before the spell becomes cast. Once funding
/// resolves the engine executes the stashed plan atomically: tap mana,
/// pay mana (non-X + X), pay additional costs, move to stack, fire
/// `SpellCast`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSpellCast {
    /// The spell object being cast (still in its origin zone).
    pub object_id: ObjectId,
    /// Player casting the spell.
    pub player: PlayerId,
    /// Card ID for behavior / `card_data` lookups.
    pub card_id: CardId,
    /// Chosen targets.
    pub targets: Vec<crate::actions::Target>,
    /// Chosen sacrifice for `AdditionalCost::SacrificeCreature`, if any.
    pub sacrifice: Option<ObjectId>,
    /// Chosen graveyard cards for `ExileXFromGraveyard` /
    /// `ExileCreaturesFromGraveyard` additional costs.
    pub exile_ids: Vec<ObjectId>,
    /// Count chosen for `ExileXFromGraveyard`.
    pub exile_count: Option<u32>,
    /// Autotap plan produced by `compute_autotap` on the non-X portion.
    pub tap_plan: Vec<(ObjectId, usize)>,
    /// Alternative cost (e.g. Rooftop Storm) if in effect.
    pub alternative_cost: Option<crate::types::ManaCost>,
    /// Non-X portion of the spell's mana cost (ready for `auto_pay`).
    pub non_x_mana_cost: crate::types::ManaCost,
    /// True if this is a flashback cast.
    pub is_flashback: bool,
}

/// What the engine is waiting for the player to do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AwaitingAction {
    DeclareAttackers,
    DeclareBlockers { defending_player: PlayerId },
    DiscardToHandSize { player: PlayerId, discard_count: usize },
    /// A card or trigger needs a player to make a choice during resolution.
    ResolutionChoice {
        player: PlayerId,
        source: ObjectId,
        choice: ResolutionChoiceKind,
    },
    /// London mulligan: `player` must decide keep or mull.
    MulliganDecision { player: PlayerId },
    /// London mulligan: `player` must put `count` cards on the bottom of
    /// their library (one per mulligan taken).
    BottomAfterMulligan { player: PlayerId, count: usize },
}

/// London mulligan cap: house rule, stop offering mulligans once a player
/// has taken this many. (Mull-to-4: after 3 mulligans you must keep.)
pub const LONDON_MULLIGAN_CAP: u32 = 3;

/// Describes what kind of mid-resolution choice is needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolutionChoiceKind {
    /// Choose whether to pay a cost (Frightful Delusion's "pay {1}").
    PayOrNot {
        description: String,
        spell_id: ObjectId,
        source_spell_id: ObjectId,
        /// What the player is being asked to pay. The engine offers the "pay"
        /// answer only when they can produce this much (CR 608.2g), and pays
        /// exactly this when they take it — it used to be hardcoded to {1} at
        /// the one place that deducted it.
        cost: crate::types::ManaCost,
    },
    /// Choose one target from a list (damage, destroy, return, counters).
    ChooseTarget {
        description: String,
        options: Vec<crate::actions::Target>,
        optional: bool,
        effect: PendingEffect,
    },
    /// A yes/no choice ("you may" abilities).
    /// If yes, the card's trigger continues with its effect.
    /// If no, nothing happens.
    YesNo {
        description: String,
        source_card: ObjectId,
    },
    /// Choose a card from hand to discard (Murder of Crows, future discard effects).
    ChooseCardFromHand {
        description: String,
        player: PlayerId,
        cards: Vec<ObjectId>,
        /// Whether the engine discards the chosen card as soon as it is
        /// chosen. True for an ordinary one-player discard.
        ///
        /// False when this is one of several choices being collected under
        /// CR 101.4 — "each player discards a card" has every player choose in
        /// APNAP order and the cards leave their hands *simultaneously*, so
        /// the source card holds the choices and discards them together once
        /// the last player has chosen. Discarding as we go would let a discard
        /// trigger fire, and be seen, while a later player is still choosing.
        discard_immediately: bool,
    },
    /// Choose one card from a revealed set to keep (Forbidden Alchemy).
    ChooseFromRevealed {
        description: String,
        revealed: Vec<ObjectId>,
    },
    /// Search library for a card matching criteria and choose one (Garruk -1, etc.).
    /// All matching cards are shown to the player. They pick one to put into hand.
    /// After choosing, the library is shuffled.
    ChooseFromLibrary {
        description: String,
        /// All cards in the library that match the search criteria.
        options: Vec<ObjectId>,
        /// The player whose library is being searched.
        searcher: PlayerId,
        /// The source permanent that initiated the search.
        source_id: ObjectId,
        /// Where the found card goes (CR 701.19). Hand for a tutor, battlefield
        /// for a land-fetch. This used to be hardcoded to `Hand`, which is why
        /// every card that fetched onto the battlefield hand-rolled its own
        /// search instead of using this.
        destination: Zone,
        /// Whether it arrives tapped — only meaningful for the battlefield.
        tapped: bool,
    },
    /// Choose a permanent type from a list of options (Creeping Renaissance).
    ChooseCardType {
        description: String,
        options: Vec<String>,
        controller: PlayerId,
    },
    /// Divide permanents into two piles (Liliana of the Veil -6).
    /// The choosing player selects a subset to form pile 1; the rest form pile 2.
    /// After division, the target player chooses which pile to sacrifice.
    DividePermanentsIntoPiles {
        description: String,
        /// All permanents to divide.
        permanents: Vec<ObjectId>,
        /// The player who will choose which pile to sacrifice (the target player).
        target_player: PlayerId,
        /// The source permanent (Liliana).
        source_id: ObjectId,
    },
    /// Choose one of two piles to sacrifice (Liliana of the Veil -6, step 2).
    ChoosePile {
        description: String,
        /// Permanents in pile 1.
        pile_1: Vec<ObjectId>,
        /// Permanents in pile 2.
        pile_2: Vec<ObjectId>,
        /// The source permanent (Liliana).
        source_id: ObjectId,
    },
    /// Choose a card name from all implemented nonland cards (Nevermore).
    ChooseCardName {
        description: String,
        /// All valid card names the player can choose from.
        options: Vec<String>,
        /// The source permanent that needs the chosen name.
        source_id: ObjectId,
    },
    /// Choose how to fund X for an X-cost spell or activated ability.
    ///
    /// The player picks specific mana sources to tap plus amounts to drain
    /// from each color in their pool; the sum determines X. This replaces
    /// the older `ChooseXValue` flow which only let the player pick X as a
    /// single number and auto-selected sources.
    ///
    /// Rules ordering: this prompt runs BEFORE the spell is placed on the
    /// stack and before `SpellCast`-style triggers fire, matching CR 601.2b
    /// (announce X) → 601.2h (pay total cost) → 601.2i (spell becomes cast).
    /// Choose which cards to exile from the graveyard as an additional
    /// cost to cast a spell with `AdditionalCost::ExileXFromGraveyard`
    /// (Harvest Pyre) or `AdditionalCost::ExileCreaturesFromGraveyard(n)`
    /// (Stitched Drake / Skaab Ruinator / Makeshift Mauler / Corpse
    /// Lunge).
    ///
    /// Rules ordering: this prompt runs BEFORE the spell is placed on the
    /// stack. Per CR 601.2h → 601.2i the total cost (including additional
    /// costs) is paid before the spell becomes cast. The engine stashes
    /// the partially-specified cast context on
    /// [`GameState::pending_spell_cast`]; the resolution handler fills in
    /// the chosen exile set and executes the cast atomically (tap mana,
    /// pay mana, exile cards, move to stack, fire `SpellCast`).
    ///
    /// For `ExileXFromGraveyard` (variable count): any size `0..=options.len()`
    /// is legal — the damage/etc. scales with X. For
    /// `ExileCreaturesFromGraveyard(n)` (fixed count): exactly `n` must be
    /// chosen. The count constraint is surfaced to players via `min`/`max`
    /// and enforced at resolve time.
    ChooseExileFromGraveyard {
        description: String,
        /// Graveyard cards eligible for exile — filtered per the spell's
        /// additional cost (creatures only for Stitched Drake et al.,
        /// all cards for Harvest Pyre).
        options: Vec<ObjectId>,
        /// Minimum number of cards the player must choose.
        min: usize,
        /// Maximum number of cards the player may choose.
        max: usize,
        /// The spell being cast (currently in its origin zone until the
        /// prompt resolves).
        source_id: ObjectId,
    },
    ChooseXFunding {
        description: String,
        /// All available funding options (pool mana + tap sources).
        options: crate::funding::FundingOptions,
        /// For spells: the object being cast (currently in `Zone::Stack` as a
        /// staging area with `x_value = None` until funding completes).
        /// For abilities: the source permanent.
        source_id: ObjectId,
        /// True for activated abilities (sets `last_activated_x_value`),
        /// false for spells (sets `obj.x_value` on the stack object).
        is_ability: bool,
    },
}

/// What happens to the chosen target when a `ResolutionChoice` is resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PendingEffect {
    /// Deal N damage to the chosen target.
    DealDamage { amount: u32, source_id: ObjectId, source_name: String },
    /// Destroy the chosen permanent.
    Destroy { source_name: String },
    /// Move chosen creature from graveyard to battlefield.
    ReturnToBattlefield { spell_id: ObjectId },
    /// Put +1/+1 counters on chosen creature.
    AddCounters { count: u32 },
    /// Apply -N/-M until end of turn to the chosen creature.
    DebuffUntilEOT { power: i32, toughness: i32, source_name: String },
    /// Target creature can't block this turn.
    CantBlockThisTurn { source_name: String },



    /// Destroy target creature matching a filter (Reaper from the Abyss).
    DestroyCreature { source_name: String },

    /// Return the chosen object to its owner's hand.
    ReturnToHand { source_name: String },
    /// Put the chosen object on top of its owner's library.
    PutOnTopOfLibrary { source_name: String },




    /// Sacrifice the chosen creature (generic sacrifice, e.g. Liliana -2).
    SacrificeCreature { source_name: String },
    /// Copy the chosen creature onto the source permanent (Evil Twin clone effect).
    /// The source becomes a copy of the target, except it retains any extra abilities
    /// stored via `card_state` markers.
    CopyCreature { source_id: ObjectId },



    /// Grant flashback to a chosen card until end of turn (Snapcaster Mage).
    GrantFlashback { source_name: String },




    /// The legend rule: the chosen permanent is KEPT, all others with the same name
    /// under that player's control are sent to the graveyard.
    LegendRuleKeep { player: PlayerId, legend_name: String },
    /// CR 603.3d: attach the chosen target to the next pending trigger
    /// in the AP/NAP push queue, then push it onto the stack and continue
    /// processing remaining pending triggers.
    /// CR 701.19: complete a library search the player was offered. Generic —
    /// the searcher, destination and tapped-ness fully describe it, so any
    /// "you may search your library for ..." uses this rather than a
    /// card-specific effect. Declining still shuffles.
    FinishLibrarySearch { searcher: PlayerId, destination: Zone, tapped: bool },

    /// A deferred resolution that belongs to the card that queued it.
    ///
    /// The engine routes the chosen target back to `source_id`'s behavior via
    /// `CardBehavior::resolve_card_effect` and does nothing else. Use this for
    /// anything card-specific: the alternative is a new engine enum variant
    /// plus an engine match arm executing one card's rules, which is how the
    /// engine ended up knowing about Ghost Quarter, Moorland Haunt, Curse of
    /// Oblivion and Elder Cathar's Human bonus by name.
    ///
    /// `key` distinguishes multiple deferred effects on the same card; a card
    /// with only one can pass `""`. Carry extra state in the source's
    /// `card_state`, not in new engine variants.
    CardEffect { source_id: ObjectId, key: String },

    AttachTargetToPendingTrigger,
}

/// Game result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameResult {
    Winner(PlayerId),
    Draw,
}

/// Day/night state for werewolf transform tracking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DayNight {
    Day,
    Night,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::CardId;

    #[test]
    fn create_and_move_object() {
        let registry = crate::cards::CardRegistry::with_all_cards();
        let mut state = GameState::new(2);
        let id = state.create_object(CardId(1), PlayerId(0), Zone::Hand, None, None);

        assert_eq!(state.get_object(id).unwrap().zone, Zone::Hand);

        state.move_object(id, Zone::Battlefield, &registry);
        let obj = state.get_object(id).unwrap();
        assert_eq!(obj.zone, Zone::Battlefield);
        assert!(obj.summoning_sick);
        assert_eq!(obj.zone_change_count, 1);
    }

    #[test]
    fn objects_in_zone_filters_correctly() {
        let mut state = GameState::new(2);
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);

        state.create_object(CardId(1), p0, Zone::Hand, None, None);
        state.create_object(CardId(2), p0, Zone::Hand, None, None);
        state.create_object(CardId(1), p1, Zone::Hand, None, None);

        assert_eq!(state.objects_in_zone(Zone::Hand, p0).len(), 2);
        assert_eq!(state.objects_in_zone(Zone::Hand, p1).len(), 1);
    }

    #[test]
    fn leaving_battlefield_resets_state() {
        let registry = crate::cards::CardRegistry::with_all_cards();
        let mut state = GameState::new(2);
        let id = state.create_object(CardId(1), PlayerId(0), Zone::Battlefield, Some(2), Some(2));

        {
            let obj = state.get_object_mut(id).unwrap();
            obj.tapped = true;
            obj.damage_marked = 1;
        }

        state.move_object(id, Zone::Graveyard, &registry);
        let obj = state.get_object(id).unwrap();
        assert!(!obj.tapped);
        assert_eq!(obj.damage_marked, 0);
        assert!(!obj.summoning_sick);
    }

    #[test]
    fn draw_from_empty_library() {
        let mut state = GameState::new(2);
        let player = state.get_player_mut(PlayerId(0));
        assert!(player.draw_top_card().is_none());
        assert!(player.has_drawn_from_empty);
    }

    #[test]
    fn face_data_uses_back_face_when_transformed() {
        let registry = crate::cards::CardRegistry::with_all_cards();
        let mut state = GameState::new(2);
        let dfc = registry.get_id_by_name("Daybreak Ranger").unwrap();
        let id = state.create_object(dfc, PlayerId(0), Zone::Battlefield, Some(2), Some(2));

        assert_eq!(state.face_data(id, &registry).unwrap().name, "Daybreak Ranger");
        state.get_object_mut(id).unwrap().is_transformed = true;
        assert_eq!(state.face_data(id, &registry).unwrap().name, "Nightfall Predator");
    }

    #[test]
    fn card_types_of_falls_back_to_registry_for_non_tokens() {
        let registry = crate::cards::CardRegistry::with_all_cards();
        let mut state = GameState::new(2);
        let pike = registry.get_id_by_name("Runechanter's Pike").unwrap();
        // Non-token permanents have empty object-level card_types.
        let id = state.create_object(pike, PlayerId(0), Zone::Battlefield, None, None);
        assert!(state.get_object(id).unwrap().card_types.is_empty());

        assert!(state.has_card_type(id, crate::types::CardType::Artifact, &registry));
        assert!(!state.is_creature(id, &registry));
    }

    #[test]
    fn is_creature_covers_cards_tokens_and_anonymous_objects() {
        let registry = crate::cards::CardRegistry::with_all_cards();
        let mut state = GameState::new(2);

        let bears = registry.get_id_by_name("Grizzly Bears").unwrap();
        let card = state.create_object(bears, PlayerId(0), Zone::Battlefield, Some(2), Some(2));
        assert!(state.is_creature(card, &registry));

        // Anonymous object with P/T (test convention).
        let anon = state.create_object(CardId(9999), PlayerId(0), Zone::Battlefield, Some(1), Some(1));
        assert!(state.is_creature(anon, &registry));

        // Aura: no P/T, not a creature.
        let pacifism = registry.get_id_by_name("Pacifism").unwrap();
        let aura = state.create_object(pacifism, PlayerId(0), Zone::Battlefield, None, None);
        assert!(!state.is_creature(aura, &registry));
    }

    #[test]
    fn subtypes_of_is_transform_aware() {
        let registry = crate::cards::CardRegistry::with_all_cards();
        let mut state = GameState::new(2);
        let dfc = registry.get_id_by_name("Daybreak Ranger").unwrap();
        let id = state.create_object(dfc, PlayerId(0), Zone::Battlefield, Some(2), Some(2));

        assert!(state.has_subtype(id, "Human", &registry));
        state.get_object_mut(id).unwrap().is_transformed = true;
        let subs = state.subtypes_of(id, &registry);
        assert!(subs.iter().any(|s| s == "Werewolf"), "back face subtypes: {subs:?}");
    }

    #[test]
    fn equipment_dynamic_pt_does_not_leak_into_own_effective_pt() {
        // Runechanter's Pike implements dynamic_pt for the equipped creature.
        // The equipment itself (base P/T None) must not report effective P/T.
        let registry = crate::cards::CardRegistry::with_all_cards();
        let mut state = GameState::new(2);
        let pike = registry.get_id_by_name("Runechanter's Pike").unwrap();
        let id = state.create_object(pike, PlayerId(0), Zone::Battlefield, None, None);

        assert_eq!(state.effective_power(id, &registry), None,
            "equipment must not have effective power from its own dynamic_pt");
        assert_eq!(state.effective_toughness(id, &registry), None,
            "equipment must not have effective toughness from its own dynamic_pt");
    }
}
