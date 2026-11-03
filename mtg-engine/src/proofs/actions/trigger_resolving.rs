//! A triggered ability resolving off the stack
//! (`triggers::resolve_next_trigger`, CR 608.2, 603.2, 113.7a): what the
//! engine hands a card's trigger hook, which is where a card proof of a
//! triggered ability starts. A card proof runs the hook (`on_enter_battlefield`,
//! `on_dies`, `on_attacks`, ...) with the event's own data and the targets
//! it was put on the stack with; these show that the engine calls exactly
//! that hook, once, with exactly those, when the rules say the ability
//! resolves, and none when its only target has become illegal.
//!
//! As in `resolving`, the questions the re-check asks of a target (CR
//! 608.2b) are any answer, the same each time they are asked: whether it
//! still meets the requirement it was chosen under (`is_target_legal`),
//! whether it may still be targeted by the ability's source
//! (`can_be_targeted_by`: hexproof, protection, 702.16b) and whether it
//! still meets what the card asks (`is_valid_target`). Every targeted
//! trigger in the pool has one target; each kind of event a targeted one
//! is on is proved with no target and with one.

use crate::actions::Target;
use crate::cards::{AttackInfo, CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{card_proof, Blank};
use crate::state::{GameState, StackEntry};
use crate::triggers::{DeadCreature, TriggerEvent};
use crate::types::Zone;

/// What is asked of the target.
#[derive(Clone, Copy)]
enum Asked {
    /// It meets the requirement it was chosen under.
    Legal,
    /// It may be targeted by the ability's source.
    Targetable,
    /// It meets what the card itself asks of it.
    Valid,
}

/// The answers for the target (the board's object 1), by question.
static mut ANSWERS: [Option<bool>; 3] = [None; 3];

fn answer(id: ObjectId, asked: Asked) -> bool {
    assert!(id == ObjectId(1), "a target this proof does not make");
    // SAFETY: a proof runs on one thread, and only this function names ANSWERS.
    let answers = unsafe { &mut *std::ptr::addr_of_mut!(ANSWERS) };
    *answers[asked as usize].get_or_insert_with(kani::any::<bool>)
}

/// Stands in for `stack::is_target_legal`: any answer for the target.
#[allow(dead_code)] // named only by `kani::stub`
fn is_target_legal(_: &GameState, target: &Target, _: &TargetRequirement, _: PlayerId, _: Option<ObjectId>, _: &CardRegistry) -> bool {
    match target {
        Target::Object(id) => answer(*id, Asked::Legal),
        _ => panic!("a target this proof does not make"),
    }
}

/// Stands in for `engine::can_be_targeted_by`: any answer.
#[allow(dead_code)] // named only by `kani::stub`
fn can_be_targeted_by(_: &GameState, id: ObjectId, _: PlayerId, _: Option<ObjectId>, _: &CardRegistry) -> bool {
    answer(id, Asked::Targetable)
}

/// Stands in for `triggers::target_requirement`, the requirement the card
/// prints on the trigger: none. Only the re-check's first question reads
/// it, and that question (`is_target_legal`) is any answer here whatever
/// the requirement. Read off the card's data rebuilt on the heap, it was
/// cloned as every shape a requirement may take, under a guard symbolic
/// execution could not settle (whether the trigger, read back off the
/// stack, has a target at all), and that ran it out of memory.
#[allow(dead_code)] // named only by `kani::stub`
fn no_requirement(_: &CardRegistry, _: crate::ids::CardId, _: Option<crate::cards::TriggerKind>) -> Option<TargetRequirement> {
    None
}

/// Which hook was called.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Hook {
    Enters,
    Dies,
    AnyDies,
    AnyEnters,
    DamagesPlayer,
    DamagesCreature,
    AnyCombatDamage,
    AnyDamage,
    EndCombat,
    Upkeep,
    EndStep,
    SpellCast,
    Leaves,
    Attacks,
    AnyAttacks,
    Blocks,
    Blocked,
    Milled,
    State,
}

/// What a hook was handed: which hook, the permanent it is on, the event's
/// data as numbers, its target and how many, the face it resolved as
/// from, and whether the ability was the one resolving.
type Handed = (Hook, ObjectId, [u64; 5], Option<Target>, usize, Option<bool>, bool);

static mut HOOKED: Option<Handed> = None;

fn hooked(state: &GameState, hook: Hook, self_id: ObjectId, data: [u64; 5], targets: &[Target]) {
    assert!(targets.len() <= 1, "no more targets than this proof gives");
    let resolving = matches!(state.resolving_ability, Some(StackEntry::Trigger(_)));
    // SAFETY: as in `answer`.
    let h = unsafe { &mut *std::ptr::addr_of_mut!(HOOKED) };
    assert!(h.is_none(), "one hook, called once");
    *h = Some((hook, self_id, data, targets.first().cloned(), targets.len(), state.resolving_trigger_from_back_face, resolving));
}

/// Where the list of what damaged the dead creature is, as the event is
/// made: the hook is handed the event's own list (the same one, not one
/// like it). Its contents, read back through the trigger's trip onto the
/// stack and off, ran the solver out of memory.
static mut DAMAGED_BY: *const ObjectId = std::ptr::null();

fn id(o: ObjectId) -> u64 {
    o.0
}

fn player(p: PlayerId) -> u64 {
    u64::from(p.0)
}

/// A card each of whose trigger hooks records what it is handed.
struct Hooked;

impl CardBehavior for Hooked {
    fn card_data(&self) -> CardData {
        CardData { name: "Hooked".into(), ..CardData::default() }
    }
    fn is_valid_target(&self, _: &GameState, _: PlayerId, target: &Target, _: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => answer(*id, Asked::Valid),
            _ => panic!("a target this proof does not make"),
        }
    }
    fn on_enter_battlefield(&self, state: &mut GameState, s: ObjectId, targets: &[Target], _: &CardRegistry) {
        hooked(state, Hook::Enters, s, [0; 5], targets);
    }
    fn on_dies(&self, state: &mut GameState, s: ObjectId, targets: &[Target], _: &CardRegistry) {
        hooked(state, Hook::Dies, s, [0; 5], targets);
    }
    fn on_any_creature_dies(&self, state: &mut GameState, s: ObjectId, dead: ObjectId, controller: PlayerId,
        damaged_by: &[ObjectId], toughness: i32, is_token: bool, targets: &[Target], _: &CardRegistry) {
        // SAFETY: as in `answer`.
        let list = unsafe { *std::ptr::addr_of!(DAMAGED_BY) };
        assert!(damaged_by.len() == 1 && std::ptr::eq(damaged_by.as_ptr(), list), "the dead creature's own list of what damaged it");
        let t = u64::from(toughness.cast_unsigned());
        hooked(state, Hook::AnyDies, s, [id(dead), player(controller), t, u64::from(is_token), 0], targets);
    }
    fn on_any_creature_enters(&self, state: &mut GameState, s: ObjectId, entered: ObjectId, controller: PlayerId, _: &CardRegistry) {
        hooked(state, Hook::AnyEnters, s, [id(entered), player(controller), 0, 0, 0], &[]);
    }
    fn on_combat_damage_to_player(&self, state: &mut GameState, s: ObjectId, damaged: PlayerId, amount: u32, _: &CardRegistry) {
        hooked(state, Hook::DamagesPlayer, s, [player(damaged), u64::from(amount), 0, 0, 0], &[]);
    }
    fn on_deals_combat_damage_to_creature(&self, state: &mut GameState, s: ObjectId, damaged: ObjectId, amount: u32, _: &CardRegistry) {
        hooked(state, Hook::DamagesCreature, s, [id(damaged), u64::from(amount), 0, 0, 0], &[]);
    }
    fn on_any_combat_damage_to_player(&self, state: &mut GameState, s: ObjectId, dealer: ObjectId, damaged: PlayerId, amount: u32, _: &CardRegistry) {
        hooked(state, Hook::AnyCombatDamage, s, [id(dealer), player(damaged), u64::from(amount), 0, 0], &[]);
    }
    fn on_any_damage_to_player(&self, state: &mut GameState, s: ObjectId, dealer: ObjectId, damaged: PlayerId, amount: u32, _: &CardRegistry) {
        hooked(state, Hook::AnyDamage, s, [id(dealer), player(damaged), u64::from(amount), 0, 0], &[]);
    }
    fn on_end_combat(&self, state: &mut GameState, s: ObjectId, _: &CardRegistry) {
        hooked(state, Hook::EndCombat, s, [0; 5], &[]);
    }
    fn on_upkeep(&self, state: &mut GameState, s: ObjectId, targets: &[Target], _: &CardRegistry) {
        hooked(state, Hook::Upkeep, s, [0; 5], targets);
    }
    fn on_end_step(&self, state: &mut GameState, s: ObjectId, targets: &[Target], _: &CardRegistry) {
        hooked(state, Hook::EndStep, s, [0; 5], targets);
    }
    fn on_spell_cast(&self, state: &mut GameState, s: ObjectId, caster: PlayerId, spell: ObjectId, targets: &[Target], _: &CardRegistry) {
        hooked(state, Hook::SpellCast, s, [player(caster), id(spell), 0, 0, 0], targets);
    }
    fn on_leave_battlefield(&self, state: &mut GameState, s: ObjectId, _: &CardRegistry) {
        hooked(state, Hook::Leaves, s, [0; 5], &[]);
    }
    fn on_attacks(&self, state: &mut GameState, s: ObjectId, attack: AttackInfo, targets: &[Target], _: &CardRegistry) {
        hooked(state, Hook::Attacks, s, [id(attack.attacker), player(attack.defending_player), 0, 0, 0], targets);
    }
    fn on_any_creature_attacks(&self, state: &mut GameState, s: ObjectId, attacker: ObjectId, controller: PlayerId, _: &CardRegistry) {
        hooked(state, Hook::AnyAttacks, s, [id(attacker), player(controller), 0, 0, 0], &[]);
    }
    fn on_blocks(&self, state: &mut GameState, s: ObjectId, blocked: ObjectId, _: &CardRegistry) {
        hooked(state, Hook::Blocks, s, [id(blocked), 0, 0, 0, 0], &[]);
    }
    fn on_becomes_blocked(&self, state: &mut GameState, s: ObjectId, blocker: ObjectId, _: &CardRegistry) {
        hooked(state, Hook::Blocked, s, [id(blocker), 0, 0, 0, 0], &[]);
    }
    fn on_creature_card_milled(&self, state: &mut GameState, s: ObjectId, milled: ObjectId, owner: PlayerId, _: &CardRegistry) {
        hooked(state, Hook::Milled, s, [id(milled), player(owner), 0, 0, 0], &[]);
    }
    fn on_state_trigger(&self, state: &mut GameState, s: ObjectId, _: &CardRegistry) {
        // CR 603.8: the flag that kept it from triggering again is down
        // before it resolves.
        let flag = state.get_object(s).expect("on the board").state_trigger_on_stack;
        hooked(state, Hook::State, s, [u64::from(flag), 0, 0, 0, 0], &[]);
    }
}

/// Each kind of event a trigger resolves from.
#[derive(Clone, Copy)]
enum Kind {
    Enters,
    Dies,
    AnyDies,
    AnyEnters,
    DamagesPlayer,
    DamagesCreature,
    AnyCombatDamage,
    AnyDamage,
    EndCombat,
    Upkeep,
    EndStep,
    SpellCast,
    Leaves,
    Attacks,
    AnyAttacks,
    Blocks,
    Blocked,
    Milled,
    State,
}

/// The event of `kind`, its data any value, the hook it is for, the data
/// that hook should be handed, and whether a trigger on it may target.
fn event_of(kind: Kind) -> (TriggerEvent, Hook, [u64; 5], bool) {
    let (a, b, c): (u64, u8, u32) = (kani::any(), kani::any(), kani::any());
    match kind {
        Kind::Enters => (TriggerEvent::SelfEntered, Hook::Enters, [0; 5], true),
        Kind::Dies => (TriggerEvent::SelfDies, Hook::Dies, [0; 5], true),
        Kind::AnyDies => {
            let (toughness, is_token, by): (i32, bool, u64) = (kani::any(), kani::any(), kani::any());
            let damaged_by = vec![ObjectId(by)];
            // SAFETY: as in `answer`.
            unsafe { *std::ptr::addr_of_mut!(DAMAGED_BY) = damaged_by.as_ptr() };
            let dead = DeadCreature {
                id: ObjectId(a), name: String::new(), controller: PlayerId(b), damaged_by,
                toughness, is_token, subtypes: Vec::new(),
            };
            let data = [a, u64::from(b), u64::from(toughness.cast_unsigned()), u64::from(is_token), 0];
            (TriggerEvent::CreatureDied { dead }, Hook::AnyDies, data, true)
        }
        Kind::AnyEnters => (TriggerEvent::CreatureEntered { entered: ObjectId(a), entered_controller: PlayerId(b) },
            Hook::AnyEnters, [a, u64::from(b), 0, 0, 0], false),
        Kind::DamagesPlayer => (TriggerEvent::CombatDamageToPlayer { damaged_player: PlayerId(b), amount: c },
            Hook::DamagesPlayer, [u64::from(b), u64::from(c), 0, 0, 0], false),
        Kind::DamagesCreature => (TriggerEvent::CombatDamageToCreature { damaged_creature: ObjectId(a), amount: c },
            Hook::DamagesCreature, [a, u64::from(c), 0, 0, 0], false),
        Kind::AnyCombatDamage => (TriggerEvent::AnyCombatDamageToPlayer { dealer: ObjectId(a), damaged_player: PlayerId(b), amount: c },
            Hook::AnyCombatDamage, [a, u64::from(b), u64::from(c), 0, 0], false),
        Kind::AnyDamage => (TriggerEvent::AnyDamageToPlayer { dealer: ObjectId(a), damaged_player: PlayerId(b), amount: c },
            Hook::AnyDamage, [a, u64::from(b), u64::from(c), 0, 0], false),
        Kind::EndCombat => (TriggerEvent::EndCombat, Hook::EndCombat, [0; 5], false),
        Kind::Upkeep => (TriggerEvent::Upkeep, Hook::Upkeep, [0; 5], true),
        Kind::EndStep => (TriggerEvent::EndStep, Hook::EndStep, [0; 5], true),
        Kind::SpellCast => (TriggerEvent::SpellCast { caster: PlayerId(b), spell_id: ObjectId(a) },
            Hook::SpellCast, [u64::from(b), a, 0, 0, 0], true),
        Kind::Leaves => (TriggerEvent::LeftBattlefield, Hook::Leaves, [0; 5], false),
        Kind::Attacks => (TriggerEvent::Attacks { attacker: ObjectId(a), defending_player: PlayerId(b) },
            Hook::Attacks, [a, u64::from(b), 0, 0, 0], true),
        Kind::AnyAttacks => (TriggerEvent::CreatureAttacked { attacker: ObjectId(a), attacker_controller: PlayerId(b) },
            Hook::AnyAttacks, [a, u64::from(b), 0, 0, 0], false),
        Kind::Blocks => (TriggerEvent::Blocks { blocked_attacker: ObjectId(a) }, Hook::Blocks, [a, 0, 0, 0, 0], false),
        Kind::Blocked => (TriggerEvent::BecomesBlocked { blocker_id: ObjectId(a) }, Hook::Blocked, [a, 0, 0, 0, 0], false),
        Kind::Milled => (TriggerEvent::CreatureCardMilled { milled_object: ObjectId(a), milled_player: PlayerId(b) },
            Hook::Milled, [a, u64::from(b), 0, 0, 0], false),
        // The flag down when the hook runs (CR 603.8).
        Kind::State => (TriggerEvent::StateTriggered, Hook::State, [0; 5], false),
    }
}

/// A triggered ability of the event `kind` resolving (CR 608.2, 113.7a,
/// 603.4): with its one target no longer legal (it does not meet the
/// requirement it was chosen under, may no longer be targeted by the
/// ability's source, or does not meet what the card asks), it does
/// nothing; otherwise the hook for its event, and only that one, is
/// called once, on the ability's source, with the event's data and the
/// target as chosen, while the ability is the one resolving and as from
/// the face it triggered from. Either way it leaves the stack, and nothing
/// is left resolving after. For a source either player's, on the
/// battlefield or gone (113.7a: the ability resolves without it), of
/// either face, and, for an event a targeted trigger in the pool is on, no
/// target or one, a creature either player's.
fn resolving(kind: Kind) {
    let mut registry = CardRegistry::new();
    let blank = registry.register(Box::new(Blank));
    let card = registry.register(Box::new(Hooked));
    let mut state = crate::proofs::cards::new_game();
    let target = state.create_object(blank, PlayerId(u8::from(kani::any::<bool>())), Zone::Battlefield, Some(1), Some(1));
    assert!(target == ObjectId(1));
    let controller = PlayerId(u8::from(kani::any::<bool>()));
    let source = state.create_object(card, controller, if kani::any() { Zone::Battlefield } else { Zone::Graveyard }, Some(1), Some(1));
    if matches!(kind, Kind::State) {
        state.get_object_mut(source).expect("just created").state_trigger_on_stack = true;
    }
    let from_back: bool = kani::any();
    let (event, hook, data, may_target) = event_of(kind);
    // Built as the collector builds it, filed under a player of its own
    // and then given its controller: filed under a controller left to the
    // solver, which of the collector's two lists it went to was a question
    // symbolic execution did not settle (as in `delayed`), and the trigger
    // read back, its targets among it, was any trigger.
    let mut c = crate::triggers::collect::Collector::new(PlayerId(0));
    c.emit_from_face(source, card, PlayerId(0), String::new(), event, from_back);
    let mut trigger = c.ap.pop().expect("just emitted");
    trigger.source.controller = controller;
    let targeted = may_target && kani::any::<bool>();
    if targeted {
        trigger.source.chosen_targets = Vec::with_capacity(1);
        trigger.source.chosen_targets.push(Target::Object(target));
    }
    state.stack.push(StackEntry::Trigger(trigger));

    assert!(crate::triggers::resolve_next_trigger(&mut state, &registry));

    let legal = !targeted || (answer(target, Asked::Legal) && answer(target, Asked::Targetable) && answer(target, Asked::Valid));
    // SAFETY: as in `answer`.
    let h = unsafe { &*std::ptr::addr_of!(HOOKED) };
    assert!(state.stack.is_empty());
    if legal {
        let Some((called, on, handed, chosen, n, face, while_resolving)) = h else { panic!("a hook called") };
        // Element by element: arrays compared whole are a `memcmp`, a loop
        // over each of their bytes.
        let same = (0..5).all(|i| handed[i] == data[i]);
        assert!(*called == hook && *on == source && same && *chosen == targeted.then_some(Target::Object(target))
            && *n == usize::from(targeted) && *face == Some(from_back) && *while_resolving);
    } else {
        assert!(h.is_none());
    }
    assert!(state.resolving_ability.is_none() && state.resolving_trigger_from_back_face.is_none());
    std::mem::forget((registry, state, c));
}

/// The proofs of a trigger's resolving, the re-check's questions answered
/// as above.
macro_rules! each_trigger_event {
    ($($name:ident: $kind:expr,)*) => {$(
        card_proof! {
            #[kani::stub(crate::stack::is_target_legal, is_target_legal)]
            #[kani::stub(crate::engine::can_be_targeted_by, can_be_targeted_by)]
            #[kani::stub(crate::triggers::target_requirement, no_requirement)]
            /// [`resolving`] for the event of its name.
            fn $name() {
                resolving($kind);
            }
        }
    )*};
}

each_trigger_event! {
    an_entering_trigger_resolves_with_its_target_unless_it_is_illegal: Kind::Enters,
    a_dying_trigger_resolves_with_its_target_unless_it_is_illegal: Kind::Dies,
    a_creature_dying_trigger_resolves_with_the_dead_and_its_target_unless_it_is_illegal: Kind::AnyDies,
    a_creature_entering_trigger_resolves_with_the_creature: Kind::AnyEnters,
    a_combat_damage_to_a_player_trigger_resolves_with_the_player_and_amount: Kind::DamagesPlayer,
    a_combat_damage_to_a_creature_trigger_resolves_with_the_creature_and_amount: Kind::DamagesCreature,
    a_watchers_combat_damage_trigger_resolves_with_the_dealer_player_and_amount: Kind::AnyCombatDamage,
    a_watchers_damage_trigger_resolves_with_the_dealer_player_and_amount: Kind::AnyDamage,
    an_end_of_combat_trigger_resolves: Kind::EndCombat,
    an_upkeep_trigger_resolves_with_its_target_unless_it_is_illegal: Kind::Upkeep,
    an_end_step_trigger_resolves_with_its_target_unless_it_is_illegal: Kind::EndStep,
    a_spell_cast_trigger_resolves_with_the_caster_spell_and_target_unless_it_is_illegal: Kind::SpellCast,
    a_leaving_trigger_resolves: Kind::Leaves,
    an_attack_trigger_resolves_with_the_attack_and_its_target_unless_it_is_illegal: Kind::Attacks,
    a_watchers_attack_trigger_resolves_with_the_attacker: Kind::AnyAttacks,
    a_blocking_trigger_resolves_with_the_attacker_blocked: Kind::Blocks,
    a_blocked_trigger_resolves_with_the_blocker: Kind::Blocked,
    a_milling_trigger_resolves_with_the_card_and_its_owner: Kind::Milled,
    a_state_trigger_resolves_with_its_flag_down: Kind::State,
}

/// A card printing an entering trigger that targets a creature and a dying
/// trigger that targets nothing.
struct Printing;

impl CardBehavior for Printing {
    fn card_data(&self) -> CardData {
        use crate::cards::{TriggerKind, TriggeredAbilityDef};
        CardData {
            name: "Printing".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef { kind: TriggerKind::EntersBattlefield, description: String::new(), target_requirement: Some(TargetRequirement::Creature) },
                TriggeredAbilityDef { kind: TriggerKind::SelfDies, description: String::new(), target_requirement: None },
            ],
            ..CardData::default()
        }
    }
}

card_proof! {
    /// What [`resolving`]'s proofs stand in for (`no_requirement`): the
    /// requirement a trigger's target is re-checked under
    /// (`triggers::target_requirement`, CR 608.2b) is the one its card
    /// prints on the ability of that kind, none for one that targets
    /// nothing or that the card does not print, and none for a trigger of
    /// no kind (a state trigger, the delayed exile).
    fn the_requirement_a_trigger_is_rechecked_under_is_the_one_its_card_prints() {
        use crate::cards::TriggerKind;
        let mut registry = CardRegistry::new();
        let card = registry.register(Box::new(Printing));
        assert!(matches!(crate::triggers::target_requirement(&registry, card, Some(TriggerKind::EntersBattlefield)),
            Some(TargetRequirement::Creature)));
        assert!(crate::triggers::target_requirement(&registry, card, Some(TriggerKind::SelfDies)).is_none());
        assert!(crate::triggers::target_requirement(&registry, card, Some(TriggerKind::Upkeep)).is_none());
        assert!(crate::triggers::target_requirement(&registry, card, None).is_none());
        std::mem::forget(registry);
    }
}
