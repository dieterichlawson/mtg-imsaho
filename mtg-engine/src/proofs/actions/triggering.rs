//! Which abilities an event triggers (CR 603.2): the trigger collector,
//! `triggers::collect::for_event`, for each kind of event a card in the
//! pool has an ability trigger on. A card proof shows which kinds of
//! trigger a card prints, what it asks of the event (its condition), and
//! what its ability does as it resolves; these show that the engine
//! raises an ability of that kind on that event, from the object it
//! should, under the controller it should, exactly when its condition
//! holds, and nothing else, and files it with the active player's
//! triggers or the other player's (603.3b).
//!
//! Every ability here is printed by [`Watching`], a card printing one
//! triggered ability and nothing else, whose condition on its event is
//! any answer (the same each time it is asked of the same object): what
//! a card's own condition is, is its proof. The other objects are of the
//! blank card, which prints no ability.

use crate::proofs::held::Held;
use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggerScope, TriggeredAbilityDef};
use crate::events::{DamageTarget, GameEvent};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::proofs::cards::{card_proof, case_by_case, Blank};
use crate::state::GameState;
use crate::triggers::collect::{for_event, Collector};
use crate::triggers::TriggerEvent;
use crate::types::{Step, Zone};

/// The answers [`Watching`]'s conditions give, by object (ids 1 to 4):
/// chosen the first time each is asked, the same after.
static mut SAID: Held<[Option<bool>; 5]> = Held::new([None; 5], (&raw const SAID).cast());

/// Whether `id`'s condition on the event holds.
fn said(id: ObjectId) -> bool {
    let i = usize::try_from(id.0).expect("an id of this board");
    assert!((1..=4).contains(&i), "an object this proof does not make");
    // SAFETY: a proof runs on one thread, and only this function names SAID.
    let said = unsafe { &mut *std::ptr::addr_of_mut!(SAID.value) };
    *said[i].get_or_insert_with(kani::any::<bool>)
}

/// A card printing one triggered ability, of `kind`, and nothing else;
/// one that triggers from `zones` (CR 603.6, 113.6) and, for one on a
/// step's beginning, on whose steps `scope` says.
struct Watching {
    kind: TriggerKind,
    zones: &'static [Zone],
    scope: TriggerScope,
}

/// [`Watching`] from the battlefield, on each player's steps.
fn watching(kind: TriggerKind) -> Watching {
    Watching { kind, zones: &[Zone::Battlefield], scope: TriggerScope::Each }
}

/// A name of its own for each kind of trigger: the registry takes a card
/// name once.
fn named(kind: &TriggerKind) -> &'static str {
    match kind {
        TriggerKind::EntersBattlefield => "Watching Itself Enter",
        TriggerKind::AnyCreatureEnters => "Watching Creatures Enter",
        TriggerKind::SelfDies => "Watching Itself Die",
        TriggerKind::AnyCreatureDies => "Watching Creatures Die",
        TriggerKind::Upkeep => "Watching Upkeeps",
        TriggerKind::EndStep => "Watching End Steps",
        TriggerKind::EndCombat => "Watching Ends of Combat",
        TriggerKind::SpellCast => "Watching Spells",
        TriggerKind::Attacks => "Watching Attacks",
        TriggerKind::Blocks => "Watching Blocks",
        TriggerKind::BecomesBlocked => "Watching Being Blocked",
        TriggerKind::CombatDamageToPlayer => "Watching Its Combat Damage",
        TriggerKind::AnyCombatDamageToPlayer => "Watching Combat Damage",
        TriggerKind::AnyDamageToPlayer => "Watching Damage",
        TriggerKind::DealsCombatDamageToCreature => "Watching Its Damage to Creatures",
        TriggerKind::LeavesBattlefield => "Watching Itself Leave",
        TriggerKind::CreatureCardMilled => "Watching Mills",
        _ => "Watching",
    }
}

impl CardBehavior for Watching {
    fn card_data(&self) -> CardData {
        CardData {
            name: named(&self.kind).into(),
            triggered_abilities: vec![TriggeredAbilityDef::new(self.kind.clone(), "it triggers")],
            ..CardData::default()
        }
    }
    fn has_etb_handler(&self) -> bool {
        self.kind == TriggerKind::EntersBattlefield
    }
    fn trigger_zones(&self, _: &TriggerKind) -> Vec<Zone> {
        self.zones.to_vec()
    }
    fn step_trigger_scope(&self, _: &TriggerKind, _: bool) -> TriggerScope {
        self.scope
    }
    fn should_trigger(&self, _: &GameState, id: ObjectId, _: &TriggerKind, _: &CardRegistry) -> bool {
        said(id)
    }
    fn should_trigger_on_creature_enters(&self, _: &GameState, id: ObjectId, _: ObjectId, _: PlayerId, _: &CardRegistry) -> bool {
        said(id)
    }
    #[allow(clippy::too_many_arguments)]
    fn should_trigger_on_creature_dies(
        &self, _: &GameState, id: ObjectId, _: ObjectId, _: PlayerId, _: &[ObjectId], _: i32, _: bool, _: &[String], _: &CardRegistry,
    ) -> bool {
        said(id)
    }
    fn should_trigger_on_spell_cast(&self, _: &GameState, id: ObjectId, _: PlayerId, _: ObjectId, _: &CardRegistry) -> bool {
        said(id)
    }
    fn should_trigger_on_blocks(&self, _: &GameState, id: ObjectId, _: ObjectId, _: &CardRegistry) -> bool {
        said(id)
    }
    fn should_trigger_on_becomes_blocked(&self, _: &GameState, id: ObjectId, _: ObjectId, _: &CardRegistry) -> bool {
        said(id)
    }
    fn should_trigger_on_damage_to_player(&self, _: &GameState, id: ObjectId, _: ObjectId, _: PlayerId, _: &CardRegistry) -> bool {
        said(id)
    }
}

/// Either player.
fn anyone() -> PlayerId {
    PlayerId(u8::from(kani::any::<bool>()))
}

/// Any of `zones`.
fn somewhere(zones: &[Zone]) -> Zone {
    let i: usize = kani::any();
    kani::assume(i < zones.len());
    zones[i]
}

/// Whether `c` holds exactly the abilities of `candidates` whose third
/// part says the event raised them, each a source and its controller, in
/// the order they are listed among its controller's, filed with the active
/// player's when its controller is the active player and with the other's
/// otherwise, each raised for an event `fits` takes. Every candidate is
/// listed, raised or not, so that the list's length is known: a list of
/// only the raised ones, built by pushing, was of a length and contents
/// the solver left open, and the comparison ran it out of memory.
fn collected(c: &Collector, candidates: &[(ObjectId, PlayerId, bool)], fits: impl Fn(ObjectId, &TriggerEvent) -> bool) -> bool {
    for (bucket, theirs) in [(&c.ap, true), (&c.nap, false)] {
        let mut count = 0;
        for &(id, controller, raised) in candidates {
            if raised && (controller == c.active_player) == theirs {
                let Some(t) = bucket.get(count) else { return false };
                if t.source.id != id || t.source.controller != controller || !fits(id, &t.event) {
                    return false;
                }
                count += 1;
            }
        }
        if count != bucket.len() {
            return false;
        }
    }
    true
}

/// Collects what the last of `events` triggers, the rest of its batch
/// before it. The batch is the caller's own array, each event of a kind
/// the harness names, read where it lies: an event read back off the heap
/// whose kind the solver chose (combat damage or not) was one it could not
/// tell from any kind at all, and it followed every collector there is,
/// three quarters of a damage proof's work. An event holding a list or a
/// name, cloned, would be the same.
fn collect(state: &mut GameState, events: &[GameEvent], registry: &CardRegistry) -> Collector {
    // Each list with its room from the start: one empty, and so
    // unallocated, until a push the solver may or may not take is one it
    // follows into an allocation down each way, the pushed trigger written
    // into each (most of what an upkeep's proof cost).
    let mut c = Collector { active_player: state.active_player, ap: Vec::with_capacity(2), nap: Vec::with_capacity(2) };
    for_event(state, events, events.last().expect("an event"), registry, &mut c);
    c
}

/// A registry of the blank card and `cards`, in that order.
fn registry_of(cards: Vec<Box<dyn CardBehavior>>) -> (CardRegistry, CardId, Vec<CardId>) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let ids = cards.into_iter().map(|c| registry.register(c)).collect();
    (registry, blank, ids)
}

/// "When this permanent enters" (CR 603.6a): a permanent entering raises
/// its own ability, under its controller, exactly when it prints one and
/// its condition holds; a permanent of a card printing none raises
/// nothing. Either player's, either player active.
fn entering_itself(prints: bool) {
    let (registry, blank, cards) = registry_of(vec![Box::new(watching(TriggerKind::EntersBattlefield))]);
    let mut state = crate::proofs::cards::new_game();
    state.active_player = anyone();
    let controller = anyone();
    let id = state.create_object(if prints { cards[0] } else { blank }, controller, Zone::Battlefield, None, None);
    let c = collect(&mut state, &[GameEvent::EnteredBattlefield { object: id, controller }], &registry);
    let expected: &[(ObjectId, PlayerId, bool)] = &[(id, controller, prints && said(id))];
    assert!(collected(&c, expected, |_, e| matches!(e, TriggerEvent::SelfEntered)));
    std::mem::forget((registry, state, c, cards));
}

case_by_case! { entering_itself;
    a_permanent_entering_raises_its_own_enters_ability: true,
    a_permanent_printing_no_enters_ability_raises_none: false,
}

/// "Whenever another creature enters" (CR 603.2, 603.6a), from the zones
/// the ability triggers from (`zones`, Dearly Departed's being its
/// owner's graveyard): a creature entering raises a watcher's ability,
/// under its controller, exactly when the watcher is in one of those zones
/// and its condition holds, and never the entering creature's own
/// ("another"); a noncreature entering raises none. The entering object a
/// watcher itself, a creature or not; the other watcher either player's,
/// on the battlefield, in a graveyard or in exile.
///
/// Each proof here watches with one permanent besides the event's own
/// object: two, each either player's, filed by the solver into either
/// player's list in either order, ran it out of memory at 7 GB.
fn another_entering(zones: &'static [Zone]) {
    let (registry, _, cards) = registry_of(vec![Box::new(Watching { kind: TriggerKind::AnyCreatureEnters, zones, scope: TriggerScope::Each })]);
    let mut state = crate::proofs::cards::new_game();
    state.active_player = anyone();
    let watcher = state.create_object(cards[0], anyone(), somewhere(&[Zone::Battlefield, Zone::Graveyard, Zone::Exile]), None, None);
    let creature: bool = kani::any();
    let controller = anyone();
    let entering = state.create_object(cards[0], controller, Zone::Battlefield,
        if creature { Some(1) } else { None }, if creature { Some(1) } else { None });
    let c = collect(&mut state, &[GameEvent::EnteredBattlefield { object: entering, controller }], &registry);
    let o = state.get_object(watcher).expect("on the board");
    let expected = [(watcher, o.controller, creature && zones.contains(&o.zone) && said(watcher))];
    assert!(collected(&c, &expected, |_, e| matches!(e, TriggerEvent::CreatureEntered { entered, entered_controller }
        if *entered == entering && *entered_controller == controller)));
    std::mem::forget((registry, state, c, cards));
}

case_by_case! { another_entering;
    another_creature_entering_raises_a_watchers_ability_on_the_battlefield: &[Zone::Battlefield],
    another_creature_entering_raises_a_watchers_ability_in_a_graveyard: &[Zone::Graveyard],
}

card_proof! {
    /// "When this creature dies" (CR 603.6c, 603.10a): a creature dying
    /// raises its own ability when it prints one, under the controller it
    /// died under. Either player's, either player active.
    fn a_creature_dying_raises_its_own_dies_ability() {
        let (registry, _, cards) = registry_of(vec![Box::new(watching(TriggerKind::SelfDies))]);
        let mut state = crate::proofs::cards::new_game();
        state.active_player = anyone();
        let controller = anyone();
        let dead = state.create_object(cards[0], controller, Zone::Graveyard, Some(2), Some(2));
        let event = GameEvent::CreatureDied {
            object: dead, name: "Watching".into(), card_id: cards[0], controller, damaged_by: Vec::new(),
            last_known_toughness: 2, is_token: false, subtypes: Vec::new(),
        };
        let c = collect(&mut state, &[event], &registry);
        assert!(collected(&c, &[(dead, controller, true)], |_, e| matches!(e, TriggerEvent::SelfDies)));
        std::mem::forget((registry, state, c, cards));
    }
}

/// Where the watcher of another creature's death was as it died.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Watched {
    /// On the battlefield, and still there.
    OnTheBattlefield,
    /// Gone to a graveyard in the same event, as two creatures dying at
    /// once see each other die.
    DiedWithIt,
    /// In a graveyard already.
    DiedBefore,
}

/// "Whenever another creature dies" (CR 603.6c, 603.10a): a creature
/// dying raises a watcher's ability, under its controller, exactly when
/// the watcher was on the battlefield as it died (on it still, or gone
/// from it in the same event, under the controller it had then, either
/// player and not only its owner: a stolen Falkenrath Noble's was given to
/// its owner, CR 603.3a) and its condition holds; never the dying
/// creature's own watching ability. The dying creature a watcher itself,
/// either player's; the other watcher either player's, where `was` says,
/// a constant in each harness: left to the solver, with the event's batch
/// of one length or the other, the formula ran out of memory at 7 GB.
fn another_dying(was: Watched) {
    let (registry, _, cards) = registry_of(vec![Box::new(watching(TriggerKind::AnyCreatureDies))]);
    let mut state = crate::proofs::cards::new_game();
    state.active_player = anyone();
    let on = was == Watched::OnTheBattlefield;
    let watcher = state.create_object(cards[0], anyone(), if on { Zone::Battlefield } else { Zone::Graveyard }, Some(1), Some(1));
    let last_controller = if was == Watched::DiedWithIt {
        let stolen_or_not = anyone();
        state.get_object_mut(watcher).expect("just created").last_controller = Some(stolen_or_not);
        stolen_or_not
    } else {
        state.get_object(watcher).expect("just created").controller
    };
    let controller = anyone();
    let dead = state.create_object(cards[0], controller, Zone::Graveyard, Some(2), Some(2));
    let event = GameEvent::CreatureDied {
        object: dead, name: "Watching".into(), card_id: cards[0], controller, damaged_by: Vec::new(),
        last_known_toughness: 2, is_token: false, subtypes: Vec::new(),
    };
    let c = if was == Watched::DiedWithIt {
        collect(&mut state, &[GameEvent::LeftBattlefield { object: watcher, to: Zone::Graveyard, last_controller }, event], &registry)
    } else {
        collect(&mut state, &[event], &registry)
    };
    let expected = [(watcher, last_controller, was != Watched::DiedBefore && said(watcher))];
    assert!(collected(&c, &expected, |_, e| matches!(e, TriggerEvent::CreatureDied { dead: d }
        if d.id == dead && d.controller == controller)));
    std::mem::forget((registry, state, c, cards));
}

case_by_case! { another_dying;
    another_creature_dying_raises_a_watchers_ability_on_the_battlefield: Watched::OnTheBattlefield,
    another_creature_dying_raises_the_ability_of_a_watcher_dying_with_it: Watched::DiedWithIt,
    another_creature_dying_raises_none_of_a_watcher_already_dead: Watched::DiedBefore,
}

/// "At the beginning of [each / your / enchanted player's] <step>" (CR
/// 503.1a, 513.1a, 507.1, 603.2): the step's beginning raises the ability
/// of a permanent printing one on it, under its controller, exactly when
/// it is on the battlefield, it is its step (each player's; its
/// controller's; the player it is attached to's) and its condition holds
/// (603.4). The permanent either player's, on the battlefield or in a
/// graveyard, attached to either player or to neither; either player
/// active.
fn beginning((step, kind, scope): (Step, TriggerKind, TriggerScope)) {
    let (registry, _, cards) = registry_of(vec![Box::new(Watching { kind, zones: &[Zone::Battlefield], scope })]);
    let mut state = crate::proofs::cards::new_game();
    state.active_player = anyone();
    let active = state.active_player;
    let watcher = state.create_object(cards[0], anyone(), somewhere(&[Zone::Battlefield, Zone::Graveyard]), None, None);
    let attached = kani::any::<u8>() % 3;
    state.get_object_mut(watcher).expect("just created").attached_to_player = (attached < 2).then_some(PlayerId(attached));
    let c = collect(&mut state, &[GameEvent::StepStarted { step }], &registry);
    let o = state.get_object(watcher).expect("on the board");
    let whose = match scope {
        TriggerScope::Each => true,
        TriggerScope::Your => o.controller == active,
        TriggerScope::AttachedPlayer => o.attached_to_player == Some(active),
    };
    let expected = [(watcher, o.controller, o.zone == Zone::Battlefield && whose && said(watcher))];
    assert!(collected(&c, &expected, |_, e| match step {
        Step::Upkeep => matches!(e, TriggerEvent::Upkeep),
        Step::EndStep => matches!(e, TriggerEvent::EndStep),
        _ => matches!(e, TriggerEvent::EndCombat),
    }));
    std::mem::forget((registry, state, c, cards));
}

case_by_case! { beginning;
    each_upkeep_raises_each_upkeep_ability: (Step::Upkeep, TriggerKind::Upkeep, TriggerScope::Each),
    your_upkeep_raises_your_upkeep_abilities: (Step::Upkeep, TriggerKind::Upkeep, TriggerScope::Your),
    enchanted_players_upkeep_raises_that_curses_upkeep_ability: (Step::Upkeep, TriggerKind::Upkeep, TriggerScope::AttachedPlayer),
    each_end_step_raises_each_end_step_ability: (Step::EndStep, TriggerKind::EndStep, TriggerScope::Each),
    your_end_step_raises_your_end_step_abilities: (Step::EndStep, TriggerKind::EndStep, TriggerScope::Your),
    your_end_of_combat_raises_your_end_of_combat_abilities: (Step::EndCombat, TriggerKind::EndCombat, TriggerScope::Your),
}

card_proof! {
    /// "Whenever a player casts a spell" (CR 601.2i, 603.2): a spell cast
    /// raises a watcher's ability, under its controller, for that caster
    /// and that spell, exactly when the watcher is on the battlefield and
    /// its condition (who cast it, what it is) holds. The watcher either
    /// player's, on the battlefield or in a graveyard; the spell either
    /// player's.
    fn a_spell_cast_raises_a_spell_watchers_ability() {
        let (registry, blank, cards) = registry_of(vec![Box::new(watching(TriggerKind::SpellCast))]);
        let mut state = crate::proofs::cards::new_game();
        state.active_player = anyone();
        let watcher = state.create_object(cards[0], anyone(), somewhere(&[Zone::Battlefield, Zone::Graveyard]), None, None);
        let caster = anyone();
        let spell = state.create_object(blank, caster, Zone::Stack, None, None);
        let c = collect(&mut state, &[GameEvent::SpellCast { player: caster, object: spell }], &registry);
        let o = state.get_object(watcher).expect("on the board");
        let expected = [(watcher, o.controller, o.zone == Zone::Battlefield && said(watcher))];
        assert!(collected(&c, &expected, |_, e| matches!(e, TriggerEvent::SpellCast { caster: p, spell_id }
            if *p == caster && *spell_id == spell)));
        std::mem::forget((registry, state, c, cards));
    }
}

// The declared attackers and blocks are a list inside the event, a variant
// narrower than `GameEvent`'s widest, which `precise-enums.py` leaves as
// written: read back, the list is of a length symbolic execution cannot
// see, and each pass of the loop over it past the one the harness declares
// was a pass over an attacker it could not rule out, the attached
// Equipment's ability raised in each (three quarters of a formula out of
// memory at 7 GB). Bounded to two passes, the declared one and the one that
// finds the list's end; the unwinding assertion holds it to that.
// loop bound: triggers::collect::combat::attackers_declared#2 2 attack
// loop bound: triggers::collect::combat::blockers_declared#2 2 block

/// What prints the attack ability [`attacking`] is over: the attacker
/// itself, or an Equipment in `zone`, attached to the attacker or not, or
/// one attached to it on the battlefield, `owner`'s with `active` active.
#[derive(Clone, Copy)]
enum Attacking {
    Itself,
    Equipment { zone: Zone, on: bool },
    Attached { owner: PlayerId, active: PlayerId },
}

/// "Whenever this creature attacks" (CR 508.3a, 603.2), on a creature or
/// on what is attached to it ("whenever equipped creature attacks"): an
/// attacker declared raises its own ability when it prints one, and that
/// of a permanent on the battlefield attached to it printing one, each
/// under its controller, for that attacker and the player it attacks.
/// The attacker, the Equipment and the player attacked each either
/// player's, either player active; where the Equipment is, and whether it
/// is attached, a constant in each harness: left to the solver, symbolic
/// execution ran out of memory at 7 GB. For the one that triggers, whose
/// it is and who is active are constants too: which of the collector's
/// lists its ability goes in, and where, left to the solver, was three
/// quarters of a formula that ran out of memory at 7 GB, the ability
/// written into each place of each list it might be at.
fn attacking(prints: Attacking) {
    let (registry, blank, cards) = registry_of(vec![Box::new(watching(TriggerKind::Attacks))]);
    let mut state = crate::proofs::cards::new_game();
    state.active_player = match prints {
        Attacking::Attached { active, .. } => active,
        _ => anyone(),
    };
    let controller = anyone();
    let itself = matches!(prints, Attacking::Itself);
    let attacker = state.create_object(if itself { cards[0] } else { blank }, controller, Zone::Battlefield, Some(2), Some(2));
    let defender = anyone();
    let expected = match prints {
        Attacking::Itself => [(attacker, controller, true)],
        Attacking::Equipment { zone, on } => {
            let owner = anyone();
            let equipment = state.create_object(cards[0], owner, zone, None, None);
            if on {
                state.get_object_mut(equipment).expect("just created").attached_to = Some(attacker);
            }
            [(equipment, owner, on && zone == Zone::Battlefield)]
        }
        Attacking::Attached { owner, .. } => {
            let equipment = state.create_object(cards[0], owner, Zone::Battlefield, None, None);
            state.get_object_mut(equipment).expect("just created").attached_to = Some(attacker);
            [(equipment, owner, true)]
        }
    };
    let c = collect(&mut state, &[GameEvent::AttackersDeclared { attackers: vec![(attacker, defender)] }], &registry);
    assert!(collected(&c, &expected, |_, ev| matches!(ev, TriggerEvent::Attacks { attacker: a, defending_player }
        if *a == attacker && *defending_player == defender)));
    std::mem::forget((registry, state, c, cards));
}

case_by_case! { attacking;
    an_attack_raises_the_attackers_own_ability: Attacking::Itself,
    an_attack_raises_its_equipments_ability: Attacking::Attached { owner: PlayerId(0), active: PlayerId(0) },
    an_attack_raises_its_equipments_ability_on_the_other_players_turn: Attacking::Attached { owner: PlayerId(0), active: PlayerId(1) },
    an_attack_raises_the_other_players_equipments_ability: Attacking::Attached { owner: PlayerId(1), active: PlayerId(1) },
    an_attack_raises_the_other_players_equipments_ability_on_the_first_players_turn: Attacking::Attached { owner: PlayerId(1), active: PlayerId(0) },
    an_attack_raises_no_ability_of_equipment_on_another: Attacking::Equipment { zone: Zone::Battlefield, on: false },
    an_attack_raises_no_ability_of_equipment_in_a_graveyard: Attacking::Equipment { zone: Zone::Graveyard, on: true },
    an_attack_raises_no_ability_of_unattached_equipment_in_a_graveyard: Attacking::Equipment { zone: Zone::Graveyard, on: false },
}

/// "Whenever this creature blocks" or "becomes blocked" (CR 509.2a,
/// 509.3c, 603.2): a block declared raises the blocker's ability on
/// blocking when it prints one and its condition (on what it blocked)
/// holds, or, `blocked`, the attacker's on becoming blocked likewise,
/// under its controller, for the other creature. Each creature either
/// player's.
fn blocking(blocked: bool) {
    let (registry, blank, cards) = registry_of(vec![Box::new(watching(if blocked { TriggerKind::BecomesBlocked } else { TriggerKind::Blocks }))]);
    let mut state = crate::proofs::cards::new_game();
    state.active_player = anyone();
    let (bc, ac) = (anyone(), anyone());
    let blocker = state.create_object(if blocked { blank } else { cards[0] }, bc, Zone::Battlefield, Some(2), Some(2));
    let attacker = state.create_object(if blocked { cards[0] } else { blank }, ac, Zone::Battlefield, Some(2), Some(2));
    let c = collect(&mut state, &[GameEvent::BlockersDeclared { assignments: vec![(blocker, attacker)] }], &registry);
    if blocked {
        assert!(collected(&c, &[(attacker, ac, said(attacker))], |_, e| matches!(e, TriggerEvent::BecomesBlocked { blocker_id }
            if *blocker_id == blocker)));
    } else {
        assert!(collected(&c, &[(blocker, bc, said(blocker))], |_, e| matches!(e, TriggerEvent::Blocks { blocked_attacker }
            if *blocked_attacker == attacker)));
    }
    std::mem::forget((registry, state, c, cards));
}

case_by_case! { blocking;
    a_block_raises_the_blockers_ability: false,
    a_block_raises_the_blocked_attackers_ability: true,
}

card_proof! {
    /// "Whenever this creature deals combat damage to a player" (CR 510.2,
    /// 603.2): combat damage a creature deals to a player raises its own
    /// ability when it prints one, under its controller, for that player
    /// and that amount; other damage raises none. The creature either
    /// player's, the player either.
    fn combat_damage_to_a_player_raises_the_creatures_own_ability() {
        let (registry, _, cards) = registry_of(vec![Box::new(watching(TriggerKind::CombatDamageToPlayer))]);
        let mut state = crate::proofs::cards::new_game();
        state.active_player = anyone();
        let controller = anyone();
        let source = state.create_object(cards[0], controller, Zone::Battlefield, Some(2), Some(2));
        let (player, combat) = (anyone(), kani::any::<bool>());
        // Each kind of damage its own constant event (`collect` says why).
        let c = if combat {
            collect(&mut state, &[GameEvent::CombatDamageDealt { source, target: DamageTarget::Player(player), amount: 2 }], &registry)
        } else {
            collect(&mut state, &[GameEvent::NonCombatDamageDealt { source, target: DamageTarget::Player(player), amount: 2 }], &registry)
        };
        assert!(collected(&c, &[(source, controller, combat)], |_, e| matches!(e, TriggerEvent::CombatDamageToPlayer { damaged_player, amount }
            if *damaged_player == player && *amount == 2)));
        std::mem::forget((registry, state, c, cards));
    }
}

/// "Whenever a creature deals [combat] damage to a player" (CR 510.2,
/// 603.2): damage a creature deals to a player raises a watcher's `watch`
/// ability, under its controller, exactly when the watcher was on the
/// battlefield, its condition holds, and the damage is combat damage or
/// the ability watches any damage. The watcher the creature itself (Rakish
/// Heir watches its own) or another permanent, either player's, in the
/// zone `elsewhere` names, a constant in each harness (left to the solver,
/// the formula ran out of memory at 7 GB); combat damage or not.
fn damage_to_a_player((watch, elsewhere): (TriggerKind, Option<Zone>)) {
    let (registry, blank, cards) = registry_of(vec![Box::new(watching(watch.clone()))]);
    let mut state = crate::proofs::cards::new_game();
    state.active_player = anyone();
    let watcher_elsewhere = elsewhere.map(|zone| state.create_object(cards[0], anyone(), zone, Some(1), Some(1)));
    let controller = anyone();
    let source = state.create_object(if elsewhere.is_some() { blank } else { cards[0] }, controller, Zone::Battlefield, Some(2), Some(2));
    let watcher = watcher_elsewhere.unwrap_or(source);
    let (player, combat) = (anyone(), kani::any::<bool>());
    // Each kind of damage its own constant event (`collect` says why).
    let c = if combat {
        collect(&mut state, &[GameEvent::CombatDamageDealt { source, target: DamageTarget::Player(player), amount: 2 }], &registry)
    } else {
        collect(&mut state, &[GameEvent::NonCombatDamageDealt { source, target: DamageTarget::Player(player), amount: 2 }], &registry)
    };
    let o = state.get_object(watcher).expect("on the board");
    let raises = combat || watch == TriggerKind::AnyDamageToPlayer;
    let expected = [(watcher, o.controller, raises && o.zone == Zone::Battlefield && said(watcher))];
    assert!(collected(&c, &expected, |_, e| match e {
        TriggerEvent::AnyCombatDamageToPlayer { dealer, damaged_player, amount } =>
            watch == TriggerKind::AnyCombatDamageToPlayer && *dealer == source && *damaged_player == player && *amount == 2,
        TriggerEvent::AnyDamageToPlayer { dealer, damaged_player, amount } =>
            watch == TriggerKind::AnyDamageToPlayer && *dealer == source && *damaged_player == player && *amount == 2,
        _ => false,
    }));
    std::mem::forget((registry, state, c, cards, watch));
}

case_by_case! { damage_to_a_player;
    damage_to_a_player_raises_another_combat_damage_watchers_ability: (TriggerKind::AnyCombatDamageToPlayer, Some(Zone::Battlefield)),
    damage_to_a_player_raises_no_combat_damage_watchers_ability_in_a_graveyard: (TriggerKind::AnyCombatDamageToPlayer, Some(Zone::Graveyard)),
    damage_to_a_player_raises_the_dealers_own_combat_damage_watching: (TriggerKind::AnyCombatDamageToPlayer, None),
    damage_to_a_player_raises_another_damage_watchers_ability: (TriggerKind::AnyDamageToPlayer, Some(Zone::Battlefield)),
    damage_to_a_player_raises_no_damage_watchers_ability_in_a_graveyard: (TriggerKind::AnyDamageToPlayer, Some(Zone::Graveyard)),
    damage_to_a_player_raises_the_dealers_own_damage_watching: (TriggerKind::AnyDamageToPlayer, None),
}

card_proof! {
    /// "Whenever this creature deals combat damage to a creature" (CR
    /// 510.2, 603.2): combat damage one creature deals another raises the
    /// dealer's ability when it prints one, under its controller, for the
    /// creature dealt damage and the amount; noncombat damage raises none,
    /// and nor does combat damage to a planeswalker, which is no creature
    /// (510.1a-c; Creepy Doll's coin was flipped for a Garruk it hit). The
    /// dealer printing it or not, either player's, dealing it to a creature
    /// or to a planeswalker.
    fn combat_damage_to_a_creature_raises_the_dealers_ability() {
        let (registry, blank, cards) = registry_of(vec![Box::new(watching(TriggerKind::DealsCombatDamageToCreature))]);
        let mut state = crate::proofs::cards::new_game();
        state.active_player = anyone();
        let prints: bool = kani::any();
        let controller = anyone();
        let source = state.create_object(if prints { cards[0] } else { blank }, controller, Zone::Battlefield, Some(2), Some(2));
        let walker: bool = kani::any();
        let (power, toughness) = if walker { (None, None) } else { (Some(3), Some(3)) };
        let damaged = state.create_object(blank, anyone(), Zone::Battlefield, power, toughness);
        if walker {
            state.get_object_mut(damaged).expect("just made").card_types.push(crate::types::CardType::Planeswalker);
        }
        let combat: bool = kani::any();
        // Each kind of damage its own constant event (`collect` says why).
        let c = if combat {
            collect(&mut state, &[GameEvent::CombatDamageDealt { source, target: DamageTarget::Object(damaged), amount: 2 }], &registry)
        } else {
            collect(&mut state, &[GameEvent::NonCombatDamageDealt { source, target: DamageTarget::Object(damaged), amount: 2 }], &registry)
        };
        let expected: &[(ObjectId, PlayerId, bool)] = &[(source, controller, combat && prints && !walker)];
        assert!(collected(&c, expected, |_, e| matches!(e, TriggerEvent::CombatDamageToCreature { damaged_creature, amount }
            if *damaged_creature == damaged && *amount == 2)));
        std::mem::forget((registry, state, c, cards));
    }
}

card_proof! {
    /// "When this permanent leaves the battlefield" (CR 603.6c, 603.10a): a
    /// permanent leaving raises its own ability when it prints one, under
    /// the controller it had as it left. The permanent printing it or not,
    /// gone to a graveyard, a hand or exile, either player's last.
    fn a_permanent_leaving_raises_its_own_leaves_ability() {
        let (registry, blank, cards) = registry_of(vec![Box::new(watching(TriggerKind::LeavesBattlefield))]);
        let mut state = crate::proofs::cards::new_game();
        state.active_player = anyone();
        let prints: bool = kani::any();
        let to = somewhere(&[Zone::Graveyard, Zone::Hand, Zone::Exile]);
        let id = state.create_object(if prints { cards[0] } else { blank }, anyone(), to, None, None);
        let last_controller = anyone();
        let c = collect(&mut state, &[GameEvent::LeftBattlefield { object: id, to, last_controller }], &registry);
        let expected: &[(ObjectId, PlayerId, bool)] = &[(id, last_controller, prints)];
        assert!(collected(&c, expected, |_, e| matches!(e, TriggerEvent::LeftBattlefield)));
        std::mem::forget((registry, state, c, cards));
    }
}

card_proof! {
    /// "Whenever a creature card is put into an opponent's graveyard from
    /// their library" (Undead Alchemist's, CR 603.2): a creature card
    /// milled raises a watcher's ability, under its controller, for that
    /// card and that player, exactly when the watcher is on the
    /// battlefield and its controller is an opponent of the player milled.
    /// The watcher either player's, on the battlefield or in a graveyard;
    /// the card either player's.
    fn a_creature_card_milled_raises_an_opponents_watchers_ability() {
        let (registry, blank, cards) = registry_of(vec![Box::new(watching(TriggerKind::CreatureCardMilled))]);
        let mut state = crate::proofs::cards::new_game();
        state.active_player = anyone();
        let watcher = state.create_object(cards[0], anyone(), somewhere(&[Zone::Battlefield, Zone::Graveyard]), None, None);
        let milled_player = anyone();
        let card = state.create_object(blank, milled_player, Zone::Graveyard, Some(1), Some(1));
        let c = collect(&mut state, &[GameEvent::CreatureCardMilled { object: card, milled_player }], &registry);
        let o = state.get_object(watcher).expect("on the board");
        let expected = [(watcher, o.controller, o.zone == Zone::Battlefield && o.controller != milled_player)];
        assert!(collected(&c, &expected, |_, e| matches!(e, TriggerEvent::CreatureCardMilled { milled_object, milled_player: p }
            if *milled_object == card && *p == milled_player)));
        std::mem::forget((registry, state, c, cards));
    }
}

card_proof! {
    /// The same, of a watcher the same check of the state-based actions
    /// took from the battlefield, as a deathtoucher beside the mill its
    /// replacement made kills Undead Alchemist: it was there as the card
    /// was milled, so its ability is raised, under the controller it had,
    /// when that player is an opponent of the player milled (CR 603.2,
    /// 608.2g). It used to raise nothing. Either player's last.
    fn a_creature_card_milled_raises_the_ability_of_a_watcher_gone_in_the_same_check() {
        let (registry, blank, cards) = registry_of(vec![Box::new(watching(TriggerKind::CreatureCardMilled))]);
        let mut state = crate::proofs::cards::new_game();
        state.active_player = anyone();
        let watcher = state.create_object(cards[0], anyone(), Zone::Graveyard, None, None);
        let last_controller = anyone();
        state.get_object_mut(watcher).expect("just made").last_controller = Some(last_controller);
        let milled_player = anyone();
        let card = state.create_object(blank, milled_player, Zone::Graveyard, Some(1), Some(1));
        // The mill last, as it is the event collected (`collect`).
        let c = collect(&mut state, &[
            GameEvent::LeftBattlefield { object: watcher, to: Zone::Graveyard, last_controller },
            GameEvent::CreatureCardMilled { object: card, milled_player },
        ], &registry);
        let expected = [(watcher, last_controller, last_controller != milled_player)];
        assert!(collected(&c, &expected, |_, e| matches!(e, TriggerEvent::CreatureCardMilled { milled_object, milled_player: p }
            if *milled_object == card && *p == milled_player)));
        std::mem::forget((registry, state, c, cards));
    }
}
