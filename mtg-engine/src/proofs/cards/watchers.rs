//! The permanents whose abilities trigger on another object's entering,
//! a spell being cast, combat damage, or blocks (CR 603.2), with the
//! condition each one's text puts on that event.
//!
//! - Champion of the Parish, 1/1: "Whenever another Human you control
//!   enters, put a +1/+1 counter on this creature."
//! - Mentor of the Meek, 2/2: "Whenever another creature you control with
//!   power 2 or less enters, you may pay {1}. If you do, draw a card."
//! - Burning Vengeance, Enchantment: "Whenever you cast a spell from your
//!   graveyard, this enchantment deals 2 damage to any target."
//! - Charmbreaker Devils, 4/4: "Whenever you cast an instant or sorcery
//!   spell, this creature gets +4/+0 until end of turn." (Its upkeep
//!   trigger is `proofs::cards::steps`'.)
//! - Rakish Heir, 2/2: "Whenever a Vampire you control deals combat damage
//!   to a player, put a +1/+1 counter on it."
//! - Ashmouth Hound, 2/1: "Whenever this creature blocks or becomes blocked
//!   by a creature, this creature deals 1 damage to that creature."
//! - Hamlet Captain, 2/2: "Whenever this creature attacks or blocks, other
//!   Humans you control get +1/+1 until end of turn."
//!
//! Each proof pins the triggers the card declares and the condition the
//! engine asks it about the event, over every such event on a board of a
//! card and a token of any characteristics; then runs the card's code for
//! the trigger resolving, the permanent on the battlefield or gone since,
//! under either player's control. That the watcher is never told about its
//! own entering ("another") is the engine's (`triggers::collect`), proved
//! in `proofs::actions`.

use super::{act, acted, any_object, any_object_of, any_target, asked_for, board, card_proof, case_by_case, each_is, kept, triggers, Act, Form, Kinds, Snapshot};
use crate::cards::{AttackInfo, CardBehavior, CardRegistry, TargetRequirement as R, TriggerKind};
use crate::damage::DamageKind;
use crate::events::DamageTarget;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameObject, GameState, PendingEffect, ResolutionChoiceKind, TemporaryEffect as T};
use crate::types::{CardType, CounterType, ManaCost, ManaSymbol, Zone};

use crate::cards::isd;

const PEOPLE: Kinds = Kinds { types: &[CardType::Creature], subtypes: &["Human", "Vampire"], keywords: &[] };

/// The board: the card's permanent, owned by player 0, on the battlefield
/// or gone to their graveyard, controlled (or last controlled) by `you`;
/// a card and a token beside it.
struct Board {
    registry: CardRegistry,
    state: GameState,
    me: ObjectId,
    others: [ObjectId; 2],
    you: PlayerId,
}

fn watching(card: Box<dyn CardBehavior>, can_be_gone: bool) -> Board {
    watching_of(card, can_be_gone, &PEOPLE)
}

/// [`watching`]'s board, the other two objects of `kinds`.
fn watching_of(card: Box<dyn CardBehavior>, can_be_gone: bool, kinds: &Kinds) -> Board {
    watching_on(card, can_be_gone, |state, blank| {
        [any_object(state, blank, kinds, Form::Card), any_object(state, blank, kinds, Form::Token)]
    })
}

/// [`watching_of`]'s board, the card of exactly `subtypes[0]` and the
/// token of `subtypes[1]`.
fn watching_with(card: Box<dyn CardBehavior>, can_be_gone: bool, kinds: &Kinds, subtypes: [&[&str]; 2]) -> Board {
    watching_on(card, can_be_gone, |state, blank| {
        [any_object_of(state, blank, kinds, Form::Card, subtypes[0]), any_object_of(state, blank, kinds, Form::Token, subtypes[1])]
    })
}

/// The board with the other two objects made by `others`.
fn watching_on(card: Box<dyn CardBehavior>, can_be_gone: bool, others: impl FnOnce(&mut GameState, CardId) -> [ObjectId; 2]) -> Board {
    let (registry, mut state, blank, card) = board(card);
    let you = PlayerId(u8::from(kani::any::<bool>()));
    let gone = can_be_gone && kani::any();
    let me = state.create_object(card, PlayerId(0), if gone { Zone::Graveyard } else { Zone::Battlefield }, None, None);
    let o = state.get_object_mut(me).expect("just created");
    if gone {
        o.last_controller = Some(you);
        state.players[0].graveyard_order.push(me);
    } else {
        o.controller = you;
    }
    let others = others(&mut state, blank);
    Board { registry, state, me, others, you }
}

impl Board {
    fn card(&self) -> CardId {
        self.state.get_object(self.me).expect("on the board").card_id
    }

    fn get(&self, id: ObjectId) -> &GameObject {
        self.state.get_object(id).expect("on the board")
    }

    fn behavior(&self) -> &dyn CardBehavior {
        self.registry.get(self.card()).expect("registered")
    }

    fn is(&self, id: ObjectId, subtype: &str) -> bool {
        self.get(id).subtypes.iter().any(|s| s == subtype)
    }

    /// One of the other two objects.
    fn either(&self) -> ObjectId {
        if kani::any() { self.others[0] } else { self.others[1] }
    }
}

card_proof! {
    /// Champion of the Parish: triggers for a Human entering under its
    /// controller's control, and nothing else entering; a +1/+1 counter on
    /// itself.
    fn champion_of_the_parish_grows_when_a_human_of_yours_enters() {
        assert!(triggers(&isd::champion_of_the_parish::ChampionOfTheParish) == [(TriggerKind::AnyCreatureEnters, None)]);
        let mut b = watching(Box::new(isd::champion_of_the_parish::ChampionOfTheParish), false);
        let entered = b.either();
        let under = PlayerId(u8::from(kani::any::<bool>()));
        let fires = b.behavior().should_trigger_on_creature_enters(&b.state, b.me, entered, under, &b.registry);
        assert!(fires == (under == b.you && b.is(entered, "Human")));
        let before = Snapshot::of(&b.state);
        let card = b.card();
        b.registry.get(card).expect("registered").on_any_creature_enters(&mut b.state, b.me, entered, under, &b.registry);
        assert!(asked_for(&[Act::Counters(b.me, CounterType::PlusOnePlusOne, 1)]) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

/// Mentor of the Meek's board: its creature and two others, of no
/// subtype, which nothing here reads.
const CREATURES: Kinds = Kinds { types: &[CardType::Creature], subtypes: &[], keywords: &[] };

/// Mentor of the Meek: triggers for another creature with power 2 or less
/// entering under its controller's control, and nothing else entering.
/// Its effect is the proof below. For the creature that entered `which`
/// (the Mentor itself 0, the card 1, the token 2), a constant in each
/// harness: its power is read through the layers, and with the creature
/// the solver's, every object's was, and the proof ran out of memory at
/// four and a half gigabytes in symbolic execution.
fn mentor_of_the_meek_triggers_for_another_small_creature_of_yours(which: u8) {
    assert!(triggers(&isd::mentor_of_the_meek::MentorOfTheMeek) == [(TriggerKind::AnyCreatureEnters, None)]);
    let b = watching_of(Box::new(isd::mentor_of_the_meek::MentorOfTheMeek), false, &CREATURES);
    let entered = match which { 0 => b.me, 1 => b.others[0], _ => b.others[1] };
    let under = PlayerId(u8::from(kani::any::<bool>()));
    let power = b.state.effective_power(entered, &b.registry);
    let fires = b.behavior().should_trigger_on_creature_enters(&b.state, b.me, entered, under, &b.registry);
    assert!(fires == (entered != b.me && under == b.you && power.is_some_and(|p| p <= 2)));
    std::mem::forget(b);
}

case_by_case! { mentor_of_the_meek_triggers_for_another_small_creature_of_yours;
    mentor_of_the_meek_does_not_trigger_for_itself: 0,
    mentor_of_the_meek_triggers_for_a_small_creature_card: 1,
    mentor_of_the_meek_triggers_for_a_small_creature_token: 2,
}

/// Mentor of the Meek's effect: its controller is asked whether to pay
/// {1}, and on "yes" pays it and, if it was paid, draws a card (CR
/// 117.12a... "if you do"); on "no", nothing. Answered `yes` or not, a
/// constant in each harness: the trigger and both answers in one harness
/// ran out of memory at four gigabytes in symbolic execution.
fn mentor_of_the_meek_draws_for_a_small_creature_if_you_pay(yes: bool) {
    let mut b = watching_of(Box::new(isd::mentor_of_the_meek::MentorOfTheMeek), false, &CREATURES);
    let entered = b.either();
    let under = b.you;
    let before = Snapshot::of(&b.state);
    let card = b.card();
    b.registry.get(card).expect("registered").on_any_creature_enters(&mut b.state, b.me, entered, under, &b.registry);
    let asked = matches!(&b.state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
        player, source, choice: ResolutionChoiceKind::YesNo { source_card, .. },
    }) if *player == b.you && *source == b.me && *source_card == b.me);
    assert!(asked && acted() == 0);
    b.state.awaiting_action = None;
    assert!(Snapshot::of(&b.state) == before);
    b.registry.get(card).expect("registered").on_yes_no_choice(&mut b.state, b.me, yes, &b.registry);
    let one = ManaCost::new(vec![ManaSymbol::Generic(1)]);
    if yes {
        match act(0) {
            Some(Act::Pay(p, cost, true)) => {
                assert!(*p == b.you && *cost == one && acted() == 2 && Some(&Act::Draw(b.you, 1)) == act(1));
            }
            Some(Act::Pay(p, cost, false)) => {
                assert!(*p == b.you && *cost == one && acted() == 1);
            }
            _ => panic!("pays first"),
        }
    } else {
        assert!(acted() == 0);
    }
    std::mem::forget(b);
}

case_by_case! { mentor_of_the_meek_draws_for_a_small_creature_if_you_pay;
    mentor_of_the_meek_pays_and_draws_on_yes: true,
    mentor_of_the_meek_does_nothing_on_no: false,
}

card_proof! {
    /// Burning Vengeance: triggers for a spell its controller casts from a
    /// graveyard, and no other (CR 601.2a: where it was cast from is the
    /// spell's); 2 damage from it to any target.
    fn burning_vengeance_deals_two_when_you_cast_from_a_graveyard() {
        assert!(triggers(&isd::burning_vengeance::BurningVengeance) == [(TriggerKind::SpellCast, Some(R::AnyTarget))]);
        let mut b = watching(Box::new(isd::burning_vengeance::BurningVengeance), false);
        let caster = PlayerId(u8::from(kani::any::<bool>()));
        let spell = b.state.create_object(CardId(1), caster, Zone::Stack, None, None);
        let from = match kani::any::<u8>() % 3 { 0 => Some(Zone::Hand), 1 => Some(Zone::Graveyard), _ => None };
        b.state.get_object_mut(spell).expect("just created").cast_from_zone = from;
        let fires = b.behavior().should_trigger_on_spell_cast(&b.state, b.me, caster, spell, &b.registry);
        assert!(fires == (caster == b.you && from == Some(Zone::Graveyard)));
        let target = any_target(b.others);
        let before = Snapshot::of(&b.state);
        let card = b.card();
        b.registry.get(card).expect("registered").on_spell_cast(&mut b.state, b.me, caster, spell, &[target.clone()], &b.registry);
        assert!(asked_for(&[Act::Effect(target, PendingEffect::DealDamage { amount: 2, source_id: b.me })]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Charmbreaker Devils: triggers for an instant or sorcery spell its
    /// controller casts, and no other spell (CR 603.2: both are the
    /// trigger's condition, asked once, as it triggers); resolving, +4/+0
    /// until end of turn on itself, whoever controls it by then (CR
    /// 113.7a), and nothing else.
    fn charmbreaker_devils_gets_four_when_you_cast_an_instant_or_sorcery() {
        let devils = isd::charmbreaker_devils::CharmbreakerDevils;
        assert!(triggers(&devils) == [(TriggerKind::Upkeep, None), (TriggerKind::SpellCast, None)]);
        let mut b = watching(Box::new(devils), false);
        let caster = PlayerId(u8::from(kani::any::<bool>()));
        let spell = b.state.create_object(CardId(1), caster, Zone::Stack, None, None);
        let kind = match kani::any::<u8>() % 3 { 0 => CardType::Instant, 1 => CardType::Sorcery, _ => CardType::Creature };
        b.state.get_object_mut(spell).expect("just created").card_types = vec![kind];
        let fires = b.behavior().should_trigger_on_spell_cast(&b.state, b.me, caster, spell, &b.registry);
        assert!(fires == (caster == b.you && kind != CardType::Creature));
        let before = Snapshot::of(&b.state);
        let card = b.card();
        b.registry.get(card).expect("registered").on_spell_cast(&mut b.state, b.me, caster, spell, &[], &b.registry);
        assert!(b.state.until_end_of_turn[before.until_end_of_turn()..] == [T::ModifyPT { target: b.me, power_mod: 4, toughness_mod: 0 }]);
        assert!(acted() == 0 && Snapshot::of(&b.state).without_until_end_of_turn() == before.without_until_end_of_turn());
        std::mem::forget(b);
    }
}

card_proof! {
    /// Rakish Heir: triggers for combat damage to a player dealt by a
    /// Vampire its controller controls, and no other; a +1/+1 counter on
    /// that Vampire.
    fn rakish_heir_grows_your_vampire_that_dealt_combat_damage() {
        assert!(triggers(&isd::rakish_heir::RakishHeir) == [(TriggerKind::AnyCombatDamageToPlayer, None)]);
        let mut b = watching(Box::new(isd::rakish_heir::RakishHeir), true);
        let source = b.either();
        let damaged = PlayerId(u8::from(kani::any::<bool>()));
        let fires = b.behavior().should_trigger_on_damage_to_player(&b.state, b.me, source, damaged, &b.registry);
        assert!(fires == (b.get(source).controller == b.you && b.is(source, "Vampire")));
        let before = Snapshot::of(&b.state);
        let card = b.card();
        b.registry.get(card).expect("registered").on_any_combat_damage_to_player(&mut b.state, b.me, source, damaged, 2, &b.registry);
        assert!(asked_for(&[Act::Counters(source, CounterType::PlusOnePlusOne, 1)]) && Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Ashmouth Hound: triggers as it blocks and as it becomes blocked; 1
    /// damage from it to the creature it blocked, or that blocked it.
    fn ashmouth_hound_deals_one_to_what_it_fights_in_combat() {
        assert!(triggers(&isd::ashmouth_hound::AshmouthHound) == [(TriggerKind::Blocks, None), (TriggerKind::BecomesBlocked, None)]);
        let mut b = watching(Box::new(isd::ashmouth_hound::AshmouthHound), true);
        let other = b.either();
        let before = Snapshot::of(&b.state);
        let card = b.card();
        let hound = b.registry.get(card).expect("registered");
        if kani::any() {
            hound.on_blocks(&mut b.state, b.me, other, &b.registry);
        } else {
            hound.on_becomes_blocked(&mut b.state, b.me, other, &b.registry);
        }
        assert!(asked_for(&[Act::Damage { source: b.me, target: DamageTarget::Object(other), amount: 1, kind: DamageKind::NonCombat }]));
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

/// The subtypes [`hamlet_captain_pumps_your_other_humans`] gives each of
/// the other two objects: none, Human, or Vampire.
const HAMLET_SUBTYPES: [&[&str]; 3] = [&[], &["Human"], &["Vampire"]];

/// Hamlet Captain: triggers as it attacks and as it blocks; +1/+1 until
/// end of turn on each other Human creature its controller controls. With
/// the other two objects' subtypes those of `HAMLET_SUBTYPES` at `card`
/// and `token`, a constant in each harness: over subtypes the solver
/// chose, each object's compared with "Human" byte by byte, the proof ran
/// out of memory at four and a half gigabytes.
fn hamlet_captain_pumps_your_other_humans((card, token): (usize, usize)) {
    assert!(triggers(&isd::hamlet_captain::HamletCaptain) == [(TriggerKind::Attacks, None), (TriggerKind::Blocks, None)]);
    let mut b = watching_with(Box::new(isd::hamlet_captain::HamletCaptain), true, &PEOPLE, [HAMLET_SUBTYPES[card], HAMLET_SUBTYPES[token]]);
    let humans = kept(&b.others, |id| {
        let o = b.get(id);
        o.zone == Zone::Battlefield && o.controller == b.you && o.card_types.contains(&CardType::Creature) && b.is(id, "Human")
    });
    let before = Snapshot::of(&b.state);
    let card = b.card();
    let captain = b.registry.get(card).expect("registered");
    if kani::any() {
        captain.on_attacks(&mut b.state, b.me, AttackInfo::new(b.me, PlayerId(1 - b.you.0)), &[], &b.registry);
    } else {
        captain.on_blocks(&mut b.state, b.me, b.others[0], &b.registry);
    }
    let added = &b.state.until_end_of_turn[before.until_end_of_turn()..];
    assert!(each_is(added, humans.len(), |i| T::ModifyPT { target: humans[i], power_mod: 1, toughness_mod: 1 }) && acted() == 0);
    assert!(Snapshot::of(&b.state).without_until_end_of_turn() == before.without_until_end_of_turn());
    std::mem::forget(b);
}

case_by_case! { hamlet_captain_pumps_your_other_humans;
    hamlet_captain_with_neither_other_object_of_a_subtype: (0, 0),
    hamlet_captain_with_the_token_a_human: (0, 1),
    hamlet_captain_with_the_token_a_vampire: (0, 2),
    hamlet_captain_with_the_card_a_human: (1, 0),
    hamlet_captain_with_both_humans: (1, 1),
    hamlet_captain_with_the_card_a_human_and_the_token_a_vampire: (1, 2),
    hamlet_captain_with_the_card_a_vampire: (2, 0),
    hamlet_captain_with_the_card_a_vampire_and_the_token_a_human: (2, 1),
    hamlet_captain_with_both_vampires: (2, 2),
}
