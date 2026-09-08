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
        /// What the ability asks of its target, captured when it went on the
        /// stack (CR 601.2c).
        ///
        /// CR 608.2b re-checks a target against the same requirement that
        /// offered it, and for an ability that requirement cannot be looked up
        /// again on resolution: paying a `SacrificeThis` cost has already
        /// removed the source, and with it the ability list. Before this rode
        /// on the entry, the ability arm re-checked only hexproof and
        /// protection plus whatever the card restated in its own
        /// `is_valid_target` — so an ability whose card restated nothing
        /// resolved against a target that had stopped satisfying it.
        target_requirement: Option<crate::cards::TargetRequirement>,
        /// The creature sacrificed to pay this ability's cost, if any.
        ///
        /// Disciple of Griselbrand's "you gain life equal to the sacrificed
        /// creature's toughness" needs to know *which* creature that was, and
        /// the cost is paid at activation (CR 601.2h) while the ability
        /// resolves later. It used to scan back for the most recent
        /// `CreatureDied` event, which is whatever died last — including
        /// something the opponent killed in the priority window in between.
        sacrificed: Option<ObjectId>,
        /// The sacrificed creature's toughness as it last existed on the
        /// battlefield (CR 608.2h), captured when the cost was paid. The
        /// events log used to be scanned for it at resolution, but
        /// `submit_action` clears events per action, and the resolving pass
        /// is a later action than the activation — the scan found nothing
        /// and Disciple of Griselbrand gained 0 life in any real game
        /// (issue #141).
        #[serde(default)]
        sacrificed_toughness: Option<i32>,
        /// True for a planeswalker loyalty ability (CR 606.5): it resolves
        /// through `on_loyalty_ability` rather than
        /// `resolve_activated_ability`. Loyalty abilities used to resolve on
        /// the spot at activation, which meant Liliana's -6 built its pile of
        /// "all permanents target player controls" while Liliana — about to
        /// die to the loyalty cost she just paid — was still on the
        /// battlefield, and the pile prompt offered a permanent that no
        /// longer existed by the time anyone could answer it.
        #[serde(default)]
        loyalty: bool,
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
    /// Ordered by id, not hashed: `HashMap`'s iteration order is seeded per
    /// process, so two identical seeded runs serialized the same game to
    /// byte-different saves and `cmp`/`sha256sum` could not confirm they
    /// matched (issue #199). Same reason `ManaPool` is a `BTreeMap`. Also
    /// makes every iteration over the objects deterministic rather than
    /// merely usually-consistent.
    pub objects: std::collections::BTreeMap<ObjectId, GameObject>,
    /// Monotonic counter for generating unique `ObjectIds`.
    pub next_object_id: u64,

    /// Every random decision the game makes, drawn from here.
    ///
    /// Coin flips (CR 705.2), "at random" choices (CR 701.9c), shuffling
    /// (CR 701.20a) and the opening roll all used to call `rand::thread_rng()`
    /// where they stood. That has two costs. A game was not a function of its
    /// inputs, so a bug seen once in a runner game could not be replayed. And
    /// nothing a card decided at random could be *tested*: Creepy Doll's
    /// "flip a coin, if you win destroy that creature" had a test that ran the
    /// hook fifty times and asserted that at least one run destroyed
    /// something, which is not the same claim.
    ///
    /// `GameState::new` starts from a fixed seed, so a state built in a test
    /// is reproducible by default. `setup_game` overwrites it from
    /// `GameConfig::rng_seed`, which a runner leaves as `None` to get a fresh
    /// game each time and sets to replay one.
    #[serde(default = "default_rng_seed")]
    pub rng_state: u64,

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

    /// Who activated the activated ability currently resolving (CR 602.2a).
    ///
    /// An ability's controller is fixed when it is put on the stack and does
    /// not follow its source. Reading the source's `controller` at resolution
    /// instead handed the ability to whoever had stolen the source in
    /// response — the case Olivia Voldaren's ruling is about: "if you activate
    /// Olivia Voldaren's last ability, and before that ability resolves you
    /// lose control of Olivia Voldaren, the ability will resolve with no
    /// effect."
    #[serde(default, skip)]
    pub resolving_ability_activator: Option<PlayerId>,

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
    pub num_spells_cast_this_turn: std::collections::BTreeMap<PlayerId, u32>,

    /// Spells cast last turn (saved at turn start for werewolf condition checking).
    #[serde(default)]
    pub num_spells_cast_last_turn: std::collections::BTreeMap<PlayerId, u32>,

    /// X value chosen for the most recently activated X-cost ability.
    /// Set by the engine before the ability goes on the stack; cards read this.
    #[serde(default)]
    pub last_activated_x_value: Option<u32>,
    /// The creature sacrificed to pay the activated ability now resolving.
    /// Threaded exactly like `last_activated_x_value`: set when the ability is
    /// activated, carried on the stack entry, restored on resolution.
    pub last_activated_sacrifice: Option<ObjectId>,
    /// Its toughness as it last existed on the battlefield (CR 608.2h),
    /// captured at cost payment — see `StackEntry::Ability::sacrificed_toughness`.
    #[serde(default)]
    pub last_activated_sacrifice_toughness: Option<i32>,

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

    /// How many actions have been submitted to reach this state. Two
    /// consecutive decision points are one action apart exactly when this
    /// moved by one — the game loop may pass on a player's behalf in
    /// between, and `events` only ever holds the last action's events.
    #[serde(default)]
    pub submit_seq: u64,

    /// When set, the game loop routes the passes and empty attack
    /// declarations it would make on a player's behalf through the decision
    /// callback, so an observer sees every submitted action and every
    /// event buffer. A runtime setting, not part of the game.
    #[serde(skip)]
    pub observe_every_submit: bool,

    /// Pending triggered abilities waiting to resolve, in APNAP order.
    /// Active player's triggers at the front (bottom of "stack"),
    /// non-active player's at the back (top). Resolved LIFO from the back.
    #[serde(default)]
    pub pending_triggers: Vec<crate::triggers::PendingTrigger>,

    /// Permanents whose entry is waiting on an enters-as-a-copy choice
    /// (CR 614.12b). `move_object` puts an object here instead of moving it
    /// when its controller has not been asked yet; the engine drains the
    /// queue before any player receives priority, and each answer completes
    /// that object's move. A queue rather than a single slot because a batch
    /// entry (Grimoire of the Dead returning every creature card in every
    /// graveyard) can need several answers, and one `awaiting_action` cannot
    /// hold them all.
    #[serde(default)]
    pub pending_entry_choices: Vec<ObjectId>,

    /// The controller a deferred entry was requested under, held until the
    /// entry actually happens (Grimoire of the Dead and Moldgraf Monstrosity
    /// return creatures under *their* controller, Fiend Hunter under the
    /// exiled creature's owner). Writing it while the card is still in a
    /// graveyard would contradict CR 108.4, which gives a card off the
    /// battlefield its owner as its controller.
    #[serde(default)]
    pub pending_entry_controllers: std::collections::BTreeMap<ObjectId, PlayerId>,

    /// Monotonic timestamp handed to each control-changing effect as it is
    /// created (CR 613.7a). Layer 2 applies them in timestamp order, so when
    /// one ends the permanent goes to whichever of the rest is latest — a
    /// question that cannot be answered from the order two separate lists
    /// happen to be in.
    #[serde(default)]
    pub next_effect_timestamp: u64,

    /// Layer 7b: effects that SET a permanent's power and/or toughness to a
    /// specific value (CR 613.4b) — Tree of Redemption's exchange. They apply
    /// on top of the printed (or characteristic-defining) values and never
    /// change them, which is what keeps the copy rules (CR 706.2) and zone
    /// changes (CR 400.7) honest without each reader having to remember a
    /// card that rewrites its own printed characteristics.
    #[serde(default)]
    pub set_pt_effects: Vec<SetPtEffect>,

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

    /// CR 514.3a: a cleanup step in which state-based actions were performed
    /// gives players priority, and once they all pass another cleanup step
    /// follows — repeating until one passes without anything happening. Set
    /// by the cleanup turn-based action when it opens that priority window;
    /// consumed by `advance_step`, which re-enters `Step::Cleanup` instead of
    /// starting the next turn.
    #[serde(default)]
    pub cleanup_repeat: bool,
}

/// Log level for game log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    /// Information only ONE player is entitled to see — a CR 701.18a "look
    /// at" (Delver's top card), or whether a fruitless library search had
    /// anything to find. Never shown to players (display_log is Info and
    /// above) and never written to the shared --log file either: in a
    /// hotseat game the log file is readable by both seats, and it used to
    /// carry these lines at Debug level (issue #119).
    Private = 0,
    /// Every priority pass, mana tap, etc.
    Debug = 1,
    /// Actions taken: land plays, phase changes.
    Info = 2,
    /// Important game events: spells cast/resolved, combat, life changes.
    Event = 3,
    /// Major game moments: turn changes, player death, game over.
    Milestone = 4,
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
    /// CR 611.2b/613.7a: an until-end-of-turn control change (Traitorous
    /// Blood). `controller` is who it hands the permanent to and `timestamp`
    /// is where it sits among the other control effects; the permanent's
    /// default controller is on the object itself, so this effect ending
    /// re-derives rather than restoring a snapshot.
    ChangeControl { target: ObjectId, controller: PlayerId, timestamp: u64 },
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
pub(crate) fn until_eot_object_target(effect: &TemporaryEffect) -> Option<ObjectId> {
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

/// The seed a game starts from when nobody chose one.
///
/// Any fixed value would do; this one is arbitrary. What matters is that it
/// is fixed, so a test that does not care about randomness still gets the
/// same game every run.
#[must_use]
pub fn default_rng_seed() -> u64 {
    0x2545_F491_4F6C_DD1D
}

impl GameState {
    /// CR 400.7: an until-end-of-turn effect created for a permanent that is
    /// no longer on the battlefield applies to nothing — the object it was
    /// about is gone, and the card in the graveyard is a new object that
    /// must not carry it back onto the battlefield. `move_object` drops the
    /// effects of a permanent as it leaves; this drops the ones an ability
    /// creates *after* its source has already left (Feral Ridgewolf's pump
    /// resolving after the wolf died in response — found by fuzzing).
    pub fn prune_effects_on_departed_objects(&mut self) {
        self.until_end_of_turn.retain(|e| match until_eot_object_target(e) {
            Some(id) => self.objects.get(&id).is_some_and(|o| o.zone == Zone::Battlefield),
            None => true,
        });
    }

    /// The next value from the game's random stream.
    ///
    /// SplitMix64: a state and an output function, both of which fit in the
    /// `u64` that is serialised with the rest of the game. The specific
    /// generator is not important — being *in* the game state is.
    pub fn next_random(&mut self) -> u64 {
        self.rng_state = self.rng_state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.rng_state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A random number below `bound`. Panics if `bound` is 0.
    pub fn random_below(&mut self, bound: u64) -> u64 {
        assert!(bound > 0, "random_below needs a positive bound");
        self.next_random() % bound
    }

    /// CR 705.2: flip a coin. True is "you win the flip".
    pub fn flip_coin(&mut self) -> bool {
        self.next_random() & 1 == 1
    }

    /// Shuffle in place (CR 701.20a).
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        // Fisher-Yates, so the shuffle is the game's own randomness rather
        // than `SliceRandom`'s thread-local one.
        for i in (1..items.len()).rev() {
            let j = usize::try_from(self.random_below(i as u64 + 1)).unwrap_or(0);
            items.swap(i, j);
        }
    }

    /// Choose up to `n` of `candidates` at random (CR 701.9c).
    ///
    /// Fewer than `n` candidates yields all of them, which is what "at random"
    /// does with a short list. Six cards say "at random" — Ghoulraiser,
    /// Charmbreaker Devils, Make a Wish, Moldgraf Monstrosity, Woodland
    /// Sleuth, Desperate Ravings — and this is the one place they each ask.
    #[must_use]
    pub fn choose_at_random(&mut self, candidates: &[ObjectId], n: usize) -> Vec<ObjectId> {
        let mut pool = candidates.to_vec();
        self.shuffle(&mut pool);
        pool.truncate(n);
        pool
    }

    /// Create a new game state for a given number of players.
    #[must_use]
    pub fn new(num_players: u8) -> Self {
        let players = (0..num_players)
            .map(|i| PlayerState::new(PlayerId(i)))
            .collect();

        Self {
            objects: std::collections::BTreeMap::new(),
            next_object_id: 1,
            rng_state: default_rng_seed(),
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
            resolving_ability_activator: None,
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
            num_spells_cast_this_turn: std::collections::BTreeMap::new(),
            num_spells_cast_last_turn: std::collections::BTreeMap::new(),
            last_activated_x_value: None,
            last_activated_sacrifice: None,
            last_activated_sacrifice_toughness: None,
            pending_ability_effect: None,
            pending_spell_cast: None,
            trigger_event_index: 0,
            submit_seq: 0,
            observe_every_submit: false,
            pending_triggers: Vec::new(),
            pending_entry_choices: Vec::new(),
            pending_entry_controllers: std::collections::BTreeMap::new(),
            next_effect_timestamp: 0,
            set_pt_effects: Vec::new(),
            pending_trigger_pushes_ap: Vec::new(),
            pending_trigger_pushes_nap: Vec::new(),
            pending_mulligan_bottoms: Vec::new(),
            mulligan_round_position: 0,
            mulligan_round_mulled: false,
            cleanup_repeat: false,
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
            base_controller: owner,
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
            cast_from_zone: None,
            instance_oracle_text: None,
            instance_continuous_effects: None,
            card_state: std::collections::BTreeMap::new(),
            counters: std::collections::BTreeMap::new(),
            regeneration_shields: 0,
            is_transformed: false,
            x_value: None,
            chosen_mode: None,
            abilities_activated_this_turn: std::collections::BTreeSet::new(),
            entering_copy_choice: EnterAsCopyChoice::Unasked,
            state_trigger_on_stack: false,
            attacked_on_turn: None,
            last_controller: None,
            token_face: None,
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
    /// empty: CR 111.4 says a token's name is its subtype(s) when the effect
    /// creating it does not name it — a "1/1 white Spirit creature token" is
    /// named `Spirit`, and the rule's own worked example, a "Goblin Scout
    /// creature token", is named `Goblin Scout`. No card in this set names a
    /// token, so the derived name is what every one of them gets.
    ///
    /// The derived name used to carry a literal `" Token"` suffix, which is
    /// not a characteristic any printed token has: it is the *name* that
    /// "creatures with the same name" (Sever the Bloodline) and "cards named"
    /// (Nevermore) compare, so a token could never match a card sharing its
    /// name. The word belongs to the renderer, and both the CLI and the LLM
    /// board now flag a token there.
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
        self.create_tokens_with_subtypes(1, name, owner, power, toughness,
            colors, card_types, keywords, subtypes, registry)
    }

    /// The same, for an effect that creates several at once.
    ///
    /// "Create thirteen 2/2 black Zombie creature tokens" is ONE event
    /// (CR 614.1b), so it meets a doubler once and is reported once. Calling
    /// the single-token helper in a loop gives the same total — Parallel
    /// Lives doubles each of the thirteen — but it is thirteen events and
    /// thirteen log lines, and the reader of a log wants the sentence the
    /// card wrote.
    #[allow(clippy::too_many_arguments)]
    pub fn create_tokens_with_subtypes(
        &mut self,
        count: u32,
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
        if count == 0 {
            return Vec::new();
        }
        // CR 111.4: the token's name is its subtype(s), unless the effect
        // gave it one.
        let derived;
        let name = if name.is_empty() && !subtypes.is_empty() {
            derived = subtypes.join(" ");
            derived.as_str()
        } else {
            name
        };

        // CR 614: a replacement effect may change how many tokens are created
        // (Parallel Lives). Two doublers compound, which falls out of running
        // the event through each in turn.
        let after = crate::replacement::apply(
            self,
            crate::replacement::ReplaceableEvent::CreatesTokens { controller: owner, count },
            registry,
        );
        let extra_copies = match after {
            Some(crate::replacement::ReplaceableEvent::CreatesTokens { count, .. }) =>
                count.saturating_sub(1),
            // Replaced entirely, or replaced with a different kind of event:
            // no tokens are created.
            _ => return Vec::new(),
        };

        let mut all_ids = Vec::new();

        // Create extra doubled copies first (cloning inputs).
        for _ in 0..extra_copies {
            let id = self.create_token_internal(name, owner, power, toughness,
                colors.clone(), card_types.clone(), keywords.clone(), subtypes.clone(), registry);
            all_ids.push(id);
        }
        // Create the final token, consuming the inputs.
        let described = format!("{power}/{toughness} {name}");
        let id = self.create_token_internal(name, owner, power, toughness,
            colors, card_types, keywords, subtypes, registry);
        all_ids.push(id);

        // The count a player reads is the count that entered, and it is
        // counted here rather than claimed by each caller. Seventeen cards
        // logged the number printed on them — "Army of the Damned created 13
        // tapped Zombie tokens" while 26 entered under Parallel Lives (CR
        // 614.1b) — and #92 fixed two of them by hand, which is why the other
        // fifteen were still wrong (issue #329). A doubler cannot desynchronise
        // a line derived from `all_ids`.
        self.log(LogLevel::Event, format!(
            "p{} created {} {described} token{}",
            owner.0, all_ids.len(), if all_ids.len() == 1 { "" } else { "s" }));

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
        let colors_printed = colors.clone();
        let keywords_printed = keywords.clone();
        let card_types_printed = card_types.clone();
        let subtypes_printed = subtypes.clone();
        let obj = GameObject {
            id,
            card_id: CardId(0), // sentinel for tokens
            name: name.to_string(),
            owner,
            controller: owner,
            base_controller: owner,
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
            cast_from_zone: None,
            instance_oracle_text: None,
            instance_continuous_effects: None,
            card_state: std::collections::BTreeMap::new(),
            counters: std::collections::BTreeMap::new(),
            regeneration_shields: 0,
            is_transformed: false,
            x_value: None,
            abilities_activated_this_turn: std::collections::BTreeSet::new(),
            chosen_mode: None,
            entering_copy_choice: EnterAsCopyChoice::Unasked,
            state_trigger_on_stack: false,
            attacked_on_turn: None,
            last_controller: None,
            // The token's printed half, frozen here. Everything an effect
            // grants it later goes into the vectors above, exactly as it does
            // for a card, and the two stay tellable apart.
            token_face: Some(TokenFace {
                name: name.to_string(),
                power: Some(power),
                toughness: Some(toughness),
                colors: colors_printed,
                keywords: keywords_printed,
                card_types: card_types_printed,
                subtypes: subtypes_printed,
            }),
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
        let Some(source) = self.get_object(source_id) else { return ObjectId(0) };
        // CR 707.8a: a copy of a permanent with its back face up shows that
        // face too. The copied characteristics below already come from the
        // face that is up; the flag is what makes every accessor agree.
        let source_transformed = source.is_transformed;
        let card_id = source.card_id;
        let is_legendary = source.is_legendary;

        // CR 706.2: a copy takes the *copiable* values — what is printed on the
        // face now showing, plus earlier copy effects — and nothing an effect
        // has since done to the permanent.
        //
        // That is exactly what the `printed_*_of` family answers, for a card
        // and for a token alike (the ruling's "if the copied creature is a
        // token, the token copies the original characteristics of that
        // token"). This used to be a second, hand-rolled copy of that logic —
        // read the face, else the object's own vectors — and the two drifted:
        // a token's vectors hold its grants as well as its printed types, so
        // copying a Zombie token that Olivia Voldaren had made a Vampire
        // produced a Vampire Zombie. One caller of one accessor cannot drift
        // from itself.
        let name = self.name_of(source_id, registry);
        let (power, toughness) = self.printed_pt_of(source_id, registry);
        let colors = self.printed_colors_of(source_id, registry);
        let keywords = self.printed_keywords_of(source_id, registry);
        let card_types = self.printed_card_types_of(source_id, registry);
        let subtypes = self.printed_subtypes_of(source_id, registry);

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
                obj.is_transformed = source_transformed;
            }
        }
        all_ids.into_iter().next().unwrap_or(ObjectId(0))
    }

    /// Remove an object from the game entirely — it ceased to exist
    /// (CR 111.7: a token outside the battlefield; CR 707.10: a copy of a
    /// spell that left the stack).
    ///
    /// Dropping it from `self.objects` is most of it, because every zone but
    /// one is derived from that map and so empties itself. The exception is a
    /// library, whose order is a list of object ids kept alongside; an id left
    /// there is a card that can be drawn and isn't — the draw comes up empty,
    /// the hand doesn't grow, and a player who should have decked out doesn't.
    pub fn cease_to_exist(&mut self, id: ObjectId) {
        self.objects.remove(&id);
        for player in &mut self.players {
            player.library_order.retain(|&other| other != id);
        }
    }

    /// Put an object into its owner's library (CR 401.1).
    ///
    /// A library is the one zone whose order the game tracks, and that order
    /// lives in `PlayerState::library_order` beside the objects themselves.
    /// `move_object` maintains the zone but not the order, so moving a card
    /// into `Zone::Library` by hand puts it in the zone at no position in it:
    /// never drawn, never milled, counted in no library size. Every effect
    /// that puts a card into a library goes through here and says where it
    /// lands — CR 701.15a and 701.17a spell out "on top" and "on the bottom"
    /// precisely because a library has an order and an effect must pick one.
    ///
    /// A token put here is listed like anything else. It ceases to exist as a
    /// state-based action (CR 111.7 / 704.5d), not the moment it arrives, and
    /// `sba::check_state_based_actions` takes it back out of the order when it
    /// does.
    pub fn put_into_library(
        &mut self,
        id: ObjectId,
        position: LibraryPosition,
        registry: &crate::cards::CardRegistry,
    ) {
        let Some(owner) = self.get_object(id).map(|o| o.owner) else { return };
        self.move_object(id, Zone::Library, registry);
        let library = &mut self.get_player_mut(owner).library_order;
        match position {
            LibraryPosition::Top => library.insert(0, id),
            LibraryPosition::Bottom => library.push(id),
        }
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
    /// Returns whether the object actually moved — see [`move_object`].
    pub fn move_object_under_control(
        &mut self,
        id: ObjectId,
        to: Zone,
        controller: PlayerId,
        registry: &crate::cards::CardRegistry,
    ) -> bool {
        self.move_object_inner(id, to, Some(controller), registry)
    }

    /// Returns whether the object actually moved. It is `false` for exactly
    /// one reason: an entry deferred because the permanent has a copy choice
    /// outstanding (CR 614.12b). A caller that writes anything about the
    /// permanent *after* the move has to ask, because otherwise it writes
    /// battlefield state onto a card that is still in a graveyard.
    ///
    /// Grimoire of the Dead did exactly that. It returned eleven creatures
    /// under its controller and stamped each one black Zombie; the one that
    /// was an Evil Twin never moved, and was left in its owner's graveyard
    /// carrying the other player as its controller and a Zombie subtype it
    /// had no business having — a CR 108.4 and CR 400.7 violation the
    /// fuzzer reported from fifteen seeds (issues #335-#349).
    pub fn move_object(&mut self, id: ObjectId, to: Zone, registry: &crate::cards::CardRegistry) -> bool {
        self.move_object_inner(id, to, None, registry)
    }

    fn move_object_inner(
        &mut self,
        id: ObjectId,
        to: Zone,
        under: Option<PlayerId>,
        registry: &crate::cards::CardRegistry,
    ) -> bool {
        // CR 614.12b: a permanent that chooses what to enter as makes that
        // choice as part of entering — before it is on the battlefield, not
        // from a trigger afterwards. If the choice has not been made, this
        // entry is deferred: the object stays where it is, the engine asks
        // its controller before anyone receives priority, and the answer
        // re-runs this move. Nothing ever sees the permanent on the
        // battlefield as its printed self with the choice outstanding.
        if to == Zone::Battlefield
            && self.objects.get(&id).is_some_and(|o| o.zone != Zone::Battlefield)
            && self.objects.get(&id)
                .is_some_and(|o| o.entering_copy_choice == EnterAsCopyChoice::Unasked)
            && registry.get(self.objects.get(&id).map_or(CardId(0), |o| o.card_id))
                .is_some_and(super::cards::CardBehavior::chooses_copy_as_it_enters)
        {
            if !self.pending_entry_choices.contains(&id) {
                self.pending_entry_choices.push(id);
            }
            // The control change is part of the entry, so it waits with it.
            // Written now, it would sit on a card in a graveyard, where CR
            // 108.4 says the owner is the controller.
            if let Some(c) = under {
                self.pending_entry_controllers.insert(id, c);
            }
            return false;
        }
        if let Some(c) = under {
            if let Some(obj) = self.get_object_mut(id) {
                obj.controller = c;
            }
        }

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
        // A library's order is kept alongside the zone, so leaving the library
        // means leaving that order too. Doing it here rather than at the call
        // site is the only way it cannot be forgotten: a card left listed in a
        // library it is no longer in is drawn as a card that isn't there.
        if from == Some(Zone::Library) {
            let owner = self.objects.get(&id).map_or(PlayerId(0), |o| o.owner);
            self.get_player_mut(owner).library_order.retain(|&other| other != id);
        }
        // The same for the graveyard's own order (CR 404.2), kept here for
        // the same reason: a pile that is only ordered when someone remembers
        // to order it is not a pile.
        {
            let owner = self.objects.get(&id).map_or(PlayerId(0), |o| o.owner);
            if from == Some(Zone::Graveyard) && to != Zone::Graveyard {
                self.get_player_mut(owner).graveyard_order.retain(|&other| other != id);
            }
            if to == Zone::Graveyard && from != Some(Zone::Graveyard) {
                let pile = &mut self.get_player_mut(owner).graveyard_order;
                pile.retain(|&other| other != id);
                pile.push(id);
            }
        }
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
        // Asked once and carried, because the same question decides both
        // halves of entering: whether the event is planned here (before the
        // move, per CR 616.1) and whether the plan is applied below. Written
        // out twice, the two could disagree — and a looser copy here plans an
        // entry for a card going to a graveyard, running every replacement
        // effect in the game against an event that is not happening.
        let is_entering = to == Zone::Battlefield
            && from.is_some_and(|z| z != Zone::Battlefield);
        let entering = if is_entering {
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
                // CR 400.7 with 602.5b/606.3: the record of which of this
                // permanent's abilities were used this turn belongs to the
                // object that leaves; what comes back has used nothing. It
                // was cleared only at turn start, so a Darkthicket Wolf or a
                // planeswalker that used its ability and died stayed locked
                // out after a same-turn reanimation.
                obj.abilities_activated_this_turn.clear();
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
                // `card_types` and `keywords` are written by the two copy
                // paths (`become_copy_of`, Evil Twin's copy handler), so they
                // are runtime grants too: a dead Evil Twin that had copied
                // Manor Gargoyle answered "artifact" from the graveyard, and
                // a reanimation brought the copied types and keywords back.
                // The legendary flag is the same kind of cache — set by the
                // resolve path or a copy — and follows the printed card on
                // the way out (CR 400.7).
                if !obj.is_token {
                    obj.subtypes.clear();
                    obj.colors.clear();
                    obj.card_types.clear();
                    obj.keywords.clear();
                    obj.is_legendary = false;
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
                // `name` and the printed P/T, so those are written back from
                // the printed card, which is where a copy left them.
                obj.is_transformed = false;
                if let Some((name, power, toughness)) = printed_reset {
                    obj.name = name;
                    obj.power = power;
                    obj.toughness = toughness;
                }
                obj.instance_continuous_effects = None;
                obj.instance_oracle_text = None;
            }

            // Clear the marks left by a cast when moving back to hand or
            // library (e.g. Runic Repetition returns an exiled flashback card
            // to hand). They are set during cast and would otherwise persist.
            if matches!(to, Zone::Hand | Zone::Library) {
                obj.cast_with_flashback = false;
                obj.cast_from_zone = None;
            }

            // CR 107.3b: X is a value chosen for one particular cast. It means
            // something in exactly two places — on the stack while that cast is
            // resolving, and on the permanent that cast produced — and nowhere
            // else. A permanent that leaves and comes back is a new object that
            // was never cast at all (CR 400.7), so its X is 0.
            //
            // Without this, Mikaeus, the Lunarch cast for X=5, killed, and
            // reanimated came back with five +1/+1 counters: `x_value` sat on
            // the object through the graveyard and its enters-with-counters
            // replacement read it again on the way back in.
            if !matches!(to, Zone::Battlefield | Zone::Stack) {
                obj.x_value = None;
            }

            // Set summoning sickness and clear stale state when entering the battlefield.
            if to == Zone::Battlefield && from != Zone::Battlefield {
                obj.card_state.clear();
                obj.last_attached_to_player = None;
                obj.summoning_sick = true;
                // CR 110.2a: whoever put it onto the battlefield is its
                // controller by default, and stays so under every control
                // effect that later ends.
                obj.base_controller = obj.controller;
            }

            // CR 400.7: leaving the battlefield makes this a new object, and a
            // new object has not been asked what to enter as. An Evil Twin
            // that entered as a copy, died, and is reanimated chooses again —
            // and one whose controller declined must not stay declined for the
            // rest of the game.
            if to != Zone::Battlefield {
                obj.entering_copy_choice = EnterAsCopyChoice::Unasked;
            }
        }

        // CR 506.4c: a creature that leaves the battlefield is removed from
        // combat. The damage step tolerates this on its own (it snapshots the
        // combat state and skips creatures the live state no longer lists),
        // but the live state must actually drop the id: object ids survive
        // zone changes, so a dead blocker left in `blocker_assignments`
        // becomes whatever that id is next — Grimoire of the Dead reanimating
        // a dead blocker mid-combat under the attacking player produced a
        // "blocker" the defending player didn't control.
        if from == Some(Zone::Battlefield) && to != Zone::Battlefield {
            self.remove_from_combat(id);
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
            // The same rule for "for as long as" control effects. Object ids
            // survive zone changes, so an entry left behind for a stolen
            // creature that died named whatever came back under that id — a
            // reanimated creature (Grimoire of the Dead, under its thief) was
            // handed to its original controller when the thief's source later
            // left. The source side stays: `expire_control_effects` needs the
            // entry to hand the permanent back when the source leaves.
            self.control_effects.retain(|c| c.object != id);
            // A layer-7b set-P/T effect is on the permanent, so it ends with
            // it (CR 400.7). The exchanged toughness used to be written into
            // the printed field and followed the card to the graveyard.
            self.set_pt_effects.retain(|e| e.object != id);
        }

        // Emit zone-change events outside the mutable borrow. Every move is
        // announced (CR 400.7) — a library reorder included, since it counts
        // as a zone change here — so a ledger can pair each change of zone
        // with the verb that caused it.
        if let Some(from_zone) = from {
            self.events.push(crate::events::GameEvent::ObjectMoved { object: id, from: from_zone, to });
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
            if is_entering {
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
        true
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
        if let Some(source) = entering.copy_of {
            self.become_copy_of(id, source, registry);
        }
        // CR 614.1c: the counters are there the instant the permanent enters,
        // and the count is the whole card for Mikaeus ("enters with X +1/+1
        // counters") and Unbreathing Horde. The log said nothing about them,
        // so a permanent entering with 0 counters — a 0/0 that dies to CR
        // 704.5f at the next SBA check — was recorded as "resolved" then
        // "died" with no cause anywhere in between (issue #299). A card whose
        // replacement does not apply at all (Somberwald Spider with no
        // creature dead this turn) records no entry, and so still says
        // nothing.
        for (counter_type, count) in &entering.counters {
            if *count > 0 {
                self.add_counters(id, *counter_type, *count);
            }
        }
        if !entering.counters.is_empty() {
            let name = self.obj_name(id);
            let what = entering.counters.iter()
                .map(|(t, n)| format!("{n} {t} counter{}", if *n == 1 { "" } else { "s" }))
                .collect::<Vec<_>>()
                .join(", ");
            self.log(LogLevel::Info, format!("{name} enters with {what}"));
        }
        if entering.tapped {
            self.arrives_tapped(id);
            let name = self.obj_name(id);
            self.log(LogLevel::Info, format!("{name} enters tapped"));
        }
    }

    /// Give `dest` the copiable values of the permanent `source` (CR 706.2).
    ///
    /// Copiable values are what is printed on the card *as modified by other
    /// copy effects* — not what non-copy effects have since done to it. That
    /// distinction is exactly the face-data / object-vector split: the
    /// `printed_*` accessors read the face when there is one and fall back to
    /// the object only for a token, which has none. Reading the object
    /// vectors directly would copy Olivia Voldaren's granted "Vampire" and
    /// Grimoire of the Dead's granted black, which the Evil Twin ruling says
    /// a copy does not get.
    ///
    /// Taking the *permanent* rather than a `CardId` is what makes a copy of
    /// a copy right: an object that is itself a copy carries the copied
    /// card's id, so its copiable values are read straight off it.
    pub(crate) fn become_copy_of(
        &mut self,
        dest: ObjectId,
        source: ObjectId,
        registry: &crate::cards::CardRegistry,
    ) {
        let Some(src) = self.get_object(source) else { return };
        let source_card = src.card_id;
        // CR 707.8: copying a transformed permanent copies the face that is
        // up, and the copy shows that face.
        let transformed = src.is_transformed;
        // Legendary is copiable (CR 707.2): the object flag, or the printed
        // supertype when nothing has set one.
        let legendary = src.is_legendary
            || self.face_data(source, registry)
                .is_some_and(|d| d.supertypes.contains(&crate::types::Supertype::Legendary));
        let name = self.name_of(source, registry);
        let (power, toughness) = self.printed_pt_of(source, registry);
        let keywords = self.printed_keywords_of(source, registry);
        let card_types = self.printed_card_types_of(source, registry);
        let subtypes = self.printed_subtypes_of(source, registry);
        let colors = self.printed_colors_of(source, registry);
        let oracle_text = self.face_data(source, registry).map(|d| d.oracle_text.clone());

        let old_name = self.get_object(dest).map(|o| o.name.clone()).unwrap_or_default();
        if let Some(obj) = self.get_object_mut(dest) {
            // CR 706.2: `card_id` is what every ability, trigger and
            // replacement lookup reads, so it is what makes this object a
            // copy. The card it is printed as goes into `copy_grantor`: it is
            // where `move_object` looks to give the printed card back on the
            // way out, and where a copy effect with an "except it has ..."
            // clause (Evil Twin) is found.
            if obj.card_id != source_card {
                obj.copy_grantor = Some(obj.card_id);
                obj.card_id = source_card;
            }
            obj.name.clone_from(&name);
            obj.power = power;
            obj.toughness = toughness;
            obj.colors = colors;
            obj.card_types = card_types;
            obj.subtypes = subtypes;
            obj.keywords = keywords;
            obj.is_legendary = legendary;
            obj.is_transformed = transformed;
            // The copy has no instance effects of its own: whatever the
            // pre-copy object was given is gone, and the copied card's static
            // abilities come from its face (CR 706.2).
            obj.instance_continuous_effects = None;
            obj.instance_oracle_text = oracle_text;
        }
        let pt = match (power, toughness) {
            (Some(p), Some(t)) => format!(" ({p}/{t})"),
            _ => String::new(),
        };
        self.log(LogLevel::Event, format!("{old_name} enters as a copy of {name}{pt}"));
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
        // CR 404.2: a graveyard is an ordered pile, and its order is arrival
        // order — not the order the cards happened to be created in when the
        // decklist was built, which is what sorting by id gives (issue #222).
        // Anything not in the recorded pile (a card put there by a path that
        // predates the order) sorts after what is, by id.
        if zone == Zone::Graveyard {
            let pile = &self.get_player(player).graveyard_order;
            result.sort_by_key(|o| (pile.iter().position(|&id| id == o.id).unwrap_or(usize::MAX), o.id));
        } else {
            result.sort_by_key(|o| o.id);
        }
        result
    }

    /// Every object in the game, in object-id order.
    ///
    /// `self.objects` is a `HashMap`, and its iteration order is seeded per
    /// process: a scan that stops at the first match, or whose result is
    /// offered to a player as a list, gives a different answer on a replay of
    /// the same game. Anything that cares about order goes through this or
    /// through `objects_in_zone` / `all_objects_in_zone`, which sort the same
    /// way. `card_data_invariants::nothing_iterates_the_object_map_in_map_order`
    /// keeps card code out of the raw map.
    #[must_use]
    pub fn objects_in_id_order(&self) -> Vec<&GameObject> {
        let mut result: Vec<_> = self.objects.values().collect();
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

    /// Check if a creature matches a `CreatureFilter`, evaluated from the
    /// perspective of the effect's source permanent.
    ///
    /// `source_id` is that permanent. Most filters need only its controller,
    /// but `ControlledByAttachedPlayer` needs the object itself — a Curse's
    /// "creatures enchanted player controls" is a question about whom the
    /// Curse is on, and there is no answering it from a `PlayerId`. This arm
    /// used to fall back to "creatures controlled by anyone who isn't the
    /// source's controller", with a comment saying so; that is a different
    /// set the moment a Curse is put on the player who controls it, which
    /// "Enchant player" allows.
    #[must_use]
    pub fn matches_filter(
        &self,
        creature_id: ObjectId,
        filter: &crate::types::CreatureFilter,
        source_id: ObjectId,
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
            CreatureFilter::And(filters) => filters.iter().all(|f| self.matches_filter(creature_id, f, source_id, source_controller, registry)),
            CreatureFilter::Or(filters) => filters.iter().any(|f| self.matches_filter(creature_id, f, source_id, source_controller, registry)),
            CreatureFilter::Not(inner) => !self.matches_filter(creature_id, inner, source_id, source_controller, registry),
            CreatureFilter::Attacking => {
                self.combat.as_ref().is_some_and(|c| c.attackers.contains_key(&creature_id))
            }
            CreatureFilter::Blocking => {
                self.combat.as_ref().is_some_and(|c|
                    c.blocker_assignments.values().any(|bs| bs.contains(&creature_id)))
            }
            CreatureFilter::ControlledByAttachedPlayer => {
                self.get_object(source_id)
                    .and_then(|o| o.attached_to_player)
                    .is_some_and(|player| creature.controller == player)
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
                self.matches_filter(creature_id, filter, source_id, source_controller, registry)
            }
            EffectScope::GlobalOther(filter) => {
                creature_id != source_id
                    && self.matches_filter(creature_id, filter, source_id, source_controller, registry)
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

    /// CR 604.3: the P/T a token's *own* ability defines, when the ability
    /// came from the effect that created it rather than from a card face.
    ///
    /// The engine holds only the link. The token records its creator under
    /// [`crate::cards::PT_DEFINED_BY`] and that object's card says what the
    /// numbers are, so the rule stays on the card that prints it. `None` when
    /// the token carries no such ability, or when its creator has left the
    /// game entirely — the token's own base P/T (0/0) is then the answer,
    /// which is the same one its creator would give with its counters gone.
    fn token_defined_pt(&self, obj: &GameObject, registry: &crate::cards::CardRegistry) -> Option<(i32, i32)> {
        let source_id = *obj.card_state.get(crate::cards::PT_DEFINED_BY)?;
        let source = self.get_object(source_id)?;
        registry.get(source.card_id)?
            .token_dynamic_pt(self, source_id, obj.id, registry)
    }

    /// Get the effective power of a creature, including continuous effects,
    /// dynamic P/T, counters, and "until end of turn" effects.
    #[must_use]
    pub fn effective_power(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Option<i32> {
        let obj = self.get_object(id)?;

        // A token whose own ability defines its P/T (CR 604.3) has no card face
        // to declare `dynamic_pt` on, so the object that created it answers.
        let mut power = if let Some((p, _)) = self.token_defined_pt(obj, registry) {
            p
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

        // Everything below layer-7c-and-friends applies to PERMANENTS: an
        // anthem reads "creatures you control", counters sit on permanents,
        // and until-end-of-turn pumps target permanents (CR 613.1). A card in
        // hand or the graveyard shows its printed P/T plus CDAs only (the
        // dynamic_pt above — CR 604.3 makes those work in every zone). Hand
        // cards used to grow with battlefield anthems (issue #57).
        if obj.zone != Zone::Battlefield {
            return Some(power);
        }

        // Layer 7b: an effect that SETS power (CR 613.4b) applies over the
        // base and under everything that modifies it.
        if let (Some(set), _) = self.set_pt_for(id) {
            power = set;
        }

        // Continuous effects (auras, anthems, debuffs — including dynamic aura P/T).
        let (p_mod, _) = self.continuous_pt_mods(id, registry);
        power += p_mod;

        // The card's OWN "gets +N/+N for each ..." static ability — an
        // ordinary layer-7c modification, so battlefield only, unlike the
        // all-zone CDA dynamic_pt above (issue #105).
        if let Some(behavior) = registry.get(obj.card_id) {
            if let Some((p, _)) = behavior.self_static_pt_mod(self, id, registry) {
                power += p;
            }
        }

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

        // See `effective_power` — the same CR 604.3 token ability.
        let mut toughness = if let Some((_, t)) = self.token_defined_pt(obj, registry) {
            t
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

        // Same zone guard as effective_power: only a permanent gets anthem,
        // counter, and until-end-of-turn modifications (CR 613.1); CDAs above
        // already applied in every zone (CR 604.3).
        if obj.zone != Zone::Battlefield {
            return Some(toughness);
        }

        // Layer 7b: an effect that SETS toughness applies over the base and
        // under everything that modifies it (CR 613.4b), which is why a Tree
        // of Redemption exchanged at 5 life with two +1/+1 counters is a 2/7
        // and not a 2/5.
        if let (_, Some(set)) = self.set_pt_for(id) {
            toughness = set;
        }

        let (_, t_mod) = self.continuous_pt_mods(id, registry);
        toughness += t_mod;

        // Own "gets +N/+N for each ..." static ability — see effective_power.
        if let Some(behavior) = registry.get(obj.card_id) {
            if let Some((_, t)) = behavior.self_static_pt_mod(self, id, registry) {
                toughness += t;
            }
        }

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

    /// Set a permanent's power and/or toughness to a specific value in layer
    /// 7b (CR 613.4b).
    ///
    /// Writing `obj.power` / `obj.toughness` instead would change the object's
    /// *printed* characteristics, which a copy effect reads (CR 706.2) and a
    /// zone change resets (CR 400.7) — Tree of Redemption's exchange did that,
    /// and both of those readers had to be patched by hand to compensate
    /// (issue #302).
    pub fn set_base_pt(&mut self, object: ObjectId, power: Option<i32>, toughness: Option<i32>) {
        let timestamp = self.next_control_timestamp();
        self.set_pt_effects.push(SetPtEffect { object, power, toughness, timestamp });
    }

    /// The layer-7b value in force for `object`, if any (CR 613.7a: latest
    /// timestamp wins).
    #[must_use]
    fn set_pt_for(&self, object: ObjectId) -> (Option<i32>, Option<i32>) {
        let mut power = None;
        let mut toughness = None;
        let mut applicable: Vec<&SetPtEffect> = self.set_pt_effects.iter()
            .filter(|e| e.object == object)
            .collect();
        applicable.sort_by_key(|e| e.timestamp);
        for effect in applicable {
            if effect.power.is_some() { power = effect.power; }
            if effect.toughness.is_some() { toughness = effect.toughness; }
        }
        (power, toughness)
    }

    /// Check if a creature is prevented from attacking (e.g., by Pacifism).
    #[must_use]
    pub fn can_attack(&self, creature_id: ObjectId, registry: &crate::cards::CardRegistry) -> bool {
        // Conditional "can't attack" (Bonds of Faith on a non-Human) comes
        // through the same query — one walk, not two.
        !self.has_effect(creature_id, &|e| matches!(e, ContinuousEffect::PreventAttack { .. }), registry)
    }

    /// Whether anything says this creature can't block.
    ///
    /// Both halves: a printed or granted static ability (Vampire Interloper,
    /// Bonds of Faith) and a "can't block this turn" effect (Nightbird's
    /// Clutches, Crossway Vampire). This used to answer only the first, and
    /// `combat::can_block_at_all` scanned `until_end_of_turn` for the second
    /// itself — so the question with the name lived in one place and the
    /// answer in two, and anything that asked `can_block` directly got half of
    /// it.
    #[must_use]
    pub fn can_block(&self, creature_id: ObjectId, registry: &crate::cards::CardRegistry) -> bool {
        if self.has_effect(creature_id, &|e| matches!(e, ContinuousEffect::PreventBlock { .. }), registry) {
            return false;
        }
        !self.until_end_of_turn.iter().any(|e| matches!(e,
            TemporaryEffect::CantBlock { target } if *target == creature_id))
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

    /// Every protection in force on `id`, described (CR 702.16).
    ///
    /// Protection is not a `Keyword` — it lives in `ContinuousEffect` and
    /// `TemporaryEffect` with a filter attached — so no pane could render it
    /// even in principle, while the engine acted on it in three places at
    /// once (issue #243). This is the one reader that turns it back into
    /// something a player can be shown.
    #[must_use]
    pub fn protections_of(
        &self,
        id: ObjectId,
        registry: &crate::cards::CardRegistry,
    ) -> Vec<String> {
        use crate::types::ContinuousEffect;
        let mut out: Vec<String> = Vec::new();
        self.walk_effects(
            id,
            &|e| matches!(e,
                ContinuousEffect::ProtectionFrom { .. } | ContinuousEffect::ProtectionFromSubtype { .. }),
            registry,
            &mut |e, _src| {
                match e {
                    ContinuousEffect::ProtectionFrom { filter, .. } =>
                        out.push(format!("protection from {}", filter.describe())),
                    ContinuousEffect::ProtectionFromSubtype { subtype, .. } =>
                        out.push(format!("protection from {}", crate::types::plural_of(subtype))),
                    _ => {}
                }
                true
            },
        );
        for effect in &self.until_end_of_turn {
            if let TemporaryEffect::GrantProtection { target, filter } = effect {
                if *target == id {
                    out.push(format!("protection from {}", filter.describe()));
                }
            }
        }
        out.dedup();
        out
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
                    if self.matches_filter(source_id, filter, src_obj.id, src_obj.controller, registry) {
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
                    // An until-end-of-turn grant has no permanent behind it any
                    // more (Spare from Evil is a spell that has resolved), so
                    // the protected creature stands in as the source. Only
                    // `ControlledByAttachedPlayer` would notice, and no grant
                    // in the set uses it — a Curse's filter is a static
                    // ability, not a grant.
                    if self.matches_filter(source_id, filter, target_id, target_controller, registry) {
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
                // Just ask. This used to re-check `until_end_of_turn` for a
                // `RemoveKeyword` on `kw` first, which `has_keyword` already
                // does as its very first step — two copies of "was this
                // keyword removed this turn", free to disagree, and neither
                // one a guard against the recursion the walk order handles.
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

    /// CR 701.20a: untap a permanent, emitting `Untapped`.
    ///
    /// The one place an *effect* clears `tapped`, so the event is emitted the
    /// same way wherever the untap comes from — the untap step, "Untap target
    /// creature", Grimgrin's activated ability, Village Bell-Ringer's ETB.
    /// Six cards wrote the field by hand and none of them emitted anything;
    /// nothing in this pool watches for an untap yet, and the first card that
    /// does would have seen five sixths of them.
    ///
    /// Not for a permanent *leaving* the battlefield, where the flag is reset
    /// because CR 400.7 makes it a new object rather than because anything
    /// untapped it, and not for one entering tapped, which was never untapped.
    pub fn untap(&mut self, id: ObjectId) {
        match self.get_object_mut(id) {
            Some(obj) if obj.tapped => obj.tapped = false,
            _ => return,
        }
        self.events.push(crate::events::GameEvent::Untapped { object: id });
    }

    /// Tap a permanent (CR 701.21a), emitting `Tapped`.
    ///
    /// "Only untapped permanents can be tapped", so tapping one that is
    /// already tapped does nothing at all — not even an event. That is the
    /// half a bare `obj.tapped = true` gets wrong: the field is already true,
    /// so the write is invisible, but the event it should not have sent is
    /// not.
    ///
    /// This is for a permanent *becoming* tapped. A permanent that arrives on
    /// the battlefield tapped was never untapped there and is not tapped by
    /// anything — see [`GameState::arrives_tapped`].
    pub fn tap(&mut self, id: ObjectId) {
        match self.get_object_mut(id) {
            // CR 110.5: tapped is a status of permanents. An effect resolving
            // through last-known information can name an object that has
            // since left the battlefield (Claustrophobia's enters-tap after
            // the enchanted creature bounced itself); tapping applies to
            // nothing then — the object in its new zone is a new object, and
            // a card in a hand cannot be tapped.
            Some(obj) if obj.zone == crate::types::Zone::Battlefield && !obj.tapped => {
                obj.tapped = true;
            }
            _ => return,
        }
        self.events.push(crate::events::GameEvent::Tapped { object: id });
    }

    /// A permanent arrives on the battlefield tapped.
    ///
    /// The counterpart to [`GameState::tap`] and deliberately not the same
    /// thing. "Create a 2/2 Wolf that's tapped and attacking", "search for a
    /// basic land and put it onto the battlefield tapped", and every
    /// enters-tapped replacement (CR 614.1c) describe the state a permanent
    /// is in when it arrives — nothing tapped it, so nothing that watches for
    /// a permanent becoming tapped should see it.
    ///
    /// The two are one field and two events' worth of difference, which is
    /// exactly why they are two named operations rather than one assignment
    /// spelled the same way in sixteen places.
    pub fn arrives_tapped(&mut self, id: ObjectId) {
        if let Some(obj) = self.get_object_mut(id) {
            obj.tapped = true;
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
        let changed = match self.get_object_mut(id) {
            Some(obj) if obj.controller != new_controller => {
                obj.controller = new_controller;
                obj.summoning_sick = true;
                true
            }
            _ => false,
        };
        // CR 506.4d: an attacking or blocking creature whose controller
        // changes is removed from combat — a stolen attacker deals no combat
        // damage for its old controller's attack.
        if changed {
            self.remove_from_combat(id);
        }
    }

    /// A player loses the game (CR 104.3): record why, say so in the log,
    /// and raise the event. The single chokepoint, so no loss is ever
    /// silent — `LossReason` used to be constructed at four sites and then
    /// discarded, leaving the log with no line for the loss and the result
    /// naming only the winner (issue #86).
    /// End the game with `result`, announcing it (CR 104.2). Every path that
    /// decides a game goes through here so the record always carries the
    /// `GameEnded` event the state-based check would have written.
    pub fn end_game(&mut self, result: GameResult) {
        if self.result.is_some() {
            return;
        }
        self.events.push(crate::events::GameEvent::GameEnded { result: result.clone() });
        self.result = Some(result);
    }

    pub fn player_loses(&mut self, player: PlayerId, reason: crate::events::LossReason) {
        if self.get_player(player).lost {
            return;
        }
        self.get_player_mut(player).lost = true;
        self.get_player_mut(player).loss_reason = Some(reason);
        self.log(LogLevel::Milestone, format!("p{} {}", player.0, reason.describe()));
        self.events.push(crate::events::GameEvent::PlayerLost { player, reason });
    }

    /// Remove a creature from the current combat (if any) — regeneration
    /// (CR 701.15c), control changes (CR 506.4d), and anything else that
    /// pulls a creature out of combat.
    ///
    /// Every piece of combat bookkeeping that names the creature goes with
    /// it — a removed attacker left in `blocked_attackers` is a creature the
    /// combat state says was blocked but never says attacked. (CR 509.2's
    /// "blocked forever" is about an attacker whose *blockers* leave; a
    /// creature removed from combat is not an attacker at all, CR 506.4c.)
    pub fn remove_from_combat(&mut self, id: ObjectId) {
        if let Some(ref mut combat) = self.combat {
            combat.attackers.remove(&id);
            combat.blocker_assignments.remove(&id);
            combat.blocked_attackers.remove(&id);
            combat.planeswalker_defenders.remove(&id);
            combat.dealt_first_strike.remove(&id);
            for blockers in combat.blocker_assignments.values_mut() {
                blockers.retain(|&b| b != id);
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
        if self.get_object(object).is_none() {
            return;
        }
        let _ = registry;
        // The effect exists even when the object is already yours. Nothing in
        // CR 611/613 makes a redundant layer-2 effect fail to happen, and the
        // "gain control ... for as long as" template is specifically the
        // durable half of a pair: steal a creature with Traitorous Blood
        // ("until end of turn"), then point Olivia Voldaren at it, and when
        // the temporary effect ends in the cleanup step the later-timestamped
        // one is still there. Returning early on
        // `controller == source_controller` made that line a five-mana no-op
        // whose log claimed a steal that had not happened (issue #253).
        let original_controller = self.base_controller(object).unwrap_or(source_controller);
        // Re-activating on the same object under the same source is the same
        // effect with a fresh timestamp, not a second one to unwind later.
        self.control_effects.retain(|e| !(e.object == object && e.source == source));
        let timestamp = self.next_control_timestamp();
        self.control_effects.push(ControlEffect {
            object,
            controller: source_controller,
            original_controller,
            source,
            source_controller,
            timestamp,
        });
        // Layer 2 is re-derived from every effect in force, so a permanent
        // already under a later-timestamped effect stays where it is.
        if let Some(controller) = self.derived_controller(object) {
            self.change_control(object, controller);
        }
    }

    /// Take the next control-effect timestamp (CR 613.7a).
    pub fn next_control_timestamp(&mut self) -> u64 {
        self.next_effect_timestamp += 1;
        self.next_effect_timestamp
    }

    /// Who a permanent goes back to once every control-changing effect on it
    /// has ended: its default controller (CR 110.2a), the player who put it
    /// onto the battlefield.
    #[must_use]
    pub fn base_controller(&self, object: ObjectId) -> Option<PlayerId> {
        self.get_object(object).map(|o| o.base_controller)
    }

    /// Who controls `object` according to the control-changing effects
    /// currently in force (CR 613.7a): the latest-timestamped one that
    /// applies, or its default controller when none does.
    ///
    /// This is the question every "the effect ended, now what?" path has to
    /// ask. Each such path used to answer it from a snapshot the effect took
    /// when it was created, which is not the same question: a Traitorous
    /// Blood cast on a creature Olivia Voldaren had stolen recorded "give it
    /// back to the thief", and when Olivia died first the cleanup step handed
    /// the creature to a player with no effect on it at all — permanently
    /// (issue #285).
    #[must_use]
    pub fn derived_controller(&self, object: ObjectId) -> Option<PlayerId> {
        let durable = self.control_effects.iter()
            .filter(|e| e.object == object)
            .map(|e| (e.timestamp, e.controller));
        let temporary = self.until_end_of_turn.iter()
            .filter_map(|e| match e {
                TemporaryEffect::ChangeControl { target, controller, timestamp } if *target == object =>
                    Some((*timestamp, *controller)),
                _ => None,
            });
        durable.chain(temporary)
            .max_by_key(|(timestamp, _)| *timestamp)
            .map(|(_, controller)| controller)
            .or_else(|| self.base_controller(object))
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
            if self.get_object(effect.object).is_none_or(|o| o.zone != Zone::Battlefield) {
                continue;
            }
            // With this effect gone, control is whatever the remaining ones
            // say (CR 613.7a) — an until-end-of-turn steal on the same
            // permanent keeps it until the cleanup step, rather than this
            // effect's own record of who had it first winning.
            let Some(controller) = self.derived_controller(effect.object) else { continue };
            let changing = self.get_object(effect.object)
                .is_some_and(|o| o.controller != controller);
            let name = self.obj_name(effect.object);
            self.change_control(effect.object, controller);
            if changing {
                self.log(LogLevel::Event, format!(
                    "{name} returns to p{}: the control effect's condition no longer holds",
                    controller.0));
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

    /// Create a regeneration shield on a permanent (CR 701.15).
    ///
    /// Like `add_counters`, this refuses anything that is not on the
    /// battlefield. Regeneration is a replacement effect on "the next time
    /// this permanent would be destroyed", and a permanent that has left is a
    /// different object (CR 400.7) which cannot be destroyed — so a shield
    /// aimed at one lands nowhere.
    ///
    /// Without the guard it landed on the graveyard object and stayed there:
    /// the cleanup step clears unused shields only from permanents *on the
    /// battlefield*, so a creature destroyed in response to its own
    /// "{B}: Regenerate this creature" kept the shield through the graveyard
    /// and came back from a reanimation with a free regeneration it never
    /// earned. Grimoire of the Dead, Unburial Rites and Moldgraf Monstrosity
    /// are all in this set.
    pub fn add_regeneration_shield(&mut self, id: ObjectId) {
        if self.objects.get(&id).is_none_or(|o| o.zone != Zone::Battlefield) {
            return;
        }
        if let Some(obj) = self.objects.get_mut(&id) {
            obj.regeneration_shields += 1;
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
        self.change_life_quiet(player, delta);
        if delta == 0 {
            return;
        }
        // Every life transition is logged HERE, with the resulting total —
        // per-card logging left non-combat life loss (Bump in the Night)
        // invisible in the log, so the totals couldn't be reconciled
        // (issue #129; #89 fixed the same for combat damage and lifelink).
        // Paths that write their own richer line with the total (the damage
        // pipeline) use `change_life_quiet`.
        let verb = if delta > 0 { "gained" } else { "lost" };
        let new_life = self.get_player(player).life;
        self.log(LogLevel::Info,
            format!("p{} {} {} life ({})", player.0, verb, delta.abs(), new_life));
    }

    /// `change_life` without the generic log line, for callers that log the
    /// change themselves with the running total (the damage pipeline's
    /// "took N combat damage (T)" and lifelink lines).
    pub fn change_life_quiet(&mut self, player: crate::ids::PlayerId, delta: i32) {
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

    /// Add mana to a player's pool and announce it (CR 106.4). Every source of
    /// mana — mana abilities and mana-producing spells alike — goes through
    /// here so the `ManaAdded` event is never skipped.
    pub fn add_mana(&mut self, player: crate::ids::PlayerId, mana_type: crate::types::ManaType, amount: u32) {
        self.get_player_mut(player).mana_pool.add(mana_type, amount);
        self.events.push(crate::events::GameEvent::ManaAdded { player, mana_type, amount });
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
        self.discard_card_inner(id, None, registry);
    }

    /// Discard a card, naming the effect that caused it: "Liliana +1: p0
    /// discarded Forest". Prefer this over [`GameState::discard_card`]
    /// wherever a source is known — a discard the log cannot attribute is a
    /// card leaving a hand for no visible reason.
    pub fn discard_card_for(
        &mut self,
        id: ObjectId,
        source: &str,
        registry: &crate::cards::CardRegistry,
    ) {
        self.discard_card_inner(id, Some(source), registry);
    }

    fn discard_card_inner(
        &mut self,
        id: ObjectId,
        source: Option<&str>,
        registry: &crate::cards::CardRegistry,
    ) {
        let Some(player) = self.get_object(id).map(|o| o.owner) else { return; };
        // A discard is public: the card moves into a public zone (CR 400.2),
        // and CR 701.8a's discard is not a hidden action. Logged HERE rather
        // than at each caller, because a caller that forgets loses the line
        // entirely — Desperate Ravings' random discard was the one path that
        // did, so a card moved hand→graveyard and the log said only "drew 2
        // cards" (issue #301).
        let name = self.obj_name(id);
        self.move_object(id, Zone::Graveyard, registry);
        let line = match source {
            Some(s) => format!("{s}: p{} discarded {name}", player.0),
            None => format!("p{} discarded {name}", player.0),
        };
        self.log(LogLevel::Event, line);
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
    // Grimoire of the Dead's "Zombie". A token has no registry face, so
    // `token_face` is its printed half: its object-level vectors start as a
    // copy of it and then accumulate grants exactly as a card's do. Reading
    // those vectors as a token's printed characteristics — which the
    // `printed_*_of` family used to do — hands a grant to a copy effect, which
    // takes copiable values only (CR 707.2).
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

    /// The creature an Aura or Equipment is attached to, falling back to the
    /// one it was attached to immediately before it left the battlefield.
    ///
    /// The counterpart of `attached_player`, and it exists for the same reason.
    /// CR 113.7a and CR 608.2g: an ability on the stack resolves even if its
    /// source is destroyed in response, and it still knows what that source was
    /// attached to. Claustrophobia's "when this Aura enters, tap enchanted
    /// creature" read `attached_to` directly, which the zone change clears —
    /// so destroying the Aura in response to its own enters trigger meant the
    /// creature was never tapped.
    ///
    /// The last known value is the one `move_object` stashes in
    /// `card_state["last_attached_to"]` on the way out.
    #[must_use]
    pub fn attached_creature(&self, id: ObjectId) -> Option<ObjectId> {
        let obj = self.get_object(id)?;
        obj.attached_to.or_else(|| obj.card_state.get("last_attached_to").copied())
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

    /// The printed half of a token — the characteristics the effect that
    /// created it gave it (CR 111.4, CR 707.2).
    ///
    /// This is what `face_data` is for a card, and the `printed_*_of` family
    /// consults it for the same reason: a token's object-level vectors carry
    /// its grants as well as its printed types, so they are not the printed
    /// half and must not be read as one.
    #[must_use]
    pub fn token_face_of(&self, id: ObjectId) -> Option<&TokenFace> {
        self.get_object(id).and_then(|o| o.token_face.as_ref())
    }

    /// Printed keywords of the object: the active face's, or a token's
    /// `token_face` — the keywords the effect that created it gave it.
    ///
    /// This is the printed set only — keywords granted by continuous or
    /// temporary effects are not included. Ask `has_keyword` for the full
    /// picture; use this when copying, which copies printed values (CR 707.2).
    #[must_use]
    pub fn printed_keywords_of(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Vec<crate::types::Keyword> {
        if let Some(data) = self.face_data(id, registry) {
            return data.keywords;
        }
        if let Some(face) = self.token_face_of(id) {
            return face.keywords.clone();
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
        if let Some(face) = self.token_face_of(id) {
            return face.card_types.clone();
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
        if let Some(face) = self.token_face_of(id) {
            return face.subtypes.clone();
        }
        self.get_object(id).map(|o| o.subtypes.clone()).unwrap_or_default()
    }

    /// Colors as printed — derived from the active face's mana cost — ignoring
    /// anything granted at runtime (Grimoire of the Dead's black). The copy
    /// counterpart of `colors_of`.
    #[must_use]
    pub fn printed_colors_of(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Vec<crate::types::Color> {
        if let Some(data) = self.face_data(id, registry) {
            // CR 204.2: a face with no mana cost says its colors with a color
            // indicator instead — every transforming back face in the set.
            if !data.color_indicator.is_empty() {
                return data.color_indicator;
            }
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
        if let Some(face) = self.token_face_of(id) {
            return face.colors.clone();
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
        if let Some(face) = self.token_face_of(id) {
            return (face.power, face.toughness);
        }
        self.get_object(id).map_or((None, None), |o| (o.power, o.toughness))
    }

    /// Whether this permanent is legendary (CR 205.4), read from the active
    /// face rather than from a flag.
    ///
    /// `obj.is_legendary` is a cache, and only the default "resolve a permanent
    /// spell" path ever filled it in. Everything that puts a permanent onto the
    /// battlefield another way — reanimation, a leaves-the-battlefield return,
    /// a blink — had to remember to stamp it, or the legend rule (CR 704.5j)
    /// silently skipped that permanent. Grimoire of the Dead remembered;
    /// nothing made the others.
    ///
    /// The flag survives for objects with no face to read: a token copy of a
    /// legendary creature is legendary and has no card behind it.
    #[must_use]
    pub fn is_legendary(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> bool {
        if let Some(data) = self.face_data(id, registry) {
            if data.supertypes.contains(&crate::types::Supertype::Legendary) {
                return true;
            }
        }
        self.get_object(id).is_some_and(|o| o.is_legendary)
    }

    /// Colors of the object: the union of any granted at runtime (Grimoire of
    /// the Dead's black) and those derived from the active face's mana cost.
    /// (Color indicators are not modeled.)
    #[must_use]
    pub fn colors_of(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Vec<crate::types::Color> {
        let mut cols = self.get_object(id).map(|o| o.colors.clone()).unwrap_or_default();
        let face = self.face_data(id, registry);
        // CR 204.2: a face with no mana cost states its colors with a color
        // indicator. Reading the cost alone made every transformed permanent
        // colorless, which Gatstaf Howler's intimidate reads directly.
        if let Some(indicator) = face.as_ref().map(|d| d.color_indicator.clone()).filter(|i| !i.is_empty()) {
            for c in indicator {
                if !cols.contains(&c) {
                    cols.push(c);
                }
            }
            return cols;
        }
        if let Some(cost) = face.and_then(|d| d.cost) {
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

    /// Continuous effects the object provides: the active face's, plus any
    /// this particular object was given.
    ///
    /// The instance list is *additional*, never a replacement. It used to be
    /// read as an override — a `Some(_)` answered for the whole object and the
    /// face was never consulted — and the only two writers both mean "these
    /// are effects this object has of its own": Nevermore's chosen name, and
    /// `become_copy_of` clearing what the pre-copy object had. Under the
    /// override reading the second of those blinded the face, so a permanent
    /// that entered as a copy had no static abilities at all: a copy of
    /// Grimgrin untapped in its untap step, a copy of Mayor of Avabruck gave
    /// no anthem (CR 706.2, 613.2).
    #[must_use]
    pub fn continuous_effects_of(&self, id: ObjectId, registry: &crate::cards::CardRegistry) -> Vec<crate::types::ContinuousEffect> {
        let Some(obj) = self.get_object(id) else { return Vec::new() };
        let mut effects = self.face_data(id, registry).map(|d| d.continuous_effects).unwrap_or_default();
        if let Some(ref inst) = obj.instance_continuous_effects {
            effects.extend(inst.iter().cloned());
        }
        effects
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
    /// CR 110.2a: the player who put this permanent onto the battlefield.
    /// Control-changing effects (layer 2) are applied on top of this, and it
    /// is who the permanent goes back to once every one of them has ended.
    /// Stamped on entry, so it is a fact about the object rather than a
    /// snapshot some effect took of whoever held it at the time.
    ///
    /// A save written before this field existed has no answer for it; those
    /// load as p0, which is only wrong for a permanent p1 put onto the
    /// battlefield while a control effect on it was live.
    #[serde(default = "first_player")]
    pub base_controller: PlayerId,
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

    /// The zone this spell was cast from (CR 601.2a), recorded as it goes on
    /// the stack. "Whenever you cast a spell from your graveyard" (Burning
    /// Vengeance) is a question about the event, and by the time a trigger is
    /// dispatched the spell has already left the zone it was cast from, so
    /// nothing else can answer it.
    ///
    /// It is not the same question as `cast_with_flashback`: flashback is one
    /// of three ways to cast a spell out of a graveyard in this pool, next to
    /// a card's own permission (Skaab Ruinator) and an alternative cost
    /// (Rooftop Storm). Asking the flashback flag missed the other two.
    #[serde(default)]
    pub cast_from_zone: Option<Zone>,

    /// Per-instance oracle text override (e.g., Bonds of Faith conditional effect).
    /// When set, aura parsing uses this instead of the card's static oracle text.
    #[serde(default)]
    pub instance_oracle_text: Option<String>,

    /// Continuous effects this particular object was given, on top of the ones
    /// printed on its active face (Nevermore's chosen name). Read through
    /// `continuous_effects_of`, which appends them to the face's.
    #[serde(default)]
    pub instance_continuous_effects: Option<Vec<crate::types::ContinuousEffect>>,

    /// Card-specific persistent state (e.g., Fiend Hunter stores the exiled creature ID).
    /// Keyed by purpose string, value is an `ObjectId`.
    #[serde(default)]
    pub card_state: std::collections::BTreeMap<String, ObjectId>,

    /// Counters on this permanent (+1/+1, -1/-1, etc.).
    pub counters: std::collections::BTreeMap<crate::types::CounterType, u32>,

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
    pub abilities_activated_this_turn: std::collections::BTreeSet<usize>,

    /// The enters-as-a-copy choice this card's controller made for it
    /// (CR 614.12b, Evil Twin). Recorded *before* the object is put onto the
    /// battlefield, and read by the card's own replacement effect as it
    /// enters, so the permanent is never on the battlefield as its printed
    /// self with the choice still outstanding. Reset on every zone change
    /// (CR 400.7) — a new object makes a new choice.
    #[serde(default)]
    pub entering_copy_choice: EnterAsCopyChoice,

    /// A token's printed characteristics, as the effect that created it set
    /// them. `None` for anything that is not a token.
    ///
    /// The characteristics layer above splits an object into a printed half
    /// (its active face) and a granted half (the object-level vectors). A
    /// token has no face, so both halves used to share one vector and
    /// nothing could tell them apart. Two things read the wrong half as a
    /// result: the CR 111.4 check took a token's *current* subtypes for the
    /// ones its name was derived from — "once a token is on the battlefield,
    /// changing its name doesn't change its subtype(s), and vice versa", so
    /// Olivia Voldaren making a Zombie token a Vampire looked like a
    /// violation — and `printed_subtypes_of` handed that granted Vampire to
    /// copy effects, which take only copiable values (CR 707.2).
    ///
    /// Written once at creation and never again: it is the token's face.
    #[serde(default)]
    pub token_face: Option<TokenFace>,
}

/// The printed half of a token — what the effect that created it said it was
/// (CR 111.4, CR 707.2). The face a token would have if it had a card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenFace {
    pub name: String,
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub colors: Vec<crate::types::Color>,
    pub keywords: Vec<crate::types::Keyword>,
    pub card_types: Vec<crate::types::CardType>,
    pub subtypes: Vec<String>,
}

/// Whether a permanent that chooses what to enter as has been asked yet, and
/// what its controller answered (CR 614.12b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EnterAsCopyChoice {
    /// The controller has not been asked. Nothing may enter in this state:
    /// `move_object` defers the entry until the choice is made.
    #[default]
    Unasked,
    /// Asked, and declined — "you may" (CR 614.12b). The permanent enters as
    /// its printed self, which for Evil Twin is a 0/0 that dies immediately.
    Declined,
    /// Asked, and answered with the permanent to enter as a copy of.
    Copy(ObjectId),
}

/// Serde fallback for `GameObject::base_controller` in saves written before
/// the field existed.
fn first_player() -> PlayerId {
    PlayerId(0)
}

/// A player's state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: PlayerId,
    pub life: i32,
    pub mana_pool: ManaPool,
    pub land_plays_remaining: u32,
    pub lost: bool,
    /// Why `lost` is set (CR 104.3), recorded so the end-of-game report can
    /// say how the game was decided rather than only who won (issue #86).
    #[serde(default)]
    pub loss_reason: Option<crate::events::LossReason>,
    pub has_drawn_from_empty: bool,
    /// Order of cards in library (first element is top of library).
    pub library_order: Vec<ObjectId>,
    /// CR 404.2: the graveyard is a single face-up pile with an order, and
    /// that order is arrival order — first element is the bottom, last is the
    /// card that got there most recently. Public information, and the answer
    /// to "which three cards did that mill just put here".
    #[serde(default)]
    pub graveyard_order: Vec<ObjectId>,
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
            loss_reason: None,
            has_drawn_from_empty: false,
            library_order: Vec::new(),
            graveyard_order: Vec::new(),
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

/// Where a card goes when an effect puts it into a library.
///
/// A library has an order, so no effect can put a card into one without
/// saying where: CR 701.15a ("put ... on top of") and CR 701.17a ("on the
/// bottom of") are separate keyword actions for that reason. Shuffling in is
/// [`LibraryPosition::Bottom`] followed by a shuffle, which is what the
/// shuffle makes of any position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryPosition {
    Top,
    Bottom,
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
    /// When this effect was created (CR 613.7a). Two control effects on one
    /// permanent are applied in this order, so the highest timestamp is the
    /// one whose controller the permanent actually has.
    #[serde(default)]
    pub timestamp: u64,
}

/// A layer-7b effect setting a permanent's power and/or toughness to a
/// specific value (CR 613.4b).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetPtEffect {
    pub object: ObjectId,
    /// `None` leaves that half alone: Tree of Redemption sets only toughness.
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    /// CR 613.7a: several 7b effects on one permanent apply in this order,
    /// so the highest timestamp is the one that decides.
    pub timestamp: u64,
}

/// Combat state, tracking attackers and blockers.
///
/// Ordered collections (`BTreeMap`/`BTreeSet`), not hashed ones: combat
/// damage is dealt and prompts are built by iterating these, and a
/// `HashMap`'s per-process iteration order made the same seeded game deal
/// its damage — and offer its blocker choices — in a different order on
/// each replay.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CombatState {
    /// Map of attacker `ObjectId` -> defending `PlayerId`. Every attacker is
    /// in here — one attacking a planeswalker defends against its controller
    /// (CR 508.1a), and additionally appears in `planeswalker_defenders`.
    pub attackers: std::collections::BTreeMap<ObjectId, PlayerId>,
    /// Attackers sent at a planeswalker rather than at the player:
    /// attacker -> the planeswalker it attacks.
    #[serde(default)]
    pub planeswalker_defenders: std::collections::BTreeMap<ObjectId, ObjectId>,
    /// Map of attacker `ObjectId` -> list of blockers assigned to it, in the
    /// order the blocks were declared.
    pub blocker_assignments: std::collections::BTreeMap<ObjectId, Vec<ObjectId>>,
    /// Map of attacker `ObjectId` -> its blockers in the damage assignment
    /// order the attacking player announced (CR 509.2). Announced once, in
    /// the declare blockers step, and used by both combat damage steps
    /// (CR 510.4 — first strike does not re-announce it). Empty for an
    /// unblocked attacker; declaration order is the fallback for a state
    /// that predates the announcement.
    #[serde(default)]
    pub damage_assignment_order: std::collections::BTreeMap<ObjectId, Vec<ObjectId>>,
    /// Attackers that became blocked when blockers were declared. Blocked-ness
    /// is permanent for the combat (CR 509.2): an attacker whose blockers all
    /// leave combat is still blocked (deals no combat damage without trample),
    /// which `blocker_assignments` alone can't express once its list empties.
    #[serde(default)]
    pub blocked_attackers: std::collections::BTreeSet<ObjectId>,
    /// Creatures that had first/double strike when first-strike combat damage
    /// was dealt (CR 510.5): they don't deal damage again in the regular
    /// combat damage step unless they have double strike.
    #[serde(default)]
    pub dealt_first_strike: std::collections::BTreeSet<ObjectId>,
    /// Whether any attackers were declared this combat. CR 508.8 skips the
    /// declare blockers and combat damage steps when none were — and the
    /// question is about the declaration, not about who is still in combat,
    /// so `attackers.is_empty()` can't answer it (a lone attacker removed
    /// from combat during the declare attackers step leaves the map empty,
    /// but the steps still happen).
    #[serde(default)]
    pub any_attackers_declared: bool,
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
    /// What the ability asks of its target, read before its costs were paid
    /// (the source may be gone by the time X is funded).
    #[serde(default)]
    pub target_requirement: Option<crate::cards::TargetRequirement>,
    /// The costs this activation has NOT paid yet, because X had not been
    /// announced (CR 601.2b precedes 601.2h, via 602.2b). `None` means the
    /// costs were already paid — the old order, kept only so a save written
    /// under it still loads.
    #[serde(default)]
    pub unpaid: Option<DeferredActivationCost>,
}

/// The whole cost of an activation, held while its X is announced.
///
/// The ability path used to tap the permanent, pay the mana, remove the
/// counters and perform the SACRIFICE, and only then ask what X was — a
/// prompt whose only purpose was to confirm a cost after that cost had been
/// charged, and therefore one with nothing to cancel. CR 602.2b applies the
/// CR 601.2 sequence to activating an ability, and in that sequence 601.2b
/// (announce X) precedes 601.2h (pay the total cost). The cast path already
/// worked this way; this is the same stash (issue #290).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredActivationCost {
    /// Mana sources to tap before paying, as `legal_actions` planned them.
    pub tap_plan: Vec<(ObjectId, usize)>,
    /// The mana cost with X stripped out; the X half is the funding response.
    pub non_x_mana_cost: crate::types::ManaCost,
    /// The `{T}` in the cost, if any.
    pub requires_tap: bool,
    /// Counters to remove from the source.
    pub counter_cost: Option<(crate::types::CounterType, u32)>,
    /// The creature the player chose to sacrifice, when the cost lets them
    /// choose one.
    pub sacrifice: Option<ObjectId>,
    pub sacrifice_cost: crate::cards::SacrificeCost,
    pub once_per_turn: bool,
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
    /// True if this is a cast from the graveyard under the card's own
    /// permission (CR 601.3a) rather than from hand. Recorded here because
    /// by the time the cast finalizes the spell is on the stack and the zone
    /// it came from can no longer be read off it.
    #[serde(default)]
    pub cast_from_graveyard: bool,
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

/// The opening hand a London mulligan always redraws to (CR 103.2, 103.4).
/// It is also the ceiling on the bottoming obligation: a player who has
/// mulliganed seven or more times keeps an empty hand, and cannot be asked
/// to bottom more cards than they hold. CR 103.4 puts no cap on the number
/// of mulligans themselves.
pub const OPENING_HAND_SIZE: usize = 7;

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
        /// How many cards this player still has to discard, counting this
        /// one. "Target player discards two cards" is one choice with
        /// `remaining: 2`; the engine re-presents it against the refreshed
        /// hand until the count runs out or the hand does.
        ///
        /// Brain Weevil used to chain its second discard itself, keeping the
        /// target player between the two in `card_state` — a
        /// `String -> ObjectId` map — as `ObjectId(player.0 as u64)`, read
        /// back with an `unwrap_or(u8::MAX)` that would have named a player
        /// who does not exist. Discarding N cards is not one card's problem.
        ///
        /// Only meaningful with `discard_immediately`: the CR 101.4 path,
        /// where several players each choose one card and they leave together,
        /// always collects exactly one per player.
        remaining: usize,
    },
    /// Choose one of a set of cards the player has *looked at* to keep
    /// (Forbidden Alchemy).
    ///
    /// Looking at a card is not revealing it (CR 701.18a): only the player
    /// who looked knows what these are, and the one they take goes to a
    /// hidden zone (CR 400.2). This used to be called `ChooseFromRevealed`
    /// with a field named `revealed`, and the handler logged the names at
    /// `Event` accordingly, which put all four cards and the one kept into
    /// the shared --log and both seats' on-screen LOG pane (issue #217).
    /// The name is the fix as much as the log level is.
    ChooseFromLookedAt {
        description: String,
        looked_at: Vec<ObjectId>,
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
    /// CR 603.3b: a player whose abilities triggered simultaneously puts them
    /// on the stack in any order. Answered by `ChosenIndex`: the chosen
    /// trigger goes on the stack *next*, so of the group the one chosen last
    /// resolves first. Raised only when the group holds at least two
    /// distinguishable triggers — several instances of the same ability from
    /// the same source are interchangeable and are not worth a prompt.
    ChooseTriggerOrder {
        description: String,
        /// Display names of the triggers still to be ordered, parallel to
        /// `indices`.
        options: Vec<String>,
        /// Which pending queue the group lives in (AP or NAP). No player
        /// receives priority while this prompt is up, so the queue cannot
        /// change underneath it.
        ap_queue: bool,
        /// Positions of the group's triggers in that queue, parallel to
        /// `options`.
        indices: Vec<usize>,
    },
    /// CR 509.2: the attacking player announces the damage assignment order
    /// among the creatures blocking one attacker. Answered by `ChosenIndex`
    /// over `options`: the chosen blocker takes the next place in the order,
    /// so the one chosen first is assigned damage first and must be assigned
    /// lethal damage before any is assigned to the one after it (CR 510.1c).
    /// Raised once per attacker blocked by two or more creatures, and
    /// re-raised until that attacker's order is complete.
    ChooseDamageAssignmentOrder {
        description: String,
        /// The attacker whose blockers are being ordered.
        attacker: ObjectId,
        /// Blockers not yet placed in the order, parallel to `options`.
        remaining: Vec<ObjectId>,
        /// Display names of those blockers.
        options: Vec<String>,
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
    /// CR 119.3: damage has a source, and the source is an object — not its
    /// name. The name used to ride along beside it and was read by nothing:
    /// `deal_damage` writes the log line itself, from the source object.
    DealDamage { amount: u32, source_id: ObjectId },
    /// Destroy the chosen permanent.
    Destroy { source_name: String },
    /// Apply -N/-M until end of turn to the chosen creature.
    DebuffUntilEOT { power: i32, toughness: i32, source_name: String },
    /// Target creature can't block this turn.
    CantBlockThisTurn { source_name: String },



    /// Destroy target creature matching a filter (Reaper from the Abyss).
    DestroyCreature { source_name: String },

    /// Return the chosen object to its owner's hand.
    ReturnToHand { source_name: String },




    /// Sacrifice the chosen creature (generic sacrifice, e.g. Liliana -2).
    SacrificeCreature { source_name: String },
    /// The answer to "you may have this permanent enter as a copy of any
    /// creature on the battlefield" (CR 614.12b — Evil Twin). `object` is
    /// the permanent that is *about to* enter: it is still on the stack (or
    /// wherever it is coming from), and recording the answer is what releases
    /// its deferred entry.
    EnterAsCopy { object: ObjectId },







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

    /// CR 508.4b: a token put onto the battlefield attacking was never
    /// declared as an attacker; its controller chooses which player or
    /// planeswalker it is attacking as it's created. The chosen target is a
    /// `Target::Player` (an opponent) or a `Target::Object` (a planeswalker an
    /// opponent controls). `remaining` carries the tokens still to be asked
    /// about — a multi-token creation (Kessig Cagebreakers) chains one choice
    /// per token, per its ruling ("You declare which player or planeswalker
    /// **each** token is attacking"). `source_id` names the prompt.
    TokenAttacks { token_id: ObjectId, remaining: Vec<ObjectId>, source_id: ObjectId },
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
